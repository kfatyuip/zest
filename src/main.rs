use tsr::{
    config::{CONFIG, CONFIG_PATH},
    route::{location_index, mime_match, status_page},
};

use chrono::{DateTime, Utc};
use clap::Parser;
use mime::Mime;
use std::{collections::HashMap, env, error::Error, io, net::IpAddr, ops::Deref, path::Path};

#[cfg(feature = "log")]
use log::log;

#[macro_use]
#[cfg(feature = "lru_cache")]
extern crate lazy_static;

#[cfg(target_os = "android")]
use std::os::android::fs::MetadataExt;

#[cfg(target_os = "linux")]
use std::os::linux::fs::MetadataExt;

#[cfg(feature = "lru_cache")]
use {
    async_mutex::Mutex, // faster than tokio::sync::Mutex
    lru::LruCache,
    std::num::NonZeroUsize,
};

use tokio::{
    fs::File,
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
};

const DATE_FORMAT: &str = "%a, %d %b %Y %H:%M:%S GMT";

#[cfg(feature = "log")]
const LOG_FORMAT: &str = "[{d(%Y-%m-%dT%H:%M:%SZ)} {h({l})}  tsr] {m}\n";

#[cfg(feature = "lru_cache")]
lazy_static! {
    static ref CACHE: Mutex<LruCache<String, String>> = {
        let cache = LruCache::new(NonZeroUsize::new(64).unwrap());
        Mutex::new(cache)
    };
}

#[derive(Clone)]
struct Response<'a> {
    version: &'a str,
    status_code: i32,
    _headers_buffer: HashMap<&'a str, String>,
}

impl<'a> Response<'a> {
    #[inline]
    fn send_header<T>(&mut self, k: &'a str, v: T) -> Option<String>
    where
        T: ToString,
    {
        self._headers_buffer.insert(k, v.to_string())
    }
    #[inline]
    fn resp(&mut self) -> String {
        let (version, status_code) = (self.version, self.status_code);
        let mut resp = format!("HTTP/{} {}\r\n", version, self.status(status_code));
        for (key, value) in &self._headers_buffer {
            resp.push_str(&format!("{}: {}\r\n", key, value));
        }
        resp.push_str("\r\n");
        resp
    }
    #[inline]
    fn status(&mut self, status_code: i32) -> String {
        let status = match status_code {
            200 => "OK",
            301 => "Moved Permanently",
            400 => "Bad Request",
            404 => "Not Found",
            501 => "Not Implemented",
            _ => "Internal Server Error", // 500
        };

        format!("{} {}", status_code, status)
    }
}

async fn handle_connection(mut stream: TcpStream) -> io::Result<(i32, String)> {
    let config = CONFIG.deref();

    let mut response: Response = Response {
        version: "1.1",
        status_code: 200,
        _headers_buffer: HashMap::new(),
    };

    let server_info = format!("TSR/{} ({})", env!("CARGO_PKG_VERSION"), config.server.info);
    response.send_header("Server", server_info.clone());

    response.send_header("Date", Utc::now().format(DATE_FORMAT));

    let buf_reader = BufReader::new(&mut stream);
    let req = buf_reader.lines().next_line().await?.unwrap_or_default();

    // GET /location HTTP/1.1
    let parts: Vec<&str> = req.split('/').collect();

    let mut mime_type: Mime = mime::TEXT_HTML_UTF_8;
    let mut buffer: Vec<u8> = Vec::new();

    if parts.len() < 3 {
        response.status_code = 400;
    } else if parts.first().unwrap().trim() != "GET" {
        response.status_code = 501;
    } else if let Some(location) = &req.split_whitespace().nth(1) {
        let location: String = urlencoding::decode(location.trim_start_matches('/'))
            .unwrap_or_default()
            .into();

        response.version = parts.last().unwrap();
        let mut path = config.server.root.join(location.split('?').next().unwrap());

        path = match path.canonicalize() {
            Ok(canonical_path) => canonical_path,
            Err(_) => {
                response.status_code = 404;
                config
                    .server
                    .root
                    .join(Path::new(
                        &config
                            .server
                            .error_page
                            .clone()
                            .unwrap_or("404.html".into()),
                    ))
                    .to_path_buf()
                    .canonicalize()
                    .unwrap_or_default()
            }
        };
        if !config.server.auto_index.unwrap_or(false)
            && !path.starts_with(config.server.root.canonicalize().unwrap())
        {
            response.status_code = 301;
        } else if path.is_dir() {
            #[allow(unused_assignments)]
            let mut html: String = String::new();
            #[cfg(feature = "lru_cache")]
            {
                let mut cache = CACHE.lock().await;
                if let Some(ctx) = cache.get(&location) {
                    html.clone_from(ctx);
                } else {
                    cache
                        .push(location.clone(), location_index(path, &location).await?)
                        .to_owned()
                        .unwrap_or_default();
                    html.clone_from(cache.get(&location).unwrap());
                }
            }
            #[cfg(not(feature = "lru_cache"))]
            {
                html = location_index(path, &location).await?;
            }

            buffer = html.into_bytes();
        } else {
            // path.is_file()
            match File::open(path.clone()).await {
                Ok(f) => {
                    let mut file = f;
                    mime_type = mime_match(path.to_str().unwrap());
                    file.read_to_end(&mut buffer).await?;

                    response.send_header(
                        "Last-Modified",
                        DateTime::from_timestamp(file.metadata().await?.st_atime(), 0)
                            .unwrap()
                            .format(DATE_FORMAT),
                    );
                }
                Err(_) => {
                    response.status_code = 500;
                }
            };
        }
    } else {
        response.status_code = 400;
    }

    if response.status_code != 200 {
        buffer = status_page(&response.status(response.status_code), server_info)
            .await
            .into()
    }
    response.send_header("Content-Length", buffer.len());
    response.send_header("Content-Type", mime_type);
    stream.write_all(response.resp().as_bytes()).await?;
    stream.write_all(&buffer).await?;
    stream.flush().await?;
    stream.shutdown().await?;

    Ok((response.status_code, req))
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = None, help = "set config file path")]
    config: Option<String>,

    #[arg(short, long, default_value = None, help = "set the listening port")]
    port: Option<i32>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let arg = Args::parse();
    *CONFIG_PATH.lock()? = arg.config.unwrap_or(String::new());
    let config = CONFIG.deref();

    #[cfg(feature = "log")]
    {
        use log4rs::{
            append::{console::ConsoleAppender, file::FileAppender},
            config::{Appender, Logger, Root},
            encode::pattern::PatternEncoder,
            Config,
        };

        let mut builder = Config::builder()
            .logger(Logger::builder().build("info", log::LevelFilter::Info))
            .logger(Logger::builder().build("error", log::LevelFilter::Error));

        let stdout = ConsoleAppender::builder()
            .encoder(Box::new(PatternEncoder::new(LOG_FORMAT)))
            .target(log4rs::append::console::Target::Stdout)
            .build();

        let stderr = ConsoleAppender::builder()
            .encoder(Box::new(PatternEncoder::new(LOG_FORMAT)))
            .target(log4rs::append::console::Target::Stderr)
            .build();

        if let Some(logging) = &config.logging {
            builder = if let Some(access_log) = &logging.access_log {
                let access_log_path = Path::new(&access_log);
                std::fs::File::create(access_log_path).unwrap();
                builder.appender(
                    Appender::builder().build(
                        "logfile_access",
                        Box::new(
                            FileAppender::builder()
                                .encoder(Box::new(PatternEncoder::new(LOG_FORMAT)))
                                .build(access_log_path)
                                .unwrap(),
                        ),
                    ),
                )
            } else {
                builder.appender(Appender::builder().build("logfile_access", Box::new(stdout)))
            };
            builder = if let Some(error_log) = &logging.error_log {
                let error_log_path = Path::new(&error_log);
                std::fs::File::create(error_log_path).unwrap();
                builder.appender(
                    Appender::builder().build(
                        "logfile_error",
                        Box::new(
                            FileAppender::builder()
                                .encoder(Box::new(PatternEncoder::new(LOG_FORMAT)))
                                .build(Path::new(&error_log))
                                .unwrap(),
                        ),
                    ),
                )
            } else {
                builder.appender(Appender::builder().build("logfile_error", Box::new(stderr)))
            }
        } else {
            builder = builder
                .appender(Appender::builder().build("logfile_access", Box::new(stdout)))
                .appender(Appender::builder().build("logfile_error", Box::new(stderr)));
        }

        let config = builder
            .build(
                Root::builder()
                    .appender("logfile_access")
                    .appender("logfile_error")
                    .build(log::LevelFilter::Info),
            )
            .unwrap();

        log4rs::init_config(config).unwrap();
    }

    let listener = TcpListener::bind(format!(
        "{}:{}",
        config.bind.addr,
        arg.port.unwrap_or(config.bind.listen)
    ))
    .await?;

    let mut _allowlist: Option<Vec<IpAddr>>;
    let mut _blocklist: Option<Vec<IpAddr>>;

    #[cfg(feature = "ip_limit")]
    {
        _allowlist = config.clone().allowlist;
        _blocklist = config.clone().blocklist;
    }

    loop {
        #[allow(unused_mut)]
        let (mut stream, _addr) = listener.accept().await?;

        #[cfg(feature = "ip_limit")]
        {
            if let Some(ref allowlist) = _allowlist {
                if !allowlist.contains(&_addr.ip()) {
                    stream.shutdown().await?;
                    continue;
                }
            }

            if let Some(ref blocklist) = _blocklist {
                if blocklist.contains(&_addr.ip()) {
                    stream.shutdown().await?;
                    continue;
                }
            }
        }

        tokio::spawn(async move {
            let (_status_code, _req) = handle_connection(stream).await.unwrap_or_default();

            #[cfg(feature = "log")]
            {
                let log_level: log::Level = match _status_code {
                    200 => log::Level::Info,
                    400.. => log::Level::Error,
                    _ => log::Level::Warn,
                };

                log!(log_level, "\"{}\" {} - {}", _req, _status_code, _addr);
            }
        });
    }
}

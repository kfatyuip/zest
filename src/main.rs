use tsr::{
    config::{CONFIG, CONFIG_PATH},
    route::{location_index, mime_match},
};

use chrono::{DateTime, Utc};
use clap::Parser;
use mime::Mime;
use std::{collections::HashMap, env, error::Error, ops::Deref, path::Path};

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
    lru::{self, LruCache},
    std::num::NonZeroUsize,
};

use tokio::{
    fs::File,
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
};

static DATE_FORMAT: &str = "%a, %d %b %Y %H:%M:%S GMT";

#[cfg(feature = "lru_cache")]
lazy_static! {
    static ref CACHE: Mutex<LruCache<String, String>> = {
        let cache = LruCache::new(NonZeroUsize::new(8).unwrap());
        Mutex::new(cache)
    };
}

#[derive(Clone)]
struct Response<'a> {
    version: &'a str,
    status_code: i32,
    _headers_buffer: HashMap<&'a str, String>,

    #[cfg(feature = "log")]
    log_level: log::Level,
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
        format!(
            "{} {}",
            status_code,
            match status_code {
                200 => "OK",
                _ => {
                    #[cfg(feature = "log")]
                    {
                        self.log_level = log::Level::Warn;
                    }

                    match status_code {
                        301 => "Moved Permanently",
                        404 => "Not Found",
                        405 => "Method Not Allowed",
                        _ => "Internal Server Error", // 500
                    }
                }
            }
        )
    }
}

async fn handle_connection(mut stream: TcpStream) -> Result<(), Box<dyn Error>> {
    let config = CONFIG.deref();

    let mut response: Response = Response {
        version: "1.1",
        status_code: 200,
        _headers_buffer: HashMap::new(),

        #[cfg(feature = "log")]
        log_level: log::Level::Info,
    };

    let buf_reader = BufReader::new(&mut stream);
    let req = buf_reader.lines().next_line().await?.unwrap_or_default();

    // GET /location HTTP/1.1
    let parts: Vec<&str> = req.split('/').collect();
    let (method, version) = if parts.len() >= 3 {
        (parts[0].trim(), parts[2])
    } else {
        stream.shutdown().await?;
        return Ok(());
    };

    response.version = version;
    let location = &req
        .split_whitespace()
        .nth(1)
        .unwrap()
        .trim_start_matches('/')
        .to_owned();

    let mut mime_type: Mime = mime::TEXT_HTML_UTF_8;
    let mut path = config.server.root.join(location.split('?').next().unwrap());
    let mut buffer: Vec<u8> = Vec::new();

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
                .canonicalize()?
        }
    };

    response.send_header(
        "Server",
        format!("TSR/{} ({})", env!("CARGO_PKG_VERSION"), config.server.info),
    );

    response.send_header("Date", Utc::now().format(DATE_FORMAT));

    if method != "GET" {
        response.status_code = 405;
    } else if cfg!(not(feature = "auto_index"))
        && !path.starts_with(
            config
                .clone()
                .server
                .root
                .canonicalize()
                .expect("bad config path"),
        )
    {
        response.status_code = 301;
    } else {
        if path.is_dir() {
            #[allow(unused_assignments)]
            #[allow(unused_mut)]
            let mut html: String = String::new();
            #[cfg(feature = "lru_cache")]
            {
                let mut cache = CACHE.lock().await;
                if let Some(ctx) = cache.get(location) {
                    html.clone_from(ctx);
                } else {
                    cache
                        .push(location.clone(), location_index(path, location).await?)
                        .to_owned()
                        .unwrap_or_default();
                    html.clone_from(cache.get(location).unwrap());
                }
            }
            #[cfg(not(feature = "lru_cache"))]
            {
                html = location_index(path, location).await?;
            }

            buffer = html.into_bytes();

            response.send_header("Content-Length", buffer.len());
        } else {
            match File::open(path.clone()).await {
                Ok(f) => {
                    let mut file = f;
                    mime_type = mime_match(path.to_str().unwrap());
                    file.read_to_end(&mut buffer).await?;

                    response.send_header("Content-Length", file.metadata().await?.len());
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

        response.send_header("Content-Type", mime_type);
    }

    stream.write_all(response.resp().as_bytes()).await?;
    stream.write_all(&buffer).await?;
    stream.flush().await?;
    stream.shutdown().await?;

    #[cfg(feature = "log")]
    log!(
        response.log_level,
        "\"{}\" {} - {}",
        req,
        response.status_code,
        stream.peer_addr()?.ip()
    );

    Ok(())
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = "config.yaml")]
    config: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    #[cfg(feature = "log")]
    {
        if !log::log_enabled!(log::Level::Info) {
            env::set_var("RUST_LOG", "info");
        }
        env_logger::init();
    }

    let arg = Args::parse();
    *CONFIG_PATH.lock().unwrap() = arg.config;

    let listener = TcpListener::bind(format!("{}:{}", CONFIG.bind.addr, CONFIG.bind.listen))
        .await
        .expect("failed to bind");

    loop {
        #[allow(unused_mut)]
        let (mut stream, addr) = listener.accept().await?;

        if (cfg!(feature = "allow_ip")
            && !CONFIG
                .clone()
                .allowlist
                .unwrap_or_default()
                .contains(&addr.ip()))
            || (cfg!(feature = "block_ip")
                && CONFIG
                    .clone()
                    .blocklist
                    .unwrap_or_default()
                    .contains(&addr.ip()))
        {
            stream.shutdown().await?;
            continue;
        }

        tokio::spawn(async move {
            let _ = handle_connection(stream).await;
        });
    }
}

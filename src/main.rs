use tsr::{
    config::{CONFIG, CONFIG_PATH},
    route::{location_index, mime_match, status_page},
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

const DATE_FORMAT: &str = "%a, %d %b %Y %H:%M:%S GMT";

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

async fn handle_connection(mut stream: TcpStream) -> Result<(i32, String), Box<dyn Error>> {
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
    } else if parts[0].trim() != "GET" {
        response.status_code = 501;
    } else {
        response.version = parts[2];
        let location = &req
            .split_whitespace()
            .nth(1)
            .unwrap()
            .trim_start_matches('/')
            .to_owned();
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
            && !path.starts_with(config.server.root.canonicalize().expect("bad config path"))
        {
            response.status_code = 301;
        } else if path.is_dir() {
            #[allow(unused_assignments)]
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
    }

    if buffer.is_empty() {
        buffer = status_page(
            response.status_code,
            &response.status(response.status_code),
            server_info,
        )
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
            let (_status_code, _req) = handle_connection(stream).await.unwrap_or_default();

            #[cfg(feature = "log")]
            {
                let log_level: log::Level = match _status_code {
                    200 => log::Level::Info,
                    400.. => log::Level::Error,
                    _ => log::Level::Warn,
                };

                log!(log_level, "\"{}\" {} - {}", _req, _status_code, addr);
            }
        });
    }
}

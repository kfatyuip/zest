use tsr::{
    config::CONFIG,
    route::{location_index, mime_match},
};

use chrono::{DateTime, Utc};
use log::log;
use mime::Mime;
use std::{
    collections::HashMap,
    env::{self, current_dir},
    error::Error,
    path::Path,
};

#[macro_use]
#[cfg(feature = "lru_cache")]
extern crate lazy_static;

#[cfg(target_os = "android")]
use std::os::android::fs::MetadataExt;

#[cfg(target_os = "linux")]
use std::os::linux::fs::MetadataExt;

#[cfg(feature = "lru_cache")]
use {
    lru::{self, LruCache},
    std::num::NonZeroUsize,
    async_mutex::Mutex, // faster than tokio::sync::Mutex
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
        format!(
            "HTTP/{} {}\n",
            self.clone().version,
            self.status(self.status_code)
        )
    }
    fn status(&mut self, status_code: i32) -> String {
        format!(
            "{} {}",
            status_code,
            match status_code {
                200 => "OK",
                _ => {
                    self.log_level = log::Level::Warn;
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
    let mut response: Response = Response {
        version: "1.1",
        status_code: 200,
        _headers_buffer: HashMap::new(),
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
    let mut path = current_dir()?.join(location.split('?').next().unwrap());

    let mut buffer: Vec<u8> = Vec::new();

    path = match path.canonicalize() {
        Ok(canonical_path) => canonical_path,
        Err(_) => {
            response.status_code = 404;
            current_dir()?.join(Path::new("404.html")).to_path_buf()
        }
    };

    response.send_header(
        "Server",
        format!("TSR/{}, {}", env!("CARGO_PKG_VERSION"), CONFIG.server.info),
    );

    response.send_header("Date", Utc::now().format(DATE_FORMAT));

    if method != "GET" {
        response.status_code = 405;
    } else if cfg!(not(feature = "auto_index")) && !path.starts_with(current_dir()?) {
        response.status_code = 301;
    } else {
        if path.is_dir() {
            #[allow(unused_mut)]
            let mut html: String;
            #[cfg(feature = "lru_cache")]
            {
                let mut cache = CACHE.lock().await;
                html = cache
                    .get_or_insert(location.clone(), || location_index(path, location))
                    .to_owned();
            }
            #[cfg(not(feature = "lru_cache"))]
            {
                html = location_index(path, location);
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
    for (key, value) in response._headers_buffer.into_iter() {
        stream
            .write_all(format!("{}: {}\r\n", key, value).as_bytes())
            .await?;
    }
    stream.write_all("\r\n".as_bytes()).await?;
    stream.write_all(&buffer).await?;
    stream.flush().await?;
    stream.shutdown().await?;

    log!(
        response.log_level,
        "\"{}\" {} - {}",
        req,
        response.status_code,
        stream.peer_addr()?.ip()
    );

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    if !log::log_enabled!(log::Level::Info) {
        env::set_var("RUST_LOG", "info");
    }
    env_logger::init();

    let listener = TcpListener::bind(format!("{}:{}", CONFIG.bind.host, CONFIG.bind.port)).await?;

    loop {
        #[allow(unused_mut)]
        let (mut stream, _) = listener.accept().await?;

        #[cfg(feature = "allow_ip")]
        if ! CONFIG.clone().allowlist.unwrap().ips.contains(&stream.peer_addr()?.ip()) {
            stream.shutdown().await?
        }

        #[cfg(feature = "block_ip")]
        if CONFIG.clone().blacklist.unwrap().ips.contains(&stream.peer_addr()?.ip()) {
            stream.shutdown().await?;
        }

        tokio::spawn(async move {
            let _ = handle_connection(stream).await;
        });
    }
}

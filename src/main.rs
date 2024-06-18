use tsr::route::{location_index, mime_match};

use chrono::{DateTime, Utc};
use log::log;
use std::{
    collections::HashMap,
    env::{self, current_dir, var},
    error::Error,
    path::Path,
};

#[macro_use]
extern crate lazy_static;

#[cfg(target_os = "android")]
use std::os::android::fs::MetadataExt;

#[cfg(target_os = "linux")]
use std::os::linux::fs::MetadataExt;

#[cfg(feature = "lru_cache")]
use {
    lru::{self, LruCache},
    std::{num::NonZeroUsize, sync::Mutex},
};

use tokio::{
    fs::File,
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
};

static PORT: i32 = 8080;
static DATE_FORMAT: &str = "%a, %d %b %Y %H:%M:%S GMT";

lazy_static! {
    static ref ENCODING: String = {
        let lang = var("LANG").unwrap_or_else(|_| String::from("en_US.UTF-8"));

        lang.split('.').last().unwrap().to_owned()
    };
}

#[cfg(feature = "lru_cache")]
lazy_static! {
    static ref CACHE: Mutex<LruCache<String, String>> = {
        let cache = LruCache::new(NonZeroUsize::new(8).unwrap());
        Mutex::new(cache)
    };
}

struct Response<'a> {
    version: &'a str,
    status: String,
    _headers_buffer: HashMap<&'a str, String>,
}

impl<'a> Response<'a> {
    #[inline]
    fn version(&mut self, v: &'a str) {
        self.version = v;
    }
    #[inline]
    fn send_header(&mut self, k: &'a str, v: String) -> Option<String> {
        self._headers_buffer.insert(k, v)
    }
    #[inline]
    fn resp(&mut self) -> String {
        format!("HTTP/{} {}\n", self.version, self.status)
    }
    fn status(&mut self, status_code: i32) {
        self.status = format!(
            "{} {}",
            status_code,
            match status_code {
                200 => "OK",
                301 => "Moved Permanently",
                404 => "Not Found",
                405 => "Method Not Allowed",
                _ => "Internal Server Error", // 500
            }
        )
    }
}

async fn handle_connection(mut stream: TcpStream) -> Result<(), Box<dyn Error>> {
    let mut response: Response = Response {
        version: "1.1",
        status: "200 OK".to_owned(),
        _headers_buffer: HashMap::new(),
    };

    let mut level = log::Level::Info;

    let buf_reader = BufReader::new(&mut stream);

    let req = buf_reader.lines().next_line().await?.unwrap();
    let mut status_code: i32 = 200;

    // GET /location HTTP/1.1
    let parts: Vec<&str> = req.split('/').collect();
    let (method, version) = if parts.len() >= 3 {
        (parts[0].trim(), parts[2])
    } else {
        return Ok(());
    };
    let location: &str = &req
        .split_whitespace()
        .nth(1)
        .unwrap()
        .trim_start_matches('/');

    let mut _type: String = "text/html".to_owned();
    let mut path = current_dir()?.join(location.split('?').nth(0).unwrap());

    let mut buffer: Vec<u8> = Vec::new();

    path = match path.canonicalize() {
        Ok(canonical_path) => canonical_path,
        Err(_) => {
            status_code = 404;
            level = log::Level::Warn;
            current_dir()?.join(Path::new("404.html")).to_path_buf()
        }
    };

    response.send_header(
        "Server",
        format!("TSR/{}, powered by Rust", env!("CARGO_PKG_VERSION")),
    );

    response.send_header("Date", Utc::now().format(DATE_FORMAT).to_string());

    if method != "GET" {
        status_code = 405;
        level = log::Level::Warn;
    } else if !path.starts_with(current_dir()?) {
        status_code = 301;
        level = log::Level::Warn;
    } else {
        if path.is_dir() {
            #[allow(unused_mut)]
            let mut html: String;
            #[cfg(feature = "lru_cache")]
            {
                let mut cache = CACHE.lock().unwrap();
                if cache.get(&location.to_owned()).is_none() {
                    cache.put(location.to_owned(), location_index(path, location));
                }
                html = cache.get(location).unwrap().to_owned();
            }
            #[cfg(not(feature = "lru_cache"))]
            {
                html = location_index(path, location);
            }

            buffer = html.clone().into_bytes();

            response.send_header("Content-Length", html.len().to_string());
        } else {
            match File::open(path.clone()).await {
                Ok(f) => {
                    let mut file = f;
                    _type = mime_match(path.to_str().unwrap());
                    file.read_to_end(&mut buffer).await?;

                    response
                        .send_header("Content-Length", file.metadata().await?.len().to_string());
                    response.send_header(
                        "Last-Modified",
                        DateTime::from_timestamp(file.metadata().await?.st_atime(), 0)
                            .unwrap()
                            .format(DATE_FORMAT)
                            .to_string(),
                    );
                }
                Err(_) => {
                    status_code = 500;
                    level = log::Level::Warn;
                }
            };
        }

        if _type.starts_with("text/") {
            response.send_header(
                "Content-Type",
                format!("{_type}; charset={}", ENCODING.to_string()),
            );
        } else {
            response.send_header("Content-Type", _type);
        }
    }

    response.version(version);
    response.status(status_code);
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
        level,
        "\"{}\" {} - {}",
        req,
        status_code,
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

    let listener = TcpListener::bind(format!("127.0.0.1:{}", PORT)).await?;

    loop {
        let (stream, _) = listener.accept().await?;

        tokio::spawn(async move {
            let _ = handle_connection(stream).await;
        });
    }
}

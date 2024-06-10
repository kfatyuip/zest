use tsr::route::{extension_match, location_index};

use chrono::{DateTime, Utc};
use log::info;
use std::{
    collections::HashMap,
    env::{self, current_dir, var},
    error::Error,
    os::linux::fs::MetadataExt,
};

use tokio::{
    fs::File,
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
};

static PORT: i32 = 8080;
static DATE_FORMAT: &str = "%a, %d %b %Y %H:%M:%S GMT";

#[inline(always)]
fn get_filesystem_encoding() -> String {
    let lang = var("LANG").unwrap_or_else(|_| String::from("en_US.UTF-8"));

    lang.split('.').last().unwrap().to_owned()
}

struct Response<'a> {
    message: String,
    _headers_buffer: HashMap<&'a str, String>,
}

impl<'a> Response<'a> {
    #[inline(always)]
    fn set_message(&mut self, version: &str, status_code: &str) {
        self.message = format!("HTTP/{} {}\n", version, status_code)
    }
    #[inline(always)]
    fn send_header(&mut self, k: &'a str, v: String) -> Option<String> {
        self._headers_buffer.insert(k, v)
    }
}

async fn handle_connection(mut stream: TcpStream) -> Result<(), Box<dyn Error>> {
    let mut response: Response = Response {
        message: "HTTP/1.1 200 OK".to_owned(),
        _headers_buffer: HashMap::new(),
    };

    let buf_reader = BufReader::new(&mut stream);

    let get: String = buf_reader.lines().next_line().await?.unwrap();
    let mut status_code: &str = "200 OK";

    // GET /location HTTP/1.1
    let method: &str = get.split('/').next().unwrap().trim();

    // support "GET" only
    if method != "GET" {
        return Ok(());
    }

    let version: &str = get.split('/').last().unwrap_or("1.1");
    let location: &str = get.split(' ').nth(1).unwrap().trim_start_matches('/');

    let mut _type: String = "text/html".to_owned();
    let mut _vec: Vec<String> = vec![];
    let path = current_dir()?.join(location.split('?').nth(0).unwrap());

    let mut buffer: Vec<u8> = Vec::new();

    let server_info = format!("TSR/{}, powered by Rust", env!("CARGO_PKG_VERSION"));
    let server_date = Utc::now().format(DATE_FORMAT).to_string();

    response.send_header("Server", server_info);
    response.send_header("Date", server_date);

    if path.is_dir() {
        let html = location_index(path, location);
        buffer = html.clone().into_bytes();
        response.send_header(
            "Content-Type",
            format!("{_type}; charset={}", get_filesystem_encoding()),
        );

        response.send_header("Content-Length", html.len().to_string());
    } else {
        let mut file = match File::open(path.clone()).await {
            Ok(f) => {
                let extension = path.extension().unwrap_or_default().to_str().unwrap();
                _type = extension_match(extension);
                f
            }
            Err(_) => {
                status_code = "404 Not Found";
                _type = "text/html".to_owned();
                match File::open("404.html").await {
                    Ok(f) => f,
                    Err(e) => return Err(e.into()),
                }
            }
        };
        file.read_to_end(&mut buffer).await?;
        if _type.contains("text") {
            response.send_header(
                "Content-Type",
                format!("{_type}; charset={}", get_filesystem_encoding()),
            );
        } else {
            response.send_header("Content-Type", _type);
        }

        response.send_header("Content-Length", file.metadata().await?.len().to_string());
        response.send_header(
            "Last-Modified",
            DateTime::from_timestamp(file.metadata().await?.st_atime(), 0)
                .unwrap()
                .format(DATE_FORMAT)
                .to_string(),
        );
    }

    info!("\"{}\" {}", get, status_code);

    response.set_message(version, status_code);
    stream.write_all(response.message.as_bytes()).await?;
    for (key, value) in response._headers_buffer.into_iter() {
        stream
            .write_all(format!("{}: {}\n", key, value).as_bytes())
            .await?;
    }
    stream.write_all("\r\n".as_bytes()).await?;
    stream.write_all(&buffer).await?;
    stream.flush().await?;

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

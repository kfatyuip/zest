use tsr::route::{extension_match, location_index};

use chrono::{DateTime, Utc};
use std::{
    collections::HashMap,
    env::{current_dir, var},
    error::Error,
    fs::File,
    io::{BufRead, BufReader, Read, Write},
    net::{TcpListener, TcpStream},
    os::linux::fs::MetadataExt,
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

fn handle_connection(mut stream: TcpStream) -> Result<(), Box<dyn Error>> {
    let mut response: Response = Response {
        message: "HTTP/1.1 200 OK".to_owned(),
        _headers_buffer: HashMap::new(),
    };

    let buf_reader = BufReader::new(&mut stream);

    let http_request: Vec<_> = buf_reader
        .lines()
        .map(|result| result.unwrap())
        .take_while(|line| !line.is_empty())
        .collect();

    println!("Request: {:?}", http_request);

    let mut status_code: &str = "200 OK";
    let get: &str = http_request.first().unwrap();

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
    let path = current_dir()
        .unwrap()
        .join(location.split('?').nth(0).unwrap());

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
        let mut file = match File::open(path.clone()) {
            Ok(f) => {
                let extension = path.extension().unwrap_or_default().to_str().unwrap();
                _type = extension_match(extension);
                f
            }
            Err(_) => {
                status_code = "404 Not Found";
                _type = "text/html".to_owned();
                match File::open("404.html") {
                    Ok(f) => f,
                    Err(e) => return Err(e.into()),
                }
            }
        };
        file.read_to_end(&mut buffer)?;
        if _type.contains("text") {
            response.send_header(
                "Content-Type",
                format!("{_type}; charset={}", get_filesystem_encoding()),
            );
        } else {
            response.send_header("Content-Type", _type);
        }

        response.send_header("Content-Length", file.metadata().unwrap().len().to_string());
        response.send_header(
            "Last-Modified",
            DateTime::from_timestamp(file.metadata().unwrap().st_atime(), 0)
                .unwrap()
                .format(DATE_FORMAT)
                .to_string(),
        );
    }

    response.set_message(version, status_code);
    stream.write_all(response.message.as_bytes())?;
    for (key, value) in response._headers_buffer.into_iter() {
        stream.write_all(format!("{}: {}\n", key, value).as_bytes())?;
    }
    stream.write_all("\r\n\r\n".as_bytes())?;
    stream.write_all(&buffer)?;

    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let listener = TcpListener::bind(format!("127.0.0.1:{}", PORT))?;

    for stream in listener.incoming() {
        let stream = stream?;

        let _ = handle_connection(stream);
    }

    Ok(())
}

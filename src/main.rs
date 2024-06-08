use tsr::route::location_index;

use std::{
    env::current_dir,
    error::Error,
    fs::{self, File},
    io::{BufRead, BufReader, Read, Write},
    net::{TcpListener, TcpStream},
    os::linux::fs::MetadataExt,
};

use chrono::{DateTime, Utc};

static PORT: i32 = 8080;
static DATE_FORMAT: &str = "%a, %d %b %Y %H:%M:%S GMT";

fn handle_connection(mut stream: TcpStream) -> Result<(), Box<dyn Error>> {
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

    let mut _header: String = String::new();
    let mut _content = String::new();

    let mut _type: String;
    let mut _vec: Vec<String> = vec![];
    let path = current_dir()
        .unwrap()
        .join(location.split('?').nth(0).unwrap());

    let mut buffer: Vec<u8> = Vec::new();

    if path.is_dir() {
        _type = "text/html".to_owned();
        let paths = fs::read_dir(path.clone())?;
        _vec = vec![];

        for entry in paths {
            let entry = entry?;
            let meta = entry.metadata()?;

            let entry = entry
                .path()
                .strip_prefix(path.clone())
                .unwrap()
                .to_str()
                .unwrap()
                .to_owned();

            if meta.is_file() {
                _vec.push(entry);
            } else {
                let mut _i = entry.clone();
                if meta.is_dir() {
                    _i = format!("{}/", entry);
                } else if meta.is_symlink() {
                    _i = format!("{}@", entry);
                }
                _vec.push(_i);
            }
        }
        _vec.sort();
        let html = location_index(location, _vec);
        _content += &html;
        _header = format!("Content-Length: {}", html.len());
    } else {
        let file = File::open(path.clone());

        let extension = path.extension().unwrap_or_default().to_str().unwrap();
        _type = match extension {
            "jpg" | "png" | "jpeg" | "gif" => format!("image/{extension}"),
            "mp3" | "ogg" | "wav" | "mp4" => format!("audio/{extension}"),
            "txt" | "text" | "toml" | "yaml" | "yml" | "ini" | "xml" | "csv" | "md" | "json" => {
                "text/plain".to_owned()
            }
            "html" | "htm" => "text/html".to_owned(),
            &_ => "application/octet-stream".to_owned(),
        };

        if file.is_ok() {
            let mut file = file.unwrap();
            file.read_to_end(&mut buffer)?;
            _header = format!(
                "Content-Length: {}\nLast-Modified: {}",
                file.metadata().unwrap().len(),
                DateTime::from_timestamp(file.metadata().unwrap().st_mtime(), 0)
                    .unwrap()
                    .format(DATE_FORMAT)
            );
        } else {
            status_code = "404 Not Found";
        }
    }
    let server_info = format!("TSR/{}, powered by Rust", env!("CARGO_PKG_VERSION"));
    let server_date = Utc::now().format(DATE_FORMAT).to_string();

    if _type.contains("text") {
        _type += "; charset=utf-8";
    }

    let header: String = format!(
        "HTTP/{version} {status_code}
Server: {server_info}
Date: {server_date}
Content-type: {_type}
{_header}\r\n\r\n"
    );

    let content = header + &_content;
    stream.write_all(content.as_bytes())?;
    stream.write_all(&buffer)?;

    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let listener = TcpListener::bind(format!("127.0.0.1:{}", PORT))?;

    for stream in listener.incoming() {
        let stream = stream?;

        handle_connection(stream)?;
    }

    Ok(())
}

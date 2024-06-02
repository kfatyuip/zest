use std::{
    env::current_dir,
    error::Error,
    fs::{self, File},
    io::{BufRead, BufReader, Read, Write},
    net::{TcpListener, TcpStream},
};

static PORT: i32 = 8080;

fn plain_html(f: Vec<String>) -> String {
    let mut html: String = "<!DOCTYPE HTML>
<html lang=\"en\">
<head>
<meta charset=\"utf-8\">
<title>Directory listing for /</title>
</head>
<body>
<h1>Directory listing for /</h1>
<hr>
<ul>"
        .to_string();

    for i in f.into_iter() {
        html += &format!("<li><a href=\"{i}\">{i}</a></li>");
    }
    html += "</ul>
<hr>
</body>
</html>";

    return html.clone();
}

fn handle_connection(mut stream: TcpStream) -> Result<(), Box<dyn Error>> {
    let buf_reader = BufReader::new(&mut stream);

    let http_request: Vec<_> = buf_reader
        .lines()
        .map(|result| result.unwrap())
        .take_while(|line| !line.is_empty())
        .collect();

    println!("Request: {:?}", http_request);

    let mut status_code: &str = "200 OK";
    let version: &str = http_request
        .first()
        .unwrap()
        .split('/')
        .last()
        .unwrap_or("1.1");
    let location = http_request
        .first()
        .unwrap()
        .split(' ')
        .nth(1)
        .unwrap()
        .trim_start_matches('/');

    let mut _content = String::new();

    let mut _type: &str;
    let mut _vec: Vec<String> = vec![];
    let path = current_dir().unwrap().join(location);
    if path.is_dir() {
        _type = "html";
        let paths = fs::read_dir(path.clone())?;
        _vec = vec![];

        for entry in paths {
            let entry = entry?;
            let meta = entry.metadata()?;

            if meta.is_dir() {
                let mut _i = format!(
                    "{}/",
                    entry
                        .path()
                        .strip_prefix(path.clone())
                        .unwrap()
                        .to_str()
                        .unwrap()
                );
                _vec.push(_i);
            } else {
                _vec.push(
                    entry
                        .path()
                        .strip_prefix(path.clone())
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .to_string(),
                );
            }
        }

        _content += plain_html(_vec).as_str();
    } else {
        _type = "plain";
        let mut buffer: String = String::new();
        let file = File::open(path);
        if file.is_ok() {
            file.unwrap().read_to_string(&mut buffer)?;
            _content += &buffer.replace("\n", "\r\n");
        } else {
            status_code = "404 Not Found";
        }
    }
    let header: String = format!(
        "HTTP/{version} {status_code}
Content-type: text/{_type}; charset=utf-8\r\n\r\n"
    );

    let content = header + &_content;
    stream.write_all(content.as_bytes())?;

    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let listener = TcpListener::bind(format!("127.0.0.1:{}", PORT)).unwrap();

    for stream in listener.incoming() {
        let stream = stream?;

        handle_connection(stream)?;
    }

    Ok(())
}

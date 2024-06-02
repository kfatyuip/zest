use std::{
    fs,
    io::{BufRead, BufReader, Write},
    net::{TcpListener, TcpStream},
    path::Path,
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

fn handle_connection(mut stream: TcpStream) {
    let buf_reader = BufReader::new(&mut stream);

    let http_request: Vec<_> = buf_reader
        .lines()
        .map(|result| result.unwrap())
        .take_while(|line| !line.is_empty())
        .collect();

    println!("Request: {:?}", http_request);

    let version: &str = http_request
        .first()
        .unwrap()
        .split('/')
        .last()
        .unwrap_or("1.1");
    let location = http_request.first().unwrap().split(' ').nth(1).unwrap();
    let mut content = format!(
        "HTTP/{version} 200 OK
Content-type: text/html; charset=utf-8\r\n\r\n"
    );

    let mut _vec: Vec<String> = vec![];
    if Path::new(location).is_dir() {
        let paths = fs::read_dir(location).unwrap();
        _vec = vec![];

        for entry in paths {
            let entry = entry.unwrap();
            let meta = entry.metadata().unwrap();

            if meta.is_dir() {
                let mut _i = format!(
                    "{}/",
                    entry
                        .path()
                        .strip_prefix(location)
                        .unwrap()
                        .to_str()
                        .unwrap()
                );
                _vec.push(_i);
            } else {
                _vec.push(
                    entry
                        .path()
                        .strip_prefix(location)
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .to_string(),
                );
            }
        }
        content += plain_html(_vec).as_str();
    }
    stream.write_all(content.as_bytes()).unwrap();
}

fn main() {
    let listener = TcpListener::bind(format!("127.0.0.1:{}", PORT)).unwrap();

    for stream in listener.incoming() {
        let stream = stream.unwrap();

        handle_connection(stream);
    }
}

use std::{fs::File, os::linux::fs::MetadataExt};

use chrono::DateTime;

#[inline(always)]
pub fn location_index(location: &str, f: Vec<String>) -> String {
    let mut html: String = format!(
        "<!DOCTYPE HTML>
<html lang=\"en\">
<head>
<meta charset=\"utf-8\">
<title>Directory listing for /{location}</title>
</head>
<body>
<h1>Directory listing for /{location}</h1>
<hr>
<ul>"
    );

    for i in f.into_iter() {
        html += &format!("\n<li><a href=\"{i}\">{i}</a></li>");
    }
    html += "\n</ul>
<hr>
</body>
</html>\n";

    return html.clone();
}

#[inline(always)]
pub fn extension_match(extension: &str) -> String {
    match extension {
        "jpg" | "png" | "jpeg" | "gif" => format!("image/{extension}"),
        "mp3" | "ogg" | "wav" | "mp4" => format!("audio/{extension}"),
        "txt" | "text" | "toml" | "yaml" | "yml" | "ini" | "xml" | "csv" | "md" | "json" => {
            "text/plain".to_owned()
        }
        "html" | "htm" => "text/html".to_owned(),
        &_ => "application/octet-stream".to_owned(),
    }
}

#[inline(always)]
pub fn file_info(file: File, date_format: &str) -> String {
    return format!(
        "Content-Length: {}\nLast-Modified: {}",
        file.metadata().unwrap().len(),
        DateTime::from_timestamp(file.metadata().unwrap().st_atime(), 0)
            .unwrap()
            .format(date_format)
    );
}

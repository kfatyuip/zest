use std::{
    fs::{self, DirEntry, File},
    os::linux::fs::MetadataExt,
    path::PathBuf,
};

use chrono::DateTime;

#[inline(always)]
pub fn location_index(path: PathBuf, location: &str) -> String {
    let entries = fs::read_dir(path.clone()).unwrap();

    let mut entries_vec: Vec<DirEntry> = entries.filter_map(|entry| entry.ok()).collect();
    entries_vec.sort_by(|a, b| {
        a.file_name()
            .to_ascii_lowercase()
            .cmp(&b.file_name().to_ascii_lowercase())
    });
    let mut _vec: Vec<String> = vec![];

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
    for entry in entries_vec {
        let meta = entry.metadata().unwrap();

        let entry = entry
            .path()
            .strip_prefix(path.clone())
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned();

        let mut displayname = entry.clone();
        if meta.is_dir() {
            displayname = format!("{}/", entry);
        } else if meta.is_symlink() {
            displayname = format!("{}@", entry);
        }
        html += &format!("\n<li><a href=\"{entry}\">{displayname}</a></li>");
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

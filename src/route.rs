use crate::config::CONFIG;
use std::{env::current_dir, path::PathBuf};
use tokio::fs::{self, read_dir, DirEntry};

use {mime, mime_guess};

#[inline]
pub async fn location_index(path: PathBuf, location: &str) -> String {
    if path == current_dir().unwrap() {
        if let Some(index) = &CONFIG.server.index {
            return fs::read_to_string(index.clone())
                .await
                .expect("failed to index");
        }
    }

    let mut entries = read_dir(path.clone()).await.unwrap();

    #[allow(unused_mut)]
    let mut entries_vec: Vec<DirEntry> = vec![];

    while let Some(entry) = entries.next_entry().await.unwrap() {
        entries_vec.push(entry);
    }
    #[cfg(feature = "index_sort")]
    {
        entries_vec.sort_by(|a, b| {
            a.file_name()
                .to_ascii_lowercase()
                .cmp(&b.file_name().to_ascii_lowercase())
        });
    }

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
        let meta = entry.metadata().await.unwrap();

        let mut linkname = entry
            .path()
            .strip_prefix(path.clone())
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned();

        let mut displayname = linkname.clone();
        if meta.is_dir() {
            displayname = format!("{}/", linkname);
            linkname = format!("{}/", linkname); // like python
        } else if meta.is_symlink() {
            displayname = format!("{}@", linkname);
        }
        html += &format!("\n<li><a href=\"{linkname}\">{displayname}</a></li>");
    }
    html += "\n</ul>
<hr>
</body>
</html>\n";

    html
}

#[inline]
pub fn mime_match(path: &str) -> mime::Mime {
    mime_guess::from_path(path)
        .first()
        .unwrap_or(mime::APPLICATION_OCTET_STREAM)
}

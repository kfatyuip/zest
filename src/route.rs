use crate::config::CONFIG;
use std::{
    env::current_dir,
    fmt::Write,
    io,
    path::{Path, PathBuf},
};
use tokio::fs::{self, read_dir, DirEntry};

#[inline]
pub async fn location_index(path: PathBuf, location: &str) -> Result<String, io::Error> {
    if path == current_dir().unwrap() {
        if let Some(index) = &CONFIG.server.index {
            return fs::read_to_string(index.clone()).await;
        }
    }

    let mut entries = read_dir(path.clone()).await.unwrap();

    let mut html: String = String::with_capacity(1024);
    html.push_str(&format!(
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
    ));

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

    for entry in entries_vec {
        process_entry(&mut html, &entry, &path).await;
    }

    html.push_str(
        "\n</ul>
<hr>
</body>
</html>\n",
    );

    Ok(html)
}

#[inline]
async fn process_entry(html: &mut String, entry: &DirEntry, path: &Path) {
    let meta = entry.metadata().await.unwrap();
    let mut linkname = entry
        .path()
        .strip_prefix(path)
        .unwrap()
        .display()
        .to_string();

    let displayname = if meta.is_dir() {
        linkname = format!("{}/", linkname);
        linkname.clone()
    } else if meta.is_symlink() {
        format!("{}@", linkname)
    } else {
        linkname.clone()
    };

    write!(
        html,
        "\n<li><a href=\"{linkname}\">{displayname}</a></li>",
        linkname = linkname,
        displayname = displayname
    )
    .unwrap();
}

#[inline]
pub fn mime_match(path: &str) -> mime::Mime {
    mime_guess::from_path(path)
        .first()
        .unwrap_or(mime::APPLICATION_OCTET_STREAM)
}

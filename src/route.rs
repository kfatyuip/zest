use crate::config::{LocationConfig, CONFIG};
use serde_yml::from_value;
use std::{
    fmt::Write,
    io::{ErrorKind, Result},
    ops::Deref,
    path::{Path, PathBuf},
};
use tokio::fs::{self, read_dir, DirEntry};

#[inline]
pub async fn location_index(path: PathBuf, location: &str) -> Result<String> {
    let config = CONFIG.deref();

    if let Some(locatons) = &config.locations {
        for (s, v) in locatons {
            if s.trim_start_matches('/') == location {
                match from_value::<LocationConfig>(v.clone()) {
                    Ok(_location) => {
                        if let Some(index) = _location.index {
                            return fs::read_to_string(index.clone()).await;
                        } else if _location.auto_index.is_none() || !_location.auto_index.unwrap() {
                            return Err(ErrorKind::Unsupported.into());
                        }
                    }
                    _ => {
                        continue;
                    }
                }
            }
        }
    }

    let mut entries = read_dir(path.clone()).await?;

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
<ul>
"
    ));

    #[allow(unused_mut)]
    let mut entries_vec: Vec<DirEntry> = vec![];

    while let Some(entry) = entries.next_entry().await? {
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
        "</ul>
<hr>
</body>
</html>
",
    );

    Ok(html)
}

#[inline]
pub async fn status_page(status: &str, info: String) -> String {
    format!(
        "<html>
<head>
    <title>{status}</title>
</head>
<body>
    <center>
        <h1>{status}</h1>
    </center>
    <hr>
    <center>{info}</center>
</body>
</html>
"
    )
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

    writeln!(
        html,
        "<li><a href=\"{linkname}\">{displayname}</a></li>",
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

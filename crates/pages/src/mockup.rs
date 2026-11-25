//! A mockup as the folder it is: a page (`index.html`) with whatever it
//! uses beside it, and a small file of settings (`mockup.yaml`), each
//! read and written as files (`studio::mockups`, which makes a record of
//! one, offers what is here under the names it had). Nothing of the table
//! it is a record of.

use std::path::{Path, PathBuf};

use kernel::{Error, Result};
use projects::{MAX_FILE_BYTES, resolve};

use crate::document::stamps;

/// The file a folder holds for it to be a mockup: its page.
pub const PAGE: &str = "index.html";

/// The file beside the page that says what is kept about the mockup.
pub const SETTINGS: &str = "mockup.yaml";

/// How many files a mockup may have for a variant to be made of it as a
/// copy, and how much they may come to.
pub const MAX_COPIED: usize = 400;
pub const MAX_COPIED_BYTES: u64 = 64 * 1024 * 1024;

/// Whether a folder's name can be a mockup's: one part of a path, which
/// can be written in an id, and not one that is hidden.
pub fn is_name(name: &str) -> bool {
    !name.is_empty() && !name.starts_with('.') && name != ".." && !name.contains(['/', '\\', ':'])
}

/// What a page says it is called, where it says: the words of its `<title>`.
pub fn titled(page: &str) -> Option<String> {
    let lower = page.to_ascii_lowercase();
    let open = lower.find("<title")?;
    let from = open + lower[open..].find('>')? + 1;
    let to = from + lower[from..].find("</title")?;
    let said = page.get(from..to)?.split_whitespace().collect::<Vec<_>>().join(" ");
    (!said.is_empty()).then_some(said)
}

/// A folder as it was found: its name, its page (for what it is called),
/// its settings where it has any, and when it was made and last changed.
pub struct Found {
    pub name: String,
    pub page: String,
    pub settings: Option<String>,
    pub at: (i64, i64),
}

/// One folder under `root`, if it is a mockup.
pub fn found(root: &Path, name: &str) -> Option<Found> {
    if !is_name(name) {
        return None;
    }
    // Through `resolve`, so that a link in the folder that leads out of it
    // is not taken for a page of it.
    let page = resolve(root, &format!("{name}/{PAGE}")).ok()?;
    let meta = std::fs::metadata(&page).ok().filter(|meta| meta.is_file())?;
    let text = match meta.len() <= MAX_FILE_BYTES {
        true => std::fs::read_to_string(&page).unwrap_or_default(),
        false => String::new(),
    };
    let mut at = stamps(&meta);
    let settings = resolve(root, &format!("{name}/{SETTINGS}")).ok().and_then(|file| {
        let meta = std::fs::metadata(&file).ok().filter(|meta| meta.is_file() && meta.len() <= 64 * 1024)?;
        at.1 = at.1.max(stamps(&meta).1);
        std::fs::read_to_string(&file).ok()
    });
    Some(Found { name: name.to_string(), page: text, settings, at })
}

/// Every mockup under `root`, by name.
pub fn all(root: &Path) -> Vec<Found> {
    let mut names: Vec<String> = std::fs::read_dir(root)
        .into_iter()
        .flatten()
        .flatten()
        // A link to a folder is not a folder of this one.
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names.iter().filter_map(|name| found(root, name)).collect()
}

/// The page a mockup begun from nothing starts as: plain, with what it is
/// called, and everything it needs in itself.
pub fn starting(title: &str) -> String {
    let title = crate::made::escape(title);
    format!(
        "<!doctype html>\n<html lang=\"en\">\n<head>\n  <meta charset=\"utf-8\">\n  <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n  <title>{title}</title>\n  <style>\n    body {{ margin: 0; font: 16px/1.5 system-ui, sans-serif; color: #1c1c1c; background: #fafafa; }}\n    main {{ max-width: 40rem; margin: 0 auto; padding: 3rem 1.25rem; }}\n    h1 {{ font-size: 2rem; margin: 0 0 0.5rem; }}\n    p {{ color: #555; }}\n  </style>\n</head>\n<body>\n  <main>\n    <h1>{title}</h1>\n    <p>A mockup. Change this file, and put what it uses beside it.</p>\n  </main>\n</body>\n</html>\n"
    )
}

/// A folder's name from what a mockup is to be called.
pub fn named_for(title: &str) -> String {
    blueprint::expr::slug(title)
}

/// Copies the files of one mockup's folder into another's, which is not
/// there yet. Links are not followed and not copied; the settings are not
/// copied either, since they are the other mockup's.
pub fn copy(from: &Path, to: &Path) -> Result<()> {
    let (mut pending, mut count, mut bytes) = (vec![PathBuf::new()], 0usize, 0u64);
    while let Some(under) = pending.pop() {
        std::fs::create_dir_all(to.join(&under))?;
        for entry in std::fs::read_dir(from.join(&under))?.flatten() {
            let (kind, name) = (entry.file_type()?, entry.file_name());
            let within = under.join(&name);
            if kind.is_dir() {
                pending.push(within);
            } else if kind.is_file() && !(under.as_os_str().is_empty() && name == SETTINGS) {
                count += 1;
                bytes += entry.metadata()?.len();
                if count > MAX_COPIED || bytes > MAX_COPIED_BYTES {
                    return Err(Error::invalid(format!(
                        "that mockup is too much to copy: more than {MAX_COPIED} files, or more than {} MB. Make the folder by hand, with what the variant needs of it",
                        MAX_COPIED_BYTES / (1024 * 1024)
                    )));
                }
                std::fs::copy(entry.path(), to.join(&within))?;
            }
        }
    }
    Ok(())
}

/// Work on a mockup's files, off the thread that waits for it.
pub async fn blocking<T: Send + 'static>(work: impl FnOnce() -> Result<T> + Send + 'static) -> Result<T> {
    tokio::task::spawn_blocking(work)
        .await
        .map_err(|err| Error::internal(format!("reading the mockups crashed: {err}")))?
}

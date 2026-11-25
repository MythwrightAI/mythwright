//! A project's folder walked as what it holds: every file and folder
//! under it, each by its path with `/` between parts, its size and when
//! it was written, leaving out the folders nothing lists (a repository's
//! own, what a build writes). Functions of the disk with nothing kept,
//! which the studio lists, scans and indexes a project's files by.

use std::path::Path;
use std::time::UNIX_EPOCH;

use serde::Serialize;

/// Directories never listed, scanned or indexed.
pub const IGNORED_DIRS: &[&str] = &[
    ".git",
    ".lanes",
    "target",
    "node_modules",
    "dist",
    "build",
    ".data",
    ".run",
    ".logs",
    ".venv",
    "__pycache__",
];
/// The most entries a listing or scan will return; a project larger than
/// this is listed up to here and says so.
pub const MAX_ENTRIES: usize = 20_000;
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Entry {
    /// Relative to the project, with `/` between parts.
    pub path: String,
    pub name: String,
    pub dir: bool,
    pub size: u64,
    /// Milliseconds since the epoch.
    pub modified: i64,
}

pub fn entry(root: &Path, path: &Path, meta: &std::fs::Metadata) -> Entry {
    let relative = path.strip_prefix(root).unwrap_or(path);
    let modified = meta
        .modified()
        .ok()
        .and_then(|at| at.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_millis() as i64);
    Entry {
        path: relative
            .components()
            .map(|part| part.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/"),
        name: path.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default(),
        dir: meta.is_dir(),
        size: if meta.is_dir() { 0 } else { meta.len() },
        modified,
    }
}

/// What a file of a project is stamped with as it is at this moment, as
/// [`walk`] stamps it; nothing where it is not there, or is a folder.
pub fn stamp_now(root: &Path, path: &str) -> Option<(i64, u64)> {
    let file = root.join(path);
    let meta = std::fs::symlink_metadata(&file).ok().filter(|meta| !meta.is_dir())?;
    let entry = entry(root, &file, &meta);
    Some((entry.modified, entry.size))
}

/// Every file and directory under `root`, skipping what is ignored, sorted
/// with directories before files.
pub fn walk(root: &Path) -> Vec<Entry> {
    let mut found = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        let Ok(listing) = std::fs::read_dir(&dir) else { continue };
        for item in listing.flatten() {
            if found.len() >= MAX_ENTRIES {
                break;
            }
            let path = item.path();
            // Links are listed as what they are, never followed: a link to a
            // parent directory would otherwise never end.
            let Ok(meta) = item.metadata() else { continue };
            let name = item.file_name().to_string_lossy().into_owned();
            if meta.is_dir() {
                if IGNORED_DIRS.contains(&name.as_str()) {
                    continue;
                }
                pending.push(path.clone());
            }
            found.push(entry(root, &path, &meta));
        }
    }
    found.sort_by(|a, b| {
        let key = |entry: &Entry| -> Vec<(bool, String)> {
            let parts: Vec<&str> = entry.path.split('/').collect();
            let last = parts.len() - 1;
            parts
                .iter()
                .enumerate()
                .map(|(index, part)| (index == last && !entry.dir, part.to_string()))
                .collect()
        };
        key(a).cmp(&key(b))
    });
    found
}

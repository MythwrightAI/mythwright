//! A document as the file it is: a Markdown file, with what is kept of
//! it beyond its heading and its body in a block of settings at its head
//! (`studio::documents`, which makes a record of one, offers what is here
//! under the names it had). Functions of a file's text and of the folder
//! it is in, and nothing of the table it is a record of.

use std::path::{Component, Path, PathBuf};
use std::time::UNIX_EPOCH;

use kernel::{Error, Result};
use projects::{MAX_FILE_BYTES, resolve, walk};
use serde_json::{Map, Value, json};

/// What a document's file ends in.
pub const ENDING: &str = ".md";

/// Whether a path within a library is a document's: a Markdown file whose
/// name can be written in an id.
pub fn is_document(path: &str) -> bool {
    path.ends_with(ENDING) && path.len() > ENDING.len() && !path.contains(':')
}

/// A file's name without its ending: what a document with no heading is called.
pub fn stem(path: &str) -> &str {
    let name = path.rsplit('/').next().unwrap_or(path);
    name.strip_suffix(ENDING).unwrap_or(name)
}

/// A document's text as its three parts: the lines of the block of
/// settings at its head, the heading it begins with, and the rest.
pub struct Parts {
    pub head: Vec<String>,
    pub heading: Option<String>,
    pub body: String,
}

pub fn parts(text: &str) -> Parts {
    let text = text.replace("\r\n", "\n");
    let (head, rest) = match text.strip_prefix("---\n").and_then(|after| after.split_once("\n---")) {
        // The closing line is three dashes and nothing else.
        Some((head, rest)) if rest.is_empty() || rest.starts_with('\n') => {
            (head.lines().map(str::to_string).collect(), rest.to_string())
        }
        _ => (Vec::new(), text),
    };
    let rest = rest.trim_start_matches('\n');
    let (heading, body) = match rest.strip_prefix("# ") {
        Some(after) => match after.split_once('\n') {
            Some((heading, body)) => (Some(heading.trim().to_string()), body),
            None => (Some(after.trim().to_string()), ""),
        },
        None => (None, rest),
    };
    Parts { head, heading, body: body.trim().to_string() }
}

/// What a document's own block of settings says of `name`, whether or not
/// its library keeps a field called that: `true` for `public: true`.
pub fn setting(text: &str, name: &str) -> Option<String> {
    let parts = parts(text);
    let line = parts.head.iter().find(|line| named(line) == Some(name))?;
    line.split_once(':').map(|(_, said)| said.trim().trim_matches(['"', '\'']).to_string())
}

/// The lines of a document's own block of settings, as they are written.
pub fn head(text: &str) -> Vec<String> {
    parts(text).head
}

/// The name a line of the block of settings is about.
pub fn named(line: &str) -> Option<&str> {
    line.split_once(':').map(|(name, _)| name.trim())
}

/// What a line of the block says, as a field of the `kind` it is about
/// keeps it. It is read forgivingly: a person wrote it, and `tags: [a, b]` is a list.
pub fn held(kind: &str, written: &str) -> Value {
    let written = written.trim();
    let read: Option<Value> = serde_json::from_str(written).ok();
    let bare = |word: &str| word.trim().trim_matches(['"', '\'']).to_string();
    match (kind, read) {
        ("list" | "records", Some(Value::Array(items))) => Value::Array(items),
        ("list" | "records", _) => {
            let inside = written.strip_prefix('[').and_then(|rest| rest.strip_suffix(']')).unwrap_or(written);
            json!(inside.split(',').map(bare).filter(|word| !word.is_empty()).collect::<Vec<_>>())
        }
        ("number", Some(Value::Number(number))) => Value::Number(number),
        ("flag", Some(Value::Bool(flag))) => Value::Bool(flag),
        ("number" | "flag", _) => Value::Null,
        (_, Some(Value::String(text))) => Value::String(text),
        _ if written.is_empty() => Value::Null,
        _ => Value::String(written.to_string()),
    }
}

/// A value as a line of the block of settings says it.
pub fn line_of(name: &str, value: &Value) -> String {
    let plain = |text: &str| {
        !text.is_empty()
            && text.trim() == text
            && !text.contains(['\n', '"', '#', ':'])
            && serde_json::from_str::<Value>(text).is_err()
    };
    match value {
        Value::String(text) if plain(text) => format!("{name}: {text}"),
        other => format!("{name}: {other}"),
    }
}

pub fn nothing(value: &Value) -> bool {
    match value {
        Value::Null => true,
        Value::String(text) => text.trim().is_empty(),
        Value::Array(items) => items.is_empty(),
        _ => false,
    }
}

/// When a file was made and last written, in milliseconds.
pub fn stamps(meta: &std::fs::Metadata) -> (i64, i64) {
    let millis = |at: std::io::Result<std::time::SystemTime>| {
        at.ok().and_then(|at| at.duration_since(UNIX_EPOCH).ok()).map(|since| since.as_millis() as i64)
    };
    let updated = millis(meta.modified()).unwrap_or(0);
    (millis(meta.created()).unwrap_or(updated), updated)
}

/// Where a new document is to be, from what was said of it: the `path`
/// given, else its title as a file's name.
pub fn path_for(data: &Map<String, Value>) -> Result<String> {
    let said = |name: &str| data.get(name).and_then(Value::as_str).unwrap_or_default().trim();
    let wanted = match said("path") {
        "" => blueprint::expr::slug(said("title")),
        path => path.trim_end_matches('/').to_string(),
    };
    if wanted.is_empty() {
        return Err(Error::invalid("a document needs a title, or a `path`, for its file to be named by"));
    }
    let wanted = if wanted.ends_with(ENDING) { wanted } else { format!("{wanted}{ENDING}") };
    let sound = Path::new(&wanted).components().all(|part| matches!(part, Component::Normal(_)));
    if !sound || !is_document(&wanted) || wanted.contains('\\') {
        return Err(Error::invalid(format!(
            "`{wanted}` is not where a document can be: say a file's name, with the folders it is in before it and `/` between them, such as `guides/install`, and no `:` in it"
        )));
    }
    Ok(wanted)
}

/// The folder a library is the documents of, and every folder under it
/// that is listed: what is followed to be told that a document changed.
pub fn folders(root: &Path) -> Vec<PathBuf> {
    let mut found = vec![root.to_path_buf()];
    found.extend(walk(root).into_iter().filter(|entry| entry.dir).map(|entry| root.join(entry.path)));
    found
}

/// Every document under `root` as its file is now: where it is in the
/// library, and its text. One too large to be a document has no text.
pub fn texts(root: &Path) -> Vec<(String, String, std::fs::Metadata)> {
    let mut found = Vec::new();
    for entry in walk(root).into_iter().filter(|entry| !entry.dir && is_document(&entry.path)) {
        // Through `resolve`, so that a link in the folder that leads out
        // of it is not read as a document of it.
        let Ok(file) = resolve(root, &entry.path) else { continue };
        let Ok(meta) = std::fs::metadata(&file) else { continue };
        let text = match meta.len() <= MAX_FILE_BYTES {
            true => std::fs::read_to_string(&file).unwrap_or_default(),
            false => String::new(),
        };
        found.push((entry.path, text, meta));
    }
    found
}

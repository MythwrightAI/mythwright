//! A path as it is written in an address, and how one page reaches
//! another: functions of plain text, which `studio::site` asks where a
//! library is written out as pages, under the names they had there.

use std::path::{Component, Path};

/// A path as it is written in an address.
pub fn written(path: &str) -> String {
    let part = |part: &str| -> String {
        part.bytes()
            .map(|byte| match byte {
                b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                    (byte as char).to_string()
                }
                other => format!("%{other:02X}"),
            })
            .collect()
    };
    path.split('/').map(part).collect::<Vec<_>>().join("/")
}

/// A path as it was before it was written in an address.
pub fn unwritten(path: &str) -> String {
    let bytes = path.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while at < bytes.len() {
        let hex = |byte: u8| (byte as char).to_digit(16);
        match (bytes[at], bytes.get(at + 1).copied().and_then(hex), bytes.get(at + 2).copied().and_then(hex))
        {
            (b'%', Some(high), Some(low)) => {
                out.push((high * 16 + low) as u8);
                at += 3;
            }
            (other, _, _) => {
                out.push(other);
                at += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// `path` under `folder`, with `.` and `..` worked out. Nothing where it
/// would lead above the top.
pub fn joined(folder: &str, path: &str) -> Option<String> {
    let mut parts: Vec<&str> = folder.split('/').filter(|part| !part.is_empty()).collect();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            other => parts.push(other),
        }
    }
    Some(parts.join("/"))
}

/// How a file at `to` is reached from a page in `from`, both from the top
/// of what is written.
pub fn between(from: &str, to: &str) -> String {
    let from: Vec<&str> = from.split('/').filter(|part| !part.is_empty()).collect();
    let to: Vec<&str> = to.split('/').collect();
    let shared = from.iter().zip(&to).take_while(|(a, b)| a == b).count().min(to.len().saturating_sub(1));
    let mut parts = vec![".."; from.len() - shared];
    parts.extend(&to[shared..]);
    written(&parts.join("/"))
}

/// The folder a file is in, from the top; nothing for one at the top.
pub fn folder_of(path: &str) -> &str {
    path.rsplit_once('/').map_or("", |(folder, _)| folder)
}

/// Whether a link says where on the network it leads: `https:`, `mailto:`.
pub fn scheme(dest: &str) -> Option<&str> {
    let (before, _) = dest.split_once(':')?;
    let sound = before.chars().next().is_some_and(|first| first.is_ascii_alphabetic())
        && before.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'));
    sound.then_some(before)
}

/// A folder of a project as its parts, from the top; nothing where it is
/// not a plain path inside the project.
pub fn parts(folder: &str) -> Option<Vec<String>> {
    let mut parts = Vec::new();
    for part in Path::new(folder).components() {
        match part {
            Component::Normal(part) => parts.push(part.to_string_lossy().into_owned()),
            Component::CurDir => {}
            _ => return None,
        }
    }
    Some(parts)
}

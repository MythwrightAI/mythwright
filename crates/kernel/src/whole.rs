//! A path made whole: every link in it followed, said from the top, and
//! said the way a person would write it.
//!
//! [`Path::canonicalize`] is the system's own answer, and on Windows that
//! answer is in the long form: `\\?\C:\Users\…`, or `\\?\UNC\server\share\…`
//! for a share on another machine. The long form names the same place and
//! is not the same path. It is not equal to the place written plainly
//! (`C:\Users\…`) and is not under it; some programs refuse it as a folder
//! to work in or as an argument; and nobody reads it gladly in a sentence.
//! A fleet said its device was short on room because no volume's `C:\`
//! is the start of `\\?\C:\…`.
//!
//! So nothing in the product asks the system for a whole path but
//! [`whole`], which says it plainly wherever the plain form means the same
//! place, and keeps the long form only where that is the one way to say it
//! ([`plainly`]: the rule, as a function of text, since it is tested where
//! there are no such paths). On every other system it is `canonicalize`
//! and nothing more: `/tmp` and `/var` are links on macOS, and following
//! them is the point.
//!
//! One thing follows from the long form being kept at all. A folder said
//! plainly may hold something that can only be said at length (a path past
//! the system's limit, deep in a project), and to `Path::starts_with` the
//! one is then not under the other. Whether one whole path is under
//! another is asked of [`within`] and [`under`], and whether a folder
//! walked up to is another of [`same`]: they answer alike whichever form
//! either path is in.

use std::io;
use std::path::{Path, PathBuf};

/// How the long form begins.
const LONG: &str = r"\\?\";
/// What comes next in it for a share on another machine.
const SHARE: &str = r"UNC\";
/// A plain path is taken by the system as it is while it is shorter than
/// this, counted as the system counts (in 16-bit units, with the nought
/// that ends it). The figure is the standard library's own
/// (`LEGACY_MAX_PATH`: the least any call of the system's takes, the one
/// that makes a folder): past it the library itself goes over to the long
/// form before it asks the system anything.
const LONGEST: usize = 248;

/// `path` as the system has it, links followed, said plainly where that
/// means the same. Fails as `canonicalize` does: where nothing is there.
pub fn whole(path: &Path) -> io::Result<PathBuf> {
    path.canonicalize().map(said_plainly)
}

/// A whole path that something else gave or kept (the system, a file this
/// program wrote before it said paths plainly), said plainly where that
/// means the same. Only Windows has a long form: anywhere else a name may
/// begin as one does, and the path is left as it is.
pub fn said_plainly(path: PathBuf) -> PathBuf {
    if !cfg!(windows) {
        return path;
    }
    // Text that is not Unicode has no plain form to be sure of.
    match path.to_str().map(plainly) {
        Some(plain) => PathBuf::from(plain),
        None => path,
    }
}

/// A Windows path in the long form, written plainly where the plain form
/// means the same place: `\\?\C:\a\b` is `C:\a\b`, and
/// `\\?\UNC\server\share\a` is `\\server\share\a`. Anything else is given
/// back as it was written: a path that is plain already; a long form that
/// is neither a drive nor a share (a volume by its id, a device); and one
/// the system would read otherwise written plainly, which is so of a path
/// at the system's old limit of length, of a name that ends in a dot or a
/// space (dropped from a plain path), of a name that is a device's (`CON`,
/// `NUL.txt`, `COM1`), and of anything a plain path takes for more than a
/// name (`.`, `..`, `/`, a wildcard).
pub fn plainly(written: &str) -> String {
    let kept = || written.to_string();
    let Some(rest) = written.strip_prefix(LONG) else { return kept() };
    let (plain, names, at_least) = match (rest.strip_prefix(SHARE), rest.as_bytes()) {
        // A share is its machine and its name, and both are there.
        (Some(share), _) => (format!(r"\\{share}"), share, 2),
        (None, [drive, b':', b'\\', ..]) if drive.is_ascii_alphabetic() => (rest.to_string(), &rest[3..], 0),
        _ => return kept(),
    };
    let names: Vec<&str> = if names.is_empty() { Vec::new() } else { names.split('\\').collect() };
    let short = plain.encode_utf16().count() + 1 < LONGEST;
    if short && names.len() >= at_least && names.iter().all(|name| sayable(name)) { plain } else { kept() }
}

/// Whether a name in a plain path is read as the name it is.
fn sayable(name: &str) -> bool {
    let more_than_a_letter = |letter: char| letter.is_control() || r#"<>:"/|?*"#.contains(letter);
    !name.is_empty()
        && !name.ends_with(['.', ' '])
        && !name.contains(more_than_a_letter)
        && !is_a_device(name)
}

/// Whether a plain path would take `name` for a device: by what is before
/// its first dot, spaces at the end of that let be, in either case.
fn is_a_device(name: &str) -> bool {
    let stem = name.split('.').next().unwrap_or(name).trim_end_matches(' ').to_ascii_uppercase();
    let numbered = |kind: &str| {
        let mut number = stem.strip_prefix(kind).unwrap_or_default().chars();
        matches!((number.next(), number.next()), (Some('0'..='9' | '¹' | '²' | '³'), None))
    };
    matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$")
        || numbered("COM")
        || numbered("LPT")
}

/// A Windows path written at length: what [`plainly`] undoes. A plain
/// path from the top of a drive or of a share is put in the long form,
/// with either stroke taken for the one the long form knows; anything
/// else (a path already long, one that does not say where it starts, a
/// device) is given back as it was written.
pub fn at_length(written: &str) -> String {
    if written.starts_with(LONG) {
        return written.to_string();
    }
    let said = written.replace('/', r"\");
    if let [drive, b':', b'\\', ..] = said.as_bytes()
        && drive.is_ascii_alphabetic()
    {
        return format!("{LONG}{}{}", drive.to_ascii_uppercase() as char, &said[1..]);
    }
    match said.strip_prefix(r"\\") {
        Some(share) if !share.starts_with(['.', '?', '\\']) => format!("{LONG}{SHARE}{share}"),
        _ => written.to_string(),
    }
}

/// What a Windows path is under another, whichever form each is written
/// in: the names after `root`'s, or nothing where `path` is not under it.
/// A path is under itself, with no names left. Names are told apart as
/// the system keeps them; which drive, in either case.
pub fn beneath(path: &str, root: &str) -> Option<String> {
    let (path, root) = (at_length(path), at_length(root));
    let names = |text: &str| -> Vec<String> {
        text.split('\\').filter(|name| !name.is_empty()).map(str::to_string).collect()
    };
    let (path, root) = (names(&path), names(&root));
    // After `?`: the drive, or the word for a share.
    let alike = |(at, (ours, theirs)): (usize, (&String, &String))| match at {
        1 => ours.eq_ignore_ascii_case(theirs),
        _ => ours == theirs,
    };
    let under = path.len() >= root.len() && path.iter().zip(&root).enumerate().all(alike);
    under.then(|| path[root.len()..].join(r"\"))
}

/// What `path` is under `root`: the rest of it, as `Path::strip_prefix`
/// gives it, and on Windows the same where one of the two is in the long
/// form and the other is not.
pub fn under(path: &Path, root: &Path) -> Option<PathBuf> {
    if let Ok(rest) = path.strip_prefix(root) {
        return Some(rest.to_path_buf());
    }
    if !cfg!(windows) {
        return None;
    }
    beneath(path.to_str()?, root.to_str()?).map(PathBuf::from)
}

/// Whether `path` is `root` or is inside it.
pub fn within(path: &Path, root: &Path) -> bool {
    under(path, root).is_some()
}

/// Whether two paths are one, whichever form each is in. Two paths that
/// [`whole`] gave for one place are equal as they are; this is for a
/// folder come to by walking up from one, which is still in the form the
/// path it was walked up from had to be in.
pub fn same(path: &Path, other: &Path) -> bool {
    path == other || under(path, other).is_some_and(|rest| rest.as_os_str().is_empty())
}

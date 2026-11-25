//! The file a program's name comes to, where the system does not say.
//!
//! On Unix a program named `tool` is the file `tool` in a folder of the
//! path, and the system finds it. On Windows a program is a file whose
//! name ends as the system's list says (`PATHEXT`: `tool.exe`,
//! `tool.cmd`), and what Rust starts by a name is `tool.exe` and nothing
//! else: a program installed as a script of the system's shell
//! (`tool.cmd`, which is how a great many are installed) is not found,
//! and is said not to be there. So there the file is looked for here,
//! and the whole of it handed over: the standard library starts such a
//! script through the system's shell with quoting of its own, and
//! refuses arguments it cannot write for that shell ([`why_not`]).
//!
//! What finds the file is a function of a name, a list of folders and a
//! list of endings, with a table of cases, and is the same on every
//! system; only [`program`] asks the system anything, and on Unix it
//! hands the name back as it was given. None of this has been run on
//! Windows.

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

/// The endings a program's file has where the system's list says none.
pub const USUAL: &[&str] = &[".COM", ".EXE", ".BAT", ".CMD"];

/// The endings of the system's list as it is written (`PATHEXT`:
/// `.COM;.EXE;.BAT;.CMD`), in its order; the usual ones where it is not
/// set or names none.
pub fn endings(listed: Option<&str>) -> Vec<String> {
    let said: Vec<String> = listed
        .unwrap_or_default()
        .split(';')
        .map(str::trim)
        .filter(|ending| ending.len() > 1 && ending.starts_with('.'))
        .map(str::to_string)
        .collect();
    if said.is_empty() { USUAL.iter().map(|ending| ending.to_string()).collect() } else { said }
}

/// Whether `name` ends in `ending`, whichever case either is written in.
fn ends(name: &str, ending: &str) -> bool {
    let (name, ending) = (name.as_bytes(), ending.as_bytes());
    name.len() > ending.len() && name[name.len() - ending.len()..].eq_ignore_ascii_case(ending)
}

/// The file the program `name` comes to: in the first of `folders` that
/// has one, the first of `endings` there is a file for.
///
/// * A name that already ends as one of the list says is that file and
///   no other: `tool.exe` is never `tool.exe.cmd`.
/// * A name with no such ending is never the file of that very name:
///   beside `tool.cmd` there is often a `tool` that is a script for
///   another shell, which this system cannot start.
/// * A name that says where it is (`C:\tools\tool`) is looked for there
///   and in no folder of the list; one that says so from wherever the
///   program is to run (`bin\tool`) is not looked for at all, since
///   that is not where this is asked.
/// * Nothing found is `None`, and whoever asked hands on the name as it
///   was given: what the system then says of it is the answer.
pub fn file(name: &str, folders: &[PathBuf], endings: &[String]) -> Option<PathBuf> {
    if name.is_empty() {
        return None;
    }
    let whole = endings.iter().any(|ending| ends(name, ending));
    let at = |place: &Path| -> Option<PathBuf> {
        if whole {
            return place.is_file().then(|| place.to_path_buf());
        }
        endings
            .iter()
            .map(|ending| {
                let mut file = place.as_os_str().to_owned();
                file.push(ending);
                PathBuf::from(file)
            })
            .find(|file| file.is_file())
    };
    let named = Path::new(name);
    if named.is_absolute() {
        return at(named);
    }
    if named.components().count() > 1 {
        return None;
    }
    folders.iter().filter(|folder| !folder.as_os_str().is_empty()).find_map(|folder| at(&folder.join(name)))
}

/// The same, from the two lists as a system writes them: the folders of
/// a path (`PATH`) and its endings (`PATHEXT`).
pub fn found(name: &str, path: Option<&OsStr>, pathext: Option<&str>) -> Option<PathBuf> {
    let folders: Vec<PathBuf> = path.map(|path| std::env::split_paths(path).collect()).unwrap_or_default();
    file(name, &folders, &endings(pathext))
}

/// What to start for the program `name`: on Windows the file it comes to
/// along `path` (the path the program is to be started with; this
/// process's own where that says none), and the name itself where no
/// file is found; on every other system the name as it was given, which
/// the system finds.
pub fn program(name: &OsStr, path: Option<&OsStr>) -> OsString {
    #[cfg(windows)]
    {
        let own = std::env::var_os("PATH");
        let path = path.or(own.as_deref());
        let pathext = std::env::var("PATHEXT").ok();
        if let Some(file) = name.to_str().and_then(|name| found(name, path, pathext.as_deref())) {
            return file.into_os_string();
        }
    }
    #[cfg(not(windows))]
    let _ = path;
    name.to_os_string()
}

/// Whether `file` is a script of the Windows shell, by how its name
/// ends: started through that shell, with what it is handed written out
/// for it.
pub fn is_script(file: &Path) -> bool {
    let name = file.to_string_lossy();
    ends(&name, ".cmd") || ends(&name, ".bat")
}

/// Why the file `file` was not started: the system's own words, but for
/// the one refusal those do not explain. A script of the Windows shell
/// is handed its arguments in a line that shell reads, and one that
/// cannot be written there so as to mean only what it says is refused
/// before anything is started.
pub fn why_not(file: &OsStr, why: &std::io::Error) -> String {
    why_not_on(cfg!(windows), file, why)
}

/// The same, said as it is on Windows or as it is elsewhere.
pub fn why_not_on(windows: bool, file: &OsStr, why: &std::io::Error) -> String {
    if windows && why.kind() == std::io::ErrorKind::InvalidInput && is_script(Path::new(file)) {
        return format!(
            "it is a script of the system's shell ({}), which is handed its arguments in a line that shell reads, \
             and one of them cannot be written there so that it means only what it says ({why}). Nothing was \
             started. Hand it that some other way (what it reads, a file, a variable of its environment), or \
             start the program the script itself starts",
            file.to_string_lossy()
        );
    }
    why.to_string()
}

/// That `file` could not be started and why, as a sentence.
pub fn not_started(file: &OsStr, why: &std::io::Error) -> String {
    format!("could not start `{}`: {}", file.to_string_lossy(), why_not(file, why))
}

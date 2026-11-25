//! Where the running program is, and whether that is where it was built.
//!
//! A program built in a checkout can read that checkout's blueprints,
//! packages and client as they are, which is what makes working on them
//! quick: edit, apply again, nothing to build. It may do so only while it
//! is run from where it was built. A copy of it anywhere else is somebody's
//! factory. What that runs is what is built into it and what was put
//! beside it, and nothing done in the checkout afterwards reaches it until
//! it is installed again: a build, an edit, a half-written file.
//!
//! "Where it was built" is told by cargo's own mark and by nothing about
//! the path: the folder the program is in holds `.fingerprint`, which cargo
//! makes in every folder it builds into. So a build folder kept outside the
//! checkout still counts, and a copy of the program in a folder of its own
//! does not, wherever that folder is.
//!
//! It is the standard library alone, and came out of the daemon (`api`),
//! which offers it under the name it always had (`api::place`).

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// What an installed copy says of itself: a file beside the program, one
/// line, written by whatever installed it.
const INSTALLED_NOTE: &str = "installed";

/// The folder the running program is in, with links followed: a command
/// that is a link to the program is the program.
pub fn program_dir() -> Option<PathBuf> {
    static DIR: OnceLock<Option<PathBuf>> = OnceLock::new();
    DIR.get_or_init(|| {
        let program = std::env::current_exe().ok()?;
        let program = kernel::whole::whole(&program).unwrap_or(program);
        program.parent().map(Path::to_path_buf)
    })
    .clone()
}

/// Whether `dir`, the folder a program is in, is one cargo builds into. A
/// test program is a level further in, among `deps`.
fn built_into(dir: &Path) -> bool {
    let among = dir.file_name().and_then(|name| name.to_str());
    let profile = match among {
        Some("deps" | "examples") => dir.parent().unwrap_or(dir),
        _ => dir,
    };
    profile.join(".fingerprint").is_dir()
}

/// The checkout this program was built from, when it is being run from
/// where it was built and the checkout is still there. `None` for a copy
/// anywhere else, which reads nothing from a checkout.
pub fn checkout() -> Option<PathBuf> {
    static ROOT: OnceLock<Option<PathBuf>> = OnceLock::new();
    ROOT.get_or_init(|| {
        if !program_dir().is_some_and(|dir| built_into(&dir)) {
            return None;
        }
        let root = kernel::whole::whole(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).ok()?;
        root.join("brand.toml").is_file().then_some(root)
    })
    .clone()
}

/// In a word, for whoever asks which this is: `checkout` or `installed`.
pub fn runs_from() -> &'static str {
    if checkout().is_some() { "checkout" } else { "installed" }
}

/// The line a copy on its own says of itself beside its program, or
/// nothing for a program run from where it was built, or with none.
fn note() -> Option<String> {
    if checkout().is_some() {
        return None;
    }
    let text = std::fs::read_to_string(program_dir()?.join(INSTALLED_NOTE)).ok()?;
    text.lines().next().map(str::trim).filter(|line| !line.is_empty()).map(str::to_string)
}

/// The version this program says it is: the one every crate here was
/// built as, or for a nightly what its package says it is
/// (`0.1.0-nightly.20261124+abc1234`), which tells one night's from
/// another's and from a pinned release of the same version.
pub fn version() -> String {
    let built = env!("CARGO_PKG_VERSION");
    note().as_deref().and_then(|note| version_in(note, built)).unwrap_or(built).to_string()
}

/// What a package's line says this program's version is: the first of
/// what it says (`<version>-nightly.<day>+<commit>, a nightly, ...`, as
/// the tooling writes it in `xtask::package::note`) where that is a
/// nightly of `built`, and nothing otherwise. A line that says another
/// version than the program was built as is not believed.
pub fn version_in<'a>(note: &'a str, built: &str) -> Option<&'a str> {
    let said = note.split(", ").next()?.trim();
    let rest = said.strip_prefix(built)?.strip_prefix("-nightly.")?;
    let (day, commit) = rest.split_once('+')?;
    let fits = |part: &str, ok: fn(char) -> bool| !part.is_empty() && part.chars().all(ok);
    (day.len() == 8 && fits(day, |c| c.is_ascii_digit()) && fits(commit, |c| c.is_ascii_hexdigit()))
        .then_some(said)
}

/// The same in words, with where: for the daemon to print as it starts, in
/// front of whoever started it.
pub fn said() -> String {
    match (checkout(), program_dir()) {
        (Some(root), _) => format!(
            "run from its checkout at {}: its blueprints, packages and client are read from there as they are",
            root.display()
        ),
        (None, Some(dir)) => {
            let build = note().map(|note| format!(" ({note})")).unwrap_or_default();
            // Not "installed": a package that was unpacked is such a copy
            // too, and nothing installed it or will install it again.
            format!("a copy on its own in {}{build}: it changes only when it is replaced", dir.display())
        }
        (None, None) => "a copy on its own".to_string(),
    }
}

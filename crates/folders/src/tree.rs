//! Copying a folder of files into, and out of, a copy of a factory.
//!
//! A copy of a factory is put somewhere else and used by something else,
//! so what is copied must not lead back to where it came from. A plain
//! copy of a folder does, in three ways, and each is left out here and
//! said to have been:
//!
//! - **A link is copied as a link and never followed**: following one
//!   would copy what it points at, from anywhere on the device. And a link
//!   that points, by its whole path, into the factory being copied is not
//!   copied at all: in the new factory it would still point there, and
//!   what was written through it would be written to the other factory.
//! - **A repository's list of its other working directories**
//!   (`.git/worktrees`) holds the whole paths of directories of the
//!   factory copied. In a copy, tidying that list, or removing a working
//!   directory by it, would be done to those directories. Branches are in
//!   the repository and go with it; the list does not, and neither do the
//!   working directories a lane makes (`.lanes`), which are that list's
//!   other half. A directory whose repository is somewhere else (a `.git`
//!   file that points outside it) would likewise be a second working
//!   directory of the same repository, and goes without that file.
//! - **What is not a file, a folder or a link** (a socket, a pipe, a
//!   device) is something a running program has open, not content.

use std::path::{Component, Path, PathBuf};

use kernel::{Error, Result};
use serde::{Deserialize, Serialize};

/// The most that is said, one by one, of what was left out; past this it
/// is counted.
const SAID: usize = 50;

/// Something that was not copied, and why.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotCopied {
    /// Where it is, under what was being copied.
    pub path: String,
    pub why: String,
}

/// What a copy came to.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Tally {
    pub files: usize,
    pub bytes: u64,
    /// What was left out, up to [`SAID`] of them.
    pub left: Vec<NotCopied>,
    /// How many more were left out than are in `left`.
    pub more: usize,
}

impl Tally {
    fn leave(&mut self, path: String, why: &str) {
        if self.left.len() < SAID {
            self.left.push(NotCopied { path, why: why.to_string() });
        } else {
            self.more += 1;
        }
    }
}

/// How a copy is made: what it must not lead back to, and whether to go on.
pub(crate) struct Copier<'a> {
    /// Directories a link must not point into: the factory being copied.
    pub guarded: &'a [PathBuf],
    /// Asked between files; true stops the copy where it is.
    pub stop: &'a (dyn Fn() -> bool + Sync),
}

fn failed(what: &str, path: &Path, err: std::io::Error) -> Error {
    Error::from(err).context(format!("{what} {}", path.display()))
}

/// Whether `dir` is a repository's own folder, by what one always has.
fn is_repository(dir: &Path) -> bool {
    dir.join("HEAD").is_file() && dir.join("objects").is_dir()
}

/// `path` with its `.` and `..` worked out, without looking at the disk.
fn plain(path: &Path) -> PathBuf {
    let mut plain = PathBuf::new();
    for part in path.components() {
        match part {
            Component::ParentDir => {
                plain.pop();
            }
            Component::CurDir => {}
            other => plain.push(other),
        }
    }
    plain
}

/// Whether a file called `.git` says that its directory's repository is
/// somewhere outside `root`.
fn repository_elsewhere(file: &Path, root: &Path) -> bool {
    let Ok(text) = std::fs::read_to_string(file) else { return false };
    let Some(said) = text.lines().find_map(|line| line.trim().strip_prefix("gitdir:")) else { return false };
    let said = Path::new(said.trim());
    if said.is_absolute() {
        return true;
    }
    let dir = file.parent().unwrap_or(root);
    !plain(&dir.join(said)).starts_with(plain(root))
}

/// Removes whatever is at `target` that is not a real folder, so that
/// nothing is written through a link that was there, and a file that
/// cannot be written over is replaced.
fn clear(target: &Path) -> Result<()> {
    match std::fs::symlink_metadata(target) {
        Ok(meta) if !meta.is_dir() => {
            std::fs::remove_file(target).map_err(|err| failed("replacing", target, err))
        }
        _ => Ok(()),
    }
}

/// Copies one file, with the time it was last changed: what follows a
/// project's files tells a changed file by it.
pub(crate) fn file(from: &Path, to: &Path) -> Result<u64> {
    clear(to)?;
    let bytes = std::fs::copy(from, to).map_err(|err| failed("copying", from, err))?;
    // A file that may not be written to keeps the time of its copying.
    if let (Ok(changed), Ok(copy)) = (
        std::fs::metadata(from).and_then(|meta| meta.modified()),
        std::fs::File::options().write(true).open(to),
    ) {
        let _ = copy.set_modified(changed);
    }
    Ok(bytes)
}

#[cfg(unix)]
fn link(points_at: &Path, to: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(points_at, to)
}

/// Makes `dir` a folder nobody but its owner can look in.
pub(crate) fn private_dir(dir: &Path) -> Result<()> {
    std::fs::create_dir_all(dir).map_err(|err| failed("making", dir, err))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))
            .map_err(|err| failed("closing", dir, err))?;
    }
    Ok(())
}

/// Makes `file` one nobody but its owner can read.
pub(crate) fn private_file(file: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(file, std::fs::Permissions::from_mode(0o600))
            .map_err(|err| failed("closing", file, err))?;
    }
    let _ = file;
    Ok(())
}

impl Copier<'_> {
    /// Copies the folder `from` to `to`, which is made if it is not there.
    /// A `from` that does not exist copies nothing. What is left out is
    /// said in `tally` under `label`, the name of what was being copied.
    pub fn folder(&self, from: &Path, to: &Path, label: &str, tally: &mut Tally) -> Result<()> {
        match std::fs::symlink_metadata(from) {
            Ok(meta) if meta.is_dir() => {}
            Ok(_) => {
                tally.leave(label.to_string(), "it is not a folder");
                return Ok(());
            }
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(err) => return Err(failed("reading", from, err)),
        }
        clear(to)?;
        std::fs::create_dir_all(to).map_err(|err| failed("making", to, err))?;
        let mut pending = vec![(from.to_path_buf(), to.to_path_buf())];
        while let Some((source, target)) = pending.pop() {
            for entry in std::fs::read_dir(&source).map_err(|err| failed("reading", &source, err))? {
                if (self.stop)() {
                    return Err(Error::unavailable("the copy was stopped before it was finished"));
                }
                let entry = entry.map_err(|err| failed("reading", &source, err))?;
                let kind = entry.file_type().map_err(|err| failed("reading", &entry.path(), err))?;
                let (path, name) = (entry.path(), entry.file_name());
                let target = target.join(&name);
                let said = || {
                    let under = path.strip_prefix(from).unwrap_or(&path);
                    format!("{label}/{}", under.to_string_lossy())
                };
                if kind.is_symlink() {
                    self.link(&path, &target, said(), tally)?;
                } else if kind.is_dir() {
                    if name == ".lanes" {
                        tally.leave(
                            said(),
                            "the working directories of lanes belong to the factory they were made in",
                        );
                    } else if name == "worktrees" && is_repository(&source) {
                        tally.leave(
                            said(),
                            "a repository's list of its other working directories names directories of the factory copied",
                        );
                    } else {
                        clear(&target)?;
                        std::fs::create_dir_all(&target).map_err(|err| failed("making", &target, err))?;
                        pending.push((path, target));
                    }
                } else if kind.is_file() {
                    if name == ".git" && repository_elsewhere(&path, from) {
                        tally.leave(said(), "it says this directory's repository is somewhere else, which a copy must not share");
                    } else {
                        tally.bytes += file(&path, &target)?;
                        tally.files += 1;
                    }
                } else {
                    tally.leave(said(), "it is not a file, a folder or a link");
                }
            }
        }
        Ok(())
    }

    #[cfg(unix)]
    fn link(&self, path: &Path, target: &Path, said: String, tally: &mut Tally) -> Result<()> {
        let points_at = std::fs::read_link(path).map_err(|err| failed("reading", path, err))?;
        if points_at.is_absolute()
            && self.guarded.iter().any(|guarded| plain(&points_at).starts_with(guarded))
        {
            tally.leave(said, "it is a link into the factory copied, and would still lead there");
            return Ok(());
        }
        clear(target)?;
        link(&points_at, target).map_err(|err| failed("making the link", target, err))
    }

    #[cfg(not(unix))]
    fn link(&self, _: &Path, _: &Path, said: String, tally: &mut Tally) -> Result<()> {
        // Where a link leads is not looked at where no link is made.
        let _ = self.guarded;
        tally.leave(said, "links are not copied on this system");
        Ok(())
    }
}

//! A path somebody hands over, held to the folder it is to be inside:
//! what every route that reads or writes a project's files asks, and the
//! path as the system has it for as much of it as is there; and a file
//! written there in one step. Functions of a path and the folder, with
//! nothing kept.

use std::path::{Component, Path, PathBuf};

use crate::{Error, Result};

/// The path as the system has it, links followed, for as much of it as is
/// there: what is not there yet is put back on the end as it was written.
pub fn real(path: &Path) -> PathBuf {
    let mut existing = path;
    let mut rest = Vec::new();
    while !existing.exists() {
        match (existing.parent(), existing.file_name()) {
            (Some(parent), Some(name)) => {
                rest.push(name.to_os_string());
                existing = parent;
            }
            _ => return path.to_path_buf(),
        }
    }
    let mut real = crate::whole::whole(existing).unwrap_or_else(|_| existing.to_path_buf());
    real.extend(rest.into_iter().rev());
    real
}

/// Turns a path given by a client into one inside `root`, or refuses it.
pub fn resolve(root: &Path, relative: &str) -> Result<PathBuf> {
    let refuse = |why: &str| Error::invalid(format!("`{relative}` is not a path inside the project: {why}"));
    let given = Path::new(relative);
    if relative.trim().is_empty() {
        return Err(refuse("it is empty"));
    }
    let mut clean = PathBuf::new();
    for part in given.components() {
        match part {
            // However it is written: where names are not told apart by
            // their case, `.GIT` is the same folder, and what is written
            // into it is run the next time git is.
            Component::Normal(name) if name.to_string_lossy().eq_ignore_ascii_case(".git") => {
                return Err(refuse("the repository's own files are off limits"));
            }
            // Some systems drop a dot or a space from the end of a name,
            // so `.git.` is `.git` there and `notes.md ` is `notes.md`.
            Component::Normal(name) if name.to_string_lossy().ends_with(['.', ' ']) => {
                return Err(refuse("a name in it ends with a dot or a space"));
            }
            Component::Normal(name) => clean.push(name),
            Component::CurDir => {}
            Component::ParentDir => return Err(refuse("it may not go up with `..`")),
            Component::RootDir | Component::Prefix(_) => return Err(refuse("it must be relative")),
        }
    }
    let joined = root.join(&clean);
    // A link inside the project may point outside it. Resolve the deepest
    // part that exists and check that it is still inside.
    let real_root = crate::whole::whole(root).map_err(|err| Error::from(err).context(root.display()))?;
    let mut existing = joined.as_path();
    while !existing.exists() {
        existing = existing.parent().ok_or_else(|| refuse("it has no parent"))?;
    }
    let real = crate::whole::whole(existing).map_err(|err| Error::from(err).context(existing.display()))?;
    if !crate::whole::within(&real, &real_root) {
        return Err(refuse("it leads outside through a link"));
    }
    // A link to something that is not there is not there either, as far as
    // the walk above can tell, and writing to it makes what it points at,
    // wherever that is.
    let mut part = joined.as_path();
    while part != root && part.starts_with(root) {
        if part.symlink_metadata().is_ok_and(|meta| meta.file_type().is_symlink()) && !part.exists() {
            return Err(refuse("it goes through a link to something that is not there"));
        }
        let Some(parent) = part.parent() else { break };
        part = parent;
    }
    Ok(joined)
}

/// Writes `content` as the file `target` in one step: under another name
/// beside it, then put in its place.
///
/// Written over where it is, a file is first emptied and then filled, and
/// whatever reads it in between reads it empty or cut short: a library
/// written out at that moment left out the page being saved, since a
/// document with nothing in it does not say it is to be published. Put in
/// place, it is the whole of what it was to whoever has it open, and the
/// whole of what it is to whoever opens it next. It keeps what it may be
/// done with (a script that could be run still can be).
///
/// A link is written through, as it always was: putting a file in its
/// place would make it a file. And where putting it in place is refused
/// (somewhere a file that is open cannot be replaced, a name too long to
/// add to), it is written where it is, which is what was done before.
pub fn put_in_place(target: &Path, content: &[u8]) -> std::io::Result<()> {
    let there = target.symlink_metadata().ok();
    if there.as_ref().is_some_and(|meta| meta.file_type().is_symlink()) {
        return std::fs::write(target, content);
    }
    let name = target.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default();
    let beside = target.with_file_name(format!(".{name}.{:x}.saving", crate::id::random_below(u64::MAX)));
    let placed = std::fs::write(&beside, content).and_then(|()| {
        if let Some(meta) = &there {
            std::fs::set_permissions(&beside, meta.permissions())?;
        }
        std::fs::rename(&beside, target)
    });
    if placed.is_err() {
        let _ = std::fs::remove_file(&beside);
        return std::fs::write(target, content);
    }
    Ok(())
}

//! What is made anew in a project's folder through the daemon: a folder,
//! and a file that is not there yet. Each is refused where anything is
//! there already under its name, so that nothing somebody had is written
//! over by being made, and each is refused where the path leaves the
//! project, as every path of a project's is (`resolve`).

use std::io::Write;

use super::*;

impl Projects {
    /// Makes an empty folder of a project, with the folders it is in, and
    /// says so. A name that is there already, a folder or a file, is
    /// refused.
    pub async fn make_folder(&self, id: &str, path: &str, by: &Principal) -> Result<Entry> {
        by.require("projects:write")?;
        // A folder made where a table keeps its records private is not made by whoever may not read them.
        self.may_read_path(id, path, by).await?;
        let (dir, relative) = (self.roots(id)?, path.to_string());
        let made = blocking(move || {
            let target = within_files(&dir, &relative)?;
            match std::fs::create_dir(&target) {
                Ok(()) => Ok(dir.entry(&target, &std::fs::metadata(&target)?)),
                Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => Err(there(&relative, &target)),
                Err(err) => Err(err.into()),
            }
        })
        .await?;
        let data = json!({ "project": id, "path": made.path, "folders": folders(&made.path) });
        let source = Source::actor(by.actor()).taking(self.takes_to_read(id, &made.path).as_deref());
        let _ = self.journal.emit("project.folder.made", source, data).await;
        Ok(made)
    }

    /// Writes a file that is not there yet, and says it was added: as
    /// [`Projects::write`] does, but refused where anything is there under
    /// its name, so that a new file never takes the place of one somebody
    /// had.
    pub async fn write_new(
        &self,
        id: &str,
        path: &str,
        body: impl AsRef<[u8]>,
        by: &Principal,
    ) -> Result<Entry> {
        by.require("projects:write")?;
        self.may_read_path(id, path, by).await?;
        if body.as_ref().len() as u64 > MAX_RAW_BYTES {
            return Err(Error::invalid("that is too much to write to one file here"));
        }
        let (dir, relative, content) = (self.roots(id)?, path.to_string(), body.as_ref().to_vec());
        let written = blocking(move || {
            let target = within_files(&dir, &relative)?;
            // Made only where nothing is: the system says so in the one step that makes it.
            let mut file = match std::fs::OpenOptions::new().write(true).create_new(true).open(&target) {
                Ok(file) => file,
                Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {
                    return Err(there(&relative, &target));
                }
                Err(err) => return Err(err.into()),
            };
            file.write_all(&content)?;
            file.sync_all()?;
            Ok(dir.entry(&target, &std::fs::metadata(&target)?))
        })
        .await?;
        // What is never listed is written and nothing more, as `write` does.
        if written.path.split('/').any(|part| IGNORED_DIRS.contains(&part)) {
            return Ok(written);
        }
        self.remember(id, &written).await?;
        self.announce(id, &written.path, "added", by).await;
        Ok(written)
    }
}

/// Where something new of a project's is to be made, with the folders it
/// is in made first: refused where it is the project itself, or where one
/// of the folders it would be in is a file.
fn within_files(dir: &Roots, relative: &str) -> Result<PathBuf> {
    let target = dir.resolve(relative)?;
    if dir.is_top(&target) {
        return Err(Error::invalid("that is the project itself"));
    }
    let mut up = target.parent();
    while let Some(folder) = up.filter(|folder| !dir.is_top(folder)) {
        if folder.exists() && !folder.is_dir() {
            let name = dir.called(folder).unwrap_or_else(|| folder.to_string_lossy().replace('\\', "/"));
            return Err(Error::invalid(format!("`{relative}` would be inside `{name}`, which is a file")));
        }
        up = folder.parent();
    }
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)?;
    }
    Ok(target)
}

/// What is said of a name that is taken.
fn there(relative: &str, target: &Path) -> Error {
    let what = if target.is_dir() { "a folder" } else { "a file" };
    Error::conflict(format!(
        "there is {what} called `{relative}` in the project already: call the new one something else"
    ))
}

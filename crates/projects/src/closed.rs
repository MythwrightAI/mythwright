//! What of a project is kept private, and from whom.
//!
//! A table may say what reading its records takes (`reads`, as its
//! blueprint writes it): a scope, beside what reading any record does. It
//! is for what is in a project and is not everybody's who works there: a
//! person's notes, in a project whose agents help with the rest of it.
//!
//! Saying so of the records would mean little if what holds them could be
//! read another way. A table that keeps its records as files (a library's
//! documents, mockups, records kept as files) keeps them in a folder of
//! the project, and whoever may read the project's files could read them
//! there. So the folder is kept with the table: this is where the files of
//! a project are asked about, by whoever hands one out. Which folder a
//! table keeps its records in is the table's (`studio::Collection::folder`),
//! and is handed here as it is declared and read. The same folders
//! are what an agent's wall hides from an agent that does not hold the
//! scope (`agents::walled`), and an event about such a record says what
//! hearing it takes (`kernel::Source::takes`).

use std::path::{Component, Path};

use kernel::{Error, Principal, Result};

use crate::Projects;

/// A folder of a project that is read by whoever holds a scope.
#[derive(Clone, Debug)]
pub(crate) struct Kept {
    /// The table whose records are in it.
    collection: String,
    /// From the top of the project, a name a part; none, for the project itself.
    folder: Vec<String>,
    scope: String,
}

/// A path's parts as they are named, with nothing that only says where to
/// go from. One that climbs is no path of a project's.
fn parts(path: &str) -> Vec<String> {
    Path::new(path)
        .components()
        .filter_map(|part| match part {
            Component::Normal(name) => Some(name.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect()
}

/// Whether the folder at `path` holds the folder `kept`, each written from
/// the top of a project.
pub fn holds(path: &str, kept: &str) -> bool {
    parts(kept).starts_with(&parts(path))
}

impl Projects {
    /// Says what reading the folder `table` of `project` keeps its records
    /// in takes (`reads`); with no scope, or no folder, that nothing is
    /// kept there any more.
    pub fn keep(&self, project: &str, table: &str, folder: Option<String>, reads: Option<&str>) {
        let mut kept = self.kept.lock().expect("projects lock");
        let of_project = kept.entry(project.to_string()).or_default();
        of_project.retain(|one| one.collection != table);
        if let (Some(folder), Some(scope)) = (folder, reads) {
            of_project.push(Kept {
                collection: table.to_string(),
                folder: parts(&folder),
                scope: scope.to_string(),
            });
        }
    }

    /// The folders of a project that are kept from `by`, each from the
    /// top of the project (`""` for the whole of it).
    pub fn kept_from(&self, project: &str, by: &Principal) -> Vec<String> {
        let kept = self.kept.lock().expect("projects lock");
        let of_project = kept.get(project).map(Vec::as_slice).unwrap_or_default();
        of_project.iter().filter(|one| !by.can(&one.scope)).map(|one| one.folder.join("/")).collect()
    }

    /// Whether a path of a project, as it is written, is within a folder
    /// kept from `by`. For what is listed by its name; what is opened is
    /// asked of [`Projects::may_read_path`], which follows links.
    pub fn is_kept_from(&self, project: &str, path: &str, by: &Principal) -> bool {
        self.keeping(project, &parts(path), by).is_some()
    }

    /// What reading the file at `path` takes of anybody, where it is in a
    /// folder a table keeps private: what an event about it says hearing
    /// it takes.
    pub fn takes_to_read(&self, project: &str, path: &str) -> Option<String> {
        let path = parts(path);
        let kept = self.kept.lock().expect("projects lock");
        kept.get(project)?.iter().find(|one| path.starts_with(&one.folder)).map(|one| one.scope.clone())
    }

    fn keeping(&self, project: &str, path: &[String], by: &Principal) -> Option<Kept> {
        let kept = self.kept.lock().expect("projects lock");
        let of_project = kept.get(project)?;
        of_project.iter().find(|one| !by.can(&one.scope) && path.starts_with(&one.folder)).cloned()
    }

    /// Refuses unless `by` may read the file or folder at `path` of a
    /// project, wherever a link there leads: what a table keeps private is
    /// not read by another name.
    pub async fn may_read_path(&self, project: &str, path: &str, by: &Principal) -> Result<()> {
        if self.kept_from(project, by).is_empty() {
            return Ok(());
        }
        let refused = |kept: Kept| {
            Error::denied(format!(
                "`{path}` is kept private in `{project}`: reading the records of `{}` takes `{}`, which {} does not hold",
                kept.collection, kept.scope, by.name
            ))
        };
        let written = parts(path);
        if let Some(kept) = self.keeping(project, &written, by) {
            return Err(refused(kept));
        }
        // And as it is on the disk: the deepest part of it that is there,
        // links followed, with what is not there yet after it.
        let Some(root) = self.dir_of(project).and_then(|root| kernel::whole::whole(&root).ok()) else {
            return Ok(());
        };
        let mut there = root.clone();
        let mut rest: Vec<String> = Vec::new();
        for (index, part) in written.iter().enumerate() {
            let next = there.join(part);
            if !next.exists() {
                rest = written[index..].to_vec();
                break;
            }
            there = next;
        }
        let Ok(real) = kernel::whole::whole(&there) else { return Ok(()) };
        let Some(inside) = kernel::whole::under(&real, &root) else { return Ok(()) };
        let mut really = parts(&inside.to_string_lossy());
        really.extend(rest);
        match self.keeping(project, &really, by) {
            Some(kept) => Err(refused(kept)),
            None => Ok(()),
        }
    }
}

impl Projects {
    /// Whether a project's repository holds anything of `folders`, each
    /// from the top of the project: a file of one that is in a commit, or
    /// staged for one. Whoever is kept from those folders is then kept
    /// from the repository too. Asked of git, with nothing of the
    /// project's run for it; where git cannot say in a few seconds, or at
    /// all, it is taken to hold them.
    pub async fn history_holds(&self, project: &str, folders: &[String]) -> bool {
        let Some(dir) = self.dir_of(project) else { return false };
        if !dir.join(".git").exists() {
            return false;
        }
        // Where nothing is started, git is not asked: taken to hold them.
        if self.is_within() {
            return true;
        }
        // The whole project kept: whatever is in the repository is of it.
        if folders.iter().any(|folder| folder.is_empty()) {
            return true;
        }
        let asks: [&[&str]; 2] =
            [&["ls-files", "--cached", "--"], &["log", "--all", "--reflog", "-n", "1", "--format=%H", "--"]];
        for ask in asks {
            let mut git = tokio::process::Command::from(kernel::git::git(&dir));
            git.args(ask).args(folders).stdin(std::process::Stdio::null()).kill_on_drop(true);
            let answered = tokio::time::timeout(std::time::Duration::from_secs(5), git.output()).await;
            match answered {
                Ok(Ok(said)) if said.status.success() && said.stdout.iter().all(u8::is_ascii_whitespace) => {}
                _ => return true,
            }
        }
        false
    }
}

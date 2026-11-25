//! Where each project keeps blueprints of its own.
//!
//! A project's own blueprints are files the factory keeps for it, in its
//! own data folder and never in the project's: a project is often a
//! repository other people read, and a blueprint somebody wrote for their
//! factory is theirs and not the repository's. That place is
//! `<data>/blueprints/projects/<project>/` ([`kept_at`]), and is said as a
//! path of the project's that begins [`PLACE`] wherever a project's file
//! is named (the routes that read and write one, what is said when one
//! changes, where a fork was written): nothing of the project's own folder
//! is ever called that. Beside it, in the same folder of the factory's, is
//! room for the blueprints the factory has for every project (`shared/`),
//! and what installed packages bring is in a folder of its own
//! (`extensions/`): three layers kept apart, so that a blueprint somebody
//! is handed can later be put in either of the last two with nothing
//! reshaped.
//!
//! A project may also keep blueprints in a folder of its repository, for
//! whoever wants them versioned with its code: it says which
//! ([`Folder::In`]), and that folder (with the project's top) is read
//! before the factory's place, a name with a file in both being refused as
//! two files for one name are. Or it says it keeps none at all
//! ([`Folder::Nowhere`]). Where it says nothing, which is every project of
//! a factory from before that did not say a folder, it keeps them in the
//! factory's place alone, and no file of its repository is read as its
//! own: a checkout of this program's source holds every blueprint that
//! ships under `blueprints/std/`, and read as the project's own each of
//! them was taken for a fork. The one exception is its fleet's blueprint
//! at its top, which a project from before is set going by and is read
//! whatever it says.
//!
//! What a project said is kept in the factory's small durable state
//! under [`NAMESPACE`], a row for each project that said something other
//! than the usual, and held in memory here for the lookups that cannot
//! wait on the database ([`Folders`]), one for the factory, which its
//! projects hold (`studio::Projects::folders`): a project that is made
//! begins with nothing said, and one deleted for good takes what it said
//! with it, each in the write of its row. It is the factory's, as the
//! project's row is: it goes into a copy, and stays where it is when a
//! project is moved to a journal of its own. Changing it is the daemon's
//! to do, which writes the row and says so in one write and then tells
//! what is held here ([`Folders::put`]); this crate says no event.
//!
//! Whatever looks for or writes a project's own blueprint asks where
//! they are through one question, [`Folders::blueprints_of`], or, where
//! all it has is a library, [`Library::folder`](super::Library::folder)
//! (answered from the same), and never writes the folder's name itself.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use journal::Journal;
use serde_json::{Value, json};

/// Where what each project said is kept, by the project's id.
pub const NAMESPACE: &str = "blueprints.folder";

/// The folder of a repository a project from before this program read
/// its own blueprints from where it said nothing: read no longer unless it
/// is said, and where its files are moved into the factory from.
pub const USUAL: &str = "blueprints";

/// The factory's place for a project's own blueprints, as a path of the
/// project's: what a file there is named by, and never a folder of the
/// project's own.
pub const PLACE: &str = "@blueprints";

/// What is kept under [`NAMESPACE`] once a start of this program has
/// settled what a factory from before vouched for of the folder a project
/// that said nothing read its own from (`api::library`): no project's id,
/// which has no line break in it.
pub const SETTLED: &str = "\nkept by the factory";

/// The folder of the factory's data folder that holds what it keeps of
/// blueprints: each project's own under `projects/`, and room beside them
/// for what the factory has for every project.
pub const KEPT: &str = "blueprints";

/// Where the factory keeps `project`'s own blueprints, in its data folder.
pub fn kept_at(data: &std::path::Path, project: &str) -> std::path::PathBuf {
    data.join(KEPT).join("projects").join(project)
}

/// What a path of a project's names in the factory's place: the rest of
/// it, from the place's top, where it begins [`PLACE`].
pub fn in_place(path: &str) -> Option<&str> {
    path.strip_prefix(PLACE).and_then(|rest| rest.strip_prefix('/').or(rest.is_empty().then_some("")))
}

/// Where a project keeps blueprints of its own.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Folder {
    /// In the factory's place alone: nothing was said.
    #[default]
    Usual,
    /// Under this folder of the project's repository, as a path from its
    /// top, and in the factory's place after it.
    In(String),
    /// Nowhere: the project has no blueprints of its own, but for its
    /// fleet's, which it is set going by.
    Nowhere,
}

impl Folder {
    /// Where a file of the project's own is written, as a path of the
    /// project's: the factory's place, whatever folder of its repository
    /// is read besides; nothing where it keeps none.
    pub fn path(&self) -> Option<&str> {
        match self {
            Folder::Usual | Folder::In(_) => Some(PLACE),
            Folder::Nowhere => None,
        }
    }

    /// The folder of the project's repository that is read for its own
    /// blueprints, where it said one.
    pub fn repo(&self) -> Option<&str> {
        match self {
            Folder::In(folder) => Some(folder),
            Folder::Usual | Folder::Nowhere => None,
        }
    }

    /// What is kept for it: nothing for the usual, which is kept as
    /// nothing said.
    pub fn kept(&self) -> Option<Value> {
        match self {
            Folder::Usual => None,
            Folder::In(folder) => Some(json!({ "folder": folder })),
            Folder::Nowhere => Some(json!({ "folder": null })),
        }
    }

    /// What was kept, read back. What cannot be read is the usual.
    pub fn read(kept: &Value) -> Folder {
        match kept.get("folder") {
            Some(Value::String(folder)) if refused(folder).is_none() => Folder::In(folder.clone()),
            Some(Value::Null) => Folder::Nowhere,
            _ => Folder::Usual,
        }
    }

    /// As it is handed to whoever asks: what was said (`usual`, `folder`
    /// or `none`), the folder of the repository (nothing but for a
    /// folder), and where a file of its own is written (nothing for none).
    pub fn shown(&self) -> Value {
        let said = match self {
            Folder::Usual => "usual",
            Folder::In(_) => "folder",
            Folder::Nowhere => "none",
        };
        json!({ "said": said, "folder": self.repo(), "written": self.path() })
    }

    /// What a request asks for: `{ "folder": "<path>" }`, a folder of the
    /// repository; `{ "none": true }`; or `{ "usual": true }`, the
    /// factory's place alone.
    pub fn asked(body: &Value) -> Result<Folder, String> {
        let flag = |name: &str| body.get(name).and_then(Value::as_bool).unwrap_or(false);
        let folder = body.get("folder").and_then(Value::as_str);
        match (folder, flag("none"), flag("usual")) {
            (Some(folder), false, false) => {
                let folder = folder.trim().trim_end_matches('/').to_string();
                if let Some(why) = refused(&folder) {
                    return Err(why);
                }
                Ok(Folder::In(folder))
            }
            (None, true, false) => Ok(Folder::Nowhere),
            (None, false, true) => Ok(Folder::Usual),
            _ => Err("say one of a `folder` (a path within the project's repository), `none`, or `usual` (kept by \
                      the factory)"
                .to_string()),
        }
    }
}

/// Why `folder` cannot be where a project keeps its own blueprints: it is
/// to be a folder within the project's, said as a path from its top with
/// `/` between parts, and not its top itself, which is looked at already.
pub fn refused(folder: &str) -> Option<String> {
    let within = "it is to be a folder within the project's, said from the project's top (`blueprints`, \
                  `tools/blueprints`)";
    if folder.is_empty() || folder == "." {
        return Some(format!(
            "the top of the project is looked at for its own already, and is no folder of \
             blueprints: {within}"
        ));
    }
    if folder.starts_with('/') || folder.starts_with('@') || folder.contains('\\') || folder.contains(':') {
        return Some(format!("`{folder}` is not a path within the project: {within}"));
    }
    if folder.split('/').any(|part| part.is_empty() || part == "." || part == "..") {
        return Some(format!("`{folder}` does not stay within the project: {within}"));
    }
    None
}

/// What each project said of where its own blueprints are, held for the
/// lookups that cannot wait on the database. One for the whole factory.
#[derive(Clone, Default)]
pub struct Folders {
    said: Arc<Mutex<HashMap<String, Folder>>>,
}

impl Folders {
    /// Takes up what the factory's journal keeps: read once, as the
    /// daemon starts.
    pub async fn read(&self, journal: &Journal) {
        for (project, kept) in journal.kv_list(NAMESPACE).await.unwrap_or_default() {
            if project != SETTLED {
                self.put(&project, Folder::read(&kept));
            }
        }
    }

    /// What `project` said: the usual where it said nothing.
    pub fn of(&self, project: &str) -> Folder {
        self.said.lock().expect("folders lock").get(project).cloned().unwrap_or_default()
    }

    /// Where a file of `project`'s own is written, as a path of the
    /// project's (the factory's place); nothing where it keeps none.
    pub fn blueprints_of(&self, project: &str) -> Option<String> {
        self.of(project).path().map(str::to_string)
    }

    /// `project` now says `folder`: told once the write that keeps it is
    /// kept.
    pub fn put(&self, project: &str, folder: Folder) {
        let mut said = self.said.lock().expect("folders lock");
        match folder {
            Folder::Usual => said.remove(project),
            folder => said.insert(project.to_string(), folder),
        };
    }
}

/// What to do to have a file called `file` read as a project's own, for
/// a sentence that says it is not there: where it is written, or that
/// the project keeps none, and the command that has it keep them.
pub fn write_as(folder: &Folder, file: &str, project: Option<&str>) -> String {
    match folder.path() {
        Some(path) => format!("Write it as `{path}/{file}` in the project"),
        None => format!(
            "The project keeps no blueprints of its own, so none of its files is read in its place; to have it \
             keep them: `{cli} project blueprints {project} --usual`",
            cli = brand::CLI,
            project = project.unwrap_or("<project>")
        ),
    }
}

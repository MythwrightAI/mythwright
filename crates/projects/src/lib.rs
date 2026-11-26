//! Projects and their files.
//!
//! This came out of `studio` whole, when that crate was at its ceiling:
//! it asked nothing of the rest but what it can be handed as plain values
//! (what a table keeps private, the record a file is), and `studio` offers
//! it under the names it always had (`studio::projects`, `studio::Projects`,
//! and the Trash's rows and files as `studio::trash`). The records' tables,
//! the Trash's among them, are the studio's: a row of the Trash is written
//! here into a table this crate does not make, as `workspaces` writes a
//! column of `talk`'s.
//!
//! A project is a directory. Its files are the project's resources: docs,
//! blueprints, code. Nothing about a project lives only in the database, so
//! an existing repository can be added as it stands and a project can be
//! opened in any other tool.
//!
//! Changes made through here are announced as they happen. Changes made
//! behind our back (an agent editing files, a build writing output) are found
//! by [`Projects::scan`], which is called at the moments something is likely
//! to have changed (an agent's turn ending, a run finishing) rather than by
//! watching or by a timer: nothing here costs anything while the project is
//! quiet.

pub use kernel::named::{folders, kind};
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use journal::{Journal, optional};
use kernel::{Error, Principal, Result, Source};
use rusqlite::params;
use serde::Serialize;
use serde_json::json;

pub use kernel::given::{put_in_place, real, resolve};
pub use kernel::walked::{Entry, MAX_ENTRIES};
pub use kernel::walked::{IGNORED_DIRS, entry, stamp_now, walk};
use texts::library::folders::PLACE;
mod closed;
pub mod copied;
pub mod events;
mod made;
mod placed;
mod renamed;
mod roots;
pub mod templates;
pub mod trashed;
mod within;
mod writing;
pub use closed::holds;
pub use placed::{Forgetting, Placed, Placing};
pub use renamed::{LONGEST_NAME, named};
pub use roots::Roots;

pub const SCHEMA: &[&str] = &[
    "
    CREATE TABLE projects (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        path TEXT NOT NULL UNIQUE,
        brownfield INTEGER NOT NULL,
        created INTEGER NOT NULL
    );

    CREATE TABLE project_files (
        project TEXT NOT NULL,
        path TEXT NOT NULL,
        mtime INTEGER NOT NULL,
        size INTEGER NOT NULL,
        PRIMARY KEY (project, path)
    ) WITHOUT ROWID;
",
    "
    -- Whether a project's own files may stand in for the blueprints that
    -- ship. One the factory made is; one made of a folder that was already
    -- there is not, until the factory's owner says so.
    ALTER TABLE projects ADD COLUMN trusted INTEGER NOT NULL DEFAULT 0;
    UPDATE projects SET trusted = 1 WHERE brownfield = 0;
",
];

/// The files a project's fleet blueprint may be, at its top and in the
/// factory's place for its own.
fn fleet_files() -> Vec<String> {
    let files = brand::blueprint_files("fleet");
    files.iter().cloned().chain(files.iter().map(|file| format!("{PLACE}/{file}"))).collect()
}

/// Above this many changes a scan reports one summary event instead of one
/// event per file.
const MAX_CHANGE_EVENTS: usize = 200;
/// The largest file that can be read as text through here.
pub const MAX_FILE_BYTES: u64 = 2 * 1024 * 1024;
/// The largest file handed over, or written, as it is: a picture, say.
pub const MAX_RAW_BYTES: u64 = 32 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Project {
    /// A slug of the name, used everywhere a project is referred to.
    pub id: String,
    pub name: String,
    pub path: String,
    /// The directory already had files when it was added; adding it wrote nothing to it.
    pub brownfield: bool,
    /// Its own files may stand in for the blueprints that ship. A project
    /// made of a folder that was already there starts without this: what
    /// is in it was written by whoever wrote the folder.
    pub trusted: bool,
    pub created: i64,
    /// Whose its folder is: [`GIVEN`], one whoever added it said, or
    /// [`OWN`], one the factory made for it where it keeps the projects
    /// it makes. Worked out from where the folder is each time a project
    /// is handed out, and kept nowhere.
    pub folder: &'static str,
    /// Whether its folder is a git repository of its own, which a page of
    /// its repository is offered for. Looked at each time it is handed out,
    /// as `folder` is, and kept nowhere.
    pub repository: bool,
}

/// A project's folder is one whoever added it said: a repository, or
/// files that were already somewhere.
pub const GIVEN: &str = "given";
/// A project's folder is one the factory made for it, since nobody said
/// one: where its documents and blueprints are kept, and nothing anybody
/// asked to have.
pub const OWN: &str = "own";

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Change {
    pub path: String,
    /// `added`, `modified` or `removed`.
    pub change: &'static str,
}

/// Whoever knows which directories on this device are already somebody's.
///
/// A project is a directory, and nothing here can see who else on the
/// device works in one: another installation with its own database could
/// adopt the same directory, and the two would then share its files. So
/// before a directory becomes a project's, whoever does know is asked, and
/// may refuse; and is told when a project is forgotten.
pub trait Holdings: Send + Sync {
    /// Asked before `dir` becomes a project's. An error refuses it, and is
    /// what the person adding the project reads.
    fn take(&self, dir: &Path) -> Result<()>;
    /// `dir` is no longer a project's here.
    fn give_back(&self, dir: &Path);
}

#[derive(Clone)]
pub struct Projects {
    pub(crate) journal: Journal,
    /// Where new projects are created unless a path is given.
    pub(crate) root: PathBuf,
    /// Project directories, for the lookups that cannot wait on the database.
    pub(crate) dirs: Arc<Mutex<HashMap<String, PathBuf>>>,
    /// Who is asked before a directory is taken, when anyone is.
    pub(crate) holdings: Arc<Mutex<Option<Arc<dyn Holdings>>>>,
    /// The projects whose own files may stand in for what ships, for the
    /// lookups that cannot wait on the database.
    pub(crate) trusted: Arc<Mutex<std::collections::HashSet<String>>>,
    /// The factory's own data folder, which no project may be made over or
    /// inside: every credential it has is a file there.
    pub(crate) data: Arc<Mutex<Option<PathBuf>>>,
    /// The folders of each project that a table keeps private, for the
    /// lookups that cannot wait on the database (`closed`).
    pub(crate) kept: Arc<Mutex<HashMap<String, Vec<closed::Kept>>>>,
    /// What is written with a project's row as it is made and as it is
    /// forgotten, by what keeps where things belong.
    pub(crate) placed: placed::Placed,
    /// Whether nothing is reached but what this was handed (`Projects::within`).
    pub(crate) within: Arc<std::sync::atomic::AtomicBool>,
    /// The files being written through here, which a look leaves alone (`writing`).
    pub(crate) writing: writing::Writing,
}

pub const PROJECT: &str = "id, name, path, brownfield, created, trusted";

pub fn read_project(row: &rusqlite::Row<'_>) -> rusqlite::Result<Project> {
    Ok(Project {
        id: row.get(0)?,
        name: row.get(1)?,
        path: row.get(2)?,
        brownfield: row.get::<_, i64>(3)? != 0,
        created: row.get(4)?,
        trusted: row.get::<_, i64>(5)? != 0,
        // Said by whoever hands the project out, who knows where the factory keeps its own (`Projects::whose`).
        folder: GIVEN,
        repository: false,
    })
}

pub async fn blocking<T: Send + 'static>(work: impl FnOnce() -> Result<T> + Send + 'static) -> Result<T> {
    tokio::task::spawn_blocking(work)
        .await
        .map_err(|err| Error::internal(format!("a file operation crashed: {err}")))?
}

impl Projects {
    pub async fn open(journal: Journal, root: impl Into<PathBuf>) -> Result<Projects> {
        journal.db().migrate("projects", SCHEMA).await?;
        // A project's tables, and what a scan last saw of its files, are
        // kept in its own journal where that keeps them, and such a
        // journal is made with the tables for them: whoever opens this
        // says so to the journal next (`studio::apart::makes`), before any
        // project can be added.
        let projects = Projects {
            journal,
            root: root.into(),
            dirs: Arc::default(),
            holdings: Arc::default(),
            trusted: Arc::default(),
            data: Arc::default(),
            kept: Arc::default(),
            placed: placed::Placed::default(),
            within: Arc::default(),
            writing: writing::Writing::default(),
        };
        for project in projects.list().await? {
            if project.trusted {
                projects.trusted.lock().expect("projects lock").insert(project.id.clone());
            }
            projects.dirs.lock().expect("projects lock").insert(project.id, PathBuf::from(project.path));
        }
        Ok(projects)
    }

    /// Says where the factory keeps what is its own, so that no project is
    /// made over that folder or inside it.
    pub fn keeps_out_of(&self, data: &Path) {
        *self.data.lock().expect("projects lock") = Some(data.to_path_buf());
    }

    /// Whether a project's own files may stand in for the blueprints that
    /// ship, without touching the database.
    pub fn is_trusted(&self, id: &str) -> bool {
        self.trusted.lock().expect("projects lock").contains(id)
    }

    /// Says whether a project's own files may stand in for the blueprints
    /// that ship. The owner's to say, and nobody else's: from then on a
    /// file in that project called `std/board` is what `std/board` means
    /// there, with the authority of whoever applies it.
    pub async fn trust(&self, id: &str, trusted: bool, by: &Principal) -> Result<Project> {
        by.require("admin")?;
        self.dir(id)?;
        let lookup = id.to_string();
        self.journal
            .db()
            .call(move |connection| {
                connection.execute("UPDATE projects SET trusted = ?2 WHERE id = ?1", params![lookup, trusted])
            })
            .await?;
        {
            let mut held = self.trusted.lock().expect("projects lock");
            if trusted {
                held.insert(id.to_string());
            } else {
                held.remove(id);
            }
        }
        let (said, of) = (Source::actor(by.actor()), json!({ "project": id }));
        let _ = if trusted {
            self.journal.emit("project.trusted", said, of).await
        } else {
            self.journal.emit("project.untrusted", said, of).await
        };
        self.get(id).await
    }

    /// Has `holdings` asked before any directory becomes a project's from
    /// here on, and told when one stops being.
    pub fn held_through(&self, holdings: Arc<dyn Holdings>) {
        *self.holdings.lock().expect("projects lock") = Some(holdings);
    }

    pub(crate) fn holdings(&self) -> Option<Arc<dyn Holdings>> {
        self.holdings.lock().expect("projects lock").clone()
    }

    /// Where a project's files are, without touching the database.
    pub fn dir_of(&self, id: &str) -> Option<PathBuf> {
        self.dirs.lock().expect("projects lock").get(id).cloned()
    }

    pub(crate) fn dir(&self, id: &str) -> Result<PathBuf> {
        self.dir_of(id).ok_or_else(|| Error::not_found(format!("there is no project `{id}`")))
    }

    /// Adds a project. With no `path` it is created under the projects
    /// directory and given a starting set of files. With a `path` that
    /// already has files in it, the directory is adopted exactly as it is.
    pub async fn create(&self, name: &str, path: Option<PathBuf>, by: &Principal) -> Result<Project> {
        self.create_with(name, path, by, true).await
    }

    /// The same, saying whether a project that is made here is given its
    /// first documents (`docs/overview.md`, `docs/architecture.md`): a
    /// project whoever adds it wants no library for begins without them.
    pub async fn create_with(
        &self,
        name: &str,
        path: Option<PathBuf>,
        by: &Principal,
        documents: bool,
    ) -> Result<Project> {
        self.create_in(name, path, by, documents, None).await
    }

    /// The same, saying which workspace it is to be in, by its id. With
    /// none said, or one that is not there, it is in the usual one
    /// (`workspaces`): its place is written with its row.
    pub async fn create_in(
        &self,
        name: &str,
        path: Option<PathBuf>,
        by: &Principal,
        documents: bool,
        workspace: Option<&str>,
    ) -> Result<Project> {
        by.require("projects:write")?;
        let name = name.trim().to_string();
        let id = blueprint::expr::slug(&name);
        if id.is_empty() {
            return Err(Error::invalid("a project needs a name with at least one letter or digit"));
        }
        // Its address is `/<id>`, which is never one of the factory's own (`kernel::paths`).
        if let Some(why) = kernel::paths::refused(&id) {
            return Err(Error::invalid(why));
        }
        if self.dir_of(&id).is_some() {
            return Err(Error::conflict(format!("there is already a project called `{id}`")));
        }
        let dir = match path {
            Some(path) if path.is_absolute() => {
                // A folder that is already somewhere is whoever's it is:
                // taking one up puts every file in it within reach of
                // anyone who may read a project.
                by.require("admin")?;
                self.outside_the_factory(&path)?;
                // Said here, in words: making the folder would only say that something is there.
                if path.is_file() {
                    return Err(Error::invalid(format!(
                        "{} is a file, and a project is a folder. Say the folder it is in, or leave the path \
                         out and the project is made where the factory keeps projects",
                        path.display()
                    )));
                }
                path
            }
            Some(_) => return Err(Error::invalid("a project's path must be absolute")),
            None => self.root.join(&id),
        };
        let (scaffold_dir, scaffold_name) = (dir.clone(), name.clone());
        let (holdings, within) = (self.holdings(), self.is_within());
        let brownfield = blocking(move || {
            // Before anything is written: a directory that is refused is left as it was.
            if let Some(holdings) = holdings {
                holdings.take(&scaffold_dir)?;
            }
            std::fs::create_dir_all(&scaffold_dir)?;
            let has_files = std::fs::read_dir(&scaffold_dir)?.next().is_some();
            if has_files {
                return Ok(true);
            }
            for (path, content) in templates::project(&scaffold_name, documents) {
                let target = scaffold_dir.join(path);
                if let Some(parent) = target.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::write(target, content)?;
            }
            // A new project is a repository from the start, so that lanes
            // can branch it. Not having git is not an error.
            if within {
                return Ok(false);
            }
            let _ = kernel::git::git(&scaffold_dir)
                .args(["init", "--quiet", "--initial-branch=main"])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status();
            Ok(false)
        })
        .await?;

        let project = Project {
            id: id.clone(),
            name,
            path: dir.to_string_lossy().into_owned(),
            brownfield,
            // What the factory wrote itself it can stand behind; a folder
            // that was already there was written by somebody else.
            trusted: !brownfield,
            created: self.journal.now_ms(),
            folder: Self::folder_of(&real(&self.root), &dir),
            repository: dir.join(".git").exists(),
        };
        // A project has a journal of its own from before anything is
        // said or kept of it, where the factory keeps its projects'
        // journals apart (`journal::Journal::own`). One whose tables
        // the factory's own journal holds (a project of a factory from
        // before, taken away and added again) is given one that keeps
        // what is run for it and no more: its rows stay where they
        // are, whole, until its owner has the project moved apart
        // (`studio::apart::makes`).
        self.journal.own(&id).await?;
        let (row, placing, asked) = (project.clone(), self.placed.anew(), workspace.map(str::to_string));
        let placer = Source::actor(by.actor());
        let written = self.journal.write(move |writing| {
            writing.transaction().execute(
                "INSERT INTO projects (id, name, path, brownfield, created, trusted) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![row.id, row.name, row.path, row.brownfield, row.created, row.trusted],
            )?;
            // And its place, with what is said of that: all of it or none (`placed`).
            placing(writing, &row.id, asked.as_deref(), &placer)
        });
        written.await.map_err(|err| match err.kind {
            kernel::ErrorKind::Conflict => Error::conflict(format!("{} is already a project", dir.display())),
            _ => err,
        })?;
        self.dirs.lock().expect("projects lock").insert(id.clone(), dir);
        self.placed.said_nothing(&id);
        if project.trusted {
            self.trusted.lock().expect("projects lock").insert(id.clone());
        }
        // Record what is there now, so the first scan reports only what changes after.
        self.snapshot(&id, false).await?;
        let data =
            json!({ "project": id, "name": project.name, "path": project.path, "brownfield": brownfield });
        let _ = self.journal.emit("project.created", Source::actor(by.actor()), data).await;
        Ok(project)
    }

    /// Refuses a folder to be taken up as a project when it holds the
    /// factory's own data or is inside it. The one place inside that a
    /// project may be is where the factory makes them itself.
    fn outside_the_factory(&self, path: &Path) -> Result<()> {
        let Some(data) = self.data.lock().expect("projects lock").clone() else { return Ok(()) };
        let (path, data, made) = (real(path), real(&data), real(&self.root));
        if kernel::whole::within(&path, &made) && path != made {
            return Ok(());
        }
        if kernel::whole::within(&data, &path) {
            return Err(Error::invalid(format!(
                "{} holds this factory's own data ({}), and a project there would put every credential \
                 the factory has among its files. Choose a folder that is the project's alone",
                path.display(),
                data.display()
            )));
        }
        if kernel::whole::within(&path, &data) {
            return Err(Error::invalid(format!(
                "{} is inside this factory's own data ({}), where its credentials are kept. Choose a \
                 folder outside it, or leave the path out and the project is made where the factory \
                 keeps projects",
                path.display(),
                data.display()
            )));
        }
        Ok(())
    }

    /// The journal whose database keeps what a scan last saw of
    /// `project`'s files: the one that keeps the project's tables
    /// ([`Journal::records_of`]).
    fn files(&self, project: &str) -> Journal {
        self.journal.records_of(Some(project))
    }

    /// Whose a folder is: the factory's own where it is inside the folder
    /// the factory makes projects in (`made`), and otherwise one somebody
    /// gave. It goes by where the folder is and by nothing kept, so a
    /// project of a factory from before is answered for as one made now
    /// is; and a folder somebody named that is in there reads as the
    /// factory's own, which is where the factory would have made it.
    fn folder_of(made: &Path, dir: &Path) -> &'static str {
        let dir = real(dir);
        if kernel::whole::within(&dir, made) && !kernel::whole::same(&dir, made) { OWN } else { GIVEN }
    }

    /// A project as it was read, saying whose its folder is.
    fn whose(made: &Path, mut project: Project) -> Project {
        project.folder = Self::folder_of(made, Path::new(&project.path));
        project.repository = Path::new(&project.path).join(".git").exists();
        project
    }

    pub async fn list(&self) -> Result<Vec<Project>> {
        let read: Vec<Project> = self
            .journal
            .db()
            .call(|connection| {
                let mut statement =
                    connection.prepare(&format!("SELECT {PROJECT} FROM projects ORDER BY created"))?;
                let rows = statement.query_map([], read_project)?;
                rows.collect()
            })
            .await?;
        let made = real(&self.root);
        Ok(read.into_iter().map(|project| Self::whose(&made, project)).collect())
    }

    pub async fn get(&self, id: &str) -> Result<Project> {
        let lookup = id.to_string();
        self.journal
            .db()
            .call(move |connection| {
                optional(connection.query_row(
                    &format!("SELECT {PROJECT} FROM projects WHERE id = ?1"),
                    [&lookup],
                    read_project,
                ))
            })
            .await?
            .map(|project| Self::whose(&real(&self.root), project))
            .ok_or_else(|| Error::not_found(format!("there is no project `{id}`")))
    }

    /// Forgets a project. Its files are left exactly where they are. It
    /// is kept in the Trash, where the factory keeps what is deleted, to
    /// be given back: what it is there is given back.
    pub async fn forget(&self, id: &str, by: &Principal) -> Result<Option<String>> {
        by.require("projects:write")?;
        self.dir(id)?;
        // The project first, and then what a scan last saw of its files,
        // which may be another file's: stopped between the two, the
        // project is gone, and what is left is written over by the look
        // at its files if it is ever added again.
        // With what was placed by it, in the one write: its channels are
        // somewhere from the moment it is gone.
        // Into the Trash, where the factory keeps what is deleted: with its
        // row and its workspace, read in the write before either goes.
        let trashed = match crate::trashed::keeping(&self.journal).await.at_once() {
            true => None,
            false => Some((self.journal.new_id("trash"), self.journal.now_ms(), by.actor())),
        };
        let trash = trashed.as_ref().map(|(trash, ..)| trash.clone());
        let (lookup, forgetting, forgetter) =
            (id.to_string(), self.placed.forgotten(), Source::actor(by.actor()));
        let written = self.journal.write(move |writing| {
            if let Some(trashed) = &trashed {
                crate::trashed::put_project(writing, &lookup, trashed)?;
            }
            forgetting(writing, &lookup, &forgetter, trashed.is_none())?;
            writing.transaction().execute("DELETE FROM projects WHERE id = ?1", [&lookup])?;
            Ok(())
        });
        written.await?;
        match &trash {
            None => self.placed.said_nothing(id),
            // Its own blueprints go with it, to be given back with it.
            Some(trash) => self.carry_place(id, trash, true)?,
        }
        let lookup = id.to_string();
        self.files(id)
            .db()
            .call(move |connection| {
                connection.execute("DELETE FROM project_files WHERE project = ?1", [&lookup])
            })
            .await?;
        let dir = self.dirs.lock().expect("projects lock").remove(id);
        self.trusted.lock().expect("projects lock").remove(id);
        if let (Some(holdings), Some(dir)) = (self.holdings(), dir) {
            let _ = blocking(move || {
                holdings.give_back(&dir);
                Ok(())
            })
            .await;
        }
        let _ =
            self.journal.emit("project.removed", Source::actor(by.actor()), json!({ "project": id })).await;
        Ok(trash)
    }

    /// Every file and directory of a project.
    pub async fn tree(&self, id: &str) -> Result<Vec<Entry>> {
        let dir = self.dir(id)?;
        blocking(move || Ok(walk(&dir))).await
    }

    /// One file's entry: its size and when it last changed.
    pub async fn stat(&self, id: &str, path: &str) -> Result<Entry> {
        let (dir, path) = (self.roots(id)?, path.to_string());
        blocking(move || {
            let target = dir.resolve(&path)?;
            let meta =
                std::fs::metadata(&target).map_err(|_| Error::not_found(format!("there is no `{path}`")))?;
            Ok(dir.entry(&target, &meta))
        })
        .await
    }

    pub async fn read(&self, id: &str, path: &str) -> Result<String> {
        let (dir, path) = (self.roots(id)?, path.to_string());
        blocking(move || {
            let target = dir.resolve(&path)?;
            let meta = std::fs::metadata(&target)
                .map_err(|_| Error::not_found(format!("there is no file `{path}`")))?;
            if meta.is_dir() {
                return Err(Error::invalid(format!("`{path}` is a directory")));
            }
            if meta.len() > MAX_FILE_BYTES {
                return Err(Error::invalid(format!(
                    "`{path}` is too large to open here ({} bytes)",
                    meta.len()
                )));
            }
            let bytes = std::fs::read(&target)?;
            String::from_utf8(bytes).map_err(|_| Error::invalid(format!("`{path}` is not text")))
        })
        .await
    }

    /// A file as it is on disk, whatever is in it: for showing a picture,
    /// or handing over something that is not text.
    pub async fn read_bytes(&self, id: &str, path: &str) -> Result<Vec<u8>> {
        let (dir, path) = (self.roots(id)?, path.to_string());
        blocking(move || {
            let target = dir.resolve(&path)?;
            let meta = std::fs::metadata(&target)
                .map_err(|_| Error::not_found(format!("there is no file `{path}`")))?;
            if meta.is_dir() {
                return Err(Error::invalid(format!("`{path}` is a directory")));
            }
            if meta.len() > MAX_RAW_BYTES {
                return Err(Error::invalid(format!(
                    "`{path}` is too large to hand over here ({} bytes)",
                    meta.len()
                )));
            }
            Ok(std::fs::read(&target)?)
        })
        .await
    }

    /// Writes a file, text or not, byte for byte, creating its directories, and announces the change.
    pub async fn write(&self, id: &str, path: &str, body: impl AsRef<[u8]>, by: &Principal) -> Result<Entry> {
        by.require("projects:write")?;
        // What a table keeps private is not written over by whoever may not read it.
        self.may_read_path(id, path, by).await?;
        if body.as_ref().len() as u64 > MAX_RAW_BYTES {
            return Err(Error::invalid("that is too much to write to one file here"));
        }
        let (dir, relative, content) = (self.roots(id)?, path.to_string(), body.as_ref().to_vec());
        let (writing, project) = (self.writing.clone(), id.to_string());
        let (written, existed, held) = blocking(move || {
            let target = dir.resolve(&relative)?;
            let existed = target.exists();
            if target.is_dir() {
                return Err(Error::invalid(format!("`{relative}` is a directory")));
            }
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            // Held from before it is on the disk until it is written down (`writing`).
            let held = dir.called(&target).map(|called| writing.hold(&project, &called));
            put_in_place(&target, &content)?;
            Ok((dir.entry(&target, &std::fs::metadata(&target)?), existed, held))
        })
        .await?;
        // A file somewhere that is never listed (a lane's place, a build's
        // output) is written and nothing more: remembered, it would be
        // found missing by the next scan and said to have been removed.
        if written.path.split('/').any(|part| IGNORED_DIRS.contains(&part)) {
            return Ok(written);
        }
        self.remember(id, &written).await?;
        drop(held);
        let change = if existed { "modified" } else { "added" };
        self.announce(id, &written.path, change, by).await;
        Ok(written)
    }

    pub async fn remove(&self, id: &str, path: &str, by: &Principal) -> Result<()> {
        by.require("projects:write")?;
        self.may_read_path(id, path, by).await?;
        // Nor is a folder that holds such a one taken away whole.
        if let Some(inside) = self.kept_from(id, by).iter().find(|kept| closed::holds(path, kept)) {
            return Err(Error::denied(format!(
                "`{path}` holds `{inside}`, which is kept private in `{id}` and is not {}'s to take away",
                by.name
            )));
        }
        let (dir, relative) = (self.roots(id)?, path.to_string());
        blocking(move || {
            let target = dir.resolve(&relative)?;
            if dir.is_top(&target) {
                return Err(Error::invalid("that is the project itself"));
            }
            match std::fs::metadata(&target) {
                Ok(meta) if meta.is_dir() => Ok(std::fs::remove_dir_all(&target)?),
                Ok(_) => Ok(std::fs::remove_file(&target)?),
                Err(_) => Err(Error::not_found(format!("there is no `{relative}`"))),
            }
        })
        .await?;
        // Whatever was under it is found, and announced, by a scan.
        self.scan(id).await?;
        Ok(())
    }

    /// Sets a project's fleet up: writes its fleet blueprint in the
    /// factory's place for its own, unless there already is one there or
    /// at its top, which is left alone.
    pub async fn init_fleet(&self, id: &str, by: &Principal) -> Result<Entry> {
        // Before anything is said of the project or of the file.
        by.require("projects:write")?;
        let project = self.get(id).await?;
        // Under whichever of the names a blueprint's file may have.
        for file in fleet_files() {
            if let Ok(existing) = self.stat(id, &file).await
                && !existing.dir
            {
                return Ok(existing);
            }
        }
        let file = format!("{PLACE}/{}", brand::blueprint_file("fleet"));
        self.write(id, &file, &templates::fleet(&project.name, &project.id), by).await
    }

    /// Whether a project's fleet blueprint is there and is word for word
    /// what this factory writes for it: one a factory made, in a folder
    /// that was let go of and is taken up again, and nobody else's text.
    pub async fn fleet_as_written(&self, id: &str) -> Result<bool> {
        let project = self.get(id).await?;
        let ours = templates::fleet(&project.name, &project.id);
        let files: Vec<PathBuf> = fleet_files().iter().filter_map(|file| self.file_of(id, file)).collect();
        // Whatever it is called there, and only if it is the one file for the name.
        blocking(move || {
            let there: Vec<String> =
                files.iter().filter_map(|file| std::fs::read_to_string(file).ok()).collect();
            Ok(there.len() == 1 && there[0] == ours)
        })
        .await
    }

    async fn announce(&self, project: &str, path: &str, change: &str, by: &Principal) {
        let data = json!({
            "project": project,
            "path": path,
            "folders": folders(path),
            "kind": kind(path),
            "change": change,
        });
        let source = Source::actor(by.actor()).taking(self.takes_to_read(project, path).as_deref());
        let _ = self.journal.emit("project.file.changed", source, data).await;
    }

    async fn remember(&self, id: &str, entry: &Entry) -> Result<()> {
        let row = (id.to_string(), entry.path.clone(), entry.modified, entry.size);
        self.files(id)
            .db()
            .call(move |connection| {
                connection.execute(
                    "INSERT OR REPLACE INTO project_files (project, path, mtime, size) VALUES (?1, ?2, ?3, ?4)",
                    params![row.0, row.1, row.2, row.3 as i64],
                )
            })
            .await
            .map(|_| ())
    }

    /// Finds what changed in a project's files since the last look, and
    /// announces it. Cheap when nothing changed: one walk of the directory
    /// and no writes.
    pub async fn scan(&self, id: &str) -> Result<Vec<Change>> {
        self.snapshot(id, true).await
    }

    pub(crate) async fn snapshot(&self, id: &str, announce: bool) -> Result<Vec<Change>> {
        let dir = self.roots(id)?;
        let now: BTreeMap<String, (i64, u64)> = blocking(move || Ok(dir.stamps())).await?;
        let project = id.to_string();
        let (seen, dir, writing) = (now.clone(), self.roots(id)?, self.writing.clone());
        let changes: Vec<Change> = self
            .files(id)
            .db()
            .call(move |connection| {
                let transaction = journal::whole(connection)?;
                let mut before: BTreeMap<String, (i64, u64)> = BTreeMap::new();
                {
                    let mut statement = transaction.prepare("SELECT path, mtime, size FROM project_files WHERE project = ?1")?;
                    let mut rows = statement.query([&project])?;
                    while let Some(row) = rows.next()? {
                        before.insert(row.get(0)?, (row.get(1)?, row.get::<_, i64>(2)? as u64));
                    }
                }
                // The walk was a while ago by now, and what is written down
                // may be newer than it: a file written through the daemon
                // since, or found by another look that walked later. So a
                // file the walk and the table disagree about is looked at
                // again, as it is now, and that is what is said of it. Going
                // by the walk, a file written in between was said to have
                // been removed, and was gone from what is searched until
                // something looked again.
                let differ = seen.iter().filter(|(path, stamp)| before.get(*path) != Some(*stamp)).map(|(path, _)| path);
                let gone = before.keys().filter(|path| !seen.contains_key(*path));
                let mut changes = Vec::new();
                for path in differ.chain(gone) {
                    let now = dir.stamp(path);
                    // One being written through the factory is its writer's
                    // to write down and say, as who they are (`writing`):
                    // asked after it is looked at, as it is now.
                    if writing.holds(&project, path) {
                        continue;
                    }
                    let change = match (before.get(path), now) {
                        (None, None) => continue,
                        (Some(old), Some(stamp)) if *old == stamp => continue,
                        (Some(_), None) => {
                            transaction.execute("DELETE FROM project_files WHERE project = ?1 AND path = ?2", params![project, path])?;
                            "removed"
                        }
                        (old, Some(stamp)) => {
                            transaction.execute(
                                "INSERT OR REPLACE INTO project_files (project, path, mtime, size) VALUES (?1, ?2, ?3, ?4)",
                                params![project, path, stamp.0, stamp.1 as i64],
                            )?;
                            if old.is_some() { "modified" } else { "added" }
                        }
                    };
                    changes.push(Change { path: path.clone(), change });
                }
                transaction.commit()?;
                Ok(changes)
            })
            .await?;
        if announce && !changes.is_empty() {
            let source = Source::actor("system");
            if changes.len() > MAX_CHANGE_EVENTS {
                let data = json!({ "project": id, "changed": changes.len() });
                let _ = self.journal.emit("project.rescanned", source, data).await;
            } else {
                for change in &changes {
                    let data = json!({
                        "project": id,
                        "path": change.path,
                        "folders": folders(&change.path),
                        "kind": kind(&change.path),
                        "change": change.change,
                    });
                    let source = source.clone().taking(self.takes_to_read(id, &change.path).as_deref());
                    let _ = self.journal.emit("project.file.changed", source, data).await;
                }
            }
        }
        Ok(changes)
    }
}

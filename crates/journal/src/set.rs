//! The journals of a factory: its own, and one for each project that has
//! a journal of its own.
//!
//! A project is to have a journal of its own, a file with a thread that
//! writes it, so that one project with a great deal to write does not
//! hold up another. The factory's own journal keeps what is nobody's, and
//! is what the others are kept beside: it is told the folder they are in
//! ([`Journal::keeps_projects_in`]), opens those there are, and makes one
//! when a project is to have one ([`Journal::own`]).
//!
//! **Which projects have one is written down, and is not worked out.** A
//! row of `journals`, in the factory's own database, says that a project
//! has a journal of its own, what it is called and which file it is in.
//! A project with no row has none, and what is said and kept of it is in
//! the factory's own: every project of a factory an earlier program made,
//! until its owner asks for it to be moved apart. A row is written when
//! [`Journal::own`] is asked, which whoever adds a project asks; nothing
//! takes one away.
//!
//! **How much of a project its journal keeps is written down too**
//! (`keeps` of its row, a level: [`keeps`]). A journal this program
//! makes keeps what is run for its project and what is said of it,
//! which is then kept there by a conductor of the project's own
//! ([`Journal::rows_of`]), and what the crates above the runner keep of
//! the project in tables of their own ([`Journal::records_of`]). Those
//! tables are made in the journal's file as the journal is made, by the
//! steps each such crate says they are made by ([`Journal::makes`]),
//! and never in a file whose row says it keeps less. One listed by a
//! program from before keeps nothing, and its project stays in the
//! factory's own.
//!
//! A journal kept beside the factory's draws its events' numbers from the
//! factory's giver ([`crate::giver`]); is measured in the factory's, where
//! measurements are read; and shuts with it at a place either was told
//! ([`Journal::shut_after`]), every one of their databases doing nothing
//! more from there. It says what it is called in its own file
//! ([`crate::NAME`]), and a file that says another name than the row that
//! names it is refused.
//!
//! Where the files are, and what they are called, is this program's own
//! to say and to change: nothing outside it reads the folder.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{OnceLock, RwLock};

use kernel::{Error, Result};
use rusqlite::Connection;

use crate::checkpoint::Kept;
use crate::{Beside, Db, FACTORY, Journal, NAME, of_project, optional};

/// What says, to a program run for it, whether a project that is added
/// is given a journal of its own: `one` has every project kept in the
/// factory's own journal, as a factory an earlier program made keeps
/// them, and `own` has a suite whose projects are not added by anybody
/// (the runner's) give each a journal of its own. A suite that is to be
/// played each way says one or the other. Unsaid, it is what a daemon
/// does: a project that is added has its own.
const SAYS: &str = "PROJECT_JOURNALS";

/// Whether a project that is added has a journal of its own, as this
/// program was told when it was started: nothing, where it was told
/// nothing. Read here and nowhere else; a daemon is never told.
pub fn apart() -> Option<bool> {
    static SAID: OnceLock<Option<bool>> = OnceLock::new();
    *SAID.get_or_init(|| match std::env::var(SAYS).ok().as_deref() {
        Some("one") => Some(false),
        Some("own") => Some(true),
        _ => None,
    })
}

/// How much of a project its own journal keeps, as the project's row of
/// `journals` says: a level, and each level keeps what the one below it
/// does. It is asked as "at least" (`>= keeps::RUNS`), so that a
/// program which comes to keep more of a project in its file makes a
/// journal at a level above these, with no column of its own, and a
/// journal made at a level below is never read as keeping what it was
/// not made to.
pub mod keeps {
    /// Nothing: a journal a program listed before anything was kept in
    /// one. What is run for the project, and what is said of it, are in
    /// the factory's own journal, with the factory's conductor.
    pub const NOTHING: i64 = 0;
    /// What is run for the project (the runner's rows, written by a
    /// conductor of the project's own) and what is said of it.
    pub const RUNS: i64 = 1;
    /// And what the crates above the runner keep of the project in
    /// tables of its own: its tables, their records and each record's
    /// history ([`super::Journal::records_of`]). A row there and what is
    /// said of it are then one journal's, and one write.
    pub const RECORDS: i64 = 2;
    /// And the turns its agents were given.
    pub const TURNS: i64 = 3;
    /// The level a journal this program makes is made at.
    pub const MADE_AT: i64 = TURNS;
}

/// Tables a crate above keeps in the file of a project's journal that
/// was made to keep at least `level` ([`keeps`]): made there as the
/// journal is made ([`Journal::makes`]).
#[derive(Clone, Copy)]
struct Made {
    level: i64,
    /// What the tables are migrated under.
    namespace: &'static str,
    steps: &'static [&'static str],
    /// Whether the factory's own database holds such rows of a project.
    held: Option<Held>,
}

/// Asks a factory's own database whether it holds rows of a project
/// that the project's journal would keep, were it made to: so of a
/// project of a factory from before ([`Journal::makes`]).
pub type Held = fn(&Connection, &str) -> rusqlite::Result<bool>;

/// The journals kept beside a factory's own.
pub(crate) struct Set {
    /// The folder their files are in; none for a factory in memory,
    /// whose projects' journals are in memory too.
    folder: Option<PathBuf>,
    /// How long each one's log is let grow, as the factory's is.
    kept: Kept,
    /// Each by its project.
    projects: RwLock<BTreeMap<String, Journal>>,
    /// How much of each project its journal keeps, as its row says
    /// ([`keeps`]).
    keeps: RwLock<BTreeMap<String, i64>>,
    /// For each journal whose project was moved apart, by the journal's
    /// name: the journal it was moved from, and where the giver of
    /// numbers stood then ([`Listed::moved`]).
    moved: RwLock<BTreeMap<String, (String, u64)>>,
    /// What a journal that is made is made with, beside its own tables.
    makes: RwLock<Vec<Made>>,
    /// One is made at a time.
    making: tokio::sync::Mutex<()>,
    /// Says how many have been made or opened, each time one is: for
    /// whoever follows every journal, to take up one made meanwhile.
    made: tokio::sync::watch::Sender<u64>,
}

impl Set {
    /// The folder the journals' files are in: none, for a factory in memory.
    pub(crate) fn folder(&self) -> Option<&std::path::Path> {
        self.folder.as_deref()
    }
}

/// A journal kept beside a factory's own, as its row says.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Listed {
    /// What it is called ([`of_project`]).
    pub name: String,
    pub project: String,
    /// Its file, in the folder they are kept in.
    pub file: String,
    /// How much of its project is kept in it ([`keeps`]).
    pub keeps: i64,
    /// For a project that was moved apart: the journal what it keeps
    /// was moved from, and where the giver of numbers stood then.
    /// Nothing for one that was made apart, as every project this
    /// program adds is.
    pub moved: Option<(String, u64)>,
}

/// The journals a factory's database lists, in the order of their names:
/// on a connection to it, the thread's own or one opened on a copy or on
/// a factory that is stopped. None, in a database from before there were
/// any.
pub fn listed(connection: &Connection) -> rusqlite::Result<Vec<Listed>> {
    let there: bool = connection.query_row(
        "SELECT EXISTS (SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'journals')",
        [],
        |row| row.get(0),
    )?;
    if !there {
        return Ok(Vec::new());
    }
    // A database a program from before listed them in says nothing of
    // what is run: none of them keeps it.
    let says: bool = connection.query_row(
        "SELECT EXISTS (SELECT 1 FROM pragma_table_info('journals') WHERE name = 'keeps')",
        [],
        |row| row.get(0),
    )?;
    let more = if says { "keeps, moved_from, moved_at" } else { "0, NULL, NULL" };
    let mut statement =
        connection.prepare(&format!("SELECT name, project, file, {more} FROM journals ORDER BY name"))?;
    let rows = statement.query_map([], |row| {
        let (from, at): (Option<String>, Option<i64>) = (row.get(4)?, row.get(5)?);
        Ok(Listed {
            name: row.get(0)?,
            project: row.get(1)?,
            file: row.get(2)?,
            keeps: row.get(3)?,
            moved: from.zip(at).map(|(from, at)| (from, at as u64)),
        })
    })?;
    rows.collect()
}

/// The file a project's journal is kept in: called for the project where
/// its id can be a file's name as it is, and otherwise with each byte
/// that cannot written out, so that no two projects have one name.
pub(crate) fn file_of(project: &str) -> String {
    let mut name = String::new();
    for byte in project.bytes() {
        match byte {
            b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' => name.push(byte as char),
            other => name.push_str(&format!("={other:02x}")),
        }
    }
    format!("{name}.db")
}

/// Holds a journal's file to what it is opened as. One that says another
/// name is refused: it is another journal's file, put where this one's
/// was looked for. One kept beside a factory's that says nothing is told
/// its name, which is written once; the factory's own says nothing.
pub(crate) async fn called(db: &Db, name: &str) -> Result<()> {
    let name = name.to_string();
    db.try_call(move |connection| {
        let says: Option<String> = optional(connection.query_row(
            "SELECT value FROM kv WHERE namespace = ?1 AND key = ?2",
            [NAME.0, NAME.1],
            |row| row.get(0),
        ))
        .map_err(crate::db_error)?;
        let says: Option<String> = says.and_then(|says| serde_json::from_str(&says).ok());
        match says {
            Some(says) if says != name => Err(Error::conflict(format!(
                "this is the journal `{says}`, and it was opened as `{name}`: a journal's file says what it is \
                 called, and one put in another's place is not read as that one"
            ))),
            None if name != FACTORY => connection
                .execute(
                    "INSERT INTO kv (namespace, key, value) VALUES (?1, ?2, ?3)",
                    [NAME.0, NAME.1, &serde_json::json!(name).to_string()],
                )
                .map(|_| ())
                .map_err(crate::db_error),
            _ => Ok(()),
        }
    })
    .await
}

impl Journal {
    /// Has this journal, a factory's own, keep the journals of its
    /// projects in `folder`, and opens each that its database lists. One
    /// that is listed and whose file is not there is not made anew: what
    /// was kept in it would be gone with nobody told, so the factory is
    /// not opened.
    pub async fn keeps_projects_in(&self, folder: impl Into<PathBuf>) -> Result<()> {
        self.keeps_projects(Some(folder.into()), Kept::default()).await
    }

    /// Like [`Journal::keeps_projects_in`], for a journal in memory: the
    /// journals of its projects are in memory too, and are gone with it.
    pub async fn keeps_projects_in_memory(&self) -> Result<()> {
        self.keeps_projects(None, Kept::default()).await
    }

    async fn keeps_projects(&self, folder: Option<PathBuf>, kept: Kept) -> Result<()> {
        if self.name() != FACTORY {
            return Err(Error::invalid(format!(
                "`{}` is a project's journal, and only a factory's own keeps others beside it",
                self.name()
            )));
        }
        let set = Set {
            folder,
            kept,
            projects: RwLock::default(),
            keeps: RwLock::default(),
            moved: RwLock::default(),
            makes: RwLock::default(),
            making: tokio::sync::Mutex::default(),
            made: tokio::sync::watch::channel(0).0,
        };
        if self.inner.set.set(set).is_err() {
            return Err(Error::conflict("this journal was told where its projects' journals are already"));
        }
        let Some(folder) = self.inner.set.get().and_then(|set| set.folder.clone()) else { return Ok(()) };
        for listed in self.inner.db.call(|connection| listed(connection)).await? {
            let file = folder.join(&listed.file);
            if !file.is_file() {
                return Err(Error::not_found(format!(
                    "the journal of the project `{}` is not where it was kept ({}): what ran for the project \
                     and what was said of it are in that file, and the factory is not started without it. \
                     Put the file back, or start the factory from a copy of it",
                    listed.project,
                    file.display()
                )));
            }
            // Opened as the factory's own was: where that waits for the
            // disk, so does each of these.
            let db = Db::open_as(&file, kept, self.inner.db.disk())?;
            let journal = self.beside(db, listed.name.clone()).await?;
            if let (Some(set), Some(moved)) = (self.inner.set.get(), listed.moved) {
                set.moved.write().expect("journal lock").insert(listed.name, moved);
            }
            self.set_of(&listed.project, journal, listed.keeps);
        }
        Ok(())
    }

    /// Opens the journal in `db` as one kept beside this one, called `name`.
    async fn beside(&self, db: Db, name: String) -> Result<Journal> {
        let (clock, device) = (self.inner.clock.clone(), self.inner.device.clone());
        Journal::open_with(db, clock, device, name, Beside::Factory(self.inner.clone())).await
    }

    fn set_of(&self, project: &str, journal: Journal, keeps: i64) {
        if let Some(set) = self.inner.set.get() {
            set.keeps.write().expect("journal lock").insert(project.to_string(), keeps);
            set.projects.write().expect("journal lock").insert(project.to_string(), journal);
            set.made.send_modify(|made| *made += 1);
        }
    }

    /// Completes once there are more journals than `known`: this one and
    /// those of its projects. For whoever reads from several and has to
    /// take up one made meanwhile. Never, for a journal that keeps none
    /// beside it.
    pub async fn more_than(&self, known: usize) {
        // Listening before looking, so that one made in between is heard.
        let mut made = self.made();
        while self.every().len() <= known {
            if made.changed().await.is_err() {
                return std::future::pending().await;
            }
        }
    }

    /// Where it is said that a journal was made beside this one. For a
    /// journal that keeps none beside it, nothing is ever said there.
    pub(crate) fn made(&self) -> tokio::sync::watch::Receiver<u64> {
        match self.inner.set.get() {
            Some(set) => set.made.subscribe(),
            None => tokio::sync::watch::channel(0).1,
        }
    }

    /// The journal of `project`'s own, where it has one.
    pub fn of_project(&self, project: &str) -> Option<Journal> {
        self.inner.set.get()?.projects.read().expect("journal lock").get(project).cloned()
    }

    /// The journal that keeps what is said of `project`: its own where
    /// it has one, and otherwise this one; and this one for what is
    /// nobody's.
    pub fn of(&self, project: Option<&str>) -> Journal {
        project.and_then(|project| self.of_project(project)).unwrap_or_else(|| self.clone())
    }

    /// For the journal called `journal`, where its project was moved
    /// apart: the journal what it keeps was moved from, by its name, and
    /// where the giver of numbers stood then. Everything the project
    /// said before is numbered at or below that, wherever it is kept
    /// now, and everything since above it. Asked of the factory's own.
    pub fn moved_of(&self, journal: &str) -> Option<(String, u64)> {
        self.inner.set.get()?.moved.read().expect("journal lock").get(journal).cloned()
    }

    /// How much of `project` its own journal keeps ([`keeps`]):
    /// nothing, for a project with no journal of its own.
    pub fn keeps_of(&self, project: &str) -> i64 {
        let Some(set) = self.inner.set.get() else { return keeps::NOTHING };
        set.keeps.read().expect("journal lock").get(project).copied().unwrap_or(keeps::NOTHING)
    }

    /// The journal that keeps what is run for `project`, where that is
    /// its own: one made to keep at least that ([`keeps::RUNS`]). None
    /// for a project with no journal of its own and for one whose
    /// journal a program from before listed: what is run for those is
    /// kept in this one.
    pub fn rows_of(&self, project: &str) -> Option<Journal> {
        if self.keeps_of(project) < keeps::RUNS {
            return None;
        }
        self.of_project(project)
    }

    /// Every project whose journal keeps what is run for it, in the
    /// order of their names, each with that journal.
    pub fn keepers(&self) -> Vec<(String, Journal)> {
        let Some(set) = self.inner.set.get() else { return Vec::new() };
        let keeps = set.keeps.read().expect("journal lock").clone();
        let projects = set.projects.read().expect("journal lock");
        keeps
            .into_iter()
            .filter(|(_, level)| *level >= keeps::RUNS)
            .filter_map(|(project, _)| {
                projects.get(&project).map(|journal| (project.clone(), journal.clone()))
            })
            .collect()
    }

    /// The journal whose database keeps what the crates above the runner
    /// keep of `project` in tables of its own (its tables, their records
    /// and each record's history): asked of the factory's own journal,
    /// by whoever reads or writes such a row, with the project the row
    /// is of. It is the project's own journal where the project's row of
    /// `journals` says that journal keeps such rows (at least
    /// [`keeps::RECORDS`]), and this one otherwise: for every project of
    /// a factory from before, for one whose journal was made before a
    /// journal kept them, and for what is nobody's.
    ///
    /// It is asked at every place such a row is read or written
    /// ([`crate::apart`] says which tables those are), so that no place
    /// is left asking the wrong file for a project whose journal keeps
    /// its rows.
    pub fn records_of(&self, project: Option<&str>) -> Journal {
        self.kept_at(project, keeps::RECORDS)
    }

    /// The journal that keeps what `project` has kept at `level`
    /// ([`keeps`]): its own where its journal was made to keep at least
    /// that, and this one otherwise, and for what is nobody's.
    pub fn kept_at(&self, project: Option<&str>, level: i64) -> Journal {
        project
            .filter(|project| self.keeps_of(project) >= level)
            .and_then(|project| self.of_project(project))
            .unwrap_or_else(|| self.clone())
    }

    /// Every journal that keeps such rows of some project ([`Self::records_of`]),
    /// this one first: for what is asked across projects, which is asked
    /// of each.
    pub fn every_records(&self) -> Vec<Journal> {
        self.every_keeping(keeps::RECORDS)
    }

    /// This journal, and each project's that keeps at least `level` of
    /// its project ([`keeps`]), in the order of the projects' names.
    pub fn every_keeping(&self, level: i64) -> Vec<Journal> {
        let mut every = vec![self.clone()];
        let Some(set) = self.inner.set.get() else { return every };
        let keeps = set.keeps.read().expect("journal lock").clone();
        let projects = set.projects.read().expect("journal lock");
        every.extend(
            keeps
                .iter()
                .filter(|(_, kept)| **kept >= level)
                .filter_map(|(project, _)| projects.get(project))
                .cloned(),
        );
        every
    }

    /// Every journal there is: this one first, then each project's own
    /// in the order of the projects' names.
    pub fn every(&self) -> Vec<Journal> {
        let mut every = vec![self.clone()];
        if let Some(set) = self.inner.set.get() {
            every.extend(set.projects.read().expect("journal lock").values().cloned());
        }
        every
    }

    /// The databases of every journal there is, each project's first and
    /// this one's last: the order they are closed in, since a project's
    /// draws its numbers from the giver this one keeps.
    pub fn dbs(&self) -> Vec<Db> {
        let mut dbs: Vec<Db> = self.every().iter().skip(1).map(|journal| journal.db().clone()).collect();
        dbs.push(self.inner.db.clone());
        dbs
    }

    /// Says that the file of a project's journal made to keep at least
    /// `level` of its project ([`keeps`]) holds the tables `steps` make,
    /// migrated under `namespace`: asked of the factory's own journal by
    /// the crate that keeps such tables, as it is opened. A journal made
    /// from then on is made with them, before its row says what it
    /// keeps; and each there already whose row says it keeps as much is
    /// brought up to what this build knows of them, as the factory's own
    /// tables are when they are opened. Nothing is made in a file whose
    /// row says it keeps less: what a project's journal keeps is said
    /// when it is made, and a database being opened changes none of it.
    ///
    /// `held` says whether the factory's own database holds such rows
    /// of a project. A project it does (one of a factory from before,
    /// taken away and added again) is given a journal that keeps what
    /// is run for it and no more ([`Journal::own`]): its rows stay
    /// where they are, whole, until its owner has it moved apart, and
    /// are not looked for in a file that has none of them.
    pub async fn makes(
        &self,
        level: i64,
        namespace: &'static str,
        steps: &'static [&'static str],
        held: Option<Held>,
    ) -> Result<()> {
        let Some(set) = self.inner.set.get() else { return Ok(()) };
        // No journal is made between this being said and those there
        // are being looked at.
        let _making = set.making.lock().await;
        {
            let mut makes = set.makes.write().expect("journal lock");
            makes.retain(|made| made.namespace != namespace);
            makes.push(Made { level, namespace, steps, held });
        }
        for journal in self.every_keeping(level).into_iter().skip(1) {
            journal.db().migrate(namespace, steps).await?;
        }
        Ok(())
    }

    /// How much of `project` a journal made for it now is made to keep:
    /// what one is made to keep now ([`keeps::MADE_AT`]), and only what
    /// is run where this journal's own database still holds rows of the
    /// project that such a journal would keep.
    async fn made_at(&self, project: &str) -> Result<i64> {
        let Some(set) = self.inner.set.get() else { return Ok(keeps::MADE_AT) };
        let asked: Vec<Held> =
            set.makes.read().expect("journal lock").iter().filter_map(|made| made.held).collect();
        let project = project.to_string();
        let held = self.inner.db.call(move |connection| {
            for held in asked {
                if held(connection, &project)? {
                    return Ok(true);
                }
            }
            Ok(false)
        });
        Ok(if held.await? { keeps::RUNS } else { keeps::MADE_AT })
    }

    /// Gives `project` a journal of its own, and gives it back: made
    /// where the project had none, which is written down in this one's
    /// database, and the one it has otherwise. Asked by whoever adds a
    /// project, before anything is said or kept of it. A journal that
    /// keeps none beside it (it was told no folder: a test's, a tool's)
    /// gives itself back, and the project's journal is this one; and so
    /// does one in a program that was told its projects are kept in one
    /// journal ([`apart`]).
    pub async fn own(&self, project: &str) -> Result<Journal> {
        if let Some(found) = self.of_project(project) {
            return Ok(found);
        }
        let level = self.made_at(project).await?;
        self.own_keeping(project, level).await
    }

    /// Like [`Journal::own`], with how much of the project a journal
    /// that is made is to keep ([`keeps`]): for whoever makes one as a
    /// program before this did, or tries a level by itself. One there
    /// already is given back as it is, whatever it keeps.
    pub async fn own_keeping(&self, project: &str, level: i64) -> Result<Journal> {
        let Some(set) = self.inner.set.get() else { return Ok(self.clone()) };
        if let Some(found) = self.of_project(project) {
            return Ok(found);
        }
        // A program told that its projects are kept in one journal makes
        // none: one that is listed already is still its project's.
        if apart() == Some(false) {
            return Ok(self.clone());
        }
        if project.is_empty() {
            return Err(Error::invalid("a journal of its own is a project's, and no project was named"));
        }
        let _making = set.making.lock().await;
        if let Some(found) = self.of_project(project) {
            return Ok(found);
        }
        // The file first, which says what it is called; then the row
        // that names it. Stopped between the two, the file is there with
        // no row and nothing said in it: it is the one that is opened
        // where this is asked again, and is taken away, as a file no
        // row names, where a daemon starts first (`moved::clear`).
        let (name, file) = (of_project(project), file_of(project));
        let db = match &set.folder {
            Some(folder) => Db::open_as(&folder.join(&file), set.kept, self.inner.db.disk())?,
            None => Db::memory()?,
        };
        let journal = self.beside(db, name.clone()).await?;
        // What it is made to keep has its tables before the row says it
        // keeps it: a file with no row is opened here again, and made
        // with whatever it still lacks.
        let makes = set.makes.read().expect("journal lock").clone();
        for made in makes.iter().filter(|made| level >= made.level) {
            journal.db().migrate(made.namespace, made.steps).await?;
        }
        let (project, now) = (project.to_string(), self.now_ms());
        let row = (name, project.clone(), file);
        self.inner
            .db
            .call(move |connection| {
                connection.execute(
                    "INSERT INTO journals (name, project, file, made, keeps) VALUES (?1, ?2, ?3, ?4, ?5)
                     ON CONFLICT (name) DO NOTHING",
                    rusqlite::params![row.0, row.1, row.2, now, level],
                )
            })
            .await?;
        self.set_of(&project, journal.clone(), level);
        Ok(journal)
    }
}

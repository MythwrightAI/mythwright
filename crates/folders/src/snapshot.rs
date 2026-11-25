//! A snapshot: a copy of one factory, in a folder, to start another from.
//!
//! Factories share nothing, and nothing copies one into another by itself.
//! But trying something against what a real factory holds, with no chance
//! of touching that factory, takes a second factory that starts with the
//! same content. So a person can ask a factory for a snapshot of itself
//! (the `factory.snapshot` action, which `std/snapshot` runs), and start a
//! new factory from one (`crate::arrival`). It goes one way, into a
//! factory that does not exist yet, and only when asked.
//!
//! The factory is only read. A snapshot is a plain folder:
//!
//! ```text
//! snapshot.json    what it is, when and from which factory, and what is not in it
//! database.db      the factory's own database
//! journals/        the journal of each project that has one of its own, a file each
//! attachments/     what was attached to messages, a file each
//! extensions/      the installed extension packages
//! projects/        the files of the projects kept in the factory's own projects folder
//! ```
//!
//! and nothing else of the data folder: what goes is listed here, and what
//! is not listed does not go. The database and the journals are each as
//! they were at one moment, the same for all of them (`journal::moment`),
//! whatever the factory was writing as the copy was taken: a copy is a
//! factory that could have been.
//!
//! The folder may be handed to another user or another device, so
//! everything secret or running is taken out when it is made, by each
//! crate for its own tables ([`leave_out`]), and taken out of the file
//! itself and not only its tables. The same is done again on arrival, to
//! a snapshot that was made, or changed, some other way.
//!
//! It still holds people's password hashes and everything said in every
//! channel: the folder is to be kept as the factory's own data is. It is
//! made readable by its owner only.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use blueprint::{ActionSpec, ValueType as T};
use conductor::{ActionResult, Registry};
use journal::Journal;
use journal::copy::Connection;
use journal::moment::Moment;
use kernel::{BoxFut, Error, Result};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use studio::Projects;

use crate::registry::settled;
use crate::tree::{self, Copier, NotCopied, Tally};

/// The file in a folder that says the folder is a snapshot, and of what.
/// It is written last: a folder without one is not a snapshot.
pub const MANIFEST_FILE: &str = "snapshot.json";
/// Where the daemon remembers which build of the client it last served
/// (`served::client`), which a copy forgets: the copy is served from another
/// folder.
pub const CLIENT: &str = "client";
/// The database in a snapshot. Not called what a factory's is: a folder is
/// a factory's data by the database in it, and a snapshot is not one.
pub const DATABASE: &str = "database.db";
/// The folders of a data folder that go into a snapshot, under the same names.
pub const ATTACHMENTS: &str = "attachments";
/// The folder of a data folder, and of a snapshot, that the journals of
/// the projects that have one of their own are in, a file each, as the
/// factory's own database lists them.
pub const JOURNALS: &str = "journals";
pub const EXTENSIONS: &str = "extensions";
/// The folder of a data folder that files are kept in while they are in
/// the Trash (`studio::trash`): it goes, with the rows that say them.
pub const TRASH: &str = studio::trash::FOLDER;
/// The folder of a data folder that the factory keeps blueprints in: each
/// project's own, in a folder of its own (`conductor::library::folders`).
/// It goes whole, as the projects' rows do.
pub const BLUEPRINTS: &str = conductor::library::folders::KEPT;
/// The folder of a snapshot that the files of projects are in.
pub const PROJECTS: &str = "projects";
/// The folder of a data folder that each agent's slot has a folder in, for
/// its credential and for its work when it has no project. It does not go.
pub const SLOTS: &str = "slots";
/// What an agent's credential is called in its slot's folder.
const TOKEN: &str = "token";
/// The folder of a data folder that the values of the sign-ins the factory
/// holds for its agents' programs are in, a file each. It does not go, and
/// neither does a row that speaks of one.
pub const SIGNINS: &str = "signins";
/// The folder of a data folder that the secrets blueprints hand to the
/// programs they run are in, a file each. It does not go.
pub const SECRETS: &str = "secrets";

const WHAT: &str = "factory-snapshot";
/// How a snapshot is laid out. One laid out in a way that came later is refused.
const FORMAT: u32 = 1;

/// What no snapshot holds, whatever the factory.
const NEVER: &[&str] = &[
    "sessions: everyone signs in again, with the password they have",
    "the credentials of agents, devices and scripts, and invitations",
    "the folder of agents' slots: each agent's credential, and whatever an agent with no project wrote in its own",
    "the sign-ins the factory holds for its agents' programs (keys, tokens, and where a provider's program keeps its own): their values, which there are, who held which and what was set aside. An agent that draws one says so until the new factory is given its own",
    "the secrets blueprints hand to the programs they run: a blueprint that names one fails, saying which, until the new factory is given its own",
    "the link to a hub, and the devices that had joined this one",
    "measurements",
    "the logs of runs and services, and where runs' steps write where a folder was said for them: two factories never write in one, so a factory started from it writes in its own data folder until its owner says otherwise. How long they are kept goes",
    "the search index, which is built again",
    "anything running: services, routers, and the work of live instances, unfinished runs and agents' turns",
    "the files of a project whose directory is outside the factory's own projects folder",
    "the address of a store service that keeps a table",
    "the variables of the profile (`profile.vars`): they are said of this factory, and are set in the new one by applying `std/profile` there",
];

/// Which factory a snapshot was made from.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Origin {
    /// What its profile is called, if it has one.
    // Called `environment` by a program before 0.1.0.
    #[serde(alias = "environment")]
    pub profile: Option<String>,
    /// The device it runs on, by the name it gives itself.
    pub device: String,
}

/// A project whose files are in a snapshot.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectHeld {
    pub id: String,
    pub name: String,
    /// Where its files are, under the snapshot's `projects` folder.
    pub at: String,
    pub files: usize,
}

/// A project that is in a snapshot without its files.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectLeft {
    pub id: String,
    pub name: String,
    /// The directory whose files were left where they are.
    pub directory: String,
    pub why: String,
    /// Its tables that are kept as files there, and so came with nothing in them.
    pub kept_as_files: Vec<String>,
}

/// What a snapshot holds besides the database.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Holds {
    pub attachments: usize,
    pub extensions: Vec<String>,
    pub projects: Vec<ProjectHeld>,
}

/// What is not in a snapshot, or is in it changed.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LeftOut {
    /// What no snapshot holds, in words.
    pub never: Vec<String>,
    /// Instances, runs and steps that were in progress, and are in it as ended.
    pub ended: conductor::copy::Ended,
    /// Services and routers that were running.
    pub unhosted: hosting::copy::Unhosted,
    /// Agents' turns that were waiting or under way, and are in it as failed.
    pub turns: usize,
    /// The agents: each is in it as it was set up, without its credential.
    pub agents: Vec<String>,
    /// The agents whose own folders held more than a credential: what each
    /// wrote there, working with no project, was left where it is.
    #[serde(default)]
    pub slots: Vec<String>,
    pub sessions: usize,
    pub credentials: usize,
    pub invites: usize,
    /// Projects that are in it without their files.
    pub projects: Vec<ProjectLeft>,
    /// Tables that were kept by a store service, and have no store in it.
    pub stores: Vec<studio::copy::Unstored>,
    /// Tables that no part of the product has said go into a copy, and
    /// that were emptied in it: none, unless one was added and forgotten.
    #[serde(default)]
    pub tables: Vec<String>,
    /// Files and folders that were not copied, up to a number of them.
    pub files: Vec<NotCopied>,
    /// How many more were not copied than are listed.
    pub more_files: usize,
}

/// What a snapshot says of itself: `snapshot.json`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    /// Always `factory-snapshot`.
    pub what: String,
    pub format: u32,
    /// An id of this snapshot's own.
    pub id: String,
    /// When it was made, in milliseconds since the epoch.
    pub made: i64,
    /// The version of the product that made it.
    pub version: String,
    pub from: Origin,
    /// The version each crate's tables are at in its database, by the name
    /// they are migrated under.
    pub schema: BTreeMap<String, i64>,
    pub holds: Holds,
    pub left_out: LeftOut,
}

impl Manifest {
    /// What the folder `from` says it is, or why it is not a snapshot.
    pub fn read(from: &Path) -> Result<Manifest> {
        let not_one = |why: String| {
            Error::invalid(format!(
                "{from} is not a snapshot of a factory: {why}. A snapshot is a folder a factory wrote a copy of itself to; \
                 make one in the factory to copy, with `{cli} apply std/snapshot --param to=<folder>`, and give that folder",
                from = from.display(),
                cli = brand::CLI,
            ))
        };
        let path = from.join(MANIFEST_FILE);
        let text = std::fs::read_to_string(&path)
            .map_err(|err| not_one(format!("it has no `{MANIFEST_FILE}` that can be read ({err})")))?;
        let manifest: Manifest = serde_json::from_str(&text).map_err(|err| {
            not_one(format!("its `{MANIFEST_FILE}` does not say what a snapshot says ({err})"))
        })?;
        if manifest.what != WHAT {
            return Err(not_one(format!("its `{MANIFEST_FILE}` says it is `{}`", manifest.what)));
        }
        if manifest.format > FORMAT {
            return Err(Error::invalid(format!(
                "{} was made by a newer version of {product} ({}) than this one ({}), and is laid out in a way this \
                 one does not know. Start the new factory with that version, or a newer one",
                from.display(),
                manifest.version,
                env!("CARGO_PKG_VERSION"),
                product = brand::DISPLAY,
            )));
        }
        if !from.join(DATABASE).is_file() {
            return Err(not_one(format!("it has no `{DATABASE}`")));
        }
        Ok(manifest)
    }
}

/// Refuses tables this build does not understand: ones it has never heard
/// of, or at a version newer than it knows. It would neither read them
/// rightly nor know what of them must not travel.
pub(crate) fn understood<'a>(
    schema: impl IntoIterator<Item = (&'a String, &'a i64)>,
    of: &Path,
) -> Result<()> {
    let known: BTreeMap<&str, usize> = [
        journal::copy::KNOWS,
        conductor::copy::KNOWS,
        hosting::copy::KNOWS,
        studio::copy::KNOWS,
        agents::copy::KNOWS,
        topics::KNOWS,
    ]
    .concat()
    .into_iter()
    .collect();
    for (namespace, version) in schema {
        let newer = match known.get(namespace.as_str()) {
            Some(known) if *version <= *known as i64 => continue,
            Some(known) => format!(
                "its `{namespace}` tables are at version {version}, and this one knows them to {known}"
            ),
            None => format!("it has `{namespace}` tables, which this one has never heard of"),
        };
        return Err(Error::invalid(format!(
            "{} was made by a newer version of {} than this one: {newer}. Start the new factory with that version, or a newer one",
            of.display(),
            brand::DISPLAY,
        )));
    }
    Ok(())
}

/// What was taken out of a copy of the database.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Removed {
    pub measurements: usize,
    pub ended: conductor::copy::Ended,
    pub unhosted: hosting::copy::Unhosted,
    pub studio: studio::copy::Removed,
    pub stood: agents::copy::Stood,
    pub devices: usize,
    /// Tables no crate has said anything about, which were emptied.
    #[serde(default)]
    pub tables: Vec<String>,
}

/// Takes out of a copy of the database everything that does not go with
/// one, as of `at`: each crate its own, lowest first, and all of it or
/// none. It is done when a snapshot is made and again when a factory is
/// started from one; done again it removes nothing more.
pub(crate) fn leave_out(connection: &Connection, at: i64, making: Option<(&str, &str)>) -> Result<Removed> {
    let all = |all: &Connection| -> rusqlite::Result<Removed> {
        let spoken_for = [
            journal::copy::TABLES,
            conductor::copy::TABLES,
            hosting::copy::TABLES,
            studio::copy::TABLES,
            agents::copy::TABLES,
            topics::TABLES_COPIED,
        ]
        .concat();
        let removed = Removed {
            measurements: journal::copy::leave_out(all)?,
            ended: conductor::copy::leave_out(all, at, making)?,
            unhosted: hosting::copy::leave_out(all)?,
            studio: studio::copy::leave_out(all)?,
            stood: agents::copy::leave_out(all, at)?,
            devices: gateway::copy::leave_out(all)?,
            // What goes is what a crate has said goes: a table nobody
            // has spoken for is emptied, and so is the small state below.
            tables: journal::copy::keep_tables(all, &spoken_for)?,
        };
        // The comparisons go, and there is nothing to say of how many.
        comparisons::copy::leave_out(all)?;
        // The daemon's own: which build of the client was last seen, which
        // is said of the folder it is served from; and what a factory that
        // was itself started from a copy remembers of that.
        journal::copy::forget(all, CLIENT)?;
        journal::copy::forget(all, crate::arrival::REMEMBERED.0)?;
        // And whatever nobody said anything about.
        journal::copy::keep_state(all, studio::copy::KEPT)?;
        Ok(removed)
    };
    together(connection, all).map_err(|err| {
        journal::db_error(err)
            .context("taking out of the copy what does not go with one (is it a factory's database?)")
    })
}

/// Does `work` on a copy's own connection, all of it or none.
fn together<T>(
    connection: &Connection,
    work: impl FnOnce(&Connection) -> rusqlite::Result<T>,
) -> rusqlite::Result<T> {
    let all = connection.unchecked_transaction()?;
    let did = work(&all)?;
    all.commit()?;
    Ok(did)
}

/// What was taken out of the copy of a project's own journal.
#[derive(Clone, Debug, Default)]
pub(crate) struct Beside {
    /// What was running for the project there, and is in it as ended.
    pub ended: conductor::copy::Ended,
    /// What it holds of the project's tables that whoever takes the
    /// copy is told of.
    pub studio: studio::copy::Beside,
    /// Turns of the project's agents that were waiting or under way
    /// there, and are in it as failed.
    pub turns: usize,
    /// Tables there that nobody has said a project's journal keeps,
    /// which were emptied.
    pub tables: Vec<String>,
}

/// Removes from a copy of a project's own journal what does not go with
/// one: of the journal's own tables what the factory's copy leaves out
/// too; of what was run for the project, where its journal keeps that,
/// what the runner leaves out of the factory's (what was running is
/// said to have been, and is not carried on); of the project's tables,
/// where its journal keeps those, what the studio leaves out of the
/// factory's; of its agents' turns, where it keeps those, what the
/// agents leave out; and whatever is there that nobody said travels. What of
/// it whoever takes the copy is told is handed back. Done when a
/// snapshot is made and again when a factory is started from one.
pub(crate) fn leave_out_beside(
    connection: &Connection,
    at: i64,
    making: Option<(&str, &str)>,
) -> Result<Beside> {
    let all = |all: &Connection| -> rusqlite::Result<Beside> {
        journal::copy::leave_out(all)?;
        // A journal that keeps what is run for its project has the
        // runner's tables; one a program from before listed has none.
        let runs: bool = all.query_row(
            "SELECT EXISTS (SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'instances')",
            [],
            |row| row.get(0),
        )?;
        let ended = match runs {
            true => conductor::copy::leave_out(all, at, making)?,
            false => Default::default(),
        };
        let studio = studio::copy::leave_out_beside(all)?;
        let turns = agents::copy::leave_out_beside(all, at)?;
        // What goes is what a crate has said a project's journal keeps:
        // any other table that is there is emptied. (The agents' own
        // are all made where turns are kept, and all spoken for.)
        let apart = studio::apart::KEPT_APART.iter().map(|(table, _)| *table);
        let spoken_for: Vec<&str> = [journal::copy::TABLES, conductor::copy::TABLES, agents::copy::TABLES]
            .concat()
            .into_iter()
            .chain(apart)
            .collect();
        let tables = journal::copy::keep_tables(all, &spoken_for)?;
        journal::copy::keep_state(all, studio::copy::KEPT_BESIDE)?;
        Ok(Beside { ended, studio, turns, tables })
    };
    together(connection, all).map_err(|err| {
        journal::db_error(err)
            .context("taking out of the copy of a project's journal what does not go with one")
    })
}

/// What is told as the journal of a project is about to be written into
/// a snapshot, by where it goes there (`journals/shop.db`), and waited
/// for: for a test, which holds the copy at that file while the factory
/// goes on writing and then looks for what was written meanwhile in the
/// snapshot. Every file has been read as of its one moment by then
/// (`journal::moment`). Never said for a factory that is run.
pub type Copying = Arc<dyn Fn(&str) + Send + Sync>;

/// The factory a snapshot is made of.
pub struct Source {
    /// Its data folder.
    pub data: PathBuf,
    /// The folder it keeps its projects in.
    pub projects: PathBuf,
    pub profile: Option<String>,
    /// Told as each project's journal is about to be written ([`Copying`]).
    pub copying: Option<Copying>,
}

/// What is known of a snapshot before it is made.
pub(crate) struct Asked {
    /// The id it will have.
    pub id: String,
    /// When it is being made, in milliseconds since the epoch.
    pub made: i64,
    /// The device it is made on, by the name it gives itself.
    pub device: String,
    /// The instance and the run that are making it: in progress as it is
    /// made, and not part of what the factory was running.
    pub by: (String, String),
}

/// Whether `id` can be the name of a file and nothing more.
pub(crate) fn is_name(id: &str) -> bool {
    !id.is_empty() && id != "." && id != ".." && !id.contains(['/', '\\', '\0'])
}

/// `under` with the parts of `at` added, which are separated by `/`.
pub(crate) fn within(under: &Path, at: &str) -> PathBuf {
    at.split('/').fold(under.to_path_buf(), |path, part| path.join(part))
}

/// Settles where a snapshot is written, and makes the folder: private to
/// whoever runs the daemon. Also says whether the folder was made here.
fn place(to: &str, source: &Source, dirs: &[(String, PathBuf)]) -> Result<(PathBuf, bool)> {
    let asked = Path::new(to.trim());
    if !asked.is_absolute() {
        return Err(Error::invalid(format!(
            "`to` is the folder the snapshot is written to, said by its whole path, such as /backups/prod-today; `{to}` is not one"
        )));
    }
    let to = settled(asked);
    let inside = |dir: &Path| kernel::whole::within(&to, &settled(dir));
    let refuse = |of: String| {
        Error::invalid(format!(
            "{} is inside {of}. A snapshot is a copy of what is there, and is written somewhere else: \
             a folder of its own, outside the factory",
            to.display()
        ))
    };
    if inside(&source.data) {
        return Err(refuse(format!("this factory's data folder ({})", source.data.display())));
    }
    if inside(&source.projects) {
        return Err(refuse(format!(
            "the folder this factory keeps its projects in ({})",
            source.projects.display()
        )));
    }
    if let Some((id, dir)) = dirs.iter().find(|(_, dir)| inside(dir)) {
        return Err(refuse(format!("the project `{id}` ({})", dir.display())));
    }
    let made = match std::fs::symlink_metadata(&to) {
        Ok(meta) if meta.is_dir() => {
            let holds = std::fs::read_dir(&to).map_err(|err| Error::from(err).context(to.display()))?.next();
            if holds.is_some() {
                return Err(Error::invalid(format!(
                    "{} already has things in it. A snapshot is written to a folder that does not exist yet, or to an empty one",
                    to.display()
                )));
            }
            false
        }
        Ok(_) => return Err(Error::invalid(format!("{} is not a folder", to.display()))),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => true,
        Err(err) => return Err(Error::from(err).context(to.display())),
    };
    tree::private_dir(&to)?;
    Ok((to, made))
}

/// What writing a snapshot came to: what it says of itself, where it is,
/// what the files came to, and how long every journal of the factory
/// stood still while it was read, in milliseconds.
pub(crate) type Taken = (Manifest, PathBuf, Tally, f64);

/// Writes a snapshot of the factory at `source`, whose own journal is
/// `journal`, to the folder `to`. It blocks for as long as the copying
/// takes. A snapshot that could not be finished is taken away again:
/// half of one is not one.
pub(crate) fn take(
    source: &Source,
    journal: &Journal,
    to: &str,
    dirs: &[(String, PathBuf)],
    asked: Asked,
    stop: &(dyn Fn() -> bool + Sync),
) -> Result<Taken> {
    let (to, made) = place(to, source, dirs)?;
    let written = write(source, journal, &to, asked, stop);
    if written.is_err() {
        // (And what a database that was being opened as a copy has beside it.)
        let beside = ["-journal", "-wal", "-shm"].map(|ending| format!("{DATABASE}{ending}"));
        for file in [MANIFEST_FILE.to_string(), DATABASE.to_string()].into_iter().chain(beside) {
            let _ = std::fs::remove_file(to.join(file));
        }
        for folder in [ATTACHMENTS, EXTENSIONS, TRASH, BLUEPRINTS, PROJECTS, JOURNALS] {
            let _ = std::fs::remove_dir_all(to.join(folder));
        }
        if made {
            let _ = std::fs::remove_dir(&to);
        }
    }
    written.map(|(manifest, tally, stood_ms)| (manifest, to, tally, stood_ms))
}

fn write(
    source: &Source,
    journal: &Journal,
    to: &Path,
    Asked { id, made, device, by }: Asked,
    stop: &(dyn Fn() -> bool + Sync),
) -> Result<(Manifest, Tally, f64)> {
    // Every file as it was at one moment, whatever is written to any of
    // them meanwhile: the factory's own database, and the journal of
    // each project that has one of its own, as the factory's lists them
    // at that moment. Each is written out whole before anything is
    // taken out of any, so that no file is held as it was for longer
    // than the copying of it and of those before it takes: what holds a
    // file so keeps its log from being brought back.
    let Moment { factory, beside, stood_ms } = journal.moment()?;
    let database = to.join(DATABASE);
    factory.write(&database)?;
    tree::private_file(&database)?;
    let mut journals = Vec::new();
    for (listed, read) in beside {
        if stop() {
            return Err(Error::unavailable("the copy was stopped before it was finished"));
        }
        let copy = to.join(JOURNALS).join(&listed.file);
        tree::private_dir(&to.join(JOURNALS))?;
        if let Some(told) = &source.copying {
            told(&format!("{JOURNALS}/{}", listed.file));
        }
        read.write(&copy)?;
        tree::private_file(&copy)?;
        journals.push(copy);
    }
    // Then, from the copies and never from the factory's own, what does
    // not travel.
    let connection = journal::copy::open(&database)?;
    let mut removed = leave_out(&connection, made, Some((&by.0, &by.1)))?;
    let mut placed = studio::copy::uproot(&connection, &source.projects).map_err(journal::db_error)?;
    let mut attached = studio::copy::attached(&connection).map_err(journal::db_error)?;
    let schema: BTreeMap<String, i64> =
        journal::copy::versions(&connection).map_err(journal::db_error)?.into_iter().collect();
    journal::copy::seal(connection)?;
    for copy in &journals {
        let connection = journal::copy::open(copy)?;
        // What was running for the project is counted with the rest,
        // and what its tables are kept by is said with the rest.
        let beside = leave_out_beside(&connection, made, Some((&by.0, &by.1)))?;
        removed.ended.instances += beside.ended.instances;
        removed.ended.runs += beside.ended.runs;
        removed.ended.steps += beside.ended.steps;
        removed.studio.with(&beside.studio);
        removed.stood.turns += beside.turns;
        for table in beside.tables {
            if !removed.tables.contains(&table) {
                removed.tables.push(table);
            }
        }
        studio::copy::kept_beside(&mut placed, &beside.studio.as_files);
        // What is attached to its records is a file beside the factory's, as a channel's is.
        attached.extend(studio::attaching::attached_ids(&connection).map_err(journal::db_error)?);
        journal::copy::seal(connection)?;
    }

    // Then the files the copy speaks of, and no others.
    let guarded = [settled(&source.data), settled(&source.projects)];
    let copier = Copier { guarded: &guarded, stop };
    let mut tally = Tally::default();
    let kept = to.join(ATTACHMENTS);
    std::fs::create_dir_all(&kept).map_err(|err| Error::from(err).context(kept.display()))?;
    let mut attachments = 0;
    for id in attached.iter().filter(|id| is_name(id)) {
        let file = source.data.join(ATTACHMENTS).join(id);
        if std::fs::symlink_metadata(&file).is_ok_and(|meta| meta.is_file()) {
            tally.bytes += tree::file(&file, &kept.join(id))?;
            tally.files += 1;
            attachments += 1;
        }
    }
    copier.folder(&source.data.join(EXTENSIONS), &to.join(EXTENSIONS), EXTENSIONS, &mut tally)?;
    copier.folder(&source.data.join(TRASH), &to.join(TRASH), TRASH, &mut tally)?;
    copier.folder(&source.data.join(BLUEPRINTS), &to.join(BLUEPRINTS), BLUEPRINTS, &mut tally)?;
    let mut extensions: Vec<String> = std::fs::read_dir(to.join(EXTENSIONS))
        .into_iter()
        .flatten()
        .flatten()
        .filter(|entry| entry.path().is_dir())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    extensions.sort();
    // An agent with no project works in its slot's own folder, and none
    // of those folders goes: which of them held work is said.
    let mut slots: Vec<String> = std::fs::read_dir(source.data.join(SLOTS))
        .into_iter()
        .flatten()
        .flatten()
        .filter(|slot| {
            std::fs::read_dir(slot.path())
                .is_ok_and(|mut files| files.any(|file| file.is_ok_and(|file| file.file_name() != TOKEN)))
        })
        .map(|slot| slot.file_name().to_string_lossy().into_owned())
        .collect();
    slots.sort();
    let (mut held, mut left) = (Vec::new(), Vec::new());
    for project in placed {
        let dir = settled(&project.dir);
        let why = if !project.held {
            "its directory is not in the factory's own projects folder, and two factories never share a directory"
        } else if !dir.is_dir() {
            "its directory was not there"
        } else {
            let before = tally.files;
            copier.folder(&dir, &within(&to.join(PROJECTS), &project.at), &project.id, &mut tally)?;
            let files = tally.files - before;
            held.push(ProjectHeld { id: project.id, name: project.name, at: project.at, files });
            continue;
        };
        left.push(ProjectLeft {
            id: project.id,
            name: project.name,
            directory: project.dir.to_string_lossy().into_owned(),
            why: why.to_string(),
            kept_as_files: project.kept_as_files,
        });
    }

    let manifest = Manifest {
        what: WHAT.to_string(),
        format: FORMAT,
        id,
        made,
        version: env!("CARGO_PKG_VERSION").to_string(),
        from: Origin { profile: source.profile.clone(), device },
        schema,
        holds: Holds { attachments, extensions, projects: held },
        left_out: LeftOut {
            never: NEVER.iter().map(|never| never.to_string()).collect(),
            ended: removed.ended,
            unhosted: removed.unhosted,
            turns: removed.stood.turns,
            agents: removed.stood.agents,
            slots,
            sessions: removed.studio.sessions,
            credentials: removed.studio.credentials,
            invites: removed.studio.invites,
            projects: left,
            stores: removed.studio.stores,
            tables: removed.tables,
            files: tally.left.clone(),
            more_files: tally.more,
        },
    };
    // Last: this is what makes the folder a snapshot.
    let path = to.join(MANIFEST_FILE);
    let text = serde_json::to_string_pretty(&manifest)?;
    std::fs::write(&path, format!("{text}\n")).map_err(|err| Error::from(err).context(path.display()))?;
    tree::private_file(&path)?;
    Ok((manifest, tally, stood_ms))
}

/// Adds `factory.snapshot` to what a blueprint can do here.
pub(crate) fn register(registry: &mut Registry, source: Source, projects: Projects) {
    let source = Arc::new(source);
    let spec = ActionSpec::new(
        "factory.snapshot",
        "Write a copy of this factory to a folder, to start another factory from. The factory is only read.",
    )
.example(SNAPSHOT_ASKED)
    .making("a copy of this factory in the folder", "to")
    .required(
        "to",
        T::String,
        "the folder to write it to, by its whole path: one that does not exist yet, or an empty one, outside the factory",
    )
    // The whole factory's content, people's password hashes among it, is
    // written where whoever asks says: the owner's to ask, and nobody else's.
    .capability("admin")
    .output("to", "the folder it was written to")
    .output("snapshot", "the snapshot's id")
    .output("made", "when it was made, in milliseconds since the epoch")
    .output("files", "how many files were copied")
    .output("bytes", "how much they came to")
    .output("left_out", "what is not in it, as its `snapshot.json` says");
    registry.add_fn(spec, move |cx, args| -> BoxFut<'static, ActionResult> {
        let (source, projects) = (source.clone(), projects.clone());
        Box::pin(async move {
            let to = args.get("to").and_then(Value::as_str).unwrap_or_default().to_string();
            let journal = cx.engine().journal().clone();
            let dirs: Vec<(String, PathBuf)> = projects
                .list()
                .await?
                .into_iter()
                .map(|project| (project.id, PathBuf::from(project.path)))
                .collect();
            let asked = Asked {
                id: journal.new_id("snap"),
                made: journal.now_ms(),
                device: journal.device().to_string(),
                by: (cx.instance().to_string(), cx.run().to_string()),
            };
            let cancel = cx.cancel().clone();
            // The copying is done off the threads everything else runs on,
            // and off the one the database's work is done on.
            let started = Instant::now();
            let factory = journal.clone();
            let taken = tokio::task::spawn_blocking(move || {
                take(&source, &factory, &to, &dirs, asked, &|| cancel.is_cancelled())
            })
            .await
            .map_err(|err| Error::internal(format!("taking the snapshot crashed: {err}")))
            .and_then(|taken| taken);
            let took_ms = started.elapsed().as_secs_f64() * 1000.0;
            journal.measure("snapshot", &[], took_ms, taken.is_ok());
            let (manifest, to, tally, stood_ms) = taken?;
            // How long every journal stood still to be read as of one moment.
            journal.measure("snapshot.stood", &[], stood_ms, true);
            let note = format!(
                "A copy of this factory was written to {}, to start another factory from.",
                to.display()
            );
            let data = json!({
                "to": to, "snapshot": manifest.id, "made": manifest.made, "profile": manifest.from.profile,
                "took_ms": took_ms.round(), "files": tally.files, "bytes": tally.bytes,
                "holds": manifest.holds, "left_out": manifest.left_out, "note": note,
            });
            cx.emit("system.snapshot", data).await?;
            Ok(json!({
                "to": to, "snapshot": manifest.id, "made": manifest.made, "files": tally.files,
                "bytes": tally.bytes, "left_out": manifest.left_out,
            }))
        })
    });
}

// What the actions here are shown by in the Reference and the editor: each a whole
// blueprint that asks it, held to checking with no warning (the API suite's `examples.rs`).
const SNAPSHOT_ASKED: &str = "\
blueprint: copy-of-here
permissions: [admin]

steps:
  copied:
    do: factory.snapshot
    with: { to: /srv/copies/factory }
";

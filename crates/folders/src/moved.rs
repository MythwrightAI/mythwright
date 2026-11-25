//! A project of a factory from before, moved to a journal of its own.
//!
//! A project this program adds is run by a conductor of its own, with
//! what is run for it kept in its own file. One that an earlier program
//! added has neither: it stays whole in the factory's own journal, with
//! the factory's conductor, and nothing changes that by the daemon
//! having started. Its owner asks for it to be moved apart, as `repack`
//! is asked: of a factory that is stopped, by a program that holds the
//! factory's folder as a daemon would while it is done, so that none is
//! started in the middle of it.
//!
//! What is moved is what the runner keeps of the project (every table
//! `runs::apart::TABLES` lists, by its line there); what is kept beside
//! those rows in the small durable state (every name of
//! `runs::beside::KEPT` and of the comparisons' list, by its line
//! there: what its trees are held to, what was vouched for them, what
//! their handlers last read); what was said of it, each event under
//! the number it had, where its journal had kept nothing said until
//! then (`runs::moved::said`); its tables, their records, each record's
//! history and what a scan last saw of its files
//! (`studio::moved::tables`); and what its agents were asked
//! (`agents::moved::tables`). It is one list ([`tables`]), and how much
//! of it there is to move is what the project's journal does not keep
//! yet: a project whose journal an earlier program made, or that was
//! moved when less was moved, is taken the rest of the way. What it
//! keeps and says from then on is in its own journal. A project is
//! moved at rest: with a run under way, or a turn of one of its agents
//! waiting, running or answered and not yet ended, it is left where it
//! is and whoever asked is told what is in the way.
//!
//! Two files are not one write, so one write of the factory's database
//! decides (`journal::moved`): before it the factory is as it was, and
//! after it the project is moved. A file that write does not name is
//! nobody's, and is taken away by the daemon as it next starts
//! ([`clear`]), or by the move when it is asked again. What was done is
//! kept in the database and said once by the daemon that next starts on
//! it (`system.moved_apart`), since nothing was running to say so.

use std::path::{Path, PathBuf};
use std::sync::Arc;

pub use journal::moved::Counted;
use journal::moved::{Kept, Row};
use kernel::{Clock, Error, Result, Source, SystemClock};
use serde_json::{Value, json};

use crate::DATABASE_FILE;
use crate::snapshot::JOURNALS;

/// Where what was done is kept until a daemon says it.
const REMEMBERED: (&str, &str) = ("factory", "moved_apart");

/// How many rows are moved in a second, for saying about how long a
/// move takes. Measured, a project of 200,000 runs and 600,000 steps
/// was moved in 15 s out of a journal of 850 MB read from the disk, on
/// the machine this was written on with other work going on beside it;
/// this is a third of that, so that a slower disk is not promised what
/// it cannot do.
const A_SECOND: u64 = 18_000;

/// A project of a factory that is stopped, and what moving it apart
/// would come to.
#[derive(Clone, Debug)]
pub struct Project {
    pub id: String,
    /// The file its journal is kept in, where it has one already.
    pub file: Option<String>,
    /// How much of it its own journal keeps (`journal::keeps`).
    pub keeps: i64,
    /// What the factory's own database holds of it, by table, and by
    /// name for what is kept beside the rows.
    pub rows: Vec<Counted>,
    /// What of it is under way, each said in a sentence: it is moved
    /// only where there is none.
    pub in_the_way: Vec<String>,
    /// What its own journal's file holds, by table and by name for what
    /// is kept beside the rows, where it has one: how whoever asked
    /// sees that a move took, with no daemon to ask.
    pub kept: Vec<Counted>,
}

impl Project {
    /// How many rows of it the factory's own database holds.
    pub fn held(&self) -> u64 {
        self.rows.iter().map(|counted| counted.moves).sum()
    }

    /// Whether it is kept apart already, with nothing left to move:
    /// its journal keeps as much of it as one made now does, and the
    /// factory's own holds nothing of it.
    pub fn apart(&self) -> bool {
        self.keeps >= journal::keeps::MADE_AT && self.held() == 0
    }

    /// Whether it would be moved: there is something to do, and nothing
    /// in the way.
    pub fn moves(&self) -> bool {
        !self.apart() && self.in_the_way.is_empty()
    }
}

/// What moving the projects of a factory apart would come to.
#[derive(Clone, Debug)]
pub struct Looked {
    pub database: PathBuf,
    /// How big the factory's database is, with its log, in bytes.
    pub bytes: u64,
    /// How much room is left on the disk it is on, in bytes, where that
    /// can be told.
    pub free: Option<u64>,
    /// The projects that were asked about, in the order of their names.
    pub projects: Vec<Project>,
}

impl Looked {
    /// How much room the move needs at most while it is done, in bytes:
    /// what is moved is written into the projects' files before it is
    /// taken out of the factory's, which is not made smaller by it.
    pub fn needs(&self) -> u64 {
        let mut moves = 0u64;
        for project in self.projects.iter().filter(|project| project.moves()) {
            moves += project.held();
        }
        // All there is, of the tables that were asked about for the
        // project with most still to move: each was asked about only
        // what its journal does not keep yet.
        let every = |project: &Project| project.rows.iter().map(|counted| counted.all).sum::<u64>();
        let all = self.projects.iter().map(every).max().unwrap_or(0);
        match all {
            0 => 0,
            all => (self.bytes as u128 * moves.min(all) as u128 / all as u128) as u64 + 1024 * 1024,
        }
    }

    /// About how long it takes, in seconds: as long as the rows that
    /// are moved are many, since each is read, written where it goes
    /// and taken out where it was.
    pub fn seconds(&self) -> u64 {
        let rows: u64 = self.projects.iter().filter(|project| project.moves()).map(Project::held).sum();
        rows.div_ceil(A_SECOND).max(1)
    }

    /// Why it would not be begun, where there is not the room.
    pub fn refused(&self) -> Option<String> {
        let (needs, free) = (self.needs(), self.free?);
        (free < needs).then(|| {
            format!(
                "it needs up to {} MB of room beside the database while it is done, and {} MB are free",
                needs.div_ceil(1024 * 1024),
                free / (1024 * 1024)
            )
        })
    }
}

/// A project that was moved.
#[derive(Clone, Debug)]
pub struct Moved {
    pub project: String,
    /// What its journal is called, and the file it is in.
    pub journal: String,
    pub file: String,
    /// How many rows of each table went, and how much of what is kept
    /// beside them under each name.
    pub rows: Vec<Counted>,
    /// Where the giver of numbers stood: everything said of the project
    /// before the move is at or below it.
    pub at: u64,
    pub took_ms: u64,
}

/// Where a move is stopped, by a test that plays one cut off.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stop {
    /// With the project's file written whole, and nothing decided.
    Written,
    /// With the move decided, and what it leaves not yet taken away.
    Decided,
}

fn wrong(err: rusqlite::Error) -> Error {
    journal::db_error(err).context("looking at what the factory keeps of its projects")
}

fn size(path: &Path) -> u64 {
    path.metadata().map(|file| file.len()).unwrap_or(0)
}

/// Every table that holds rows of a project, by whoever keeps it, and
/// every name something is kept under beside them: the runner's own,
/// and what a way of a comparison has a thing read. Those go wherever
/// rows go.
/// What was said of a project is moved with it where its journal has
/// not kept what is said until now (`keeps`, as its row says): a
/// project whose journal keeps that already has only what is still run
/// for it in the factory's rows moved, with what is kept beside those,
/// and what the factory's conductor said of that meanwhile stays where
/// it was said.
/// What the crates above the runner keep of a project (its tables,
/// their records and each record's history, what a scan saw of its
/// files; its agents' turns) is moved the same way, each table where
/// the project's journal does not keep it yet: every one, for a project
/// with no journal of its own; and for one whose journal keeps what is
/// run and no more, these alone.
fn tables(keeps: i64) -> Vec<Kept> {
    let mut tables = runs::moved::tables();
    tables.extend(runs::moved::beside(&[runs::beside::KEPT, comparisons::copy::KEPT]));
    if keeps < journal::keeps::RUNS {
        tables.push(runs::moved::said());
    }
    let above = studio::moved::tables().into_iter().chain(agents::moved::tables());
    // (And a table some rows of which may be there all the same.)
    tables.extend(above.filter(|(level, kept)| keeps < *level || kept.strays).map(|(_, kept)| kept));
    tables
}

/// What a table that goes with a project holds, in a few words, for
/// whoever is told how much of it goes. One that has none is said by
/// its name.
/// In the order they are said in, after what is run: the tables are
/// handed to the move in the order it is to take them out in.
const ROWS: &[(&str, &str)] = &[
    ("collections", "tables"),
    ("records", "records"),
    ("record_history", "lines of their history"),
    ("project_files", "files a scan last saw"),
    ("turns", "turns of its agents"),
    ("events", "events said of it"),
];

/// What a project's own journal keeps of it at the level `keeps`
/// (`journal::keeps`), in words: nothing, for one that keeps none of it.
pub fn keeping(keeps: i64) -> Option<&'static str> {
    match keeps {
        keeps if keeps >= journal::keeps::TURNS => Some(
            "what is run for it, what is said of it, its tables with their records, and what its agents \
             were asked",
        ),
        keeps if keeps >= journal::keeps::RECORDS => {
            Some("what is run for it, what is said of it and its tables with their records")
        }
        keeps if keeps >= journal::keeps::RUNS => Some("what is run for it and what is said of it"),
        _ => None,
    }
}

/// The rows of `table`, as they are said to a person.
pub fn rows_of(table: &str) -> String {
    match ROWS.iter().find(|(kept, _)| *kept == table) {
        Some((_, words)) => words.to_string(),
        None => table.replace('_', " "),
    }
}

/// What is kept under a name that goes with a project, in a few words,
/// for whoever is told how much of it goes.
const CALLED: &[(&str, &str)] = &[
    (runs::beside::PINNED, "trees held to the texts they reached"),
    (runs::beside::TEXTS, "texts they are held to"),
    (runs::beside::VOUCHED, "trees with files of the project's own vouched for"),
    (runs::beside::POLL, "readings a handler tells its next from"),
    (runs::beside::AT, "times a handler last fired at"),
    (runs::beside::ASKED, "runs a step asked for, by what the asking was named"),
    (runs::beside::FITS, "texts that last checked"),
    (runs::beside::FORKED, "forks, with what each was forked from"),
    (runs::beside::PROJECT_ENVIRONMENTS, "environments, with their variables"),
];

/// What is kept under `name` beside a project's rows, as it is said to
/// a person; a name nobody gave words is said as it is.
pub fn said_of(name: &str) -> String {
    match CALLED.iter().find(|(kept, _)| *kept == name) {
        Some((_, words)) => words.to_string(),
        None => format!("kept under `{name}`"),
    }
}

/// How much went, or would go, of each table (`beside` is not) or under
/// each name of what is kept beside the rows (`beside` is), leaving out
/// what the database keeps by itself.
pub fn went(counted: &[Counted], beside: bool) -> Vec<(String, u64)> {
    let follows: Vec<&str> =
        runs::moved::tables().iter().filter(|kept| kept.follows).map(|kept| kept.table).collect();
    let meant = counted
        .iter()
        .filter(|counted| counted.beside == beside && !follows.contains(&counted.table.as_str()));
    let mut went: Vec<(String, u64)> = meant.map(|counted| (counted.table.clone(), counted.moves)).collect();
    if !beside {
        // What is run first, as it was handed over; then what has words
        // of its own, in the order of those.
        went.sort_by_key(|(table, _)| ROWS.iter().position(|(kept, _)| kept == table).map(|at| at + 1));
    }
    went
}

/// How much a project's own file holds of each table that is moved,
/// and under each name of what is kept beside the rows: nothing, for a
/// file that is not there or keeps none of them.
fn held_in(file: &Path) -> Vec<Counted> {
    if !file.is_file() {
        return Vec::new();
    }
    let Ok(connection) = journal::moved::open(file) else { return Vec::new() };
    let tables = tables(journal::keeps::NOTHING);
    let held = tables.iter().filter(|kept| !kept.follows);
    held.filter_map(|kept| kept.holds(&connection).ok()).collect()
}

/// Holds a factory to the tables this program keeps: one that an
/// earlier program ran last has not had them brought up, which a daemon
/// does as it starts and nothing here does.
fn current(connection: &journal::copy::Connection) -> Result<()> {
    let has = journal::copy::versions(connection).map_err(wrong)?;
    let (above, turns) = (studio::moved::knows(), agents::copy::KNOWS);
    for (namespace, version) in
        journal::copy::KNOWS.iter().chain(runs::copy::KNOWS).chain(&above).chain(turns)
    {
        let at = has.iter().find(|(name, _)| name == namespace).map(|(_, at)| *at).unwrap_or(0);
        if (at as usize) < *version {
            return Err(Error::conflict(format!(
                "this factory was last run by an earlier program, and its tables are brought up by the daemon \
                 as it starts: start it once with this program (`{cli} up`), stop it (`{cli} down`), and ask again",
                cli = brand::CLI
            )));
        }
        if (at as usize) > *version {
            return Err(Error::conflict(
                "this factory was last run by a later program than this one, which does not know its tables",
            ));
        }
    }
    Ok(())
}

/// The projects a factory has: those it lists, those something was
/// applied for, and those with a journal of their own.
fn projects(connection: &journal::copy::Connection) -> Result<Vec<String>> {
    let mut all = runs::moved::projects(connection).map_err(wrong)?;
    all.extend(journal::listed(connection).map_err(wrong)?.into_iter().map(|listed| listed.project));
    // The list of projects is the studio's, read here as it is in a
    // copy: by a connection to a factory that is stopped.
    let listed: bool = connection
        .query_row(
            "SELECT EXISTS (SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'projects')",
            [],
            |row| row.get(0),
        )
        .map_err(wrong)?;
    if listed {
        let mut statement = connection.prepare("SELECT id FROM projects").map_err(wrong)?;
        let ids = statement.query_map([], |row| row.get::<_, String>(0)).map_err(wrong)?;
        all.extend(ids.collect::<rusqlite::Result<Vec<_>>>().map_err(wrong)?);
    }
    all.sort();
    all.dedup();
    Ok(all)
}

fn looked(data: &Path, only: Option<&str>) -> Result<Looked> {
    let database = data.join(DATABASE_FILE);
    let connection = journal::moved::open(&database)?;
    current(&connection)?;
    let journals = journal::listed(&connection).map_err(wrong)?;
    let mut ids = projects(&connection)?;
    if let Some(only) = only {
        if !ids.iter().any(|id| id == only) {
            return Err(Error::not_found(format!(
                "this factory has no project `{only}`{}",
                match ids.is_empty() {
                    true => String::new(),
                    false => format!(
                        ": it has {}",
                        ids.iter().map(|id| format!("`{id}`")).collect::<Vec<_>>().join(", ")
                    ),
                }
            )));
        }
        ids.retain(|id| id == only);
    }
    let mut projects = Vec::new();
    for id in ids {
        let listed = journals.iter().find(|listed| listed.project == id);
        let tables = tables(listed.map_or(journal::keeps::NOTHING, |listed| listed.keeps));
        let rows = journal::moved::count(&connection, &id, &tables).map_err(wrong)?;
        let mut in_the_way = Vec::new();
        for run in runs::moved::under_way(&connection, &id).map_err(wrong)? {
            let what = match run.flow.strip_prefix("on.") {
                Some(handler) => format!("its handler `{handler}`"),
                None => "its steps".to_string(),
            };
            in_the_way
                .push(format!("`{}` (`{}`, {}) has {what} under way", run.blueprint, run.key, run.instance));
        }
        for turn in agents::moved::in_hand(&connection, &id).map_err(wrong)? {
            in_the_way
                .push(format!("a turn of the agent `{}` is {} ({})", turn.agent, turn.status, turn.turn));
        }
        let kept = listed.map(|listed| held_in(&data.join(JOURNALS).join(&listed.file))).unwrap_or_default();
        projects.push(Project {
            kept,
            id,
            file: listed.map(|listed| listed.file.clone()),
            keeps: listed.map_or(journal::keeps::NOTHING, |listed| listed.keeps),
            rows,
            in_the_way,
        });
    }
    let bytes = size(&database) + size(&data.join(format!("{DATABASE_FILE}-wal")));
    let free = hosting::resources(data)
        .ok()
        .and_then(|found| found.free_disk_mb)
        .map(|mb| mb.saturating_mul(1024 * 1024));
    Ok(Looked { database, bytes, free, projects })
}

const HELD_FOR: &str = "a project is moved to a journal of its own";

/// What moving the projects of the factory at `data` apart would come
/// to: every project, or `only` the one named. Nothing is changed by
/// asking; a factory that is running is not asked about.
pub fn look(data: &Path, only: Option<&str>) -> Result<Looked> {
    let _held = crate::repack::hold(data, HELD_FOR)?;
    looked(data, only)
}

/// Why a project that was asked for by name is not moved.
fn in_the_way(project: &Project) -> Error {
    Error::conflict(format!(
        "`{}` is moved only at rest, and something of it is under way: {}. Start the factory and let that \
         finish, or stop it (`{cli} stop <instance>`, `{cli} agent stop <name>`); then stop the factory and \
         ask again. Nothing was moved",
        project.id,
        project.in_the_way.join("; "),
        cli = brand::CLI
    ))
}

/// Moves the projects of the factory at `data` to journals of their
/// own: every project that is at rest and not apart already, or `only`
/// the one named, which is refused where something of it is under way.
/// `begin` is told what was found first, and what it prints is what
/// whoever asked reads before anything is done. `stop` is for a test
/// that plays a move cut off. The projects that were moved.
pub async fn run(
    data: &Path,
    only: Option<&str>,
    stop: Option<Stop>,
    begin: impl FnOnce(&Looked),
) -> Result<Vec<Moved>> {
    let _held = crate::repack::hold(data, HELD_FOR)?;
    // What a move that was cut off left is nobody's, and is taken away
    // before another is begun.
    clear(data);
    let asked = looked(data, only)?;
    begin(&asked);
    if let (Some(_), Some(project)) = (only, asked.projects.first())
        && !project.in_the_way.is_empty()
    {
        return Err(in_the_way(project));
    }
    if asked.projects.iter().any(Project::moves)
        && let Some(why) = asked.refused()
    {
        return Err(Error::conflict(why));
    }
    let folder = data.join(JOURNALS);
    let mut moved = Vec::new();
    for project in asked.projects.iter().filter(|project| project.moves()) {
        let done = one(&asked.database, &folder, project, &tables(project.keeps), stop).await?;
        let Some(done) = done else { return Ok(moved) };
        moved.push(done);
    }
    Ok(moved)
}

/// Moves one project, or stops where it was told to.
async fn one(
    database: &Path,
    folder: &Path,
    project: &Project,
    tables: &[Kept],
    stop: Option<Stop>,
) -> Result<Option<Moved>> {
    let began = std::time::Instant::now();
    let clock: Arc<dyn Clock> = Arc::new(SystemClock);
    let name = journal::of_project(&project.id);
    std::fs::create_dir_all(folder)?;
    let file = journal::moved::file_for(folder, &project.id);
    let place = folder.join(&file);
    let before = project.file.as_ref().map(|file| folder.join(file));
    // 1. The project's file, whole: what its journal held where it had
    // one, the tables as they are made now, and every row of it, with
    // what is kept beside them.
    let blocking = |err: tokio::task::JoinError| Error::internal(format!("moving a project crashed: {err}"));
    if let Some(before) = before.clone().filter(|before| before.is_file()) {
        let to = place.clone();
        tokio::task::spawn_blocking(move || journal::copy::take(&before, &to)).await.map_err(blocking)??;
    }
    {
        let made = journal::moved::made(&place, &name, clock.clone()).await?;
        runs::Store::open(&made).await?;
        // And the tables of what the crates above the runner keep of a
        // project, by the steps a daemon makes them by in such a file.
        studio::apart::make(made.db()).await?;
        agents::apart::make(made.db()).await?;
        let db = made.db().clone();
        drop(made);
        db.close();
        db.closed().await;
    }
    let placed = runs::moved::placed();
    let carried = {
        let (database, place, id, tables, placed) =
            (database.to_path_buf(), place.clone(), project.id.clone(), tables.to_vec(), placed.clone());
        tokio::task::spawn_blocking(move || journal::moved::carry(&database, &place, &id, &tables, &placed))
            .await
            .map_err(blocking)??
    };
    // Whatever of another project hears this one kept a place in its
    // journal, which had nothing said in it: where what was said is
    // moved, those are taken away too, each in its own file.
    if tables.iter().any(|kept| kept.said) {
        let (database, folder, placed, name) =
            (database.to_path_buf(), folder.to_path_buf(), placed.clone(), name.clone());
        let mine = [Some(file.clone()), project.file.clone()];
        tokio::task::spawn_blocking(move || {
            let others: Vec<String> = journal::listed(&journal::moved::open(&database)?)
                .map_err(wrong)?
                .into_iter()
                .map(|listed| listed.file)
                .filter(|file| !mine.contains(&Some(file.clone())))
                .collect();
            journal::moved::unplace_beside(&folder, &others, &placed, &name)
        })
        .await
        .map_err(blocking)??;
    }
    if stop == Some(Stop::Written) {
        return Ok(None);
    }
    // 2. The one write that decides it.
    let row = Row {
        name: name.clone(),
        project: project.id.clone(),
        file: file.clone(),
        // Everything a journal made now keeps was moved, whichever of
        // it the project's journal kept already.
        keeps: journal::keeps::MADE_AT,
        made: clock.now_ms(),
    };
    let by_name = |beside: bool| -> serde_json::Map<String, Value> {
        went(&carried, beside).into_iter().map(|(name, moves)| (name, json!(moves))).collect()
    };
    let note =
        json!({ "moved": project.id, "journal": name, "rows": by_name(false), "beside": by_name(true) });
    let at = {
        let (database, tables, carried) = (database.to_path_buf(), tables.to_vec(), carried.clone());
        tokio::task::spawn_blocking(move || {
            journal::moved::decide(&database, &row, &tables, &placed, &carried, REMEMBERED, &note, began)
        })
        .await
        .map_err(blocking)??
    };
    if stop == Some(Stop::Decided) {
        return Ok(None);
    }
    // 3. The file its journal was in before, which nothing names now.
    if let Some(before) = before.filter(|before| *before != place) {
        journal::moved::put_away(&before);
    }
    Ok(Some(Moved {
        project: project.id.clone(),
        journal: name,
        file,
        rows: carried,
        at,
        took_ms: began.elapsed().as_millis() as u64,
    }))
}

/// Takes away the files beside the factory's database that no project's
/// journal is kept in: what a move that was cut off left, before or
/// after it was decided. Whoever calls this holds the folder's lock and
/// has not opened the database: the daemon, as it starts, and the move
/// as it is asked again. How many files, and how big they were.
pub fn clear(data: &Path) -> Option<(usize, u64)> {
    journal::moved::clear(&data.join(DATABASE_FILE), &data.join(JOURNALS))
}

/// Says that they were taken away, in the factory's own journal.
pub async fn announce_cleared(journal: &journal::Journal, cleared: Option<(usize, u64)>) {
    if let Some((files, bytes)) = cleared {
        let said = json!({ "files": files, "bytes": bytes });
        let _ = journal.emit("system.move_cleared", Source::actor("system"), said).await;
    }
}

/// Says, once for each, in the factory's own journal, that a project
/// was moved to a journal of its own while the daemon was stopped, and
/// measures how long it took.
pub async fn announce(journal: &journal::Journal) {
    let Some(done) = journal.kv_get(REMEMBERED.0, REMEMBERED.1).await.ok().flatten() else { return };
    for moved in done.as_array().into_iter().flatten() {
        journal.measure("db.moved_apart", &[], moved["took_ms"].as_f64().unwrap_or_default(), true);
        let _ = journal.emit("system.moved_apart", Source::actor("system"), moved.clone()).await;
    }
    let _ = journal.kv_set(REMEMBERED.0, REMEMBERED.1, &Value::Null).await;
}

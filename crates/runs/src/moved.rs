//! What of a project is kept here, for whoever moves a project of a
//! factory from before to a journal of its own (`journal::moved`).
//!
//! The move knows no table. It is handed every table kept here with how
//! a row of each says whose it is ([`crate::apart::TABLES`], which is
//! what the rows are held to), and what of a project is under way,
//! which is what a move waits for: a run under way keeps numbers that
//! are places in the journal it began in.
//!
//! What a conductor keeps beside its rows goes with them ([`beside`]),
//! by what each name's line says of whose it is
//! ([`crate::beside::KEPT`]): a project's conductor looks in its own
//! journal for what its trees are held to, what was vouched for them
//! and what their handlers last read, and a project moved without them
//! would be run as one nobody had held to anything.

use journal::moved::{Kept, Placed};
use rusqlite::Connection;

use crate::apart::{TABLES, Told};
use crate::beside::{OWED, PINNED, Whose};

/// What the database keeps by its own rules, from the rows of another
/// table: it follows them where they go and is written by nobody.
const FOLLOWS: &[&str] = &["alive", "under_way"];

/// Every table kept here, as the move is handed them.
pub fn tables() -> Vec<Kept> {
    TABLES
        .iter()
        .map(|(table, told)| {
            let depth = match told {
                Told::Says => 0,
                Told::Instance(_) | Told::SaysOrInstance(_) => 1,
                Told::Run(_) => 2,
            };
            Kept {
                says: matches!(told, Told::SaysOrInstance(_)),
                follows: FOLLOWS.contains(table),
                ..Kept::rows(table, told.reads(), depth)
            }
        })
        .collect()
}

/// The instance a key of something kept by instance is of, as the
/// database is asked it of a row called `row`: the key, as far as the
/// first `/` or line break ([`crate::beside::instance_of`]).
fn instance_of(row: &str) -> String {
    let (key, line) = (format!("{row}.key"), "char(10)");
    format!(
        "CASE WHEN instr({key}, '/') > 0 AND (instr({key}, {line}) = 0 OR instr({key}, '/') < instr({key}, {line}))
              THEN substr({key}, 1, instr({key}, '/') - 1)
              WHEN instr({key}, {line}) > 0 THEN substr({key}, 1, instr({key}, {line}) - 1)
              ELSE {key} END"
    )
}

/// The project of the instance a key of `row` is of, where the rows
/// that are asked hold that instance.
fn of_instance(row: &str) -> String {
    format!("(SELECT project FROM instances WHERE id = {})", instance_of(row))
}

/// What is kept beside the rows under the names of `lists`, as the move
/// is handed it: a line for each name that goes with a project, by what
/// its own line says of whose it is.
///
/// - An instance's goes where the instance's rows go, over anything
///   the file it goes to has under its key: it was kept by the
///   conductor that had the instance until now.
/// - A project's goes with the project
///   ([`crate::beside::project_of`]), unless the file it goes to has
///   something under its key: that is what the project's own conductor
///   kept, of what is applied for the project now.
/// - The conductor's own, with what is there, is a text by its
///   revision: it is written where the project goes for each of the
///   project's trees that is held to it, which is where what is pinned
///   for one of the project's instances names the revision anywhere,
///   and is left where it was for whatever else is held to it there;
///   each conductor puts away what nothing of its own is held to as it
///   starts. A journal's note of itself ([`OWED`]) says nothing of
///   another journal, and stays.
/// - The factory's stays.
///
/// What is the project's is taken out of the factory's file in the
/// write that takes its rows out, so no note is left for a conductor to
/// bring anything over by.
pub fn beside(lists: &[&[(&'static str, Whose, &'static str)]]) -> Vec<Kept> {
    let kept = lists.iter().flat_map(|list| list.iter());
    kept.filter_map(|(name, whose, _)| match whose {
        Whose::Instance => Some(Kept::beside(name, of_instance("kept"), 1)),
        Whose::Project => {
            let whose = "CASE WHEN instr(kept.key, char(10)) > 1
                              THEN substr(kept.key, 1, instr(kept.key, char(10)) - 1) END";
            // What the conductor of the file it goes to kept for the
            // project already is of what is applied for it now.
            Some(Kept::beside(name, whose.to_string(), 0).yielding())
        }
        Whose::Conductor if *name == OWED.0 => None,
        Whose::Conductor => {
            // What is pinned is read as whatever it is: a revision is
            // found wherever a pin names one, so that something a pin
            // comes to name later is not left behind by this.
            let whose = format!(
                "(SELECT ?1 WHERE EXISTS (
                    SELECT 1 FROM kv pins, json_tree(CASE WHEN json_valid(pins.value) THEN pins.value END) named
                    WHERE pins.namespace = '{PINNED}' AND {} IS ?1 AND named.atom = kept.key))",
                of_instance("pins")
            );
            Some(Kept::beside(name, whose, 2).staying())
        }
        Whose::Factory => None,
    })
    .collect()
}

/// What was said of a project, as the move is handed it: an event is
/// the project's that its data names, else that of the instance that
/// said it, as an event is kept now (`Journal::emit` keeps what names a
/// project in that project's journal, and a project's conductor says
/// what it runs in its own). An event that names no project and was
/// said by nothing of one stays the factory's.
pub fn said() -> Kept {
    journal::moved::said(
        "coalesce(nullif(json_extract(kept.data, '$.project'), ''), \
         (SELECT project FROM instances WHERE instances.id = kept.instance))"
            .to_string(),
    )
}

/// The tables kept here that keep places among a journal's events: a
/// handler's place in each journal it hears.
pub fn placed() -> Vec<Placed> {
    vec![Placed { table: "places", journal: "journal" }]
}

/// A run of a project that is under way, as a factory that is stopped
/// keeps it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnderWay {
    pub run: String,
    /// What it is a run of: its steps, or a handler by its name.
    pub flow: String,
    pub instance: String,
    pub blueprint: String,
    /// What the instance is called where it was started.
    pub key: String,
    /// When it began, in milliseconds.
    pub started: i64,
}

/// The runs of `project` that are under way, in the order they were
/// made, on a connection to a factory's database that is stopped.
pub fn under_way(connection: &Connection, project: &str) -> rusqlite::Result<Vec<UnderWay>> {
    let mut statement = connection.prepare(
        "SELECT runs.id, runs.flow, instances.id, instances.blueprint, instances.key, runs.started
         FROM under_way CROSS JOIN runs ON runs.id = under_way.run
         LEFT JOIN instances ON instances.id = runs.instance
         WHERE under_way.project = ?1 ORDER BY runs.rowid",
    )?;
    let rows = statement.query_map([project], |row| {
        Ok(UnderWay {
            run: row.get(0)?,
            flow: row.get(1)?,
            instance: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
            blueprint: row.get::<_, Option<String>>(3)?.unwrap_or_default(),
            key: row.get::<_, Option<String>>(4)?.unwrap_or_default(),
            started: row.get(5)?,
        })
    })?;
    rows.collect()
}

/// Every project that something kept here says, in the order of their
/// names: what was applied for each.
pub fn projects(connection: &Connection) -> rusqlite::Result<Vec<String>> {
    let mut statement = connection
        .prepare("SELECT DISTINCT project FROM instances WHERE project IS NOT NULL ORDER BY project")?;
    statement.query_map([], |row| row.get(0))?.collect()
}

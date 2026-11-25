//! A project of a factory from before, moved to a journal of its own.
//!
//! A project this program adds has a journal of its own from the start
//! ([`crate::Journal::own`]). One that a program from before added has
//! none: everything kept of it is in the factory's own journal, and stays
//! there until its owner asks for it to be moved apart. This is the
//! moving, which is asked of a factory that is stopped, by whoever holds
//! its folder; nothing here is ever done by opening a database.
//!
//! **It knows no table.** Whoever keeps rows of a project says which
//! tables, and how a row of each says whose it is ([`Kept`]); what is
//! here writes those rows into the project's file and takes them out of
//! the factory's. What is kept of a project beside its rows, under a
//! name of the small durable state, is a line of the same list
//! ([`Kept::beside`]) and goes the same way, in the same two writes: or
//! a project's conductor would look in its own journal for what the
//! factory's still held.
//!
//! **A row goes as it is**, every column, in the order the rows came
//! in, but for what its line says otherwise: a column whose row said
//! something only by where it was in the factory's file is written out
//! ([`Kept::writes`]), and a table whose keeper goes by the order of its
//! rows is written in the order its line says ([`Kept::order`]). And a
//! row that is told apart through another table that moves is written
//! after it and taken out before it ([`Kept::depth`]).
//!
//! **Two files are not one write, so one write decides.**
//!
//! 1. The project's file is made whole beside the factory's, as a
//!    journal's file is made now ([`made`]), and every row of the project
//!    is written into it ([`carry`]), counted, read back and held to the
//!    disk. Until the next step it is a file nothing names.
//! 2. One transaction of the factory's database ([`decide`]): the
//!    project's row of `journals` (its file, how much of the project the
//!    file keeps, that it was moved, and where the giver of numbers
//!    stood), and the project's rows taken out of the factory's tables,
//!    as many of each as were written into the file or none at all.
//!    Before it the factory is as it was. After it the project is moved.
//! 3. What the row no longer names is taken away.
//!
//! Cut off before the second step, a file lies in the folder that no row
//! names; cut off after it, perhaps the file the project had before. A
//! file of that folder that no row names is nobody's, and is taken away
//! by whoever next holds the folder ([`clear`]): the move asked again, or
//! the daemon as it starts, before it opens anything.
//!
//! **What was said of the project is moved as its rows are**, where its
//! journal had kept nothing said until then: the journal's own events
//! are one more table handed to this ([`said`]), written into the
//! project's file and taken out of the factory's in the same two steps.
//! An event keeps its number where it goes, and the giver of numbers is
//! not touched: the project's journal draws from it as any does, above
//! everything before. Three things are written down for that, each by
//! the step that makes it so: the highest number the factory's journal
//! ever gave, before any is taken out; that the events of both journals
//! have holes the list of gaps does not say; and, in the project's row,
//! where the giver stood, which is what a place kept in the factory's
//! journal comes to in the project's ([`crate::joined`]). A place that
//! anything kept in the project's journal while it held nothing said is
//! taken away with the move, in every file ([`Placed`]), so that it is
//! worked out by that rule and not from the beginning.

use std::path::{Path, PathBuf};

use kernel::{Clock, Error, Result};
use rusqlite::Connection;
use serde_json::Value;

use crate::db::db_error;
use crate::{Beside, Db, Journal, giver, numbered, set};

/// A table that holds rows of projects, as whoever keeps it says.
#[derive(Clone, Debug)]
pub struct Kept {
    pub table: &'static str,
    /// The name of the small durable state what is meant is kept
    /// under, where the table is that one: a line for each name, since
    /// each says whose its keys are another way. Nothing for a table
    /// every row of which is meant.
    pub under: Option<&'static str>,
    /// What reads the project of a row called `kept`, as the database is
    /// asked it: nothing for a row that is nobody's. Where a row may be
    /// kept for several projects it is asked of the one in hand, `?1`.
    pub whose: String,
    /// Whether a row says its project in a column `project` that one a
    /// program from before wrote may have left unsaid: it is written as
    /// the project where it is moved, since the file it goes to is held
    /// to every row saying whose it is.
    pub says: bool,
    /// How many other tables are asked to tell whose a row is. Rows are
    /// written the shallowest first and taken out the deepest first, so
    /// that what a row is reached through is there when it is asked.
    pub depth: u8,
    /// Kept by the database itself from the rows of another table: it
    /// is neither written nor taken out, and is held to have followed.
    pub follows: bool,
    /// Kept once in each file, for what is there: it is written where
    /// the project goes and left where it was, for whatever else of
    /// that file it is kept for. Whose it is, is said through what
    /// goes, so that none of it is the project's once that has gone.
    pub stays: bool,
    /// Of what is kept under a name: whether a key that is there
    /// already where the project goes is left as that file has it.
    /// Otherwise what goes is written over it, since it is kept for
    /// rows that go, by the conductor that had them until now.
    pub yields: bool,
    /// The journal's own events: each keeps its number where it goes,
    /// and both journals write down what that leaves ([`said`]).
    pub said: bool,
    /// Columns that are written another way than as the row holds them,
    /// each with what is written in it, read of a row called `kept`:
    /// what a row said only by where it was in the file it leaves (which
    /// row of its table it was) is written out where it goes.
    pub writes: Vec<(&'static str, String)>,
    /// What the rows are written in the order of, read of a row called
    /// `kept`, where that is not the order they came in: for a table
    /// whose keeper goes by a row's place in its file.
    pub order: Option<String>,
    /// Whether rows of the project may be in the factory's file though
    /// the project's own journal keeps the table: the line is then
    /// handed over whatever that journal keeps, and what it brings is
    /// written among the rows there already, every one of them again,
    /// in the line's order.
    pub strays: bool,
}

impl Kept {
    /// The rows of `table`, with how a row called `kept` says whose it
    /// is and how many other tables that is asked of ([`Kept::depth`]):
    /// each written as it is, in the order it came in.
    pub fn rows(table: &'static str, whose: String, depth: u8) -> Kept {
        let (says, follows, stays, yields, said) = (false, false, false, false, false);
        let (writes, order, strays) = (Vec::new(), None, false);
        Kept { table, under: None, whose, says, depth, follows, stays, yields, said, writes, order, strays }
    }

    /// What is kept under `name` in the small durable state, with how
    /// a key there says whose it is: read of a row called `kept`, as of
    /// any table.
    pub fn beside(name: &'static str, whose: String, depth: u8) -> Kept {
        Kept { under: Some(name), ..Kept::rows("kv", whose, depth) }
    }

    /// The same, saying its project where it goes ([`Kept::says`]).
    pub fn saying(self) -> Kept {
        Kept { says: true, ..self }
    }

    /// The same, with `column` written as `how` reads of a row called
    /// `kept` ([`Kept::writes`]).
    pub fn writing(mut self, column: &'static str, how: impl Into<String>) -> Kept {
        self.writes.push((column, how.into()));
        self
    }

    /// The same, written in the order of what `by` reads of a row
    /// called `kept` ([`Kept::order`]).
    pub fn ordered(self, by: impl Into<String>) -> Kept {
        Kept { order: Some(by.into()), ..self }
    }

    /// The same, looked for whatever the project's journal keeps
    /// ([`Kept::strays`]).
    pub fn straying(self) -> Kept {
        Kept { strays: true, ..self }
    }

    /// The same, giving way to a key that is there already where the
    /// project goes ([`Kept::yields`]).
    pub fn yielding(self) -> Kept {
        Kept { yields: true, ..self }
    }

    /// The same, written where the project goes and left where it was
    /// ([`Kept::stays`]): what is kept once in each file under a key is
    /// the same thing in both, so it gives way too.
    pub fn staying(self) -> Kept {
        Kept { stays: true, yields: true, ..self }
    }

    /// What it is counted as: its table, or the name it is kept under.
    pub fn called(&self) -> &'static str {
        self.under.unwrap_or(self.table)
    }

    /// Which rows of its table are meant at all, of a row called `kept`.
    fn part(&self) -> String {
        match self.under {
            Some(name) => format!("kept.namespace = '{}'", name.replace('\'', "''")),
            None => "1".to_string(),
        }
    }

    /// How much of this line a database holds, whoever's it is: every
    /// row of its table, or every key under its name.
    pub fn holds(&self, connection: &Connection) -> rusqlite::Result<Counted> {
        let asked = format!("SELECT count(*) FROM {} kept WHERE {}", quoted(self.table), self.part());
        let rows = connection.query_row(&asked, [], |row| row.get::<_, i64>(0))? as u64;
        Ok(Counted { table: self.called().to_string(), beside: self.under.is_some(), moves: rows, all: rows })
    }

    /// Whether `counted` is the count of this line.
    fn is(&self, counted: &Counted) -> bool {
        counted.table == self.called() && counted.beside == self.under.is_some()
    }
}

/// The journal's own events, as a table that is moved: `whose` reads
/// the project of an event called `kept`, which whoever keeps what the
/// events are of says (the project its data names, else that of what
/// said it). They are written last and taken out first, since whose an
/// event is may be read through any other table.
///
/// An event keeps its number where it goes. So the journal it leaves
/// writes down the highest number it ever gave before any is taken out
/// (`numbered::keep_said`: a journal goes on from above what it holds,
/// and what it held is no longer there to say), and both journals write
/// down that their events have holes all through them
/// (`numbered::HOLES`), which the newest so many of each are then
/// counted over from the rows. The giver of numbers is not touched: what
/// either journal says next is drawn above everything there is.
///
/// What is the factory's for good ([`crate::HERE`]: what is said of a
/// channel, and that a project was put in a workspace) stays, whatever
/// project it names: its row is kept in the factory's file, and an event
/// is kept where its row is.
pub fn said(whose: String) -> Kept {
    let here: Vec<String> = crate::HERE.iter().map(|begins| format!("kept.topic LIKE '{begins}%'")).collect();
    let whose = format!("CASE WHEN {} THEN NULL ELSE {whose} END", here.join(" OR "));
    Kept { said: true, ..Kept::rows("events", whose, u8::MAX) }
}

/// A table that keeps places among a journal's events, as whoever keeps
/// it says: which column names the journal and which the place.
#[derive(Clone, Debug)]
pub struct Placed {
    pub table: &'static str,
    pub journal: &'static str,
}

/// Takes away every place kept in the journal called `journal`, in the
/// database called `schema` on this connection, where it keeps such a
/// table at all. Asked as a project's events are moved into a journal
/// that had kept nothing said until then: a place there says nothing
/// was heard, since there was nothing, and one left where it is would
/// have its keeper hear everything that is moved in. With none kept,
/// the keeper's place there is worked out from its place in the journal
/// the events were moved from ([`crate::joined`]), once, and kept.
fn unplace(
    connection: &Connection,
    schema: &str,
    placed: &[Placed],
    journal: &str,
) -> rusqlite::Result<usize> {
    let mut gone = 0;
    for placed in placed {
        let there: bool = connection.query_row(
            &format!(
                "SELECT EXISTS (SELECT 1 FROM {schema}.sqlite_master WHERE type = 'table' AND name = ?1)"
            ),
            [placed.table],
            |row| row.get(0),
        )?;
        if there {
            gone += connection.execute(
                &format!(
                    "DELETE FROM {schema}.{} WHERE {} = ?1",
                    quoted(placed.table),
                    quoted(placed.journal)
                ),
                [journal],
            )?;
        }
    }
    Ok(gone)
}

/// The same in each of the `files` of `folder`, which are other
/// projects' journals: something of theirs may hear this project too.
/// Done before the move is decided: cut off after it, a place that was
/// taken away is worked out again as it would have been, since the
/// journal it was kept in still holds nothing.
pub fn unplace_beside(folder: &Path, files: &[String], placed: &[Placed], journal: &str) -> Result<()> {
    for file in files {
        let path = folder.join(file);
        if !path.is_file() {
            continue;
        }
        let wrong = |err: rusqlite::Error| db_error(err).context(path.display());
        let connection = open(&path)?;
        unplace(&connection, "main", placed, journal).map_err(wrong)?;
        let _: rusqlite::Result<i64> =
            connection.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| row.get(0));
        connection.close().map_err(|(_, err)| wrong(err))?;
    }
    Ok(())
}

/// How many rows of a table are a project's, and how many it holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Counted {
    /// The table; or the name, for what is kept under a name of the
    /// small durable state.
    pub table: String,
    /// Whether it is such a name, and no table.
    pub beside: bool,
    pub moves: u64,
    pub all: u64,
}

/// What the row of `journals` is to say once a project is moved.
#[derive(Clone, Debug)]
pub struct Row {
    /// What the journal is called ([`crate::of_project`]).
    pub name: String,
    pub project: String,
    /// Its file, in the folder the journals are kept in.
    pub file: String,
    /// How much of the project the file keeps ([`crate::keeps`]).
    pub keeps: i64,
    /// When, for a row that is new.
    pub made: i64,
}

fn text(path: &Path) -> Result<&str> {
    path.to_str().ok_or_else(|| {
        Error::invalid(format!(
            "{} cannot be written as text, which a database's place has to be",
            path.display()
        ))
    })
}

fn with(path: &Path, ending: &str) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(ending);
    PathBuf::from(name)
}

/// Opens a factory's database, or a journal's file, that nothing else
/// has open: the folder is held by whoever asks.
pub fn open(path: &Path) -> Result<Connection> {
    let wrong = |err: rusqlite::Error| db_error(err).context(path.display());
    let connection = Connection::open(path).map_err(wrong)?;
    connection.busy_timeout(std::time::Duration::from_secs(5)).map_err(wrong)?;
    connection
        .query_row("SELECT count(*) FROM sqlite_master", [], |row| row.get::<_, i64>(0))
        .map_err(|err| db_error(err).context(format!("{} is not a usable database", path.display())))?;
    Ok(connection)
}

fn quoted(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

/// Makes the file a project's journal is kept in, called `name`, as one
/// is made now, or opens the one there is under that name; with the
/// journal's own tables in it and nothing said. Whoever keeps rows there
/// makes its tables next, by opening what it opens on any journal, and
/// closes it before anything is carried.
pub async fn made(file: &Path, name: &str, clock: std::sync::Arc<dyn Clock>) -> Result<Journal> {
    Journal::open_with(Db::open(file)?, clock, String::new(), name.to_string(), Beside::Nothing).await
}

/// The file a project that is moved is given, in `folder`: the one a
/// project is given when it is added, or, where a file of that name is
/// there (the project had a journal that kept less), the first after it
/// that is not.
pub fn file_for(folder: &Path, project: &str) -> String {
    let usual = set::file_of(project);
    let stem = usual.trim_end_matches(".db").to_string();
    let mut file = usual;
    let mut n = 1;
    while folder.join(&file).exists() {
        n += 1;
        file = format!("{stem}.{n}.db");
    }
    file
}

/// How many rows of each table are `project`'s, in a factory's database.
pub fn count(connection: &Connection, project: &str, tables: &[Kept]) -> rusqlite::Result<Vec<Counted>> {
    tables
        .iter()
        .map(|kept| {
            connection.query_row(
                &format!(
                    "SELECT count(*) FILTER (WHERE {} IS ?1), count(*) FROM {} kept WHERE {}",
                    kept.whose,
                    quoted(kept.table),
                    kept.part()
                ),
                [project],
                |row| {
                    Ok(Counted {
                        table: kept.called().to_string(),
                        beside: kept.under.is_some(),
                        moves: row.get::<_, i64>(0)? as u64,
                        all: row.get::<_, i64>(1)? as u64,
                    })
                },
            )
        })
        .collect()
}

/// The columns of a table, in the order they were made.
fn columns(connection: &Connection, schema: &str, table: &str) -> rusqlite::Result<Vec<String>> {
    let mut statement =
        connection.prepare(&format!("SELECT name FROM {schema}.pragma_table_info({})", quoted(table)))?;
    statement.query_map([], |row| row.get(0))?.collect()
}

/// Whether the rows of a table are numbered by the database, in the
/// order they were written: such a table is read in that order, so that
/// the rows are in the same order where they go.
fn in_order(connection: &Connection, table: &str) -> rusqlite::Result<bool> {
    let made: String = connection.query_row(
        "SELECT sql FROM main.sqlite_master WHERE type = 'table' AND name = ?1",
        [table],
        |row| row.get(0),
    )?;
    Ok(!made.to_ascii_uppercase().contains("WITHOUT ROWID"))
}

/// Writes every row of `project` that the factory's database at
/// `factory` holds in `tables` into the journal's file at `file`, whose
/// tables are there ([`made`]) and which holds none of them yet; reads
/// it back, and holds it to the disk. The factory's database is only
/// read. How many rows of each table are the project's there: what is
/// decided by.
pub fn carry(
    factory: &Path,
    file: &Path,
    project: &str,
    tables: &[Kept],
    placed: &[Placed],
) -> Result<Vec<Counted>> {
    let wrong = |err: rusqlite::Error| {
        db_error(err).context(format!("writing what is kept of `{project}` into {}", file.display()))
    };
    let mut connection = open(factory)?;
    // The factory's own log is folded back first, which nothing that has
    // the database open lets happen: whoever asked holds the folder, and
    // this is what says that nothing still has the database itself.
    let busy: i64 =
        connection.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| row.get(0)).map_err(wrong)?;
    if busy != 0 {
        return Err(Error::conflict(format!(
            "{} is in use: something else has it open. Nothing was changed",
            factory.display()
        )));
    }
    connection.execute("ATTACH DATABASE ?1 AS apart", [text(file)?]).map_err(wrong)?;
    let mut order: Vec<&Kept> = tables.iter().collect();
    order.sort_by_key(|kept| kept.depth);
    let transaction = connection.transaction().map_err(wrong)?;
    for kept in order.iter().filter(|kept| !kept.follows) {
        let (table, part) = (quoted(kept.table), kept.part());
        let there = columns(&transaction, "apart", kept.table).map_err(wrong)?;
        if there.is_empty() || there != columns(&transaction, "main", kept.table).map_err(wrong)? {
            return Err(Error::conflict(format!(
                "`{}` is not kept in {} as this program keeps it, so nothing was moved",
                kept.table,
                factory.display()
            )));
        }
        let held = format!("SELECT count(*) FROM apart.{table} kept WHERE {part}");
        let already: i64 = transaction.query_row(&held, [], |row| row.get(0)).map_err(wrong)?;
        let names = there.iter().map(|column| quoted(column)).collect::<Vec<_>>().join(", ");
        // What is written in each column of a row that comes, called as
        // the column is.
        let comes = |column: &String| match kept.writes.iter().find(|(written, _)| written == column) {
            Some((_, how)) => how.clone(),
            None if kept.says && column == "project" => "?1".to_string(),
            None => format!("kept.{}", quoted(column)),
        };
        let values = there
            .iter()
            .map(|column| format!("{} AS {}", comes(column), quoted(column)))
            .collect::<Vec<_>>()
            .join(", ");
        let ordered = match &kept.order {
            Some(by) => format!("ORDER BY {by}"),
            None if in_order(&transaction, kept.table).map_err(wrong)? => "ORDER BY kept.rowid".to_string(),
            None => String::new(),
        };
        // A row of a table is the project's alone, and one that is there
        // already is a fault. A key of the small durable state may be
        // there, and the line says which of the two then stands.
        let there_already = match (kept.under.is_some(), kept.yields) {
            (false, _) => "",
            (true, true) => "ON CONFLICT DO NOTHING",
            (true, false) => "ON CONFLICT (namespace, key) DO UPDATE SET value = excluded.value",
        };
        let coming = format!("FROM main.{table} kept WHERE {part} AND {} IS ?1", kept.whose);
        let wrote = match (&kept.order, kept.strays && already > 0) {
            // Rows of the line's are there already, in its order, and
            // those that come belong among them: every one is written
            // again, those there as they are, in the one order.
            (Some(by), true) => {
                let as_they_are = there
                    .iter()
                    .map(|column| format!("kept.{column} AS {column}", column = quoted(column)))
                    .collect::<Vec<_>>()
                    .join(", ");
                let set_aside = format!(
                    "CREATE TEMP TABLE there_before AS SELECT * FROM apart.{table}; DELETE FROM apart.{table};"
                );
                transaction.execute_batch(&set_aside).map_err(wrong)?;
                let all = transaction
                    .execute(
                        &format!(
                            "INSERT INTO apart.{table} ({names})
                             SELECT {names} FROM (
                                 SELECT {values}, {by} AS in_order {coming}
                                 UNION ALL
                                 SELECT {as_they_are}, {by} AS in_order FROM temp.there_before kept)
                             ORDER BY in_order"
                        ),
                        [project],
                    )
                    .map_err(wrong)?;
                transaction.execute_batch("DROP TABLE temp.there_before;").map_err(wrong)?;
                all.saturating_sub(already as usize)
            }
            _ => transaction
                .execute(
                    &format!("INSERT INTO apart.{table} ({names}) SELECT {values} {coming} {ordered} {there_already}"),
                    [project],
                )
                .map_err(wrong)?,
        };
        let now: i64 = transaction.query_row(&held, [], |row| row.get(0)).map_err(wrong)?;
        // What is kept under a name is read back key by key, below.
        if kept.under.is_none() && now - already != wrote as i64 {
            return Err(Error::internal(format!(
                "`{}` did not come out as it went in, so nothing was moved",
                kept.table
            )));
        }
        if kept.said && wrote > 0 {
            numbered::keep_holes(&transaction, "apart").map_err(wrong)?;
        }
    }
    // Read back: as many rows of each as the factory's holds for the
    // project, those the database keeps by its own rules among them;
    // and every key of the project's under each name, with what was
    // kept under it unless the line gives way to what was there.
    let held = count(&transaction, project, tables).map_err(wrong)?;
    for (kept, held) in tables.iter().zip(&held) {
        let table = quoted(kept.table);
        if kept.under.is_some() {
            let missing: i64 = transaction
                .query_row(
                    &format!(
                        "SELECT count(*) FROM main.{table} kept WHERE {} AND {} IS ?1 AND NOT EXISTS
                         (SELECT 1 FROM apart.{table} there
                          WHERE there.namespace = kept.namespace AND there.key = kept.key
                            AND (?2 OR there.value = kept.value))",
                        kept.part(),
                        kept.whose
                    ),
                    rusqlite::params![project, kept.yields],
                    |row| row.get(0),
                )
                .map_err(wrong)?;
            if missing != 0 {
                return Err(Error::internal(format!(
                    "{missing} of the {} things kept for the project under `{}` are not where they went, so \
                     nothing was moved",
                    held.moves,
                    kept.called()
                )));
            }
            continue;
        }
        let there: i64 = transaction
            .query_row(&format!("SELECT count(*) FROM apart.{table}"), [], |row| row.get(0))
            .map_err(wrong)?;
        if (there as u64) < held.moves {
            return Err(Error::internal(format!(
                "`{}` holds {there} rows where it went and {} of the project's where it was, so nothing was moved",
                kept.table, held.moves
            )));
        }
    }
    // What was said of the project comes into a journal that kept
    // nothing said: no place there is left to say otherwise.
    if tables.iter().any(|kept| kept.said) {
        unplace(&transaction, "apart", placed, &crate::of_project(project)).map_err(wrong)?;
    }
    transaction.commit().map_err(wrong)?;
    let sound: String =
        connection.query_row("PRAGMA apart.quick_check", [], |row| row.get(0)).map_err(wrong)?;
    if sound != "ok" {
        return Err(Error::internal(format!(
            "{} did not read back sound ({sound}), so nothing was moved",
            file.display()
        )));
    }
    // All of it in the one file, and that file on the disk, before
    // anything is decided by it.
    let _: i64 =
        connection.query_row("PRAGMA apart.wal_checkpoint(TRUNCATE)", [], |row| row.get(0)).map_err(wrong)?;
    connection.execute_batch("DETACH DATABASE apart;").map_err(wrong)?;
    connection.close().map_err(|(_, err)| wrong(err))?;
    std::fs::OpenOptions::new().write(true).open(file)?.sync_all()?;
    #[cfg(unix)]
    if let Some(folder) = file.parent() {
        let _ = std::fs::File::open(folder).and_then(|folder| folder.sync_all());
    }
    Ok(held)
}

/// Decides the move of a project, in one write of the factory's
/// database at `factory`: its row of `journals` as `row` says, with
/// where its rows were moved from (this journal) and where the giver of
/// numbers stood; and the project's rows taken out of `tables`, as many
/// of each as `carried` says were written into its file, what is kept
/// beside them under a name with them. Where a table holds another
/// number of them by now, nothing is written at all.
/// `note` is kept with it under `kept` in the small durable state, added
/// to what is there, for the daemon that next starts to say, with where
/// the giver stood (`at`) and how long the move took since `began`
/// (`took_ms`). Where the giver stood.
pub fn decide(
    factory: &Path,
    row: &Row,
    tables: &[Kept],
    placed: &[Placed],
    carried: &[Counted],
    kept: (&str, &str),
    note: &Value,
    began: std::time::Instant,
) -> Result<u64> {
    let wrong =
        |err: rusqlite::Error| db_error(err).context(format!("moving `{}` to its own journal", row.project));
    let mut connection = open(factory)?;
    let transaction =
        connection.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate).map_err(wrong)?;
    // Everything said before the move is at or below this, and whatever
    // the project's journal says is drawn above it. It is written in
    // the row, with where from, only where what was said of the project
    // is moved: it is what a place in the journal it was moved from
    // comes to in its own ([`crate::joined`]), and a project whose
    // journal kept what was said of it already has nothing said moved.
    let events = tables.iter().any(|kept| kept.said);
    let at = giver::mark(&transaction, "main")
        .and_then(|mark| Ok(mark.max(numbered::last(&transaction, "main")?)))
        .map_err(wrong)?;
    transaction
        .execute(
            "INSERT INTO journals (name, project, file, made, keeps, moved_from, moved_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT (name) DO UPDATE
             SET file = excluded.file, keeps = max(keeps, excluded.keeps),
                 moved_from = CASE WHEN ?8 THEN excluded.moved_from ELSE moved_from END,
                 moved_at = CASE WHEN ?8 THEN excluded.moved_at ELSE moved_at END",
            rusqlite::params![
                row.name,
                row.project,
                row.file,
                row.made,
                row.keeps,
                events.then_some(crate::FACTORY),
                events.then_some(at),
                events
            ],
        )
        .map_err(wrong)?;
    let mut order: Vec<&Kept> = tables.iter().collect();
    order.sort_by_key(|kept| std::cmp::Reverse(kept.depth));
    for table in order.iter().filter(|kept| !kept.follows) {
        let meant =
            format!("main.{} AS kept WHERE {} AND {} IS ?1", quoted(table.table), table.part(), table.whose);
        let took = match table.stays {
            // Left where it is, and only held to being what was written.
            true => transaction
                .query_row(&format!("SELECT count(*) FROM {meant}"), [&row.project], |row| {
                    row.get::<_, i64>(0)
                })
                .map_err(wrong)? as usize,
            false => transaction.execute(&format!("DELETE FROM {meant}"), [&row.project]).map_err(wrong)?,
        };
        let meant = carried.iter().find(|counted| table.is(counted)).map(|counted| counted.moves);
        if meant != Some(took as u64) {
            return Err(Error::conflict(format!(
                "`{}` holds {took} rows of `{}` and {} were written into its journal: the factory was written \
                 to meanwhile, by something that should not have had it open. Nothing was moved",
                table.called(),
                row.project,
                meant.unwrap_or(0)
            )));
        }
    }
    // Events were taken out: the highest number ever given here is
    // written down, and that what is left has holes all through it.
    let said = |kept: &&Kept| kept.said && carried.iter().any(|c| kept.is(c) && c.moves > 0);
    if tables.iter().any(|kept| said(&kept)) {
        numbered::keep_said(&transaction, at).map_err(wrong)?;
        numbered::keep_holes(&transaction, "main").map_err(wrong)?;
    }
    if events {
        unplace(&transaction, "main", placed, &row.name).map_err(wrong)?;
    }
    // What the database keeps from those rows has gone with them, and
    // what stays is the project's by nothing that is left.
    for left in count(&transaction, &row.project, tables).map_err(wrong)? {
        if left.moves != 0 {
            return Err(Error::internal(format!(
                "`{}` still holds {} rows of `{}` with the rest taken out, so nothing was moved",
                left.table, left.moves, row.project
            )));
        }
    }
    let said: Option<String> = crate::optional(transaction.query_row(
        "SELECT value FROM kv WHERE namespace = ?1 AND key = ?2",
        [kept.0, kept.1],
        |row| row.get(0),
    ))
    .map_err(wrong)?;
    let mut said: Vec<Value> = said.and_then(|said| serde_json::from_str(&said).ok()).unwrap_or_default();
    let mut note = note.clone();
    note["at"] = serde_json::json!(at);
    note["took_ms"] = serde_json::json!(began.elapsed().as_millis() as u64);
    said.push(note);
    crate::copy::note(&transaction, kept.0, kept.1, &Value::Array(said)).map_err(wrong)?;
    transaction.commit().map_err(wrong)?;
    // The rows taken out are folded back into the one file, so that the
    // daemon that next starts does not open a log as long as they were.
    let _: rusqlite::Result<i64> =
        connection.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| row.get(0));
    connection.close().map_err(|(_, err)| wrong(err))?;
    Ok(at as u64)
}

/// The files in `folder` that no row of `journals` in the factory's
/// database at `factory` names: what a move that was cut off left, and
/// what a project had before it was moved. None where the database
/// cannot be read: nothing is taken for nobody's on a guess.
pub fn unnamed(factory: &Path, folder: &Path) -> Vec<PathBuf> {
    if !factory.is_file() || !folder.is_dir() {
        return Vec::new();
    }
    // Only read, on a connection that may do nothing else: asked by a
    // daemon as it starts, it brings no log back and writes nothing.
    let flags = rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX;
    let Ok(connection) = Connection::open_with_flags(factory, flags) else { return Vec::new() };
    let _ = connection.busy_timeout(std::time::Duration::from_secs(5));
    let Ok(listed) = set::listed(&connection) else { return Vec::new() };
    let named: Vec<String> = listed.into_iter().map(|listed| listed.file).collect();
    let Ok(entries) = std::fs::read_dir(folder) else { return Vec::new() };
    let mut unnamed: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|file| file.is_file() && file.extension().is_some_and(|ending| ending == "db"))
        .filter(|file| {
            file.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| !named.iter().any(|n| n == name))
        })
        .collect();
    unnamed.sort();
    unnamed
}

/// Takes away the files of `folder` that no row names ([`unnamed`]),
/// each with its log. Whoever asks holds the factory's folder, and has
/// not opened its database: the daemon as it starts, or the move asked
/// again. How many files, and how big they were together.
pub fn clear(factory: &Path, folder: &Path) -> Option<(usize, u64)> {
    let mut cleared = None;
    for file in unnamed(factory, folder) {
        let bytes = file.metadata().map(|file| file.len()).unwrap_or(0);
        if std::fs::remove_file(&file).is_ok() {
            for ending in ["-wal", "-shm"] {
                let _ = std::fs::remove_file(with(&file, ending));
            }
            let (files, all) = cleared.unwrap_or((0, 0));
            cleared = Some((files + 1, all + bytes));
        }
    }
    cleared
}

/// Takes away the file a project's journal was kept in before it was
/// moved, which its row no longer names.
pub fn put_away(file: &Path) {
    for ending in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(with(file, ending));
    }
}

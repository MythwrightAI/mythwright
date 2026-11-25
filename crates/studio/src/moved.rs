//! What of a project is kept here, for whoever moves a project of a
//! factory from before to a journal of its own (`journal::moved`).
//!
//! The move knows no table. It is handed each table a project's own
//! journal keeps ([`crate::apart::KEPT_APART`]) with how a row of it
//! says whose it is ([`crate::apart::TABLES`], which is what the rows
//! are held to) and how much a journal has to keep for the table to be
//! kept there: a project whose journal keeps less has its rows of that
//! table in the factory's file, and they go when it is moved.
//!
//! A row goes as it is, every column: a record under its id, a table
//! with where its store is reached and what declared it, a line of
//! history under the number of the event it is of, which the event
//! keeps wherever it is kept. Nothing of a record is under way between
//! two writes, so nothing kept here is in the way of a move beyond what
//! the runner says is.

use journal::moved::Kept;

use crate::apart::{KEPT_APART, TABLES};

/// Every table of a project's that is kept here, as the move is handed
/// them, each with the level a journal keeps it from (`journal::keeps`).
/// A line of history is told apart through its record and the tables
/// there are, so it is written after them and taken out before them:
/// the list is in the order its tables are taken out in, and how deep
/// each is said to be tells the move so.
pub fn tables() -> Vec<(i64, Kept)> {
    let last = KEPT_APART.len().saturating_sub(1);
    let lines = KEPT_APART.iter().enumerate().filter_map(|(at, (table, level))| {
        let told = journal::apart::told(TABLES, table)?;
        Some((*level, Kept::rows(table, told.reads(), (last - at) as u8)))
    });
    lines.collect()
}

/// The tables those rows are kept in where they are moved from, the
/// factory's own file, by the names they are migrated under there, each
/// with the version of it this build knows: a factory whose are older
/// was last run by an earlier program, and is brought up by a daemon as
/// it starts, never by a move.
pub fn knows() -> Vec<(&'static str, usize)> {
    let moved = ["collections", "projects"];
    crate::copy::KNOWS.iter().filter(|(namespace, _)| moved.contains(namespace)).copied().collect()
}

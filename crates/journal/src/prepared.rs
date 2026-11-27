//! Statements asked again and again, prepared once for their connection.
//!
//! Each database has one thread and one connection, and the same few
//! hundred statements are asked of it all day: a step's record written,
//! an instance read, a run made. Prepared afresh each time, as
//! `Connection::execute` and `Connection::query_row` do, a statement was
//! read and planned again for every row: three tenths of what the
//! journal's thread did while cards were worked (2026-11-26,
//! `docs/build-times.md`). Asked through this, each is prepared the
//! first time and kept with the connection ([`CACHED`] at most), and
//! what it answers is the same.
//!
//! Kept, a statement is planned once for every value it is given. The
//! database otherwise plans again, as the statement is next asked, every
//! statement whose plan looked at a value it was given (a key compared
//! with a column an index holds, where it is built to weigh values), and
//! that was nearly every statement here: kept or not, each was planned
//! anew every time it was asked. So it is told to plan by the statement
//! alone ([`keep_prepared`]). Nothing here is planned by a value it is
//! given: no table's values are weighed (nothing asks the database to),
//! and where a partial index is to be read the statement says what the
//! index's own condition says, or what implies it whatever the value
//! (`owes = ?1` implies `owes IS NOT NULL`).

use rusqlite::config::DbConfig;
use rusqlite::{Connection, Params, Row};

/// How many prepared statements a connection keeps: more than the
/// statements a factory at work asks of one database, so that none of
/// those is let go to make room for another.
pub const CACHED: usize = 512;

/// `execute` and `query_row` with the statement kept once prepared.
pub trait Prepared {
    /// As `Connection::execute`.
    fn execute_kept<P: Params>(&self, sql: &str, params: P) -> rusqlite::Result<usize>;

    /// As `Connection::query_row`.
    fn query_row_kept<T, P, F>(&self, sql: &str, params: P, read: F) -> rusqlite::Result<T>
    where
        P: Params,
        F: FnOnce(&Row<'_>) -> rusqlite::Result<T>;
}

impl Prepared for Connection {
    fn execute_kept<P: Params>(&self, sql: &str, params: P) -> rusqlite::Result<usize> {
        self.prepare_cached(sql)?.execute(params)
    }

    fn query_row_kept<T, P, F>(&self, sql: &str, params: P, read: F) -> rusqlite::Result<T>
    where
        P: Params,
        F: FnOnce(&Row<'_>) -> rusqlite::Result<T>,
    {
        self.prepare_cached(sql)?.query_row(params, read)
    }
}

/// Has `connection` keep what it prepares, and plan each statement once.
pub(crate) fn keep_prepared(connection: &Connection) -> rusqlite::Result<()> {
    connection.set_prepared_statement_cache_capacity(CACHED);
    connection.set_db_config(DbConfig::SQLITE_DBCONFIG_ENABLE_QPSG, true)?;
    Ok(())
}

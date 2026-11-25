//! How an event comes by its number, and the table events are kept in.
//!
//! The first programs had the database number the events (`AUTOINCREMENT`),
//! and for that the database keeps the last number it gave in a table of
//! its own, which it writes again for every row: a page written for each
//! thing said, beside the event's. The journal gives the number itself
//! now, on the database's thread where events are written one at a time
//! ([`crate::Journal::emit`]): the next of a run of numbers it has drawn
//! from the factory's one giver ([`crate::giver`]), and the first of
//! another run where that one is used up. So an event's number is above
//! every one the journal gave before, and not always by one.
//!
//! What the database's table was for is still held to: a number is never
//! given twice, though the last events be taken out and the journal opened
//! again. A place somebody kept in the journal must never be ahead of what
//! is said next. So the highest number the journal ever gave is written
//! down with whatever takes events out ([`keep_said`]), which is the only
//! time the events themselves might stop saying it, and is read when the
//! journal is opened ([`last`]): the giver begins above it, whatever its
//! own mark says, and is told it where it is kept elsewhere.
//!
//! A journal made now has a table the database does not number. One from
//! before goes on with the table it has: the journal gives its events
//! their numbers all the same, the database goes on writing its page for
//! each, and the numbers it kept are read with the rest, so none is given
//! again. Making its table anew means writing every event out again, and
//! that is done when the journal's owner asks (`journal::rewrite`), never
//! by opening it.

use std::sync::Arc;

use rusqlite::Connection;

use crate::giver::Run;

/// Where the journal keeps the highest number it ever gave an event, of
/// its small durable state.
pub(crate) const SAID: (&str, &str) = ("journal", "said");

/// Where the journal keeps which numbers it passed over, of its small
/// durable state: the newest of the gaps in its events' numbers, each
/// the first number after it and how many were passed over before that.
pub(crate) const GAPS: (&str, &str) = ("journal", "gaps");

/// How many gaps are kept, the newest. There is one for each time the
/// journal went on above numbers it never gave: at a start, and where
/// another journal drew in between. Older ones are let go, and what is
/// counted across them ([`floor`]) then takes their numbers for given.
const GAPS_KEPT: usize = 256;

/// The word that has the database number a table's rows.
const NUMBERED: &str = "AUTOINCREMENT";

/// How the events table is made, as the database keeps it.
fn made(connection: &Connection, schema: &str) -> rusqlite::Result<Option<String>> {
    let found = connection.query_row(
        &format!("SELECT sql FROM {schema}.sqlite_master WHERE type = 'table' AND name = 'events'"),
        [],
        |row| row.get::<_, String>(0),
    );
    match found {
        Ok(sql) => Ok(Some(sql)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(other) => Err(other),
    }
}

/// Whether the database numbers this journal's events, as the first
/// programs had it: `schema` is `main`, or the name a database was
/// attached under.
pub fn by_the_database(connection: &Connection, schema: &str) -> rusqlite::Result<bool> {
    Ok(made(connection, schema)?.is_some_and(|sql| sql.to_ascii_uppercase().contains(NUMBERED)))
}

/// How the events table of `schema` is made, without the database
/// numbering it: every column as it is, whichever program made them.
/// Nothing, where there is no such table.
pub(crate) fn unnumbered(connection: &Connection, schema: &str) -> rusqlite::Result<Option<String>> {
    Ok(made(connection, schema)?.map(|sql| {
        let at = sql.to_ascii_uppercase().find(NUMBERED);
        match at {
            Some(at) => format!("{}{}", sql[..at].trim_end(), &sql[at + NUMBERED.len()..]),
            None => sql,
        }
    }))
}

/// The highest number an event was ever given: the highest there is, the
/// highest written down when events were taken out, and, where the
/// database numbered them, the highest it remembers.
pub(crate) fn last(connection: &Connection, schema: &str) -> rusqlite::Result<i64> {
    let there: i64 =
        connection
            .query_row(&format!("SELECT coalesce(max(seq), 0) FROM {schema}.events"), [], |row| row.get(0))?;
    let kept: i64 = connection.query_row(
        &format!(
            "SELECT coalesce((SELECT CAST(value AS INTEGER) FROM {schema}.kv WHERE namespace = ?1 AND key = ?2), 0)"
        ),
        [SAID.0, SAID.1],
        |row| row.get(0),
    )?;
    // Where the database numbered them it keeps the last number it gave in
    // a table of its own, which is there from when the events' table was
    // made. If that cannot be read the journal is not opened: taken for
    // nothing, a number that was given and since taken out would be given
    // again, and a place somebody kept would be ahead of what is said.
    let remembered: i64 = match by_the_database(connection, schema)? {
        true => connection
            .query_row(
                &format!("SELECT coalesce(max(seq), 0) FROM {schema}.sqlite_sequence WHERE name = 'events'"),
                [],
                |row| row.get(0),
            )
            .map_err(|err| {
                let code = match &err {
                    rusqlite::Error::SqliteFailure(failure, _) => failure.extended_code,
                    _ => rusqlite::ffi::SQLITE_ERROR,
                };
                let said = format!(
                    "the last number the database gave an event could not be read (its `sqlite_sequence`), \
                     and without it a number might be given twice, so the journal is not opened: {err}"
                );
                rusqlite::Error::SqliteFailure(rusqlite::ffi::Error::new(code), Some(said))
            })?,
        false => 0,
    };
    Ok(there.max(kept).max(remembered))
}

/// Writes down the highest number an event was ever given, with whatever
/// takes events out: one opened afterwards goes on from above it.
pub(crate) fn keep_said(connection: &Connection, last: i64) -> rusqlite::Result<()> {
    connection
        .execute(
            "INSERT INTO kv (namespace, key, value) VALUES (?1, ?2, ?3)
             ON CONFLICT (namespace, key) DO UPDATE SET value = excluded.value",
            rusqlite::params![SAID.0, SAID.1, last.to_string()],
        )
        .map(|_| ())
}

/// Under what a journal keeps that its events have holes all through
/// them that the list of gaps does not say: a project's events were
/// moved out of it, or into it, each keeping its number
/// ([`crate::moved`]). The newest so many of such a journal are counted
/// from its rows ([`holed`]).
pub(crate) const HOLES: (&str, &str) = ("journal", "holes");

/// Writes down, in the database called `schema` on this connection,
/// that its events have holes the gaps do not say.
pub(crate) fn keep_holes(connection: &Connection, schema: &str) -> rusqlite::Result<()> {
    connection
        .execute(
            &format!(
                "INSERT INTO {schema}.kv (namespace, key, value) VALUES (?1, ?2, 'true')
                 ON CONFLICT (namespace, key) DO UPDATE SET value = excluded.value"
            ),
            [HOLES.0, HOLES.1],
        )
        .map(|_| ())
}

/// For a journal whose events have holes the gaps do not say: the
/// number at or below which every event is older than the newest `keep`,
/// read from the rows themselves. Nothing, for a journal with no such
/// holes, which is counted over its gaps ([`floor`]).
pub(crate) fn holed(connection: &Connection, keep: u64) -> rusqlite::Result<Option<u64>> {
    let marked: bool = connection.query_row(
        "SELECT EXISTS (SELECT 1 FROM kv WHERE namespace = ?1 AND key = ?2)",
        [HOLES.0, HOLES.1],
        |row| row.get(0),
    )?;
    if !marked {
        return Ok(None);
    }
    let below: Option<i64> = crate::optional(connection.query_row(
        "SELECT seq FROM events ORDER BY seq DESC LIMIT 1 OFFSET ?1",
        [keep as i64],
        |row| row.get(0),
    ))?;
    Ok(Some(below.unwrap_or(0) as u64))
}

/// The gaps the journal has written down, oldest first.
pub(crate) fn gaps(connection: &Connection) -> rusqlite::Result<Vec<(u64, u64)>> {
    let kept: Option<String> = crate::optional(connection.query_row(
        "SELECT value FROM kv WHERE namespace = ?1 AND key = ?2",
        [GAPS.0, GAPS.1],
        |row| row.get(0),
    ))?;
    Ok(kept.and_then(|kept| serde_json::from_str(&kept).ok()).unwrap_or_default())
}

/// Writes down that the journal went on at `first` having passed over the
/// `numbers` before it, with what is written under `first`.
fn keep_gap(connection: &Connection, first: u64, numbers: u64) -> rusqlite::Result<()> {
    let mut kept = gaps(connection)?;
    kept.push((first, numbers));
    let newest = kept.split_off(kept.len().saturating_sub(GAPS_KEPT));
    connection
        .execute(
            "INSERT INTO kv (namespace, key, value) VALUES (?1, ?2, ?3)
             ON CONFLICT (namespace, key) DO UPDATE SET value = excluded.value",
            rusqlite::params![GAPS.0, GAPS.1, serde_json::json!(newest).to_string()],
        )
        .map(|_| ())
}

/// The number below which lie all but the newest `keep` events of a
/// journal whose last is `last`: `keep` below it where every number
/// between was given, and further below by each of the `gaps` that lie
/// between. Nought where the journal has not said as many.
pub(crate) fn floor(last: u64, keep: u64, gaps: &[(u64, u64)]) -> u64 {
    let (mut top, mut need) = (last, keep);
    for (first, numbers) in gaps.iter().rev().filter(|(first, _)| *first <= last) {
        // From the first after the gap to `top`, every number was given.
        let given = (top + 1).saturating_sub(*first);
        if given >= need {
            break;
        }
        need -= given;
        top = first.saturating_sub(numbers + 1);
    }
    top.saturating_sub(need)
}

/// What a journal that is being opened holds and goes on from: the number
/// of the last event that is in it, and the last number ever given, which
/// is higher where the last events were taken out. The first is what the
/// journal held when it was opened ([`crate::Journal::held_when_opened`]);
/// the second is what the next event is numbered above. One in which
/// nothing was ever said is first given the table a journal is made with
/// now, which costs nothing: there is nothing in it to write out.
pub(crate) fn settle(connection: &mut Connection) -> rusqlite::Result<(i64, i64)> {
    if by_the_database(connection, "main")? && last(connection, "main")? == 0 {
        let anew = unnumbered(connection, "main")?.unwrap_or_default();
        let transaction = crate::gather::whole(connection)?;
        // Its indexes go with it, and are made again as they were.
        let indexes: Vec<String> = {
            let mut statement = transaction.prepare(
                "SELECT sql FROM sqlite_master WHERE type = 'index' AND tbl_name = 'events' AND sql IS NOT NULL",
            )?;
            statement.query_map([], |row| row.get(0))?.collect::<rusqlite::Result<_>>()?
        };
        transaction.execute_batch("DROP TABLE events;")?;
        transaction.execute_batch(&anew)?;
        for index in indexes {
            transaction.execute_batch(&index)?;
        }
        transaction.commit()?;
    }
    let held: i64 = connection.query_row("SELECT coalesce(max(seq), 0) FROM events", [], |row| row.get(0))?;
    Ok((held, last(connection, "main")?))
}

impl crate::Inner {
    /// The run of numbers in hand, for a piece of the database's work to
    /// number its events from: a copy, put back once the events are
    /// written ([`crate::Inner::put_back`]). Work that fails takes back
    /// what it wrote, a mark it raised among it, and puts nothing back:
    /// the run in hand is then as it was, and no number was given away.
    ///
    /// A run whose numbers were given away meanwhile (a journal joined
    /// that holds numbers as high, [`crate::Giver::above`]) is in hand
    /// no longer: the copy is of no run, and what is said next draws
    /// another, above the highest number this journal gave.
    pub(crate) fn run(&self) -> Run {
        let run = *self.numbers.run.lock().expect("journal lock");
        match self.numbers.giver.is_given_away(&run) {
            true => Run::default(),
            false => run,
        }
    }

    /// Has `run` be the run in hand: what is left of the numbers a piece
    /// of work numbered its events from, which it found as `before`, now
    /// that they are written. Where the commit that was to keep them is
    /// not made, what was in hand before the commit is in hand again: the
    /// numbers it gave were nobody's, and are the next ones'; and the
    /// runs it `drew` are the giver's again ([`crate::Inner::undraw`]),
    /// which is said here and nowhere else, so that a run is given back
    /// once: by the work that failed, or by the commit that was not made.
    pub(crate) fn put_back(self: &Arc<Self>, before: Run, run: Run, drew: Vec<Run>) {
        if run == before {
            return;
        }
        *self.numbers.run.lock().expect("journal lock") = run;
        let inner = self.clone();
        crate::gather::then(move |kept| {
            if kept {
                return;
            }
            // Each piece of work in the commit says so, in the order they
            // were done: the first of them found what was there before
            // any, and what the later ones found is not gone back to.
            let mut hand = inner.numbers.run.lock().expect("journal lock");
            if before < *hand {
                *hand = before;
            }
            inner.undraw(&drew);
        });
    }

    /// The number of the next event: the next of `run`, or where that is
    /// used up the first of another, drawn here and added to `drew`. On
    /// the database's thread, in the transaction the event is written in.
    /// `though_shut` for the one thing said though the journal has shut.
    pub(crate) fn number(
        self: &Arc<Self>,
        connection: &Connection,
        run: &mut Run,
        drew: &mut Vec<Run>,
        though_shut: bool,
    ) -> rusqlite::Result<u64> {
        loop {
            if let Some(number) = run.take() {
                // A journal whose giver is kept elsewhere asks for its
                // next run when half of this one is given, and waits for
                // nothing ([`crate::giver`]).
                if !self.numbers.here && run.left() <= crate::giver::RUN / 2 {
                    self.draw_ahead();
                }
                return Ok(number);
            }
            // The highest it gave before: the last of the run that is
            // used up, or with no run yet what it was opened with.
            let last = run.last().unwrap_or_else(|| self.given.load(std::sync::atomic::Ordering::SeqCst));
            *run = self.draw(connection, though_shut)?;
            drew.push(*run);
            // Where the run does not begin one above that, the numbers
            // between are never an event's: written down with the first
            // event after them, for whatever counts events by number.
            let first = run.first();
            if first > last + 1 {
                keep_gap(connection, first, first - last - 1)?;
            }
        }
    }

    /// Gives the giver back what a piece of work `drew` and then wrote no
    /// event with: it failed, and took back what it had written, or the
    /// commit that was to keep it was not made. So a write that keeps
    /// nothing gives no number away, though it was the one to draw them.
    /// Asked once for a run: a run given back twice could be given back
    /// from under whoever was handed it in between.
    pub(crate) fn undraw(&self, drew: &[Run]) {
        for run in drew.iter().rev() {
            self.numbers.giver.take_back(*run);
        }
    }

    /// Asks for the next run ahead of need, where none is asked for.
    fn draw_ahead(&self) {
        let mut ahead = self.numbers.ahead.lock().expect("journal lock");
        if ahead.is_none() {
            *ahead = self.numbers.giver.draw_ahead();
        }
    }

    /// Draws a run from the giver, which has kept how far it has given
    /// before any number of the run is an event's.
    fn draw(self: &Arc<Self>, connection: &Connection, last: bool) -> rusqlite::Result<Run> {
        let giver = self.numbers.giver.clone();
        if !self.numbers.here {
            // Its mark is in another database. The run asked for ahead
            // is the one, and is there unless half a run was given
            // sooner than its mark could be kept; with none asked for
            // (the first after it was opened) that database is asked and
            // waited for. How long is what this journal's thread stood
            // still.
            let began = std::time::Instant::now();
            let asked = self.numbers.ahead.lock().expect("journal lock").take();
            let ahead = match asked {
                // What is said though the journal has shut waits for
                // nothing that may never be answered: the database the
                // mark is in may be doing nothing more.
                Some(asked) if last => giver.ahead_if_kept(asked).unwrap_or_else(|asked| {
                    *self.numbers.ahead.lock().expect("journal lock") = Some(asked);
                    None
                }),
                Some(asked) => giver.ahead(asked),
                None => None,
            };
            let drawn = match ahead.filter(|run| !giver.is_given_away(run)) {
                Some(run) => Ok(run),
                None => giver.draw_apart(last),
            };
            let took = began.elapsed().as_secs_f64() * 1000.0;
            crate::Journal { inner: self.clone() }.measure_quietly("db.draw", &[], took, drawn.is_ok());
            return drawn;
        }
        // The mark is raised in the transaction that is open, and is kept
        // with what the run is first used for or not at all. Where the
        // commit is not made nothing on the disk says the run was drawn,
        // and nothing of it is in hand ([`crate::Inner::put_back`]): the
        // giver has it back, unless somebody has drawn since.
        giver.draw_here(connection)
    }
}

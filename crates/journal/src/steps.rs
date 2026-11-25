//! The steps a database's tables are made by.
//!
//! Each crate says how its tables are made as a list of steps
//! ([`Db::migrate`]), and a database keeps how many of a list's steps it
//! has run: it runs those after, and none before. So a list only ever
//! grows at its end. A step written in before the end is never run by a
//! database that was past its place, which runs the step that was pushed
//! along a second time instead; and a step that was changed is run again
//! by nobody. A factory that had run every step of its tables was handed
//! a list with one written in before the last, ran the last again, and
//! would not start.
//!
//! What holds a list to that is a file kept with the code
//! (`contract/steps.txt`, and the test of the API suite that reads it):
//! what each step of each list comes to ([`mark`]), which a step that
//! was there has to come to still. And a database keeps the same thing
//! of the steps it ran, beside how many: one handed a list whose first
//! steps are not the ones it ran is refused before anything is run,
//! in a sentence, where it would otherwise have been left as no program
//! makes one. A database from before this keeps only the count: it is
//! taken to have run the steps this program has, and keeps what they
//! were from then on.
//!
//! A step that has to be changed all the same is written down with why
//! (`contract/steps-changed.txt`, by that test and no other way), and
//! that file is built in here: a database that ran the step as it was
//! is one that ran the step, and is not refused.

use kernel::{Error, Result};
use rusqlite::Connection;

use crate::db::{Db, db_error};

/// The steps that were changed and meant to be, each a line: the day,
/// the list, which step, what it came to, what it comes to, and why.
const CHANGED: &str = include_str!("../../../contract/steps-changed.txt");

/// What a step comes to: the same for the same text and for no other,
/// comments and layout included, since the text is what a database is
/// handed. It tells one step from another and keeps nothing secret.
pub fn mark(step: &str) -> String {
    // FNV-1a: small, and the same on every system and in every build.
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in step.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}")
}

/// Whether the step that came to `was`, the `step`th of `list`, is the
/// one that comes to `is`: it is that step, or was changed to it, once
/// or more, as `changed` says.
fn became(list: &str, step: usize, was: &str, is: &str, changed: &str) -> bool {
    let changes: Vec<(&str, &str)> = changed
        .lines()
        .filter(|line| !line.starts_with('#'))
        .filter_map(|line| {
            let mut said = line.split('\t').skip(1);
            let (of, at, from, to) = (said.next()?, said.next()?, said.next()?, said.next()?);
            (of == list && at.parse() == Ok(step)).then_some((from, to))
        })
        .collect();
    // Everything the step became, by one change or by several: a step
    // changed back and then to something else was each of them.
    let mut became = vec![was];
    while let Some((_, to)) = changes.iter().find(|(from, to)| became.contains(from) && !became.contains(to))
    {
        became.push(to);
    }
    became.contains(&is)
}

/// Which of the steps a database ran is not the step a list has in its
/// place: the first such, counted from one, and nothing where each is.
/// `ran` is what each step the database ran came to, in their order, and
/// `has` what each step of `list` comes to; `changed` is the steps that
/// were changed and meant to be, as `contract/steps-changed.txt` says
/// them. A database that ran more steps than the list has is not looked
/// at here: it is refused for that before.
pub fn parted(list: &str, ran: &[&str], has: &[String], changed: &str) -> Option<usize> {
    (1..=ran.len().min(has.len())).find(|step| {
        let (was, is) = (ran[step - 1], has[step - 1].as_str());
        // What was changed is read only where a step differs, which in
        // a database that is run none does.
        was != is && !became(list, *step, was, is, changed)
    })
}

/// Makes where a database keeps what it has run of each list, in a
/// database that is new: how many steps, and what each was.
///
/// A database from before keeps the count alone, and is given where the
/// rest is kept the first time this program opens it: that is one line
/// of what the database says of its own tables, written once, and no
/// row of anything. A program from before goes on reading and writing
/// the count as it did.
pub(crate) fn kept_in(connection: &Connection) -> rusqlite::Result<()> {
    connection.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_versions (
             namespace TEXT PRIMARY KEY,
             version INTEGER NOT NULL,
             steps TEXT
         );",
    )?;
    let keeps = |connection: &Connection| connection.prepare("SELECT steps FROM schema_versions").is_ok();
    if !keeps(connection) {
        let given = connection.execute_batch("ALTER TABLE schema_versions ADD COLUMN steps TEXT");
        // Refused where another that opened the file in the same moment
        // gave it first: then it is there.
        if let Err(refused) = given
            && !keeps(connection)
        {
            return Err(refused);
        }
    }
    Ok(())
}

/// Why a database is not handed a list its first steps are not of.
fn refused(namespace: &str, within: &str, ran: usize, step: usize) -> Error {
    Error::conflict(format!(
        "the `{namespace}` tables in {within} were made by other steps than this program has: of the {ran} \
         it has run, step {step} is not the one this program has in its place, so a step was changed, moved, \
         or written in before the last. A database runs only the steps after those it has run, and this \
         program's would leave it as no program makes one. Nothing was changed. Run the program this \
         database was last run by, or one whose steps for these tables differ from that one's only by what \
         is added at the end"
    ))
}

impl Db {
    /// Brings `namespace`'s tables up to date. `steps[n]` takes the schema
    /// from version `n` to `n + 1`; steps are only ever appended.
    ///
    /// The database keeps how many it has run and what each was
    /// ([`mark`]). Handed a list whose first steps are not those, it is
    /// refused before anything is run. Where it has run them all and
    /// kept what they were, nothing is written.
    pub async fn migrate(&self, namespace: &'static str, steps: &'static [&'static str]) -> Result<()> {
        let within = match self.file() {
            Some(file) => file.display().to_string(),
            None => "a database in memory".to_string(),
        };
        self.try_call(move |connection| {
            let transaction = crate::gather::whole(connection).map_err(db_error)?;
            let (current, kept): (i64, Option<String>) = transaction
                .query_row(
                    "SELECT version, steps FROM schema_versions WHERE namespace = ?1",
                    [namespace],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .or_else(|err| match err {
                    rusqlite::Error::QueryReturnedNoRows => Ok((0, None)),
                    other => Err(other),
                })
                .map_err(db_error)?;
            if current as usize > steps.len() {
                return Err(Error::conflict(format!(
                    "the `{namespace}` tables are at version {current}, newer than this build understands ({}); \
                     refusing to run against them",
                    steps.len()
                )));
            }
            let current = current as usize;
            let has: Vec<String> = steps.iter().map(|step| mark(step)).collect();
            // What it kept of the steps it ran: nothing, for a database
            // from before this was kept, and of no more than it says it
            // ran, where its count was put back by hand.
            let ran: Vec<&str> = kept.as_deref().unwrap_or_default().split(' ').filter(|one| !one.is_empty()).collect();
            let ran = &ran[..ran.len().min(current)];
            if let Some(step) = parted(namespace, ran, &has, CHANGED) {
                return Err(refused(namespace, &within, current, step));
            }
            for step in &steps[current..] {
                transaction.execute_batch(step).map_err(|err| db_error(err).context(namespace))?;
            }
            let now = has.join(" ");
            if current != steps.len() || kept.as_deref() != Some(now.as_str()) {
                transaction
                    .execute(
                        "INSERT INTO schema_versions (namespace, version, steps) VALUES (?1, ?2, ?3)
                         ON CONFLICT (namespace) DO UPDATE SET version = excluded.version, steps = excluded.steps",
                        rusqlite::params![namespace, steps.len() as i64, now],
                    )
                    .map_err(db_error)?;
            }
            transaction.commit().map_err(db_error)
        })
        .await
    }
}

//! Letting logs go: how long the detail of a run's steps is kept, and
//! where their output is written. Two settings of the factory, said by
//! its owner, each kept in the factory's journal.
//!
//! - **How long** ([`Keeping`]). Unsaid, everything is kept for ever, as
//!   it always was. Said as a number of days, what is older than that of
//!   two things is taken away: the files a run's steps printed into
//!   ([`crate::Logs::let_go_before`]), and what the runner said of its
//!   own steps as they went ([`Journal::let_go_of`]; which topics those
//!   are is the runner's to say, since they are its own). What a run came
//!   to is never taken: its row, how it ended and what is said of that,
//!   and nothing else of the journal either. Whoever does the taking
//!   keeps where it took up to ([`Went`]), so that a run's page can say
//!   that its steps' detail was let go, and after how long.
//! - **Where** ([`Writing`]). Unsaid, a run's steps write in the data
//!   folder, as they always did. Said as a folder, they write there from
//!   then on, under `runs/` as in the data folder: one the owner names is
//!   made where it is not there, and refused in a sentence where nothing
//!   can be written in it ([`writable`]). The folders said before are
//!   kept, so that a file written in one is still found there
//!   ([`crate::logs`]), and its days are counted there as anywhere. The
//!   databases stay in the data folder, whatever is said here.
//!
//! How long goes into a copy of the factory: it is how the factory is
//! kept. Where does not: a copy is another factory, and two factories
//! never write in one folder; and a copy holds no logs of runs, so the
//! folders written in before name nothing of the copy's.

use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;

use kernel::{Error, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{Journal, Logs};

/// Where how long logs are kept is kept, of the factory's small durable
/// state: it goes into a copy.
pub const KEEPS: &str = "logs.kept";
/// Where the folder a run's steps write in is kept: it does not.
pub const WRITES: &str = "logs.folder";
const KEY: &str = "factory";

/// The most days that may be said: a hundred years.
pub const MOST_DAYS: u32 = 36_500;

/// How many events are taken out in one piece of the database's work, so
/// that a journal that has kept a great deal does not hold the writer for
/// the whole of it.
const AT_A_TIME: usize = 4096;

/// How long the detail of a run's steps is kept.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Keeping {
    /// For how many days; for ever where nothing is said.
    #[serde(default)]
    pub days: Option<u32>,
    /// Who last said, by name, and when.
    #[serde(default)]
    pub by: Option<String>,
    #[serde(default)]
    pub at: Option<i64>,
    /// How far it was last let go of.
    #[serde(default)]
    pub went: Option<Went>,
}

/// How far the detail of runs' steps was let go of: what was last before
/// `before` (milliseconds since 1970), as it was kept for `days`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Went {
    pub before: i64,
    pub days: u32,
}

impl Keeping {
    /// What is said of a run that ended at `ended` whose steps' detail
    /// was let go: nothing for one that ended since, or under way.
    pub fn of_run(&self, ended: Option<i64>) -> Option<String> {
        said_of_run(self.went, ended)
    }
}

/// What is said of a run that ended at `ended`, where what runs' steps
/// printed went as far as `went`: nothing for one that ended since, or is
/// under way, or where nothing went.
pub fn said_of_run(went: Option<Went>, ended: Option<i64>) -> Option<String> {
    let went = went?;
    ended.filter(|ended| *ended < went.before)?;
    let days = match went.days {
        1 => "a day".to_string(),
        many => format!("{many} days"),
    };
    Some(format!(
        "what its steps printed, and what was said of each as it went, was let go after {days}, as the \
         factory keeps it; what it came to is kept"
    ))
}

/// Where a run's steps write.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Writing {
    /// The folder, whole; the data folder where nothing is said.
    #[serde(default)]
    pub folder: Option<String>,
    /// The folders said before, the latest first, where what was written
    /// then is still read.
    #[serde(default)]
    pub before: Vec<String>,
    #[serde(default)]
    pub by: Option<String>,
    #[serde(default)]
    pub at: Option<i64>,
}

/// `folder` made whole, where a run's steps can write in it: made where
/// it is not there, and refused, in a sentence, where it is not said from
/// the top of the disk or nothing can be written in it.
pub fn writable(folder: &str) -> Result<PathBuf> {
    let asked = Path::new(folder.trim());
    if !asked.is_absolute() {
        return Err(Error::invalid(format!(
            "a folder for logs is said whole, from the top of the disk, and `{folder}` is not; say it as `/` \
             and the folders down to it"
        )));
    }
    let runs = asked.join(crate::logs::RUNS);
    let refused = |what: &str, err: std::io::Error| {
        Error::invalid(format!(
            "logs cannot be written in `{}`: {what} ({err}). Name a folder this program's user may write \
             in, or say `usual` for the data folder",
            asked.display()
        ))
    };
    std::fs::create_dir_all(&runs).map_err(|err| refused("it could not be made", err))?;
    let probe = runs.join(".written");
    std::fs::write(&probe, b"").map_err(|err| refused("nothing could be written in it", err))?;
    let _ = std::fs::remove_file(&probe);
    kernel::whole::whole(asked).map_err(|err| refused("it could not be read back", err))
}

impl Journal {
    /// How long the detail of a run's steps is kept, as was said.
    pub async fn logs_kept(&self) -> Keeping {
        let kept = self.kv_get(KEEPS, KEY).await.ok().flatten();
        kept.and_then(|kept| serde_json::from_value(kept).ok()).unwrap_or_default()
    }

    /// Says how long: for `days`, or for ever with nothing. Gives back
    /// what was said before and what is said now. Nought days is refused:
    /// what a step prints would be taken as it is printed.
    pub async fn keep_logs_for(&self, days: Option<u32>, by: &str) -> Result<(Keeping, Keeping)> {
        if let Some(days) = days.filter(|days| *days == 0 || *days > MOST_DAYS) {
            return Err(Error::invalid(format!(
                "logs are kept for a day at least and {MOST_DAYS} at most, and {days} is neither; or say \
                 `always` to keep them for ever"
            )));
        }
        let was = self.logs_kept().await;
        let now = Keeping { days, by: Some(by.to_string()), at: Some(self.now_ms()), went: was.went };
        self.kv_set(KEEPS, KEY, &json!(now)).await?;
        Ok((was, now))
    }

    /// Keeps how far the detail of runs' steps was let go of, and has
    /// `logs` say so.
    pub async fn went(&self, logs: &Logs, went: Went) -> Result<()> {
        let mut now = self.logs_kept().await;
        now.went = Some(went);
        self.kv_set(KEEPS, KEY, &json!(now)).await?;
        logs.has_gone(Some(went));
        Ok(())
    }

    /// Where a run's steps write, as was said.
    pub async fn writing(&self) -> Writing {
        let kept = self.kv_get(WRITES, KEY).await.ok().flatten();
        kept.and_then(|kept| serde_json::from_value(kept).ok()).unwrap_or_default()
    }

    /// Has `logs` write and read where was said: for a factory as it
    /// starts.
    pub async fn tell_logs(&self, logs: &Logs) {
        logs.has_gone(self.logs_kept().await.went);
        let writing = self.writing().await;
        let before = writing.before.iter().map(PathBuf::from).collect();
        logs.write_runs_in(writing.folder.map(PathBuf::from), before);
    }

    /// Says where: in `folder`, which is made where it is not there and
    /// refused where nothing can be written in it, or with nothing in the
    /// data folder. `logs` writes there from now, and goes on reading
    /// where it wrote before. Gives back what was said before and now.
    pub async fn write_logs_in(
        &self,
        logs: &Logs,
        folder: Option<&str>,
        by: &str,
    ) -> Result<(Writing, Writing)> {
        let folder = match folder {
            Some(folder) => Some(writable(folder)?.to_string_lossy().into_owned()),
            None => None,
        };
        let was = self.writing().await;
        let mut before: Vec<String> = was.folder.iter().chain(&was.before).cloned().collect();
        before.retain(|one| Some(one) != folder.as_ref());
        before.dedup();
        let now = Writing { folder, before, by: Some(by.to_string()), at: Some(self.now_ms()) };
        self.kv_set(WRITES, KEY, &json!(now)).await?;
        self.tell_logs(logs).await;
        Ok((was, now))
    }

    /// Takes out of this journal the events on `topics` said before
    /// `before` (milliseconds since 1970), but for any numbered above
    /// `below` and any of the runs in `keep`. Gives back how many went.
    ///
    /// For whoever lets what was said of runs' steps go: `below` is the
    /// least place anything alive that could be owed one of them keeps
    /// here, so that nothing is taken that was not yet handed to whoever
    /// is owed it, and `keep` the runs under way, which read back what
    /// they said of themselves when they are taken up.
    ///
    /// Where the old ones end is found as [`Journal::prune_events`] finds
    /// it, from the few events whose time is in an index; what is taken
    /// is then taken a piece of work at a time, in the order the events
    /// were said. Events are taken from the middle of the journal, so it
    /// keeps that its events have holes the gaps do not say
    /// ([`crate::numbered::HOLES`]): the newest so many are then counted
    /// from its rows, and not over the gaps, which would count what was
    /// taken here as there.
    pub async fn let_go_of(
        &self,
        topics: &[&str],
        before: i64,
        below: Option<u64>,
        keep: &[String],
    ) -> Result<usize> {
        if topics.is_empty() || below == Some(0) {
            return Ok(0);
        }
        let below = below.map_or(i64::MAX, |below| below.min(i64::MAX as u64) as i64);
        let end = self.inner.db.call(move |connection| old_end(connection, before, below)).await?;
        let (topics, keep) = (json!(topics).to_string(), json!(keep).to_string());
        let (mut gone, mut after) = (0, 0i64);
        loop {
            let (inner, topics, keep) = (self.inner.clone(), topics.clone(), keep.clone());
            let taken = self.inner.db.call(move |connection| {
                let transaction = crate::gather::whole(connection)?;
                let taken: Vec<i64> = {
                    let mut statement = transaction.prepare(
                        "SELECT seq FROM events
                         WHERE topic IN (SELECT value FROM json_each(?1)) AND seq > ?2 AND seq <= ?3 AND ts < ?4
                           AND (run IS NULL OR run NOT IN (SELECT value FROM json_each(?5)))
                         ORDER BY seq LIMIT ?6",
                    )?;
                    let rows = statement.query_map(
                        rusqlite::params![topics, after, end, before, keep, AT_A_TIME as i64],
                        |row| row.get(0),
                    )?;
                    rows.collect::<rusqlite::Result<_>>()?
                };
                if !taken.is_empty() {
                    transaction.execute(
                        "DELETE FROM events WHERE seq IN (SELECT value FROM json_each(?1))",
                        [json!(taken).to_string()],
                    )?;
                    crate::numbered::keep_said(&transaction, inner.given.load(Ordering::SeqCst) as i64)?;
                    crate::numbered::keep_holes(&transaction, "main")?;
                }
                transaction.commit()?;
                Ok(taken)
            });
            let Some(taken) = self.unless_shut(taken).await else { break };
            let taken = taken?;
            gone += taken.len();
            match taken.last() {
                Some(last) if taken.len() == AT_A_TIME => after = *last,
                _ => break,
            }
        }
        Ok(gone)
    }
}

/// The number at or below which every event of the journal that is not
/// numbered above `below` was said before `before`, as far as that can be
/// found from the index of when: from the last of the few in it that is
/// old, event by event until one is not.
fn old_end(connection: &rusqlite::Connection, before: i64, below: i64) -> rusqlite::Result<i64> {
    let sampled: i64 = connection.query_row(
        "SELECT coalesce(max(seq), 0) FROM events INDEXED BY events_when
         WHERE seq % 512 = 0 AND ts < ?1 AND seq <= ?2",
        [before, below],
        |row| row.get(0),
    )?;
    let newer: Option<i64> = crate::optional(connection.query_row(
        "SELECT seq FROM events WHERE seq > ?1 AND seq <= ?2 AND ts >= ?3 ORDER BY seq LIMIT 1",
        [sampled, below, before],
        |row| row.get(0),
    ))?;
    // Just before the first that is not old: a place, and no number of
    // another event's.
    Ok(newer.map_or(below, |seq| seq - 1))
}

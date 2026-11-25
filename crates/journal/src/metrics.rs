//! Measurements, rolled up by the minute.
//!
//! Anything that takes time or can fail is measured the same way: a name, a
//! few labels, a duration and whether it worked. Samples are aggregated in
//! memory and written once per flush as one row per (minute, name, labels),
//! so measuring costs a hash lookup, and a busy hour is a few hundred rows
//! rather than a few hundred thousand.
//!
//! Each row keeps a count, an error count, sum, min, max and a small
//! log-scale histogram, which is enough to answer the questions that matter
//! (how often, how often it fails, typical and worst-case duration) over any
//! window, and survives a restart, unlike counters kept "since boot".
//!
//! A minute with no samples has no row. Absence is reported as absence,
//! never as zero.
//!
//! Not everything measured is worth a write of its own. How long the
//! database took over each piece of its work, and how loaded the device
//! is, are measured whether or not anything is happening: written out as
//! they came, a factory nobody was using wrote to its disk every ten
//! seconds for ever, most of it the measurement of the write before. Such
//! measurements are taken quietly ([`Journal::measure_quietly`]): they are
//! kept in memory, where every reader sees them as it sees the rest, and
//! are written with the next thing that is worth a write, when the daemon
//! stops, or once they have waited an hour.
//!
//! What was measured is in memory or in the database at every instant,
//! never in neither and never in both, for whoever reads it. Both sides
//! of that are pieces of the database's work, which its thread does one
//! at a time: a flush takes what it writes out of memory in the piece
//! that writes it, and a reading looks in memory in the piece that reads
//! what is stored. Taken out of memory before the write was on the
//! thread, as it was, what was being written was in neither for a
//! reading that read the database before the write and memory after the
//! taking: a comparison once answered three turns where six were meant.
//! What a write that is not kept was to keep is put back in memory
//! before anything else is done on the thread ([`crate::gather::then`]).

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::{Arc, Mutex};

use kernel::Result;
use serde::Serialize;

use crate::Journal;

/// Histogram buckets: bucket `i` holds values below `2^i`. Thirty-two buckets
/// reach from under a millisecond to about 25 days.
const BUCKETS: usize = 32;

#[derive(Clone, Debug)]
struct Sample {
    count: u64,
    errors: u64,
    sum: f64,
    min: f64,
    max: f64,
    hist: [u64; BUCKETS],
}

impl Default for Sample {
    fn default() -> Self {
        Self { count: 0, errors: 0, sum: 0.0, min: f64::INFINITY, max: f64::NEG_INFINITY, hist: [0; BUCKETS] }
    }
}

impl Sample {
    fn record(&mut self, value: f64, ok: bool) {
        self.count += 1;
        self.errors += u64::from(!ok);
        self.sum += value;
        self.min = self.min.min(value);
        self.max = self.max.max(value);
        self.hist[bucket(value)] += 1;
    }

    fn merge(&mut self, other: &Sample) {
        self.count += other.count;
        self.errors += other.errors;
        self.sum += other.sum;
        self.min = self.min.min(other.min);
        self.max = self.max.max(other.max);
        for (mine, theirs) in self.hist.iter_mut().zip(&other.hist) {
            *mine += theirs;
        }
    }

    /// An estimate of the value below which `fraction` of samples fall: the
    /// upper edge of the bucket that crosses it, clamped to what was seen.
    fn quantile(&self, fraction: f64) -> f64 {
        let wanted = (self.count as f64 * fraction).ceil().max(1.0) as u64;
        let mut seen = 0;
        for (index, count) in self.hist.iter().enumerate() {
            seen += count;
            if seen >= wanted {
                return ((1u64 << index) as f64).clamp(self.min, self.max);
            }
        }
        self.max
    }

    fn encode_hist(&self) -> String {
        let parts: Vec<String> =
            self.hist.iter().enumerate().filter(|(_, n)| **n > 0).map(|(i, n)| format!("{i}:{n}")).collect();
        parts.join(",")
    }

    fn decode_hist(text: &str) -> [u64; BUCKETS] {
        let mut hist = [0; BUCKETS];
        for part in text.split(',') {
            if let Some((index, count)) = part.split_once(':')
                && let (Ok(index), Ok(count)) = (index.parse::<usize>(), count.parse::<u64>())
                && index < BUCKETS
            {
                hist[index] = count;
            }
        }
        hist
    }

    fn stats(&self) -> MetricStats {
        MetricStats {
            count: self.count,
            errors: self.errors,
            error_rate: if self.count == 0 { 0.0 } else { self.errors as f64 / self.count as f64 },
            avg: if self.count == 0 { 0.0 } else { self.sum / self.count as f64 },
            sum: self.sum,
            min: self.min,
            max: self.max,
            p50: self.quantile(0.5),
            p95: self.quantile(0.95),
        }
    }
}

fn bucket(value: f64) -> usize {
    if value < 1.0 {
        return 0;
    }
    ((value as u64).ilog2() as usize + 1).min(BUCKETS - 1)
}

type Key = (i64, String, String);

/// How long what was measured quietly may wait for something worth a write,
/// in minutes, before it is written by itself.
const QUIET_FOR_MINUTES: i64 = 60;

/// What has been measured and not yet written.
#[derive(Default)]
struct Pending {
    samples: HashMap<Key, Sample>,
    /// Whether any of it is worth a write by itself.
    worth_a_write: bool,
}

impl Pending {
    /// Puts back what a write that was not kept had taken.
    fn put_back(&mut self, samples: HashMap<Key, Sample>, worth_a_write: bool) {
        for (key, sample) in samples {
            self.samples.entry(key).or_default().merge(&sample);
        }
        self.worth_a_write |= worth_a_write;
    }
}

/// Shared with the pieces of the database's work that take what is to be
/// written out of it, and with what they leave to follow their commit.
#[derive(Default)]
pub(crate) struct Aggregator {
    pending: Arc<Mutex<Pending>>,
}

/// Which measurements to read.
#[derive(Clone, Debug, Default)]
pub struct MetricQuery {
    /// Exact name, or a prefix ending in `*` (e.g. `http.*`).
    pub name: String,
    /// Only series carrying all of these labels. A label named more
    /// than once is carried with any of the values it is named with
    /// (`project` twice: what was measured for either project).
    pub labels: Vec<(String, String)>,
    /// Inclusive start, in milliseconds since the epoch.
    pub since_ms: i64,
    /// Exclusive end; `0` means now.
    pub until_ms: i64,
    /// Label keys to split the result by. Empty gives one series per name.
    pub group_by: Vec<String>,
    /// Width of each point in minutes; `0` returns totals only.
    pub step_minutes: i64,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct MetricStats {
    pub count: u64,
    pub errors: u64,
    pub error_rate: f64,
    pub avg: f64,
    /// Every value added up: what a cost came to in all. Of times it is
    /// seldom what is wanted, and is there all the same.
    pub sum: f64,
    pub min: f64,
    pub max: f64,
    pub p50: f64,
    pub p95: f64,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct MetricSeries {
    pub name: String,
    pub labels: BTreeMap<String, String>,
    pub total: MetricStats,
    /// `(start of bucket in ms, stats)`, oldest first. Buckets with no
    /// samples are absent.
    pub points: Vec<(i64, MetricStats)>,
}

/// `a=1,b=2` with keys sorted, so equal label sets are equal strings.
fn encode_labels(labels: &[(&str, &str)]) -> String {
    let mut pairs: Vec<&(&str, &str)> = labels.iter().filter(|(_, value)| !value.is_empty()).collect();
    pairs.sort();
    pairs
        .iter()
        .map(|(key, value)| format!("{key}={}", value.replace([',', '='], "_")))
        .collect::<Vec<_>>()
        .join(",")
}

/// Whether what is kept under `labels` is what `asked` asks for: of each
/// label asked, it carries a value that label was asked with. One asked
/// once is carried with that value; one asked several times over (two
/// projects) with any of them, since nothing carries a label twice and
/// asked for every one of them nothing would be found.
fn carries(labels: &BTreeMap<String, String>, asked: &[(String, String)]) -> bool {
    asked.iter().all(|(key, _)| {
        let has = labels.get(key);
        asked.iter().any(|(named, value)| named == key && has == Some(value))
    })
}

fn decode_labels(text: &str) -> BTreeMap<String, String> {
    text.split(',')
        .filter_map(|pair| pair.split_once('='))
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

impl Journal {
    /// Records that something named `name` took `value` (milliseconds, for
    /// durations) and whether it succeeded.
    pub fn measure(&self, name: &str, labels: &[(&str, &str)], value: f64, ok: bool) {
        self.record(name, labels, value, ok, true);
    }

    /// Records a measurement that is not worth a write of its own: one
    /// taken whether or not anything is happening. It is read like any
    /// other from the moment it is taken, and is written with the next
    /// measurement that is worth a write, or after an hour.
    pub fn measure_quietly(&self, name: &str, labels: &[(&str, &str)], value: f64, ok: bool) {
        self.record(name, labels, value, ok, false);
    }

    fn record(&self, name: &str, labels: &[(&str, &str)], value: f64, ok: bool, worth_a_write: bool) {
        // What is measured through a journal kept beside a factory's own
        // is kept in the factory's, where measurements are read.
        if let Some(factory) = &self.inner.measured_in {
            if let Some(inner) = factory.upgrade() {
                Journal { inner }.record(name, labels, value, ok, worth_a_write);
            }
            return;
        }
        let minute = self.now_ms().div_euclid(60_000);
        let key = (minute, name.to_string(), encode_labels(labels));
        let mut pending = self.inner.metrics.pending.lock().expect("metrics lock");
        pending.samples.entry(key).or_default().record(value, ok);
        pending.worth_a_write |= worth_a_write;
    }

    /// Writes pending measurements to the database if any of them is worth
    /// a write, or has waited an hour for one that is. This is what is
    /// asked on a timer: a factory at rest is then not written to.
    pub async fn flush_metrics_when_due(&self) -> Result<()> {
        let due = {
            let pending = self.inner.metrics.pending.lock().expect("metrics lock");
            let now = self.now_ms().div_euclid(60_000);
            pending.worth_a_write
                || pending.samples.keys().any(|(minute, ..)| now - minute >= QUIET_FOR_MINUTES)
        };
        if due { self.flush_metrics().await } else { Ok(()) }
    }

    /// Writes pending measurements to the database, whatever they are.
    /// They are taken out of memory on the database's thread, in the
    /// piece of work that writes them, so that a reading (which looks in
    /// memory in the piece that reads what is stored) finds them in one
    /// place or the other. Where what was written is not kept, they are
    /// put back before the thread does anything else.
    pub async fn flush_metrics(&self) -> Result<()> {
        if self.inner.metrics.pending.lock().expect("metrics lock").samples.is_empty() {
            return Ok(());
        }
        let pending = Arc::clone(&self.inner.metrics.pending);
        self.inner
            .db
            .call(move |connection| {
                let (samples, worth_a_write) = {
                    let mut held = pending.lock().expect("metrics lock");
                    (std::mem::take(&mut held.samples), std::mem::take(&mut held.worth_a_write))
                };
                if samples.is_empty() {
                    return Ok(());
                }
                match write(connection, &samples) {
                    Ok(()) => {
                        crate::gather::then(move |kept| {
                            if !kept {
                                pending.lock().expect("metrics lock").put_back(samples, worth_a_write);
                            }
                        });
                        Ok(())
                    }
                    Err(err) => {
                        pending.lock().expect("metrics lock").put_back(samples, worth_a_write);
                        Err(err)
                    }
                }
            })
            .await
    }

    /// Every rollup in the query's window whose name it asks for, stored or
    /// not yet flushed.
    async fn rollups(&self, query: &MetricQuery) -> Result<Vec<(Key, Sample)>> {
        let until_ms = if query.until_ms > 0 { query.until_ms } else { self.now_ms() + 1 };
        let (first, last) = (query.since_ms.div_euclid(60_000), (until_ms - 1).div_euclid(60_000));
        let (prefix, exact) = match query.name.strip_suffix('*') {
            Some(prefix) => (prefix.to_string(), false),
            None => (query.name.clone(), true),
        };
        let matches_name = move |name: &str| if exact { name == prefix } else { name.starts_with(&prefix) };

        // What is stored and what is in memory are read in one piece of
        // the database's work: a flush takes what it writes out of memory
        // in a piece of its own there, which is before this or after it.
        let pending = Arc::clone(&self.inner.metrics.pending);
        self.inner
            .db
            .call(move |connection| {
                let mut statement = connection.prepare(
                    "SELECT minute, name, labels, count, errors, sum, min, max, hist FROM metrics
                         WHERE minute >= ?1 AND minute <= ?2",
                )?;
                let mut rows = statement.query([first, last])?;
                let mut found: Vec<(Key, Sample)> = Vec::new();
                while let Some(row) = rows.next()? {
                    let name: String = row.get(1)?;
                    if matches_name(&name) {
                        let sample = Sample {
                            count: row.get::<_, i64>(3)? as u64,
                            errors: row.get::<_, i64>(4)? as u64,
                            sum: row.get(5)?,
                            min: row.get(6)?,
                            max: row.get(7)?,
                            hist: Sample::decode_hist(&row.get::<_, String>(8)?),
                        };
                        found.push(((row.get(0)?, name, row.get(2)?), sample));
                    }
                }
                let pending = pending.lock().expect("metrics lock");
                found.extend(
                    pending
                        .samples
                        .iter()
                        .filter(|((minute, name, _), _)| {
                            (first..=last).contains(minute) && matches_name(name)
                        })
                        .map(|(key, sample)| (key.clone(), sample.clone())),
                );
                Ok(found)
            })
            .await
    }

    /// Reads measurements, including any not yet flushed.
    pub async fn query_metrics(&self, query: MetricQuery) -> Result<Vec<MetricSeries>> {
        let rollups = self.rollups(&query).await?;

        // Group, then bucket by time within each group.
        type Group = (String, BTreeMap<String, String>);
        let mut groups: BTreeMap<Group, (Sample, BTreeMap<i64, Sample>)> = BTreeMap::new();
        for ((minute, name, labels), sample) in rollups {
            let labels = decode_labels(&labels);
            if !carries(&labels, &query.labels) {
                continue;
            }
            let kept: BTreeMap<String, String> = query
                .group_by
                .iter()
                .filter_map(|key| Some((key.clone(), labels.get(key)?.clone())))
                .collect();
            let (total, points) = groups.entry((name, kept)).or_default();
            total.merge(&sample);
            if query.step_minutes > 0 {
                let start = minute.div_euclid(query.step_minutes) * query.step_minutes;
                points.entry(start).or_default().merge(&sample);
            }
        }
        Ok(groups
            .into_iter()
            .map(|((name, labels), (total, points))| MetricSeries {
                name,
                labels,
                total: total.stats(),
                points: points
                    .into_iter()
                    .map(|(minute, sample)| (minute * 60_000, sample.stats()))
                    .collect(),
            })
            .collect())
    }

    /// The labels that what the query asks for is kept by, each with the
    /// values it has there, for whoever offers them to be asked by. `own`
    /// names what measurements say of themselves, which is left out: what
    /// remains is what was said about them from outside.
    pub async fn metric_labels(
        &self,
        query: MetricQuery,
        own: &[&str],
    ) -> Result<BTreeMap<String, BTreeSet<String>>> {
        let mut found: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for ((_, _, labels), _) in self.rollups(&query).await? {
            let labels = decode_labels(&labels);
            if !carries(&labels, &query.labels) {
                continue;
            }
            for (name, value) in labels {
                if !own.contains(&name.as_str()) {
                    found.entry(name).or_default().insert(value);
                }
            }
        }
        Ok(found)
    }

    /// Deletes rollups older than `before_ms`.
    pub async fn prune_metrics(&self, before_ms: i64) -> Result<usize> {
        let minute = before_ms.div_euclid(60_000);
        self.inner
            .db
            .call(move |connection| connection.execute("DELETE FROM metrics WHERE minute < ?1", [minute]))
            .await
    }
}

/// Adds `samples` to what is stored, in a whole of the piece of work's own.
fn write(connection: &mut rusqlite::Connection, samples: &HashMap<Key, Sample>) -> rusqlite::Result<()> {
    let transaction = crate::gather::whole(connection)?;
    for ((minute, name, labels), sample) in samples {
        let mut sample = sample.clone();
        let existing = crate::optional(transaction.query_row(
            "SELECT count, errors, sum, min, max, hist FROM metrics
             WHERE minute = ?1 AND name = ?2 AND labels = ?3",
            rusqlite::params![minute, name, labels],
            read_sample,
        ))?;
        if let Some(existing) = existing {
            sample.merge(&existing);
        }
        transaction.execute(
            "INSERT OR REPLACE INTO metrics (minute, name, labels, count, errors, sum, min, max, hist)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![
                minute,
                name,
                labels,
                sample.count as i64,
                sample.errors as i64,
                sample.sum,
                sample.min,
                sample.max,
                sample.encode_hist()
            ],
        )?;
    }
    transaction.commit()
}

fn read_sample(row: &rusqlite::Row<'_>) -> rusqlite::Result<Sample> {
    Ok(Sample {
        count: row.get::<_, i64>(0)? as u64,
        errors: row.get::<_, i64>(1)? as u64,
        sum: row.get(2)?,
        min: row.get(3)?,
        max: row.get(4)?,
        hist: Sample::decode_hist(&row.get::<_, String>(5)?),
    })
}

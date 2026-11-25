//! Letting the detail of runs' steps go after so many days, as the
//! factory's owner says (`journal::letgo`), and where their output is
//! written.
//!
//! What goes is what was said of steps while they ran, and nothing a run
//! came to: the files its steps printed into, and of the runner's own
//! events those it says of itself as a step and a run go ([`TOPICS`]).
//! How a run ended (`run.finished`, `run.failed`, where they were kept),
//! its row and its steps' rows with what each gave, everything said of
//! instances, and everything anybody else says, stay. All of what the
//! runner says of its own is kept at all only where a text in force could
//! be owed it (`crate::owed`), and only passed on otherwise: what goes
//! here is what something heard and has been handed.
//!
//! **Nothing goes that is owed to anybody.** Something alive whose text
//! hears one of those topics keeps a place in each journal it hears from,
//! and is owed everything said there after it: nothing after the least
//! such place goes ([`Engine::held_back`]). A `wait` for one of them keeps
//! its place with its step, and while something whose text waits for one
//! is alive, none of them goes anywhere. While a conductor is taking
//! everything up again, not every text is in force, and none goes. What
//! a comparison counts is held the same way, by the place it counts
//! from, which whoever asks hands in. And a run under way reads back what
//! it said of itself when it is taken up, so nothing of it goes, events
//! or files: a step that waits writes nothing for as long as it waits.
//!
//! It is done as the daemon starts, on its timer, once an hour, and when
//! the owner says how long (`api`, which asks [`Engine::let_logs_go`]).

use std::collections::{BTreeMap, HashSet};
use std::time::Instant;

use journal::letgo::{Keeping, Went, Writing};
use kernel::{Principal, Result, Source};
use runs::InstanceFilter;
use serde::Serialize;
use serde_json::json;

use crate::engine::Engine;

/// What the runner says of its own steps and runs as they go, and lets go
/// of with their logs: every `step.*`, and that a run began or was taken
/// up again. What a run came to is not among them.
pub const TOPICS: &[&str] = &[
    "run.started",
    "run.resumed",
    "step.started",
    "step.finished",
    "step.failed",
    "step.retrying",
    "step.skipped",
    "step.cancelled",
    "step.waiting",
];

const DAY_MS: i64 = 24 * 3600 * 1000;

/// What was let go of.
#[derive(Clone, Debug, Default, Serialize)]
pub struct LetGo {
    /// Of what was last written before this, in milliseconds since 1970.
    pub before: i64,
    pub days: u32,
    /// Events, and the files of steps with the bytes they held.
    pub events: usize,
    pub files: u64,
    pub bytes: u64,
    /// Where events were held back for being owed, a sentence; nothing
    /// where none was.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub held: Option<String>,
}

impl Engine {
    /// How long the detail of runs' steps is kept.
    pub async fn logs_kept(&self) -> Keeping {
        self.journal().logs_kept().await
    }

    /// Where runs' steps write.
    pub async fn logs_written(&self) -> Writing {
        self.journal().writing().await
    }

    /// What is said of `run`'s steps where their detail was let go.
    pub fn let_go_said(&self, ended: Option<i64>) -> Option<String> {
        journal::letgo::said_of_run(self.inner.logs.went(), ended)
    }

    /// Says how long the detail of runs' steps is kept, and says so.
    pub async fn keep_logs_for(&self, days: Option<u32>, by: &Principal) -> Result<Keeping> {
        let (was, now) = self.journal().keep_logs_for(days, &by.name).await?;
        let said = json!({ "days": now.days, "was": was.days, "by": by.actor(), "by_name": by.name });
        self.emit("system.logs", Source::default(), said).await;
        Ok(now)
    }

    /// Says where runs' steps write from now, and says so.
    pub async fn write_logs_in(&self, folder: Option<&str>, by: &Principal) -> Result<Writing> {
        let (was, now) = self.journal().write_logs_in(&self.inner.logs, folder, &by.name).await?;
        let said = json!({ "folder": now.folder, "was": was.folder, "by": by.actor(), "by_name": by.name });
        self.emit("system.logs", Source::default(), said).await;
        Ok(now)
    }

    /// Lets go of what is older than the factory keeps, where it says,
    /// in every journal and every folder; `counted` is, for each journal
    /// by its name, the place a comparison counts from there, which
    /// nothing after goes. Nothing where it keeps everything. Measured,
    /// and said where anything went.
    pub async fn let_logs_go(&self, counted: &BTreeMap<String, u64>) -> Result<Option<LetGo>> {
        let Some(days) = self.logs_kept().await.days else { return Ok(None) };
        let began = Instant::now();
        let before = self.journal().now_ms() - i64::from(days) * DAY_MS;
        let mut going = Vec::new();
        for conductor in self.every() {
            going.extend(conductor.inner.store.unfinished_runs().await?.into_iter().map(|run| run.id));
        }
        let mut done = LetGo { before, days, ..LetGo::default() };
        match self.held_back().await? {
            Err(why) => done.held = Some(format!("nothing said of steps went, since {why}")),
            Ok(held) => {
                for journal in self.journal().every() {
                    let below =
                        [held.get(journal.name()), counted.get(journal.name())].into_iter().flatten().min();
                    let gone = journal.let_go_of(TOPICS, before, below.copied(), &going).await?;
                    done.events += gone;
                    if below.is_some() {
                        done.held = Some("what something alive is still owed was kept".into());
                    }
                }
            }
        }
        let (logs, keep) = (self.inner.logs.clone(), going.into_iter().collect::<HashSet<String>>());
        let files = tokio::task::spawn_blocking(move || logs.let_go_before(before, &keep));
        (done.files, done.bytes) = files.await.unwrap_or_default();
        self.journal().went(&self.inner.logs, Went { before, days }).await?;
        let journal = self.journal();
        journal.measure("system.logs_let_go", &[], began.elapsed().as_secs_f64() * 1000.0, true);
        journal.measure("system.logs_let_go_events_count", &[], done.events as f64, true);
        journal.measure("system.logs_let_go_files_count", &[], done.files as f64, true);
        journal.measure("system.logs_let_go_bytes", &[], done.bytes as f64, true);
        if done.events > 0 || done.files > 0 {
            self.emit("system.logs.let_go", Source::default(), json!(done)).await;
        }
        Ok(Some(done))
    }

    /// For each journal by its name, the least place anything alive that
    /// could be owed what [`TOPICS`] are keeps there; a journal not named
    /// holds nothing back. Why not, in words, where everything is held: a
    /// conductor is taking everything up again, something alive is not in
    /// hand (its text is not known), or something alive waits for one.
    pub(crate) async fn held_back(&self) -> Result<std::result::Result<BTreeMap<String, u64>, String>> {
        if self.inner.owed.is_resuming() {
            return Ok(Err("everything was being taken up again".into()));
        }
        let mut least: BTreeMap<String, u64> = BTreeMap::new();
        for conductor in self.every() {
            let alive =
                InstanceFilter { live_only: true, limit: usize::MAX >> 1, ..InstanceFilter::default() };
            let alive = conductor.inner.store.instances(alive).await?;
            for row in alive {
                let live = conductor.inner.live.lock().expect("engine lock").get(&row.id).cloned();
                let Some(live) = live else {
                    return Ok(Err(format!("`{}` is alive and was not in hand", row.blueprint)));
                };
                let blueprint = live.blueprint();
                let (mut handlers, mut waits) = (false, false);
                for topic in TOPICS {
                    let (hears, waits_for) = crate::owed::heard_by(&blueprint, topic);
                    (handlers, waits) = (handlers || hears, waits || waits_for);
                }
                if waits {
                    return Ok(Err(format!("`{}` waits for what is said of steps", row.blueprint)));
                }
                if !handlers {
                    continue;
                }
                let everywhere = blueprint.triggers.iter().any(crate::trigger::everywhere);
                for journal in conductor.hears_from(&live, everywhere) {
                    let place = conductor.inner.store.least_place(&row.id, journal.name()).await?;
                    // One with no place there yet reads it from its beginning.
                    let place = place.unwrap_or(0);
                    let at = least.entry(journal.name().to_string()).or_insert(place);
                    *at = (*at).min(place);
                }
            }
        }
        Ok(Ok(least))
    }
}

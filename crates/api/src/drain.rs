//! A daemon asked to stop once what it has in hand is done
//! (`mw down --drain`, the route `/api/system/stop` with `drain`).
//!
//! From the moment it is asked it begins nothing new: nothing somebody
//! applies, no handler's run (each waits with the event it was handed
//! and its place unmoved, for the next daemon: `Engine::hold_new_work`)
//! and no turn that no run asked for (`agents::drained`). It then waits
//! for the turns in hand and the runs under way to end, and stops as
//! `down` stops. What waits for a person (a review, a question, a held
//! card) has no run and no turn while it waits, so it holds nothing up.
//!
//! The wait always has a bound, said as it begins: past it the daemon
//! stops anyway and says what was still in hand and what becomes of each.
//! What it waits for is looked at again each time a turn or a run is said
//! to have ended, and never on a timer: the bound is the one time there
//! is (*Being told, not looking* in `docs/architecture.md`).

use std::sync::Mutex;
use std::time::Duration;

use kernel::{Error, Result, Source};
use serde_json::{Value, json};

use crate::state::App;

/// How long a drain waits where nobody says.
pub const USUAL: Duration = Duration::from_secs(600);

/// The longest a drain may be told to wait.
pub const LONGEST: Duration = Duration::from_secs(24 * 3600);

/// What is heard that may end a drain.
const ENDINGS: &str = "run.finished,agent.turn.finished";

/// The drain of this daemon, once it was asked for.
#[derive(Default)]
pub struct Drain {
    /// When it began and the latest it stops, in milliseconds.
    asked: Mutex<Option<(i64, i64)>>,
    /// How it came out, once it has: what whoever waits is answered.
    over: tokio::sync::OnceCell<Value>,
    told: tokio::sync::Notify,
}

impl Drain {
    /// When it began and the latest it stops, where it was asked for.
    pub fn asked(&self) -> Option<(i64, i64)> {
        *self.asked.lock().expect("drain lock")
    }

    /// The same as it is handed out: nothing where no drain was asked for.
    pub fn said(&self) -> Value {
        match self.asked() {
            Some((since, until)) => json!({ "since": since, "until": until }),
            None => Value::Null,
        }
    }
}

/// How long to wait, as somebody said it: the usual where nothing was.
pub fn within(said: Option<&str>) -> Result<Duration> {
    let Some(said) = said.filter(|said| !said.trim().is_empty()) else { return Ok(USUAL) };
    let ms = blueprint::duration::parse(said).map_err(Error::invalid)?;
    let within = Duration::from_millis(ms);
    if within.is_zero() || within > LONGEST {
        return Err(Error::invalid(format!(
            "a drain waits for more than no time and a day at most, and not `{said}`: say how long, such as `10m`, \
             or stop at once with `{} down`",
            brand::CLI
        )));
    }
    Ok(within)
}

/// Begins draining, or joins the drain begun already, which then stops
/// by the sooner of the two bounds. What it waits for, as it begins.
pub async fn begin(app: &App, within: Duration, by: Source) -> Result<Value> {
    let now = app.journal.now_ms();
    let latest = now + within.as_millis() as i64;
    let first = {
        let mut asked = app.drain.asked.lock().expect("drain lock");
        let first = asked.is_none();
        let (since, until) = asked.unwrap_or((now, latest));
        *asked = Some((since, until.min(latest)));
        first
    };
    app.drain.told.notify_waiters();
    let waits = waits(app).await?;
    let (since, until) = app.drain.asked().unwrap_or((now, latest));
    let said = json!({ "since": since, "until": until, "turns": waits.0.len(), "runs": waits.1.len() });
    let _ = app.journal.emit("system.draining", by, said).await;
    if first {
        app.engine.hold_new_work();
        app.agents.drain();
        let app = app.clone();
        tokio::spawn(async move { drained(app).await });
    }
    standing(app).await
}

/// How a drain stands: when it began, the latest it stops, and what it
/// waits for. Nothing where no drain was asked for.
pub async fn standing(app: &App) -> Result<Value> {
    let Some((since, until)) = app.drain.asked() else { return Ok(json!({ "draining": null })) };
    let (turns, runs) = waits(app).await?;
    Ok(json!({
        "draining": { "since": since, "until": until },
        "turns": turns,
        "runs": runs,
        "over": app.drain.over.get(),
    }))
}

/// How the drain came out, once it has: waited for.
pub async fn over(app: &App) -> Value {
    loop {
        let told = app.drain.told.notified();
        if let Some(over) = app.drain.over.get() {
            return over.clone();
        }
        told.await;
    }
}

/// The turns in hand, with what becomes of each where the daemon stops
/// now, and the runs under way, each in words.
async fn waits(app: &App) -> Result<(Vec<Value>, Vec<Value>)> {
    let turns = app.agents.in_hand_now().await?;
    let turns = turns.into_iter().map(|turn| json!(turn)).collect();
    let mut runs = Vec::new();
    for run in app.engine.under_way().await? {
        let of = app.engine.instance(&run.instance).await.map(|instance| instance.blueprint);
        runs.push(json!({
            "run": run.id,
            "of": of.unwrap_or_else(|_| run.instance.clone()),
            "flow": run.flow,
            "becomes": "the next start takes it up, as it does at any stop",
        }));
    }
    Ok((turns, runs))
}

/// Waits for what is in hand to end, or for the bound, and stops.
async fn drained(app: App) {
    let mut ended = app.journal.follow_every(app.journal.now(), ENDINGS);
    let outcome = loop {
        let Ok((turns, runs)) = waits(&app).await else { break json!({ "drained": false, "cut": [] }) };
        if turns.is_empty() && runs.is_empty() {
            break json!({ "drained": true, "cut": [] });
        }
        let until = app.drain.asked().map_or(0, |(_, until)| until);
        if app.journal.now_ms() >= until {
            break json!({ "drained": false, "cut": cut(&turns, &runs) });
        }
        let bound = app.journal.clock().sleep_until(until);
        tokio::select! {
            _ = ended.next_or_behind() => {}
            () = bound => {}
            // A second drain may have brought the bound nearer.
            () = app.drain.told.notified() => {}
        }
    };
    let waited = app.journal.now_ms() - app.drain.asked().map_or(0, |(since, _)| since);
    let outcome = json!({ "drained": outcome["drained"], "cut": outcome["cut"], "waited_ms": waited });
    let said = json!({ "drained": outcome["drained"], "cut": outcome["cut"] });
    let _ = app.journal.emit("system.stopping", Source::actor("system"), said).await;
    let _ = app.drain.over.set(outcome);
    app.drain.told.notify_waiters();
    // After whoever waits has been answered, as a plain stop does.
    app.journal.clock().sleep(Duration::from_millis(100)).await;
    app.stop.notify_one();
}

/// What was still in hand at the bound, a sentence each.
fn cut(turns: &[Value], runs: &[Value]) -> Vec<String> {
    let turns = turns.iter().map(|turn| {
        format!("{}'s turn {}: {}", text(turn, "slot"), text(turn, "turn"), text(turn, "becomes"))
    });
    let runs = runs.iter().map(|run| {
        format!(
            "a run of {} ({}) {}: {}",
            text(run, "of"),
            text(run, "flow"),
            text(run, "run"),
            text(run, "becomes")
        )
    });
    turns.chain(runs).collect()
}

fn text<'a>(value: &'a Value, key: &str) -> &'a str {
    value[key].as_str().unwrap_or_default()
}

//! A row that steers what runs is in effect when its write is answered.
//!
//! A table whose rows say what a running instance does (one that
//! `steers` another, or one another steers) is heard by that instance's
//! handlers after the write, each in its own time. Answered at once, a
//! write left a gap in which whoever wrote it, and went on in the next
//! request, had it go the way it went before, without a word. So a write
//! of such a row through any door of the API (the client, `mw record`, a
//! key's program) is answered once each handler that hears it has done
//! with it, and with each such row that what they did wrote in turn
//! (`conductor::handled`). Within [`WITHIN`]: past that it is answered
//! all the same, saying what had not taken it yet. A write of any other
//! table is answered as it always was.
//!
//! Nothing is kept of the wait. A daemon stopped while a write waits has
//! the row written and its hearers' places where they were, and they take
//! it as it starts again.

use std::collections::HashMap;
use std::time::Duration;

use journal::{EventQuery, Journal};
use kernel::Event;
use serde_json::Value;

use crate::state::App;

/// The longest a write waits for what hears it.
pub const WITHIN: Duration = Duration::from_secs(10);

/// What is said of a row: added, changed or taken away.
const WRITTEN: &str = "record.created,record.updated,record.removed";

/// Where the journal that keeps `project`'s events is, before a write.
pub(crate) fn mark(app: &App, project: &str) -> u64 {
    app.engine.journal().of(Some(project)).last_seq()
}

/// Once a row of `collection` of `project` was written (the journal was
/// at `before` as it began), waits for it to have been taken, where the
/// table steers or is steered, and puts in `answer` what had not taken
/// it (`untaken`) where something had not within [`WITHIN`].
pub(crate) async fn said(app: &App, (project, collection): (&str, &str), before: u64, answer: &mut Value) {
    let steering = app.studio.collections.steering(project, collection).await.unwrap_or(false);
    if !steering {
        return;
    }
    if let Some(untaken) = taken(app, project, before).await {
        answer["untaken"] = Value::String(untaken);
    }
}

/// Waits until each row of a table that steers or is steered, written in
/// `project` since `before`, has been done with by whatever hears it, and
/// so each such row written meanwhile, until none is left. A sentence of
/// what had not, where the wait ran out.
async fn taken(app: &App, project: &str, before: u64) -> Option<String> {
    let journal = app.engine.journal().of(Some(project));
    let mut over = journal.clock().sleep(WITHIN);
    let mut steering: HashMap<String, bool> = HashMap::new();
    let mut after = before;
    loop {
        let query = EventQuery { after, topics: WRITTEN.into(), limit: 100, ..EventQuery::default() };
        let written = journal.read(query).await.ok()?;
        after = written.last()?.seq;
        for event in written {
            if !steers(app, project, &event, &mut steering).await {
                continue;
            }
            tokio::select! {
                () = app.engine.handled(&journal, &event) => {}
                () = &mut over => return Some(untaken(app, &journal, &event)),
            }
        }
    }
}

/// Whether `event` says a row of `project` was written of a table that
/// steers or is steered, each table asked once.
async fn steers(app: &App, project: &str, event: &Event, asked: &mut HashMap<String, bool>) -> bool {
    let Some(collection) = event.data["collection"].as_str() else { return false };
    if event.data["project"] != project {
        return false;
    }
    if let Some(known) = asked.get(collection) {
        return *known;
    }
    let steering = app.studio.collections.steering(project, collection).await.unwrap_or(false);
    asked.insert(collection.to_string(), steering);
    steering
}

/// What is said of a row whose hearers had not all done with it in time.
fn untaken(app: &App, journal: &Journal, event: &Event) -> String {
    let handling = app.engine.handling(journal, event);
    let who: Vec<String> =
        handling.iter().map(|(instance, handler)| format!("`{handler}` of `{instance}`")).collect();
    let row = event.data["id"].as_str().unwrap_or("the row");
    format!(
        "{row} is written, but {} had not taken it after {} seconds: until it does, what the row steers goes \
         as it did. `{} runs` shows what is under way.",
        if who.is_empty() { "what hears it".to_string() } else { who.join(", ") },
        WITHIN.as_secs(),
        brand::CLI,
    )
}

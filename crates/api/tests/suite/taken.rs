//! A fleet having taken what was written to one of its tables.
//!
//! A change to a row asks the fleet to bring itself to its tables
//! (`std/fleet`'s `changed`), and what does that says `fleet.arranged`
//! with what it was asked for: `row` and the number of the change. Any
//! `fleet.arranged` said after the write is not the fleet having taken
//! it: one asked for by an earlier change may end after the write,
//! having read the tables before it. So a test waits for the one asked
//! for by its own change, or by a later one of the same project, which
//! read the tables after it: of two changes made together, the first
//! may be called off by the second, which then takes both.

use std::sync::Arc;

use kernel::Event;
use serde_json::Value;

use crate::Rig;

/// The fleet's own tables: a change to any of them is heard.
const TABLES: [&str; 4] = ["agents", "stages", "segments", "actions"];

/// Waits for the first change to one of the fleet's tables written
/// since `before` to have been taken by the fleet, and hands what it
/// said of itself then.
pub(crate) async fn by_fleet(rig: &Rig, before: u64) -> Value {
    let written = rig
        .event("record.*", move |event| {
            event.seq > before
                && event.data["collection"].as_str().is_some_and(|table| TABLES.contains(&table))
        })
        .await;
    arranged_for(rig, before, &written).await
}

/// Waits for an arrangement asked for by `written`, or by a later
/// change of its project.
pub(crate) async fn arranged_for(rig: &Rig, before: u64, written: &Arc<Event>) -> Value {
    let number = rig.number(written.seq);
    let project = written.data["project"].clone();
    rig.event("fleet.arranged", move |event| {
        event.seq > before
            && event.data["project"] == project
            && asked(event).is_some_and(|asked| asked >= number)
    })
    .await
    .data
    .clone()
}

/// The number of the change an arrangement was asked for by, where a
/// change asked for it.
pub(crate) fn asked(event: &Event) -> Option<u64> {
    event.data["asked"].as_str()?.strip_prefix("row ")?.parse().ok()
}

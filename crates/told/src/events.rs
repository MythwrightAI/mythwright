//! What is said of a change somebody asked for that they may not make
//! themselves (`crate::asks`): each said in the write that keeps the ask,
//! where its record's project's rows are, and heard by whoever may read
//! records. None carries the cells' values: those are read through the
//! line's part, by whoever may read the record. And what is said of a
//! run that waits for a person to let it go on (`crate::goes`): heard by
//! whoever may read blueprints and what they do.

use blueprint::EventSpec;
use blueprint::ValueType::*;

pub fn events() -> Vec<EventSpec> {
    let ask = |topic: &str, summary: &str| {
        EventSpec::new(topic, summary)
            .field("ask", String, "the ask, by its id")
            .field("id", String, "the record it would change")
            .field("project", String, "the project the record is kept for")
            .field("collection", String, "the table it is a record of")
            .field("cells", List, "the names of the cells it would set")
            .field("asker", String, "who asked, as an actor")
    };
    let all = vec![
        ask(
            "record.asked",
            "Somebody asked for a change of a record they may not make themselves, and whoever could make it was told.",
        )
        .field("why", String, "what they said of why"),
        ask(
            "record.ask.approved",
            "A change somebody asked for was approved and made, as theirs, in the same write as this is said.",
        )
        .field("by", String, "who approved it, as an actor: the key, where it was a key"),
        ask("record.ask.declined", "A change somebody asked for was declined, and nothing was made.")
            .field("by", String, "who declined it, as an actor")
            .field("why", String, "what they said of why; nothing, where they said nothing"),
        ask(
            "record.ask.decided",
            "A change somebody asked for came to something and waits no more: approved, declined, stale (its \
             record was changed since), expired or withdrawn. Said for whatever waits on one, beside the event \
             that says which.",
        )
        .field("by", String, "who decided it, as an actor; `system` for one stale or expired")
        .field("status", String, "what it came to")
        .maybe("why", String, "what whoever declined it said of why, where it was declined"),
    ];
    let go = |topic: &str, summary: &str| {
        EventSpec::new(topic, summary)
            .field("ask", String, "the ask, by its id")
            .field("run", String, "the run that waits")
            .field("step", String, "the step of it that asked")
            .field("instance", String, "what runs")
            .field("blueprint", String, "the blueprint it runs")
            .field("environment", String, "the environment whose rule says who approves; nothing, where none")
            .field("asker", String, "who started the run, as an actor")
            .maybe("project", String, "the project it runs for, where it runs for one")
    };
    let goes = vec![
        go(
            "project.approval.asked",
            "A run waits for a person to let it go on (`approval.ask`), and whoever may decide was told in their inbox.",
        )
        .field("to", List, "whom it was told to, as actors"),
        go(
            "project.approval.decided",
            "A run that waited to be let go on waits no more: somebody let it go on or declined, or it waited \
             its time. The step that asked hears this.",
        )
        .field("status", String, "what it came to: approved, declined or expired")
        .field("by", String, "who decided, as an actor; `system` where it waited its time")
        .maybe("why", String, "what whoever declined said of why, where it was declined"),
    ];
    let mut all = blueprint::events::heard(all, &[("record", "records:read")]);
    // A run is read by whoever may read blueprints and what they do.
    all.extend(blueprint::events::heard(goes, &[("project.approval", "blueprints:read")]));
    all
}

//! A line of a record's history as it is read: what a blueprint's step did
//! is said as the factory's bot is called now, with whose work it was done
//! from after the sentence; anybody else's as it was written.

use kernel::Event;
use kernel::worded::{Rule, as_called, from_what, said, through};
use serde_json::{Value, json};

/// Who a line says did it, what it says, what its event said, what the bot
/// is called now, and who and what it comes to.
type Case<'a> = (&'a str, &'a str, &'a Value, &'a str, (&'a str, &'a str));

#[test]
fn what_a_blueprints_step_did_is_said_as_the_bot_is_called_now_and_anybody_elses_as_it_was_written() {
    let moved = json!({ "through": "std/lane-round", "from": "ready", "to": "doing" });
    let planned = json!({ "through": "std/plan", "wrote": "agent:shop-architect-1" });
    let cases: &[Case] = &[
        // Written as the bot is called now: as it was.
        (
            "conductor",
            "conductor moved it from ready to doing",
            &moved,
            "conductor",
            ("conductor", "conductor moved it from ready to doing"),
        ),
        // The bot called something else since: its new name.
        (
            "conductor",
            "conductor moved it from ready to doing",
            &moved,
            "foreman",
            ("foreman", "foreman moved it from ready to doing"),
        ),
        // A line from before, which said the blueprint as who did it.
        (
            "std/lane-round",
            "std/lane-round moved it from ready to doing",
            &moved,
            "conductor",
            ("conductor", "conductor moved it from ready to doing"),
        ),
        // Whose work it was done from, after the sentence, and once.
        (
            "conductor",
            "conductor added it to draft from what shop-architect-1 wrote",
            &planned,
            "foreman",
            ("foreman", "foreman added it to draft from what shop-architect-1 wrote"),
        ),
        (
            "conductor",
            "conductor added it to draft",
            &planned,
            "conductor",
            ("conductor", "conductor added it to draft from what shop-architect-1 wrote"),
        ),
        // From before, where it was said with the blueprint.
        (
            "std/plan (from what shop-architect-1 wrote)",
            "std/plan (from what shop-architect-1 wrote) added it to draft",
            &planned,
            "conductor",
            ("conductor", "conductor added it to draft from what shop-architect-1 wrote"),
        ),
        // A person's is theirs, whatever the bot is called.
        (
            "Ada",
            "Ada moved it from ready to doing",
            &json!({ "from": "ready" }),
            "foreman",
            ("Ada", "Ada moved it from ready to doing"),
        ),
        ("Ada", "Ada moved it", &json!({ "through": "" }), "foreman", ("Ada", "Ada moved it")),
    ];
    for (who, said, data, bot, (was, says)) in cases {
        assert_eq!(
            as_called(who, said, data, bot),
            (was.to_string(), says.to_string()),
            "{who}: {said} as {bot}"
        );
    }
    assert_eq!((through(&moved), through(&json!({ "through": "" }))), (Some("std/lane-round"), None));
    assert_eq!(
        from_what(&json!({ "through": "std/plan", "wrote": "user:ada" })),
        "",
        "only an agent is said to have written it"
    );
}

#[test]
fn the_usual_words_leave_out_which_blueprint_did_it_and_from_whose_work_which_are_said_after_the_line() {
    let rule = Rule { topic: "record.created".into(), says: None };
    let data =
        json!({ "id": "shop-notes-1", "collection": "notes", "through": "std/plan", "wrote": "agent:a" });
    let event = Event {
        seq: 1,
        ts: 0,
        topic: "record.created".into(),
        source: kernel::Source::actor("user:ada"),
        data,
    };
    assert_eq!(said(&rule, &event, "conductor"), "conductor: created (collection notes)");
}

/// What a sentence of `card.left` comes to for what its event said.
fn left(data: Value) -> String {
    let rule = Rule {
        topic: "card.left".into(),
        says: Some("it was left for a person[, having been sent back {rounds} times][: {why}]".into()),
    };
    let event =
        Event { seq: 1, ts: 0, topic: "card.left".into(), source: kernel::Source::actor("user:ada"), data };
    said(&rule, &event, "conductor")
}

#[test]
fn each_part_in_square_brackets_is_said_only_where_every_name_in_it_says_something_each_decided_by_itself() {
    let why = "none of its 2 contenders' work could be kept (otter unkept, slate unkept)";
    let cases = [
        // No rounds and a why: the first part left out, the second said.
        (json!({ "why": why, "as": "" }), format!("it was left for a person: {why}")),
        (json!({ "why": why, "rounds": "" }), format!("it was left for a person: {why}")),
        (json!({ "why": why, "rounds": null }), format!("it was left for a person: {why}")),
        // Rounds and no why, or a why that says nothing.
        (json!({ "rounds": 2 }), "it was left for a person, having been sent back 2 times".to_string()),
        (
            json!({ "rounds": 2, "why": "" }),
            "it was left for a person, having been sent back 2 times".to_string(),
        ),
        (
            json!({ "rounds": 2, "why": "Wrong." }),
            "it was left for a person, having been sent back 2 times: Wrong.".to_string(),
        ),
        // Neither: the sentence alone.
        (json!({}), "it was left for a person".to_string()),
        (json!({ "rounds": "", "why": "" }), "it was left for a person".to_string()),
    ];
    for (data, says) in cases {
        assert_eq!(left(data.clone()), says, "{data}");
    }
    // A part of two names is said only where both say something, and with
    // a bracket in what is said, only the sentence's brackets are parts.
    let rule = Rule {
        topic: "x.y".into(),
        says: Some("{actor} did it[ from {from} to {to}][, saying {why}]".into()),
    };
    let event = |data: Value| Event {
        seq: 1,
        ts: 0,
        topic: "x.y".into(),
        source: kernel::Source::actor("user:ada"),
        data,
    };
    assert_eq!(
        said(&rule, &event(json!({ "from": "a", "why": "[x] y" })), "Ada"),
        "Ada did it, saying [x] y"
    );
    assert_eq!(said(&rule, &event(json!({ "from": "a", "to": "b" })), "Ada"), "Ada did it from a to b");
}

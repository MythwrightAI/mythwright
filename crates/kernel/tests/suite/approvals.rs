//! A change somebody asks for that they may not make themselves: which
//! cells it changes, what it takes, what an asker lacks of that and what
//! asking comes to (`kernel::approvals`).

use kernel::approvals::{
    Cell, Comes, Kind, Need, Of, changed, comes, lacking, says_nothing, scopes_of, told,
};
use serde_json::{Map, Value, json};

fn cells(value: Value) -> Map<String, Value> {
    value.as_object().unwrap().clone()
}

fn held(scopes: &[&str]) -> Vec<String> {
    scopes.iter().map(|scope| scope.to_string()).collect()
}

/// What changing a worker's row of a fleet's agents takes: writing
/// records, the table's own word, its `permission` set to `full`, and the
/// scopes its `holds` would give.
fn worker_needs() -> Vec<Need> {
    vec![
        Need::new(Of::Records, &["records:write"]),
        Need::new(Of::Table, &[]),
        Need::new(Of::Cell("permission".into()), &["shell"]),
        Need::new(Of::Cell("holds".into()), &["board", "shell", "secrets:use"]),
    ]
}

#[test]
fn a_kind_is_read_by_its_word_and_one_there_is_not_says_which_there_are() {
    assert_eq!(Kind::read("cells"), Ok(Kind::Cells));
    assert_eq!(Kind::Cells.as_str(), "cells");
    assert_eq!(Kind::read("go"), Ok(Kind::Go));
    assert_eq!(
        Kind::read("moves"),
        Err("there is no kind of change called `moves`: it is one of cells, go".into())
    );
}

#[test]
fn a_cell_says_nothing_where_it_holds_nothing_or_empty_text() {
    for (value, nothing) in [
        (Value::Null, true),
        (json!(""), true),
        (json!(" "), false),
        (json!(0), false),
        (json!(false), false),
        (json!([]), false),
    ] {
        assert_eq!(says_nothing(&value), nothing, "{value}");
    }
}

#[test]
fn what_a_change_changes_is_each_cell_asked_that_holds_something_else_by_its_name() {
    let kept = cells(json!({ "count": 1, "permission": "edit", "told": "", "role": "worker" }));
    let asked = cells(json!({
        "permission": "full",
        "count": 1,
        "told": null,
        "notes": "",
        "holds": ["board"],
        "role": "Worker",
    }));
    assert_eq!(
        changed(&kept, &asked),
        [
            Cell { name: "holds".into(), before: Value::Null, after: json!(["board"]) },
            Cell { name: "permission".into(), before: json!("edit"), after: json!("full") },
            Cell { name: "role".into(), before: json!("worker"), after: json!("Worker") },
        ],
        "the same value, and nothing for nothing (null, empty text, a cell not there), is no change; \
         and a cell's case is its value"
    );
    assert_eq!(changed(&kept, &Map::new()), [], "nothing asked, nothing changed");
}

#[test]
fn what_an_asker_lacks_is_each_thing_with_the_scopes_of_it_they_do_not_hold() {
    let needs = worker_needs();
    assert_eq!(
        lacking(&needs, &held(&["records", "board"])),
        [
            Need::new(Of::Cell("permission".into()), &["shell"]),
            Need::new(Of::Cell("holds".into()), &["shell", "secrets:use"]),
        ]
    );
    assert_eq!(
        lacking(&needs, &held(&["records", "board", "shell", "secrets"])),
        [],
        "an area holds its verbs"
    );
    assert_eq!(lacking(&needs, &held(&["*"])), [], "everything holds everything");
    assert_eq!(
        lacking(&needs, &held(&[]))[0],
        Need::new(Of::Records, &["records:write"]),
        "a thing that takes nothing (the table's own word, here) is never lacking"
    );
    assert_eq!(lacking(&needs, &held(&[])).len(), 3);
    assert_eq!(
        lacking(&[Need::new(Of::Named("card".into()), &["member:ch_1"])], &held(&["*"])).len(),
        1,
        "being a member of something is held by nothing, everything included"
    );
}

#[test]
fn every_scope_a_change_takes_is_said_once_in_the_order_first_needed() {
    assert_eq!(scopes_of(&worker_needs()), ["records:write", "shell", "board", "secrets:use"]);
    assert!(scopes_of(&[]).is_empty());
}

#[test]
fn asking_makes_what_the_asker_may_make_keeps_what_they_may_not_and_does_nothing_for_nothing() {
    let cells = changed(&cells(json!({ "permission": "edit" })), &cells(json!({ "permission": "full" })));
    let needs = worker_needs();
    assert_eq!(comes(&cells, &needs, &held(&["*"])), Comes::Make);
    assert_eq!(
        comes(&cells, &needs, &held(&["records", "board"])),
        Comes::Ask(lacking(&needs, &held(&["records", "board"]))),
    );
    assert_eq!(comes(&[], &needs, &held(&[])), Comes::Nothing, "nothing is changed, so nothing is asked");
}

#[test]
fn what_a_thing_is_is_said_in_words() {
    for (of, said) in [
        (Of::Records, "writing records"),
        (Of::Table, "changing this table's rows"),
        (Of::Cell("holds".into()), "setting `holds`"),
        (Of::Steered("agents".into()), "writing the rows of `agents`, which this table steers"),
        (Of::Named("card".into()), "reading what `card` names"),
        (Of::Standing("wall".into()), "changing anything of a row whose `wall` says what it does"),
    ] {
        assert_eq!(of.said(), said);
    }
}

#[test]
fn whoever_could_make_it_is_told_who_asks_what_and_which_cells() {
    let cells = changed(
        &cells(json!({ "count": 1, "permission": "edit" })),
        &cells(json!({ "count": 2, "permission": "full" })),
    );
    assert_eq!(
        told("shop-worker-1", "shop-agents-2", "the agents of Shop", &cells),
        "shop-worker-1 asks to change shop-agents-2 (the agents of Shop): count, permission"
    );
}

#[test]
fn an_ask_waiting_stands_expired_once_it_has_lasted_its_time_and_stale_once_its_record_was_changed_and_decided_as_it_was()
 {
    use kernel::approvals::{LASTS_MS, Status, stands};
    let at = 1_000;
    assert_eq!(stands(Status::Waiting, at, (5, 5), at + 1), Status::Waiting);
    assert_eq!(
        stands(Status::Waiting, at, (5, 6), at + 1),
        Status::Stale,
        "its record was changed at all since"
    );
    assert_eq!(stands(Status::Waiting, at, (5, 5), at + LASTS_MS), Status::Expired);
    assert_eq!(stands(Status::Waiting, at, (5, 6), at + LASTS_MS), Status::Expired, "its time first");
    for decided in [Status::Approved, Status::Declined, Status::Withdrawn] {
        assert_eq!(stands(decided, at, (5, 6), at + LASTS_MS), decided, "decided, it is as it was decided");
    }
    for status in Status::ALL {
        assert_eq!(Status::read(status.as_str()), *status);
    }
    assert_eq!(Status::read("no such word"), Status::Withdrawn, "a word not known is taken for decided");
}

#[test]
fn whoever_asked_is_told_what_came_of_it_in_a_sentence() {
    use kernel::approvals::{Status, decided};
    assert_eq!(
        decided(Status::Approved, "shop-workers-1", "Ada", ""),
        "Ada approved your change of shop-workers-1, and it is made."
    );
    assert_eq!(
        decided(Status::Declined, "shop-workers-1", "Ada", " not today "),
        "Ada declined your change of shop-workers-1: not today."
    );
    assert_eq!(
        decided(Status::Declined, "shop-workers-1", "Ada", ""),
        "Ada declined your change of shop-workers-1."
    );
    assert!(decided(Status::Stale, "r", "", "").contains("changed since you asked"));
    assert!(decided(Status::Expired, "r", "", "").contains("seven days"));
}

//! Tickets that are things there already (`deciding::gathered`, asked by
//! `std/feature` and its round as `tickets.named`, `tickets.fit` and
//! `tickets.placed`): tables of what is handed and what it comes to, with
//! no word of `fit` left unreached and a case for each thing it refuses.

use deciding::gathered::{Cells, Fit, Thing, fit, named, placed, things};
use serde_json::{Value, json};

fn cells() -> Cells {
    Cells { over: "done".into(), begins: "backlog".into(), ..Cells::default() }
}

/// A board with five cards on it: two that wait on nothing, one that
/// waits on a card outside any plan, one that is done, and one that is
/// part of something else.
fn board() -> Vec<Thing> {
    let rows = json!([
        { "id": "shop-1", "title": "The till", "body": "Takes money.", "status": "backlog", "after": [], "labels": ["next"] },
        { "id": "shop-2", "title": "The shelves", "status": "ready", "after": [], "labels": ["next", "wood"] },
        { "id": "shop-3", "title": "The sign", "status": "backlog", "after": ["shop-9"], "labels": [] },
        { "id": "shop-4", "title": "The floor", "status": "done", "after": [], "labels": ["next"] },
        { "id": "shop-5", "title": "The door", "status": "doing", "after": [], "part_of": "shop-features-2" },
        { "id": "shop-9", "title": "The lease", "status": "backlog", "after": [] },
        { "title": "A row with no id is no row" },
    ]);
    things(rows.as_array().unwrap(), &cells())
}

fn ids(of: &[&str]) -> Vec<String> {
    of.iter().map(|id| id.to_string()).collect()
}

#[test]
fn the_rows_a_plan_is_to_put_in_order_are_said_for_whoever_plans() {
    let rows = board();
    assert_eq!(rows.len(), 6, "a row with no id is left out");
    // Those named, in the order they were named, each once; one that is
    // no row is said to be missing.
    let asked = named(&rows, &ids(&["shop-3", "shop-1", "shop-3", "shop-77"]), "", &cells());
    assert_eq!(
        (asked.ids.clone(), asked.missing.clone(), asked.fix),
        (ids(&["shop-3", "shop-1"]), ids(&["shop-77"]), false)
    );
    assert_eq!(
        asked.shown,
        "shop-3, in backlog: The sign\nIt waits on shop-9.\n\nshop-1, in backlog: The till\nTakes money.\n\n\
         shop-77 was named, and there is no such thing."
    );
    // What it is part of is said too.
    let part = named(&rows, &ids(&["shop-5"]), "", &cells());
    assert_eq!(part.shown, "shop-5, in doing: The door\nIt is part of shop-features-2.");
    // With a label, every row that carries it and is not done, after
    // those that were named; and that is then to be written down.
    let labelled = named(&rows, &ids(&["shop-2"]), "next", &cells());
    assert_eq!((labelled.ids, labelled.fix), (ids(&["shop-2", "shop-1"]), true));
    // A label that brings nothing more than was named fixes nothing.
    assert!(!named(&rows, &ids(&["shop-1", "shop-2"]), "next", &cells()).fix);
    // Nothing named and no label: nothing, and no words.
    let nothing = named(&rows, &[], "", &cells());
    assert_eq!((nothing.ids.len(), nothing.shown.as_str(), nothing.fix), (0, "", false));
}

fn tickets(written: Value) -> Vec<Value> {
    written.as_array().unwrap().clone()
}

#[test]
fn a_plan_fits_what_is_there_or_is_whole_or_is_refused_in_a_sentence_for_each_thing() {
    let rows = board();
    let of = "shop-features-1";
    // What reading the plan came to and why not; its tickets; the rows it
    // may take up; what it comes to; and what is said of why.
    type Case = (&'static str, Value, &'static [&'static str], Fit, &'static str);
    let cases: Vec<Case> = vec![
        // Nothing gathered: tickets that are all new fit, as they always did.
        ("read", json!([{ "title": "A" }, { "title": "B", "after": [1] }]), &[], Fit::Fits, ""),
        // No tickets and nothing gathered: one piece of work.
        ("none", json!([]), &[], Fit::Whole, ""),
        ("read", json!([]), &[], Fit::Whole, ""),
        // A block that could not be read is whole where nothing is
        // gathered, as it was, and refused where something is.
        ("unread", json!([]), &[], Fit::Whole, "this is not JSON"),
        (
            "unread",
            json!([]),
            &["shop-1"],
            Fit::Unfit,
            "The tickets its plan ends with could not be read (this is not JSON).",
        ),
        // Rows there already and a new one, in order.
        (
            "read",
            json!([{ "card": "shop-1" }, { "card": "shop-2" }, { "title": "The opening", "after": [1, 2, "shop-9"] }]),
            &["shop-1", "shop-2"],
            Fit::Fits,
            "",
        ),
        // Gathered and named by no ticket: taken as it stands.
        ("read", json!([{ "card": "shop-1" }]), &["shop-1", "shop-3"], Fit::Fits, ""),
        ("none", json!([]), &["shop-1", "shop-2"], Fit::Fits, ""),
        // One that is done is taken, and nothing waits on it.
        (
            "read",
            json!([{ "card": "shop-4" }, { "card": "shop-1", "after": [1] }]),
            &["shop-1", "shop-4"],
            Fit::Fits,
            "",
        ),
        // Everything it gathers is gone, and it lists nothing.
        (
            "none",
            json!([]),
            &["shop-77", "shop-78"],
            Fit::Unfit,
            "It lists no tickets, and of what it gathers (shop-77 and shop-78) nothing is there any more.",
        ),
        // Neither a row nor a title.
        (
            "read",
            json!([{ "body": "Something." }]),
            &[],
            Fit::Unfit,
            "Ticket 1 names nothing there already and has no `title`: it is one or the other.",
        ),
        // A row it was not asked to put in order, with and without any that it was.
        (
            "read",
            json!([{ "card": "shop-2" }]),
            &["shop-1"],
            Fit::Unfit,
            "Ticket 1 is shop-2, which is not one of those this was asked to put in order (shop-1).",
        ),
        (
            "read",
            json!([{ "card": "shop-2" }]),
            &[],
            Fit::Unfit,
            "Ticket 1 is shop-2, which is not one of those this was asked to put in order.",
        ),
        // One that is gone since it was asked for.
        (
            "read",
            json!([{ "card": "shop-77" }]),
            &["shop-77"],
            Fit::Unfit,
            "Ticket 1 is shop-77, and there is no such thing any more.",
        ),
        // Named twice.
        (
            "read",
            json!([{ "card": "shop-1" }, { "card": "shop-1" }]),
            &["shop-1"],
            Fit::Unfit,
            "shop-1 is named by two tickets, and is one thing.",
        ),
        // `card` that is no id.
        (
            "read",
            json!([{ "card": 3 }]),
            &["shop-1"],
            Fit::Unfit,
            "`card` of ticket 1 is the id of something there already (\"shop-3\"); it is 3.",
        ),
        // Waiting on a place no ticket is at, on a row that is not there,
        // and on something that is neither.
        (
            "read",
            json!([{ "title": "A", "after": [4] }]),
            &[],
            Fit::Unfit,
            "Ticket 1 waits on ticket 4, and the tickets are 1 to 1.",
        ),
        (
            "read",
            json!([{ "title": "A", "after": ["shop-77"] }]),
            &[],
            Fit::Unfit,
            "Ticket 1 waits on shop-77, and there is no such thing.",
        ),
        (
            "read",
            json!([{ "title": "A", "after": [true] }]),
            &[],
            Fit::Unfit,
            "`after` of ticket 1 lists what it waits on, each a place in the list (1) or the id of something there already (\"shop-3\"); one of them is true.",
        ),
        (
            "read",
            json!([{ "title": "A", "after": "shop-1" }]),
            &[],
            Fit::Unfit,
            "`after` of ticket 1 is the list of what it waits on ([1, \"shop-3\"]); it is \"shop-1\".",
        ),
        // Part of something else already.
        (
            "read",
            json!([{ "card": "shop-5" }]),
            &["shop-5"],
            Fit::Unfit,
            "shop-5 is part of shop-features-2 already: leave it out of this, or take it out of that first.",
        ),
        (
            "none",
            json!([]),
            &["shop-5"],
            Fit::Unfit,
            "shop-5 is part of shop-features-2 already: leave it out of this, or take it out of that first.",
        ),
        // Waiting on itself: at once, between two tickets, and by a road
        // through a row outside the plan (shop-3 waits on shop-9 already).
        (
            "read",
            json!([{ "title": "A", "after": [1] }]),
            &[],
            Fit::Unfit,
            "Ticket 1 would wait on itself: ticket 1 waits on ticket 1.",
        ),
        (
            "read",
            json!([{ "card": "shop-1", "after": [2] }, { "title": "B", "after": [1] }]),
            &["shop-1"],
            Fit::Unfit,
            "Shop-1 would wait on itself: shop-1 waits on ticket 2, which waits on shop-1.",
        ),
        (
            "read",
            json!([{ "card": "shop-3" }, { "title": "The lease signed", "after": [1] }, { "card": "shop-9", "after": [2] }]),
            &["shop-3", "shop-9"],
            Fit::Unfit,
            "Shop-3 would wait on itself: shop-3 waits on shop-9, which waits on ticket 2, which waits on shop-3.",
        ),
        // Several things wrong are each said.
        (
            "read",
            json!([{ "card": "shop-2" }, { "body": "x" }]),
            &["shop-1"],
            Fit::Unfit,
            "Ticket 1 is shop-2, which is not one of those this was asked to put in order (shop-1). Ticket 2 names nothing there already and has no `title`: it is one or the other.",
        ),
    ];
    let mut reached = Vec::new();
    for (read, written, may, came, why) in cases {
        let unread = if read == "unread" { "this is not JSON" } else { "" };
        let answer = fit(read, unread, &tickets(written.clone()), &ids(may), &rows, of, &cells());
        assert_eq!(answer.came, came, "{written} over {may:?}: {}", answer.why);
        assert_eq!(answer.why, why, "{written} over {may:?}");
        if came != Fit::Fits {
            assert!(answer.new.is_empty(), "{written}: nothing is to be made of a plan that does not fit");
        }
        reached.push(came);
    }
    for word in Fit::ALL {
        assert!(reached.contains(word), "no case comes to `{}`", word.word());
    }
    // The tickets that are new are handed back to be made, in order, each
    // with its title and what is wanted.
    let mixed = tickets(
        json!([{ "card": "shop-1" }, { "title": " The opening ", "body": "Open." }, { "title": "The party" }]),
    );
    let answer = fit("read", "", &mixed, &ids(&["shop-1"]), &rows, of, &cells());
    assert_eq!(
        answer.new,
        tickets(json!([{ "title": "The opening", "body": "Open." }, { "title": "The party", "body": "" }]))
    );
}

fn made(of: &[&str]) -> Vec<Value> {
    of.iter().map(|id| json!({ "id": id })).collect()
}

#[test]
fn every_thing_of_a_plan_says_what_it_waits_on_and_is_part_of_once_its_new_tickets_are_rows() {
    let rows = board();
    let of = "shop-features-1";
    // Two rows there already side by side, a new one after both and
    // after a row outside the plan; and one gathered that no ticket names.
    let plan = tickets(json!([
        { "card": "shop-1" },
        { "card": "shop-2", "after": [] },
        { "title": "The opening", "after": [1, 2, "shop-9"] },
    ]));
    let placed_ =
        placed(&plan, &made(&["shop-10"]), &ids(&["shop-1", "shop-2", "shop-3"]), &rows, of, &cells())
            .unwrap();
    assert_eq!(
        placed_.links,
        tickets(json!([
            { "id": "shop-1", "after": [], "part_of": of },
            { "id": "shop-2", "after": [], "part_of": of },
            { "id": "shop-10", "after": ["shop-1", "shop-2", "shop-9"], "part_of": of },
            { "id": "shop-3", "after": ["shop-9"], "part_of": of },
        ]))
    );
    assert_eq!(placed_.ids, ids(&["shop-1", "shop-2", "shop-10", "shop-3"]));
    assert_eq!((placed_.added, placed_.taken), (1, 3));
    // Made ready: the new one, and those there already that have not been
    // begun. One that is ready or in hand is left where it is.
    assert_eq!(placed_.ready, ids(&["shop-1", "shop-10", "shop-3"]));
    // Nothing changed for a row left as it was.
    assert_eq!(placed_.told, "");

    // A row that says what it waits on waits on exactly that and on what
    // it already waited on outside the plan; what it waited on inside the
    // plan is the plan's to say.
    let plan = tickets(
        json!([{ "card": "shop-3", "after": [2] }, { "card": "shop-1" }, { "card": "shop-9", "after": [] }]),
    );
    let again = placed(&plan, &[], &ids(&["shop-1", "shop-3", "shop-9"]), &rows, of, &cells()).unwrap();
    assert_eq!(again.links[0], json!({ "id": "shop-3", "after": ["shop-1"], "part_of": of }));
    assert_eq!(again.told, "shop-3 now waits on shop-1.");
    let plan = tickets(json!([{ "card": "shop-3", "after": [2] }, { "card": "shop-1" }]));
    let kept = placed(&plan, &[], &ids(&["shop-1", "shop-3"]), &rows, of, &cells()).unwrap();
    assert_eq!(kept.links[0], json!({ "id": "shop-3", "after": ["shop-1", "shop-9"], "part_of": of }));
    assert_eq!(kept.told, "shop-3 now waits on shop-1 and shop-9.");
    // Said to wait on nothing, it does.
    let plan = tickets(json!([{ "card": "shop-3", "after": [] }, { "card": "shop-9" }]));
    let freed = placed(&plan, &[], &ids(&["shop-3", "shop-9"]), &rows, of, &cells()).unwrap();
    assert_eq!(freed.links[0]["after"], json!([]));
    assert_eq!(freed.told, "shop-3 now waits on nothing.");

    // Nothing waits on what is done; and a row in hand that is given
    // something to wait on is said not to be held back.
    let rows_in_hand = {
        let mut rows = board();
        rows.iter_mut().find(|row| row.id == "shop-5").unwrap().within.clear();
        rows
    };
    let plan =
        tickets(json!([{ "card": "shop-4" }, { "card": "shop-5", "after": [1, 3] }, { "card": "shop-1" }]));
    let hand =
        placed(&plan, &[], &ids(&["shop-1", "shop-4", "shop-5"]), &rows_in_hand, of, &cells()).unwrap();
    assert_eq!(hand.links[1], json!({ "id": "shop-5", "after": ["shop-1"], "part_of": of }));
    assert_eq!(hand.told, "shop-5 now waits on shop-1. It is in doing already, and is not held back.");
    assert_eq!(hand.ready, ids(&["shop-1"]), "one that is done, and one in hand, are left where they are");

    // The cell that keeps what a thing is part of is whoever asks' to name.
    let named_otherwise = Cells { within: "piece_of".into(), ..cells() };
    let other =
        placed(&tickets(json!([{ "title": "A" }])), &made(&["x-1"]), &[], &[], of, &named_otherwise).unwrap();
    assert_eq!(other.links, tickets(json!([{ "id": "x-1", "after": [], "piece_of": of }])));
}

#[test]
fn a_plan_taken_up_a_second_time_makes_nothing_twice() {
    let of = "shop-features-1";
    // Cut short the first time: one of its two new tickets was made, and
    // one of the rows it names was already said to be part of it.
    let mut rows = board();
    rows.iter_mut().find(|row| row.id == "shop-1").unwrap().within = of.into();
    rows.push(Thing {
        id: "shop-10".into(),
        title: "The opening".into(),
        state: "backlog".into(),
        within: of.into(),
        ..Thing::default()
    });
    let plan = tickets(json!([
        { "card": "shop-1" },
        { "card": "shop-2", "after": [1] },
        { "title": "The opening", "after": [1, 2] },
        { "title": "The party", "after": [3] },
        { "title": "The party", "after": [3] },
    ]));
    let may = ids(&["shop-1", "shop-2"]);
    // The ticket whose row is there is not new; the two called the same
    // are two, and neither was made.
    let again = fit("read", "", &plan, &may, &rows, of, &cells());
    assert_eq!(again.came, Fit::Fits, "{}", again.why);
    assert_eq!(
        again.new,
        tickets(json!([{ "title": "The party", "body": "" }, { "title": "The party", "body": "" }]))
    );
    let placed_ = placed(&plan, &made(&["shop-11", "shop-12"]), &may, &rows, of, &cells()).unwrap();
    assert_eq!(placed_.ids, ids(&["shop-1", "shop-2", "shop-10", "shop-11", "shop-12"]));
    assert_eq!(placed_.links[2], json!({ "id": "shop-10", "after": ["shop-1", "shop-2"], "part_of": of }));
    assert_eq!(placed_.links[3]["after"], json!(["shop-10"]));
    // One of the two called the same was made before the stop: it is the
    // first of them, and the other is still to be made.
    rows.push(Thing {
        id: "shop-11".into(),
        title: "The party".into(),
        state: "backlog".into(),
        within: of.into(),
        ..Thing::default()
    });
    let once = fit("read", "", &plan, &may, &rows, of, &cells());
    assert_eq!(once.new.len(), 1, "{:?}", once.new);
    // Everything made: nothing is new, and it comes to the same.
    rows.push(Thing {
        id: "shop-12".into(),
        title: "The party".into(),
        state: "backlog".into(),
        within: of.into(),
        ..Thing::default()
    });
    assert!(fit("read", "", &plan, &may, &rows, of, &cells()).new.is_empty());
    let all = placed(&plan, &[], &may, &rows, of, &cells()).unwrap();
    assert_eq!(all.ids, placed_.ids);
    // A plan that was one piece of work, whose one card was made: it is
    // not whole a second time, and its card is taken as it stands.
    let whole = vec![Thing {
        id: "shop-1".into(),
        title: "A cart".into(),
        state: "backlog".into(),
        within: of.into(),
        ..Thing::default()
    }];
    let again = fit("none", "", &[], &[], &whole, of, &cells());
    assert_eq!((again.came, again.new.len()), (Fit::Fits, 0));
    assert_eq!(placed(&[], &[], &[], &whole, of, &cells()).unwrap().ids, ids(&["shop-1"]));
    // A row a ticket names outright is that ticket's, whatever it is called.
    let named = tickets(json!([{ "title": "The till" }, { "card": "shop-1" }]));
    let rows = {
        let mut rows = board();
        rows.iter_mut().find(|row| row.id == "shop-1").unwrap().within = of.into();
        rows
    };
    assert_eq!(fit("read", "", &named, &ids(&["shop-1"]), &rows, of, &cells()).new.len(), 1);
}

#[test]
fn what_was_made_that_does_not_fit_the_tickets_is_refused_in_a_sentence() {
    let rows = board();
    let of = "shop-features-1";
    let plan = tickets(json!([{ "card": "shop-1" }, { "title": "A" }, { "title": "B" }]));
    let may = ids(&["shop-1"]);
    assert_eq!(
        placed(&plan, &made(&["shop-10"]), &may, &rows, of, &cells()).unwrap_err(),
        "2 of the tickets are new and 1 things were made for them: one each, in the same order"
    );
    assert_eq!(
        placed(&plan, &[json!({ "id": "shop-10" }), json!({ "title": "B" })], &may, &rows, of, &cells())
            .unwrap_err(),
        "what ticket 3 came to has no `id`: {\"title\":\"B\"}"
    );
    // A plan that does not fit is not placed.
    let unfit = tickets(json!([{ "card": "shop-2" }]));
    assert!(placed(&unfit, &[], &may, &rows, of, &cells()).unwrap_err().starts_with("Ticket 1 is shop-2"));
}

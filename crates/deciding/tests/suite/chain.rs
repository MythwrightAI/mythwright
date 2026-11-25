//! Who is next for a card that waits on somebody (`deciding::chain`,
//! asked by `std/chain` as `chain.next` and `chain.read`): a table of the
//! rungs, what said there is a wait, and what that comes to.

use std::collections::BTreeSet;

use deciding::chain::{Asked, Came, KINDS, next, read};
use serde_json::{Map, Value, json};

fn said(carried: Value) -> Map<String, Value> {
    carried.as_object().cloned().unwrap_or_default()
}

/// A card in review that waits for a person to review it, with these
/// rows, having been moved four times.
fn waits(rows: Value) -> Asked {
    Asked {
        rows: rows.as_array().unwrap().clone(),
        topic: "card.waits".into(),
        said: said(
            json!({ "card": "shop-4", "kind": "review", "why": "shop-4 waits for a person to review it." }),
        ),
        card: "shop-4".into(),
        column: "review".into(),
        labels: vec!["ui".into()],
        moves: 4,
        kept: vec![],
        due: None,
        project: "shop".into(),
        worked: vec!["shop-worker-1".into()],
    }
}

#[test]
fn a_wait_is_told_to_each_rung_in_turn_while_it_stands_and_to_nobody_where_no_rung_is_for_it() {
    let two = json!([
        { "title": "owner", "waits": ["review"], "step": 2, "who": "applier" },
        { "title": "lead", "waits": ["review"], "step": 1, "who": "@grace", "after": "4h" },
    ]);
    // Begun: the lowest step first, with the time until the next.
    let first = next(&waits(two.clone())).unwrap();
    assert_eq!(first.came, Some(Came::Acts));
    assert_eq!((first.rung.who.as_str(), first.kind.as_str()), ("@grace", "review"));
    assert_eq!(first.says, "shop-4 waits for a person to review it.");
    assert_eq!(first.keep, vec![json!("review"), json!(4), json!(1)]);
    assert_eq!((first.next, first.after.as_str()), (1, "4h"));
    // Said again of the same coming: begun already.
    let again = next(&Asked { kept: first.keep.clone(), ..waits(two.clone()) }).unwrap();
    assert_eq!(again.came, Some(Came::Nothing));
    // Its time come, the next rung, which says no time: none after it.
    // What the card's row keeps is kept as text, as a list of a row is.
    let as_kept = vec![json!("review"), json!("4"), json!("1")];
    let due = Asked { kept: as_kept, due: Some(1), ..waits(two.clone()) };
    let second = next(&due).unwrap();
    assert_eq!((second.came, second.rung.who.as_str()), (Some(Came::Acts), "applier"));
    assert_eq!(second.keep, vec![json!("review"), json!(4), json!(2)]);
    assert_eq!((second.next, second.after.as_str()), (0, ""));
    // A time come for a card moved since, for a rung it is past, or for
    // another kind of wait: nothing.
    for (what, kept, due) in [
        ("moved since", json!(["review", 5, 1]), 1),
        ("past it", json!(["review", 4, 2]), 1),
        ("another wait", json!(["left", 4, 1]), 1),
    ] {
        let asked = Asked { kept: kept.as_array().unwrap().clone(), due: Some(due), ..waits(two.clone()) };
        assert_eq!(next(&asked).unwrap().came, Some(Came::Nothing), "{what}");
    }
    // A wait of a new coming is begun again from the first rung.
    let anew =
        next(&Asked { kept: vec![json!("review"), json!(2), json!(2)], ..waits(two.clone()) }).unwrap();
    assert_eq!((anew.came, anew.rung.who.as_str()), (Some(Came::Acts), "@grace"));
    // No rung for it: nothing, and nobody is told by the chain.
    for (what, rows) in [
        ("no row", json!([])),
        ("a row that names nobody", json!([{ "title": "nobody", "who": "" }])),
        ("a row of another kind", json!([{ "title": "lead", "waits": ["asked"], "who": "@grace" }])),
        ("a row of another column", json!([{ "title": "lead", "column": "ready", "who": "@grace" }])),
        ("a row of another label", json!([{ "title": "lead", "labelled": ["docs"], "who": "@grace" }])),
    ] {
        assert_eq!(next(&waits(rows)).unwrap().came, Some(Came::Nothing), "{what}");
    }
    // A row that says no kind is for every kind but `idle`, and one held
    // to the card's column and label is for it.
    let any = json!([{ "title": "lead", "column": "review", "labelled": "ui, docs", "who": "@grace" }]);
    assert_eq!(next(&waits(any)).unwrap().came, Some(Came::Acts));
}

#[test]
fn which_kind_of_wait_it_is_is_said_by_what_said_there_is_one() {
    let every = json!([{ "title": "all", "who": "applier" }, { "title": "idle", "waits": ["idle"], "column": "parked", "who": "@nell" }]);
    let cases = [
        (
            "card.left",
            json!({ "card": "shop-4", "why": "nobody was there" }),
            "left",
            "shop-4 is left for a person: nobody was there",
        ),
        (
            "card.left",
            json!({ "card": "shop-4", "as": "blocked", "why": "I was not let run it" }),
            "blocked",
            "shop-4: its agent could not do what it needed, and it is left for a person: I was not let run it",
        ),
        (
            "card.asked",
            json!({ "card": "shop-4", "asker": "shop-worker-1", "question": "Which?" }),
            "asked",
            "shop-4: shop-worker-1 asks: Which?",
        ),
        (
            "card.waits",
            json!({ "card": "shop-4", "kind": "review" }),
            "review",
            "shop-4 waits for a person to review it.",
        ),
        (
            "card.planned",
            json!({ "card": "shop-4", "agent": "shop-architect-1" }),
            "plan",
            "shop-4: shop-architect-1 has said how it would be done, and the plan waits. Move it on to accept it, or say on it what should be otherwise.",
        ),
        (
            "board.item.held",
            json!({ "card": "shop-4", "says": "shop-4 waits for Signed." }),
            "held",
            "shop-4 waits for Signed.",
        ),
        (
            "board.item.moved",
            json!({ "id": "shop-4", "to": "parked" }),
            "idle",
            "shop-4 is in parked, where nobody works its cards: it waits for a person.",
        ),
    ];
    let mut reached = BTreeSet::new();
    for (topic, carried, kind, says) in cases {
        let column = if kind == "idle" { "parked" } else { "review" };
        let asked =
            Asked { topic: topic.into(), said: said(carried), column: column.into(), ..waits(every.clone()) };
        let answer = next(&asked).unwrap_or_else(|why| panic!("{topic}: {why}"));
        assert_eq!((answer.kind.as_str(), answer.says.as_str()), (kind, says), "{topic}");
        let who = if kind == "idle" { "@nell" } else { "applier" };
        assert_eq!(answer.rung.who, who, "{topic}");
        reached.insert(kind);
    }
    assert_eq!(reached, KINDS.iter().copied().collect(), "every kind of wait has a case");
    // A column no rung names for `idle` waits on nobody, whatever a row
    // that says no kind says.
    let elsewhere = Asked {
        topic: "board.item.moved".into(),
        said: said(json!({ "id": "shop-4", "to": "shelf" })),
        column: "shelf".into(),
        ..waits(every.clone())
    };
    assert_eq!(next(&elsewhere).unwrap().came, Some(Came::Nothing));
    // What the chain has, before anything is run for a card.
    assert_eq!(read(every.as_array().unwrap()).unwrap(), (true, vec!["parked".to_string()]));
    assert_eq!(read(&[]).unwrap(), (false, Vec::new()));
    assert_eq!(read(&[json!({ "title": "nobody" })]).unwrap(), (false, Vec::new()));
}

#[test]
fn what_makes_no_sense_is_refused_in_a_sentence() {
    let refused = |rows: Value, topic: &str| next(&Asked { topic: topic.into(), ..waits(rows) }).unwrap_err();
    assert_eq!(
        refused(json!([{ "who": "applier", "waits": ["soon"] }]), "card.waits"),
        "a rung of the chain waits for `soon`, which is no kind of wait: `review`, `left`, `blocked`, `asked`, `held`, `plan`, `idle`"
    );
    assert!(
        refused(json!([{ "who": "applier", "after": "a while" }]), "card.waits")
            .starts_with("a rung of the chain says to wait `a while`: ")
    );
    assert!(refused(json!([]), "card.worked").starts_with("`card.worked` says no wait a rung is for"));
    assert_eq!(
        refused(json!(["applier"]), "card.waits"),
        "`rows` is the rows of the chain, each a row; one of them is \"applier\""
    );
}

#[test]
fn a_rung_may_have_an_agent_look_or_review_and_says_what_it_is_to_be_told_and_may_end_with() {
    use deciding::chain::looks_with;
    let rows = json!([
        { "title": "lead", "waits": ["review"], "step": 1, "who": "agent:shop-lead-1", "does": "review", "told": "Look as a reviewer would." },
        { "title": "architect", "step": 2, "who": "role:architect", "does": "look", "after": "2h" },
        { "title": "owner", "step": 3, "who": "applier" },
    ]);
    let first = next(&waits(rows.clone())).unwrap();
    assert_eq!(
        (first.rung.does.as_str(), first.rung.who.as_str(), first.rung.told.as_str(), first.further),
        ("review", "agent:shop-lead-1", "Look as a reviewer would.", true)
    );
    let due =
        Asked { kept: vec![json!("review"), json!("4"), json!("1")], due: Some(1), ..waits(rows.clone()) };
    let second = next(&due).unwrap();
    assert_eq!(
        (second.rung.does.as_str(), second.rung.who.as_str(), second.after.as_str()),
        ("look", "role:architect", "2h")
    );
    // The words an agent that looks may end with, and `answer` where the
    // card waits on a question.
    assert_eq!(looks_with("review"), ["stay=on", "back=back", "ask=asks", "up=left"]);
    assert_eq!(looks_with("asked"), ["stay=on", "back=back", "ask=asks", "up=left", "answer=again"]);
    // What makes no sense is refused in a sentence.
    let refused = |row: Value| next(&waits(json!([row]))).unwrap_err();
    assert_eq!(
        refused(json!({ "who": "applier", "does": "shout" })),
        "a rung of the chain does `shout`, and a rung does `tell`, `look` or `review`"
    );
    assert!(
        refused(json!({ "who": "@grace", "does": "look" }))
            .starts_with("a rung of the chain that looks names an agent")
    );
    assert!(
        refused(json!({ "who": "agent:shop-lead-1", "does": "review" }))
            .starts_with("a rung of the chain that reviews names one agent")
    );
    assert!(
        refused(json!({ "who": "role:lead", "waits": ["review"], "does": "review" }))
            .starts_with("a rung of the chain that reviews names one agent")
    );
}

#[test]
fn what_an_agent_that_looked_decided_comes_to_for_the_card_as_it_now_is() {
    use deciding::chain::{Decided, Looked, decided};
    let looked = |came: &str| Looked {
        item: "shop-4".into(),
        kind: "review".into(),
        came: came.into(),
        says: "The check needs `shell`; run it with leave.".into(),
        why: "With tax.".into(),
        question: "Who may give it `shell`?".into(),
        agent: "shop-lead-1".into(),
        may_move: true,
        found: true,
        status: "review".into(),
        taken: "ready".into(),
        asker: String::new(),
        moves: 4,
        began: 4,
        after: "2h".into(),
        further: true,
        worked: vec!["shop-worker-1".into()],
    };
    // What it is, what was asked; and what it comes to: the word, where the
    // card goes, what goes with it, what a person is asked, whether the
    // next rung is asked for after a while, and after how long.
    type Case = (&'static str, Looked, Decided, &'static str, &'static str, &'static str, bool, &'static str);
    let cases: Vec<Case> = vec![
        (
            "it stays, and the next rung is told after the rung's time",
            looked("on"),
            Decided::Stay,
            "",
            "",
            "",
            true,
            "2h",
        ),
        (
            "back to where it was taken from, with what was said",
            looked("back"),
            Decided::Back,
            "ready",
            "The check needs `shell`; run it with leave.",
            "",
            false,
            "",
        ),
        (
            "back, by an agent that may not move a card: a person is asked",
            Looked { may_move: false, ..looked("back") },
            Decided::Ask,
            "",
            "",
            "shop-4: shop-lead-1 would send it back to ready, and may not move a card itself: The check needs `shell`; run it with leave.",
            true,
            "2h",
        ),
        (
            "back, where it was taken from nowhere: a person is asked",
            Looked { taken: String::new(), ..looked("back") },
            Decided::Ask,
            "",
            "",
            "shop-4: shop-lead-1 would send it back to be worked, and it was taken from nowhere it can go back to: The check needs `shell`; run it with leave.",
            true,
            "2h",
        ),
        (
            "a person asked",
            looked("asks"),
            Decided::Ask,
            "",
            "",
            "shop-4: shop-lead-1 asks: Who may give it `shell`?",
            true,
            "2h",
        ),
        ("the next rung now", looked("left"), Decided::Up, "", "", "", true, "0s"),
        (
            "the next rung now, where there is none",
            Looked { further: false, ..looked("left") },
            Decided::Up,
            "",
            "",
            "",
            false,
            "0s",
        ),
        (
            "the question on it answered",
            Looked { kind: "asked".into(), asker: "shop-worker-1".into(), ..looked("again") },
            Decided::Answer,
            "",
            "With tax.",
            "",
            true,
            "2h",
        ),
        (
            "an answer where nothing was asked: a person is asked",
            looked("again"),
            Decided::Ask,
            "",
            "",
            "shop-4: shop-lead-1 would answer what was asked, and nothing is: With tax.",
            true,
            "2h",
        ),
        (
            "moved while it looked",
            Looked { moves: 5, ..looked("back") },
            Decided::Nothing,
            "",
            "",
            "",
            false,
            "",
        ),
        (
            "gone while it looked",
            Looked { found: false, ..looked("back") },
            Decided::Nothing,
            "",
            "",
            "",
            false,
            "",
        ),
        (
            "looked at by one of the role that worked it: whatever it decided, a person is asked",
            Looked { agent: "shop-worker-1".into(), ..looked("back") },
            Decided::Ask,
            "",
            "",
            "shop-4: shop-worker-1 looked at it and worked it, and an agent does not judge its own work: a person is to.",
            true,
            "2h",
        ),
    ];
    let mut reached = BTreeSet::new();
    for (what, asked, word, to, note, asks, later, after) in cases {
        let came = decided(&asked).unwrap_or_else(|why| panic!("{what}: {why}"));
        assert_eq!(
            (
                came.decided,
                came.to.as_str(),
                came.note.as_str(),
                came.asks.as_str(),
                came.later,
                came.after.as_str()
            ),
            (Some(word), to, note, asks, later, after),
            "{what}"
        );
        reached.insert(word.word());
    }
    assert_eq!(reached, Decided::ALL.iter().map(|one| one.word()).collect(), "every word has a case");
    assert!(decided(&looked("maybe")).unwrap_err().starts_with("`came` is what the agent's turn came to"));
}

#[test]
fn a_rung_that_would_have_an_agent_judge_its_own_work_is_passed_over_to_the_next_or_a_person_is_told() {
    let rows = json!([
        { "title": "self", "waits": ["review"], "step": 1, "who": "agent:shop-worker-1", "does": "review" },
        { "title": "lead", "step": 2, "who": "agent:shop-lead-1", "does": "look", "after": "2h" },
        { "title": "owner", "step": 3, "who": "applier" },
    ]);
    // The worker's own rung is passed over: the lead looks first.
    let first = next(&waits(rows.clone())).unwrap();
    assert_eq!((first.rung.who.as_str(), first.keep[2].clone()), ("agent:shop-lead-1", json!(2)));
    // The lead worked it too: passed over as well, to the owner.
    let both = Asked { worked: vec!["shop-worker-1".into(), "shop-lead-1".into()], ..waits(rows.clone()) };
    let owner = next(&both).unwrap();
    assert_eq!((owner.rung.who.as_str(), owner.rung.does.as_str()), ("applier", "tell"));
    // A rung that tells is never passed over, whoever it names.
    let told = json!([{ "title": "self", "who": "agent:shop-worker-1" }]);
    assert_eq!(next(&waits(told)).unwrap().rung.who, "agent:shop-worker-1");
    // With nobody after it, whoever applied the fleet is told why.
    let only =
        json!([{ "title": "self", "waits": ["review"], "who": "agent:shop-worker-1", "does": "review" }]);
    let told = next(&waits(only)).unwrap();
    assert_eq!((told.came, told.rung.who.as_str()), (Some(Came::Acts), "applier"));
    assert_eq!(
        told.says,
        "shop-4 waits for a person to review it. shop-worker-1 is next for it on the chain, and worked it: an agent \
         does not judge its own work, so a person is to."
    );
    assert_eq!((told.keep[2].clone(), told.further), (json!(2), false));
}

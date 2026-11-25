//! What a told line holds beside its words: its parts checked as they
//! are told, which cards a part lists, what passing and failing one comes
//! to, and what was said on a card for whoever reads it and for its agent.

use kernel::parts::{
    Cards, Done, Line, Part, Pressing, Then, Verdict, discussed, keep, keeps, of, press, read, said_as_text,
    shown, thread,
};
use serde_json::{Value, json};

#[test]
fn a_part_is_checked_as_it_is_told_and_kept_with_what_it_leaves_unsaid_filled_in() {
    // What a step says, the project it is applied for, and what is kept.
    let cards = json!([{ "kind": "cards", "column": "staging" }]);
    assert_eq!(
        read(&cards, Some("shop")).unwrap(),
        [json!({
            "kind": "cards", "project": "shop", "column": "staging", "label": "",
            "fail_to": "ready", "pass_to": "", "pass_press": "", "reason": true,
        })]
    );
    let said = json!([
        { "kind": "links", "links": ["http://127.0.0.1:7760/", "https://example.invalid/a"] },
        { "kind": "cards", "project": "mill", "label": "try", "fail_to": "needs-analysis", "pass_to": "done", "reason": false },
    ]);
    let kept = read(&said, Some("shop")).unwrap();
    assert_eq!(kept[0], said[0]);
    assert_eq!(
        (&kept[1]["project"], &kept[1]["column"], &kept[1]["reason"]),
        (&json!("mill"), &json!(""), &json!(false))
    );
    assert_eq!(read(&Value::Null, None).unwrap(), Vec::<Value>::new(), "a line need hold none");
    // A message of a channel, opened where it was said.
    let message = json!([{ "kind": "message", "channel": "ch_1", "message": "msg_2" }]);
    assert_eq!(read(&message, None).unwrap(), [message[0].clone()]);
    assert_eq!(of(&message), [Part::Message { channel: "ch_1".into(), message: "msg_2".into() }]);
    let refused = read(&json!([{ "kind": "message", "channel": "ch_1" }]), None).unwrap_err();
    assert!(refused.contains("says the `channel` and the `message`"), "{refused}");
    // Read back, as the line keeps them; and what is no part is passed over.
    let back = of(&json!([kept[0], kept[1], { "kind": "pictures" }, "words"]));
    assert_eq!(back.len(), 2);
    assert_eq!(
        back[0],
        Part::Links(vec!["http://127.0.0.1:7760/".into(), "https://example.invalid/a".into()])
    );
    let Part::Cards(read_back) = &back[1] else { panic!("{back:?}") };
    assert_eq!(
        (read_back.label.as_str(), read_back.fail_to.as_str(), read_back.reason),
        ("try", "needs-analysis", false)
    );
}

#[test]
fn a_part_that_cannot_be_told_with_is_refused_saying_why() {
    // What is told, and the words that refuse it.
    let cases: &[(Value, Option<&str>, &str)] = &[
        (json!("cards"), None, "`parts` is a list of parts"),
        (json!([{ "column": "staging" }]), None, "part 1: a part says its `kind`"),
        (json!([{ "kind": "pictures" }]), None, "`pictures` is no kind of part"),
        (json!([{ "kind": "links" }]), None, "says `links`: a list of addresses"),
        (json!([{ "kind": "links", "links": ["ftp://x"] }]), None, "is no address a link leads to"),
        (json!([{ "kind": "links", "links": ["https://a b"] }]), None, "is no address"),
        (
            json!([{ "kind": "links", "links": ["https://a"], "column": "x" }]),
            None,
            "takes tab, links, and not `column`",
        ),
        (json!([{ "kind": "cards" }]), Some("shop"), "say one of the two"),
        (json!([{ "kind": "cards", "column": "a", "label": "b" }]), Some("shop"), "say one of the two"),
        (json!([{ "kind": "cards", "column": "staging" }]), None, "say `project`"),
        (
            json!([{ "kind": "cards", "column": "staging", "reason": "yes" }]),
            Some("shop"),
            "`reason` is true or false",
        ),
        (json!([{ "kind": "cards", "column": "ready" }]), Some("shop"), "leaves `ready`"),
        (
            json!([{ "kind": "cards", "column": "staging", "pass_to": "staging" }]),
            Some("shop"),
            "leaves `staging`",
        ),
        (
            json!([{ "kind": "cards", "column": "staging", "pass_to": "done", "pass_press": "signed" }]),
            Some("shop"),
            "say one of the two",
        ),
        (json!([{ "kind": "cards", "column": 3 }]), Some("shop"), "`column` of a part is a word"),
        // The cards a part keeps are the telling's, and never a step's.
        (
            json!([{ "kind": "cards", "column": "staging", "kept": ["shop-1"] }]),
            Some("shop"),
            "and not `kept`",
        ),
        (json!([{ "kind": "links", "links": ["https://a"] }, 7]), None, "part 2: a part is a mapping"),
    ];
    for (parts, project, says) in cases {
        let refused = read(parts, *project).expect_err(&parts.to_string());
        assert!(refused.contains(says), "{parts}: {refused}");
    }
    let many: Vec<Value> = (0..11).map(|_| json!({ "kind": "links", "links": ["https://a"] })).collect();
    assert!(read(&json!(many), None).unwrap_err().contains("10 parts at most"));
}

fn staging() -> Cards {
    Cards {
        project: "shop".into(),
        column: "staging".into(),
        fail_to: "ready".into(),
        reason: true,
        ..Cards::default()
    }
}

/// A part of staging that kept these cards as it was told, which go to done when passed.
fn kept(cards: &[&str]) -> Cards {
    Cards { pass_to: "done".into(), kept: Some(cards.iter().map(|id| id.to_string()).collect()), ..staging() }
}

fn pressing(verdict: Verdict, reason: &str) -> Pressing {
    Pressing {
        id: "shop-3".into(),
        verdict: Some(verdict),
        reason: reason.into(),
        by: "user:ada".into(),
        status: "staging".into(),
        worked: vec!["agent:shop-worker-1".into()],
        ..Pressing::default()
    }
}

#[test]
fn a_part_of_a_column_keeps_the_cards_it_held_as_it_was_told_and_a_line_hands_out_none_of_them() {
    let told = read(
        &json!([
            { "kind": "links", "links": ["https://a"] },
            { "kind": "cards", "column": "staging", "pass_to": "done" },
            { "kind": "cards", "label": "try" },
        ]),
        Some("shop"),
    )
    .unwrap();
    // Which part keeps the cards of which column: one of a column, and no other.
    let kept: Vec<_> = told.iter().map(keeps).collect();
    assert_eq!(kept, [None, Some(("shop".to_string(), "staging".to_string())), None]);
    let mut part = told[1].clone();
    keep(&mut part, vec!["shop-2".into(), "shop-1".into()]);
    let line = json!([told[0], part, told[2]]);
    // Read back, it lists those, in the column's order; a part that keeps
    // none, as a program from before kept it, lists what is there.
    let back = of(&line);
    let Part::Cards(cards) = &back[1] else { panic!("{back:?}") };
    assert_eq!(cards.kept, Some(vec!["shop-2".to_string(), "shop-1".to_string()]));
    let Part::Cards(labelled) = &back[2] else { panic!("{back:?}") };
    assert_eq!(labelled.kept, None);
    let Part::Cards(before) = &of(&json!([told[1]]))[0] else { panic!() };
    assert_eq!(before.kept, None, "a line told before cards were kept keeps none");
    // Handed out with the line, the parts say what they were told with and none of the cards.
    assert_eq!(shown(&line), json!([told[0], told[1], told[2]]));
    assert_eq!(shown(&Value::Null), Value::Null);
}

#[test]
fn a_part_lists_the_cards_of_its_column_or_its_label() {
    let labelled = Cards { column: String::new(), label: "try".into(), ..staging() };
    let labels = |words: &[&str]| words.iter().map(|word| word.to_string()).collect::<Vec<_>>();
    // The part, where a card is, what it holds, and whether it is listed.
    let cases = [
        (staging(), "staging", labels(&[]), true),
        (staging(), "ready", labels(&["staging"]), false),
        (labelled.clone(), "ready", labels(&["try", "ui"]), true),
        (labelled.clone(), "staging", labels(&["ui"]), false),
    ];
    for (cards, status, held, listed) in cases {
        assert_eq!(cards.lists(status, &held), listed, "{cards:?} {status} {held:?}");
    }
}

#[test]
fn a_press_comes_to_a_move_with_why_or_to_the_card_approved_and_then_what_the_part_says_or_is_refused() {
    let done = Cards { pass_to: "done".into(), ..staging() };
    let pressed = Cards { pass_press: "signed".into(), ..staging() };
    let passed = |then| Ok(Done::Passed { then });
    let unreasoned = Cards { reason: false, ..staging() };
    let elsewhere = Pressing { status: "ready".into(), ..pressing(Verdict::Pass, "") };
    let by = |by: &str, verdict| Pressing { by: by.into(), ..pressing(verdict, "Too slow") };
    let assigned = Pressing { assignee: Some("agent:scout".into()), ..by("agent:scout", Verdict::Pass) };
    // The part, the press, and what it comes to.
    let cases: Vec<(Cards, Pressing, Result<Done, &str>)> = vec![
        (
            staging(),
            pressing(Verdict::Fail, " The total is wrong. "),
            Ok(Done::Moved { to: "ready".into(), said: "The total is wrong.".into() }),
        ),
        (staging(), pressing(Verdict::Fail, "  "), Err("is failed here only with why")),
        (
            unreasoned.clone(),
            pressing(Verdict::Fail, ""),
            Ok(Done::Moved { to: "ready".into(), said: String::new() }),
        ),
        // Passed, it is approved, and no words are said on it for that.
        (staging(), pressing(Verdict::Pass, ""), passed(Then::Nothing)),
        (staging(), pressing(Verdict::Pass, "Works here."), passed(Then::Nothing)),
        (done.clone(), pressing(Verdict::Pass, ""), passed(Then::Move("done".into()))),
        (done.clone(), pressing(Verdict::Pass, "Good."), passed(Then::Move("done".into()))),
        (pressed.clone(), pressing(Verdict::Pass, ""), passed(Then::Press("signed".into()))),
        (staging(), elsewhere, Err("is not one of the cards in staging any more: it is in ready")),
        (
            staging(),
            by("agent:shop-worker-1", Verdict::Fail),
            Err("shop-worker-1 worked on shop-3, and an agent does not pass or fail its own work"),
        ),
        (staging(), assigned, Err("scout worked on shop-3")),
        (staging(), by("agent:critic", Verdict::Pass), passed(Then::Nothing)),
        (
            staging(),
            by("key:k1", Verdict::Fail),
            Ok(Done::Moved { to: "ready".into(), said: "Too slow".into() }),
        ),
        (staging(), Pressing { verdict: None, ..pressing(Verdict::Pass, "") }, Err("`pass` or `fail`")),
        // A part that kept its cards: one of those, where the part says it
        // is, is pressed; one that moved on is not; one that came to the
        // column after is no card the line was told of.
        (kept(&["shop-3"]), pressing(Verdict::Pass, ""), passed(Then::Move("done".into()))),
        (
            kept(&["shop-3"]),
            Pressing { status: "done".into(), ..pressing(Verdict::Fail, "Late.") },
            Err("is not one of the cards in staging any more: it is in done"),
        ),
        (
            kept(&["shop-1"]),
            pressing(Verdict::Pass, ""),
            Err("shop-3 is not one of the cards this line was told of"),
        ),
    ];
    for (cards, pressed, came) in cases {
        let found = press(&cards, &pressed);
        match came {
            Ok(done) => assert_eq!(found, Ok(done), "{pressed:?}"),
            Err(says) => {
                assert!(found.as_ref().is_err_and(|why| why.contains(says)), "{pressed:?}: {found:?}")
            }
        }
    }
    assert_eq!(Verdict::read("fail"), Ok(Verdict::Fail));
    assert!(Verdict::read("maybe").unwrap_err().contains("`pass` or `fail`"));
}

fn line(topic: &str, actor: &str, data: Value) -> Line {
    let who = actor.split(':').nth(1).unwrap_or(actor);
    Line::of(topic, actor, who, 0, &data)
}

#[test]
fn what_was_said_on_a_card_since_it_came_to_the_column_is_its_thread() {
    let lines = [
        line("board.item.created", "user:ada", json!({ "status": "ready" })),
        line("record.commented", "user:ada", json!({ "body": "Before." })),
        line("board.item.moved", "user:ada", json!({ "to": "staging", "from": "review" })),
        line("record.commented", "user:ada", json!({ "body": "Now the cart." })),
        line("card.asked", "user:ada", json!({ "asker": "shop-worker-1", "question": "Which cart?" })),
        line("board.item.updated", "user:ada", json!({ "changed": ["labels"] })),
        line("record.commented", "user:mira", json!({ "body": "The blue one." })),
    ];
    let thread = thread(&lines, "staging");
    let read: Vec<(&str, &str)> =
        thread.iter().map(|line| (line.who.as_str(), line.words.as_str())).collect();
    assert_eq!(read, [("ada", "Now the cart."), ("shop-worker-1", "Which cart?"), ("mira", "The blue one.")]);
    assert_eq!(thread[1].actor, "agent:shop-worker-1", "a question is its agent's");
    // Where nothing says it came there (a label's part), all of it.
    assert_eq!(count(&lines, ""), 4);
    assert_eq!(count(&lines, "doing"), 4);
}

fn count(lines: &[Line], column: &str) -> usize {
    thread(lines, column).len()
}

#[test]
fn what_its_agent_is_told_is_what_others_said_since_its_last_turn_and_not_what_it_is_told_already() {
    let turns = ["card.worked".to_string(), "card.asked".to_string()];
    let lines = [
        line("record.commented", "user:ada", json!({ "body": "Mind the tax." })),
        line("card.worked", "user:ada", json!({ "agent": "shop-worker-1" })),
        line("record.commented", "agent:shop-worker-1", json!({ "body": "My own words." })),
        line("record.commented", "user:mira", json!({ "body": "The total\nis wrong." })),
        line("record.commented", "agent:critic", json!({ "body": "So is the date." })),
        line("record.commented", "user:ada", json!({ "body": "Too slow" })),
    ];
    let told = discussed(&lines, &turns, " Too slow ");
    assert_eq!(said_as_text(&told), "- mira: The total\n  is wrong.\n- critic: So is the date.");
    // Before any turn, everything said; with none since, nothing.
    assert_eq!(discussed(&lines[..1], &turns, "").len(), 1);
    assert!(discussed(&lines[..2], &turns, "").is_empty());
    // Since a question it asked, what others said after it, its own excepted.
    let asked = [
        line("card.asked", "user:ada", json!({ "asker": "shop-worker-1", "question": "Which?" })),
        line("record.commented", "user:ada", json!({ "body": "This one." })),
        line("record.commented", "agent:shop-worker-1", json!({ "body": "Thanks." })),
    ];
    assert_eq!(said_as_text(&discussed(&asked, &turns, "")), "- ada: This one.");
    assert_eq!(said_as_text(&[]), "");
}

#[test]
fn a_part_of_a_change_somebody_asked_for_is_read_back_and_is_never_one_a_step_tells_with() {
    let kept = json!([{ "kind": "change", "project": "shop", "ask": "ask_1" }]);
    assert_eq!(of(&kept), [Part::Change { project: "shop".into(), ask: "ask_1".into() }]);
    let told = read(&kept, Some("shop")).unwrap_err();
    assert!(told.contains("`change` is no kind of part"), "{told}");
}

#[test]
fn a_part_of_a_run_that_waits_for_a_go_ahead_is_read_back_and_is_never_one_a_step_tells_with() {
    let kept = json!([{ "kind": "go", "project": "shop", "ask": "ask_2" }]);
    assert_eq!(of(&kept), [Part::Go { project: "shop".into(), ask: "ask_2".into() }]);
    let told = read(&kept, Some("shop")).unwrap_err();
    assert!(told.contains("`go` is no kind of part"), "{told}");
}

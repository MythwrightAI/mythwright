//! A review that said how it came out and then ended, as every review
//! does, is never taken for one that ended with nothing said, whichever
//! of its stage's handlers comes first. The stage hears what a
//! review says in one handler (`decided`) and that it is over in another
//! (`ended`), and a stage's handlers take turns by who asks first: after
//! a stop, the one that heard the ending asked first, read the review as
//! one that said nothing, and the card was said to wait for a person
//! beside a verdict that passed.
//!
//! The order is made here with no stop, by what holds the card's turn.
//! What a review says is heard by `decided`, which takes its turn by the
//! card (never behind another card's), with the handler that
//! begins what a card is due (`due`); the handler that hears endings takes
//! no turn by the card. So the card is said to be due again, from a command
//! a row lays after its review (as a rung of a chain says it), and `due` is
//! kept at that command, holding the card's turn. Its review, a way of the
//! project's own that waits to be let go before it says that the card
//! passed and again after, then passes and ends: what it said waits for
//! the card's turn, and its ending is heard meanwhile, while the review is
//! still kept as under way. The command is let go only then.
//!
//! And a review that asks for another in its place (reviews arranged
//! another way) and ends is the same: its ending is heard while the
//! card's turn is held by a command laid before the review, with the
//! review still kept and nothing begun in its place; once the turn goes
//! on, what it said is taken in before its ending, which comes to nothing
//! with the card still in review. The order in which the ending is taken
//! in before the review asked for in its place is begun cannot be made
//! now: `due`, which begins that one, hears what the review asked before
//! the ending is said, and takes the same turn. What holds the rule there
//! is that `decided` takes in what the review said, and takes it off by
//! its name, before its ending, whichever comes first of the two.

use std::path::Path;

use serde_json::{Value, json};

use crate::Rig;
use crate::fleets::{add, add_row, card, params, rig};
use crate::lanes::{git, move_card, moved};

/// A way of reviewing of the project's own: it waits at a hold of its
/// card's before it passes the card, and at another after.
const WAITS: &str = r#"blueprint: waits
description: Pass one card, when let go.
permissions: [board, shell]

params:
  project: { type: string }
  item: { type: string }
  stage: { type: string, default: review }
  role: { type: string, default: reviewer }
  picks: { type: string, default: first }
  seats: { type: int, default: 1 }
  needed: { type: int, default: 0 }
  pool: { type: string, default: "" }
  arranged: { type: string, default: "" }

steps:
  looked:
    do: shell
    with: { cmd: BEFORE }
  passed:
    needs: [looked]
    do: emit
    with:
      topic: review.passed
      card: "${params.item}"
      project: "${params.project}"
      by: nobody
      words: looked over
      approved: 1
      seats: 1
  over:
    needs: [passed]
    do: shell
    with: { cmd: AFTER }
"#;

#[tokio::test]
async fn a_review_that_passed_is_not_taken_for_one_that_said_nothing_where_its_ending_is_heard_first() {
    let (rig, at) = rig().await;
    let hold = |name: &str| {
        let hold = at.join(name);
        crate::hold(&hold);
        hold
    };
    let (looks, ends, checks) = (hold("before-shop-1"), hold("after-shop-1"), hold("hold-check"));
    let mut begun = crate::Listening::at(&at.join("began-check"));
    shop_reviewed_by_waits(&rig, &at).await;
    // A command after the review, which says it began and waits.
    let file = |name: &str| at.join(name).to_string_lossy().into_owned();
    let (begins, waits) = (file("began-check"), file("hold-check"));
    let command = crate::standin::line(&["tell", &begins, "check", "held", &waits, "check"]);
    let row =
        json!({ "title": "Checks", "stage": "review", "order": 15, "does": "std/run", "command": command });
    add_row(&rig, "segments", row).await;
    // The command is the last thing done after the review: nothing is
    // merged, so that the list gone through from it ends there.
    crate::fleets::change(&rig, "stages", "review", json!({ "merge": false })).await;

    // The card is worked, and its review waits.
    let cart = add(&rig, "Add a cart", &[]).await;
    assert_eq!(cart, "shop-1");
    move_card(&rig, &cart, "ready", None).await;
    review_of(&rig, &cart, "instance.started", 0).await;

    // The card's turn is held: it is said to be due again, from the
    // command laid after its review, as a rung of a chain says it, and the
    // stage's handler that begins what a card is due is kept at that
    // command, in the card's turn.
    let mark = rig.mark();
    let due =
        json!({ "card": cart, "project": "shop", "stage": "review", "again": true, "row": "", "order": 15 });
    rig.send("/api/emit/review.due", due).await;
    crate::soon("the command the card was said to be due again from to have begun", async {
        begun.next_line().await.unwrap()
    })
    .await;

    // The review passes and ends meanwhile. What it said waits for the
    // card's turn; its ending is heard by the handler that hears endings,
    // which takes no turn by the card, while the review is still kept as
    // under way: it says again that the review is over, and works out
    // nothing.
    crate::pass(&ends, 1);
    crate::let_go("the card's review", &looks).await;
    let ended = review_of(&rig, &cart, "instance.finished", mark).await;
    let passed = rig.event("review.passed", move |event| event.seq > mark).await;
    assert_eq!(passed.data["card"], json!(cart));
    assert!(passed.seq < ended, "a review says how it came out before it ends");
    let said = rig.event("review.ended", move |event| event.seq > mark).await;
    assert_eq!(said.data["card"], json!(cart), "the ending was heard with the review still kept");

    // Let go, the card goes on. The list it was said to be due again from
    // has ended by itself as well, after the command, which is taken in
    // last and comes to nothing more: the card has gone on.
    crate::pass(&checks, 8);
    moved(&rig, &cart, "done", mark).await;
    let whose = cart.clone();
    finished(&rig, mark, "on.decided", move |event| {
        event["topic"] == "review.came" && event["data"]["card"] == json!(whose)
    })
    .await;

    // The stage came to the review's ending before it came to its
    // verdict: the order this is for.
    let verdict = stage_run(&rig, "on.decided", None, |event| {
        event["topic"] == "review.passed" && event["data"]["card"] == json!(cart)
    })
    .await
    .expect("the stage heard the card's verdict");
    let over = format!("review-{cart}-");
    let ending = stage_run(&rig, "on.ended", Some(&verdict.instance), |event| {
        event["topic"] == "instance.finished"
            && event["data"]["key"].as_str().is_some_and(|key| key.starts_with(&over))
    })
    .await
    .expect("the stage heard the review's ending");
    let (ending, verdict) = (began(&rig, &ending.id).await, began(&rig, &verdict.id).await);
    assert!(ending < verdict, "the order was not made: the verdict was taken in first");

    // The card is not said to wait for a person.
    assert_eq!(card(&rig, &cart).await["status"], "done");
    assert_eq!(rig.count_of("card.waits").await, 0, "{}", rig.lately().await);
    assert_eq!(rig.count_of("card.left").await, 0, "{}", rig.lately().await);

    // That the review is over was said again, heard after its verdict by
    // the handler that heard the verdict, and came to nothing.
    let mut follow = rig.follow_after(said.seq, "run.finished");
    crate::soon("the stage to have heard that the review is over", async {
        loop {
            let event = follow.next().await.unwrap();
            let Some(run) = event.source.run.clone() else { continue };
            if event.data["flow"] == "on.decided"
                && runs(&rig).await.iter().any(|ran| ran.id == run && ran.event["topic"] == "review.ended")
            {
                return;
            }
        }
    })
    .await;
    assert_eq!(rig.count_of("card.waits").await, 0, "{}", rig.lately().await);
}

#[tokio::test]
async fn a_review_that_asks_for_another_in_its_place_is_not_taken_for_one_that_said_nothing_as_it_ends() {
    let (rig, at) = rig().await;
    // The fleet's reviewer waits here, each time it is asked.
    let reviewer = at.join("hold");
    crate::hold(&reviewer);
    let checks = at.join("hold-check");
    crate::hold(&checks);
    let mut begun = crate::Listening::at(&at.join("began-check"));
    let mut checked = async || {
        crate::soon("the command before a review to have begun", async { begun.next_line().await.unwrap() })
            .await;
    };
    crate::fleets::shop(&rig, json!({})).await;
    // A command before the review, which says it began and waits: it is
    // gone through by the handler that begins a card's review.
    let file = |name: &str| at.join(name).to_string_lossy().into_owned();
    let (begins, waits) = (file("began-check"), file("hold-check"));
    let command = crate::standin::line(&["tell", &begins, "check", "held", &waits, "check"]);
    let row =
        json!({ "title": "Checks", "stage": "review", "order": 5, "does": "std/run", "command": command });
    add_row(&rig, "segments", row).await;

    // The first card's review is in session, its reviewer waiting; the
    // second's has asked for its place behind it and not begun.
    let first = add(&rig, "Add a cart", &[]).await;
    move_card(&rig, &first, "ready", None).await;
    checked().await;
    crate::pass(&checks, 1);
    let seat = format!("review:{first}:");
    rig.event("agent.turn.started", move |event| {
        event.data["thread"].as_str().is_some_and(|thread| thread.starts_with(&seat))
    })
    .await;
    let second = add(&rig, "Add a basket", &[]).await;
    move_card(&rig, &second, "ready", None).await;
    checked().await;
    crate::pass(&checks, 1);
    let asked = second.clone();
    rig.event("pool.asked", move |event| {
        event.data["about"] == asked.as_str() && event.data["pool"] == "shop/review"
    })
    .await;

    // The handler that begins a card's review is kept at the second card,
    // at the command before its review: said to be due again from where
    // that command is, as a rung of a chain says it.
    let mark = rig.mark();
    let due =
        json!({ "card": second, "project": "shop", "stage": "review", "again": true, "row": "", "order": 5 });
    rig.send("/api/emit/review.due", due).await;
    checked().await;

    // Reviews are arranged another way: the second's review asks for
    // another in its place and ends, and the one that would be begun waits
    // its turn behind the command.
    crate::fleets::change(&rig, "stages", "review", json!({ "needed": 1 })).await;
    // What the review asks, said by it with `again`: the stage, carried
    // over to its row as changed, also looks at its column and says each
    // card there is due a review, saying no `again`, and that may come
    // first.
    let asked = second.clone();
    let again = rig
        .event("review.due", move |event| {
            event.seq > mark
                && event.source.instance.is_some()
                && event.data["card"] == asked.as_str()
                && event.data["again"] == true
        })
        .await;
    assert_eq!(again.data["again"], true);
    let ended = review_of(&rig, &second, "instance.finished", mark).await;
    assert!(again.seq < ended, "a review says what it asks before it ends");
    let over = format!("review-{second}-");
    let ending = finished(&rig, mark, "on.ended", move |event| {
        event["topic"] == "instance.finished"
            && event["data"]["key"].as_str().is_some_and(|key| key.starts_with(&over))
    })
    .await;
    // The order this is for: nothing of a review of the second card begun
    // since, the command still waiting.
    let review = format!("review-{second}-");
    let all = journal::EventQuery { topics: "instance.started".into(), limit: 10_000, ..Default::default() };
    let begun = rig.said(all).await.into_iter().filter(|event| {
        event.seq > mark && event.data["key"].as_str().is_some_and(|key| key.starts_with(&review))
    });
    assert_eq!(
        begun.count(),
        0,
        "the order was not made: another review was begun before the ending was heard"
    );
    assert!(ending > ended);
    // It was heard with the review still kept: what the review said waits
    // for the card's turn, behind the command, with what its ending comes
    // to, and the handler that hears endings says again that it is over.
    let said = rig.event("review.ended", move |event| event.seq > mark).await;
    assert_eq!(said.data["card"], json!(second), "the ending was heard with the review still kept");
    let taken_in = runs(&rig).await.into_iter().filter(|ran| {
        ran.flow == "on.decided" && ran.finished.is_some() && ran.event["data"]["card"] == json!(second)
    });
    assert_eq!(taken_in.count(), 0, "what the review said is taken in only in the card's turn");

    // The command let go, the card's turn goes on: what the review said and
    // its ending are heard by the one handler, in the order they were said,
    // and come to nothing, the card still in review.
    crate::pass(&checks, 1);
    let review = format!("review-{second}-");
    assert!(again.data["review"].as_str().is_some_and(|key| key.starts_with(&review)), "{}", again.data);
    // It said which review it was, and was taken off by that name.
    let asked = finished(&rig, mark, "on.decided", move |event| {
        event["topic"] == "review.due"
            && event["data"]["review"].as_str().is_some_and(|key| key.starts_with(&review))
    })
    .await;
    let whose = second.clone();
    let heard = finished(&rig, mark, "on.decided", move |event| {
        event["topic"] == "review.ended" && event["data"]["card"] == json!(whose)
    })
    .await;
    assert!(asked < heard, "what it said is taken in before its ending");
    assert_eq!(card(&rig, &second).await["status"], "review");
    assert_eq!(rig.count_of("card.waits").await, 0, "{}", rig.lately().await);

    // Taken out of review, and everything let go: the first card goes on,
    // and nothing is said to wait for a person.
    move_card(&rig, &second, "draft", None).await;
    crate::pass(&checks, 8);
    crate::let_go("the first card's reviewer", &reviewer).await;
    moved(&rig, &first, "done", mark).await;
    // The command let go, the handler that was kept at it has done.
    rig.event("run.finished", move |event| {
        event.seq > ending && event.data["blueprint"] == "std/reviewing" && event.data["flow"] == "on.due"
    })
    .await;
    assert_eq!(rig.count_of("card.waits").await, 0, "{}", rig.lately().await);
}

#[tokio::test]
async fn a_review_that_said_how_it_came_out_is_ended_by_its_stage_once_that_is_taken_in() {
    // The review that ships, which said how it came out and then ended
    // itself, had its ending heard as often as not before its stage took
    // in what it said, wherever the stage was slower: the handler that
    // hears endings found it kept still, said again that it was over, and
    // the handler that heard the verdict ran once more for that, to no
    // end (a card cost two runs more, which way round going by
    // the machine). It says which review it is, and its stage ends it once
    // it has taken it off. The order is made as above: what it says waits
    // for the card's turn, held by a command laid after its review.
    let (rig, at) = rig().await;
    let reviewer = at.join("hold");
    crate::hold(&reviewer);
    let checks = at.join("hold-check");
    crate::hold(&checks);
    let mut begun = crate::Listening::at(&at.join("began-check"));
    crate::fleets::shop(&rig, json!({})).await;
    let file = |name: &str| at.join(name).to_string_lossy().into_owned();
    let (begins, waits) = (file("began-check"), file("hold-check"));
    let command = crate::standin::line(&["tell", &begins, "check", "held", &waits, "check"]);
    let row =
        json!({ "title": "Checks", "stage": "review", "order": 15, "does": "std/run", "command": command });
    add_row(&rig, "segments", row).await;
    crate::fleets::change(&rig, "stages", "review", json!({ "merge": false })).await;

    // The card is worked, and its reviewer looks.
    let cart = add(&rig, "Add a cart", &[]).await;
    move_card(&rig, &cart, "ready", None).await;
    let seat = format!("review:{cart}:");
    rig.event("agent.turn.started", move |event| {
        event.data["thread"].as_str().is_some_and(|thread| thread.starts_with(&seat))
    })
    .await;

    // The card's turn is held at the command laid after its review.
    let mark = rig.mark();
    let due =
        json!({ "card": cart, "project": "shop", "stage": "review", "again": true, "row": "", "order": 15 });
    rig.send("/api/emit/review.due", due).await;
    crate::soon("the command the card was said to be due again from to have begun", async {
        begun.next_line().await.unwrap()
    })
    .await;

    // The review passes, saying which review it is, and what it said
    // waits for the card's turn.
    crate::let_go("the card's reviewer", &reviewer).await;
    let whose = cart.clone();
    let passed =
        rig.event("review.passed", move |event| event.seq > mark && event.data["card"] == json!(whose)).await;

    // Let go, the card goes on.
    crate::pass(&checks, 8);
    moved(&rig, &cart, "done", mark).await;
    let verdict = stage_run(&rig, "on.decided", None, |event| {
        event["topic"] == "review.passed" && event["data"]["card"] == json!(cart)
    })
    .await
    .expect("the stage heard the card's verdict");
    let ended = review_of(&rig, &cart, "instance.finished", mark).await;
    assert!(
        began(&rig, &verdict.id).await < ended,
        "the review ended before its stage took in what it said: {}",
        rig.lately().await
    );
    let review = format!("review-{cart}-");
    assert!(passed.data["review"].as_str().is_some_and(|key| key.starts_with(&review)), "{}", passed.data);

    // Nothing was said again of its ending, and nothing heard it to no end.
    crate::simulation::rested(&rig).await;
    let over = journal::EventQuery {
        topics: "review.ended".into(),
        limit: 10_000,
        after: mark,
        ..Default::default()
    };
    let said = rig.said(over).await;
    assert!(said.is_empty(), "its ending was said again: {said:?}");
    let ending = stage_run(&rig, "on.ended", Some(&verdict.instance), |event| {
        event["topic"] == "instance.finished"
            && event["data"]["key"].as_str().is_some_and(|key| key.starts_with(&review))
    })
    .await;
    assert!(ending.is_none(), "the stage heard the review's ending to no end: {ending:?}");
    assert_eq!(card(&rig, &cart).await["status"], "done");
    assert_eq!(rig.count_of("card.waits").await, 0, "{}", rig.lately().await);
}

/// Waits for a run of a handler of the stage that reviews, set off by what
/// `by` says of, to have finished after `mark`: where that came.
async fn finished(rig: &Rig, mark: u64, flow: &'static str, by: impl Fn(&Value) -> bool) -> u64 {
    let mut follow = rig.follow_after(mark, "run.finished");
    crate::soon(&format!("a run `{flow}` of the stage that reviews to have finished"), async {
        loop {
            let event = follow.next().await.unwrap();
            let Some(run) = event.source.run.clone() else { continue };
            if event.data["blueprint"] != "std/reviewing" || event.data["flow"] != flow {
                continue;
            }
            if runs(rig).await.iter().any(|ran| ran.id == run && by(&ran.event)) {
                return event.seq;
            }
        }
    })
    .await
}

/// `shop`, a repository with a board and the fleet, whose cards are
/// reviewed by [`WAITS`], each at holds called after it in `at`.
async fn shop_reviewed_by_waits(rig: &Rig, at: &Path) {
    let project = rig.send("/api/projects", json!({ "name": "Shop" })).await;
    let dir = Path::new(project["path"].as_str().unwrap());
    git(dir, &["init", "--quiet", "."]);
    git(dir, &["config", "user.name", "Ada"]);
    git(dir, &["config", "user.email", "ada@example.com"]);
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "--quiet", "-m", "first"]);
    let columns = ["draft", "ready", "doing", "review", "done"];
    let board = json!({
        "blueprint": "std/board", "project": "shop", "params": { "columns": columns }, "wait": true,
    });
    let applied = rig.send("/api/apply", board).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
    let hold = |side: &str| {
        let path = at.join(format!("{side}-ITEM")).to_string_lossy().replace("ITEM", "${params.item}");
        crate::standin::cmd(&["held", &path, "review"])
    };
    let text = WAITS.replace("BEFORE", &hold("before")).replace("AFTER", &hold("after"));
    let saved = json!({ "path": format!("@blueprints/{}", brand::blueprint_file("waits")), "content": text });
    let (status, answer) =
        rig.call(reqwest::Method::PUT, "/api/projects/shop/file", Some(&rig.token), Some(saved)).await;
    assert_eq!(status, reqwest::StatusCode::OK, "{answer}");
    let mut params = params(json!({}));
    params["review"] = json!("waits");
    let fleet = json!({ "blueprint": "std/fleet", "project": "shop", "params": params, "wait": true });
    let applied = rig.send("/api/apply", fleet).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
}

/// Where the review of `card` was said to have started, or to be over,
/// after `mark`.
async fn review_of(rig: &Rig, card: &str, topic: &str, mark: u64) -> u64 {
    let review = format!("review-{card}-");
    let of = move |event: &kernel::Event| {
        event.seq > mark && event.data["key"].as_str().is_some_and(|key| key.starts_with(&review))
    };
    rig.event(topic, of).await.seq
}

async fn runs(rig: &Rig) -> Vec<conductor::RunInfo> {
    rig.node().app.engine.recent_runs(500, Some("shop".into())).await.unwrap()
}

/// The first run of a handler of the stage that reviews (of `of`, where
/// that is known) set off by what `by` says of.
async fn stage_run(
    rig: &Rig,
    flow: &str,
    of: Option<&str>,
    by: impl Fn(&Value) -> bool,
) -> Option<conductor::RunInfo> {
    let reviewing = |run: &&conductor::RunInfo| {
        run.flow == flow && of.is_none_or(|of| run.instance == of) && by(&run.event)
    };
    runs(rig).await.iter().filter(reviewing).min_by_key(|run| run.started).cloned()
}

/// Where the run's beginning came.
async fn began(rig: &Rig, run: &str) -> u64 {
    let run = run.to_string();
    rig.event("run.started", move |event| event.source.run.as_deref() == Some(run.as_str())).await.seq
}

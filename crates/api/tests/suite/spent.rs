//! What one card cost, by round (`GET /api/board/items/{id}/cost`): a
//! card worked through the fleet that ships by stand-ins that say what
//! each turn took, as an agent's program does, sent back once by its
//! review, worked again and passed. What is laid out is held against
//! what happened: each round's turns, what they took and what was
//! decided at its end, and what all of them came to, on a clock that
//! stands still. And it is read by whoever may read the board and by
//! nobody else.

use std::path::Path;
use std::sync::Arc;

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;
use crate::fleets::{add, shop};
use crate::lanes::{history, move_card, moved};

/// What a stand-in says of a turn, as an agent's program says it: a
/// line of JSON, with what the turn cost and the tokens it read and wrote.
fn turn(text: &str, cost: f64, read: u64, wrote: u64) -> String {
    json!({ "text": text, "cost": cost, "in": read, "out": wrote }).to_string()
}

/// A package for a product there is not, whose program is the stand-in
/// doing `words`, read as an agent's program that says what it took.
fn package(name: &str, words: &[&str]) -> String {
    // Each word written as a string the reader takes whatever is in it.
    let args: Vec<String> = words.iter().map(|word| json!(word).to_string()).collect();
    format!(
        "extension: {name}\ntitle: {name}\ndescription: Not a real agent.\nagent:\n  program: {program}\n  \
         args:\n    - [{args}]\n  input: ${{ turn.prompt }}\n  output:\n    - when: ${{ event.text != null }}\n      \
         reply: ${{ event.text }}\n      cost_usd: ${{ event.cost }}\n      input_tokens: ${{ event.in }}\n      \
         output_tokens: ${{ event.out }}\n",
        program = json!(crate::STANDIN),
        args = args.join(", "),
    )
}

/// A factory with the fleet that ships in `shop`, whose worker and
/// reviewer are run by stand-ins that say what each turn took: the
/// reviewer sends the work back while `sends-back` is in `at`, and
/// approves otherwise.
async fn factory() -> (Rig, std::path::PathBuf) {
    // A clock that stands still: every turn and every line of the card's
    // history is in one millisecond, and its rounds are told apart by what
    // is written of each asking, never by when.
    let clock = kernel::ManualClock::new();
    let rig =
        Rig::owned(Rig::with("api-spent", move |config, _| config.clock = Some(Arc::new(clock))).await).await;
    let at = rig.dir.clone();
    let says = at.join("sends-back").to_string_lossy().into_owned();
    let works = turn("I changed a file.", 0.25, 1000, 200);
    let maker = package("maker", &["swallow", "append", "CHANGES.md", "changed", "say", &works]);
    let approves = turn("It does what the card asks.\nDECISION: approve", 0.01, 300, 30);
    let looker = package("looker", &["swallow", "says", &says, "say", &approves]);
    for source in [maker, looker] {
        rig.send("/api/extensions", json!({ "source": source })).await;
    }
    let rows = json!([
        { "title": "worker", "role": "worker", "count": 1, "adapter": "maker" },
        { "title": "reviewer", "role": "reviewer", "count": 1, "adapter": "looker", "permission": "read" },
    ]);
    shop(&rig, json!({ "agents": rows })).await;
    (rig, at)
}

fn sends_back(at: &Path) {
    let back = turn("The total is wrong.\nDECISION: send_back", 0.02, 400, 40);
    std::fs::write(at.join("sends-back"), format!("{back}\n")).unwrap();
}

/// What the route lays out of a round, written down short: who took each
/// turn and what it did, and what was decided.
fn shape(round: &Value) -> (Vec<(String, String)>, String, String) {
    let turns = round["turns"].as_array().unwrap();
    let turns = turns
        .iter()
        .map(|one| (one["agent"].as_str().unwrap().to_string(), one["does"].as_str().unwrap().to_string()))
        .collect();
    let decided = &round["decided"];
    (
        turns,
        decided["came"].as_str().unwrap().to_string(),
        decided["words"].as_str().unwrap_or_default().to_string(),
    )
}

#[tokio::test]
async fn a_card_sent_back_once_and_passed_is_laid_out_as_two_rounds_each_with_its_turns_what_they_took_and_what_review_decided()
 {
    let (rig, at) = factory().await;
    sends_back(&at);
    let id = add(&rig, "Add a cart", &[]).await;
    move_card(&rig, &id, "ready", None).await;
    let taken = moved(&rig, &id, "doing", 0).await;
    let back = moved(&rig, &id, "doing", taken).await;
    std::fs::remove_file(at.join("sends-back")).unwrap();
    moved(&rig, &id, "done", back).await;
    // What decided each round is in the card's history once it is said there.
    let done = format!(
        "conductor moved it from review to done, saying: approved by shop-reviewer-1; lane/{id} was merged"
    );
    history(&rig, &id, &done).await;

    let cost = rig.read(&format!("/api/board/items/{id}/cost")).await;
    assert_eq!(cost["card"], id.as_str());
    // Each turn is placed by what is written of its asking, none by the order it was written.
    assert_eq!(cost["by_order"], 0, "{cost:#}");
    let rounds = cost["rounds"].as_array().unwrap();
    let worker = ("shop-worker-1".to_string(), "works".to_string());
    let reviewer = ("shop-reviewer-1".to_string(), "looks".to_string());
    assert_eq!(
        rounds.iter().map(shape).collect::<Vec<_>>(),
        [
            (
                vec![worker.clone(), reviewer.clone()],
                "back".into(),
                "shop-reviewer-1: The total is wrong.".into()
            ),
            (vec![worker, reviewer], "passed".into(), "It does what the card asks.".into()),
        ],
        "{cost:#}"
    );
    assert_eq!(rounds[0]["decided"]["by"], "conductor");
    assert_eq!(rounds[1]["decided"]["by"], "shop-reviewer-1");
    assert_eq!(rounds[1]["decided"]["said"], "shop-reviewer-1 approved it: It does what the card asks.");
    // Each round's turns, as their programs said, and what each round came to.
    let chose: Vec<&Value> = rounds.iter().map(|round| &round["turns"][1]["chose"]).collect();
    assert_eq!(chose, [&json!("send_back"), &json!("approve")]);
    let first = &rounds[0]["spent"];
    assert_eq!(
        (&first["turns"], &first["sent"], &first["wrote"], &first["unsaid"]),
        (&json!(2), &json!(1400), &json!(240), &json!(0))
    );
    assert!((first["cost_usd"].as_f64().unwrap() - 0.27).abs() < 1e-9, "{first}");
    let second = &rounds[1]["spent"];
    assert_eq!((&second["sent"], &second["wrote"]), (&json!(1300), &json!(230)));
    assert!((second["cost_usd"].as_f64().unwrap() - 0.26).abs() < 1e-9, "{second}");
    for round in rounds {
        for one in round["turns"].as_array().unwrap() {
            assert_eq!((&one["status"], &one["says"]), (&json!("done"), &json!(true)), "{one}");
            assert!(one["took_ms"].as_i64().is_some_and(|took| took >= 0), "{one}");
        }
    }
    // And what all of them came to.
    let all = &cost["spent"];
    assert_eq!((&all["turns"], &all["sent"], &all["wrote"]), (&json!(4), &json!(2700), &json!(470)));
    assert!((all["cost_usd"].as_f64().unwrap() - 0.53).abs() < 1e-9, "{all}");
    let took: i64 = rounds.iter().map(|round| round["spent"]["took_ms"].as_i64().unwrap()).sum();
    assert_eq!(all["took_ms"].as_i64().unwrap(), took);
}

#[tokio::test]
async fn what_a_card_cost_is_read_by_whoever_may_read_the_board_and_by_nobody_else() {
    let rig = Rig::owned(Rig::with("api-spent-read", |_, _| {}).await).await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    rig.board("shop").await;
    let id = add(&rig, "Add a cart", &[]).await;
    let path = format!("/api/board/items/{id}/cost");
    // A card nobody has done anything with has no rounds.
    let nothing = rig.read(&path).await;
    assert_eq!(nothing["rounds"], json!([]));
    assert_eq!(nothing["spent"]["turns"], 0);

    // A key that reads the board is told it; one that reads the rest of
    // the project and not the board is not, in the words of the scope.
    let reads = crate::held::key(&rig, &["board:read"]).await;
    let (status, told) = rig.call(Method::GET, &path, Some(&reads), None).await;
    assert_eq!((status, &told["rounds"]), (StatusCode::OK, &json!([])), "{told}");
    let less = crate::held::key(&rig, &["projects:read", "records:read", "agents:read"]).await;
    let (status, refused) = rig.call(Method::GET, &path, Some(&less), None).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    assert!(crate::held::said(&refused).contains("board:read"), "{refused}");
    // And one that is no card is said to be none, as a card's other routes say it.
    let (status, missing) = rig.call(Method::GET, "/api/board/items/shop-99/cost", Some(&reads), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{missing}");
}

#[tokio::test]
async fn a_card_left_for_a_person_says_each_part_of_why_only_where_its_event_says_it_in_its_history_and_in_what_it_cost()
 {
    let rig = Rig::owned(Rig::with("api-spent-left", |_, _| {}).await).await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    rig.board("shop").await;
    let id = add(&rig, "Add a cart", &[]).await;
    // Left as a round leaves a card (`left` of `std/working`): why, a word that says nothing, and
    // no count of how often it was sent back; then as a stage that reviews leaves one, the other way round.
    let why = "none of its 2 contenders' work could be kept (otter unkept, slate unkept)";
    let left = |data: Value| {
        let step = json!({ "do": "emit", "with": data }).to_string();
        format!("blueprint: leaves\nsteps:\n  left: {step}\n")
    };
    for data in [
        json!({ "topic": "card.left", "card": id, "project": "shop", "why": why, "as": "" }),
        json!({ "topic": "card.left", "card": id, "project": "shop", "rounds": 2, "why": "" }),
    ] {
        let said =
            rig.send("/api/apply", json!({ "project": "shop", "source": left(data), "wait": true })).await;
        assert_eq!(said["finished"]["status"], "ok", "{said}");
    }
    let first = format!("it was left for a person: {why}");
    let second = "it was left for a person, having been sent back 2 times";
    history(&rig, &id, &first).await;
    let said = history(&rig, &id, second).await;
    assert!(!said.iter().any(|one| one.contains("back  times")), "{said:?}");
    // What it cost says the same, in the board's words, each round left with why where there is one.
    let cost = rig.read(&format!("/api/board/items/{id}/cost")).await;
    let decided: Vec<(&Value, &Value, &Value)> = cost["rounds"]
        .as_array()
        .unwrap()
        .iter()
        .map(|round| (&round["decided"]["came"], &round["decided"]["said"], &round["decided"]["words"]))
        .collect();
    assert_eq!(
        decided,
        [(&json!("left"), &json!(first), &json!(why)), (&json!("left"), &json!(second), &json!(""))],
        "{cost:#}"
    );
}

//! What a blueprint tells somebody with beside its words (`parts` of
//! `inbox.tell`, `kernel::parts`), read and pressed from their line: a
//! board's cards in one column, each passed or failed where the line is
//! read, a card failed going back to be worked with why, and what was
//! said on it since its agent last had it told to that agent.

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;
use crate::fleets::{add, given, shop_with};
use crate::kept::{asked, key};
use crate::lanes::{history, move_card, moved};

/// A text that tells whom it is handed, with these parts.
const TOLD: &str = r#"blueprint: preview-told
permissions: [inbox]
params:
  to: { type: list }
  parts: { type: list }
steps:
  told:
    do: inbox.tell
    with: { to: "${params.to}", says: "The preview is ready to try.", why: preview, parts: "${params.parts}" }
"#;

/// The fleet: one worker, whose work goes to `staging` and stays there.
fn fleet() -> Value {
    json!({
        "agents": [{ "title": "worker", "role": "worker", "count": 1, "adapter": "writer" }],
        "stages": [{ "title": "work", "column": "ready", "does": "work", "role": "worker", "pass": "staging" }],
    })
}

/// A card made ready, worked, and come to rest in `staging`.
async fn staged(rig: &Rig, title: &str) -> String {
    let id = add(rig, title, &[]).await;
    move_card(rig, &id, "ready", None).await;
    let came = moved(rig, &id, "staging", 0).await;
    crate::lanes::at_rest(rig, came).await;
    id
}

/// `token`'s one line that says the preview is ready: its id.
async fn line(rig: &Rig, token: &str) -> String {
    let (lines, _) = crate::tells::inbox(rig, token).await;
    let found = lines.iter().find(|line| line["why"] == "preview").expect("a line of the preview");
    found["id"].as_str().unwrap().to_string()
}

/// The parts of a line as `token` reads them now.
async fn parts(rig: &Rig, token: &str, line: &str) -> Vec<Value> {
    let (status, read) = rig.call(Method::GET, &format!("/api/inbox/{line}/parts"), Some(token), None).await;
    assert_eq!(status, StatusCode::OK, "{read}");
    read["parts"].as_array().unwrap().clone()
}

/// Presses `verdict` on a card of a line's part: how it went, and every word.
async fn press(
    rig: &Rig,
    token: &str,
    line: &str,
    part: usize,
    card: &str,
    verdict: &str,
    reason: &str,
) -> (StatusCode, String) {
    let path = format!("/api/inbox/{line}/parts/{part}/cards/{card}");
    asked(rig, token, Method::POST, &path, Some(json!({ "verdict": verdict, "reason": reason }))).await
}

#[tokio::test]
async fn a_card_a_told_line_lists_is_passed_or_failed_from_it_and_failed_is_worked_again_told_why_and_what_was_said()
 {
    let (rig, _) = crate::fleets::rig().await;
    let rig = &rig;
    shop_with(rig, &["draft", "ready", "doing", "staging", "done"], fleet()).await;
    // One the factory does not start, which holds its own box and the board.
    let invited = rig.send("/api/agent/invite", json!({ "name": "scout" })).await;
    let joined = rig
        .call(Method::POST, "/api/agent/join", None, Some(json!({ "invitation": invited["invitation"] })))
        .await;
    let scout = joined.1["token"].as_str().expect("an agent that joined").to_string();
    let (cart, shelf, label) =
        (staged(rig, "Fix the cart").await, staged(rig, "Add a shelf").await, staged(rig, "Label it").await);

    // 1: a part that cannot be told with refuses the telling, saying why.
    let wrong = json!({ "to": ["applier"], "parts": [{ "kind": "cards" }] });
    let body = json!({ "source": TOLD, "project": "shop", "key": "wrong", "params": wrong, "wait": true });
    let applied = rig.send("/api/apply", body).await;
    assert_eq!(applied["finished"]["status"], "failed", "{applied}");
    assert!(applied.to_string().contains("say one of the two"), "{applied}");

    // 2: told with a link and the cards in staging, which go to done when
    // passed, and with a second part of the same that says no column.
    let said = json!({
        "to": ["applier", "agent:scout"],
        "parts": [
            { "kind": "links", "links": ["http://127.0.0.1:7760/"] },
            { "kind": "cards", "column": "staging", "pass_to": "done" },
            { "kind": "cards", "column": "staging" },
        ],
    });
    let body = json!({ "source": TOLD, "project": "shop", "key": "told", "params": said, "wait": true });
    let applied = rig.send("/api/apply", body).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
    let mine = line(rig, &rig.token).await;
    let (lines, _) = crate::tells::inbox(rig, &rig.token).await;
    let kept = &lines.iter().find(|one| one["id"] == mine.as_str()).unwrap()["parts"];
    assert_eq!(kept[1]["fail_to"], "ready", "what a part leaves unsaid is kept filled in: {kept}");
    assert_eq!(kept[1]["project"], "shop", "the project it was applied for: {kept}");
    let read = parts(rig, &rig.token, &mine).await;
    assert_eq!(read[0], json!({ "kind": "links", "links": ["http://127.0.0.1:7760/"] }));
    let listed: Vec<&str> =
        read[1]["cards"].as_array().unwrap().iter().map(|card| card["id"].as_str().unwrap()).collect();
    assert_eq!(listed, [cart.as_str(), shelf.as_str(), label.as_str()]);
    assert!(read[1]["cards"].as_array().unwrap().iter().all(|card| card["may"] == true), "{read:?}");

    // 3: something said on a card is its thread there.
    rig.send(&format!("/api/records/{cart}/comments"), json!({ "body": "The total is wrong." })).await;
    history(rig, &cart, "Ada said: The total is wrong.").await;
    let read = parts(rig, &rig.token, &mine).await;
    let thread = &read[1]["cards"][0]["thread"];
    assert_eq!(
        (thread.as_array().unwrap().len(), &thread[0]["words"]),
        (1, &json!("The total is wrong.")),
        "{read:?}"
    );

    // 4: failed, it asks why first; then it goes to ready with why, and is
    // worked again by its agent, which is told why and what was said.
    let refused = press(rig, &rig.token, &mine, 2, &cart, "fail", "  ").await;
    assert_eq!(refused.0, StatusCode::BAD_REQUEST, "{}", refused.1);
    assert!(refused.1.contains("is failed here only with why"), "{}", refused.1);
    let before = rig.mark();
    let failed = press(rig, &rig.token, &mine, 2, &cart, "fail", "The cart forgets the tax.").await;
    assert_eq!(failed.0, StatusCode::OK, "{}", failed.1);
    moved(rig, &cart, "ready", before).await;
    history(rig, &cart, "Ada moved it from staging to ready, saying: The cart forgets the tax.").await;
    let again = moved(rig, &cart, "staging", before).await;
    crate::lanes::at_rest(rig, again).await;
    let prompts = given(rig, "shop-worker-1").await;
    let last = prompts.last().unwrap();
    assert!(last.contains("This card was sent back") && last.contains("The cart forgets the tax."), "{last}");
    assert!(
        last.contains("What was said on the card since you last had it:\n\n- Ada: The total is wrong."),
        "{last}"
    );
    assert_eq!(last.matches("The cart forgets the tax.").count(), 1, "why is told once: {last}");
    // Worked a second time with nothing said since, it is told nothing of it.
    rig.send(&format!("/api/records/{cart}/comments"), json!({ "body": "Better." })).await;
    let before = rig.mark();
    let failed = press(rig, &rig.token, &mine, 2, &cart, "fail", "Still the tax.").await;
    assert_eq!(failed.0, StatusCode::OK, "{}", failed.1);
    let again = moved(rig, &cart, "staging", before).await;
    crate::lanes::at_rest(rig, again).await;
    let last = given(rig, "shop-worker-1").await.pop().unwrap();
    assert!(last.contains("- Ada: Better.") && !last.contains("The total is wrong."), "{last}");

    // 5: passed, it is approved by whoever passed it, a line of its history
    // and nothing said on it, and then goes where the part says; with no
    // column, it stays.
    let passed = press(rig, &rig.token, &mine, 2, &shelf, "pass", "").await;
    assert_eq!(passed.0, StatusCode::OK, "{}", passed.1);
    assert!(
        passed.1.contains("\"approved\":true") && passed.1.contains("\"moved\":\"done\""),
        "{}",
        passed.1
    );
    history(rig, &shelf, "Ada approved it").await;
    assert_eq!(crate::fleets::card(rig, &shelf).await["status"], "done");
    let passed = press(rig, &rig.token, &mine, 3, &label, "pass", "Looks right.").await;
    assert_eq!(passed.0, StatusCode::OK, "{}", passed.1);
    history(rig, &label, "Ada approved it").await;
    assert_eq!(crate::fleets::card(rig, &label).await["status"], "staging");
    let kept = rig.read(&format!("/api/records/{label}/history")).await;
    let said: Vec<&Value> = kept["history"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|line| line["topic"] == "record.commented")
        .collect();
    assert!(said.is_empty(), "no words are said on a card for passing it: {said:?}");
    // A card that is no longer where the part lists it is not pressed there.
    let gone = press(rig, &rig.token, &mine, 2, &shelf, "fail", "Late.").await;
    assert!(gone.1.contains("is not one of the cards in staging any more: it is in done"), "{}", gone.1);
    let none = press(rig, &rig.token, &mine, 1, &label, "pass", "").await;
    assert_eq!(none.0, StatusCode::NOT_FOUND, "a part of links lists no cards: {}", none.1);

    // 6: an agent does not pass or fail its own work, and is offered none.
    let theirs = line(rig, &scout).await;
    let given = json!({ "assignee": "agent:scout" });
    let (status, answer) =
        rig.call(Method::PATCH, &format!("/api/board/items/{label}"), Some(&rig.token), Some(given)).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    let read = parts(rig, &scout, &theirs).await;
    let card = read[2]["cards"]
        .as_array()
        .unwrap()
        .iter()
        .find(|card| card["id"] == label.as_str())
        .unwrap()
        .clone();
    assert_eq!(card["may"], false, "{card}");
    assert!(card["why_not"].as_str().unwrap().contains("scout worked on"), "{card}");
    let own = press(rig, &scout, &theirs, 3, &label, "fail", "Mine.").await;
    assert!(own.1.contains("an agent does not pass or fail its own work"), "{}", own.1);
    assert_eq!(crate::fleets::card(rig, &label).await["status"], "staging");

    // 7: a key that may read the box and not change the board is shown the
    // cards, offered nothing, and refused; another's line is not there.
    let narrow = key(rig, &rig.token, &["inbox", "board:read"]).await;
    let read = parts(rig, &narrow, &mine).await;
    assert!(read[1]["cards"].as_array().unwrap().iter().all(|card| card["may"] == false), "{read:?}");
    let refused = press(rig, &narrow, &mine, 3, &label, "pass", "").await;
    assert_eq!(refused.0, StatusCode::FORBIDDEN, "{}", refused.1);
    let blind = key(rig, &rig.token, &["inbox"]).await;
    let read = parts(rig, &blind, &mine).await;
    assert!(
        read[1]["cards"].is_null() && read[1]["refused"].as_str().unwrap().contains("board:read"),
        "{read:?}"
    );
    let (status, _) =
        rig.call(Method::GET, &format!("/api/inbox/{theirs}/parts"), Some(&rig.token), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

/// A text that tells its applier about `about`, with the cards of staging.
const KEPT: &str = r#"blueprint: preview-kept
permissions: [inbox]
params:
  about: { type: string }
steps:
  told:
    do: inbox.tell
    with:
      to: applier
      says: "The preview is ready to try."
      why: preview
      about: "${params.about}"
      parts: [{ kind: cards, column: staging, pass_to: done }]
"#;

/// The ids of the cards a part lists, with where each is, or `gone`.
fn where_each(part: &Value) -> Vec<(String, String)> {
    let cards = part["cards"].as_array().unwrap_or_else(|| panic!("{part}"));
    let said = |card: &Value| match card["gone"] == true {
        true => "gone".to_string(),
        false => card["status"].as_str().unwrap().to_string(),
    };
    cards.iter().map(|card| (card["id"].as_str().unwrap().to_string(), said(card))).collect()
}

#[tokio::test]
async fn a_told_line_keeps_the_cards_its_column_held_and_lists_each_where_it_is_now_or_gone() {
    let rig = &Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    let columns = json!({ "columns": ["ready", "staging", "done"] });
    let board = json!({ "blueprint": "std/board", "project": "shop", "params": columns, "wait": true });
    assert_eq!(rig.send("/api/apply", board).await["finished"]["status"], "ok");
    let mut cards = Vec::new();
    for title in ["Fix the cart", "Add a shelf", "Label it", "Paint it"] {
        let card = rig.send("/api/board/shop/items", json!({ "title": title })).await;
        cards.push(card["id"].as_str().unwrap().to_string());
    }
    let [cart, shelf, label, paint] = [&cards[0], &cards[1], &cards[2], &cards[3]];
    for card in [cart, shelf, label] {
        move_card(rig, card, "staging", None).await;
    }
    let tell = |key: &str| {
        let body = json!({ "source": KEPT, "project": "shop", "key": key, "params": { "about": "build-1" }, "wait": true });
        async move { assert_eq!(rig.send("/api/apply", body).await["finished"]["status"], "ok") }
    };

    // 1: told, the line keeps the three, which it lists; one that comes
    // to staging after is no card of it, and is not pressed from there.
    tell("first").await;
    let mine = line(rig, &rig.token).await;
    move_card(rig, paint, "staging", None).await;
    let staged = |cards: &[&String]| {
        cards.iter().map(|id| (id.to_string(), "staging".to_string())).collect::<Vec<_>>()
    };
    let read = parts(rig, &rig.token, &mine).await;
    assert_eq!(where_each(&read[0]), staged(&[cart, shelf, label]));
    assert_eq!(read[0]["kept"], true, "{read:?}");
    assert!(read[0]["cards"].as_array().unwrap().iter().all(|card| card["may"] == true), "{read:?}");
    let refused = press(rig, &rig.token, &mine, 1, paint, "pass", "").await;
    assert_eq!(refused.0, StatusCode::BAD_REQUEST, "{}", refused.1);
    assert!(refused.1.contains("is not one of the cards this line was told of"), "{}", refused.1);
    // Handed out with the line, the part says none of its cards.
    let (lines, _) = crate::tells::inbox(rig, &rig.token).await;
    let handed = &lines.iter().find(|one| one["id"] == mine.as_str()).unwrap()["parts"][0];
    assert!(handed["kept"].is_null() && handed["column"] == "staging", "{handed}");

    // 2: passed, a card goes to done and is still listed there, offered
    // nothing; one taken away is said to be gone, where it was.
    let passed = press(rig, &rig.token, &mine, 1, cart, "pass", "").await;
    assert_eq!(passed.0, StatusCode::OK, "{}", passed.1);
    let (status, taken) =
        rig.call(Method::DELETE, &format!("/api/board/items/{shelf}"), Some(&rig.token), None).await;
    assert_eq!(status, StatusCode::OK, "{taken}");
    let read = parts(rig, &rig.token, &mine).await;
    let now = [
        (cart.to_string(), "done".into()),
        (shelf.to_string(), "gone".into()),
        (label.to_string(), "staging".into()),
    ];
    assert_eq!(where_each(&read[0]), now);
    let listed = read[0]["cards"].as_array().unwrap();
    assert_eq!(listed.iter().map(|card| card["may"] == true).collect::<Vec<_>>(), [false, false, true]);
    assert!(listed[0]["why_not"].as_str().unwrap().contains("it is in done"), "{}", listed[0]);
    assert!(listed[1]["why_not"].as_str().unwrap().contains("no longer on the board"), "{}", listed[1]);
    let late = press(rig, &rig.token, &mine, 1, cart, "fail", "Late.").await;
    assert!(late.1.contains("is not one of the cards in staging any more: it is in done"), "{}", late.1);

    // 3: told again of the same thing, the line is raised and keeps the column as it is then.
    tell("again").await;
    assert_eq!(line(rig, &rig.token).await, mine, "the line there is is raised");
    let read = parts(rig, &rig.token, &mine).await;
    assert_eq!(where_each(&read[0]), staged(&[label, paint]));

    // 4: a key that may read the box and not the board is shown none of them, and told why.
    let blind = key(rig, &rig.token, &["inbox"]).await;
    let read = parts(rig, &blind, &mine).await;
    assert!(
        read[0]["cards"].is_null() && read[0]["refused"].as_str().unwrap().contains("board:read"),
        "{read:?}"
    );

    // 5: a line told before cards were kept lists what its column holds when it is read.
    let id = mine.clone();
    rig.node()
        .app
        .journal
        .db()
        .call(move |connection| {
            connection
                .execute("UPDATE inbox SET parts = json_remove(parts, '$[0].kept') WHERE id = ?1", [&id])
        })
        .await
        .unwrap();
    move_card(rig, label, "done", None).await;
    let read = parts(rig, &rig.token, &mine).await;
    assert_eq!(read[0]["kept"], false, "{read:?}");
    assert_eq!(where_each(&read[0]), staged(&[paint]));
}

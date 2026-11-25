//! A card's priority is a choice on the board that ships (`urgent`,
//! `high`, `normal`, `low`, nothing standing for `normal`), where it was a
//! number: a board from before takes the numbers its cards hold as words,
//! once, each said in the card's history; a number written now is the
//! same word; and what a pool goes by is where the word stands among what
//! the board offers, in the order they stand in now (`kernel::ranked`).

use reqwest::Method;
use serde_json::{Value, json};

use crate::Rig;

/// The board as a program from before declared it, its priority a number,
/// kept where `store` says.
fn before(store: &str) -> String {
    format!(
        "blueprint: before
permissions: [projects, board]
steps:
  board:
    do: collection.ensure
    with:
      name: board
      store: {store}
      model: card
      fields:
        title: {{ type: text, required: true }}
        status: {{ type: choice, of: [backlog, ready, doing, review, done], required: true }}
        priority: {{ type: number }}
      views:
        board: {{ kind: board, by: status }}
"
    )
}

/// Where each card's choices stand, as `board.get` gives them.
const RANKED: &str = "blueprint: ranked
permissions: [board]
params:
  card: { type: string }
steps:
  card:
    do: board.get
    with: { id: \"${params.card}\" }
outputs:
  ranked: ${ steps.card.out.ranked.priority }
";

async fn applied(rig: &Rig, body: Value) -> Value {
    let answer = rig.send("/api/apply", body).await;
    assert_eq!(answer["finished"]["status"], "ok", "{answer}");
    answer
}

/// Where a card's priority stands, as a pool is told it.
async fn rank(rig: &Rig, card: &str) -> Value {
    let body = json!({ "source": RANKED, "project": "shop", "params": { "card": card }, "wait": true });
    applied(rig, body).await["finished"]["outputs"]["ranked"].clone()
}

/// What each card holds of its priority, by its id.
async fn held(rig: &Rig) -> Vec<(String, Value)> {
    let cards = rig.read("/api/board/shop").await;
    let items = cards["items"].as_array().unwrap();
    items
        .iter()
        .map(|card| (card["id"].as_str().unwrap().to_string(), card["more"]["priority"].clone()))
        .collect()
}

#[tokio::test]
async fn a_board_from_before_takes_the_numbers_its_cards_hold_as_words_once_each_said_in_its_history() {
    for store in ["local", "files"] {
        let rig = Rig::new().await;
        rig.send("/api/projects", json!({ "name": "Shop" })).await;
        applied(&rig, json!({ "source": before(store), "project": "shop", "wait": true })).await;
        let mut ids = Vec::new();
        for (title, priority) in
            [("Cart", json!(2)), ("Till", json!(-1)), ("Shelf", json!(0)), ("Price", json!(0.5))]
        {
            let card = rig
                .send("/api/board/shop/items", json!({ "title": title, "fields": { "priority": priority } }))
                .await;
            ids.push(card["id"].as_str().unwrap().to_string());
        }
        let plain = rig.send("/api/board/shop/items", json!({ "title": "Basket" })).await;
        let plain = plain["id"].as_str().unwrap().to_string();

        // The board that ships, applied over them: each number is the
        // word as far from `normal` as it was from nought, the first or
        // the last where it went past them.
        let board = json!({ "blueprint": "std/board", "project": "shop", "params": { "store": store }, "wait": true });
        applied(&rig, board.clone()).await;
        let words = [json!("urgent"), json!("low"), json!("normal"), json!("high")];
        let mut meant: Vec<(String, Value)> = ids.iter().cloned().zip(words).collect();
        meant.push((plain.clone(), Value::Null));
        assert_eq!(held(&rig).await, meant, "{store}");

        // Each card's history says so, by the factory's bot; the one that
        // held nothing says nothing of it.
        for (id, was, now) in [(&ids[0], "2", "urgent"), (&ids[1], "-1", "low"), (&ids[3], "0.5", "high")] {
            let line = format!("its priority, the number {was}, came to be {now}");
            crate::lanes::history(&rig, id, &line).await;
            let lines = rig.read(&format!("/api/records/{id}/history")).await;
            let line = lines["history"]
                .as_array()
                .unwrap()
                .iter()
                .find(|one| one["said"] == line.as_str())
                .unwrap()
                .clone();
            assert_eq!(line["who"], "conductor", "{store}: {line}");
        }
        let lines = rig.read(&format!("/api/records/{plain}/history")).await;
        assert!(!lines.to_string().contains("came to be"), "{store}: {lines}");
        assert_eq!(rig.count_of("record.reworded").await, 4, "{store}");

        // Once: its steps run again, nothing is said or changed.
        let mut again = board;
        again["force"] = json!(true);
        applied(&rig, again).await;
        assert_eq!(held(&rig).await, meant, "{store}");
        assert_eq!(rig.count_of("record.reworded").await, 4, "{store}");

        // A number written now is the same word, and a word is a word.
        let now =
            rig.send("/api/board/shop/items", json!({ "title": "Bag", "fields": { "priority": 1 } })).await;
        assert_eq!(now["more"]["priority"], "high", "{store}: {now}");
        let (status, wrong) = rig
            .call(
                Method::POST,
                "/api/board/shop/items",
                Some(&rig.token),
                Some(json!({ "title": "Box", "fields": { "priority": "soon" } })),
            )
            .await;
        assert_eq!(status, reqwest::StatusCode::BAD_REQUEST, "{store}: {wrong}");
    }
}

#[tokio::test]
async fn a_cards_priority_ranks_by_where_its_word_stands_among_what_the_board_offers_as_they_stand_now() {
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    applied(&rig, json!({ "blueprint": "std/board", "project": "shop", "wait": true })).await;
    let mut ids = Vec::new();
    for priority in [json!("urgent"), json!("low"), Value::Null] {
        let card = rig
            .send("/api/board/shop/items", json!({ "title": "Cart", "fields": { "priority": priority } }))
            .await;
        ids.push(card["id"].as_str().unwrap().to_string());
    }
    // The first word the highest, and nothing where `normal` is.
    let mut ranks = Vec::new();
    for id in &ids {
        ranks.push(rank(&rig, id).await);
    }
    assert_eq!(ranks, [json!(4), json!(1), json!(2)]);

    // Somebody puts the words in another order, and the ranks follow it.
    let order = json!({ "of": ["low", "normal", "high", "urgent"] });
    let (status, said) = rig
        .call(Method::PATCH, "/api/collections/shop/board/fields/priority", Some(&rig.token), Some(order))
        .await;
    assert_eq!(status, reqwest::StatusCode::OK, "{said}");
    let mut ranks = Vec::new();
    for id in &ids {
        ranks.push(rank(&rig, id).await);
    }
    assert_eq!(ranks, [json!(1), json!(4), json!(3)]);
}

#[tokio::test]
async fn a_choice_says_as_what_nothing_stands_for_only_a_word_it_offers() {
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    let declared = |unsaid: &str| {
        format!(
            "blueprint: notes\npermissions: [projects, records]\nsteps:\n  notes: {{ do: collection.ensure, with: {{ name: notes, fields: {{ title: {{ type: text }}, mood: {{ type: choice, of: [glad, sad], unsaid: {unsaid} }} }} }} }}\n"
        )
    };
    let refused =
        rig.send("/api/apply", json!({ "source": declared("calm"), "project": "shop", "wait": true })).await;
    assert_eq!(refused["finished"]["status"], "failed", "{refused}");
    let why = refused["finished"]["error"].as_str().unwrap();
    assert!(why.contains("for a choice is the one word of `of` that nothing chosen stands for"), "{why}");
    applied(&rig, json!({ "source": declared("sad"), "project": "shop", "wait": true })).await;
}

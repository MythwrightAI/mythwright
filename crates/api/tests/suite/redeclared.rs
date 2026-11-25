//! A blueprint applied again with other fields for a collection that has
//! records, from the outside: what is there can still be written, or the
//! blueprint is told why it cannot say so.

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;

/// A board whose cards each have `size`, declared as `size` says, kept
/// where `store` says.
fn board(store: &str, size: &str) -> String {
    format!(
        "blueprint: plan
permissions: [projects, board]
steps:
  board:
    do: collection.ensure
    with:
      name: board
      store: {store}
      fields:
        title: {{ type: text, required: true }}
        body: {{ type: text }}
        status: {{ type: choice, of: [todo, done], required: true }}
        labels: {{ type: list }}
{size}      views:
        board: {{ kind: board, by: status }}
"
    )
}

async fn applied(rig: &Rig, source: &str) -> Value {
    rig.send("/api/apply", json!({ "source": source, "project": "shop", "wait": true })).await
}

#[tokio::test]
async fn a_board_that_needs_more_of_a_card_is_given_it_as_the_card_is_added_and_loses_no_card_when_it_no_longer_does()
 {
    for store in ["local", "files"] {
        let rig = Rig::new().await;
        rig.send("/api/projects", json!({ "name": "Shop" })).await;
        let needed = board(store, "        size: { type: number, required: true }\n");
        assert_eq!(applied(&rig, &needed).await["finished"]["status"], "ok", "{store}");

        let add = |card: Value| rig.call(Method::POST, "/api/board/shop/items", Some(&rig.token), Some(card));
        let (status, bare) = add(json!({ "title": "Basket" })).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{store}: {bare}");
        assert_eq!(bare["error"]["message"], "`size` is needed", "{store}");
        let (status, basket) = add(json!({ "title": "Basket", "fields": { "size": 3 } })).await;
        assert_eq!(status, StatusCode::OK, "{store}: {basket}");
        assert_eq!((&basket["id"], &basket["more"]), (&json!("shop-1"), &json!({ "size": 3 })), "{store}");
        let (status, own) = add(json!({ "title": "Till", "fields": { "size": 1, "title": "Other" } })).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{store}: {own}");
        assert!(own["error"]["message"].as_str().unwrap().starts_with("`title` is one of a card's own"));

        // A blueprint's step says the same thing among `fields`.
        let step = "blueprint: stock\npermissions: [board]\nsteps:\n  add:\n    do: board.add\n    with: { title: Checkout, fields: { size: 5 } }\n";
        let added = applied(&rig, step).await;
        assert_eq!(added["finished"]["status"], "ok", "{store}: {added}");
        let cards = rig.read("/api/board/shop").await;
        assert_eq!(cards["items"][1]["more"], json!({ "size": 5 }), "{store}: {cards}");

        // It cannot become words while cards hold numbers of it.
        let worded = board(store, "        size: { type: choice, of: [small, big] }\n");
        let refused = applied(&rig, &worded).await;
        assert_eq!(refused["finished"]["status"], "failed", "{store}: {refused}");
        assert!(
            refused["finished"]["error"].as_str().unwrap().contains(
                "`size` was a `number` and is now declared a `choice`, which is not what 2 record(s) of `board` hold of it"
            ),
            "{store}: {refused}"
        );

        // Declared without it, every card is still moved and changed.
        assert_eq!(applied(&rig, &board(store, "")).await["finished"]["status"], "ok", "{store}");
        let change = json!({ "status": "done", "title": "Trolley" });
        let (status, moved) =
            rig.call(Method::PATCH, "/api/board/items/shop-1", Some(&rig.token), Some(change)).await;
        assert_eq!(status, StatusCode::OK, "{store}: {moved}");
        assert_eq!((&moved["status"], &moved["title"]), (&json!("done"), &json!("Trolley")), "{store}");
        assert_eq!(moved.get("more"), None, "{store}: {moved}");
        let (status, stray) = rig
            .call(Method::PATCH, "/api/board/items/shop-2", Some(&rig.token), Some(json!({ "size": 8 })))
            .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{store}: {stray}");
    }
}

/// Shelves, each holding what `holds` lets it.
fn shelves(holds: &str) -> String {
    format!(
        "blueprint: shelves
permissions: [projects, records]
steps:
  shelves:
    do: collection.ensure
    with:
      name: shelves
      fields:
        title: {{ type: text, required: true }}
        holds: {holds}
"
    )
}

#[tokio::test]
async fn a_choice_made_free_text_keeps_what_its_records_chose_and_an_option_taken_from_a_choice_still_in_use_is_refused()
 {
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    assert_eq!(
        applied(&rig, &shelves("{ type: choice, of: [tins, jars] }")).await["finished"]["status"],
        "ok"
    );
    let top = rig
        .send(
            "/api/collections/shop/shelves/records",
            json!({ "fields": { "title": "Top", "holds": "tins" } }),
        )
        .await;
    let id = top["id"].as_str().unwrap().to_string();

    // Still a choice, and no longer one of them: the record would hold
    // what nobody could choose or see, so it is refused, as it was.
    let fewer = applied(&rig, &shelves("{ type: choice, of: [jars] }")).await;
    assert_eq!(fewer["finished"]["status"], "failed", "{fewer}");
    let why = fewer["finished"]["error"].as_str().unwrap();
    assert!(why.contains("`tins` is still the `holds` of 1 record(s) of `shelves`"), "{why}");

    // No longer a choice at all: it takes any word, the ones it had among
    // them, and what a record chose is what it holds.
    let free = applied(&rig, &shelves("{ type: text }")).await;
    assert_eq!(free["finished"]["status"], "ok", "{free}");
    assert_eq!(rig.read(&format!("/api/records/{id}")).await["data"]["holds"], "tins");
    let (status, changed) = rig
        .call(
            Method::PATCH,
            &format!("/api/records/{id}"),
            Some(&rig.token),
            Some(json!({ "fields": { "holds": "boxes" } })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{changed}");
    assert_eq!(rig.read(&format!("/api/records/{id}")).await["data"]["holds"], "boxes");
}

#[tokio::test]
async fn a_board_from_before_is_given_its_order_when_what_ships_is_applied_again_and_keeps_what_people_saved()
{
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    // A board as a text from before this declared it: its cards, what
    // each waits on and what it is part of, and one view, the board.
    let before = "blueprint: std-board-before
permissions: [projects, board]
steps:
  board:
    do: collection.ensure
    with:
      name: board
      model: card
      fields:
        title: { type: text, required: true }
        status: { type: choice, of: [backlog, ready, doing, review, done], required: true }
        part_of: { type: text, quiet: true }
        after: { type: list }
      views:
        board: { kind: board, by: status }
";
    let made = applied(&rig, before).await;
    assert_eq!(made["finished"]["status"], "ok", "{made}");
    let add = |card: Value| rig.call(Method::POST, "/api/board/shop/items", Some(&rig.token), Some(card));
    add(json!({ "title": "Floor" })).await;
    let (status, shelves) =
        add(json!({ "title": "Shelves", "fields": { "after": ["shop-1"], "part_of": "shop-features-1" } }))
            .await;
    assert_eq!(status, StatusCode::OK, "{shelves}");
    // Somebody saved a view of their own beside it, and gathered the board by who has each card.
    let save = |view: Value| {
        rig.call(Method::POST, "/api/collections/shop/board/views", Some(&rig.token), Some(view))
    };
    let (status, saved) = save(json!({ "name": "Mine", "kind": "table" })).await;
    assert_eq!(status, StatusCode::OK, "{saved}");
    let (status, saved) =
        save(json!({ "name": "board", "kind": "board", "by": "status", "group": "assignee" })).await;
    assert_eq!(status, StatusCode::OK, "{saved}");

    // What ships now is applied over it.
    let now =
        rig.send("/api/apply", json!({ "blueprint": "std/board", "project": "shop", "wait": true })).await;
    assert_eq!(now["finished"]["status"], "ok", "{now}");
    let tables = rig.read("/api/collections?project=shop").await;
    let board = tables["collections"].as_array().unwrap().iter().find(|one| one["name"] == "board").unwrap();
    let view =
        |name: &str| board["views"].as_array().unwrap().iter().find(|one| one["name"] == name).cloned();
    assert_eq!(
        view("order"),
        Some(
            json!({ "name": "order", "kind": "order", "by": "after", "saved": false, "group": "part_of", "done": "status" })
        ),
        "the order by what comes first is there, as what ships says it"
    );
    let theirs = view("board").unwrap();
    assert_eq!((&theirs["group"], &theirs["saved"]), (&json!("assignee"), &json!(true)), "{theirs}");
    assert!(view("Mine").is_some(), "{board}");
    // And the cards are as they were: what each waits on and what it is part of, as text.
    let cards = rig.read("/api/board/shop").await;
    let shelves = cards["items"].as_array().unwrap().iter().find(|one| one["title"] == "Shelves").unwrap();
    assert_eq!(shelves["more"]["after"], json!(["shop-1"]), "{shelves}");
    assert_eq!(shelves["more"]["part_of"], "shop-features-1", "{shelves}");

    // An order is saved by whoever looks at the board too, and refused where it goes by nothing that says what comes before.
    let (status, saved) =
        save(json!({ "name": "Steps", "kind": "order", "by": "after", "group": "part_of" })).await;
    assert_eq!(status, StatusCode::OK, "{saved}");
    let (status, refused) = save(json!({ "name": "Wrong", "kind": "order", "by": "title" })).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");
    assert!(
        refused["error"]["message"].as_str().unwrap().contains("is an order and needs `by`"),
        "{refused}"
    );
}

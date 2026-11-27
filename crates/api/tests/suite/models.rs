//! A board is held to what a card is when its blueprint is applied, from
//! the outside: a blueprint that says otherwise is refused with the field,
//! one that leaves out what a card can do without still has it, and one
//! written before a blueprint could name a model works as it did.

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;

/// A blueprint that makes a board with these lines about it.
fn board(about: &str) -> String {
    format!(
        "blueprint: cards
permissions: [projects, board]
steps:
  board:
    do: collection.ensure
    with:
      name: board
{about}
      views:
        board: {{ kind: board, by: status }}
"
    )
}

async fn applied(rig: &Rig, source: &str) -> Value {
    rig.send("/api/apply", json!({ "source": source, "project": "shop", "wait": true })).await
}

/// What the project's board is declared as, where it has one.
async fn declared(rig: &Rig) -> Option<Value> {
    let listed = rig.read("/api/collections?project=shop").await;
    listed["collections"].as_array().unwrap().iter().find(|one| one["name"] == "board").cloned()
}

fn names(collection: &Value) -> Vec<&str> {
    collection["fields"].as_array().unwrap().iter().map(|field| field["name"].as_str().unwrap()).collect()
}

#[tokio::test]
async fn a_board_whose_blueprint_says_a_card_is_something_else_is_refused_when_it_is_applied() {
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;

    let as_text = board(
        "      model: card
      fields:
        title: { type: text, required: true }
        status: { type: text, required: true }",
    );
    let refused = applied(&rig, &as_text).await;
    assert_eq!(refused["finished"]["status"], "failed", "{refused}");
    let why = refused["finished"]["error"].as_str().unwrap();
    assert!(
        why.contains("`board` is a `card`, whose `status` is a `choice` that is needed; it is declared here as a `text` that is needed"),
        "{why}"
    );
    assert_eq!(declared(&rig).await, None, "nothing was made");

    let unknown = board(
        "      model: sprint
      fields:
        title: { type: text, required: true }",
    );
    let refused = applied(&rig, &unknown).await;
    let why = refused["finished"]["error"].as_str().unwrap();
    assert!(why.contains("`sprint` is not a model a collection can be; there is: card"), "{why}");
}

#[tokio::test]
async fn a_board_whose_blueprint_leaves_out_what_a_card_can_do_without_has_it_all_the_same() {
    for store in ["local", "files"] {
        let rig = Rig::new().await;
        rig.send("/api/projects", json!({ "name": "Shop" })).await;
        let short = board(&format!(
            "      model: card
      store: {store}
      fields:
        title: {{ type: text, required: true }}
        status: {{ type: choice, of: [todo, done], required: true }}
        size: {{ type: number }}"
        ));
        let made = applied(&rig, &short).await;
        assert_eq!(made["finished"]["status"], "ok", "{store}: {made}");

        let is = declared(&rig).await.unwrap();
        assert_eq!(is["model"], "card", "{store}");
        assert_eq!(
            names(&is),
            [
                "size", "status", "title", "body", "assignee", "labels", "due", "moved", "pressed", "sent",
                "given"
            ],
            "{store}"
        );

        let card =
            rig.send("/api/board/shop/items", json!({ "title": "Basket", "labels": ["urgent"] })).await;
        let id = card["id"].as_str().unwrap();
        let change = json!({ "due": "2026-11-30", "assignee": "agent:builder", "size": 3, "status": "done" });
        let (status, changed) =
            rig.call(Method::PATCH, &format!("/api/board/items/{id}"), Some(&rig.token), Some(change)).await;
        assert_eq!(status, StatusCode::OK, "{store}: {changed}");
        assert_eq!(
            (&changed["due"], &changed["assignee"], &changed["labels"], &changed["status"], &changed["more"]),
            (
                &json!("2026-11-30"),
                &json!("agent:builder"),
                &json!(["urgent"]),
                &json!("done"),
                &json!({ "size": 3 })
            ),
            "{store}: {changed}"
        );
        let board = rig.read("/api/board/shop").await;
        assert_eq!(board["items"][0], changed, "{store}");
    }
}

#[tokio::test]
async fn a_board_whose_blueprint_was_written_before_one_could_name_a_model_works_as_it_did() {
    let mut rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    // The fields a board had, in full, and no word of a model.
    let before = board(
        "      fields:
        title: { type: text, required: true }
        body: { type: text }
        status: { type: choice, of: [todo, doing, done], required: true }
        assignee: { type: actor }
        labels: { type: list }
        due: { type: date }",
    );
    let made = applied(&rig, &before).await;
    assert_eq!(made["finished"]["status"], "ok", "{made}");
    assert_eq!(declared(&rig).await.unwrap()["model"], "card", "a board is a card's without saying so");

    let card = rig.send("/api/board/shop/items", json!({ "title": "Basket", "body": "With handles." })).await;
    let id = card["id"].as_str().unwrap().to_string();
    let moving = json!({ "status": "doing", "note": "Starting on it." });
    let (status, moved) =
        rig.call(Method::PATCH, &format!("/api/board/items/{id}"), Some(&rig.token), Some(moving)).await;
    assert_eq!(status, StatusCode::OK, "{moved}");
    assert_eq!((&moved["status"], &moved["body"]), (&json!("doing"), &json!("With handles.")));

    // Applied again as it is (nothing changed, so nothing runs), and after
    // the daemon has been stopped and started, it is the same board with
    // the same card on it.
    applied(&rig, &before).await;
    rig.restart().await;
    let board = rig.read("/api/board/shop").await;
    assert_eq!(board["columns"], json!(["todo", "doing", "done"]));
    assert_eq!(
        (&board["items"][0]["id"], &board["items"][0]["title"], &board["items"][0]["status"]),
        (&json!(id), &json!("Basket"), &json!("doing"))
    );

    // The standard board says which model its cards are, and is the same thing.
    let standard =
        rig.send("/api/apply", json!({ "blueprint": "std/board", "project": "shop", "wait": true })).await;
    assert_eq!(standard["finished"]["status"], "ok", "{standard}");
    let is = declared(&rig).await.unwrap();
    assert_eq!(is["model"], "card");
    // A card's seven, and what the standard board adds of its own, which
    // is no part of the model: the cards it waits on, what is asked on it
    // and who asked, how often it was sent back, what it waits for, how soon it is to be worked,
    // the cards whose work it puts right,
    // and what a stage keeps of a card it works (where it took it from,
    // what its agent is next to be told, and where in its round it asked).
    assert_eq!(
        names(&is),
        [
            "after", "again", "asker", "assignee", "at", "body", "due", "fixes", "given", "labels", "moved",
            "part_of", "plan", "pressed", "priority", "question", "rounds", "rung", "sent", "status",
            "taken", "title", "waiting"
        ]
    );
}

/// What a card's row says of how it came to be where it is.
async fn came(rig: &Rig, id: &str) -> Value {
    rig.read(&format!("/api/records/{id}")).await["data"]["moved"].clone()
}

/// Moves a card as its owner, saying something as it goes.
async fn move_saying(rig: &Rig, id: &str, moving: Value) {
    let (status, moved) =
        rig.call(Method::PATCH, &format!("/api/board/items/{id}"), Some(&rig.token), Some(moving)).await;
    assert_eq!(status, StatusCode::OK, "{moved}");
}

#[tokio::test]
async fn a_cards_row_says_where_it_was_moved_from_by_what_kind_of_actor_what_was_said_and_how_often() {
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    rig.board("shop").await;
    let made = rig.send("/api/board/shop/items", json!({ "title": "Basket" })).await;
    let id = made["id"].as_str().unwrap().to_string();
    assert_eq!(came(&rig, &id).await, json!([]), "a card nobody has moved says nothing of a move");

    // A person moves it and says why: the row says so in the write that
    // moved it, whoever has heard of the move by then.
    move_saying(&rig, &id, json!({ "status": "ready", "note": "  It is wanted this week. " })).await;
    assert_eq!(came(&rig, &id).await, json!(["draft", "user", "It is wanted this week.", "1"]));

    // A blueprint's step moves it on, saying nothing: counted, and what
    // was said before is not said of this move.
    let on = format!(
        "blueprint: on\npermissions: [board]\nsteps:\n  on: {{ do: board.move, with: {{ id: {id}, to: doing }} }}\n"
    );
    assert_eq!(applied(&rig, &on).await["finished"]["status"], "ok");
    assert_eq!(came(&rig, &id).await, json!(["ready", "blueprint", "", "2"]));

    // Put above another card in the column it is in, it has not been
    // moved from anywhere, and its row says what it said.
    let other = rig.send("/api/board/shop/items", json!({ "title": "Till", "status": "doing" })).await;
    move_saying(&rig, &id, json!({ "status": "doing", "before": other["id"] })).await;
    assert_eq!(came(&rig, &id).await, json!(["ready", "blueprint", "", "2"]));
}

#[tokio::test]
async fn a_board_declared_before_a_card_said_how_it_came_takes_it_with_its_cards_as_they_were() {
    let mut rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    rig.board("shop").await;
    let made = rig.send("/api/board/shop/items", json!({ "title": "Basket" })).await;
    let id = made["id"].as_str().unwrap().to_string();
    // As a program from before left it: the board is declared with no
    // such field, and its card's row has none.
    // In the database that keeps the board and its cards.
    let db = rig.node().app.journal.records_of(Some("shop")).db().clone();
    let rows = db
        .call(|connection| {
            let board = "project = 'shop' AND name = 'board'";
            let fields: String = connection.query_row(
                &format!("SELECT fields FROM collections WHERE {board}"),
                [],
                |row| row.get(0),
            )?;
            let mut fields: Vec<Value> = serde_json::from_str(&fields).unwrap();
            fields.retain(|field| field["name"] != "moved");
            let fields = serde_json::to_string(&fields).unwrap();
            connection.execute(&format!("UPDATE collections SET fields = ?1 WHERE {board}"), [fields])?;
            connection.execute(
                "UPDATE records SET data = json_remove(data, '$.moved') WHERE collection = 'board'",
                [],
            )
        })
        .await
        .unwrap();
    assert_eq!(rows, 1);
    rig.restart().await;

    // The board has the field all the same, as it has any of a card's own,
    // and a move writes it.
    assert!(names(&declared(&rig).await.unwrap()).contains(&"moved"));
    move_saying(&rig, &id, json!({ "status": "ready", "note": "Next." })).await;
    assert_eq!(came(&rig, &id).await, json!(["draft", "user", "Next.", "1"]));

    // Declared again by the board that ships, over its rows as they are,
    // it is the same board with the same card.
    let again = json!({
        "blueprint": "std/board", "project": "shop", "wait": true,
        "params": { "columns": ["draft", "ready", "doing", "review", "done", "kept"] },
    });
    let applied = rig.send("/api/apply", again).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
    assert_eq!(came(&rig, &id).await, json!(["draft", "user", "Next.", "1"]));
    assert_eq!(rig.read("/api/board/shop").await["items"][0]["status"], "ready");
}

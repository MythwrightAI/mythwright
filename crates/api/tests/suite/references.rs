//! A field that names a record of another collection, from the outside:
//! a blueprint declares it, a card is given one over HTTP, and what it
//! names is read where the client reads it.

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;

/// Sprints, kept as files, and a board whose cards may each be in a sprint
/// and may come after another card, kept where `store` says.
fn plan(store: &str) -> String {
    format!(
        "blueprint: plan
permissions: [projects, board]
steps:
  sprints:
    do: collection.ensure
    with:
      name: sprints
      store: files
      fields:
        title: {{ type: text, required: true }}
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
        sprint: {{ type: record, of: sprints }}
        after: {{ type: record, of: board }}
      views:
        board: {{ kind: board, by: status }}
"
    )
}

async fn applied(rig: &Rig, source: &str) -> Value {
    rig.send("/api/apply", json!({ "source": source, "project": "shop", "wait": true })).await
}

async fn change(rig: &Rig, card: &str, change: Value) -> (StatusCode, Value) {
    rig.call(Method::PATCH, &format!("/api/board/items/{card}"), Some(&rig.token), Some(change)).await
}

#[tokio::test]
async fn a_card_is_given_the_record_it_names_over_http_and_keeps_naming_it_when_that_record_is_gone() {
    for store in ["local", "files"] {
        let rig = Rig::new().await;
        rig.send("/api/projects", json!({ "name": "Shop" })).await;
        let made = applied(&rig, &plan(store)).await;
        assert_eq!(made["finished"]["status"], "ok", "{store}: {made}");

        // A sprint is a file, and whoever writes the file has made one.
        let sprint = json!({ "path": "sprints/1.md", "content": "# November\n" });
        rig.call(Method::PUT, "/api/projects/shop/file", Some(&rig.token), Some(sprint)).await;
        let sprints = rig.read("/api/collections/shop/sprints/records").await;
        assert_eq!(
            (&sprints["records"][0]["id"], &sprints["records"][0]["data"]["title"]),
            (&json!("shop-sprints-1"), &json!("November")),
            "{store}: {sprints}"
        );

        let basket = rig.send("/api/board/shop/items", json!({ "title": "Basket" })).await;
        let checkout = rig.send("/api/board/shop/items", json!({ "title": "Checkout" })).await;
        assert_eq!(basket.get("more"), None, "{store}: a card with nothing more says nothing of it");
        let (status, placed) =
            change(&rig, "shop-2", json!({ "sprint": "shop-sprints-1", "after": basket["id"] })).await;
        assert_eq!(status, StatusCode::OK, "{store}: {placed}");
        assert_eq!(placed["more"], json!({ "sprint": "shop-sprints-1", "after": "shop-1" }), "{store}");
        let board = rig.read("/api/board/shop").await;
        assert_eq!(board["items"][1]["more"], placed["more"], "{store}: {board}");
        if store == "files" {
            let file = rig.read("/api/projects/shop/file?path=board/2.md").await;
            let kept = file["content"].as_str().unwrap();
            assert!(kept.contains("sprint: \"shop-sprints-1\"\n"), "{kept}");
        }

        for (wrong, of) in
            [(json!({ "sprint": "shop-1" }), "sprints"), (json!({ "after": "shop-9" }), "board")]
        {
            let (status, refused) = change(&rig, "shop-2", wrong).await;
            assert_eq!(status, StatusCode::BAD_REQUEST, "{store}: {refused}");
            let message = refused["error"]["message"].as_str().unwrap();
            assert!(message.contains(&format!("is not a record of `{of}`")), "{store}: {message}");
        }
        let (status, stray) = change(&rig, "shop-2", json!({ "epic": "shop-1" })).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{store}: {stray}");
        assert!(
            stray["error"]["message"].as_str().unwrap().starts_with("`epic` is not a field of `board`"),
            "{store}: {stray}"
        );

        // A blueprint's step says the same thing among a card's `fields`.
        let step = "blueprint: order\npermissions: [board]\nsteps:\n  swap:\n    do: board.update\n    with: { id: shop-1, fields: { after: shop-2 } }\n";
        let swapped = applied(&rig, step).await;
        assert_eq!(swapped["finished"]["status"], "ok", "{store}: {swapped}");
        let unknown = step.replace("shop-2", "shop-7");
        let refused = applied(&rig, &unknown).await;
        assert!(
            refused["finished"]["error"].as_str().unwrap().contains("`shop-7` is not a record of `board`"),
            "{store}: {refused}"
        );

        // The sprint's file is deleted, and the card that was first is taken off the board.
        rig.call(Method::DELETE, "/api/projects/shop/file?path=sprints/1.md", Some(&rig.token), None).await;
        rig.call(Method::DELETE, "/api/board/items/shop-1", Some(&rig.token), None).await;
        assert_eq!(rig.read("/api/collections/shop/sprints/records").await["records"], json!([]));
        let (status, moved) =
            change(&rig, checkout["id"].as_str().unwrap(), json!({ "status": "done" })).await;
        assert_eq!(status, StatusCode::OK, "{store}: {moved}");
        assert_eq!(
            (&moved["status"], &moved["more"]),
            (&json!("done"), &json!({ "sprint": "shop-sprints-1", "after": "shop-1" })),
            "{store}: it still says what it named"
        );
        let (status, cleared) = change(&rig, "shop-2", json!({ "sprint": null, "after": "" })).await;
        assert_eq!((status, cleared.get("more")), (StatusCode::OK, None), "{store}: {cleared}");
    }
}

#[tokio::test]
async fn a_field_that_names_a_collection_the_project_has_not_is_refused_when_the_blueprint_is_applied() {
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    let source = plan("local").replace("of: sprints", "of: releases");
    let refused = applied(&rig, &source).await;
    assert_eq!(refused["finished"]["status"], "failed", "{refused}");
    assert!(
        refused["finished"]["error"]
            .as_str()
            .unwrap()
            .contains("the project `shop` has no collection called `releases`; declare `releases` first"),
        "{refused}"
    );
    let (status, none) = rig.get("/api/collections/shop/releases/records").await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{none}");
}

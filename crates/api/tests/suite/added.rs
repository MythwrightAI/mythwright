//! A table's columns as the people who look at it change them, through
//! the daemon: a column added, called something else, retyped, its
//! options changed, and taken away; by whoever holds what each takes
//! and nobody else; kept beside what a blueprint declares when it is
//! applied again, a fleet's tables among them; and written a part of the
//! table's row at a time, so that nothing chosen beside it is lost.

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;
use crate::fleets::{add as add_card, add_row, card, change, fleet, rig as fleet_rig, row, shop};
use crate::held::{key, said};
use crate::lanes::{move_card, moved};

const TABLES: &str = "blueprint: tables
permissions: [projects, board]
steps:
  stock:
    do: collection.ensure
    with:
      name: stock
      fields:
        title: { type: text, required: true }
        points: { type: number }
        stage: { type: choice, of: [new, old] }
      views: { rows: { kind: table } }
  guarded:
    do: collection.ensure
    with: { name: guarded, takes: shell, fields: { title: { type: text, required: true } } }
  moods:
    do: collection.ensure
    with: { name: moods, reads: private:journal, fields: { title: { type: text, required: true } } }
  board:
    do: collection.ensure
    with:
      name: board
      fields: { title: { type: text, required: true }, status: { type: choice, of: [todo, done], required: true } }
      views: { board: { kind: board, by: status } }
";

async fn applied(rig: &Rig, source: &str) -> Value {
    rig.send("/api/apply", json!({ "source": source, "project": "shop", "wait": true })).await
}

fn fields_of(table: &str) -> String {
    format!("/api/collections/shop/{table}/fields")
}

/// A table as the daemon hands it out to the rig's owner.
async fn table(rig: &Rig, name: &str) -> Value {
    let tables = rig.read("/api/collections?project=shop").await;
    let found = tables["collections"].as_array().unwrap().iter().find(|one| one["name"] == name).cloned();
    found.unwrap_or_else(|| panic!("the shop has no {name}: {tables}"))
}

fn field(table: &Value, name: &str) -> Option<Value> {
    table["fields"].as_array().unwrap().iter().find(|one| one["name"] == name).cloned()
}

fn names(table: &Value) -> Vec<String> {
    table["fields"].as_array().unwrap().iter().map(|one| one["name"].as_str().unwrap().to_string()).collect()
}

#[tokio::test]
async fn a_tables_columns_are_changed_by_whoever_holds_what_each_change_takes_and_nobody_else() {
    let rig = &Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    assert_eq!(applied(rig, TABLES).await["finished"]["status"], "ok");
    let ask = |token: &str, method: Method, path: String, body: Option<Value>| {
        let token = token.to_string();
        async move { rig.call(method, &path, Some(&token), body).await }
    };
    let notes = || Some(json!({ "label": "Notes", "kind": "text" }));

    // A member holds the scope unless it was taken away: a column of
    // theirs is the table's, for everybody, and everybody is told.
    let member = key(rig, kernel::principal::MEMBER_SCOPES).await;
    let (status, added) = ask(&member, Method::POST, fields_of("stock"), notes()).await;
    assert_eq!(status, StatusCode::OK, "{added}");
    assert_eq!(added["field"], "notes");
    let kept = field(&added["table"], "notes").unwrap();
    assert_eq!(kept, json!({ "name": "notes", "kind": "text", "of": [], "required": false, "added": true }));
    let told = rig.event("collection.fields", |event| event.data["field"] == "notes").await;
    assert_eq!(told.data, json!({ "project": "shop", "name": "stock", "field": "notes", "did": "added" }));
    // The scopes there are say it, so that whoever is shown what they hold is shown this.
    let meta = rig.read("/api/meta").await;
    assert!(meta["scopes"].as_array().unwrap().iter().any(|area| area["area"] == "columns"), "{meta}");

    // What an agent holds, and what somebody who writes records and no
    // more holds: the whole of `records`, and not this. Each may call a
    // column something else and put what a choice offers in order, as
    // each may save a view; neither adds, retypes or takes away.
    let agent = key(rig, kernel::principal::AGENT_SCOPES).await;
    let clerk = key(rig, &["projects:read", "records"]).await;
    let one = |name: &str| format!("{}/{name}", fields_of("stock"));
    for token in [&agent, &clerk] {
        for (method, path, body) in [
            (Method::POST, fields_of("stock"), Some(json!({ "label": "More", "kind": "text" }))),
            (Method::POST, fields_of("stock"), Some(json!({ "like": "points" }))),
            (Method::PATCH, one("notes"), Some(json!({ "kind": "number" }))),
            (Method::PATCH, one("notes"), Some(json!({ "kind": "choice", "of": ["a", "b"] }))),
            (Method::DELETE, one("notes"), None),
        ] {
            let (status, refused) = ask(token, method.clone(), path.clone(), body).await;
            assert_eq!(status, StatusCode::FORBIDDEN, "{method} {path}: {refused}");
            assert!(said(&refused).contains("needs `columns`"), "{refused}");
        }
        let (status, called) =
            ask(token, Method::PATCH, one("points"), Some(json!({ "label": "Story points" }))).await;
        assert_eq!(status, StatusCode::OK, "{called}");
        assert_eq!(field(&called, "points").unwrap()["label_chosen"], "Story points");
        let (status, ordered) =
            ask(token, Method::PATCH, one("stage"), Some(json!({ "of": ["old", "new"] }))).await;
        assert_eq!(status, StatusCode::OK, "{ordered}");
        assert_eq!(field(&ordered, "stage").unwrap()["of"], json!(["old", "new"]));
        let (status, _) = ask(token, Method::PATCH, one("points"), Some(json!({ "label": null }))).await;
        assert_eq!(status, StatusCode::OK);
        let (status, _) =
            ask(token, Method::PATCH, one("stage"), Some(json!({ "of": ["new", "old"] }))).await;
        assert_eq!(status, StatusCode::OK);
    }
    assert_eq!(names(&table(rig, "stock").await), ["points", "stage", "title", "notes"]);
    // Somebody who reads and does not write is refused all of it, and
    // told nothing of a table by it; and nobody at all is let in unasked.
    let reader = key(rig, &["projects:read", "records:read", "board:read", "columns"]).await;
    for name in ["stock", "board", "moods", "nothing"] {
        for (method, path, body) in [
            (Method::POST, fields_of(name), notes()),
            (Method::PATCH, format!("{}/title", fields_of(name)), Some(json!({ "label": "Name" }))),
            (Method::DELETE, format!("{}/title", fields_of(name)), None),
        ] {
            let (status, refused) = ask(&reader, method.clone(), path, body).await;
            assert_eq!(status, StatusCode::FORBIDDEN, "{method} {name}: {refused}");
            assert!(!said(&refused).contains("private"), "nothing is said of the table: {refused}");
        }
    }
    let (status, _) = rig.call(Method::POST, &fields_of("stock"), None, notes()).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    // The board's columns take what changing its cards takes, and a table
    // that says what changing it takes, that too.
    let shaper = key(rig, &["projects:read", "records", "columns"]).await;
    let (status, refused) = ask(&shaper, Method::POST, fields_of("board"), notes()).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    let (status, refused) = ask(&shaper, Method::POST, fields_of("guarded"), notes()).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    assert!(said(&refused).contains("shell"), "{refused}");
    let (status, added) = ask(&member, Method::POST, fields_of("board"), notes()).await;
    assert_eq!(status, StatusCode::OK, "{added}");

    // A table kept from whoever asks is not there for them, in the words
    // of one the project has not got; its owner's column is heard of by
    // whoever may read the table, and by nobody else.
    for (method, path, body) in [
        (Method::POST, "", notes()),
        (Method::PATCH, "/title", Some(json!({ "label": "Name" }))),
        (Method::DELETE, "/title", None),
    ] {
        let (status, kept) =
            ask(&shaper, method.clone(), format!("{}{path}", fields_of("moods")), body.clone()).await;
        let (_, missing) =
            ask(&shaper, method.clone(), format!("{}{path}", fields_of("nothing")), body).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{method}: {kept}");
        assert_eq!(said(&kept).replace("moods", "nothing"), said(&missing), "{method}");
    }
    assert_eq!(names(&table(rig, "moods").await), ["title"]);
    let (status, added) = ask(&rig.token, Method::POST, fields_of("moods"), notes()).await;
    assert_eq!(status, StatusCode::OK, "{added}");
    let heard = |events: &Value| -> Vec<String> {
        let events = events["events"].as_array().unwrap().iter();
        events.map(|event| event["data"]["name"].as_str().unwrap_or("-").to_string()).collect()
    };
    let topics = "/api/events/recent?topics=collection.fields&limit=100";
    assert!(heard(&rig.read(topics).await).iter().any(|name| name == "moods"));
    let (status, events) = rig.call(Method::GET, topics, Some(&shaper), None).await;
    assert_eq!(status, StatusCode::OK, "{events}");
    let shaper_heard = heard(&events);
    assert!(shaper_heard.iter().any(|name| name == "stock"), "{shaper_heard:?}");
    assert!(!shaper_heard.iter().any(|name| name == "moods"), "{shaper_heard:?}");

    // What is declared is nobody's to retype or take away, whatever they
    // hold; and a change says one thing, in a sentence where it says none.
    for (method, body) in [(Method::PATCH, Some(json!({ "kind": "text" }))), (Method::DELETE, None)] {
        let (status, refused) = ask(&rig.token, method, one("points"), body).await;
        assert_eq!(status, StatusCode::CONFLICT, "{refused}");
        assert!(said(&refused).contains("`points` of `stock` is declared with the table"), "{refused}");
    }
    for wrong in
        [json!({}), json!({ "label": "A", "kind": "text" }), json!({ "label": 7 }), json!({ "of": "a" })]
    {
        let (status, refused) = ask(&rig.token, Method::PATCH, one("notes"), Some(wrong.clone())).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{wrong}: {refused}");
        assert!(said(&refused).starts_with("say one thing of the column"), "{refused}");
    }
    let (status, refused) =
        ask(&rig.token, Method::PATCH, one("nothing"), Some(json!({ "label": "A" }))).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{refused}");
    // Retyped where the rows fit, its options changed, and taken away.
    let (status, retyped) =
        ask(&member, Method::PATCH, one("notes"), Some(json!({ "kind": "choice", "of": ["Low", "High"] })))
            .await;
    assert_eq!(status, StatusCode::OK, "{retyped}");
    assert_eq!(field(&retyped, "notes").unwrap()["of"], json!(["low", "high"]));
    let row = rig
        .send(
            "/api/collections/shop/stock/records",
            json!({ "fields": { "title": "Basket", "notes": "low" } }),
        )
        .await;
    let (status, refused) = ask(&member, Method::PATCH, one("notes"), Some(json!({ "of": ["high"] }))).await;
    assert_eq!(status, StatusCode::CONFLICT, "{refused}");
    assert!(said(&refused).starts_with("`low` is still the `notes` of 1 record(s)"), "{refused}");
    let rename = json!({ "rename": { "from": "low", "to": "Small" } });
    let (status, renamed) = ask(&member, Method::PATCH, one("notes"), Some(rename)).await;
    assert_eq!(status, StatusCode::OK, "{renamed}");
    let id = row["id"].as_str().unwrap();
    assert_eq!(rig.read(&format!("/api/records/{id}")).await["data"]["notes"], "small");
    let (status, removed) = ask(&member, Method::DELETE, one("notes"), None).await;
    assert_eq!(status, StatusCode::OK, "{removed}");
    assert_eq!(names(&removed), ["points", "stage", "title"]);
    assert!(rig.read(&format!("/api/records/{id}")).await["data"].get("notes").is_none());
    let did: Vec<String> = rig
        .said(journal::EventQuery { topics: "collection.fields".into(), limit: 1000, ..Default::default() })
        .await
        .iter()
        .filter(|event| event.data["name"] == "stock" && event.data["field"] == "notes")
        .map(|event| event.data["did"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(did, ["added", "retyped", "options", "removed"]);
}

#[tokio::test]
async fn a_fleet_applied_again_keeps_the_columns_somebody_added_to_its_tables_and_works_a_card_through_them()
{
    let (rig, _at) = fleet_rig().await;
    shop(&rig, json!({})).await;
    let add = |table: &'static str, body: Value| {
        let rig = &rig;
        async move { rig.send(&format!("/api/collections/shop/{table}/fields"), body).await }
    };
    // A column of somebody's own in each of the three tables the fleet
    // reads, and on the board; each row says something there.
    for table in ["agents", "stages", "segments"] {
        let added = add(table, json!({ "label": "Remarks", "kind": "text" })).await;
        assert_eq!(added["field"], "remarks", "{table}: {added}");
    }
    add("agents", json!({ "label": "Seniority", "kind": "choice", "of": ["junior", "senior"] })).await;
    add("board", json!({ "label": "Estimate", "kind": "number" })).await;
    change(&rig, "agents", "worker", json!({ "remarks": "ours", "seniority": "senior" })).await;
    change(&rig, "stages", "work", json!({ "remarks": "the first stage" })).await;
    change(&rig, "stages", "review", json!({ "remarks": "the second" })).await;
    // A thing done before anybody looks, whose row says something in the column too.
    let check = json!({
        "title": "Tests", "stage": "review", "order": 5, "does": "std/run", "command": "exit 0", "remarks": "always",
    });
    add_row(&rig, "segments", check).await;
    // What a declared column is called, and the order of what one offers, are theirs too.
    let called = json!({ "label": "How many" });
    let (status, answer) = rig
        .call(Method::PATCH, "/api/collections/shop/agents/fields/count", Some(&rig.token), Some(called))
        .await;
    assert_eq!(status, StatusCode::OK, "{answer}");

    // The fleet applied again declares its tables again: every column of
    // theirs is there with what the rows hold, and the fleet is as it was.
    // (Applied with something else said, so that its steps run again.)
    fleet(&rig, json!({ "rounds": 3 })).await;
    for (name, has) in
        [("agents", vec!["remarks", "seniority"]), ("stages", vec!["remarks"]), ("segments", vec!["remarks"])]
    {
        let kept = table(&rig, name).await;
        for one in has {
            assert_eq!(
                field(&kept, one).map(|field| field["added"].clone()),
                Some(json!(true)),
                "{name}.{one}"
            );
        }
    }
    let agents = table(&rig, "agents").await;
    assert_eq!(field(&agents, "count").unwrap()["label_chosen"], "How many");
    let worker = rig.read(&format!("/api/records/{}", row(&rig, "agents", "worker").await)).await;
    assert_eq!(
        (&worker["data"]["remarks"], &worker["data"]["seniority"]),
        (&json!("ours"), &json!("senior"))
    );
    let work = rig.read(&format!("/api/records/{}", row(&rig, "stages", "work").await)).await;
    assert_eq!(work["data"]["remarks"], "the first stage");
    let tests = rig.read(&format!("/api/records/{}", row(&rig, "segments", "Tests").await)).await;
    assert_eq!(tests["data"]["remarks"], "always");
    // The board's blueprint applied again too, saying a column more.
    let board = json!({
        "blueprint": "std/board", "project": "shop",
        "params": { "columns": ["draft", "ready", "doing", "testing", "review", "done"] }, "wait": true,
    });
    assert_eq!(rig.send("/api/apply", board).await["finished"]["status"], "ok");
    assert_eq!(field(&table(&rig, "board").await, "estimate").unwrap()["added"], true);

    // A card that says something in their column goes through every
    // stage: worked, checked by the row's command, reviewed and done.
    // Nothing the fleet hands on of a row is refused for a cell it does
    // not know.
    let id = add_card(&rig, "Add a cart", &[]).await;
    let sized = json!({ "fields": { "estimate": 3 } });
    let (status, answer) =
        rig.call(Method::PATCH, &format!("/api/board/items/{id}"), Some(&rig.token), Some(sized)).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    move_card(&rig, &id, "ready", None).await;
    let reviewed = moved(&rig, &id, "review", 0).await;
    moved(&rig, &id, "done", reviewed).await;
    let done = card(&rig, &id).await;
    assert_eq!(done["more"]["estimate"], 3, "{done}");
    // A row changed in their column is a row changed: the fleet takes it and goes on.
    change(&rig, "agents", "worker", json!({ "remarks": "theirs" })).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn columns_added_side_by_side_with_a_view_saved_an_icon_chosen_and_the_table_declared_are_all_kept() {
    let rig = &Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    assert_eq!(applied(rig, TABLES).await["finished"]["status"], "ok");
    // Each is a part of the one row a table is kept as: written whole
    // from what was read a moment before, the last would take the others away.
    let post = |path: String, body: Value| async move {
        rig.call(Method::POST, &path, Some(&rig.token), Some(body)).await
    };
    let put = |path: String, body: Value| async move {
        rig.call(Method::PUT, &path, Some(&rig.token), Some(body)).await
    };
    let patch = |path: String, body: Value| async move {
        rig.call(Method::PATCH, &path, Some(&rig.token), Some(body)).await
    };
    for round in 0..8 {
        let (icon, view) = (["star", "heart", "moon"][round % 3], ["Mine", "Yours"][round % 2]);
        let all = tokio::join!(
            post(fields_of("stock"), json!({ "label": format!("Extra {round}"), "kind": "text" })),
            post("/api/collections/shop/stock/views".into(), json!({ "name": view, "kind": "list" })),
            put(format!("{}/points/icon", fields_of("stock")), json!({ "icon": icon })),
            patch(format!("{}/stage", fields_of("stock")), json!({ "label": format!("Stage {round}") })),
            post("/api/apply".into(), json!({ "source": TABLES, "project": "shop", "wait": true })),
        );
        for (status, answer) in [all.0, all.1, all.2, all.3, all.4] {
            assert_eq!(status, StatusCode::OK, "round {round}: {answer}");
        }
        let stock = table(rig, "stock").await;
        for before in 0..=round {
            let kept = field(&stock, &format!("extra_{before}"));
            assert_eq!(kept.map(|field| field["added"].clone()), Some(json!(true)), "round {round}: {stock}");
        }
        assert_eq!(field(&stock, "points").unwrap()["icon_chosen"], icon, "round {round}");
        assert_eq!(
            field(&stock, "stage").unwrap()["label_chosen"],
            format!("Stage {round}"),
            "round {round}"
        );
        let views = stock["views"].as_array().unwrap();
        assert!(views.iter().any(|one| one["name"] == view), "round {round}: {views:?}");
    }
}

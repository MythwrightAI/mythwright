//! Records of a collection that is not a board, from the outside: a
//! blueprint declares a project's sprints, and they are made, changed and
//! taken away over HTTP and by a blueprint's steps.

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;

/// Sprints that keep a history, and a board whose cards may each be in
/// one; both kept where `store` says.
fn plan(store: &str) -> String {
    format!(
        "blueprint: plan
permissions: [projects, board]
steps:
  sprints:
    do: collection.ensure
    with:
      name: sprints
      store: {store}
      fields:
        title: {{ type: text, required: true }}
        size: {{ type: number }}
      history:
        record.created: \"{{actor}} planned it\"
        record.updated: \"{{actor}} changed its {{changed}}\"
        record.removed: \"{{actor}} took it away\"
        record.commented: \"{{actor}} said: {{body}}\"
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
      views:
        board: {{ kind: board, by: status }}
"
    )
}

async fn applied(rig: &Rig, source: &str) -> Value {
    rig.send("/api/apply", json!({ "source": source, "project": "shop", "wait": true })).await
}

async fn planned(store: &str) -> Rig {
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    let made = applied(&rig, &plan(store)).await;
    assert_eq!(made["finished"]["status"], "ok", "{store}: {made}");
    rig
}

async fn ask(rig: &Rig, method: Method, path: &str, body: Option<Value>) -> (StatusCode, Value) {
    rig.call(method, path, Some(&rig.token), body).await
}

fn message(answer: &Value) -> &str {
    answer["error"]["message"].as_str().unwrap_or_default()
}

/// What has been written into a record's history, once there are `lines` of it.
async fn history(rig: &Rig, record: &'static str, lines: usize) -> Vec<String> {
    let mut written = rig.follow("collection.history");
    crate::soon(&format!("{lines} lines of the history of {record}"), async {
        loop {
            let kept = rig.read(&format!("/api/records/{record}/history")).await;
            let said: Vec<String> = kept["history"]
                .as_array()
                .unwrap()
                .iter()
                .map(|line| line["said"].as_str().unwrap().to_string())
                .collect();
            if said.len() >= lines {
                return said;
            }
            written.next().await.unwrap();
        }
    })
    .await
}

#[tokio::test]
async fn a_record_is_made_read_changed_and_taken_away_over_http_wherever_its_collection_is_kept() {
    for store in ["local", "files"] {
        let rig = planned(store).await;
        let sprints = "/api/collections/shop/sprints/records";
        let november = rig.send(sprints, json!({ "fields": { "title": "November", "size": 5 } })).await;
        assert_eq!(
            (&november["id"], &november["collection"], &november["data"]),
            (&json!("shop-sprints-1"), &json!("sprints"), &json!({ "title": "November", "size": 5 })),
            "{store}: {november}"
        );
        rig.send(sprints, json!({ "fields": { "title": "December" } })).await;
        let read = rig.read(sprints).await;
        assert_eq!(read["records"].as_array().unwrap().len(), 2, "{store}: {read}");
        assert_eq!(rig.read("/api/records/shop-sprints-2").await["data"]["title"], "December", "{store}");

        let one = "/api/records/shop-sprints-1";
        let (status, grown) = ask(&rig, Method::PATCH, one, Some(json!({ "fields": { "size": 8 } }))).await;
        assert_eq!(status, StatusCode::OK, "{store}: {grown}");
        assert_eq!(
            grown["data"],
            json!({ "title": "November", "size": 8 }),
            "{store}: the title is as it was"
        );
        if store == "files" {
            let file = rig.read("/api/projects/shop/file?path=sprints/1.md").await;
            let kept = file["content"].as_str().unwrap();
            assert!(kept.contains("size: 8\n") && kept.contains("# November\n"), "{kept}");
        }

        // What is not so is said, with the status it has everywhere else.
        for (method, path, body, status, says) in [
            (
                Method::POST,
                sprints,
                json!({ "title": "January" }),
                StatusCode::BAD_REQUEST,
                "`fields` is needed",
            ),
            (
                Method::POST,
                sprints,
                json!({ "fields": { "size": 3 } }),
                StatusCode::BAD_REQUEST,
                "`title` is needed",
            ),
            (
                Method::PATCH,
                one,
                json!({ "fields": { "goal": "ship" } }),
                StatusCode::BAD_REQUEST,
                "`goal` is not a field of `sprints`",
            ),
            (
                Method::PATCH,
                one,
                json!({ "fields": { "size": "big" } }),
                StatusCode::BAD_REQUEST,
                "`size` must be a number",
            ),
            (
                Method::PATCH,
                "/api/records/shop-sprints-9",
                json!({ "fields": { "size": 1 } }),
                StatusCode::NOT_FOUND,
                "there is no record `shop-sprints-9`",
            ),
            (
                Method::POST,
                "/api/collections/shop/releases/records",
                json!({ "fields": { "title": "1.0" } }),
                StatusCode::NOT_FOUND,
                "has no collection called `releases`",
            ),
        ] {
            let (answered, refused) = ask(&rig, method.clone(), path, Some(body)).await;
            assert_eq!(answered, status, "{store}: {method} {path}: {refused}");
            assert!(message(&refused).contains(says), "{store}: {method} {path}: {refused}");
        }

        // A card is in the sprint, and is not itself written this way.
        let card = json!({ "title": "Basket", "fields": { "sprint": "shop-sprints-1" } });
        let card = rig.send("/api/board/shop/items", card).await;
        assert_eq!(card["more"], json!({ "sprint": "shop-sprints-1" }), "{store}: {card}");
        assert_eq!(rig.read("/api/records/shop-1").await["data"]["title"], "Basket", "{store}: it is read");
        let cards = "/api/collections/shop/board/records";
        for (method, path) in [
            (Method::POST, cards),
            (Method::PATCH, "/api/records/shop-1"),
            (Method::DELETE, "/api/records/shop-1"),
        ] {
            let (status, refused) =
                ask(&rig, method.clone(), path, Some(json!({ "fields": { "title": "Till" } }))).await;
            assert_eq!(status, StatusCode::BAD_REQUEST, "{store}: {method} {path}: {refused}");
            assert!(
                message(&refused).contains("Use `board.add`, `board.update` and `board.move`"),
                "{store}: {refused}"
            );
        }

        // Taken away, it is gone; its history is not, and the card that was in it still moves.
        let (status, gone) = ask(&rig, Method::DELETE, one, None).await;
        assert_eq!((status, &gone["ok"]), (StatusCode::OK, &json!(true)), "{store}: {gone}");
        assert_eq!(rig.get(one).await.0, StatusCode::NOT_FOUND, "{store}");
        assert_eq!(
            history(&rig, "shop-sprints-1", 3).await,
            ["Ada planned it", "Ada changed its size", "Ada took it away"],
            "{store}"
        );
        let (status, moved) =
            ask(&rig, Method::PATCH, "/api/board/items/shop-1", Some(json!({ "status": "done" }))).await;
        assert_eq!(
            (status, &moved["more"]),
            (StatusCode::OK, &json!({ "sprint": "shop-sprints-1" })),
            "{store}: {moved}"
        );
    }
}

#[tokio::test]
async fn a_key_reads_and_writes_records_only_as_far_as_it_says() {
    let rig = planned("local").await;
    let sprints = "/api/collections/shop/sprints/records";
    rig.send(sprints, json!({ "fields": { "title": "November" } })).await;
    let mut keys = Vec::new();
    for scopes in [json!(["board", "projects"]), json!(["records:read"])] {
        let made = rig.send("/api/credentials", json!({ "name": "narrow", "scopes": scopes })).await;
        keys.push(made["token"].as_str().unwrap().to_string());
    }
    let (boards, reader) = (keys[0].clone(), keys[1].clone());

    for path in [sprints, "/api/records/shop-sprints-1"] {
        let (status, refused) = rig.call(Method::GET, path, Some(&boards), None).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{path}: {refused}");
        assert!(message(&refused).contains("needs `records:read`"), "{refused}");
        assert_eq!(rig.call(Method::GET, path, Some(&reader), None).await.0, StatusCode::OK, "{path}");
    }
    let more = json!({ "fields": { "title": "December" } });
    for (method, path) in [
        (Method::POST, sprints),
        (Method::PATCH, "/api/records/shop-sprints-1"),
        (Method::DELETE, "/api/records/shop-sprints-1"),
    ] {
        let (status, refused) = rig.call(method.clone(), path, Some(&reader), Some(more.clone())).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{method} {path}: {refused}");
        assert!(message(&refused).contains("needs `records:write`"), "{refused}");
    }
    assert_eq!(rig.read(sprints).await["records"].as_array().unwrap().len(), 1, "nothing was written");

    // Saying something on a record is a change to it, of the same kind:
    // on a sprint it needs what writing a sprint needs, and on a card what
    // writing the board needs, which is not the same thing.
    let words = json!({ "body": "Two weeks is too short." });
    let said = "/api/records/shop-sprints-1/comments";
    for key in [&reader, &boards] {
        let (status, refused) = rig.call(Method::POST, said, Some(key), Some(words.clone())).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
        assert!(message(&refused).contains("needs `records:write`"), "{refused}");
    }
    rig.send("/api/board/shop/items", json!({ "title": "Basket" })).await;
    let (status, refused) =
        rig.call(Method::POST, "/api/records/shop-1/comments", Some(&reader), Some(words.clone())).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    assert!(message(&refused).contains("needs `board:write`"), "{refused}");
    // The key that may write the board gets as far as the board itself,
    // which here keeps no history of its cards and so refuses what would
    // be kept nowhere, saying what to do.
    let (status, unkept) =
        rig.call(Method::POST, "/api/records/shop-1/comments", Some(&boards), Some(words)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{unkept}");
    assert!(
        message(&unkept).starts_with("what is said on a record of `board` would be kept nowhere")
            && message(&unkept).contains("apply its blueprint again"),
        "{unkept}"
    );
    assert_eq!(rig.count_of("record.commented").await, 0);
}

#[tokio::test]
async fn a_blueprints_steps_make_change_read_and_take_away_records_and_its_history_says_the_factorys_bot_did()
{
    let rig = planned("local").await;
    let steps = "blueprint: season
permissions: [records]
steps:
  winter:
    do: record.add
    with: { collection: sprints, fields: { title: Winter, size: 3 } }
  spring:
    do: record.add
    with: { collection: sprints, fields: { title: Spring } }
  grow:
    do: record.update
    with: { id: \"${steps.winter.out.id}\", fields: { size: 8 } }
  read:
    do: record.get
    with: { id: \"${steps.winter.out.id}\" }
  all:
    do: record.list
    with: { collection: sprints }
  drop:
    do: record.remove
    with: { id: \"${steps.spring.out.id}\" }
outputs:
  said: \"${steps.read.out.data.title} is ${steps.read.out.data.size} of ${steps.all.out.count}, with ${steps.all.out.items[1].id}\"
";
    let ran = applied(&rig, steps).await;
    assert_eq!(ran["finished"]["status"], "ok", "{ran}");
    assert_eq!(ran["finished"]["outputs"]["said"], "Winter is 8 of 2, with shop-sprints-2", "{ran}");
    let left = rig.read("/api/collections/shop/sprints/records").await;
    assert_eq!(left["records"].as_array().unwrap().len(), 1, "{left}");
    assert_eq!(left["records"][0]["data"], json!({ "title": "Winter", "size": 8 }));
    assert_eq!(
        history(&rig, "shop-sprints-1", 2).await,
        ["conductor planned it", "conductor changed its size"],
        "the factory's bot, and not whoever applied the blueprint"
    );
    assert_eq!(history(&rig, "shop-sprints-2", 2).await, ["conductor planned it", "conductor took it away"]);

    // A blueprint that does not say it uses records is told so, and one may not write a card this way.
    let unsaid = steps.replace("permissions: [records]", "permissions: [board]");
    let (status, checked) = rig.post("/api/blueprints/check", json!({ "source": unsaid })).await;
    assert_eq!(status, StatusCode::OK, "{checked}");
    assert!(
        checked.to_string().contains("this uses `records:write`, which is not listed under `permissions`"),
        "{checked}"
    );
    let card = "blueprint: sneak\npermissions: [records]\nsteps:\n  add:\n    do: record.add\n    with: { collection: board, fields: { title: Till, status: todo } }\n";
    let refused = applied(&rig, card).await;
    assert_eq!(refused["finished"]["status"], "failed", "{refused}");
    assert!(
        refused["finished"]["error"].as_str().unwrap().contains("a board's cards are not added as records"),
        "{refused}"
    );
}

#[tokio::test]
async fn the_rows_a_table_starts_with_are_added_once_however_often_it_is_asked_and_a_row_somebody_changed_is_left()
 {
    let rig = planned("local").await;
    // Asked under another name each time, so that each asking is done.
    let starts = |name: &str| {
        format!(
            "blueprint: {name}
permissions: [records]
steps:
  first:
    do: rows.start
    with:
      collection: sprints
      rows:
        - {{ title: Winter, size: 3 }}
        - {{ title: Spring }}
        - {{ title: Summer, size: 5 }}
outputs:
  added: ${{steps.first.out.added}}
  count: ${{steps.first.out.count}}
  there: ${{steps.first.out.there}}
"
        )
    };
    let titles = |rows: &Value| -> Vec<Value> {
        rows["records"].as_array().unwrap().iter().map(|row| row["data"].clone()).collect()
    };
    let ran = applied(&rig, &starts("once")).await;
    assert_eq!(ran["finished"]["status"], "ok", "{ran}");
    let all = json!(["shop-sprints-1", "shop-sprints-2", "shop-sprints-3"]);
    assert_eq!(ran["finished"]["outputs"], json!({ "added": all, "count": 3, "there": 0 }), "{ran}");
    assert_eq!(history(&rig, "shop-sprints-2", 1).await, ["conductor planned it"], "added as a record is");

    // Somebody changes a row, and it is asked a second time: nothing is
    // added, and the row holds what they gave it.
    let (status, changed) =
        ask(&rig, Method::PATCH, "/api/records/shop-sprints-1", Some(json!({ "fields": { "size": 8 } })))
            .await;
    assert_eq!(status, StatusCode::OK, "{changed}");
    let ran = applied(&rig, &starts("twice")).await;
    assert_eq!(ran["finished"]["outputs"], json!({ "added": [], "count": 0, "there": 3 }), "{ran}");
    let rows = rig.read("/api/collections/shop/sprints/records").await;
    assert_eq!(
        titles(&rows),
        [
            json!({ "title": "Winter", "size": 8 }),
            json!({ "title": "Spring", "size": null }),
            json!({ "title": "Summer", "size": 5 })
        ]
    );

    // It adds what the table has not got, and that is all it knows: one
    // taken away is one the table has not got. Whether a table is still
    // to be given its first rows is whoever asks.
    let (status, gone) = ask(&rig, Method::DELETE, "/api/records/shop-sprints-2", None).await;
    assert_eq!(status, StatusCode::OK, "{gone}");
    let ran = applied(&rig, &starts("thrice")).await;
    let back = json!({ "added": ["shop-sprints-4"], "count": 1, "there": 2 });
    assert_eq!(ran["finished"]["outputs"], back, "{ran}");

    // A row that says nothing of what it is called could not be found
    // again, and nothing is added of a list that has one.
    let unnamed = starts("unnamed").replace("- { title: Spring }", "- { size: 2 }");
    let refused = applied(&rig, &unnamed).await;
    assert_eq!(refused["finished"]["status"], "failed", "{refused}");
    assert!(refused["finished"]["error"].as_str().unwrap().contains("row 2 says nothing there"), "{refused}");
    assert_eq!(
        rig.read("/api/collections/shop/sprints/records").await["records"].as_array().unwrap().len(),
        3
    );
}

#[tokio::test]
async fn a_watch_says_in_a_channel_what_happens_to_a_record_that_is_not_a_card() {
    let rig = planned("local").await;
    let watch = json!({
        "blueprint": "std/watch", "project": "shop", "key": "watch-sprints", "wait": true,
        "params": { "collection": "sprints", "channel": "general" },
    });
    assert_eq!(rig.send("/api/apply", watch).await["finished"]["status"], "ok");
    let sprints = "/api/collections/shop/sprints/records";
    rig.send(sprints, json!({ "fields": { "title": "November" } })).await;
    ask(&rig, Method::PATCH, "/api/records/shop-sprints-1", Some(json!({ "fields": { "size": 5 } }))).await;
    // A card is not a sprint, and the watch says nothing of it.
    rig.send("/api/board/shop/items", json!({ "title": "Basket" })).await;
    // What is said on the sprint is kept in its history like the rest, and
    // changes nothing of it.
    let on =
        rig.send("/api/records/shop-sprints-1/comments", json!({ "body": "Two weeks is too short." })).await;
    assert_eq!((&on["id"], &on["data"]["size"]), (&json!("shop-sprints-1"), &json!(5)));

    let mut posted = rig.follow("channel.message");
    let said = crate::soon("three lines from the watch", async {
        loop {
            let messages = rig.read("/api/channels/general/messages").await;
            let said: Vec<String> = messages["messages"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|message| message["author"] == "bot:watch")
                .map(|message| message["body"].as_str().unwrap().to_string())
                .collect();
            if said.len() >= 3 {
                return said;
            }
            posted.next().await.unwrap();
        }
    })
    .await;
    assert_eq!(
        said,
        [
            "shop-sprints-1: Ada planned it",
            "shop-sprints-1: Ada changed its size",
            "shop-sprints-1: Ada said: Two weeks is too short.",
        ]
    );
    assert_eq!(history(&rig, "shop-sprints-1", 3).await[2], "Ada said: Two weeks is too short.");
}

#[tokio::test]
async fn a_tables_rows_are_read_with_what_they_leave_unsaid_filled_in_whatever_the_table_is_for() {
    // Shelves and the boxes on them: tables of nobody's fleet. What a row
    // leaves unsaid is filled in by what the blueprint that reads hands,
    // and where it hands nothing, by the kind of field it is.
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    let tables = "blueprint: stock
permissions: [projects]
steps:
  shelves:
    do: collection.ensure
    with:
      name: shelves
      fields:
        title: { type: text, required: true }
        aisle: { type: text }
        holds: { type: number }
        open: { type: flag }
        tags: { type: list }
  boxes:
    do: collection.ensure
    with:
      name: boxes
      fields:
        title: { type: text, required: true }
        aisle: { type: text }
        weight: { type: number }
";
    assert_eq!(applied(&rig, tables).await["finished"]["status"], "ok");
    let add = |table: &'static str, fields: Value| {
        let rig = &rig;
        async move {
            let made = rig
                .send(&format!("/api/collections/shop/{table}/records"), json!({ "fields": fields }))
                .await;
            made["id"].as_str().unwrap().to_string()
        }
    };
    add("boxes", json!({ "title": "nails", "aisle": "tools", "weight": 3 })).await;
    add("boxes", json!({ "title": "screws", "aisle": "tools", "weight": 2 })).await;
    add("boxes", json!({ "title": "broken", "aisle": "tools", "weight": 9, })).await;
    add("boxes", json!({ "title": "paint", "aisle": "paint", "weight": 5 })).await;
    let first =
        add("shelves", json!({ "title": "Tools", "holds": 4, "open": true, "tags": ["heavy"] })).await;
    // Emptied, a cell says no more than one never filled.
    let second = add("shelves", json!({ "title": "Paint", "aisle": "", "holds": null })).await;
    let third = add("shelves", json!({ "title": "Spare tools", "aisle": "tools" })).await;

    let read = "blueprint: counted
permissions: [records]
steps:
  boxes:
    do: rows.read
    with: { collection: boxes, once: title, adding: weight }
  shelves:
    do: rows.read
    with:
      collection: shelves
      had: [shop-shelves-9]
      once: aisle
      fill: { aisle: { slug_of: title }, holds: 1 }
      counting:
        weight: { of: ${steps.boxes.out.rows}, by: aisle, sum: weight }
        boxes: { of: ${steps.boxes.out.rows}, by: aisle }
outputs:
  shelves: ${steps.shelves.out}
  weight: ${steps.boxes.out.sum}
";
    let outputs = applied(&rig, read).await["finished"]["outputs"].clone();
    assert_eq!(outputs["weight"], 19, "{outputs}");
    let shelves = &outputs["shelves"];
    assert_eq!(
        shelves["rows"],
        json!([
            {
                "id": first, "title": "Tools", "aisle": "tools", "holds": 4, "open": true, "tags": ["heavy"],
                "again": false, "weight": 14, "weight_rows": 3, "boxes": 3, "boxes_rows": 3,
            },
            {
                "id": second, "title": "Paint", "aisle": "paint", "holds": 1, "open": false, "tags": [],
                "again": false, "weight": 5, "weight_rows": 1, "boxes": 1, "boxes_rows": 1,
            },
            {
                "id": third, "title": "Spare tools", "aisle": "tools", "holds": 1, "open": false, "tags": [],
                "again": true, "weight": 14, "weight_rows": 3, "boxes": 3, "boxes_rows": 3,
            },
        ]),
        "{shelves}"
    );
    // The row that says in `aisle` what an earlier row says, with that row.
    assert_eq!(shelves["again"].as_array().unwrap().len(), 1, "{shelves}");
    assert_eq!(
        (&shelves["again"][0]["id"], &shelves["again"][0]["first"]["title"]),
        (&json!(third), &json!("Tools"))
    );
    assert_eq!(shelves["ids"], json!([first, second, third]));
    assert_eq!(shelves["gone"], json!(["shop-shelves-9"]));
    assert_eq!(shelves["count"], 3);
    // Asked nothing of what rows say, none is picked out and none is left over.
    assert_eq!((&shelves["those"], &shelves["others"]), (&json!([]), &json!([])));

    // Which rows say what is asked of a cell: exactly a value, anything
    // but one, something, or nothing; and of several cells, all of it. A
    // cell emptied and one never filled both say nothing, filled in or not.
    let asked = "blueprint: picked
permissions: [records]
steps:
  tools:
    do: rows.read
    with: { collection: shelves, fill: { aisle: { slug_of: title } }, saying: { aisle: tools } }
  not_tools:
    do: rows.read
    with: { collection: shelves, fill: { aisle: { slug_of: title } }, saying: { aisle: { not: tools } } }
  said:
    do: rows.read
    with: { collection: shelves, saying: { aisle: { said: true } } }
  unsaid:
    do: rows.read
    with: { collection: shelves, saying: { aisle: { said: false }, tags: { said: false } } }
  open_tools:
    do: rows.read
    with: { collection: shelves, saying: { open: true, holds: 4 } }
outputs:
  tools: ${steps.tools.out.those}
  not_tools: ${steps.not_tools.out.those}
  left: ${steps.not_tools.out.others}
  said: ${steps.said.out.those}
  unsaid: ${steps.unsaid.out.those}
  open_tools: ${steps.open_tools.out.those}
";
    let picked = applied(&rig, asked).await["finished"]["outputs"].clone();
    assert_eq!(picked["tools"], json!([first, third]), "{picked}");
    assert_eq!(picked["not_tools"], json!([second]), "{picked}");
    assert_eq!(picked["left"], json!([first, third]), "{picked}");
    assert_eq!(picked["said"], json!([third]), "{picked}");
    assert_eq!(picked["unsaid"], json!([second]), "{picked}");
    assert_eq!(picked["open_tools"], json!([first]), "{picked}");

    // What makes no sense is refused in a sentence, and nothing is guessed:
    // a cell no row has, and something asked of a cell that is none of the three.
    for (saying, why) in [
        ("{ colour: red }", "`saying` names the cell `colour`, which no row of `shelves` has"),
        ("{ aisle: { like: tools } }", "`saying` says of a cell what a row is to say in it"),
        ("{ aisle: { said: perhaps } }", "`saying` says of a cell what a row is to say in it"),
    ] {
        let wrong = format!(
            "blueprint: wrong\npermissions: [records]\nsteps:\n  read:\n    do: rows.read\n    with: {{ collection: shelves, saying: {saying} }}\n"
        );
        let refused = applied(&rig, &wrong).await;
        assert_eq!(refused["finished"]["status"], "failed", "{saying}: {refused}");
        assert!(refused.to_string().contains(why), "{saying}: {refused}");
    }
}

#[tokio::test]
async fn what_is_said_on_a_record_and_could_not_be_kept_is_refused_and_not_answered_as_taken() {
    let rig = planned("local").await;
    rig.send("/api/collections/shop/sprints/records", json!({ "fields": { "title": "November" } })).await;
    let before = history(&rig, "shop-sprints-1", 1).await;

    // The journal cannot keep what is said on a record, as when its disk
    // is full: the saying is all there is of it.
    // The journal that keeps what is said of the project: its own.
    let journal = rig.node().app.journal.of(Some("shop"));
    let refuse = "CREATE TRIGGER full BEFORE INSERT ON events WHEN NEW.topic = 'record.commented'
                  BEGIN SELECT RAISE(FAIL, 'database or disk is full'); END;";
    journal.db().call(move |connection| connection.execute_batch(refuse)).await.unwrap();
    let said = "/api/records/shop-sprints-1/comments";
    let words = json!({ "body": "Two weeks is too short." });
    let (status, answer) = ask(&rig, Method::POST, said, Some(words.clone())).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{answer}");
    let told = message(&answer);
    assert!(
        told.starts_with("What was said on `shop-sprints-1` was not kept") && told.ends_with("Say it again."),
        "{told}"
    );

    // Said again when the journal keeps again, it is taken, and is on the
    // record once.
    journal.db().call(|connection| connection.execute_batch("DROP TRIGGER full;")).await.unwrap();
    let (status, answer) = ask(&rig, Method::POST, said, Some(words)).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    let after = history(&rig, "shop-sprints-1", before.len() + 1).await;
    assert_eq!(after.len(), before.len() + 1, "{after:?}");
    assert!(after.last().unwrap().contains("Two weeks is too short."), "{after:?}");
}

#[tokio::test]
async fn a_record_of_a_table_called_in_several_words_is_read_changed_spoken_on_and_taken_away_by_its_id() {
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    let text = "blueprint: asked\nsteps:\n  made:\n    do: collection.ensure\n    with:\n      name: open-questions\n      fields:\n        title: { type: text, required: true }\n        size: { type: number }\n      history: [\"record.*\"]\n";
    assert_eq!(applied(&rig, text).await["finished"]["status"], "ok");
    let added = rig
        .send("/api/collections/shop/open-questions/records", json!({ "fields": { "title": "Why?" } }))
        .await;
    assert_eq!(
        (&added["id"], &added["collection"]),
        (&json!("shop-open-questions-1"), &json!("open-questions"))
    );
    // An id is a name: where its project's name ends and its table's
    // begins is not read from it. The record is its table's all the same.
    let one = "/api/records/shop-open-questions-1";
    let read = rig.read(one).await;
    assert_eq!(
        (&read["collection"], &read["data"]["title"]),
        (&json!("open-questions"), &json!("Why?")),
        "{read}"
    );
    let (status, grown) = ask(&rig, Method::PATCH, one, Some(json!({ "fields": { "size": 8 } }))).await;
    assert_eq!(status, StatusCode::OK, "{grown}");
    assert_eq!(grown["data"], json!({ "title": "Why?", "size": 8 }));
    let said = json!({ "body": "Because." });
    let (status, answer) = ask(&rig, Method::POST, &format!("{one}/comments"), Some(said)).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    assert!(history(&rig, "shop-open-questions-1", 3).await.len() >= 3);
    let (status, answer) = ask(&rig, Method::DELETE, one, None).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    let (status, gone) = ask(&rig, Method::GET, one, None).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{gone}");
    assert_eq!(message(&gone), "there is no record `shop-open-questions-1`");
    assert_eq!(rig.read("/api/collections/shop/open-questions/records").await["records"], json!([]));
}

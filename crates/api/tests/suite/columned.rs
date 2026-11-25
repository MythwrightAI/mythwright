//! What a view of a table says of its columns: the order they stand in
//! (`columns`), which it is frozen up to (`freezes`), the figure at the
//! foot of some (`figures`), and its rows gathered by one (`group`, of a
//! table as of a board). Said where a table is declared and saved by
//! whoever looks at it, by the door every change of a view goes through;
//! kept through a declaration, and one that takes a column away; and
//! saved by whoever may save a view and nobody else.

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;
use crate::held::{applied, key, said};

/// A table of stock, with `views` declared beside its fields and, where `shelf` is said, a column of shelves.
fn stock(shelf: bool, views: &str) -> String {
    let shelf = if shelf { "\n        shelf: { type: text }" } else { "" };
    format!(
        "blueprint: stock
steps:
  stock:
    do: collection.ensure
    with:
      name: stock
      fields:
        title: {{ type: text, required: true }}
        points: {{ type: number }}
        done: {{ type: flag }}
        status: {{ type: choice, of: [todo, done] }}{shelf}
{views}"
    )
}

const VIEWS: &str = "      views:
        rows: { kind: table, columns: [status, points], freezes: status, figures: { points: sum, done: ticked }, widths: { title: 320, done: 70 }, group: status }
";

async fn shop() -> Rig {
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    rig
}

/// The view `name` of the shop's stock, as whoever reads its tables is told it.
async fn view(rig: &Rig, name: &str) -> Value {
    let tables = rig.read("/api/collections?project=shop").await;
    let stock = tables["collections"].as_array().unwrap().iter().find(|one| one["name"] == "stock").cloned();
    let views = stock.unwrap_or_else(|| panic!("the shop has no stock: {tables}"))["views"].clone();
    views.as_array().unwrap().iter().find(|one| one["name"] == name).cloned().unwrap_or(Value::Null)
}

/// Saves a view of the stock with `token`.
async fn save(rig: &Rig, token: &str, view: Value) -> (StatusCode, Value) {
    rig.call(Method::POST, "/api/collections/shop/stock/views", Some(token), Some(view)).await
}

#[tokio::test]
async fn a_blueprint_says_the_order_of_a_views_columns_what_it_is_frozen_up_to_and_its_figures_and_what_is_wrong_is_refused()
 {
    let rig = shop().await;
    assert_eq!(applied(&rig, &rig.token, &stock(true, VIEWS)).await, ("ok".to_string(), String::new()));
    let rows = view(&rig, "rows").await;
    assert_eq!(rows["columns"], json!(["status", "points"]), "{rows}");
    assert_eq!(rows["freezes"], "status", "{rows}");
    assert_eq!(rows["figures"], json!({ "points": "sum", "done": "ticked" }), "{rows}");
    assert_eq!(rows["widths"], json!({ "title": 320, "done": 70 }), "{rows}");
    assert_eq!(rows["group"], "status", "{rows}");

    // What is wrong is said where the blueprint is applied, and the table is as it was.
    let cases = [
        (
            "        rows: { kind: table, freezes: nowhere }\n",
            "is frozen up to `nowhere`, which is not a column of the table",
        ),
        ("        rows: { kind: list, freezes: title }\n", "says `freezes: title`, which is for a table"),
        (
            "        rows: { kind: table, figures: { title: sum } }\n",
            "says `sum` at the foot of `title`, which is a `text`",
        ),
        (
            "        rows: { kind: table, figures: { points: median } }\n",
            "says `median` at the foot of `points`, which is not a figure",
        ),
        (
            "        rows: { kind: table, figures: { nowhere: filled } }\n",
            "says a figure at the foot of `nowhere`, which is not a column",
        ),
        (
            "        rows: { kind: gallery, figures: { points: sum } }\n",
            "says `figures`, which is for a table",
        ),
        (
            "        rows: { kind: table, columns: title }\n",
            "says `columns`, which is a list of its columns in the order they stand",
        ),
        (
            "        rows: { kind: table, figures: sum }\n",
            "says `figures`, which is a map from a column to the figure at its foot",
        ),
        (
            "        rows: { kind: table, widths: { title: 20 } }\n",
            "says `title` is 20 pixels wide, and a column is from 60 to 2000",
        ),
        (
            "        rows: { kind: gallery, widths: { title: 200 } }\n",
            "says `widths`, which is for a table: how wide each of its columns is",
        ),
        (
            "        rows: { kind: gallery, group: status }\n",
            "says `group: status`, which is for a board, an order or a table",
        ),
    ];
    for (views, wanted) in cases {
        let (status, why) = applied(&rig, &rig.token, &stock(true, &format!("      views:\n{views}"))).await;
        assert_eq!(status, "failed", "{views}");
        assert!(why.contains(wanted), "{views}: {why}");
    }
    assert_eq!(view(&rig, "rows").await, rows);

    // A view that says none of it is kept with none of it, as one from before a view could say it is.
    assert_eq!(
        applied(&rig, &rig.token, &stock(true, "      views:\n        rows: { kind: table }\n")).await.0,
        "ok"
    );
    let plain = view(&rig, "rows").await;
    for key in ["columns", "freezes", "figures", "widths", "group"] {
        assert!(plain.get(key).is_none(), "`{key}` of {plain}");
    }
}

#[tokio::test]
async fn somebody_saves_what_a_view_says_of_its_columns_and_it_is_said_kept_and_outlasts_a_column_taken_away()
{
    let rig = shop().await;
    assert_eq!(applied(&rig, &rig.token, &stock(true, "")).await.0, "ok");
    let mark = rig.mark();
    let mine = json!({
        "name": "Mine", "kind": "table",
        "columns": ["shelf", "points", "nowhere"], "freezes": "shelf",
        "figures": { "points": "mean", "shelf": "different" }, "widths": { "shelf": 180, "nowhere": 90 },
        "group": "status",
    });
    let (status, kept) = save(&rig, &rig.token, mine.clone()).await;
    assert_eq!(status, StatusCode::OK, "{kept}");
    let told = rig.event("collection.views", |event| event.seq > mark).await;
    assert_eq!(
        (&told.data["project"], &told.data["name"], &told.data["view"]),
        (&json!("shop"), &json!("stock"), &json!("Mine"))
    );
    let saved = view(&rig, "Mine").await;
    // The order is kept as it was said: a name that is no column is passed over where it is read, not refused.
    for key in ["columns", "freezes", "figures", "widths", "group"] {
        assert_eq!(saved[key], mine[key], "`{key}` of {saved}");
    }
    assert_eq!(saved["saved"], true);

    // Saved again saying none of it, it says none: each is said whole by whoever saves.
    let (status, _) =
        save(&rig, &rig.token, json!({ "name": "Mine", "kind": "table", "columns": [], "figures": {} }))
            .await;
    assert_eq!(status, StatusCode::OK);
    let bare = view(&rig, "Mine").await;
    for key in ["columns", "freezes", "figures", "widths", "group"] {
        assert!(bare.get(key).is_none(), "`{key}` of {bare}");
    }

    // What is wrong is refused in a sentence, and the view is as it was.
    let wrongly = [
        (
            json!({ "freezes": "nowhere" }),
            "is frozen up to `nowhere`, which is not a column of the table; it has:",
        ),
        (
            json!({ "figures": { "title": "sum" } }),
            "says `sum` at the foot of `title`, which is a `text`: a column of that kind can say filled, empty, different",
        ),
        (
            json!({ "figures": { "done": "sum" } }),
            "which is a `flag`: a column of that kind can say filled, empty, ticked",
        ),
        (
            json!({ "figures": { "points": "best" } }),
            "which is not a figure; there are: filled, empty, sum, mean, least, greatest, ticked, different",
        ),
        (
            json!({ "figures": { "gone": "filled" } }),
            "says a figure at the foot of `gone`, which is not a column of the table",
        ),
        (
            json!({ "columns": "shelf" }),
            "says `columns`, which is a list of its columns in the order they stand",
        ),
        (json!({ "freezes": 3 }), "says `freezes`, which is the one column it is frozen up to"),
        (json!({ "kind": "list", "freezes": "title" }), "says `freezes: title`, which is for a table"),
        (json!({ "kind": "list", "figures": { "points": "sum" } }), "says `figures`, which is for a table"),
        (
            json!({ "widths": { "points": 12 } }),
            "says `points` is 12 pixels wide, and a column is from 60 to 2000",
        ),
        (json!({ "widths": { "points": 5000 } }), "says `points` is 5000 pixels wide"),
        (
            json!({ "widths": { "points": "wide" } }),
            "says `widths`, which is a map from a column to how wide it is",
        ),
        (json!({ "kind": "list", "widths": { "points": 90 } }), "says `widths`, which is for a table"),
    ];
    for (more, wanted) in wrongly {
        let mut asked = json!({ "name": "Mine", "kind": "table" });
        asked.as_object_mut().unwrap().extend(more.as_object().unwrap().clone());
        let (status, refused) = save(&rig, &rig.token, asked.clone()).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{asked}: {refused}");
        assert!(said(&refused).contains(wanted), "{asked}: {refused}");
    }
    assert_eq!(view(&rig, "Mine").await, bare);

    // Said again, and the table declared again without the column the view
    // is frozen up to and says a figure of: the view is somebody's still,
    // with what it said, which is passed over where it is read.
    assert_eq!(save(&rig, &rig.token, mine.clone()).await.0, StatusCode::OK);
    assert_eq!(applied(&rig, &rig.token, &stock(false, "")).await.0, "ok");
    let outlasted = view(&rig, "Mine").await;
    assert_eq!(outlasted, saved, "the view was kept whole through the declaration");
    // Saved from then on, what names the column that is gone is refused, as anything said now is.
    let (status, refused) = save(&rig, &rig.token, mine).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");
    assert!(
        said(&refused).contains("is frozen up to `shelf`, which is not a column of the table"),
        "{refused}"
    );
}

#[tokio::test]
async fn what_a_view_says_of_its_columns_is_saved_by_whoever_may_save_a_view_and_by_nobody_who_only_reads() {
    let rig = shop().await;
    assert_eq!(applied(&rig, &rig.token, &stock(true, "")).await.0, "ok");
    let guarded = "blueprint: guarded\nsteps:\n  made:\n    do: collection.ensure\n    with:\n      name: guarded\n      takes: shell\n      fields:\n        title: { type: text, required: true }\n        points: { type: number }\n";
    assert_eq!(applied(&rig, &rig.token, guarded).await.0, "ok");
    let each = [
        json!({ "name": "Mine", "kind": "table", "columns": ["points", "status"] }),
        json!({ "name": "Mine", "kind": "table", "freezes": "points" }),
        json!({ "name": "Mine", "kind": "table", "figures": { "points": "sum" } }),
        json!({ "name": "Mine", "kind": "table", "widths": { "points": 140 } }),
        json!({ "name": "Mine", "kind": "table", "group": "status" }),
    ];
    // Somebody who reads and does not write saves none of it, and the table has no such view.
    let reader = key(&rig, &["projects:read", "records:read", "board:read"]).await;
    for asked in &each {
        let (status, refused) = save(&rig, &reader, asked.clone()).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{asked}: {refused}");
    }
    assert_eq!(view(&rig, "Mine").await, Value::Null);
    // Whoever may change the table's records saves each, and what was said is what is kept.
    let clerk = key(&rig, &["projects:read", "records"]).await;
    for asked in &each {
        let (status, kept) = save(&rig, &clerk, asked.clone()).await;
        assert_eq!(status, StatusCode::OK, "{asked}: {kept}");
        let saved = view(&rig, "Mine").await;
        for key in ["columns", "freezes", "figures", "widths", "group"] {
            assert_eq!(saved.get(key), asked.get(key), "`{key}` of {saved}");
        }
    }
    // What changing a table's records takes beside that, saving this of it takes too.
    let asked =
        json!({ "name": "Mine", "kind": "table", "freezes": "points", "figures": { "points": "sum" } });
    let (status, refused) =
        rig.call(Method::POST, "/api/collections/shop/guarded/views", Some(&clerk), Some(asked)).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    assert!(said(&refused).contains("shell"), "{refused}");
}

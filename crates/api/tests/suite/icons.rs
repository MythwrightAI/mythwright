//! The icon a table's column is drawn with: said where the table is
//! declared, or chosen by somebody who looks at it, and the field's
//! either way. What is chosen stands over what is declared, is kept when
//! the table is declared again, and is chosen by whoever may save a view
//! of the table and nobody else.

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;

/// A table whose first column says its icon, and a board beside it.
fn stock(fields: &str) -> String {
    format!(
        "blueprint: stock
permissions: [projects, board]
steps:
  stock:
    do: collection.ensure
    with:
      name: stock
      fields:
{fields}
  board:
    do: collection.ensure
    with:
      name: board
      fields:
        title: {{ type: text, required: true }}
        status: {{ type: choice, of: [todo, done], required: true }}
      views:
        board: {{ kind: board, by: status }}
"
    )
}

const FIRST: &str = "        title: { type: text, required: true, icon: tag }
        points: { type: number }
        shelf: { type: text }";

async fn applied(rig: &Rig, source: &str) -> Value {
    rig.send("/api/apply", json!({ "source": source, "project": "shop", "wait": true })).await
}

async fn key(rig: &Rig, scopes: &[&str]) -> String {
    let made = rig.send("/api/credentials", json!({ "name": "helper", "scopes": scopes })).await;
    made["token"].as_str().unwrap().to_string()
}

fn said(body: &Value) -> String {
    body["error"]["message"].as_str().unwrap_or_default().to_string()
}

fn icon_of(table: &str, field: &str) -> String {
    format!("/api/collections/shop/{table}/fields/{field}/icon")
}

/// Chooses a field's icon with a credential, or with none takes the choice away.
async fn choose(
    rig: &Rig,
    token: &str,
    table: &str,
    field: &str,
    icon: Option<Value>,
) -> (StatusCode, Value) {
    let (method, body) = match icon {
        Some(icon) => (Method::PUT, Some(json!({ "icon": icon }))),
        None => (Method::DELETE, None),
    };
    rig.call(method, &icon_of(table, field), Some(token), body).await
}

/// A field of a table as the daemon hands the table out.
async fn field(rig: &Rig, table: &str, name: &str) -> Option<Value> {
    let tables = rig.read("/api/collections?project=shop").await;
    let table = tables["collections"].as_array().unwrap().iter().find(|one| one["name"] == table)?.clone();
    table["fields"].as_array().unwrap().iter().find(|one| one["name"] == name).cloned()
}

#[tokio::test]
async fn a_columns_icon_is_declared_or_chosen_and_what_is_chosen_is_kept_when_the_table_is_declared_again() {
    let rig = &Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    assert_eq!(applied(rig, &stock(FIRST)).await["finished"]["status"], "ok");
    let choose = |table: &'static str, field: &'static str, icon: Value| async move {
        choose(rig, &rig.token, table, field, Some(icon)).await
    };

    // What the declaration says is the field's, and a field that says
    // nothing says nothing: its row is as a table from before has it.
    let title = field(rig, "stock", "title").await.unwrap();
    assert_eq!((&title["icon"], title.get("icon_chosen")), (&json!("tag"), None), "{title}");
    let points = field(rig, "stock", "points").await.unwrap();
    assert_eq!((points.get("icon"), points.get("icon_chosen")), (None, None), "{points}");

    // Somebody chooses one, and everybody is told which field and which icon.
    let (status, chosen) = choose("stock", "points", json!("star")).await;
    assert_eq!(status, StatusCode::OK, "{chosen}");
    let of = |table: &Value, name: &str| {
        table["fields"].as_array().unwrap().iter().find(|one| one["name"] == name).cloned().unwrap()
    };
    assert_eq!(of(&chosen, "points")["icon_chosen"], "star", "the table comes back as it now is: {chosen}");
    let told = rig.event("collection.icon", |event| event.data["field"] == "points").await;
    assert_eq!(
        told.data,
        json!({ "project": "shop", "name": "stock", "field": "points", "icon": "star" }),
        "{told:?}"
    );
    // What is chosen stands beside what is declared, which is still said.
    assert_eq!(choose("stock", "title", json!("heart")).await.0, StatusCode::OK);
    assert_eq!(choose("stock", "shelf", json!("moon")).await.0, StatusCode::OK);
    let title = field(rig, "stock", "title").await.unwrap();
    assert_eq!((&title["icon"], &title["icon_chosen"]), (&json!("tag"), &json!("heart")), "{title}");
    // A view saved after it answers with the table as it is, icons and all.
    let view = json!({ "name": "Mine", "kind": "list" });
    let saved = rig.send("/api/collections/shop/stock/views", view).await;
    assert_eq!(of(&saved, "title")["icon_chosen"], "heart", "{saved}");
    assert!(saved["views"].as_array().unwrap().iter().any(|one| one["name"] == "Mine"), "{saved}");
    // A board's column is chosen the same way.
    let (status, board) = choose("board", "status", json!("columns-3")).await;
    assert_eq!(status, StatusCode::OK, "{board}");
    assert_eq!(of(&board, "status")["icon_chosen"], "columns-3", "{board}");

    // Declared again saying other icons, and without one of its fields:
    // what was chosen is kept by the field's name, and goes with a field
    // that is gone.
    let second = "        title: { type: text, required: true, icon: bookmark }
        points: { type: number, icon: hash }";
    assert_eq!(applied(rig, &stock(second)).await["finished"]["status"], "ok");
    let title = field(rig, "stock", "title").await.unwrap();
    assert_eq!((&title["icon"], &title["icon_chosen"]), (&json!("bookmark"), &json!("heart")), "{title}");
    let points = field(rig, "stock", "points").await.unwrap();
    assert_eq!((&points["icon"], &points["icon_chosen"]), (&json!("hash"), &json!("star")), "{points}");
    assert_eq!(field(rig, "stock", "shelf").await, None);
    assert_eq!(field(rig, "board", "status").await.unwrap()["icon_chosen"], "columns-3");
    assert_eq!(applied(rig, &stock(FIRST)).await["finished"]["status"], "ok");
    let shelf = field(rig, "stock", "shelf").await.unwrap();
    assert_eq!(shelf.get("icon_chosen"), None, "a field that comes back is a new one: {shelf}");
    assert_eq!(field(rig, "stock", "points").await.unwrap()["icon_chosen"], "star");

    // Taken away, what is declared is all the field says, and that is told too.
    let mark = rig.mark();
    let (status, removed) =
        rig.call(Method::DELETE, &icon_of("stock", "title"), Some(&rig.token), None).await;
    assert_eq!(status, StatusCode::OK, "{removed}");
    let title = of(&removed, "title");
    assert_eq!((&title["icon"], title.get("icon_chosen")), (&json!("tag"), None), "{title}");
    let told = rig.event("collection.icon", |event| event.seq > mark).await;
    assert_eq!(told.data, json!({ "project": "shop", "name": "stock", "field": "title" }), "{told:?}");
    // Taking away what was never chosen is the same done twice.
    let (status, _) = rig.call(Method::DELETE, &icon_of("stock", "title"), Some(&rig.token), None).await;
    assert_eq!(status, StatusCode::OK);

    // A name is a short word, whatever set of drawings somebody has: one
    // this program has never heard of is kept, and anything else refused.
    assert_eq!(choose("stock", "points", json!("no-such-drawing-9")).await.0, StatusCode::OK);
    for wrong in [json!("Star"), json!("two words"), json!("a/b"), json!(""), json!("x".repeat(49))] {
        let (status, refused) = choose("stock", "points", wrong.clone()).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{wrong}: {refused}");
        assert!(said(&refused).contains("lowercase letters, digits and dashes"), "{refused}");
    }
    let (status, refused) = choose("stock", "points", json!(7)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");
    assert_eq!(field(rig, "stock", "points").await.unwrap()["icon_chosen"], "no-such-drawing-9");
    // And a field or a table there is not, is said not to be there.
    let (status, refused) = choose("stock", "weight", json!("star")).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{refused}");
    assert!(said(&refused).contains("no field called `weight`"), "{refused}");
    assert_eq!(choose("nothing", "title", json!("star")).await.0, StatusCode::NOT_FOUND);

    // A declaration that says something that is no name is refused where it is applied.
    let wrong = stock("        title: { type: text, required: true, icon: Not A Name }");
    let refused = applied(rig, &wrong).await;
    assert_eq!(refused["finished"]["status"], "failed", "{refused}");
    let why = refused["finished"]["error"].as_str().unwrap_or_default();
    assert!(why.contains("the field `title` says `icon`") && why.contains("lowercase letters"), "{refused}");
}

#[tokio::test]
async fn an_icon_is_chosen_by_whoever_may_save_a_view_and_a_table_kept_private_is_not_told_of() {
    let rig = &Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    let tables = "blueprint: tables
permissions: [projects, board]
steps:
  stock:
    do: collection.ensure
    with: { name: stock, fields: { title: { type: text, required: true } } }
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
    assert_eq!(applied(rig, tables).await["finished"]["status"], "ok");
    let ask = |token: &str, method: Method, table: &str| {
        let (token, table) = (token.to_string(), table.to_string());
        let icon = (method == Method::PUT).then(|| json!("star"));
        async move { choose(rig, &token, &table, "title", icon).await }
    };

    // Somebody who reads and does not write is refused, as they are a view.
    let reader = key(rig, &["projects:read", "records:read", "board:read"]).await;
    for table in ["stock", "board", "moods", "nothing"] {
        for method in [Method::PUT, Method::DELETE] {
            let (status, refused) = ask(&reader, method.clone(), table).await;
            assert_eq!(status, StatusCode::FORBIDDEN, "{method} {table}: {refused}");
            assert!(!said(&refused).contains("private"), "nothing is said of the table: {refused}");
        }
    }
    let view = json!({ "name": "Mine", "kind": "table" });
    let (status, _) =
        rig.call(Method::POST, "/api/collections/shop/stock/views", Some(&reader), Some(view.clone())).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, _) =
        rig.call(Method::PUT, &icon_of("stock", "title"), None, Some(json!({ "icon": "star" }))).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    // A member may, of a table and of the board; and whoever may change
    // records and not cards chooses a table's and not the board's.
    let member = key(rig, kernel::principal::MEMBER_SCOPES).await;
    assert_eq!(ask(&member, Method::PUT, "stock").await.0, StatusCode::OK);
    assert_eq!(ask(&member, Method::PUT, "board").await.0, StatusCode::OK);
    let clerk = key(rig, &["projects:read", "records"]).await;
    assert_eq!(ask(&clerk, Method::PUT, "stock").await.0, StatusCode::OK);
    let (status, refused) = ask(&clerk, Method::PUT, "board").await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    // What changing a table's records takes beside that, this takes too.
    let (status, refused) = ask(&clerk, Method::PUT, "guarded").await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    assert!(said(&refused).contains("shell"), "{refused}");
    assert_eq!(field(rig, "guarded", "title").await.unwrap().get("icon_chosen"), None);

    // A table kept from whoever asks is not there for them: the words
    // are those of a table the project has not got, for choosing and
    // for taking away.
    for method in [Method::PUT, Method::DELETE] {
        let (status, kept) = ask(&clerk, method.clone(), "moods").await;
        let (missing_status, missing) = ask(&clerk, method.clone(), "nothing").await;
        assert_eq!((status, missing_status), (StatusCode::NOT_FOUND, StatusCode::NOT_FOUND), "{kept}");
        assert_eq!(said(&kept).replace("moods", "nothing"), said(&missing), "{method}");
    }
    assert_eq!(field(rig, "moods", "title").await.unwrap().get("icon_chosen"), None);
    // Its owner chooses, and of that only whoever may read the table hears.
    assert_eq!(ask(&rig.token, Method::PUT, "moods").await.0, StatusCode::OK);
    let heard = |events: &Value| -> Vec<String> {
        let events = events["events"].as_array().unwrap().iter();
        events.map(|event| event["data"]["name"].as_str().unwrap_or("-").to_string()).collect()
    };
    let topics = "/api/events/recent?topics=collection.icon&limit=100";
    assert!(heard(&rig.read(topics).await).iter().any(|name| name == "moods"));
    let (status, events) = rig.call(Method::GET, topics, Some(&clerk), None).await;
    assert_eq!(status, StatusCode::OK, "{events}");
    let clerk_heard = heard(&events);
    assert!(clerk_heard.iter().any(|name| name == "stock"), "{clerk_heard:?}");
    assert!(!clerk_heard.iter().any(|name| name == "moods"), "{clerk_heard:?}");
}

#[tokio::test(flavor = "multi_thread")]
async fn icons_chosen_side_by_side_with_a_view_being_saved_are_all_kept() {
    let rig = &Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    assert_eq!(applied(rig, &stock(FIRST)).await["finished"]["status"], "ok");
    // Each is a part of the one row a table is kept as: written whole
    // from what was read a moment before, the last would take the others away.
    let choose = |field: &'static str, icon: &'static str| async move {
        choose(rig, &rig.token, "stock", field, Some(json!(icon))).await
    };
    let save = |name: &'static str| async move {
        let view = json!({ "name": name, "kind": "list" });
        rig.call(Method::POST, "/api/collections/shop/stock/views", Some(&rig.token), Some(view)).await
    };
    for round in 0..8 {
        let (icons, views) = (["star", "heart", "moon"], ["Mine", "Yours"]);
        let (icon, view) = (icons[round % 3], views[round % 2]);
        let all =
            tokio::join!(choose("title", icon), save(view), choose("points", icon), choose("shelf", icon));
        for (status, answer) in [all.0, all.1, all.2, all.3] {
            assert_eq!(status, StatusCode::OK, "{answer}");
        }
        for name in ["title", "points", "shelf"] {
            let kept = field(rig, "stock", name).await.unwrap();
            assert_eq!(kept["icon_chosen"], icon, "round {round}, `{name}`: {kept}");
        }
        let tables = rig.read("/api/collections?project=shop").await;
        let stock = tables["collections"].as_array().unwrap().iter().find(|one| one["name"] == "stock");
        let views = stock.unwrap()["views"].as_array().unwrap();
        assert!(views.iter().any(|one| one["name"] == view), "round {round}: {views:?}");
    }
}

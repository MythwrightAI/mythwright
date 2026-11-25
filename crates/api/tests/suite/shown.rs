//! What a table says of how it is shown: that it sits beside another of
//! its project (`beside`), which of its fields says what of that a row
//! is about (`about`), its columns in groups (`groups`), and that its
//! rows are not written by hand (`by_hand: false`). Said where the table
//! is declared, read back with the table, and nothing more: with a key
//! that holds less, a table that sits beside the board is read and
//! written by exactly whoever read and wrote it before, and so is the
//! board; and one whose rows are not written by hand is written by every
//! door that writes any table's, for exactly whoever writes those.

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;
use crate::held::{applied, key, said};

/// A table declared by a blueprint, with `more` said of it beside its fields.
fn table(name: &str, more: &str) -> String {
    format!(
        "blueprint: {name}\nsteps:\n  made:\n    do: collection.ensure\n    with:\n      name: {name}\n{more}      fields:\n        title: {{ type: text, required: true }}\n        column: {{ type: text }}\n"
    )
}

/// A shop with a board.
async fn shop() -> Rig {
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    let board = json!({ "blueprint": "std/board", "project": "shop", "wait": true });
    assert_eq!(rig.send("/api/apply", board).await["finished"]["status"], "ok");
    rig
}

/// What the shop's table `name` says of itself to whoever reads its tables.
async fn declared(rig: &Rig, name: &str) -> Value {
    let tables = rig.read("/api/collections?project=shop").await;
    let found = tables["collections"].as_array().unwrap().iter().find(|one| one["name"] == name).cloned();
    found.unwrap_or_else(|| panic!("the shop has no table called {name}: {tables}"))
}

#[tokio::test]
async fn a_blueprint_says_a_table_sits_beside_another_and_what_a_row_is_about_and_the_table_says_so_to_whoever_reads_it()
 {
    let rig = shop().await;
    let stages = table("stages", "      beside: board\n      about: column\n");
    assert_eq!(applied(&rig, &rig.token, &stages).await, ("ok".to_string(), String::new()));
    let said_of = declared(&rig, "stages").await;
    assert_eq!((&said_of["beside"], &said_of["about"]), (&json!("board"), &json!("column")), "{said_of}");
    // One that is beside it and about nothing, one that says nothing, and
    // the board, which says nothing of what sits beside it.
    assert_eq!(applied(&rig, &rig.token, &table("agents", "      beside: board\n")).await.0, "ok");
    let agents = declared(&rig, "agents").await;
    assert_eq!((&agents["beside"], agents.get("about")), (&json!("board"), None), "{agents}");
    assert_eq!(applied(&rig, &rig.token, &table("notes", "")).await.0, "ok");
    for name in ["notes", "board"] {
        let plain = declared(&rig, name).await;
        assert!(plain.get("beside").is_none() && plain.get("about").is_none(), "{plain}");
    }

    // Declared again saying nothing, it sits beside nothing: by whoever
    // may declare a table, since nothing was guarded by its saying so.
    let narrow = key(&rig, &["projects", "records", "blueprints"]).await;
    assert_eq!(applied(&rig, &narrow, &table("stages", "")).await.0, "ok");
    assert!(declared(&rig, "stages").await.get("beside").is_none());
    assert_eq!(applied(&rig, &narrow, &stages).await.0, "ok");
    assert_eq!(declared(&rig, "stages").await["beside"], "board");

    // What it cannot say is refused where the blueprint is applied, each
    // in its sentence, and the table is left as it was.
    for (more, why) in [
        (
            "      beside: stages\n",
            "`stages` says it sits beside `stages`, which is itself; `beside` names another table of the same \
             project, as in `beside: board`",
        ),
        (
            "      beside: board\n      about: lane\n",
            "`stages` says what a row of it is about is in its field `lane` (`about`), and it has no field called \
             that; it has: column, title",
        ),
        (
            "      beside: depot/board\n",
            "`beside` names another table of the same project, as in `beside: board`; `depot/board` is not a \
             table's name, and a table sits beside one of its own project and of no other",
        ),
        (
            "      beside: board\n      groups: [{ Where: [column, lane] }]\n",
            "the group `Where` of `stages` names `lane`, and `stages` has no field called that; it has: column, title",
        ),
        (
            "      groups: [{ Where: [column] }, { Again: [title, column] }]\n",
            "`column` of `stages` is named in the group `Where` and again in `Again`: a field is drawn in one \
             place, under one heading",
        ),
        (
            "      groups: { Where: [column] }\n",
            "`groups` is a list of headings in the order they are drawn, each with the fields under it in order",
        ),
        (
            "      about: column\n",
            "`about: column` says which field says what a row is about of the table this one sits beside, and no \
             `beside` says which table that is",
        ),
    ] {
        let (status, refused) = applied(&rig, &rig.token, &table("stages", more)).await;
        assert_eq!(status, "failed", "{more}");
        assert!(refused.contains(why), "{more}: {refused}");
    }
    let kept = declared(&rig, "stages").await;
    assert_eq!((&kept["beside"], &kept["about"]), (&json!("board"), &json!("column")), "{kept}");

    // Its columns in groups are said in the order they were written, of
    // the groups and of the fields in each, whatever the fields are called.
    let grouped =
        table("stages", "      beside: board\n      groups:\n        - Where it is: [title, column]\n");
    assert_eq!(applied(&rig, &rig.token, &grouped).await.0, "ok");
    assert_eq!(
        declared(&rig, "stages").await["groups"],
        json!([{ "name": "Where it is", "fields": ["title", "column"] }])
    );
    // And what has a page of its own under its project sits beside
    // nothing: a library is refused the word where it is declared.
    let library = "blueprint: docs\nsteps:\n  made:\n    do: collection.ensure\n    with:\n      name: guides\n      store: documents\n      beside: board\n      fields:\n        title: { type: text }\n";
    let (status, refused) = applied(&rig, &rig.token, library).await;
    assert_eq!(status, "failed");
    assert!(
        refused.contains(
            "`guides` says it sits beside `board`, and a board, a library and mockups each have a page of their \
             own under their project: only a table of plain records sits beside another"
        ),
        "{refused}"
    );
}

/// Everything `token` asks of the table `name` and of the board, each
/// with how it was answered: the status, and why where it was refused,
/// with the table's name taken out so that two tables can be held alike.
async fn asked(rig: &Rig, name: &str, token: &str) -> Vec<(String, StatusCode, String)> {
    let mut all = Vec::new();
    let first = rig.read(&format!("/api/collections/shop/{name}/records")).await;
    let first = first["records"][0]["id"].as_str().unwrap().to_string();
    let row = json!({ "fields": { "title": "work", "column": "ready" } });
    let (status, added) = rig
        .call(Method::POST, &format!("/api/collections/shop/{name}/records"), Some(token), Some(row))
        .await;
    all.push(("add".to_string(), status, said(&added)));
    let mut asks = vec![
        (Method::GET, format!("/api/collections/shop/{name}/records"), None),
        (Method::GET, format!("/api/collections/shop/{name}/commented"), None),
        (Method::GET, format!("/api/records/{first}"), None),
        (Method::GET, format!("/api/records/{first}/history"), None),
        (Method::PATCH, format!("/api/records/{first}"), Some(json!({ "fields": { "column": "doing" } }))),
        (Method::POST, format!("/api/records/{first}/comments"), Some(json!({ "body": "Seen." }))),
        (
            Method::POST,
            format!("/api/collections/shop/{name}/views"),
            Some(json!({ "name": "mine", "kind": "list" })),
        ),
        (Method::GET, "/api/board/shop".to_string(), None),
        (Method::POST, "/api/board/shop/items".to_string(), Some(json!({ "title": "A card" }))),
    ];
    if let Some(id) = added["id"].as_str() {
        asks.push((Method::DELETE, format!("/api/records/{id}"), None));
    }
    for (method, path, body) in asks {
        let (status, answer) = rig.call(method.clone(), &path, Some(token), body).await;
        all.push((format!("{method} {path}"), status, said(&answer)));
    }
    let plain = |words: String| words.replace(&first, "the row").replace(name, "the table");
    all.into_iter().map(|(what, status, why)| (plain(what), status, plain(why))).collect()
}

#[tokio::test]
async fn a_table_that_sits_beside_the_board_is_read_and_written_by_whoever_read_and_wrote_it_before_and_so_is_the_board()
 {
    let rig = shop().await;
    let history = "      history: [\"record.*\"]\n";
    // Two tables alike but for the one thing, and two that are kept
    // private alike but for it.
    for (name, more) in [
        ("beside", "      beside: board\n      about: column\n"),
        ("apart", ""),
        ("sealed", "      beside: board\n      about: column\n      reads: private:notes\n"),
        ("closed", "      reads: private:notes\n"),
    ] {
        let text = table(name, &format!("{more}{history}"));
        assert_eq!(applied(&rig, &rig.token, &text).await, ("ok".to_string(), String::new()), "{name}");
        let row = json!({ "fields": { "title": "review", "column": "review" } });
        rig.send(&format!("/api/collections/shop/{name}/records"), row).await;
    }
    let keys: [&[&str]; 7] = [
        &["projects", "records", "board"],
        &["projects", "records", "board", "private:notes"],
        &["projects:read", "records:read", "board:read"],
        &["projects:read", "records"],
        &["projects:read", "board"],
        &["projects:read", "records:read", "private:notes"],
        &["search"],
    ];
    for scopes in keys {
        let token = key(&rig, scopes).await;
        for (beside, apart) in [("beside", "apart"), ("sealed", "closed")] {
            let (of_beside, of_apart) = (asked(&rig, beside, &token).await, asked(&rig, apart, &token).await);
            assert_eq!(of_beside, of_apart, "{scopes:?}, {beside}");
        }
    }
    // The comparison was of something: the first key wrote the open
    // tables and was kept from the private ones, and the second wrote both.
    let writes = key(&rig, keys[0]).await;
    let open = asked(&rig, "beside", &writes).await;
    assert!(open.iter().all(|(_, status, _)| *status == StatusCode::OK), "{open:?}");
    let kept = asked(&rig, "sealed", &writes).await;
    assert!(kept.iter().any(|(_, status, _)| *status != StatusCode::OK), "{kept:?}");
    let holds = key(&rig, keys[1]).await;
    let kept = asked(&rig, "sealed", &holds).await;
    assert!(kept.iter().all(|(_, status, _)| *status == StatusCode::OK), "{kept:?}");

    // And what is listed of a project's tables to somebody a table is
    // kept from leaves it out, beside the board or not.
    let reads = key(&rig, keys[2]).await;
    let (status, tables) = rig.call(Method::GET, "/api/collections?project=shop", Some(&reads), None).await;
    assert_eq!(status, StatusCode::OK, "{tables}");
    let mut names: Vec<&str> =
        tables["collections"].as_array().unwrap().iter().map(|one| one["name"].as_str().unwrap()).collect();
    names.sort_unstable();
    assert_eq!(names, ["apart", "beside", "board"], "{tables}");
}

#[tokio::test]
async fn a_table_whose_rows_are_not_written_by_hand_says_so_and_every_door_writes_its_rows_as_it_writes_any_tables()
 {
    let rig = shop().await;
    // Said where it is declared, read back with the table; `true` is what
    // saying nothing says, and so is said by nothing.
    let runs = table("runs", "      by_hand: false\n      history: [\"record.*\"]\n");
    assert_eq!(applied(&rig, &rig.token, &runs).await, ("ok".to_string(), String::new()));
    assert_eq!(declared(&rig, "runs").await["by_hand"], json!(false));
    let loose = table("loose", "      by_hand: true\n      history: [\"record.*\"]\n");
    assert_eq!(applied(&rig, &rig.token, &loose).await.0, "ok");
    assert!(declared(&rig, "loose").await.get("by_hand").is_none());
    // A blueprint's step adds a row to it, as a script's command does.
    let adds = "blueprint: adds\nsteps:\n  added:\n    do: record.add\n    with:\n      collection: runs\n      fields: { title: review, column: passed }\n";
    assert_eq!(applied(&rig, &rig.token, adds).await, ("ok".to_string(), String::new()));
    rig.send(
        "/api/collections/shop/loose/records",
        json!({ "fields": { "title": "review", "column": "passed" } }),
    )
    .await;
    // Whoever may write records, whoever may only read them and whoever
    // holds nothing of them are each answered alike of the two tables, by
    // every route that adds, changes, reads, says something on and takes
    // away a row.
    for scopes in [&["projects", "records", "board"][..], &["projects:read", "records:read"], &["search"]] {
        let token = key(&rig, scopes).await;
        assert_eq!(asked(&rig, "runs", &token).await, asked(&rig, "loose", &token).await, "{scopes:?}");
    }
    let writes = key(&rig, &["projects", "records", "board"]).await;
    let wrote = asked(&rig, "runs", &writes).await;
    assert!(wrote.iter().all(|(_, status, _)| *status == StatusCode::OK), "{wrote:?}");
    // Declared again saying nothing, by whoever may declare a table, it
    // is any table again; and saying something that is no `true` or
    // `false` is refused before the blueprint is applied.
    let narrow = key(&rig, &["projects", "records", "blueprints"]).await;
    assert_eq!(applied(&rig, &narrow, &table("runs", "")).await.0, "ok");
    assert!(declared(&rig, "runs").await.get("by_hand").is_none());
    assert_eq!(applied(&rig, &narrow, &runs).await.0, "ok");
    assert_eq!(declared(&rig, "runs").await["by_hand"], json!(false));
    let asked =
        json!({ "source": table("runs", "      by_hand: sometimes\n"), "project": "shop", "wait": true });
    let (status, refused) = rig.call(Method::POST, "/api/apply", Some(&rig.token), Some(asked)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");
    assert!(said(&refused).contains("`by_hand` must be bool"), "{refused}");
    assert_eq!(declared(&rig, "runs").await["by_hand"], json!(false), "as it was");
}

//! A table kept by a store service: a database that is not the
//! installation's own, behind an address.
//!
//! The store here is the one that ships, served in this process from a file
//! of the test's own. What hosts it for real is a blueprint, and that is
//! tested where the real program can be run (the command line's tests).

use std::path::PathBuf;

use api::store::StoreServer;
use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;

/// Where a rig's store keeps its records.
fn file(rig: &Rig) -> PathBuf {
    rig.dir.join("store").join("cards.db")
}

async fn store(rig: &Rig) -> StoreServer {
    StoreServer::start(&file(rig), "127.0.0.1:0".parse().unwrap()).await.unwrap()
}

/// The same at a port that is the test's own for as long as `port` is
/// kept (`crate::ports`): for a store whose address is asked, or come
/// back to, once the store has stopped. An address the system gave is
/// the system's to give again from that moment, and with suites starting
/// nodes side by side something else was now and then listening there.
async fn store_at(rig: &Rig, port: &crate::ports::Port) -> StoreServer {
    let address = std::net::SocketAddr::from(([127, 0, 0, 1], port.number()));
    StoreServer::start(&file(rig), address).await.unwrap()
}

/// Applies `std/board` to a project, as kept by the store at `database`.
async fn board_at(rig: &Rig, project: &str, database: &str, columns: &[&str]) -> Value {
    let params = json!({ "store": "service", "database": database, "columns": columns });
    let body = json!({ "blueprint": "std/board", "project": project, "params": params, "wait": true });
    rig.send("/api/apply", body).await
}

const COLUMNS: &[&str] = &["backlog", "doing", "done"];

/// What the store's own database holds of a project's board, read from the
/// file and not through anything that serves it: each card's id, title
/// and column, in the order they were made.
fn in_the_file(rig: &Rig, project: &str) -> Vec<(String, String, String)> {
    let database = rusqlite::Connection::open(file(rig)).unwrap();
    let mut rows = database
        .prepare("SELECT id, data FROM records WHERE project = ?1 AND collection = 'board' ORDER BY number")
        .unwrap();
    rows.query_map([project], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))
        .unwrap()
        .map(|row| {
            let (id, data) = row.unwrap();
            let data: Value = serde_json::from_str(&data).unwrap();
            (id, data["title"].as_str().unwrap().to_string(), data["status"].as_str().unwrap().to_string())
        })
        .collect()
}

/// How many records the installation's own databases hold, of anything:
/// the factory's, and each project's that keeps its records.
async fn in_the_installation(rig: &Rig) -> i64 {
    let mut held = 0;
    for journal in rig.node().app.journal.every_records() {
        let count = |database: &mut rusqlite::Connection| {
            database.query_row("SELECT count(*) FROM records", [], |row| row.get::<_, i64>(0))
        };
        held += journal.db().call(count).await.unwrap();
    }
    held
}

async fn change(rig: &Rig, id: &str, body: Value) -> Value {
    let (status, changed) =
        rig.call(Method::PATCH, &format!("/api/board/items/{id}"), Some(&rig.token), Some(body)).await;
    assert_eq!(status, StatusCode::OK, "{changed}");
    changed
}

fn titles(board: &Value) -> Vec<(&str, &str)> {
    board["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| (item["title"].as_str().unwrap(), item["status"].as_str().unwrap()))
        .collect()
}

#[tokio::test]
async fn a_board_kept_by_a_store_service_has_its_cards_in_the_stores_database_and_not_the_installations() {
    let rig = Rig::new().await;
    let store = store(&rig).await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    let applied = board_at(&rig, "shop", &store.url(), COLUMNS).await;
    assert_eq!(applied["finished"]["status"], "ok", "{}", applied["finished"]);
    assert_eq!(applied["finished"]["outputs"]["store"], "service");
    assert_eq!(applied["finished"]["outputs"]["database"], store.url());

    // The table says where it is kept, for the client to say so too.
    let declared = rig.read("/api/collections?project=shop").await;
    assert_eq!(declared["collections"][0]["store"], "service");
    assert_eq!(declared["collections"][0]["at"], store.url());

    // Added, moved, changed and removed through the API, like any board.
    let mugs = rig.send("/api/board/shop/items", json!({ "title": "Sell mugs" })).await;
    let boxes = rig.send("/api/board/shop/items", json!({ "title": "Pack boxes", "status": "doing" })).await;
    let gone = rig.send("/api/board/shop/items", json!({ "title": "Print flyers" })).await;
    let (mugs, boxes, gone) =
        (mugs["id"].as_str().unwrap(), boxes["id"].as_str().unwrap(), gone["id"].as_str().unwrap());
    assert_eq!((mugs, boxes, gone), ("shop-1", "shop-2", "shop-3"), "numbered as on any board");
    assert_eq!(change(&rig, mugs, json!({ "status": "done" })).await["status"], "done");
    let renamed = change(&rig, boxes, json!({ "title": "Pack the boxes", "labels": ["urgent"] })).await;
    assert_eq!(renamed["labels"], json!(["urgent"]));
    let (status, _) =
        rig.call(Method::DELETE, &format!("/api/board/items/{gone}"), Some(&rig.token), None).await;
    assert_eq!(status, StatusCode::OK);

    // They come back from the store,
    let board = rig.read("/api/board/shop").await;
    assert_eq!(titles(&board), [("Pack the boxes", "doing"), ("Sell mugs", "done")]);
    // because that is where they are: in its file,
    let kept = in_the_file(&rig, "shop");
    let expected = [("shop-1", "Sell mugs", "done"), ("shop-2", "Pack the boxes", "doing")]
        .map(|(id, title, status)| (id.to_string(), title.to_string(), status.to_string()));
    assert_eq!(kept, expected);
    // and not in the installation's own database.
    assert_eq!(in_the_installation(&rig).await, 0);

    // Another project's board is kept as it always was, beside it.
    rig.send("/api/projects", json!({ "name": "Blog" })).await;
    rig.board("blog").await;
    rig.send("/api/board/blog/items", json!({ "title": "Write the first post" })).await;
    assert_eq!(in_the_installation(&rig).await, 1);
    assert_eq!(in_the_file(&rig, "blog"), []);
    store.stop().await;
}

#[tokio::test]
async fn applying_the_boards_blueprint_again_keeps_the_cards_and_does_not_move_a_board_between_stores() {
    let rig = Rig::new().await;
    let store = store(&rig).await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    board_at(&rig, "shop", &store.url(), COLUMNS).await;
    rig.send("/api/board/shop/items", json!({ "title": "Sell mugs" })).await;
    rig.send("/api/board/shop/items", json!({ "title": "Pack boxes", "status": "doing" })).await;

    // The same again, and again with one more column: the cards are where they were.
    let same = board_at(&rig, "shop", &store.url(), COLUMNS).await;
    assert_eq!(same["changed"], false, "{same}");
    let wider = board_at(&rig, "shop", &store.url(), &["backlog", "doing", "review", "done"]).await;
    assert_eq!(wider["finished"]["status"], "ok", "{}", wider["finished"]);
    let board = rig.read("/api/board/shop").await;
    assert_eq!(board["columns"], json!(["backlog", "doing", "review", "done"]));
    assert_eq!(titles(&board), [("Sell mugs", "backlog"), ("Pack boxes", "doing")]);
    assert_eq!(rig.read("/api/collections?project=shop").await["collections"][0]["at"], store.url());

    // Changing the columns from the board itself leaves it where it is kept.
    rig.send("/api/board/shop/columns", json!({ "columns": ["backlog", "doing", "done"] })).await;
    let declared = rig.read("/api/collections?project=shop").await;
    assert_eq!(
        (&declared["collections"][0]["store"], &declared["collections"][0]["at"]),
        (&json!("service"), &json!(store.url()))
    );

    // A board that is in the installation's database is not moved to a
    // store by saying so: where it is kept was settled when it was made.
    rig.send("/api/projects", json!({ "name": "Blog" })).await;
    rig.board("blog").await;
    rig.send("/api/board/blog/items", json!({ "title": "Write the first post" })).await;
    let refused = board_at(&rig, "blog", &store.url(), &["draft", "ready", "doing", "review", "done"]).await;
    assert_eq!(refused["finished"]["status"], "failed", "{}", refused["finished"]);
    assert!(
        refused["finished"]["error"]
            .as_str()
            .unwrap()
            .contains("`board` is kept in `local` and cannot be moved to `service` by declaring it again"),
        "{}",
        refused["finished"]
    );
    assert_eq!(titles(&rig.read("/api/board/blog").await), [("Write the first post", "draft")]);
    assert_eq!(in_the_file(&rig, "blog"), [], "and nothing of it went to the store");
    // Nor the other way.
    let params = json!({ "store": "local", "columns": COLUMNS });
    let body = json!({ "blueprint": "std/board", "project": "shop", "params": params, "wait": true });
    let refused = rig.send("/api/apply", body).await;
    assert!(
        refused["finished"]["error"]
            .as_str()
            .unwrap()
            .contains("`board` is kept in `service` and cannot be moved to `local`"),
        "{}",
        refused["finished"]
    );
    store.stop().await;
}

#[tokio::test]
async fn when_the_store_service_is_not_answering_the_board_says_so_and_works_again_when_it_is_back() {
    let rig = Rig::new().await;
    let own = crate::ports::free();
    let store = store_at(&rig, &own).await;
    let (address, url) = (store.addr, store.url());
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    board_at(&rig, "shop", &url, COLUMNS).await;
    let card = rig.send("/api/board/shop/items", json!({ "title": "Sell mugs" })).await;
    let id = card["id"].as_str().unwrap();

    store.stop().await;
    let said = |refused: &Value| {
        let message = refused["error"]["message"].as_str().unwrap().to_string();
        assert_eq!(refused["error"]["kind"], "unavailable", "{refused}");
        assert!(
            message.starts_with(&format!(
                "`board` of `shop` is kept by a store service at {url}, and nothing is answering there."
            )),
            "{message}"
        );
        assert!(message.contains("Start the blueprint that hosts the store"), "{message}");
    };
    let (status, refused) = rig.get("/api/board/shop").await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{refused}");
    said(&refused);
    let (status, refused) = rig.post("/api/board/shop/items", json!({ "title": "Pack boxes" })).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{refused}");
    said(&refused);
    let (status, refused) = rig
        .call(
            Method::PATCH,
            &format!("/api/board/items/{id}"),
            Some(&rig.token),
            Some(json!({ "status": "done" })),
        )
        .await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{refused}");
    // What the table is, is still known: only its records are elsewhere.
    assert_eq!(rig.read("/api/collections?project=shop").await["collections"][0]["store"], "service");
    // A board cannot be declared against a store that is not there, either.
    rig.send("/api/projects", json!({ "name": "Blog" })).await;
    let unmade = board_at(&rig, "blog", &url, COLUMNS).await;
    assert_eq!(unmade["finished"]["status"], "failed", "{}", unmade["finished"]);
    assert!(
        unmade["finished"]["error"].as_str().unwrap().contains("nothing is answering there"),
        "{}",
        unmade["finished"]
    );
    assert_eq!(rig.read("/api/collections?project=blog").await["collections"], json!([]));

    // Back where it was, with what it had, and the board is as it was left.
    let store = StoreServer::start(&file(&rig), address).await.unwrap();
    assert_eq!(titles(&rig.read("/api/board/shop").await), [("Sell mugs", "backlog")]);
    assert_eq!(change(&rig, id, json!({ "status": "done" })).await["status"], "done");
    rig.send("/api/board/shop/items", json!({ "title": "Pack boxes" })).await;
    assert_eq!(
        titles(&rig.read("/api/board/shop").await),
        [("Pack boxes", "backlog"), ("Sell mugs", "done")]
    );
    // How the store answered, and that it did not, was measured.
    rig.node().app.journal.flush_metrics().await.unwrap();
    let measured = rig.read("/api/metrics?name=store.request&by=op").await;
    assert!(measured.to_string().contains("\"list\""), "{measured}");
    // By what it says of itself, none of which is taken for a label.
    assert_eq!(rig.read("/api/metrics/labels").await, json!({ "labels": {} }));
    store.stop().await;
}

#[tokio::test]
async fn a_store_that_is_started_somewhere_else_is_found_by_saying_its_address_again() {
    let rig = Rig::new().await;
    let store = store(&rig).await;
    let before = store.url();
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    board_at(&rig, "shop", &before, COLUMNS).await;
    rig.send("/api/board/shop/items", json!({ "title": "Sell mugs" })).await;

    // The same database, answering at another address.
    store.stop().await;
    let store = self::store(&rig).await;
    assert_ne!(store.url(), before);
    let moved = board_at(&rig, "shop", &store.url(), COLUMNS).await;
    assert_eq!(moved["finished"]["status"], "ok", "{}", moved["finished"]);
    assert_eq!(rig.read("/api/collections?project=shop").await["collections"][0]["at"], store.url());
    assert_eq!(titles(&rig.read("/api/board/shop").await), [("Sell mugs", "backlog")]);

    // An address is of a store service, and is not said of a board kept elsewhere.
    let params = json!({ "store": "service", "database": "cards.db", "columns": COLUMNS });
    let body = json!({ "blueprint": "std/board", "project": "shop", "params": params, "wait": true });
    let refused = rig.send("/api/apply", body).await;
    assert!(
        refused["finished"]["error"]
            .as_str()
            .unwrap()
            .contains("`cards.db` is not the address of a store service"),
        "{}",
        refused["finished"]
    );
    rig.send("/api/projects", json!({ "name": "Blog" })).await;
    let params = json!({ "database": store.url() });
    let body = json!({ "blueprint": "std/board", "project": "blog", "params": params, "wait": true });
    let refused = rig.send("/api/apply", body).await;
    assert!(
        refused["finished"]["error"].as_str().unwrap().contains("say `store: service`, or leave `at` out"),
        "{}",
        refused["finished"]
    );
    store.stop().await;
}

#[tokio::test]
async fn a_cards_history_is_kept_by_the_installation_for_a_board_in_a_store_service() {
    let rig = Rig::new().await;
    let store = store(&rig).await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    board_at(&rig, "shop", &store.url(), COLUMNS).await;
    let card = rig.send("/api/board/shop/items", json!({ "title": "Sell mugs" })).await;
    let id = card["id"].as_str().unwrap().to_string();
    change(&rig, &id, json!({ "status": "doing" })).await;
    rig.event("collection.history", |event| {
        event.data["record"] == id.as_str() && event.data["project"] == "shop"
    })
    .await;
    let moved = rig.event("board.item.moved", |event| event.data["id"] == id.as_str()).await;
    let history = soon_said(&rig, &id, "Ada moved it from backlog to doing", moved.seq).await;
    assert!(history.contains(&"Ada added it to backlog".to_string()), "{history:?}");

    // It is the installation's to keep: the store was handed records and nothing else.
    let database = rusqlite::Connection::open(file(&rig)).unwrap();
    let tables: Vec<String> = database
        .prepare("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert_eq!(tables, ["records", "schema_versions"]);

    // And it outlives the card, and the store.
    rig.call(Method::DELETE, &format!("/api/board/items/{id}"), Some(&rig.token), None).await;
    store.stop().await;
    let history = rig.read(&format!("/api/records/{id}/history")).await;
    assert!(history.to_string().contains("Ada moved it from backlog to doing"), "{history}");
}

/// A record's history once it holds `line`, which is written as the event
/// numbered `seq` is read: waited for by the events that say a line was
/// written, not by looking again.
async fn soon_said(rig: &Rig, id: &str, line: &str, seq: u64) -> Vec<String> {
    let mut written = rig.follow_after(seq, "collection.history");
    crate::soon(&format!("`{line}` in the history of {id}"), async {
        loop {
            let history = rig.read(&format!("/api/records/{id}/history")).await;
            let said: Vec<String> = history["history"]
                .as_array()
                .unwrap()
                .iter()
                .map(|one| one["said"].as_str().unwrap().to_string())
                .collect();
            if said.iter().any(|one| one == line) {
                return said;
            }
            written.next().await.unwrap();
        }
    })
    .await
}

/// A process that is being started has, until it becomes the program it was
/// started as, a copy of everything its parent has open: a listening socket
/// among it. Closing the socket in the parent does not end it then, and for
/// that instant it still takes connections that nobody will ever answer.
/// These tests start processes all the while, so a store that had "stopped"
/// was sometimes still taking them, and the board said it "could not be
/// reached" where nothing was answering at all.
///
/// Unix's alone: it is of what a program being started holds of what
/// its parent has open, which is how Unix starts one; and it asks three
/// hundred times of an address nothing listens at, which Windows takes
/// two seconds to refuse each time.
#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn a_store_that_has_stopped_takes_no_connection_whatever_the_process_is_starting_at_the_time() {
    let rig = Rig::new().await;
    // Something else in this process starting programs, one after another.
    let done = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let starting: Vec<_> = (0..4)
        .map(|_| {
            let done = done.clone();
            std::thread::spawn(move || {
                while !done.load(std::sync::atomic::Ordering::Relaxed) {
                    let _ = std::process::Command::new("true").status();
                }
            })
        })
        .collect();

    let mut taken = 0;
    for _ in 0..300 {
        // At an address that is this test's own, so that what answers
        // there once the store has stopped, or is in its way as it comes
        // back, is the store's doing: an address the system gave was
        // the system's to give to another suite the moment it was let
        // go, and what came of that had to be let through here.
        let own = crate::ports::free();
        let store = store_at(&rig, &own).await;
        let address = store.addr;
        store.stop().await;
        let refused = match tokio::net::TcpStream::connect(address).await {
            Err(err) if err.kind() == std::io::ErrorKind::ConnectionRefused => true,
            other => {
                taken += 1;
                drop(other);
                false
            }
        };
        // And its address is free at once, for the store that comes back.
        match StoreServer::start(&file(&rig), address).await {
            Ok(back) => back.stop().await,
            Err(err) => panic!(
                "the store's address was not free for it to come back to (refused meanwhile: {refused}): {}",
                err.message
            ),
        }
    }
    done.store(true, std::sync::atomic::Ordering::Relaxed);
    for thread in starting {
        thread.join().unwrap();
    }
    assert_eq!(taken, 0, "a stopped store took {taken} of 300 connections");
}

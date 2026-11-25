//! A node holds a journal for each project that has one of its own,
//! beside the factory's: made when the project is added, listed in the
//! factory's database, opened when the node starts and given back when it
//! stops. A project with no journal of its own, which is every project
//! of a factory from before, is kept in the factory's. A copy of a
//! factory takes each journal's file, and a factory started from one has
//! them (`moment.rs`); one that lists a journal it does not hold is not
//! started from.

use std::path::{Path, PathBuf};

use journal::{EventQuery, Journal};
use kernel::Source;
use serde_json::{Value, json};

use crate::snapshots::{Shared, taken};
use crate::{Rig, in_time};

/// A node with its owner signed in, and where it keeps its data.
async fn rig(label: &str) -> (Rig, PathBuf) {
    let mut data = PathBuf::new();
    let rig = Rig::with(label, |config, _| data = config.data_dir.clone()).await;
    (Rig::owned(rig).await, data)
}

/// What every journal of the node is called, the factory's own first.
fn called(rig: &Rig) -> Vec<String> {
    rig.node().app.journal.every().iter().map(|journal| journal.name().to_string()).collect()
}

/// What the test itself said in `journal`: what is said of a project by
/// everything else is kept there too (that it was added, to begin with).
async fn topics(journal: &Journal) -> Vec<String> {
    let all = EventQuery { limit: usize::MAX, ..Default::default() };
    let said = journal.read(all).await.unwrap().into_iter().map(|event| event.topic);
    said.filter(|topic| topic.starts_with("kept.") || topic.starts_with("said.")).collect()
}

/// The database files in the folder a factory keeps its projects' journals in.
fn files(data: &Path) -> Vec<String> {
    let mut files: Vec<String> = std::fs::read_dir(data.join("journals"))
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| name.ends_with(".db"))
        .collect();
    files.sort();
    files
}

#[tokio::test]
async fn a_project_that_is_added_has_a_journal_of_its_own_and_has_it_when_the_daemon_starts_again() {
    if crate::one_journal() {
        return;
    }
    let (mut rig, data) = rig("journals-added").await;
    assert_eq!(called(&rig), ["factory"], "a factory with no project has its own journal and no other");
    assert!(files(&data).is_empty());

    for name in ["Shop", "Yard"] {
        rig.send("/api/projects", json!({ "name": name })).await;
    }
    assert_eq!(called(&rig), ["factory", "project:shop", "project:yard"]);
    assert_eq!(files(&data), ["shop.db", "yard.db"]);
    // Which projects have one is written down in the factory's own.
    let listed = rig.node().app.journal.db().call(|connection| journal::listed(connection)).await.unwrap();
    let rows: Vec<(&str, &str)> =
        listed.iter().map(|one| (one.project.as_str(), one.file.as_str())).collect();
    assert_eq!(rows, [("shop", "shop.db"), ("yard", "yard.db")]);

    // Something kept in a project's own is there when the daemon is back.
    let shop = rig.node().app.journal.of_project("shop").unwrap();
    let said = shop.emit("kept.here", Source::actor("system"), json!({ "project": "shop" })).await.unwrap();
    rig.restart().await;
    assert_eq!(called(&rig), ["factory", "project:shop", "project:yard"]);
    let shop = rig.node().app.journal.of_project("shop").unwrap();
    assert_eq!(topics(&shop).await, ["kept.here"]);
    // And what either says next is numbered above all that was said.
    let next = shop.emit("kept.here", Source::actor("system"), json!({ "project": "shop" })).await.unwrap();
    let started = rig.event("system.started", |_| true).await;
    assert!(next.seq > said.seq && next.seq != started.seq);
}

/// How many rows `table` has in the database `file`: none, where the
/// file has no such table.
fn rows(file: &Path, table: &str) -> i64 {
    let connection = journal::copy::Connection::open(file).unwrap();
    connection.query_row(&format!("SELECT count(*) FROM {table}"), [], |row| row.get(0)).unwrap_or(0)
}

/// Adds a card to `project`'s board and moves it on, and gives its id
/// once both are in its history.
pub(crate) async fn card(rig: &Rig, project: &str) -> String {
    let made = rig.send(&format!("/api/board/{project}/items"), json!({ "title": "Basket" })).await;
    let id = made["id"].as_str().unwrap().to_string();
    let asked = Some(json!({ "status": "doing" }));
    let path = format!("/api/board/items/{id}");
    let (status, moved) = rig.call(reqwest::Method::PATCH, &path, Some(&rig.token), asked).await;
    assert_eq!(status, reqwest::StatusCode::OK, "{moved}");
    history(rig, &id, 2).await;
    id
}

/// Waits until the card `id` has `lines` of history, and gives them.
pub(crate) async fn history(rig: &Rig, id: &str, lines: usize) -> Vec<String> {
    let mut written = rig.follow("collection.history");
    crate::soon(&format!("{lines} lines of the history of {id}"), async {
        loop {
            let kept = rig.read(&format!("/api/records/{id}/history")).await;
            let said = kept["history"].as_array().unwrap().iter();
            let said: Vec<String> = said.map(|line| line["topic"].as_str().unwrap().to_string()).collect();
            if said.len() >= lines {
                return said;
            }
            written.next().await.unwrap();
        }
    })
    .await
}

#[tokio::test]
async fn a_project_from_before_stays_in_the_factorys_journal_with_its_cards_and_one_added_beside_it_has_its_own()
 {
    if crate::one_journal() {
        return;
    }
    let (mut rig, data) = rig("journals-before").await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    rig.stop().await;
    // As a factory an earlier program made holds a project: a row of
    // its projects, and no journal but the factory's.
    {
        let connection = journal::copy::Connection::open(data.join("journal.db")).unwrap();
        assert_eq!(connection.execute("DELETE FROM journals", []).unwrap(), 1);
    }
    std::fs::remove_dir_all(data.join("journals")).unwrap();

    rig.start().await;
    assert_eq!(called(&rig), ["factory"]);
    let journal = rig.node().app.journal.clone();
    assert!(journal.of(Some("shop")).db().is(journal.db()), "what is said of it is kept in the factory's");
    assert!(rig.read("/api/projects").await.to_string().contains("\"shop\""));
    rig.board("shop").await;
    let of_shop = card(&rig, "shop").await;
    // Nothing gives it one by the daemon having started; a project
    // added now has its own, beside it.
    rig.send("/api/projects", json!({ "name": "Yard" })).await;
    assert_eq!(called(&rig), ["factory", "project:yard"]);
    assert_eq!(files(&data), ["yard.db"]);
    rig.board("yard").await;
    let of_yard = card(&rig, "yard").await;
    // Each one's table, its cards and what was said of them are in one
    // file: the factory's own for the project from before, and its own
    // for the one added now, of which the factory's holds nothing.
    let (factory, yard) = (data.join("journal.db"), data.join("journals").join("yard.db"));
    let held = |file: &Path| ["collections", "records", "record_history"].map(|table| rows(file, table));
    assert_eq!((held(&factory), held(&yard)), ([1, 1, 2], [1, 1, 2]));
    let said = |journal: Journal| async move {
        let moved = EventQuery { topics: "board.item.moved".into(), ..Default::default() };
        let all = journal.read(moved).await.unwrap();
        all.into_iter().map(|event| event.data["id"].as_str().unwrap().to_string()).collect::<Vec<_>>()
    };
    assert_eq!(said(journal.clone()).await, std::slice::from_ref(&of_shop));
    assert_eq!(said(journal.of_project("yard").unwrap()).await, std::slice::from_ref(&of_yard));

    // Started again, nothing was moved, and each is read, changed and
    // written of where it is.
    rig.restart().await;
    assert_eq!(called(&rig), ["factory", "project:yard"]);
    assert_eq!((held(&factory), held(&yard)), ([1, 1, 2], [1, 1, 2]));
    for (id, project) in [(&of_shop, "shop"), (&of_yard, "yard")] {
        let board = rig.read(&format!("/api/board/{project}")).await;
        assert_eq!(board["items"][0]["id"].as_str(), Some(id.as_str()), "{board}");
        assert_eq!(history(&rig, id, 2).await, ["board.item.created", "board.item.moved"]);
        let path = format!("/api/board/items/{id}");
        let asked = Some(json!({ "status": "review" }));
        let (status, moved) = rig.call(reqwest::Method::PATCH, &path, Some(&rig.token), asked).await;
        assert_eq!(status, reqwest::StatusCode::OK, "{moved}");
        assert_eq!(history(&rig, id, 3).await.len(), 3);
    }
    assert_eq!((held(&factory), held(&yard)), ([1, 1, 3], [1, 1, 3]));

    // Taken away and added again, the project from before is given a
    // journal of its own, which keeps what is run for it and none of
    // its tables: its cards are where they were, and are still its.
    let (status, gone) =
        rig.call(reqwest::Method::DELETE, "/api/projects/shop", Some(&rig.token), None).await;
    assert_eq!(status, reqwest::StatusCode::OK, "{gone}");
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    assert_eq!(called(&rig), ["factory", "project:shop", "project:yard"]);
    let journal = rig.node().app.journal.clone();
    assert_eq!(journal.keeps_of("shop"), journal::keeps::RUNS);
    assert_eq!(journal.keeps_of("yard"), journal::keeps::MADE_AT);
    let board = rig.read("/api/board/shop").await;
    assert_eq!(board["items"][0]["id"].as_str(), Some(of_shop.as_str()), "{board}");
    let another = card(&rig, "shop").await;
    assert_ne!(another, of_shop);
    assert_eq!(held(&factory), [1, 2, 5]);
    assert_eq!(held(&data.join("journals").join("shop.db")), [0, 0, 0]);
}

#[tokio::test]
async fn a_snapshot_that_lists_a_journal_it_does_not_hold_is_not_started_from() {
    if crate::one_journal() {
        return;
    }
    let shared = Shared::new();
    let prod = shared.factory("prod").await;
    prod.send("/api/projects", json!({ "name": "Shop" })).await;
    let to = shared.snapshot("prod-copy");
    taken(&prod, &to).await;
    std::fs::remove_file(to.join("journals").join("shop.db")).unwrap();
    let (config, _dir) = shared.config(Some("dev"), &to);
    let why = crate::snapshots::refused(config).await;
    assert!(why.contains("is not whole") && why.contains("the project `shop`"), "{why}");
}

#[tokio::test]
async fn what_a_repack_left_beside_a_projects_journal_is_taken_away_as_the_daemon_starts() {
    if crate::one_journal() {
        return;
    }
    let (mut rig, data) = rig("journals-repack").await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    rig.stop().await;
    let left = journal::rewrite::beside(&data.join("journals").join("shop.db"));
    std::fs::write(&left, vec![7u8; 3000]).unwrap();
    // With the factory stopped, writing it out again looks at each
    // file, and finds a project's as one is made now.
    let looked = api::repack::run(&data, |_| {}).unwrap();
    assert!(looked.is_none(), "there was nothing to write out again");

    in_time("the daemon to start", rig.start()).await;
    assert!(!left.exists());
    assert_eq!(rig.event("system.repack_cleared", |_| true).await.data["bytes"], 3000);
    assert_eq!(called(&rig), ["factory", "project:shop"]);
}

#[tokio::test]
async fn what_a_projects_journal_says_is_read_and_followed_with_the_factorys_from_a_place_in_each() {
    if crate::one_journal() {
        return;
    }
    let (rig, _data) = rig("journals-read").await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    let journal = rig.node().app.journal.clone();
    let shop = journal.of_project("shop").unwrap();
    let say = |journal: &Journal, n: u64| {
        let journal = journal.clone();
        async move { journal.emit("kept.here", Source::actor("system"), json!({ "n": n })).await.unwrap().seq }
    };
    let first = [say(&shop, 1).await, say(&journal, 2).await];

    // What was said of late is every journal's, the newest first, and
    // comes with the place that is now: text, and no number.
    let recent = rig.read("/api/events/recent?topics=kept.here").await;
    let said = |answer: &Value| -> Vec<u64> {
        answer["events"]
            .as_array()
            .unwrap()
            .iter()
            .map(|event| event["data"]["n"].as_u64().unwrap())
            .collect()
    };
    // (Two said in one millisecond in two journals have no order.)
    let sorted = |mut said: Vec<u64>| {
        said.sort_unstable();
        said
    };
    assert_eq!(sorted(said(&recent)), [1, 2], "{recent}");
    let cursor = recent["cursor"].as_str().unwrap().to_string();
    assert!(cursor.parse::<u64>().is_err() && cursor.contains("project:shop"), "{cursor}");
    assert_eq!(recent["last_seq"].as_u64(), first.iter().copied().max());

    // Read again from that place there is nothing; and what each says
    // next is what there is after it, whichever journal keeps it.
    let later = [say(&shop, 3).await, say(&journal, 4).await, say(&shop, 5).await];
    let after = rig.read(&format!("/api/events/recent?topics=kept.here&after={cursor}")).await;
    assert_eq!(sorted(said(&after)), [3, 4, 5], "{after}");
    // The newest first in each journal, whatever the order between them.
    let of_shop: Vec<u64> = said(&after).into_iter().filter(|n| n % 2 == 1).collect();
    assert_eq!(of_shop, [5, 3], "{after}");
    // An event's own number is still taken for a place, and of its own
    // journal means what was said after it.
    let numbered = rig.read(&format!("/api/events/recent?topics=kept.here&after={}", later[0])).await;
    assert!(said(&numbered).contains(&5) && !said(&numbered).contains(&3), "{numbered}");

    // The stream opened at the place hears what each journal said since,
    // each journal's in its order, and every frame has a place of its own.
    let stream = format!("{}/api/events?topics=kept.here&since={cursor}", rig.url);
    let mut response = reqwest::Client::new().get(stream).bearer_auth(&rig.token).send().await.unwrap();
    let mut text = String::new();
    let frames = loop {
        let frames: Vec<(String, u64)> = text
            .split("\n\n")
            .filter_map(|frame| {
                let line =
                    |prefix: &str| frame.lines().find_map(|line| line.strip_prefix(prefix)).map(str::trim);
                let data: Value = serde_json::from_str(line("data:")?).ok()?;
                Some((line("id:")?.to_string(), data["data"]["n"].as_u64()?))
            })
            .collect();
        if frames.len() >= 3 {
            break frames;
        }
        let chunk = in_time("more of the stream", response.chunk()).await.unwrap().expect("the stream ended");
        text.push_str(&String::from_utf8_lossy(&chunk));
    };
    let heard: Vec<u64> = frames.iter().map(|(_, n)| *n).collect();
    let of_shop: Vec<u64> = heard.iter().copied().filter(|n| n % 2 == 1).collect();
    assert_eq!((of_shop, heard.len()), (vec![3, 5], 3), "{heard:?}");
    let mut ids: Vec<&String> = frames.iter().map(|(id, _)| id).collect();
    ids.dedup();
    assert_eq!(ids.len(), 3, "{frames:?}");
    // And the test's own way of waiting for an event hears a project's.
    let heard = rig.event("kept.here", |event| event.data["n"] == 5).await.seq;
    assert_eq!(rig.number(heard), later[2]);
}

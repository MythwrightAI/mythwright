//! Following more than a page (`talk::following`): a wiki with every page
//! under it, one added later included; a table and a channel, each one
//! line saying what changed; hourly, one line once the hour is over, and
//! not at all; nothing one may not read; and a digest told once whatever
//! stops.

use std::sync::Arc;
use std::time::Duration;

use kernel::ManualClock;
use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;
use crate::kept::{Person, person};

const TABLES: &str = "blueprint: tables
permissions: [projects, board]
steps:
  stock:
    do: collection.ensure
    with:
      name: stock
      fields: { title: { type: text, required: true } }
      views: { rows: { kind: table } }
  moods:
    do: collection.ensure
    with: { name: moods, reads: private:journal, fields: { title: { type: text, required: true } } }
";

/// A shop with a table anybody reads and one kept private.
async fn shop(rig: &Rig) {
    rig.send("/api/tier", json!({ "tier": "premium" })).await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    let applied = rig.send("/api/apply", json!({ "source": TABLES, "project": "shop", "wait": true })).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
}

/// A node on a clock the test moves, with its owner.
async fn ticking() -> (Rig, ManualClock) {
    let clock = ManualClock::new();
    let moved = clock.clone();
    let rig = Rig::with("api-following", move |config, _| config.clock = Some(Arc::new(moved))).await;
    (Rig::owned(rig).await, clock)
}

/// `who` follows `kind` `thing` as `more` says: the follow, which is to go well.
async fn follows(rig: &Rig, who: &Person, kind: &str, thing: &str, more: Value) -> Value {
    let mut body = json!({ "kind": kind, "thing": thing });
    for (key, value) in more.as_object().cloned().unwrap_or_default() {
        body[key] = value;
    }
    let (status, follow) = rig.call(Method::POST, "/api/following", Some(&who.token), Some(body)).await;
    assert_eq!(status, StatusCode::OK, "{follow}");
    follow
}

/// The lines of `token`'s box, every one, of `kind`.
async fn lines(rig: &Rig, token: &str, kind: &str) -> Vec<Value> {
    let (status, found) = rig.call(Method::GET, "/api/inbox?all=true", Some(token), None).await;
    assert_eq!(status, StatusCode::OK, "{found}");
    found["lines"].as_array().unwrap().iter().filter(|line| line["kind"] == kind).cloned().collect()
}

/// Waits until `owner` is told a line of `kind` about what begins with
/// `about`, raised to `count`, after `mark`.
async fn told(rig: &Rig, mark: u64, owner: &str, kind: &str, about: &str, count: u64) {
    let (owner, kind, about) = (owner.to_string(), kind.to_string(), about.to_string());
    rig.event("inbox.told", move |event| {
        event.seq > mark
            && event.data["owner"] == owner.as_str()
            && event.data["kind"] == kind.as_str()
            && event.data["about"].as_str().is_some_and(|said| said.starts_with(&about))
            && event.data["count"] == count
    })
    .await;
}

fn id(page: &Value) -> String {
    page["id"].as_str().unwrap().to_string()
}

/// Adds a row called `title` to the shop's `table` as `token`.
async fn row(rig: &Rig, table: &str, title: &str) -> String {
    let path = format!("/api/collections/shop/{table}/records");
    id(&rig.send(&path, json!({ "fields": { "title": title } })).await)
}

/// The changes a line told of, oldest first: each what it was, and what it changed.
async fn changes(rig: &Rig, token: &str, line: &Value) -> Vec<(Value, Value, Value)> {
    let path = format!("/api/inbox/{}/changes", line["id"].as_str().unwrap());
    let (status, found) = rig.call(Method::GET, &path, Some(token), None).await;
    assert_eq!(status, StatusCode::OK, "{found}");
    found["changes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|one| (one["noun"].clone(), one["verb"].clone(), one["label"].clone()))
        .collect()
}

#[tokio::test]
async fn following_a_wiki_with_its_pages_tells_of_a_page_added_under_it_later_by_the_wikis_one_line() {
    let rig = Rig::new().await;
    rig.send("/api/tier", json!({ "tier": "premium" })).await;
    let nell = person(&rig, "Nell").await;
    let wiki = id(&rig.send("/api/topics", json!({ "title": "Frogs: a field guide" })).await);
    let path = format!("/api/topics/{wiki}/follow");
    let (status, page) = rig
        .call(Method::POST, &path, Some(&nell.token), Some(json!({ "follows": true, "pages": true })))
        .await;
    assert_eq!(status, StatusCode::OK, "{page}");
    assert_eq!(page["data"]["followers"], json!([nell.actor]), "the page says who follows it, as it did");

    // A page made under it after she followed: her one line of the wiki, opening the new page.
    let tree = id(&rig.send("/api/topics", json!({ "title": "Tree frogs", "under": wiki })).await);
    told(&rig, 0, &nell.actor, "followed", &wiki, 1).await;
    let line = lines(&rig, &nell.token, "followed").await;
    assert_eq!(line.len(), 1, "{line:?}");
    assert_eq!((&line[0]["about"], &line[0]["place"]), (&json!(wiki), &json!(tree)));
    // A page under that one, changed: the same line, raised.
    let glass = id(&rig.send("/api/topics", json!({ "title": "Glass frogs", "under": tree })).await);
    told(&rig, 0, &nell.actor, "followed", &wiki, 2).await;
    let (status, _) = rig
        .call(
            Method::PATCH,
            &format!("/api/records/{glass}"),
            Some(&rig.token),
            Some(json!({ "fields": { "body": "See-through." } })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    told(&rig, 0, &nell.actor, "followed", &wiki, 3).await;
    let line = lines(&rig, &nell.token, "followed").await;
    assert_eq!((line.len(), &line[0]["place"]), (1, &json!(glass)), "{line:?}");
    assert_eq!(line[0]["on_record"]["did"], json!("Revised the page"));

    // Listed as she follows it, and followed alone again by the page's own door as before.
    let (_, listed) = rig.call(Method::GET, "/api/following", Some(&nell.token), None).await;
    let listed = listed["following"].as_array().unwrap().clone();
    assert_eq!(listed.len(), 1, "{listed:?}");
    assert_eq!(
        (
            &listed[0]["kind"],
            &listed[0]["thing"],
            &listed[0]["pages"],
            &listed[0]["often"],
            &listed[0]["title"]
        ),
        (&json!("page"), &json!(wiki), &json!(true), &json!("once"), &json!("Frogs: a field guide"))
    );
    rig.call(Method::POST, &path, Some(&nell.token), Some(json!({ "follows": true, "pages": false }))).await;
    rig.send("/api/topics", json!({ "title": "Toads", "under": wiki })).await;
    rig.send(&format!("/api/topics/{wiki}/replies"), json!({ "body": "Which is the loudest?" })).await;
    told(&rig, 0, &nell.actor, "followed", &wiki, 4).await;
    let line = lines(&rig, &nell.token, "followed").await;
    assert_eq!(
        (&line[0]["place"], &line[0]["count"]),
        (&json!(wiki), &json!(4)),
        "the page added told her nothing"
    );
}

#[tokio::test]
async fn following_a_table_and_a_channel_is_one_line_each_saying_what_changed_since_it_was_read() {
    let rig = Rig::new().await;
    shop(&rig).await;
    rig.send("/api/channels", json!({ "name": "general" })).await;
    let nell = person(&rig, "Nell").await;
    let stock = follows(&rig, &nell, "table", "shop/stock", json!({})).await;
    assert_eq!((&stock["title"], &stock["often"]), (&json!("stock of Shop"), &json!("once")));
    follows(&rig, &nell, "channel", "general", json!({})).await;

    let basket = row(&rig, "stock", "Basket").await;
    row(&rig, "stock", "Crate").await;
    let (status, _) = rig
        .call(
            Method::PATCH,
            &format!("/api/records/{basket}"),
            Some(&rig.token),
            Some(json!({ "fields": { "title": "Big basket" } })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    told(&rig, 0, &nell.actor, "following", "table:shop/stock", 3).await;
    rig.send("/api/channels/general/messages", json!({ "body": "Stock is in." })).await;
    told(&rig, 0, &nell.actor, "following", "channel:", 1).await;
    let told_her = lines(&rig, &nell.token, "following").await;
    assert_eq!(told_her.len(), 2, "{told_her:?}");
    let (channel, table) = (&told_her[0], &told_her[1]);
    assert_eq!(table["says"], json!("stock of Shop: 1 row changed, 2 rows added"), "{table}");
    assert_eq!(channel["says"], json!("#general: 1 new message"), "{channel}");
    assert!(!channel.to_string().contains("Stock is in"), "a line keeps no words of a message: {channel}");
    assert_eq!(
        changes(&rig, &nell.token, table).await,
        [
            (json!("row"), json!("added"), json!("Big basket")),
            (json!("row"), json!("added"), json!("Crate")),
            (json!("row"), json!("changed"), json!("Big basket")),
        ]
    );

    // Read, it says what came after.
    let read = format!("/api/inbox/{}/read", table["id"].as_str().unwrap());
    rig.call(Method::POST, &read, Some(&nell.token), None).await;
    row(&rig, "stock", "Sack").await;
    told(&rig, 0, &nell.actor, "following", "table:shop/stock", 4).await;
    let table = lines(&rig, &nell.token, "following")
        .await
        .into_iter()
        .find(|line| line["about"] == "table:shop/stock");
    assert_eq!(table.unwrap()["says"], json!("stock of Shop: 1 row added"));

    // Her own change tells her nothing; unfollowed, nothing more.
    row(&rig, "stock", "Box").await;
    let (status, _) = rig
        .call(
            Method::POST,
            "/api/collections/shop/stock/records",
            Some(&nell.token),
            Some(json!({ "fields": { "title": "Mine" } })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    told(&rig, 0, &nell.actor, "following", "table:shop/stock", 5).await;
    follows(&rig, &nell, "table", "shop/stock", json!({ "follows": false })).await;
    row(&rig, "stock", "Bag").await;
    rig.send("/api/channels/general/messages", json!({ "body": "One more." })).await;
    told(&rig, 0, &nell.actor, "following", "channel:", 2).await;
    let table = lines(&rig, &nell.token, "following")
        .await
        .into_iter()
        .find(|line| line["about"] == "table:shop/stock");
    assert_eq!(table.unwrap()["count"], json!(5), "neither hers nor what came after she stopped");
    let (_, listed) = rig.call(Method::GET, "/api/following", Some(&nell.token), None).await;
    assert_eq!(listed["following"].as_array().unwrap().len(), 1, "{listed}");
}

#[tokio::test]
async fn hourly_is_one_line_once_the_hour_is_over_and_off_is_followed_and_told_nothing() {
    let (rig, clock) = ticking().await;
    shop(&rig).await;
    let (nell, mira, rose) =
        (person(&rig, "Nell").await, person(&rig, "Mira").await, person(&rig, "Rose").await);
    let hourly = follows(&rig, &nell, "table", "shop/stock", json!({ "often": "hourly" })).await;
    assert_eq!(hourly["often"], json!("hourly"));
    follows(&rig, &mira, "table", "shop/stock", json!({})).await;
    follows(&rig, &rose, "table", "shop/stock", json!({ "often": "off" })).await;

    for title in ["Basket", "Crate"] {
        row(&rig, "stock", title).await;
    }
    // Told at once, Mira says when the changes were heard: in the same write as what waits for Nell's hour.
    told(&rig, 0, &mira.actor, "following", "table:shop/stock", 2).await;
    assert!(lines(&rig, &nell.token, "following").await.is_empty(), "nothing until the hour is over");

    clock.advance(Duration::from_secs(3600));
    told(&rig, 0, &nell.actor, "following", "table:shop/stock#", 1).await;
    let digest = lines(&rig, &nell.token, "following").await;
    assert_eq!(digest.len(), 1, "{digest:?}");
    assert_eq!(
        (&digest[0]["says"], &digest[0]["why"]),
        (&json!("stock of Shop: 2 rows added"), &json!("digest"))
    );
    assert_eq!(changes(&rig, &nell.token, &digest[0]).await.len(), 2);
    assert!(lines(&rig, &rose.token, "following").await.is_empty(), "off tells nothing");

    // An hour with nothing in it is no line.
    clock.advance(Duration::from_secs(3600));
    row(&rig, "stock", "Sack").await;
    told(&rig, 0, &mira.actor, "following", "table:shop/stock", 3).await;
    assert_eq!(lines(&rig, &nell.token, "following").await.len(), 1);
    let mark = rig.mark();
    clock.advance(Duration::from_secs(3600));
    told(&rig, mark, &nell.actor, "following", "table:shop/stock#", 1).await;
    let digests = lines(&rig, &nell.token, "following").await;
    assert_eq!(digests.len(), 2, "one line an hour: {digests:?}");
    assert_eq!(digests[0]["says"], json!("stock of Shop: 1 row added"));
}

#[tokio::test]
async fn a_digest_that_waits_as_the_daemon_stops_is_told_once_after_it_starts_again() {
    let (mut rig, clock) = ticking().await;
    shop(&rig).await;
    let (nell, mira) = (person(&rig, "Nell").await, person(&rig, "Mira").await);
    follows(&rig, &nell, "table", "shop/stock", json!({ "often": "hourly" })).await;
    follows(&rig, &mira, "table", "shop/stock", json!({})).await;
    row(&rig, "stock", "Basket").await;
    told(&rig, 0, &mira.actor, "following", "table:shop/stock", 1).await;

    // Stopped while it waits, and started again after its hour.
    rig.restart().await;
    clock.advance(Duration::from_secs(3600));
    told(&rig, 0, &nell.actor, "following", "table:shop/stock#", 1).await;
    // Stopped again after it was told, and a while passes: nothing is told twice.
    rig.restart().await;
    clock.advance(Duration::from_secs(2 * 3600));
    row(&rig, "stock", "Crate").await;
    told(&rig, 0, &mira.actor, "following", "table:shop/stock", 2).await;
    let mark = rig.mark();
    clock.advance(Duration::from_secs(3600));
    told(&rig, mark, &nell.actor, "following", "table:shop/stock#", 1).await;
    let digests = lines(&rig, &nell.token, "following").await;
    let said: Vec<&Value> = digests.iter().map(|line| &line["says"]).collect();
    assert_eq!(said, [&json!("stock of Shop: 1 row added"), &json!("stock of Shop: 1 row added")]);
    let first = changes(&rig, &nell.token, &digests[1]).await;
    assert_eq!(first, [(json!("row"), json!("added"), json!("Basket"))], "the first told Basket alone, once");
}

#[tokio::test]
async fn nobody_follows_what_they_may_not_read_or_is_told_of_it_once_they_may_not() {
    let rig = Rig::new().await;
    shop(&rig).await;
    let nell = person(&rig, "Nell").await;
    rig.send("/api/channels", json!({ "name": "Pay review", "kept": true })).await;
    let general = id(&rig.send("/api/channels", json!({ "name": "general" })).await);
    let refuse = |kind: &'static str, thing: &'static str| {
        let token = nell.token.clone();
        let rig = &rig;
        async move {
            let body = json!({ "kind": kind, "thing": thing });
            let (status, refused) = rig.call(Method::POST, "/api/following", Some(&token), Some(body)).await;
            assert!(
                status == StatusCode::FORBIDDEN || status == StatusCode::NOT_FOUND,
                "{kind} {thing}: {status} {refused}"
            );
        }
    };
    refuse("table", "shop/moods").await;
    refuse("channel", "pay-review").await;
    let (status, refused) = rig
        .call(
            Method::POST,
            "/api/following",
            Some(&nell.token),
            Some(json!({ "kind": "folder", "thing": "x" })),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");
    let (_, listed) = rig.call(Method::GET, "/api/following", Some(&nell.token), None).await;
    assert_eq!(listed["following"], json!([]));

    // A key of hers that reads no projects follows none.
    let narrow = crate::kept::key(&rig, &nell.token, &["inbox"]).await;
    let body = json!({ "kind": "project", "thing": "shop" });
    let (status, refused) = rig.call(Method::POST, "/api/following", Some(&narrow), Some(body)).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");

    // A channel she followed, kept to its members without her: what is said there tells her nothing.
    follows(&rig, &nell, "channel", "general", json!({})).await;
    rig.send("/api/channels/general/messages", json!({ "body": "Before." })).await;
    told(&rig, 0, &nell.actor, "following", "channel:", 1).await;
    let (status, kept) =
        rig.call(Method::POST, &format!("/api/channels/{general}/keep"), Some(&rig.token), None).await;
    assert_eq!(status, StatusCode::OK, "{kept}");
    rig.send(&format!("/api/channels/{general}/messages"), json!({ "body": "After." })).await;
    let elsewhere = id(&rig.send("/api/channels", json!({ "name": "elsewhere" })).await);
    follows(&rig, &nell, "channel", "elsewhere", json!({})).await;
    rig.send("/api/channels/elsewhere/messages", json!({ "body": "Here." })).await;
    told(&rig, 0, &nell.actor, "following", &format!("channel:{elsewhere}"), 1).await;
    let lines = lines(&rig, &nell.token, "following").await;
    assert_eq!(lines.len(), 1, "the line of a channel she may not read is passed over: {lines:?}");
    assert_eq!(lines[0]["says"], json!("#elsewhere: 1 new message"));
}

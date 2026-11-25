//! A watch (`std/watch`): what is written into the history of something
//! is said in a channel as it is written, whatever the something is.

use reqwest::Method;
use serde_json::{Value, json};

use crate::Rig;

/// What the watch called `name` has said in `channel`, oldest first.
async fn said_by(rig: &Rig, channel: &str, name: &str) -> Vec<String> {
    let messages = rig.read(&format!("/api/channels/{channel}/messages")).await;
    messages["messages"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|message| message["author"] == format!("bot:{name}").as_str())
        .map(|message| message["body"].as_str().unwrap().to_string())
        .collect()
}

async fn card(rig: &Rig, project: &str, title: &str) -> String {
    let added = rig.send(&format!("/api/board/{project}/items"), json!({ "title": title })).await;
    added["id"].as_str().unwrap().to_string()
}

async fn move_to(rig: &Rig, id: &str, column: &str) {
    let change = json!({ "status": column });
    rig.call(Method::PATCH, &format!("/api/board/items/{id}"), Some(&rig.token), Some(change)).await;
}

async fn watch(rig: &Rig, project: Option<&str>, key: &str, params: Value) {
    let mut asked = json!({ "blueprint": "std/watch", "key": key, "params": params, "wait": true });
    if let Some(project) = project {
        asked["project"] = json!(project);
    }
    let applied = rig.send("/api/apply", asked).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
}

/// Waits until the watch has said `lines` things in the channel.
async fn told(rig: &Rig, channel: &str, name: &'static str, lines: usize) -> Vec<String> {
    let mut posted = rig.follow("channel.message");
    crate::soon(&format!("{lines} lines from {name} in {channel}"), async {
        loop {
            let said = said_by(rig, channel, name).await;
            if said.len() >= lines {
                return said;
            }
            posted.next().await.unwrap();
        }
    })
    .await
}

#[tokio::test]
async fn a_watch_says_in_a_channel_what_happens_to_the_one_thing_it_watches_and_nothing_of_any_other() {
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    rig.board("shop").await;
    let (cart, other) =
        (card(&rig, "shop", "Add a cart").await, card(&rig, "shop", "Write the policy").await);

    watch(&rig, None, "watch-cart", json!({ "record": cart, "channel": "general" })).await;
    move_to(&rig, &other, "doing").await;
    move_to(&rig, &cart, "doing").await;
    rig.send(&format!("/api/records/{cart}/comments"), json!({ "body": "With tax, please." })).await;

    let said = told(&rig, "general", "watch", 2).await;
    assert_eq!(
        said,
        [format!("{cart}: Ada moved it from draft to doing"), format!("{cart}: Ada said: With tax, please.")]
    );
}

#[tokio::test]
async fn a_watch_of_a_kind_says_what_happens_to_every_one_of_them_in_its_project_and_stops_when_it_is_stopped()
 {
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    rig.send("/api/projects", json!({ "name": "Depot" })).await;
    rig.board("shop").await;
    rig.board("depot").await;

    watch(
        &rig,
        Some("shop"),
        "watch-board",
        json!({ "collection": "board", "channel": "general", "name": "board news" }),
    )
    .await;
    let elsewhere = card(&rig, "depot", "Count the shelves").await;
    let first = card(&rig, "shop", "Add a cart").await;
    let second = card(&rig, "shop", "Write the policy").await;
    move_to(&rig, &elsewhere, "doing").await;

    let said = told(&rig, "general", "board-news", 2).await;
    assert_eq!(
        said,
        [format!("{first}: Ada added it to draft"), format!("{second}: Ada added it to draft")],
        "and nothing of another project's"
    );

    // Stopped, it says no more: the next thing said of a card is written
    // into its history and nobody is told of it in the channel.
    let live = rig.read("/api/instances?live=true&project=shop").await;
    let instance = live["instances"]
        .as_array()
        .unwrap()
        .iter()
        .find(|instance| instance["blueprint"] == "std/watch")
        .unwrap_or_else(|| panic!("no watch among {live}"))["id"]
        .as_str()
        .unwrap()
        .to_string();
    let (status, answer) =
        rig.call(Method::POST, &format!("/api/instances/{instance}/stop"), Some(&rig.token), None).await;
    assert_eq!(status, reqwest::StatusCode::OK, "{answer}");
    move_to(&rig, &first, "doing").await;
    crate::lanes::history(&rig, &first, "Ada moved it from draft to doing").await;
    let after = said_by(&rig, "general", "board-news").await;
    assert_eq!(after.len(), 2, "{after:?}");
}

/// How many events each of the watches' handlers was handed, by the
/// handler and the project its watch is of: what was delivered to them,
/// whatever they then said of it.
async fn handed(rig: &Rig) -> Vec<(String, String, u64)> {
    let query = journal::MetricQuery {
        name: "handler.event".into(),
        group_by: vec!["handler".into(), "project".into()],
        labels: vec![("blueprint".into(), "std/watch".into())],
        ..Default::default()
    };
    let mut handed: Vec<(String, String, u64)> = rig
        .node()
        .app
        .journal
        .query_metrics(query)
        .await
        .unwrap()
        .into_iter()
        .map(|series| {
            let label = |name: &str| series.labels.get(name).cloned().unwrap_or_default();
            (label("handler"), label("project"), series.total.count)
        })
        .collect();
    handed.sort();
    handed
}

#[tokio::test]
async fn a_watch_is_woken_for_what_it_watches_and_for_nothing_else() {
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    rig.send("/api/projects", json!({ "name": "Depot" })).await;
    rig.board("shop").await;
    rig.board("depot").await;
    let (cart, other) =
        (card(&rig, "shop", "Add a cart").await, card(&rig, "shop", "Write the policy").await);
    let elsewhere = card(&rig, "depot", "Count the shelves").await;
    // Everything said of the three as they were added has been written
    // down before anything is watching.
    for id in [&cart, &other, &elsewhere] {
        crate::lanes::history(&rig, id, "Ada added it to draft").await;
    }

    // One watch of one card, and one of everything on the shop's board.
    watch(&rig, None, "watch-cart", json!({ "record": cart, "channel": "general", "name": "cart" })).await;
    let board = json!({ "collection": "board", "channel": "general", "name": "shop" });
    watch(&rig, Some("shop"), "watch-board", board).await;
    move_to(&rig, &elsewhere, "doing").await;
    move_to(&rig, &other, "doing").await;
    move_to(&rig, &cart, "doing").await;
    rig.send(&format!("/api/records/{cart}/comments"), json!({ "body": "With tax, please." })).await;

    // Each has said all that was its to say, in the order it happened: a
    // handler takes one event at a time, so by its last it has had them all.
    assert_eq!(told(&rig, "general", "cart", 2).await.len(), 2);
    assert_eq!(told(&rig, "general", "shop", 3).await.len(), 3);

    // Four lines were written in that time, one of them another project's.
    // The watch of one card was handed the two that were that card's, by
    // the handler that is addressed by a record; the watch of the board
    // the three that were the shop's, by the one addressed by a project;
    // and the other handler of each, which waits for nothing, was handed
    // nothing. Nobody was woken for the card in the depot.
    assert_eq!(
        handed(&rig).await,
        [("kind".to_string(), "shop".to_string(), 3), ("one".to_string(), String::new(), 2)]
    );
}

#[tokio::test]
async fn a_watch_of_one_thing_applied_in_a_project_passes_on_that_things_news_and_none_of_the_projects() {
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    rig.board("shop").await;
    let (cart, other) =
        (card(&rig, "shop", "Add a cart").await, card(&rig, "shop", "Write the policy").await);
    for id in [&cart, &other] {
        crate::lanes::history(&rig, id, "Ada added it to draft").await;
    }

    // In a project its other handler is addressed by the project, and is
    // woken for what happens there; it has nothing to say of any of it.
    let one = json!({ "record": cart, "collection": "board", "channel": "general", "name": "cart" });
    watch(&rig, Some("shop"), "watch-cart", one).await;
    move_to(&rig, &other, "doing").await;
    move_to(&rig, &cart, "doing").await;
    crate::lanes::history(&rig, &other, "Ada moved it from draft to doing").await;

    let said = told(&rig, "general", "cart", 1).await;
    assert_eq!(said, [format!("{cart}: Ada moved it from draft to doing")]);
    // A second thing said of it is the second line: nothing came between.
    rig.send(&format!("/api/records/{cart}/comments"), json!({ "body": "With tax, please." })).await;
    let said = told(&rig, "general", "cart", 2).await;
    assert_eq!(said.len(), 2, "{said:?}");
    assert!(said[1].starts_with(&format!("{cart}: ")), "{said:?}");
}

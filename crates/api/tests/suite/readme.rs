//! What the README writes out of blueprints is what the program takes, and
//! does what the README says it does.
//!
//! The README is what most people read first, and its blueprints are what
//! they copy. So every block of it that is a blueprint (written as a
//! block of YAML that begins `blueprint:`) is read out of it as it stands
//! and checked as a project's own would be, with nothing said of it, and
//! held to being laid out as `mw fmt` lays it out; and each of "Blueprints,
//! by example" is applied and played as the README says it is, so that one
//! that comes to do something else fails here, saying which.

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::{Rig, soon};

/// The README as it stands.
fn readme() -> String {
    std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../README.md")).unwrap()
}

/// Every blueprint the README writes out, by the name it says it is, as written.
fn written() -> Vec<(String, String)> {
    let text = readme();
    let mut found = Vec::new();
    for block in text.split("```yaml\n").skip(1) {
        let block = block.split("```").next().unwrap();
        if let Some(name) = block.strip_prefix("blueprint: ") {
            found.push((name.lines().next().unwrap().trim().to_string(), block.to_string()));
        }
    }
    found
}

/// The one the README writes out under `name`.
fn text_of(name: &str) -> String {
    let found = written().into_iter().find(|(called, _)| called == name);
    found.unwrap_or_else(|| panic!("the README no longer writes out `{name}`")).1
}

/// Applies `source` for `project`, waiting for its steps.
async fn applied(rig: &Rig, source: &str, project: &str) {
    let body = json!({ "source": source, "project": project, "wait": true });
    let applied = rig.send("/api/apply", body).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
}

/// The channel called `name`: its id.
async fn channel(rig: &Rig, name: &str) -> String {
    let made = rig.send("/api/channels", json!({ "name": name, "project": name })).await;
    made["id"].as_str().unwrap().to_string()
}

/// What was said in a channel, the words alone.
async fn said_in(rig: &Rig, id: &str) -> Vec<String> {
    let said = rig.read(&format!("/api/channels/{id}/messages")).await;
    let messages = said["messages"].as_array().cloned().unwrap_or_default();
    messages.iter().map(|message| message["body"].as_str().unwrap_or_default().to_string()).collect()
}

#[tokio::test]
async fn every_blueprint_the_readme_writes_out_checks_with_nothing_said_and_is_laid_out_the_one_way() {
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "shop" })).await;
    let all = written();
    let names: Vec<&str> = all.iter().map(|(name, _)| name.as_str()).collect();
    for wanted in ["greeter", "writing", "orders", "nudge"] {
        assert!(names.contains(&wanted), "the README no longer writes out `{wanted}`: it writes {names:?}");
    }
    for (name, source) in &all {
        let checked = rig.send("/api/blueprints/check", json!({ "source": source, "project": "shop" })).await;
        assert_eq!(checked["ok"], true, "the README's `{name}` does not check: {}", checked["diagnostics"]);
        assert_eq!(
            checked["diagnostics"],
            json!([]),
            "what is said of the README's `{name}`, which is to say nothing"
        );
        let laid =
            blueprint::format(source, &Default::default()).unwrap_or_else(|_| panic!("`{name}` is laid out"));
        assert_eq!(&laid, source, "the README's `{name}` is not laid out as `mw fmt` lays it out");
    }
}

#[tokio::test]
async fn a_board_of_ones_own_has_the_columns_its_blueprint_gives_it_and_a_card_moves_through_them() {
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "blog" })).await;

    applied(&rig, &text_of("writing"), "blog").await;
    let card = rig.send("/api/board/blog/items", json!({ "title": "A post about the launch" })).await;
    let id = card["id"].as_str().unwrap();
    let (status, moved) = rig
        .call(
            Method::PATCH,
            &format!("/api/board/items/{id}"),
            Some(&rig.token),
            Some(json!({ "status": "drafting" })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{moved}");

    let board = rig.read("/api/board/blog").await;
    assert_eq!(board["columns"], json!(["ideas", "drafting", "editing", "published"]), "{board}");
    assert_eq!(board["items"][0]["status"], "drafting", "{board}");
    assert_eq!(card["status"], "ideas", "a card begins in the first column: {card}");
}

#[tokio::test]
async fn an_event_of_ones_own_said_with_a_key_for_that_topic_puts_a_card_on_the_board_and_says_so() {
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "shop" })).await;
    rig.board("shop").await;
    // The fleet makes the project's channel; here it is made by hand.
    let floor = channel(&rig, "shop").await;
    applied(&rig, &text_of("orders"), "shop").await;

    // A key that may say that one topic, and nothing else.
    let made =
        rig.send("/api/credentials", json!({ "name": "payments", "scopes": ["emit:shop.paid"] })).await;
    let key = made["token"].as_str().unwrap();
    let paid = json!({ "order": "o-17" });
    let (status, said) = rig.call(Method::POST, "/api/emit/shop.paid", Some(key), Some(paid)).await;
    assert_eq!(status, StatusCode::OK, "{said}");
    let (status, refused) =
        rig.call(Method::POST, "/api/emit/shop.refunded", Some(key), Some(json!({ "order": "o-17" }))).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");

    let card = soon("a card for the order paid", async {
        loop {
            let board = rig.read("/api/board/shop").await;
            if let Some(card) = board["items"].as_array().and_then(|items| items.first()).cloned() {
                return card;
            }
            tokio::task::yield_now().await;
        }
    })
    .await;
    assert_eq!(card["title"], "Pack order o-17", "{card}");
    let id = card["id"].as_str().unwrap().to_string();
    let told = soon("the order said in the channel", async {
        loop {
            let said = said_in(&rig, &floor).await;
            if let Some(told) = said.into_iter().find(|body| body.starts_with("Order")) {
                return told;
            }
            tokio::task::yield_now().await;
        }
    })
    .await;
    assert_eq!(told, format!("Order o-17 is paid: {id} is to pack it."));
}

/// What a sandbox of the README's `nudge` came to, with `acts` done to it in order.
async fn nudged(rig: &Rig, acts: &[Value]) -> Value {
    let source = text_of("nudge");
    let made = rig.send("/api/sandboxes", json!({ "source": source, "first": ["std/board"] })).await;
    let id = made["sandbox"].as_str().unwrap().to_string();
    let path = format!("/api/sandboxes/{id}/acts");
    for act in acts {
        rig.send(&path, act.clone()).await;
    }
    rig.read(&format!("/api/sandboxes/{id}")).await
}

/// What was said in a sandbox's channel called `name`.
fn told(sandbox: &Value, name: &str) -> Vec<String> {
    let channels = sandbox["channels"].as_array().cloned().unwrap_or_default();
    let messages = channels
        .iter()
        .filter(|channel| channel["name"] == name)
        .flat_map(|channel| channel["messages"].as_array().cloned().unwrap_or_default());
    messages.map(|message| message["body"].as_str().unwrap_or_default().to_string()).collect()
}

#[tokio::test]
async fn a_card_that_waits_a_day_in_review_is_said_and_one_that_moves_on_within_the_day_is_not() {
    let rig = Rig::new().await;
    // What the README's sandbox is told, by the routes `mw sandbox` asks.
    let does = |action: &str, with: Value| json!({ "do": { "action": action, "with": with } });
    let by = |by: &str| json!({ "advance": { "by": by } });
    let into_review = [
        does("channel.ensure", json!({ "name": "sandbox" })),
        does("board.add", json!({ "title": "A post about the launch", "status": "doing" })),
        does("board.move", json!({ "id": "sandbox-1", "to": "review" })),
    ];

    let waited = nudged(&rig, &[into_review.to_vec(), vec![by("25h")]].concat()).await;
    assert_eq!(told(&waited, "sandbox"), ["A post about the launch has waited a day in review."], "{waited}");

    let moved_on = [
        into_review.to_vec(),
        vec![by("2h"), does("board.move", json!({ "id": "sandbox-1", "to": "done" })), by("25h")],
    ];
    let went = nudged(&rig, &moved_on.concat()).await;
    assert_eq!(told(&went, "sandbox"), Vec::<String>::new(), "{went}");
}

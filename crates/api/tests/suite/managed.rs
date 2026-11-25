//! A channel's details through the daemon's routes (`talk::managed`,
//! `routes/renamed.rs`): what it is called and what it says it is for,
//! changed by its managers (whoever made it, whoever a manager names one)
//! and whoever manages the factory, and by nobody else in it, whichever
//! door they come by (a route, a key that holds less, the bot); who
//! manages it, the last manager staying.

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;
use crate::kept::{key, person, say};

/// The words the daemon refused with.
fn why(answer: &Value) -> &str {
    answer["error"]["message"].as_str().unwrap_or_default()
}

async fn change(rig: &Rig, token: &str, channel: &str, body: Value) -> (StatusCode, Value) {
    rig.call(Method::PATCH, &format!("/api/channels/{channel}"), Some(token), Some(body)).await
}

async fn manage(rig: &Rig, token: &str, channel: &str, member: &str, manages: bool) -> (StatusCode, Value) {
    let body = json!({ "member": member, "manages": manages });
    rig.call(Method::POST, &format!("/api/channels/{channel}/managers"), Some(token), Some(body)).await
}

/// Who manages the channel as it is listed, by actor.
async fn managing(rig: &Rig, channel: &str) -> Vec<String> {
    let read = rig.read(&format!("/api/channels/{channel}")).await;
    let members = read["members"].as_array().cloned().unwrap_or_default();
    members
        .iter()
        .filter(|one| one["manages"] == true)
        .map(|one| one["actor"].as_str().unwrap().into())
        .collect()
}

/// What the factory's bot answered to what `token` said in `channel`.
async fn asked_the_bot(rig: &Rig, token: &str, channel: &str, words: &str) -> String {
    let said = say(rig, token, channel, json!({ "body": words })).await;
    let answer = rig
        .event("channel.message", move |event| {
            event.data["reply_to"] == said.as_str() && event.data["author_kind"] == "bot"
        })
        .await;
    answer.data["body"].as_str().unwrap().to_string()
}

#[tokio::test]
async fn a_channels_details_are_changed_by_its_managers_and_whoever_manages_the_factory_and_by_nobody_else() {
    let rig = Rig::new().await;
    rig.send("/api/tier", json!({ "tier": "premium" })).await;
    let (mira, nell) = (person(&rig, "Mira").await, person(&rig, "Nell").await);
    // Mira makes it, and manages it; Nell comes in by saying something.
    let asked = json!({ "name": "orchard", "topic": "everything" });
    let (status, made) = rig.call(Method::POST, "/api/channels", Some(&mira.token), Some(asked)).await;
    assert_eq!(status, StatusCode::OK, "{made}");
    let id = made["id"].as_str().unwrap().to_string();
    assert_eq!(managing(&rig, &id).await, std::slice::from_ref(&mira.actor));
    say(&rig, &nell.token, "orchard", json!({ "body": "hello" })).await;

    // Nell writes there and manages nothing: refused every change, in a
    // sentence that says who does, with nothing changed and nothing said.
    let mark = rig.mark();
    for body in
        [json!({ "topic": "pears" }), json!({ "name": "pears" }), json!({ "name": "pears", "topic": "x" })]
    {
        let (status, refused) = change(&rig, &nell.token, "orchard", body).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
        assert_eq!(
            why(&refused),
            "Nell is not one of its managers: #orchard is managed by Mira, who change what it is called, what it \
             says it is for and who else manages it, as whoever manages the factory does (needs `admin`)"
        );
    }
    let (status, refused) = manage(&rig, &nell.token, "orchard", &nell.actor, true).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    // The bot does it as whoever asks: refused the same.
    let answered =
        asked_the_bot(&rig, &nell.token, "orchard", "@conductor channel rename orchard pears").await;
    assert!(answered.contains("Nell is not one of its managers"), "{answered}");
    let said = journal::EventQuery {
        after: mark,
        topics: "channel.renamed,channel.described,channel.managers".into(),
        ..Default::default()
    };
    assert!(rig.said(said).await.is_empty());

    // Mira says what it is for, with a key of hers that holds what writing
    // in channels takes; one that holds less is refused that.
    let reads = key(&rig, &mira.token, &["channels:read"]).await;
    assert_eq!(change(&rig, &reads, "orchard", json!({ "topic": "fruit" })).await.0, StatusCode::FORBIDDEN);
    let writes = key(&rig, &mira.token, &["channels"]).await;
    let mark = rig.mark();
    let (status, now) = change(&rig, &writes, "orchard", json!({ "topic": "  the quince trees  " })).await;
    assert_eq!((status, now["topic"].as_str()), (StatusCode::OK, Some("the quince trees")), "{now}");
    let told = rig.event("channel.described", move |event| event.seq > mark).await;
    assert_eq!(
        (told.data["was"].as_str(), told.data["by"].as_str()),
        (Some("everything"), Some(mira.actor.as_str()))
    );
    let (status, refused) = change(&rig, &writes, "orchard", json!({ "topic": "one\ntwo" })).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");
    let (status, refused) = change(&rig, &writes, "orchard", json!({})).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");

    // She makes Nell one, who renames it and says what it is for at once.
    let (status, named) = manage(&rig, &mira.token, "orchard", &nell.actor, true).await;
    assert_eq!(status, StatusCode::OK, "{named}");
    assert_eq!(managing(&rig, &id).await, [mira.actor.clone(), nell.actor.clone()]);
    let (status, now) =
        change(&rig, &nell.token, "orchard", json!({ "name": "Quince trees", "topic": "pruned" })).await;
    assert_eq!(
        (status, now["name"].as_str(), now["topic"].as_str()),
        (StatusCode::OK, Some("quince-trees"), Some("pruned"))
    );
    // Nobody who is not in it is made one, and nobody who is not a person.
    let (status, refused) = manage(&rig, &mira.token, &id, "user:nobody", true).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");

    // Nell takes Mira off; the last manager stays.
    assert_eq!(manage(&rig, &nell.token, &id, &mira.actor, false).await.0, StatusCode::OK);
    let (status, refused) = manage(&rig, &nell.token, &id, &nell.actor, false).await;
    assert_eq!(status, StatusCode::CONFLICT, "{refused}");
    assert!(why(&refused).contains("last manager stays"), "{refused}");
    assert_eq!(change(&rig, &mira.token, &id, json!({ "topic": "mine" })).await.0, StatusCode::FORBIDDEN);

    // Whoever manages the factory does all of it, in it or not; with a key
    // of theirs that holds no `admin`, they are as anybody else.
    let (status, now) = change(&rig, &rig.token, &id, json!({ "topic": "for the shop's deploys" })).await;
    assert_eq!((status, now["topic"].as_str()), (StatusCode::OK, Some("for the shop's deploys")), "{now}");
    let less = key(&rig, &rig.token, &["channels"]).await;
    assert_eq!(change(&rig, &less, &id, json!({ "topic": "less" })).await.0, StatusCode::FORBIDDEN);
    assert_eq!(manage(&rig, &rig.token, &id, &mira.actor, true).await.0, StatusCode::OK);
    // The bot does it for the owner as anybody does: as whoever asks.
    let answered =
        asked_the_bot(&rig, &rig.token, "quince-trees", "@conductor channel rename quince-trees pears").await;
    assert_eq!(answered, "#quince-trees is called pears now.");
}

#[tokio::test]
async fn a_private_channels_details_are_told_to_its_members_alone() {
    let rig = Rig::new().await;
    rig.send("/api/tier", json!({ "tier": "premium" })).await;
    let (mira, otto) = (person(&rig, "Mira").await, person(&rig, "Otto").await);
    let made = json!({ "name": "Pay review", "kept": true });
    let (_, kept) = rig.call(Method::POST, "/api/channels", Some(&mira.token), Some(made)).await;
    let id = kept["id"].as_str().unwrap().to_string();
    // Somebody who is not in it is answered as for a channel that is not there.
    let (status, _) = change(&rig, &otto.token, &id, json!({ "topic": "gossip" })).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (mut to_mira, mut to_otto) =
        (crate::kept::Told::of(&rig, &mira.token).await, crate::kept::Told::of(&rig, &otto.token).await);
    assert_eq!(change(&rig, &mira.token, &id, json!({ "topic": "salaries" })).await.0, StatusCode::OK);
    to_mira.until("\"topic\":\"salaries\"").await;
    rig.send("/api/channels", json!({ "name": "lobby", "topic": "open" })).await;
    let (status, _) = change(&rig, &rig.token, "lobby", json!({ "topic": "everybody's" })).await;
    assert_eq!(status, StatusCode::OK);
    to_otto.until("everybody's").await;
    assert!(!to_otto.heard.contains("salaries") && !to_otto.heard.contains(&id), "{}", to_otto.heard);
}

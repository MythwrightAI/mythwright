//! What is withdrawn stops counting: a session that was ended and a key
//! that was withdrawn are nobody's word from then on, and that includes
//! what was already being answered for them.

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::{Rig, soon};

/// A stream of events opened with `token`, already answered.
async fn stream(rig: &Rig, token: &str) -> reqwest::Response {
    let opened = reqwest::Client::new()
        .get(format!("{}/api/events", rig.url))
        .bearer_auth(token)
        .send()
        .await
        .unwrap();
    assert_eq!(opened.status(), StatusCode::OK);
    opened
}

/// Waits for a stream to be ended from the daemon's side.
async fn ended(what: &str, mut stream: reqwest::Response) {
    soon(what, async { while let Ok(Some(_)) = stream.chunk().await {} }).await;
}

/// Another session of the owner's: its token.
async fn signed_in_again(rig: &Rig) -> String {
    let login = json!({ "email": "ada@example.com", "password": "correct horse" });
    let (status, signed) = rig.call(Method::POST, "/api/auth/login", None, Some(login)).await;
    assert_eq!(status, StatusCode::OK, "{signed}");
    signed["token"].as_str().unwrap().to_string()
}

/// A key the owner makes, holding these scopes or what the owner does: its id and its token.
async fn made(rig: &Rig, name: &str, scopes: Option<&[&str]>) -> (String, String) {
    let mut asked = json!({ "name": name });
    if let Some(scopes) = scopes {
        asked["scopes"] = json!(scopes);
    }
    let key: Value = rig.send("/api/credentials", asked).await;
    (key["id"].as_str().unwrap().to_string(), key["token"].as_str().unwrap().to_string())
}

#[tokio::test]
async fn signing_out_everywhere_ends_every_session_of_whoever_asks_and_what_each_was_being_told() {
    let rig = Rig::new().await;
    let other = signed_in_again(&rig).await;
    let key = rig.send("/api/credentials", json!({ "name": "script" })).await;
    let key = key["token"].as_str().unwrap().to_string();
    // Somebody else, signed in too.
    rig.send("/api/tier", json!({ "tier": "premium" })).await;
    let code = rig.send("/api/invites", json!({})).await["code"].as_str().unwrap().to_string();
    let grace =
        json!({ "email": "grace@example.com", "name": "Grace", "password": "correct horse", "invite": code });
    let (_, grace) = rig.call(Method::POST, "/api/auth/register", None, Some(grace)).await;
    let grace = grace["token"].as_str().unwrap().to_string();
    let (told, hers) = (stream(&rig, &other).await, stream(&rig, &grace).await);

    // A key is no session, and signs nobody out.
    let everywhere = json!({ "everywhere": true });
    let (status, refused) =
        rig.call(Method::POST, "/api/auth/logout", Some(&key), Some(everywhere.clone())).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    assert!(refused["error"]["message"].as_str().unwrap().contains("a key is no session"), "{refused}");
    // Nor does nobody.
    let (status, _) = rig.call(Method::POST, "/api/auth/logout", None, Some(everywhere.clone())).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let out = rig.send("/api/auth/logout", everywhere).await;
    assert_eq!((&out["sessions"], &out["keys"]), (&json!(2), &json!(1)), "{out}");
    for (whose, token) in [("the one that asked", rig.token.as_str()), ("the other", other.as_str())] {
        let (status, _) = rig.call(Method::GET, "/api/me", Some(token), None).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{whose} is still somebody's word");
    }
    // What the other session was being told stops being told.
    ended("the stream of a session that was ended to be over", told).await;

    // A key is withdrawn by its own name, and nobody else was signed out.
    for (whose, token) in [("their key", key.as_str()), ("somebody else", grace.as_str())] {
        let (status, _) = rig.call(Method::GET, "/api/me", Some(token), None).await;
        assert_eq!(status, StatusCode::OK, "{whose} was signed out too");
    }
    // Somebody else goes on being told: said with the key, heard by her.
    let (status, said) =
        rig.call(Method::POST, "/api/channels", Some(&key), Some(json!({ "name": "after" }))).await;
    assert_eq!(status, StatusCode::OK, "{said}");
    let mut hers = hers;
    soon("somebody else to go on being told", async {
        let mut heard = String::new();
        while !heard.contains("channel.created") {
            heard.push_str(&String::from_utf8_lossy(&hers.chunk().await.unwrap().unwrap()));
        }
    })
    .await;
    let said = rig.event("identity.signed_out", |_| true).await;
    assert_eq!(said.data["sessions"], 2, "{:?}", said.data);
}

#[tokio::test]
async fn what_a_key_was_being_told_stops_when_the_key_is_withdrawn() {
    let rig = Rig::new().await;
    let ((gone, gone_token), (_, kept_token)) =
        (made(&rig, "gone", None).await, made(&rig, "kept", None).await);
    let (told, mut kept) = (stream(&rig, &gone_token).await, stream(&rig, &kept_token).await);

    let (status, revoked) =
        rig.call(Method::DELETE, &format!("/api/credentials/{gone}"), Some(&rig.token), None).await;
    assert_eq!(status, StatusCode::OK, "{revoked}");
    ended("the stream of a key that was withdrawn to be over", told).await;

    // The other key's goes on.
    rig.send("/api/channels", json!({ "name": "after" })).await;
    soon("the key that was kept to go on being told", async {
        let mut heard = String::new();
        while !heard.contains("channel.created") {
            heard.push_str(&String::from_utf8_lossy(&kept.chunk().await.unwrap().unwrap()));
        }
    })
    .await;
}

/// A blueprint that stays, listening, once it is applied.
fn stays(name: &str) -> String {
    format!(
        "blueprint: {name}\non:\n  heard:\n    event: channel.message\n    steps:\n      again: {{ do: emit, with: {{ topic: {name}.heard }} }}\n"
    )
}

/// Applies `source` as `token`: the instance.
async fn applied(rig: &Rig, token: &str, source: &str) -> String {
    let asked = json!({ "source": source, "wait": true });
    let (status, applied) = rig.call(Method::POST, "/api/apply", Some(token), Some(asked)).await;
    assert_eq!((status, &applied["finished"]["status"]), (StatusCode::OK, &json!("ok")), "{applied}");
    applied["instance"].as_str().unwrap().to_string()
}

/// The instances that are alive, by id.
async fn alive(rig: &Rig) -> Vec<String> {
    let all = rig.read("/api/instances?live=true").await;
    all["instances"].as_array().unwrap().iter().map(|one| one["id"].as_str().unwrap().to_string()).collect()
}

/// What was said on `topic`, oldest first.
async fn said(rig: &Rig, topic: &str) -> Vec<Value> {
    let query = journal::EventQuery { topics: topic.to_string(), limit: 1000, ..Default::default() };
    rig.said(query).await.into_iter().map(|event| event.data).collect()
}

/// Withdraws the key `id` as the owner, saying `how` after it: the answer.
async fn withdrawn(rig: &Rig, id: &str, how: &str) -> Value {
    let (status, answer) =
        rig.call(Method::DELETE, &format!("/api/credentials/{id}{how}"), Some(&rig.token), None).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    answer
}

/// The ids of the instances listed under `field` of an answer.
fn ids(answer: &Value, field: &str) -> Vec<String> {
    answer[field].as_array().unwrap().iter().map(|one| one["id"].as_str().unwrap().to_string()).collect()
}

#[tokio::test]
async fn withdrawing_a_key_stops_what_it_applied_and_leaves_it_running_only_where_it_is_told_to() {
    let rig = Rig::new().await;
    let scopes: &[&str] = &["blueprints", "channels", "emit"];
    let ((first, first_token), (second, second_token)) =
        (made(&rig, "first", Some(scopes)).await, made(&rig, "second", Some(scopes)).await);
    // Each key applies something that stays; so does the owner, signed in.
    let by_first = applied(&rig, &first_token, &stays("one")).await;
    let by_second = applied(&rig, &second_token, &stays("two")).await;
    let by_owner = applied(&rig, &rig.token, &stays("mine")).await;
    // Who somebody is with a key says which key it is.
    let (_, me) = rig.call(Method::GET, "/api/me", Some(&first_token), None).await;
    assert_eq!(me["principal"]["key"], first, "{me}");

    // Asked first, what withdrawing it would come to is said and nothing
    // is changed: what it applied, its own and nobody else's.
    let look = rig.read(&format!("/api/credentials/{first}/withdrawing")).await;
    assert_eq!(ids(&look, "running"), std::slice::from_ref(&by_first), "{look}");
    assert_eq!((&look["running"][0]["blueprint"], &look["keys"]), (&json!("one"), &json!([])), "{look}");
    let (status, _) = rig.call(Method::GET, "/api/me", Some(&first_token), None).await;
    assert_eq!(status, StatusCode::OK, "asking first withdrew it");

    // Withdrawn, the key is nobody's, and what it applied is stopped with
    // it, as stopping it by hand stops it, saying why.
    let said_now = withdrawn(&rig, &first, "").await;
    assert_eq!(ids(&said_now, "stopped"), std::slice::from_ref(&by_first), "{said_now}");
    assert_eq!((&said_now["running"], &said_now["left"]), (&json!([]), &json!([])), "{said_now}");
    let (status, _) = rig.call(Method::GET, "/api/me", Some(&first_token), None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let stopped =
        rig.event("instance.stopped", |event| event.source.instance.as_deref() == Some(&by_first)).await;
    assert_eq!(stopped.data["why"], "withdrawn", "{:?}", stopped.data);
    let still = alive(&rig).await;
    assert!(!still.contains(&by_first), "{still:?}");

    // Told to leave it, what the key applied goes on with what it held,
    // the answer says which it is, and what is said of the key says it
    // was left.
    let left = withdrawn(&rig, &second, "?leave=true").await;
    assert_eq!(ids(&left, "running"), std::slice::from_ref(&by_second), "{left}");
    assert_eq!(left["stopped"], json!([]), "{left}");
    let still = alive(&rig).await;
    assert!(still.contains(&by_second) && still.contains(&by_owner), "{still:?}");
    let revoked = said(&rig, "identity.key.revoked").await;
    assert_eq!(revoked.len(), 2, "{revoked:?}");
    assert_eq!((&revoked[0]["key"], &revoked[0]["leave"]), (&json!(first), &Value::Null), "{revoked:?}");
    assert_eq!((&revoked[1]["key"], &revoked[1]["leave"]), (&json!(second), &json!(true)), "{revoked:?}");
}

#[tokio::test]
async fn a_key_made_with_a_key_is_withdrawn_with_it_and_what_it_applied_is_stopped_too() {
    let rig = Rig::new().await;
    // A key that holds less, a key made with that, and one made with that.
    let scopes: &[&str] = &["blueprints", "channels", "emit"];
    let (first, first_token) = made(&rig, "first", Some(scopes)).await;
    let made_with = |token: String, name: &'static str| {
        let rig = &rig;
        async move {
            let asked = json!({ "name": name, "scopes": ["blueprints", "emit"] });
            let (status, key) = rig.call(Method::POST, "/api/credentials", Some(&token), Some(asked)).await;
            assert_eq!(status, StatusCode::OK, "{key}");
            (key["id"].as_str().unwrap().to_string(), key["token"].as_str().unwrap().to_string())
        }
    };
    let (second, second_token) = made_with(first_token.clone(), "second").await;
    let (third, third_token) = made_with(second_token.clone(), "third").await;
    let (other, other_token) = made(&rig, "other", Some(scopes)).await;
    let by_third = applied(&rig, &third_token, &stays("deep")).await;
    let by_other = applied(&rig, &other_token, &stays("beside")).await;
    let made_said = said(&rig, "identity.key.made").await;
    let of = |key: &str| made_said.iter().find(|one| one["key"] == key).unwrap()["with"].clone();
    assert_eq!((of(&second), of(&third), of(&first)), (json!(first), json!(second), Value::Null));

    // Asked first, what goes with it is said: the keys made with it, all
    // the way down, and what those applied.
    let look = rig.read(&format!("/api/credentials/{first}/withdrawing")).await;
    let named: Vec<&str> =
        look["keys"].as_array().unwrap().iter().map(|one| one["id"].as_str().unwrap()).collect();
    assert_eq!(named, [second.as_str(), third.as_str()], "{look}");
    assert_eq!(ids(&look, "running"), std::slice::from_ref(&by_third), "{look}");

    // Withdrawn, each is nobody's, what the last applied is stopped, and
    // what another key of the same person applied goes on.
    let gone = withdrawn(&rig, &first, "").await;
    assert_eq!(ids(&gone, "stopped"), std::slice::from_ref(&by_third), "{gone}");
    for (which, token) in [("first", &first_token), ("second", &second_token), ("third", &third_token)] {
        let (status, _) = rig.call(Method::GET, "/api/me", Some(token), None).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "the {which} is still somebody's credential");
    }
    let (status, _) = rig.call(Method::GET, "/api/me", Some(&other_token), None).await;
    assert_eq!(status, StatusCode::OK);
    let still = alive(&rig).await;
    assert!(!still.contains(&by_third) && still.contains(&by_other), "{still:?}");
    let revoked = said(&rig, "identity.key.revoked").await;
    let with: Vec<(&str, &Value)> =
        revoked.iter().map(|one| (one["key"].as_str().unwrap(), &one["with"])).collect();
    assert_eq!(
        with,
        [(first.as_str(), &Value::Null), (second.as_str(), &json!(first)), (third.as_str(), &json!(first))]
    );
    // Withdrawing what is already withdrawn finds no key in use.
    let (status, _) =
        rig.call(Method::DELETE, &format!("/api/credentials/{second}"), Some(&rig.token), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let _ = other;
}

#[tokio::test]
async fn a_rolled_key_is_the_same_key_with_another_secret_and_what_it_applied_goes_on() {
    let rig = Rig::new().await;
    let scopes: &[&str] = &["blueprints", "channels", "emit"];
    let ((key, old), (other, other_token)) =
        (made(&rig, "rolled", Some(scopes)).await, made(&rig, "other", Some(scopes)).await);
    let by_key = applied(&rig, &old, &stays("kept")).await;
    let told = stream(&rig, &old).await;

    // A key that holds less may not roll another of its maker's.
    let (status, refused) = rig
        .call(Method::POST, &format!("/api/credentials/{key}/roll"), Some(&other_token), Some(json!({})))
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    let (status, refused) =
        rig.call(Method::DELETE, &format!("/api/credentials/{key}"), Some(&other_token), None).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");

    // It may roll itself, as a script that changes its key on a schedule does.
    let (status, rolled) =
        rig.call(Method::POST, &format!("/api/credentials/{key}/roll"), Some(&old), Some(json!({}))).await;
    assert_eq!(status, StatusCode::OK, "{rolled}");
    let new = rolled["token"].as_str().unwrap().to_string();
    assert_eq!((&rolled["id"], &rolled["scopes"]), (&json!(key), &json!(scopes)), "{rolled}");
    assert_ne!(new, old);
    // The secret it had is refused from then, and what it was told ends.
    let (status, _) = rig.call(Method::GET, "/api/me", Some(&old), None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    ended("the stream of a key's old secret to be over", told).await;
    // The new one is the same key, holding what it held.
    let (status, me) = rig.call(Method::GET, "/api/me", Some(&new), None).await;
    assert_eq!((status, &me["principal"]["key"]), (StatusCode::OK, &json!(key)), "{me}");
    assert!(alive(&rig).await.contains(&by_key), "what it applied was stopped as it was rolled");
    let rolled_said = said(&rig, "identity.key.rolled").await;
    assert_eq!((rolled_said.len(), &rolled_said[0]["key"]), (1, &json!(key)), "{rolled_said:?}");

    // Withdrawn after, what it applied before it was rolled is its own still.
    let gone = withdrawn(&rig, &key, "").await;
    assert_eq!(ids(&gone, "stopped"), [by_key], "{gone}");
    let _ = other;
}

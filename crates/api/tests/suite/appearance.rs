//! How what somebody reads looks for them, kept with their account
//! (`people::appearance`), by the routes that read and change it.
//!
//! One journey in one factory, with its owner, a person, and keys of
//! theirs that hold less: a setting kept and there for the same person
//! signed in again, as in another browser; refused in a sentence where it
//! is not one; nobody else reading or hearing it, the owner of the factory
//! included.

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;
use crate::kept::{Told, asked, key, person, reads_none, signed_in};

/// What `token` is handed of its setting.
async fn appearance(rig: &Rig, token: &str) -> Value {
    let (status, kept) = rig.call(Method::GET, "/api/appearance", Some(token), None).await;
    assert_eq!(status, StatusCode::OK, "{kept}");
    kept
}

/// Keeps `body` as `token`'s setting, which is to go well: the answer.
async fn keeps(rig: &Rig, token: &str, body: Value) -> Value {
    let (status, kept) = rig.call(Method::PUT, "/api/appearance", Some(token), Some(body)).await;
    assert_eq!(status, StatusCode::OK, "{kept}");
    kept
}

#[tokio::test]
async fn how_what_somebody_reads_looks_is_theirs_wherever_they_sign_in_and_nobody_elses() {
    let owned = Rig::new().await;
    let rig = &owned;
    rig.send("/api/tier", json!({ "tier": "premium" })).await;
    let mira = person(rig, "Mira").await;
    let narrow = key(rig, &mira.token, &["channels"]).await;
    let reads = key(rig, &mira.token, &["themes:read"]).await;
    let (mut to_mira, mut to_owner) = (Told::of(rig, &mira.token).await, Told::of(rig, &rig.token).await);

    // 1: nothing is kept for anybody yet.
    assert_eq!(appearance(rig, &mira.token).await, json!({ "appearance": null, "nth": 0, "changed": null }));

    // 2: what is not a setting is refused in a sentence, and nothing is kept.
    for (body, says) in [
        (json!({ "width": "wide", "margins": "none" }), "nothing said of how what one reads looks"),
        (json!({ "width": "widest" }), "is none of them"),
        (json!("dark"), "a mapping of `text`, `width`, `colour`, `contents` and `typeface`"),
    ] {
        let (status, refused) =
            asked(rig, &mira.token, Method::PUT, "/api/appearance", Some(body.clone())).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{body}: {refused}");
        assert!(refused.contains(says), "{body}: {refused}");
    }
    assert_eq!(appearance(rig, &mira.token).await["appearance"], Value::Null);

    // 3: kept, it is hers, said to her alone and whole, and there for her
    // signed in again, as in another browser, and for a key of hers that
    // reads it.
    let kept = keeps(rig, &mira.token, json!({ "width": "wide", "text": "largest" })).await;
    let hers = json!({ "text": "largest", "width": "wide", "colour": "automatic", "contents": "shown", "typeface": "sans" });
    assert_eq!((&kept["appearance"], &kept["nth"]), (&hers, &json!(1)));
    to_mira.until("largest").await;
    let elsewhere = signed_in(rig, "Mira").await;
    assert_eq!(appearance(rig, &elsewhere).await, kept);
    assert_eq!(appearance(rig, &reads).await, kept);
    assert_eq!(keeps(rig, &elsewhere, hers.clone()).await["nth"], 1, "the same again is no write");

    // 4: nobody else reads it, the owner of the factory included, and a
    // key that holds less is refused for the scope it lacks.
    assert_eq!(appearance(rig, &rig.token).await["appearance"], Value::Null);
    let (status, refused) = asked(rig, &narrow, Method::GET, "/api/appearance", None).await;
    assert!(status == StatusCode::FORBIDDEN && refused.contains("needs `themes:read`"), "{refused}");
    reads_none("by a key that holds nothing of it", &refused, &["largest"]);
    let (status, refused) = asked(rig, &reads, Method::PUT, "/api/appearance", Some(hers)).await;
    assert!(status == StatusCode::FORBIDDEN && refused.contains("needs `themes:write`"), "{refused}");

    // 5: what the owner was told is theirs alone, and what Mira was told hers.
    keeps(rig, &rig.token, json!({ "colour": "light", "text": "smaller" })).await;
    to_owner.until("smaller").await;
    keeps(rig, &mira.token, json!({ "colour": "dark", "text": "larger" })).await;
    to_mira.until("\"dark\"").await;
    reads_none("on what the owner is told", &to_owner.heard, &["largest", "\"dark\""]);
    reads_none("on what Mira is told", &to_mira.heard, &["smaller", "\"light\""]);
}

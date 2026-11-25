//! How a time is written for somebody, kept with their account
//! (`people::clocked`), by the routes that read and change it.
//!
//! One journey in one factory, with its owner, a person, and keys of
//! theirs that hold less: a setting kept and there for the same person
//! signed in again, as in another browser; refused in a sentence where it
//! is not one; the zone a browser says handed over first and taken only
//! where nothing was kept; nobody else reading or hearing it, the owner of
//! the factory included.

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;
use crate::kept::{Told, asked, key, person, reads_none, signed_in};

/// What `token` is handed of its setting.
async fn clock(rig: &Rig, token: &str) -> Value {
    let (status, kept) = rig.call(Method::GET, "/api/clock", Some(token), None).await;
    assert_eq!(status, StatusCode::OK, "{kept}");
    kept
}

/// Keeps `body` as `token`'s setting, which is to go well: the answer.
async fn keeps(rig: &Rig, token: &str, path: &str, body: Value) -> Value {
    let method = if path.ends_with("first") { Method::POST } else { Method::PUT };
    let (status, kept) = rig.call(method, path, Some(token), Some(body)).await;
    assert_eq!(status, StatusCode::OK, "{kept}");
    kept
}

#[tokio::test]
async fn how_a_time_is_written_for_somebody_is_theirs_wherever_they_sign_in_and_nobody_elses() {
    let owned = Rig::new().await;
    let rig = &owned;
    rig.send("/api/tier", json!({ "tier": "premium" })).await;
    let mira = person(rig, "Mira").await;
    let narrow = key(rig, &mira.token, &["channels"]).await;
    let reads = key(rig, &mira.token, &["clock:read"]).await;
    let (mut to_mira, mut to_owner) = (Told::of(rig, &mira.token).await, Told::of(rig, &rig.token).await);

    // 1: nothing is kept for anybody yet.
    assert_eq!(clock(rig, &mira.token).await, json!({ "clock": null, "nth": 0, "changed": null }));

    // 2: what is not a setting is refused in a sentence, and nothing is kept.
    for (body, says) in [
        (json!({ "zone": "America/Denver", "seconds": true }), "nothing said of how a time is written"),
        (json!({ "zone": "Mountain Time" }), "is none of them"),
        (json!({ "zone": "America/Denver", "hours": "13h" }), "`12h` or `24h`"),
    ] {
        let (status, refused) = asked(rig, &mira.token, Method::PUT, "/api/clock", Some(body.clone())).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{body}: {refused}");
        assert!(refused.contains(says), "{body}: {refused}");
    }
    assert_eq!(clock(rig, &mira.token).await["clock"], Value::Null);

    // 3: kept, it is hers, said to her alone and whole, and there for her
    // signed in again, as in another browser, and for a key of hers that
    // reads it.
    let hers = json!({ "zone": "America/Denver", "hours": "12h" });
    let kept = keeps(rig, &mira.token, "/api/clock", hers.clone()).await;
    assert_eq!((&kept["clock"], &kept["nth"]), (&hers, &json!(1)));
    to_mira.until("America/Denver").await;
    let elsewhere = signed_in(rig, "Mira").await;
    assert_eq!(clock(rig, &elsewhere).await, kept);
    assert_eq!(clock(rig, &reads).await, kept);
    assert_eq!(
        keeps(rig, &elsewhere, "/api/clock", hers.clone()).await["nth"],
        1,
        "the same again is no write"
    );

    // 4: the zone a browser says is handed over first: taken only where
    // nothing is kept, and answered with what is kept either way.
    let browser = json!({ "zone": "Pacific/Auckland" });
    assert_eq!(keeps(rig, &mira.token, "/api/clock/first", browser.clone()).await, kept);
    let owners = keeps(rig, &rig.token, "/api/clock/first", browser).await;
    assert_eq!(owners["clock"], json!({ "zone": "Pacific/Auckland", "hours": "12h" }));

    // 5: nobody else reads it, the owner of the factory included, and a
    // key that holds less is refused for the scope it lacks.
    assert_eq!(clock(rig, &rig.token).await, owners);
    let (status, refused) = asked(rig, &narrow, Method::GET, "/api/clock", None).await;
    assert!(status == StatusCode::FORBIDDEN && refused.contains("needs `clock:read`"), "{refused}");
    reads_none("by a key that holds nothing of it", &refused, &["Denver"]);
    let (status, refused) = asked(rig, &reads, Method::PUT, "/api/clock", Some(hers)).await;
    assert!(status == StatusCode::FORBIDDEN && refused.contains("needs `clock:write`"), "{refused}");

    // 6: what the owner was told is theirs alone, and what Mira was told hers.
    keeps(rig, &rig.token, "/api/clock", json!({ "zone": "Europe/Lisbon", "hours": "24h" })).await;
    to_owner.until("Europe/Lisbon").await;
    keeps(rig, &mira.token, "/api/clock", json!({ "zone": "America/Phoenix" })).await;
    to_mira.until("America/Phoenix").await;
    reads_none("on what the owner is told", &to_owner.heard, &["Denver", "Phoenix"]);
    reads_none("on what Mira is told", &to_mira.heard, &["Auckland", "Lisbon"]);
}

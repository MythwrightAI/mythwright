//! How somebody arranged the side panel for themselves, kept with their
//! account (`people::arranged`), by the routes that read and change it.
//!
//! One journey in one factory, with its owner, a person, and keys of
//! theirs that hold less: an arrangement kept and there for the same
//! person signed in again, as in another browser; refused in a sentence
//! where it is not one; what a browser kept before handed over first and
//! taken only where nothing was kept; nobody else reading or hearing it,
//! the owner of the factory included; and a copy of the factory keeping
//! what each had.

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;
use crate::kept::{Told, asked, key, person, reads_none, signed_in};
use crate::snapshots::{Shared, taken};

/// What `token` is handed of its arrangement.
async fn arranged(rig: &Rig, token: &str) -> Value {
    let (status, kept) = rig.call(Method::GET, "/api/arranged", Some(token), None).await;
    assert_eq!(status, StatusCode::OK, "{kept}");
    kept
}

/// Keeps `body` as `token`'s arrangement, which is to go well: the answer.
async fn keeps(rig: &Rig, token: &str, path: &str, body: Value) -> Value {
    let method = if path.ends_with("first") { Method::POST } else { Method::PUT };
    let (status, kept) = rig.call(method, path, Some(token), Some(body)).await;
    assert_eq!(status, StatusCode::OK, "{kept}");
    kept
}

fn hers() -> Value {
    json!({ "groups": [{ "id": "g1", "name": "Quince", "channels": ["ch_greengage", "ch_medlar"] }],
            "pinned": ["ch_damson"], "things": ["/board/sloe"] })
}

#[tokio::test]
async fn how_somebody_arranged_the_side_panel_is_theirs_wherever_they_sign_in_and_nobody_elses() {
    let shared = Shared::new();
    let registry = shared.registry();
    let rig = Rig::with("api-arranged", move |config, _| {
        config.profile = Some("prod".into());
        config.registry = Some(registry);
    })
    .await;
    let owned = Rig::owned(rig).await;
    let rig = &owned;
    rig.send("/api/tier", json!({ "tier": "premium" })).await;
    let mira = person(rig, "Mira").await;
    let narrow = key(rig, &mira.token, &["channels"]).await;
    let reads = key(rig, &mira.token, &["arranged:read"]).await;
    let (mut to_mira, mut to_owner) = (Told::of(rig, &mira.token).await, Told::of(rig, &rig.token).await);

    // 1: nothing is kept for anybody yet.
    assert_eq!(arranged(rig, &mira.token).await, json!({ "arrangement": null, "nth": 0, "changed": null }));

    // 2: what is not an arrangement is refused in a sentence, and nothing is kept.
    for (body, says) in [
        (json!({ "folds": {} }), "nothing an arrangement says"),
        (json!({ "groups": [{ "id": "g1", "name": "" }] }), "is called something"),
        (json!({ "pinned": ["ch_a", "ch_a"] }), "said twice"),
    ] {
        let (status, refused) =
            asked(rig, &mira.token, Method::PUT, "/api/arranged", Some(body.clone())).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{body}: {refused}");
        assert!(refused.contains(says), "{body}: {refused}");
    }
    assert_eq!(arranged(rig, &mira.token).await["arrangement"], Value::Null);

    // 3: kept, it is hers, said to her alone and whole, and there for her
    // signed in again, as in another browser, and for a key of hers that
    // reads it.
    let kept = keeps(rig, &mira.token, "/api/arranged", hers()).await;
    assert_eq!((&kept["arrangement"], &kept["nth"]), (&hers(), &json!(1)));
    to_mira.until("ch_greengage").await;
    let elsewhere = signed_in(rig, "Mira").await;
    assert_eq!(arranged(rig, &elsewhere).await, kept);
    assert_eq!(arranged(rig, &reads).await, kept);
    // The same again is no write and nothing said.
    assert_eq!(keeps(rig, &elsewhere, "/api/arranged", hers()).await["nth"], 1);

    // 4: what a browser kept before is handed over first: taken only where
    // nothing is kept, and answered with what is kept either way.
    let before = json!({ "pinned": ["ch_bullace"] });
    assert_eq!(keeps(rig, &mira.token, "/api/arranged/first", before.clone()).await, kept);
    let owners = keeps(rig, &rig.token, "/api/arranged/first", before.clone()).await;
    assert_eq!((&owners["arrangement"]["pinned"], &owners["nth"]), (&json!(["ch_bullace"]), &json!(1)));

    // 5: nobody else reads it, the owner of the factory included, and a
    // key that holds less is refused for the scope it lacks.
    assert_eq!(arranged(rig, &rig.token).await, owners);
    let (status, refused) = asked(rig, &narrow, Method::GET, "/api/arranged", None).await;
    assert!(status == StatusCode::FORBIDDEN && refused.contains("needs `arranged:read`"), "{refused}");
    reads_none("by a key that holds nothing of it", &refused, &["Quince", "greengage"]);
    let (status, refused) = asked(rig, &reads, Method::PUT, "/api/arranged", Some(json!({}))).await;
    assert!(status == StatusCode::FORBIDDEN && refused.contains("needs `arranged:write`"), "{refused}");

    // 6: what the owner was told is theirs alone, and what Mira was told hers.
    keeps(rig, &rig.token, "/api/arranged", json!({ "pinned": ["ch_mirabelle"] })).await;
    to_owner.until("ch_mirabelle").await;
    let marker = json!({ "groups": [{ "id": "g1", "name": "Quince", "channels": ["ch_marker"] }] });
    keeps(rig, &mira.token, "/api/arranged", marker).await;
    to_mira.until("ch_marker").await;
    reads_none("on what the owner is told", &to_owner.heard, &["Quince", "greengage", "medlar"]);
    reads_none("on what Mira is told", &to_mira.heard, &["mirabelle", "bullace"]);

    // 7: a copy of the factory keeps what each had.
    let to = shared.snapshot("copy");
    taken(rig, &to).await;
    let copy = shared.copy("dev1", &to).await;
    let mira_there = signed_in(&copy, "Mira").await;
    assert_eq!(arranged(&copy, &mira_there).await, arranged(rig, &mira.token).await);
    assert_eq!(arranged(&copy, &copy.token).await, arranged(rig, &rig.token).await);
}

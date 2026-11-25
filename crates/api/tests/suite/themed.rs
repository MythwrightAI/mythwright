//! The themes somebody makes for the client, kept with their account
//! (`people::themes`), by the routes that read and change them.
//!
//! One journey in one factory, with its owner, a person, and keys of
//! theirs that hold less: a theme made, refused where its colours or its
//! name are not a theme's, chosen, and there for the same person signed
//! in again elsewhere; nobody else reading, changing, choosing or hearing
//! it, the owner included, in the words a theme that never was is
//! refused in; the owner offering one of theirs to everybody, a member
//! finding it with who offered it and choosing it, a member refused the
//! offering and the withdrawing, and the owner withdrawing it; and a copy
//! of the factory keeping what each had.

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;
use crate::kept::{Told, asked, key, person, reads_none, signed_in};
use crate::snapshots::{Shared, taken};

fn dusk() -> Value {
    json!({ "ground": "#191724", "text": "#e0def4", "accent": "#ebbcba", "ok": "#9ccfd8", "bad": "#eb6f92",
            "info": "#c4a7e7", "agent": "#31748f", "card": "#f6c177" })
}

/// What `token` is handed of themes.
async fn themes(rig: &Rig, token: &str) -> Value {
    let (status, kept) = rig.call(Method::GET, "/api/themes", Some(token), None).await;
    assert_eq!(status, StatusCode::OK, "{kept}");
    kept
}

/// Asks something of themes as `token`, which is to go well: the answer.
async fn does(rig: &Rig, token: &str, method: Method, path: &str, body: Option<Value>) -> Value {
    let (status, answer) = rig.call(method, path, Some(token), body).await;
    assert_eq!(status, StatusCode::OK, "{path}: {answer}");
    answer
}

fn ids(themes: &Value) -> Vec<&str> {
    themes.as_array().unwrap().iter().map(|theme| theme["id"].as_str().unwrap()).collect()
}

#[tokio::test]
async fn a_theme_somebody_makes_is_theirs_wherever_they_sign_in_and_an_admin_offers_one_to_everybody() {
    let shared = Shared::new();
    let registry = shared.registry();
    let rig = Rig::with("api-themes", move |config, _| {
        config.profile = Some("prod".into());
        config.registry = Some(registry);
    })
    .await;
    let owned = Rig::owned(rig).await;
    let rig = &owned;
    rig.send("/api/tier", json!({ "tier": "premium" })).await;
    let mira = person(rig, "Mira").await;
    // Keys of Mira's: one that holds nothing of themes, and one that
    // reads her own and changes none; and one of the owner's that changes
    // themes and does not manage the factory.
    let narrow = key(rig, &mira.token, &["channels"]).await;
    let reads = key(rig, &mira.token, &["themes:read"]).await;
    let unmanaging = key(rig, &rig.token, &["themes"]).await;
    let (mut to_mira, mut to_owner, mut to_reads) =
        (Told::of(rig, &mira.token).await, Told::of(rig, &rig.token).await, Told::of(rig, &reads).await);

    // 1: nothing is kept for anybody yet; the usual theme is chosen.
    assert_eq!(themes(rig, &mira.token).await, json!({ "own": [], "offered": [], "chosen": null }));

    // 2: what is not a theme is refused in a sentence, and nothing is kept.
    let refusals = [
        (json!({ "name": "", "colours": dusk() }), "is called something"),
        (json!({ "name": "Dusk", "colours": { "ground": "#191724" } }), "text is not a colour"),
        (json!({ "name": "Dusk", "colours": near() }), "too near the ground"),
        (json!({ "name": "Dusk" }), "a mapping of eight"),
        (
            json!({ "name": "Dusk", "colours": dusk(), "light": near() }),
            "In the light version: The text is too near",
        ),
        (
            json!({ "name": "Dusk", "colours": dusk(), "light": "#faf4ed" }),
            "In the light version: A theme's colours",
        ),
        (json!({ "name": "Dusk", "colours": dusk(), "look": { "size": "huge" } }), "is none of them"),
        (
            json!({ "name": "Dusk", "colours": dusk(), "look": { "colour": "red" } }),
            "nothing a theme's look says",
        ),
    ];
    for (body, says) in refusals {
        let (status, refused) =
            asked(rig, &mira.token, Method::POST, "/api/themes", Some(body.clone())).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{body}: {refused}");
        assert!(refused.contains(says), "{body}: {refused}");
    }
    assert_eq!(themes(rig, &mira.token).await["own"], json!([]));

    // 3: made, it is hers, said to her alone with the theme whole, and
    // handed to her by a key of hers that reads themes.
    let made = does(
        rig,
        &mira.token,
        Method::POST,
        "/api/themes",
        Some(json!({ "name": "Dusk", "colours": dusk() })),
    )
    .await;
    let dusk_id = made["id"].as_str().unwrap().to_string();
    assert_eq!(
        (&made["owner"], &made["owner_name"], &made["offered"]),
        (&json!(mira.actor), &json!("Mira"), &Value::Null)
    );
    to_mira.until(&dusk_id).await;
    to_reads.until(&dusk_id).await;
    assert_eq!(ids(&themes(rig, &reads).await["own"]), [dusk_id.as_str()]);
    // Handed over again to the letter, it is the one there is.
    let again = does(
        rig,
        &mira.token,
        Method::POST,
        "/api/themes",
        Some(json!({ "name": "Dusk", "colours": dusk() })),
    )
    .await;
    assert_eq!(again["id"], made["id"]);

    // 4: chosen, and there for her signed in again, as in another browser.
    let chosen = json!({ "theme": dusk_id });
    does(rig, &mira.token, Method::PUT, "/api/themes/chosen", Some(chosen.clone())).await;
    let elsewhere = signed_in(rig, "Mira").await;
    let there = themes(rig, &elsewhere).await;
    assert_eq!((ids(&there["own"]), &there["chosen"]), (vec![dusk_id.as_str()], &json!(dusk_id)));
    assert_eq!(there["own"][0]["colours"], dusk());
    assert_eq!(there["own"][0]["light"], Value::Null, "one made with no light version has none");
    assert_eq!(there["own"][0]["look"], json!({}), "and one made with no look says nothing of it");
    // Changed, it is the same theme with what it now says, a light version
    // and a look with it, which are said with the theme and handed with it.
    let look = json!({ "size": "largest", "density": "compact", "letters": "usual" });
    let changed = json!({ "name": "Late dusk", "colours": dusk(), "light": dawn(), "look": look });
    let late = does(rig, &mira.token, Method::PUT, &format!("/api/themes/{dusk_id}"), Some(changed)).await;
    assert_eq!((&late["id"], &late["name"], &late["light"]), (&json!(dusk_id), &json!("Late dusk"), &dawn()));
    let kept_look = json!({ "size": "largest", "density": "compact" });
    assert_eq!(late["look"], kept_look, "what is usual is kept as nothing said");
    to_mira.until("#faf4ed").await;
    let there = themes(rig, &elsewhere).await;
    assert_eq!((&there["own"][0]["light"], &there["own"][0]["look"]), (&dawn(), &kept_look));

    // 5: nobody else reads, changes, chooses or offers it, the owner of
    // the factory included, and each is refused as for one that never
    // was; a key that holds less is refused for the scope it lacks.
    assert_eq!(themes(rig, &rig.token).await, json!({ "own": [], "offered": [], "chosen": null }));
    let body = json!({ "name": "Mine now", "colours": dusk() });
    for (method, path, body) in [
        (Method::PUT, format!("/api/themes/{dusk_id}"), Some(body.clone())),
        (Method::DELETE, format!("/api/themes/{dusk_id}"), None),
        (Method::POST, format!("/api/themes/{dusk_id}/offer"), None),
    ] {
        let hers = asked(rig, &rig.token, method.clone(), &path, body.clone()).await;
        let never = asked(rig, &rig.token, method, &path.replace(&dusk_id, "thm_never"), body).await;
        assert_eq!(hers.0, StatusCode::NOT_FOUND, "{path}: {}", hers.1);
        assert_eq!((hers.0, hers.1.replace(&dusk_id, "thm_never")), never, "{path}");
    }
    let (status, refused) =
        asked(rig, &rig.token, Method::PUT, "/api/themes/chosen", Some(chosen.clone())).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{refused}");
    let (status, refused) = asked(rig, &narrow, Method::GET, "/api/themes", None).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    assert!(refused.contains("needs `themes:read`"), "{refused}");
    reads_none("by a key that holds no themes", &refused, &["Dusk", &dusk_id]);
    let (status, refused) =
        asked(rig, &reads, Method::POST, "/api/themes", Some(json!({ "name": "X", "colours": dusk() })))
            .await;
    assert!(status == StatusCode::FORBIDDEN && refused.contains("needs `themes:write`"), "{refused}");
    // Choosing is of what there is: a theme that ships, or nothing.
    does(rig, &rig.token, Method::PUT, "/api/themes/chosen", Some(json!({ "theme": "rose-pine" }))).await;
    let (status, refused) =
        asked(rig, &rig.token, Method::PUT, "/api/themes/chosen", Some(json!({ "theme": "neon" }))).await;
    assert!(status == StatusCode::NOT_FOUND && refused.contains("no theme `neon`"), "{refused}");

    // 6: the owner offers one of theirs to everybody; Mira is told, finds
    // it with who offered it, and chooses it. She may not offer hers, nor
    // may a key of the owner's that does not manage the factory.
    let harbour = does(
        rig,
        &rig.token,
        Method::POST,
        "/api/themes",
        Some(json!({ "name": "Harbour", "colours": dusk() })),
    )
    .await;
    let harbour_id = harbour["id"].as_str().unwrap().to_string();
    for (token, path) in [
        (&mira.token, format!("/api/themes/{dusk_id}/offer")),
        (&unmanaging, format!("/api/themes/{harbour_id}/offer")),
    ] {
        let (status, refused) = asked(rig, token, Method::POST, &path, None).await;
        assert!(status == StatusCode::FORBIDDEN && refused.contains("needs `admin`"), "{refused}");
    }
    let offered = does(rig, &rig.token, Method::POST, &format!("/api/themes/{harbour_id}/offer"), None).await;
    assert_eq!(offered["offered"]["name"], "Ada");
    to_mira.until("theme.offered").await;
    assert!(to_mira.heard.contains("Harbour"));
    let hers = themes(rig, &mira.token).await;
    assert_eq!(ids(&hers["offered"]), [harbour_id.as_str()]);
    assert_eq!(
        (&hers["offered"][0]["owner_name"], &hers["offered"][0]["offered"]["by"]),
        (&json!("Ada"), &json!(format!("user:{}", owner_id(rig).await)))
    );
    does(rig, &mira.token, Method::PUT, "/api/themes/chosen", Some(json!({ "theme": harbour_id }))).await;
    // Hers is not offered to the owner by being chosen or anything else.
    assert_eq!(themes(rig, &rig.token).await["offered"], json!([]));

    // 7: a member withdraws nothing; the owner withdraws it, and it is
    // chosen by nobody but its owner, who still has it.
    let path = format!("/api/themes/{harbour_id}/offer");
    let (status, refused) = asked(rig, &mira.token, Method::DELETE, &path, None).await;
    assert!(status == StatusCode::FORBIDDEN && refused.contains("needs `admin`"), "{refused}");
    let withdrawn = does(rig, &rig.token, Method::DELETE, &path, None).await;
    assert_eq!(withdrawn["offered"], Value::Null);
    to_mira.until("theme.withdrawn").await;
    let hers = themes(rig, &mira.token).await;
    assert_eq!((&hers["offered"], &hers["chosen"]), (&json!([]), &Value::Null));
    assert_eq!(ids(&themes(rig, &rig.token).await["own"]), [harbour_id.as_str()]);
    let (status, refused) = asked(rig, &rig.token, Method::DELETE, &path, None).await;
    assert!(status == StatusCode::NOT_FOUND && refused.contains("offered to everybody"), "{refused}");
    does(rig, &mira.token, Method::PUT, "/api/themes/chosen", Some(chosen.clone())).await;

    // 8: the owner heard nothing of Mira's own, from first to last.
    does(rig, &rig.token, Method::POST, "/api/themes", Some(json!({ "name": "Marker", "colours": dusk() })))
        .await;
    to_owner.until("Marker").await;
    reads_none("on what the owner is told", &to_owner.heard, &["Dusk", &dusk_id]);
    reads_none("on what Mira is told", &to_mira.heard, &["Marker"]);

    // 9: a copy of the factory keeps what each had.
    let to = shared.snapshot("copy");
    taken(rig, &to).await;
    let copy = shared.copy("dev1", &to).await;
    let mira_there = signed_in(&copy, "Mira").await;
    assert_eq!(themes(&copy, &mira_there).await, themes(rig, &mira.token).await);
    assert_eq!(themes(&copy, &copy.token).await, themes(rig, &rig.token).await);

    // 10: taken away, it is gone and chosen no longer.
    does(rig, &mira.token, Method::DELETE, &format!("/api/themes/{dusk_id}"), None).await;
    assert_eq!(themes(rig, &mira.token).await, json!({ "own": [], "offered": [], "chosen": null }));
}

/// The light version of Dusk.
fn dawn() -> Value {
    json!({ "ground": "#faf4ed", "text": "#575279", "accent": "#b4637a", "ok": "#286983", "bad": "#a8324a",
            "info": "#56949f", "agent": "#907aa9", "card": "#ea9d34" })
}

/// The colours of Dusk with text that cannot be read on its ground.
fn near() -> Value {
    let mut colours = dusk();
    colours["text"] = json!("#1a1825");
    colours
}

async fn owner_id(rig: &Rig) -> String {
    let (_, me) = rig.call(Method::GET, "/api/me", Some(&rig.token), None).await;
    me["principal"]["id"].as_str().unwrap().to_string()
}

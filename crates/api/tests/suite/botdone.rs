//! What the factory's bot did lately, for whoever reads about it
//! (`/api/system/speaker/history`): the lines of records' histories a
//! blueprint's step wrote, worded as everywhere else, and what it said in
//! channels; each held to what whoever asks may read, so that a record of
//! a table kept from them, a board they may not read and a channel kept
//! to its members are not handed, nor said to be there.

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;
use crate::kept::{key, person};

/// A card added and a row of a table kept private, each by a step; and
/// something said by the bot where anybody reads it and where few do.
const DOES: &str = r#"
blueprint: does
permissions: [board, records, projects, channels]
params: { kept: { type: string } }
steps:
  card: { do: board.add, with: { project: shop, title: Ledger } }
  notes:
    do: collection.ensure
    with: { name: notes, reads: "private:notes", fields: { title: { type: text, required: true } }, history: [record.*] }
  note: { needs: [notes], do: record.add, with: { collection: notes, fields: { title: Quietly } } }
  open: { do: channel.post, with: { channel: general, body: Said where anybody reads. } }
  among: { do: channel.post, with: { channel: "${params.kept}", body: Said among few. } }
"#;

/// What the bot did lately as `token` is handed it: each line's kind and words.
async fn done(rig: &Rig, token: &str) -> Vec<(String, String)> {
    let (status, answer) =
        rig.call(Method::GET, "/api/system/speaker/history?limit=50", Some(token), None).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    let line =
        |one: &Value| (one["kind"].as_str().unwrap().to_string(), one["said"].as_str().unwrap().to_string());
    answer["done"].as_array().unwrap().iter().map(line).collect()
}

#[tokio::test]
async fn what_the_bot_did_lately_is_said_as_the_histories_say_it_and_holds_nothing_kept_from_whoever_reads() {
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    rig.board("shop").await;
    rig.send("/api/channels", json!({ "name": "general" })).await;
    let kept = rig.send("/api/channels", json!({ "name": "Few", "kept": true })).await;
    let kept = kept["id"].as_str().unwrap().to_string();
    let applied = rig
        .send(
            "/api/apply",
            json!({ "source": DOES, "project": "shop", "params": { "kept": kept }, "wait": true }),
        )
        .await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
    let record = |said: &str| ("record".to_string(), said.to_string());
    let said = |words: &str| ("said".to_string(), words.to_string());
    let card = record("conductor added it to draft");
    let note = record("conductor: created (collection notes)");
    let (open, among) = (said("Said where anybody reads."), said("Said among few."));
    // Its lines are written a moment after what they say: waited for.
    crate::soon("the bot's lines to be written", async {
        while done(&rig, &rig.token).await.len() < 4 {
            rig.event("collection.history", |_| true).await;
        }
    })
    .await;

    // Whoever reads everything is handed all of it, newest first, each
    // with the blueprint it was done through.
    let all = done(&rig, &rig.token).await;
    for one in [&card, &note, &open, &among] {
        assert!(all.contains(one), "{one:?} in {all:?}");
    }
    let (_, answer) = rig.call(Method::GET, "/api/system/speaker/history", Some(&rig.token), None).await;
    assert_eq!(answer["name"], "conductor");
    let first = answer["done"].as_array().unwrap();
    assert!(first.iter().all(|one| one["via"] == "does"), "{answer:#}");
    assert!(
        first.windows(2).all(|two| two[0]["ts"].as_i64() >= two[1]["ts"].as_i64()),
        "newest first: {answer:#}"
    );
    let line = first.iter().find(|one| one["kind"] == "record" && one["collection"] == "board").unwrap();
    assert_eq!((&line["record"], &line["project"]), (&json!("shop-1"), &json!("shop")), "{line}");

    // Somebody who is not in the kept channel and may not read the private
    // table is handed only what they read: the board and the open
    // channel. A key of theirs that holds less is handed less: one that
    // reads channels and no board, what was said where anybody reads; one
    // that reads neither, nothing.
    rig.send("/api/tier", json!({ "tier": "premium" })).await;
    let grace = person(&rig, "Grace").await;
    let theirs: std::collections::HashSet<_> = done(&rig, &grace.token).await.into_iter().collect();
    assert_eq!(theirs, [card.clone(), open.clone()].into());
    let channels = key(&rig, &grace.token, &["channels:read"]).await;
    assert_eq!(done(&rig, &channels).await, std::slice::from_ref(&open));
    let nothing = key(&rig, &grace.token, &["records:read"]).await;
    assert_eq!(done(&rig, &nothing).await, Vec::<(String, String)>::new());

    // Called something else, the bot is said by its new name in every line.
    rig.send("/api/system/speaker", json!({ "name": "foreman" })).await;
    assert!(done(&rig, &grace.token).await.contains(&record("foreman added it to draft")));
}

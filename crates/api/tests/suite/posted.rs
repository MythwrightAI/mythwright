//! A file of a project posted as that file (`routes/posted.rs`): an agent
//! that attaches a file of the channel's project, saying where it has it
//! as the command line does, posts a message that names the file and
//! keeps no copy of it; whoever reads the message reads the file as it is
//! then, as far as they may read that project's files, and one that went
//! is said to be gone. A copy attached is written among a project's files
//! by whoever may write them, and the message names that file from then
//! on. Of a channel kept to its members, nothing of either is told
//! outside it; and a copy of the factory and the Trash keep what names a
//! file as it is.

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;
use crate::kept::{Told, key, person, say};
use crate::snapshots::{Shared, taken};

/// A project `shop` with a file of its own, and where its folder is, whole.
async fn shop(rig: &Rig) -> std::path::PathBuf {
    rig.send("/api/projects", json!({ "name": "Shop", "set_up": ["docs"] })).await;
    write(rig, "docs/distribution.md", "# Distribution\n\nBy the river.\n").await;
    kernel::whole::whole(&rig.node().app.studio.projects.dir_of("shop").unwrap()).unwrap()
}

async fn write(rig: &Rig, path: &str, content: &str) {
    let body = Some(json!({ "path": path, "content": content }));
    let (status, wrote) = rig.call(Method::PUT, "/api/projects/shop/file", Some(&rig.token), body).await;
    assert_eq!(status, StatusCode::OK, "{wrote}");
}

/// Puts `holds` in `channel` as `token`, saying it is at `from` where that is said: what is kept.
async fn attach(
    rig: &Rig,
    token: &str,
    channel: &str,
    name: &str,
    from: Option<&std::path::Path>,
    holds: &str,
) -> Value {
    let mut to = format!("{}/api/channels/{channel}/attachments?name={name}", rig.url);
    if let Some(from) = from {
        to.push_str(&format!("&from={}", encoded(&from.to_string_lossy())));
    }
    let sent = rig.client.post(to).bearer_auth(token).body(holds.to_string()).send().await.unwrap();
    assert_eq!(sent.status(), StatusCode::OK);
    sent.json().await.unwrap()
}

/// `said` as it goes in an address.
fn encoded(said: &str) -> String {
    said.bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (byte as char).to_string(),
            _ => format!("%{byte:02X}"),
        })
        .collect()
}

/// Says `body` in `channel` with `attached` as `token`: the message.
async fn post(rig: &Rig, token: &str, channel: &str, attached: &Value) -> Value {
    let body = json!({ "body": "The distribution.", "attachments": [attached["id"]] });
    let path = format!("/api/channels/{channel}/messages");
    let (status, said) = rig.call(Method::POST, &path, Some(token), Some(body)).await;
    assert_eq!(status, StatusCode::OK, "{said}");
    said
}

/// What is attached to the message `id` of `channel`, as `token` reads the channel.
async fn attached_as(rig: &Rig, token: &str, channel: &str, id: &str) -> Value {
    let (status, read) =
        rig.call(Method::GET, &format!("/api/channels/{channel}/messages"), Some(token), None).await;
    assert_eq!(status, StatusCode::OK, "{read}");
    let found = read["messages"].as_array().unwrap().iter().find(|message| message["id"] == id).cloned();
    found.unwrap_or_else(|| panic!("the message is there: {read}"))["attachments"][0].clone()
}

/// The file of an attachment as `token` is handed it: how it went, and what it holds.
async fn file(rig: &Rig, token: &str, id: &str) -> (StatusCode, String) {
    let answer =
        rig.client.get(format!("{}/api/attachments/{id}", rig.url)).bearer_auth(token).send().await.unwrap();
    (answer.status(), answer.text().await.unwrap())
}

/// The credential an agent's program reads each time it asks the factory anything.
fn token(rig: &Rig, name: &str) -> String {
    std::fs::read_to_string(rig.dir.join(format!("data/slots/{name}/token"))).unwrap().trim().to_string()
}

#[tokio::test]
async fn a_project_file_an_agent_attaches_is_posted_as_that_file_and_read_as_each_reader_may_read_it() {
    let rig = Rig::new().await;
    rig.send("/api/tier", json!({ "tier": "premium" })).await;
    let root = shop(&rig).await;
    rig.send("/api/channels", json!({ "name": "shop-talk", "project": "shop" })).await;
    rig.send("/api/agents", json!({ "name": "writer", "project": "shop", "channel": "shop-talk" })).await;
    let writer = token(&rig, "writer");
    let holds = "# Distribution\n\nBy the river.\n";

    // Said where it has it, the file it wrote is posted as that file: named, and not kept here.
    let from = root.join("docs").join("distribution.md");
    let kept = attach(&rig, &writer, "shop-talk", "distribution.md", Some(&from), holds).await;
    assert_eq!(
        kept["file"],
        json!({ "project": "shop", "path": "docs/distribution.md", "state": "there" }),
        "{kept}"
    );
    assert_eq!(kept["size"], holds.len());
    let id = kept["id"].as_str().unwrap().to_string();
    assert!(!rig.dir.join("data/attachments").join(&id).exists(), "no copy of it is kept");
    let said = post(&rig, &writer, "shop-talk", &kept).await;
    assert_eq!(said["attachments"][0]["file"]["path"], "docs/distribution.md", "{said}");
    assert_eq!(said["attachments"][0]["file"]["state"], "there", "the answer says what it is now: {said}");
    let message = said["id"].as_str().unwrap().to_string();

    // It is the file and not a copy: changed in the project, it is read as it is now, with its size now.
    let now = "# Distribution\n\nBy the river, and by road.\n";
    write(&rig, "docs/distribution.md", now).await;
    assert_eq!(file(&rig, &rig.token, &id).await, (StatusCode::OK, now.to_string()));
    let seen = attached_as(&rig, &rig.token, "shop-talk", &message).await;
    assert_eq!(
        (seen["file"]["state"].as_str(), seen["size"].as_u64()),
        (Some("there"), Some(now.len() as u64))
    );

    // Whoever reads the channel and not the project's files sees that it names a file, and nothing of the file.
    let mira = person(&rig, "Mira").await;
    let reads = key(&rig, &mira.token, &["channels:read"]).await;
    let theirs = attached_as(&rig, &reads, "shop-talk", &message).await;
    assert_eq!(
        (theirs["file"]["state"].as_str(), theirs["size"].as_u64()),
        (Some("kept"), Some(0)),
        "{theirs}"
    );
    assert_eq!(file(&rig, &reads, &id).await.0, StatusCode::FORBIDDEN);
    let (status, seen) =
        rig.call(Method::GET, &format!("/api/attachments/{id}/seen"), Some(&reads), None).await;
    assert_eq!((status, seen["file"]["state"].as_str()), (StatusCode::OK, Some("kept")), "{seen}");

    // Bytes that are not what the file holds, and a file outside the project, are kept as copies.
    let other = attach(&rig, &writer, "shop-talk", "distribution.md", Some(&from), "something else").await;
    assert_eq!(other["file"], Value::Null, "{other}");
    let outside = rig.dir.join("elsewhere.md");
    std::fs::write(&outside, "far away").unwrap();
    let outside = kernel::whole::whole(&outside).unwrap();
    let far = attach(&rig, &writer, "shop-talk", "elsewhere.md", Some(&outside), "far away").await;
    assert_eq!(far["file"], Value::Null, "{far}");

    // A file that goes is said to be gone, and offers nothing.
    let (status, gone) = rig
        .call(Method::DELETE, "/api/projects/shop/file?path=docs%2Fdistribution.md", Some(&rig.token), None)
        .await;
    assert_eq!(status, StatusCode::OK, "{gone}");
    let seen = attached_as(&rig, &rig.token, "shop-talk", &message).await;
    assert_eq!(seen["file"]["state"], "gone", "{seen}");
    assert_eq!(file(&rig, &rig.token, &id).await.0, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn a_copy_is_added_to_a_project_by_whoever_may_write_its_files_and_the_message_names_that_file_from_then_on()
 {
    let rig = Rig::new().await;
    rig.send("/api/tier", json!({ "tier": "premium" })).await;
    shop(&rig).await;
    rig.send("/api/channels", json!({ "name": "shop-talk", "project": "shop" })).await;
    let kept = attach(&rig, &rig.token, "shop-talk", "brief.md", None, "# Brief\n").await;
    assert_eq!(kept["file"], Value::Null, "nothing said of where it is: a copy");
    let id = kept["id"].as_str().unwrap().to_string();
    let message = post(&rig, &rig.token, "shop-talk", &kept).await["id"].as_str().unwrap().to_string();
    let to = format!("/api/attachments/{id}/project");
    let asked = json!({ "project": "shop", "path": "docs/brief.md" });

    // Whoever may write in the channel and not the project's files is refused, and nothing is written.
    let mira = person(&rig, "Mira").await;
    let talks = key(&rig, &mira.token, &["channels", "projects:read"]).await;
    let (status, refused) = rig.call(Method::POST, &to, Some(&talks), Some(asked.clone())).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    let dir = rig.node().app.studio.projects.dir_of("shop").unwrap();
    assert!(!dir.join("docs/brief.md").exists());
    // A path where something is already is refused, in the daemon's sentence, and the message is as it was.
    let there = json!({ "project": "shop", "path": "docs/distribution.md" });
    let (status, refused) = rig.call(Method::POST, &to, Some(&rig.token), Some(there)).await;
    assert_eq!(status, StatusCode::CONFLICT, "{refused}");
    assert_eq!(attached_as(&rig, &rig.token, "shop-talk", &message).await["file"], Value::Null);

    // Whoever may is answered with what it is now, the file is written, the message names it, and that is said.
    let mark = rig.mark();
    let (status, placed) = rig.call(Method::POST, &to, Some(&rig.token), Some(asked.clone())).await;
    assert_eq!(status, StatusCode::OK, "{placed}");
    assert_eq!(placed["file"], json!({ "project": "shop", "path": "docs/brief.md", "state": "there" }));
    assert_eq!(std::fs::read_to_string(dir.join("docs/brief.md")).unwrap(), "# Brief\n");
    assert!(!rig.dir.join("data/attachments").join(&id).exists(), "the copy went");
    let said = rig.event("channel.attachment.placed", move |event| event.seq > mark).await;
    assert_eq!(
        (said.data["message"].as_str(), said.data["path"].as_str()),
        (Some(message.as_str()), Some("docs/brief.md"))
    );
    let seen = attached_as(&rig, &rig.token, "shop-talk", &message).await;
    assert_eq!(seen["file"]["path"], "docs/brief.md", "{seen}");
    // Asked again, it is a file of the project already.
    let (status, refused) = rig.call(Method::POST, &to, Some(&rig.token), Some(asked)).await;
    assert_eq!(status, StatusCode::CONFLICT, "{refused}");
}

#[tokio::test]
async fn a_private_channels_posted_file_is_told_nobody_outside_it_and_is_kept_as_it_is_in_a_copy_and_the_trash()
 {
    let shared = Shared::new();
    let rig = shared.factory("prod").await;
    rig.send("/api/tier", json!({ "tier": "premium" })).await;
    let root = shop(&rig).await;
    let (mira, otto) = (person(&rig, "Mira").await, person(&rig, "Otto").await);
    let made = json!({ "name": "Pay review", "kept": true, "project": "shop" });
    let (status, kept) = rig.call(Method::POST, "/api/channels", Some(&mira.token), Some(made)).await;
    assert_eq!(status, StatusCode::OK, "{kept}");
    let channel = kept["id"].as_str().unwrap().to_string();
    rig.send("/api/channels", json!({ "name": "lobby" })).await;
    let mut to_otto = Told::of(&rig, &otto.token).await;

    // Mira posts the project's file, and adds a copy to the project: Otto, who is not in it, hears of neither.
    let from = root.join("docs").join("distribution.md");
    let posted = attach(
        &rig,
        &mira.token,
        &channel,
        "distribution.md",
        Some(&from),
        "# Distribution\n\nBy the river.\n",
    )
    .await;
    assert_eq!(posted["file"]["path"], "docs/distribution.md", "{posted}");
    let message = post(&rig, &mira.token, &channel, &posted).await["id"].as_str().unwrap().to_string();
    let copy = attach(&rig, &mira.token, &channel, "pay.md", None, "# Pay\n").await;
    post(&rig, &mira.token, &channel, &copy).await;
    let asked = json!({ "project": "shop", "path": "pay.md" });
    let to = format!("/api/attachments/{}/project", copy["id"].as_str().unwrap());
    let (status, placed) = rig.call(Method::POST, &to, Some(&mira.token), Some(asked)).await;
    assert_eq!(status, StatusCode::OK, "{placed}");
    let (status, _) = rig
        .call(Method::POST, &to, Some(&otto.token), Some(json!({ "project": "shop", "path": "x.md" })))
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "a channel kept from him has nothing of his to add");
    assert_eq!(file(&rig, &otto.token, posted["id"].as_str().unwrap()).await.0, StatusCode::NOT_FOUND);
    say(&rig, &rig.token, "lobby", json!({ "body": "after the greengage" })).await;
    to_otto.until("after the greengage").await;
    for words in ["The distribution.", "channel.attachment.placed", "att_", "Pay review"] {
        assert!(!to_otto.heard.contains(words), "{words} was told outside the channel: {}", to_otto.heard);
    }

    // A copy of the factory keeps what names a file as it is.
    let copied = shared.snapshot("posted");
    taken(&rig, &copied).await;
    let dev = shared.copy("dev", &copied).await;
    let there = attached_as(&dev, &dev.token, &channel, &message).await;
    assert_eq!(there["file"]["path"], "docs/distribution.md", "{there}");

    // Into the Trash with the channel, and given back with it, naming the same file.
    let (status, gone) =
        rig.call(Method::DELETE, &format!("/api/channels/{channel}"), Some(&rig.token), None).await;
    assert_eq!(status, StatusCode::OK, "{gone}");
    let trash = gone["trash"].as_str().unwrap();
    rig.send(&format!("/api/trash/{trash}/restore"), json!({})).await;
    let back = attached_as(&rig, &mira.token, &channel, &message).await;
    assert_eq!(
        (back["file"]["path"].as_str(), back["file"]["state"].as_str()),
        (Some("docs/distribution.md"), Some("there"))
    );
}

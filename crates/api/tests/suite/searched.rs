//! Searching everything (`GET /api/search`): a table's rows, the topics and
//! what was said on them, a library's documents, and projects, channels,
//! people, agents and runs by what they are called. Each is found with
//! where it is, by whoever may read it and by nobody else, and as it is
//! now once it changed.

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;
use crate::kept::Person;

/// A project's tables: rows of its own, a library of guides, and rows
/// kept private.
const TABLES: &str = r#"
blueprint: tables
permissions: [projects]
steps:
  bugs:
    do: collection.ensure
    with:
      name: bugs
      fields: { title: { type: text, required: true }, notes: { type: text }, size: { type: number } }
  guides:
    do: collection.ensure
    with:
      name: guides
      store: documents
      at: notes/guides
      fields: { title: { type: text, required: true }, body: { type: text }, path: { type: text } }
  moods:
    do: collection.ensure
    with:
      name: moods
      reads: private:journal
      fields: { title: { type: text, required: true }, went: { type: text } }
"#;

/// Registers `name`, invited by the owner.
async fn person(rig: &Rig, name: &str, email: &str) -> Person {
    let code =
        rig.send("/api/invites", json!({ "email": email })).await["code"].as_str().unwrap().to_string();
    let body = json!({ "email": email, "name": name, "password": "correct horse", "invite": code });
    let (status, made) = rig.call(Method::POST, "/api/auth/register", None, Some(body)).await;
    assert_eq!(status, StatusCode::OK, "{made}");
    Person {
        token: made["token"].as_str().unwrap().to_string(),
        actor: format!("user:{}", made["account"]["id"].as_str().unwrap()),
    }
}

/// Writes a file of a project.
async fn write(rig: &Rig, project: &str, path: &str, content: &str) {
    let body = json!({ "path": path, "content": content });
    let (status, written) =
        rig.call(Method::PUT, &format!("/api/projects/{project}/file"), Some(&rig.token), Some(body)).await;
    assert_eq!(status, StatusCode::OK, "{written}");
}

async fn key(rig: &Rig, scopes: &[&str]) -> String {
    let made = rig.send("/api/credentials", json!({ "name": "helper", "scopes": scopes })).await;
    made["token"].as_str().unwrap().to_string()
}

/// The project `life` with its tables.
async fn life(rig: &Rig) {
    rig.send("/api/projects", json!({ "name": "Life" })).await;
    let applied = rig.send("/api/apply", json!({ "source": TABLES, "project": "life", "wait": true })).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
}

/// What a search for `words` finds as `token` (the owner where none),
/// once the index has taken in everything that has happened.
async fn found(rig: &Rig, token: Option<&str>, words: &str) -> Vec<Value> {
    rig.node().app.studio.search.catch_up().await.unwrap();
    let (status, answer) = match token {
        Some(token) => rig.call(Method::GET, &format!("/api/search?q={words}"), Some(token), None).await,
        None => rig.get(&format!("/api/search?q={words}")).await,
    };
    assert_eq!(status, StatusCode::OK, "{answer}");
    answer["hits"].as_array().unwrap().clone()
}

/// Of what was found, each one's kind and title.
fn called(hits: &[Value]) -> Vec<(String, String)> {
    let said = |hit: &Value, key: &str| hit[key].as_str().unwrap_or_default().to_string();
    hits.iter().map(|hit| (said(hit, "kind"), said(hit, "title"))).collect()
}

fn pair(kind: &str, title: &str) -> (String, String) {
    (kind.to_string(), title.to_string())
}

/// A topic whole once `replies` replies of it are in its history.
async fn with_replies(rig: &Rig, topic: &str, replies: usize) -> Value {
    loop {
        let read = rig.read(&format!("/api/topics/{topic}")).await;
        if read["replies"].as_array().map_or(0, Vec::len) >= replies {
            return read;
        }
        let mark = rig.mark();
        rig.event("collection.history", |event| event.seq > mark).await;
    }
}

#[tokio::test]
async fn a_row_of_any_table_is_found_by_its_words_in_its_table_and_by_nobody_it_is_kept_from() {
    let rig = Rig::new().await;
    life(&rig).await;
    let bug = rig
        .send(
            "/api/collections/life/bugs/records",
            json!({ "fields": { "title": "Pantry door", "notes": "the hinge squeaks" } }),
        )
        .await;
    let id = bug["id"].as_str().unwrap().to_string();
    rig.send(
        "/api/collections/life/moods/records",
        json!({ "fields": { "title": "Sunday", "went": "the hinge again" } }),
    )
    .await;

    let hits = found(&rig, None, "hinge").await;
    let mut both = called(&hits);
    both.sort();
    assert_eq!(both, [pair("record", "Pantry door"), pair("record", "Sunday")], "{hits:?}");
    let door = hits.iter().find(|hit| hit["title"] == "Pantry door").unwrap();
    assert_eq!((door["project"].as_str(), door["place"].as_str()), (Some("life"), Some("bugs")), "{door}");
    assert_eq!(door["link"], json!({ "id": id, "at": "life", "table": "bugs", "where": "bugs" }));
    assert!(door["snippet"].as_str().unwrap().contains("\u{e000}hinge\u{e001}"), "{door}");

    // A key that may read the project's rows finds what is open; one that may not, none.
    let reads = key(&rig, &["search", "projects:read", "records:read"]).await;
    assert_eq!(called(&found(&rig, Some(&reads), "hinge").await), [pair("record", "Pantry door")]);
    let not = key(&rig, &["search", "projects:read"]).await;
    assert_eq!(called(&found(&rig, Some(&not), "hinge").await), []);
    // And what is kept private is found with what it is kept under.
    let private = key(&rig, &["search", "records:read", "private:journal"]).await;
    assert_eq!(called(&found(&rig, Some(&private), "hinge").await).len(), 2);

    // Changed, it is found by what it says now; taken away, by nothing.
    let (status, changed) = rig
        .call(
            Method::PATCH,
            &format!("/api/records/{id}"),
            Some(&rig.token),
            Some(json!({ "fields": { "notes": "oiled" } })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{changed}");
    assert_eq!(called(&found(&rig, None, "hinge").await), [pair("record", "Sunday")]);
    assert_eq!(called(&found(&rig, None, "oiled").await), [pair("record", "Pantry door")]);
    let (status, removed) =
        rig.call(Method::DELETE, &format!("/api/records/{id}"), Some(&rig.token), None).await;
    assert_eq!(status, StatusCode::OK, "{removed}");
    assert_eq!(called(&found(&rig, None, "oiled").await), []);
}

#[tokio::test]
async fn a_topic_and_what_was_said_on_it_are_found_under_its_wiki_and_a_reply_by_its_number() {
    let rig = Rig::new().await;
    let wiki = rig.send("/api/topics", json!({ "title": "Handbook", "body": "Everything we do." })).await;
    let wiki = wiki["id"].as_str().unwrap().to_string();
    let page = rig
        .send(
            "/api/topics",
            json!({ "title": "Rolling back", "body": "Restore the quince snapshot.", "under": wiki }),
        )
        .await;
    let page = page["id"].as_str().unwrap().to_string();
    let replied =
        rig.send(&format!("/api/topics/{page}/replies"), json!({ "body": "the medlar way is faster" })).await;
    let seq = replied["seq"].as_u64().expect("the number it was said at");

    let hits = found(&rig, None, "quince").await;
    assert_eq!(called(&hits), [pair("topic", "Rolling back")]);
    assert_eq!(
        (hits[0]["place"].as_str(), hits[0]["link"]["id"].as_str()),
        (Some("Handbook"), Some(page.as_str()))
    );
    let hits = found(&rig, None, "medlar").await;
    assert_eq!(called(&hits), [pair("topic", "Rolling back")]);
    assert_eq!(hits[0]["link"]["reply"], json!(seq), "{hits:?}");

    // Topics are read by whoever may read records.
    let not = key(&rig, &["search", "projects:read", "channels:read"]).await;
    assert_eq!(called(&found(&rig, Some(&not), "medlar quince").await), []);
    assert_eq!(called(&found(&rig, Some(&not), "quince").await), []);

    // The wiki called something else, the page under it is found under that.
    let (status, renamed) = rig
        .call(
            Method::PATCH,
            &format!("/api/records/{wiki}"),
            Some(&rig.token),
            Some(json!({ "fields": { "title": "Runbook" } })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{renamed}");
    let hits = found(&rig, None, "quince").await;
    assert_eq!(hits[0]["place"], "Runbook", "{hits:?}");

    // A full pass after, once what was said is in the topic's history,
    // writes nothing: everything is as it was read.
    with_replies(&rig, &page, 1).await;
    let search = &rig.node().app.studio.search;
    search.reindex().await.unwrap();
    assert_eq!(search.reindex().await.unwrap(), 0, "a second pass finds everything as it was");
    assert_eq!(called(&found(&rig, None, "medlar").await), [pair("topic", "Rolling back")]);
}

#[tokio::test]
async fn a_document_of_a_library_is_found_as_one_at_the_heading_nearest_what_matched() {
    let rig = Rig::new().await;
    life(&rig).await;
    let text = "# Install\n\nGet the tools.\n\n## Rolling back\n\nRestore the greengage snapshot.\n";
    write(&rig, "life", "notes/guides/ops/install.md", text).await;
    write(&rig, "life", "notes/loose.md", "a greengage elsewhere").await;

    let hits = found(&rig, None, "greengage").await;
    let mut kinds = called(&hits);
    kinds.sort();
    assert_eq!(kinds, [pair("document", "Install"), pair("file", "notes/loose.md")], "{hits:?}");
    let install = hits.iter().find(|hit| hit["kind"] == "document").unwrap();
    assert_eq!(install["place"], "guides › ops");
    assert_eq!(
        (&install["link"]["path"], &install["link"]["line"], &install["link"]["heading"]),
        (&json!("notes/guides/ops/install.md"), &json!(5), &json!("Rolling back")),
        "{install}"
    );
    // Narrowed to documents, only it.
    rig.node().app.studio.search.catch_up().await.unwrap();
    let narrowed = rig.read("/api/search?q=greengage&kind=document").await;
    assert_eq!(called(narrowed["hits"].as_array().unwrap()), [pair("document", "Install")]);
    // Written again, it is found by what it says now.
    let again = "# Install\n\nRestore the damson snapshot.\n";
    write(&rig, "life", "notes/guides/ops/install.md", again).await;
    rig.node().app.studio.search.catch_up().await.unwrap();
    rig.node().app.studio.search.reindex().await.unwrap();
    assert_eq!(called(&found(&rig, None, "damson").await), [pair("document", "Install")]);
}

#[tokio::test]
async fn projects_channels_people_agents_and_runs_are_found_by_what_they_are_called_by_whoever_may_read_them()
{
    let rig = Rig::new().await;
    rig.send("/api/tier", json!({ "tier": "premium" })).await;
    rig.send("/api/projects", json!({ "name": "Riverside Shop" })).await;
    let nell = person(&rig, "Nell Quarry", "nell@example.com").await;
    rig.send("/api/channels", json!({ "name": "quarry-talk" })).await;
    let kept = rig.send("/api/channels", json!({ "name": "Quarry secrets", "kept": true })).await;
    let kept = kept["id"].as_str().unwrap().to_string();
    let source = "blueprint: quarrying\nsteps:\n  a: { do: noop }\n";
    let applied = rig.send("/api/apply", json!({ "source": source, "wait": true })).await;
    let run = applied["finished"]["id"].as_str().expect("the run it started").to_string();

    let hits = found(&rig, None, "river").await;
    assert_eq!(called(&hits)[0], pair("project", "Riverside Shop"), "{hits:?}");
    assert_eq!(hits[0]["link"]["id"], "riverside-shop");
    let mut quarry = called(&found(&rig, None, "quarr").await);
    quarry.sort();
    assert_eq!(
        quarry,
        [
            pair("channel", "#quarry-talk"),
            pair("channel", "Quarry secrets"),
            pair("person", "Nell Quarry"),
            pair("run", "quarrying")
        ]
    );
    let hits = found(&rig, None, &run).await;
    assert_eq!(called(&hits), [pair("run", &run)], "{hits:?}");
    assert_eq!(hits[0]["link"]["run"], json!(run));

    // Somebody who is not in the channel kept to its members does not find it.
    let theirs: Vec<(String, String)> = called(&found(&rig, Some(&nell.token), "quarr").await);
    assert!(theirs.contains(&pair("channel", "#quarry-talk")), "{theirs:?}");
    assert!(!theirs.contains(&pair("channel", "Quarry secrets")), "{theirs:?}");
    // Nobody is found by a key that may read none of it.
    let none = key(&rig, &["search"]).await;
    assert_eq!(called(&found(&rig, Some(&none), "quarr").await), []);
    assert_eq!(called(&found(&rig, Some(&none), "river").await), []);
    // Called something else, a channel is found by that, and by nothing else.
    let (status, renamed) = rig
        .call(
            Method::PATCH,
            "/api/channels/quarry-talk",
            Some(&rig.token),
            Some(json!({ "name": "pit-talk" })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{renamed}");
    let hits = called(&found(&rig, None, "pit").await);
    assert_eq!(hits, [pair("channel", "#pit-talk")]);
    assert!(!called(&found(&rig, None, "quarry talk").await).contains(&pair("channel", "#quarry-talk")));
    // The one kept to its members, by them.
    let (status, mine) = rig.get(&format!("/api/channels/{kept}")).await;
    assert_eq!((status, &mine["title"]), (StatusCode::OK, &json!("Quarry secrets")));

    // An agent, by its name, for whoever may read agents.
    rig.send(
        "/api/agents",
        json!({ "name": "flintknapper", "adapter": "builtin", "project": "riverside-shop" }),
    )
    .await;
    // And the blueprint that brings it up, applied under its name.
    assert_eq!(
        called(&found(&rig, None, "flint").await),
        [pair("agent", "flintknapper"), pair("run", "std/agent · flintknapper")]
    );
    let channels = key(&rig, &["search", "channels:read"]).await;
    assert_eq!(called(&found(&rig, Some(&channels), "flint").await), []);
}

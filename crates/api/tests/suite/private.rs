//! A table that is kept private: its records, the files they are kept in
//! and what is said of them are read by whoever holds what it is kept
//! under, and by nobody who only may read the project.
//!
//! Each way of reading is tried with a key that holds what an agent
//! usually holds, which is everything of the project but that.

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;

/// A person's notes: the days are theirs alone, the guides are the
/// project's, and how each day went is kept in the factory's own database.
const NOTES: &str = r#"
blueprint: notes
permissions: [projects]
steps:
  days:
    do: collection.ensure
    with:
      name: days
      store: documents
      at: notes/days
      reads: private:journal
      fields: { title: { type: text, required: true }, body: { type: text }, path: { type: text } }
      history: { record.created: "{actor} wrote it", record.updated: "{actor} changed its {changed}", record.commented: "{actor} said: {body}", record.removed: "{actor} took it away" }
  guides:
    do: collection.ensure
    with:
      name: guides
      store: documents
      at: notes/guides
      fields: { title: { type: text, required: true }, body: { type: text }, path: { type: text } }
      history: { record.created: "{actor} wrote it", record.commented: "{actor} said: {body}" }
  moods:
    do: collection.ensure
    with:
      name: moods
      reads: private:journal
      fields: { title: { type: text, required: true }, went: { type: text } }
      history: { record.created: "{actor} added it", record.commented: "{actor} said: {body}", record.removed: "{actor} took it away" }
"#;

async fn key(rig: &Rig, scopes: &[&str]) -> String {
    let made = rig.send("/api/credentials", json!({ "name": "helper", "scopes": scopes })).await;
    made["token"].as_str().unwrap().to_string()
}

fn said(body: &Value) -> String {
    body["error"]["message"].as_str().unwrap_or_default().to_string()
}

/// The project `life`, its tables, and a note of each kind written by its owner.
async fn life(rig: &Rig) -> std::path::PathBuf {
    let project = rig.send("/api/projects", json!({ "name": "Life" })).await;
    let applied = rig.send("/api/apply", json!({ "source": NOTES, "project": "life", "wait": true })).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
    for (path, content) in [
        ("notes/days/2026-11-14.md", "# Saturday\n\nThe marmalade is under the stairs.\n"),
        ("notes/guides/bread.md", "# Bread\n\nFlour, water, salt, and marmalade on it afterwards.\n"),
    ] {
        let (status, written) = rig
            .call(
                Method::PUT,
                "/api/projects/life/file",
                Some(&rig.token),
                Some(json!({ "path": path, "content": content })),
            )
            .await;
        assert_eq!(status, StatusCode::OK, "{written}");
    }
    rig.send(
        "/api/collections/life/moods/records",
        json!({ "fields": { "title": "Saturday", "went": "well" } }),
    )
    .await;
    std::path::PathBuf::from(project["path"].as_str().unwrap())
}

#[tokio::test]
async fn what_a_table_keeps_private_is_read_by_whoever_holds_what_it_is_kept_under_and_by_no_other_way() {
    let rig = Rig::new().await;
    let dir = life(&rig).await;
    let helper = key(&rig, kernel::principal::AGENT_SCOPES).await;
    let ask = |method: Method, path: &'static str, body: Option<Value>| {
        let (rig, helper) = (&rig, helper.clone());
        async move { rig.call(method, path, Some(&helper), body).await }
    };
    let refused = |status: StatusCode, body: &Value, what: &str| {
        assert_eq!(status, StatusCode::FORBIDDEN, "{what}: {body}");
        assert!(
            said(body).contains("kept private") && said(body).contains("`private:journal`"),
            "{what}: {body}"
        );
    };

    // The tables there are, to each: one that is kept from it is not there.
    let names = |listed: &Value| -> Vec<String> {
        let mut names: Vec<String> = listed["collections"]
            .as_array()
            .unwrap()
            .iter()
            .map(|one| one["name"].as_str().unwrap().into())
            .collect();
        names.sort();
        names
    };
    assert_eq!(names(&rig.read("/api/collections?project=life").await), ["days", "guides", "moods"]);
    let (_, listed) = ask(Method::GET, "/api/collections?project=life", None).await;
    assert_eq!(names(&listed), ["guides"]);

    // Its records, by any name: listed, read, changed, spoken on, added to, taken away.
    let (status, body) = ask(Method::GET, "/api/collections/life/days/records", None).await;
    refused(status, &body, "the days");
    let (status, body) = ask(Method::GET, "/api/collections/life/moods/records", None).await;
    refused(status, &body, "the moods");
    let (status, body) = ask(Method::GET, "/api/records/life-days:2026-11-14", None).await;
    refused(status, &body, "one day");
    let (status, body) = ask(Method::GET, "/api/records/life-moods-1", None).await;
    refused(status, &body, "one mood");
    let change = Some(json!({ "fields": { "went": "badly" } }));
    let (status, body) = ask(Method::PATCH, "/api/records/life-moods-1", change).await;
    refused(status, &body, "changing one");
    let (status, body) =
        ask(Method::POST, "/api/records/life-moods-1/comments", Some(json!({ "body": "and?" }))).await;
    refused(status, &body, "speaking on one");
    let added = Some(json!({ "fields": { "title": "Sunday" } }));
    let (status, body) = ask(Method::POST, "/api/collections/life/moods/records", added).await;
    refused(status, &body, "adding one");
    let (status, body) = ask(Method::DELETE, "/api/records/life-moods-1", None).await;
    refused(status, &body, "taking one away");
    let (status, body) = ask(Method::GET, "/api/records/life-moods-1/history", None).await;
    refused(status, &body, "its history");
    // What is not kept from it, it reads as it did.
    let (status, guides) = ask(Method::GET, "/api/collections/life/guides/records", None).await;
    assert_eq!((status, guides["records"].as_array().unwrap().len()), (StatusCode::OK, 1), "{guides}");

    // The files the days are kept in: not listed, not read, not written over, not taken away.
    let (_, tree) = ask(Method::GET, "/api/projects/life/tree", None).await;
    let paths: Vec<&str> =
        tree["entries"].as_array().unwrap().iter().map(|entry| entry["path"].as_str().unwrap()).collect();
    assert!(paths.contains(&"notes/guides/bread.md") && paths.contains(&"notes"), "{paths:?}");
    assert!(!paths.iter().any(|path| path.starts_with("notes/days")), "{paths:?}");
    let day = "/api/projects/life/file?path=notes/days/2026-11-14.md";
    let (status, body) = ask(Method::GET, day, None).await;
    refused(status, &body, "the file");
    let (status, body) = ask(Method::GET, "/api/projects/life/raw?path=notes/days/2026-11-14.md", None).await;
    refused(status, &body, "the file as it is");
    // Nor whether it is there, which is asked of a path somebody wrote before it is drawn as a link.
    let (status, body) =
        ask(Method::GET, "/api/projects/life/file?path=notes/days/2026-11-14.md&head", None).await;
    refused(status, &body, "whether the file is there");
    let over = Some(json!({ "path": "notes/days/2026-11-14.md", "content": "gone" }));
    let (status, body) = ask(Method::PUT, "/api/projects/life/file", over).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
    let (status, body) = ask(Method::DELETE, day, None).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
    // Nor the folder that holds them, taken away whole; nor by a link that leads there.
    let (status, body) = ask(Method::DELETE, "/api/projects/life/file?path=notes", None).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
    // (A link is made as Unix makes one: making one on Windows takes a
    // leave that not everybody there has.)
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(dir.join("notes/days"), dir.join("another-way")).unwrap();
        let (status, body) =
            ask(Method::GET, "/api/projects/life/file?path=another-way/2026-11-14.md", None).await;
        refused(status, &body, "the file by a link");
    }
    assert_eq!(
        std::fs::read_to_string(dir.join("notes/days/2026-11-14.md")).unwrap().lines().next(),
        Some("# Saturday")
    );
    // What is beside them is read as it was.
    let (status, bread) = ask(Method::GET, "/api/projects/life/file?path=notes/guides/bread.md", None).await;
    assert_eq!(status, StatusCode::OK, "{bread}");

    // As pages.
    let (status, _) = ask(Method::GET, "/library/life/days/", None).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, _) = ask(Method::GET, "/library/life/guides/", None).await;
    assert_eq!(status, StatusCode::OK);

    // Found by searching: the word is in both, and one of them is the helper's to find.
    rig.node().app.studio.search.catch_up().await.unwrap();
    let found = |hits: &Value| -> Vec<String> {
        let mut paths: Vec<String> = hits["hits"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|hit| hit["link"]["path"].as_str().map(str::to_string))
            .collect();
        paths.sort();
        paths
    };
    assert_eq!(
        found(&rig.read("/api/search?q=marmalade").await),
        ["notes/days/2026-11-14.md", "notes/guides/bread.md"]
    );
    let (_, hits) = ask(Method::GET, "/api/search?q=marmalade", None).await;
    assert_eq!(found(&hits), ["notes/guides/bread.md"]);

    // What is said of them as it happens: the owner speaks on a day and on a mood.
    rig.send("/api/records/life-days:2026-11-14/comments", json!({ "body": "the cupboard, not the stairs" }))
        .await;
    rig.send("/api/records/life-moods-1/comments", json!({ "body": "better by evening" })).await;
    rig.send("/api/records/life-guides:bread/comments", json!({ "body": "less salt" })).await;
    let heard = |events: &Value| -> Vec<String> {
        events["events"]
            .as_array()
            .unwrap()
            .iter()
            .rev()
            .map(|event| {
                format!(
                    "{} {}",
                    event["topic"].as_str().unwrap(),
                    event["data"]["id"].as_str().or(event["data"]["path"].as_str()).unwrap_or("-")
                )
            })
            .collect()
    };
    let topics = "/api/events/recent?topics=record.**,project.file.changed&limit=100";
    let owner = heard(&rig.read(topics).await);
    for mine in [
        "record.commented life-days:2026-11-14",
        "record.commented life-moods-1",
        "project.file.changed notes/days/2026-11-14.md",
        "record.created life-moods-1",
    ] {
        assert!(owner.iter().any(|one| one == mine), "the owner hears `{mine}`: {owner:?}");
    }
    let (_, events) = ask(Method::GET, topics, None).await;
    let helper_heard = heard(&events);
    assert!(helper_heard.iter().any(|one| one == "record.commented life-guides:bread"), "{helper_heard:?}");
    assert!(
        helper_heard.iter().any(|one| one == "project.file.changed notes/guides/bread.md"),
        "{helper_heard:?}"
    );
    assert!(
        !helper_heard.iter().any(|one| one.contains("days") || one.contains("moods")),
        "{helper_heard:?}"
    );

    // Of a record that is gone, its history is still the table's to keep.
    let (status, gone) = rig.call(Method::DELETE, "/api/records/life-moods-1", Some(&rig.token), None).await;
    assert_eq!(status, StatusCode::OK, "{gone}");
    let (status, body) = ask(Method::GET, "/api/records/life-moods-1/history", None).await;
    refused(status, &body, "the history of one that is gone");

    // Given what they are kept under, a key reads them as their owner does.
    let holds: Vec<&str> =
        kernel::principal::AGENT_SCOPES.iter().copied().chain(["private:journal"]).collect();
    let trusted = key(&rig, &holds).await;
    let (status, days) =
        rig.call(Method::GET, "/api/collections/life/days/records", Some(&trusted), None).await;
    assert_eq!((status, days["records"].as_array().unwrap().len()), (StatusCode::OK, 1), "{days}");
    let (status, file) = rig.call(Method::GET, day, Some(&trusted), None).await;
    assert!(status == StatusCode::OK && file["content"].as_str().unwrap().contains("marmalade"), "{file}");
    // And one kept under another word is not opened by this one.
    let other: Vec<&str> =
        kernel::principal::AGENT_SCOPES.iter().copied().chain(["private:letters"]).collect();
    let other = key(&rig, &other).await;
    let (status, _) = rig.call(Method::GET, "/api/collections/life/days/records", Some(&other), None).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn a_table_is_not_declared_out_from_under_what_keeps_it_private_and_a_step_reads_no_more_than_whoever_applied_it()
 {
    let rig = Rig::new().await;
    life(&rig).await;
    // A person of the factory who was not given the journal.
    let member = key(&rig, kernel::principal::MEMBER_SCOPES).await;
    let apply = |source: String| {
        let (rig, member) = (&rig, member.clone());
        async move {
            let asked = json!({ "source": source, "project": "life", "wait": true });
            let (status, answer) = rig.call(Method::POST, "/api/apply", Some(&member), Some(asked)).await;
            assert_eq!(status, StatusCode::OK, "{answer}");
            let finished = answer["finished"].clone();
            let why = finished["steps"][0]["error"]
                .as_str()
                .or(finished["error"].as_str())
                .unwrap_or_default()
                .to_string();
            (finished["status"].as_str().unwrap_or_default().to_string(), why)
        }
    };

    // Declared again with nothing said of who reads it, it would be open: refused.
    let open = "blueprint: open\npermissions: [projects]\nsteps:\n  days:\n    do: collection.ensure\n    with: { name: moods, fields: { title: { type: text } } }\n";
    let (status, why) = apply(open.to_string()).await;
    assert_eq!(status, "failed", "{why}");
    assert!(why.contains("private:journal"), "{why}");
    let (status, body) =
        rig.call(Method::GET, "/api/collections/life/moods/records", Some(&member), None).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");

    // Nor is one kept private under something whoever declares it does not hold.
    let mine = "blueprint: mine\npermissions: [projects]\nsteps:\n  letters:\n    do: collection.ensure\n    with: { name: letters, reads: private:letters, fields: { title: { type: text } } }\n";
    let (status, why) = apply(mine.to_string()).await;
    assert!(status == "failed" && why.contains("private:letters"), "{why}");

    // A step reads a record, or a file, as whoever applied it may.
    let reads = "blueprint: reads\npermissions: [records]\nsteps:\n  look:\n    do: record.list\n    with: { collection: moods }\n";
    let (status, why) = apply(reads.to_string()).await;
    assert!(status == "failed" && why.contains("kept private"), "{why}");
    let file = "blueprint: file\npermissions: [projects]\nsteps:\n  look:\n    do: fs.read\n    with: { path: notes/days/2026-11-14.md }\n";
    let (status, why) = apply(file.to_string()).await;
    assert!(status == "failed" && why.contains("kept private"), "{why}");

    // A scope nobody could hold, and a board, are refused where they are said.
    let wrong = "blueprint: wrong\npermissions: [projects]\nsteps:\n  it:\n    do: collection.ensure\n    with: { name: letters, reads: nobody, fields: { title: { type: text } } }\n";
    let asked = json!({ "source": wrong, "project": "life", "wait": true });
    let answer = rig.send("/api/apply", asked).await;
    let why = answer["finished"]["steps"][0]["error"].as_str().unwrap_or_default();
    assert!(why.contains("`nobody` is not something a credential can hold"), "{answer}");
    let board = "blueprint: board\npermissions: [projects]\nsteps:\n  it:\n    do: collection.ensure\n    with: { name: board, model: card, reads: private:journal, fields: { title: { type: text } } }\n";
    let answer = rig.send("/api/apply", json!({ "source": board, "project": "life", "wait": true })).await;
    let why = answer["finished"]["steps"][0]["error"].as_str().unwrap_or_default();
    assert!(why.contains("a board cannot be kept private"), "{answer}");
}

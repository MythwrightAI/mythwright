//! A project's fork of a blueprint that ships, over HTTP: what it was
//! forked from is kept as it is made and forgotten as it is taken away,
//! each said; its summary says what it was forked from, where it differs
//! and how the default has moved since, whatever the program ships by
//! then; one an earlier program made says only what its texts show; and
//! none of it is handed to whoever may not read the project's files.

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;

const OWN: &str = "/api/projects/shop/own";

/// A package whose blueprint says its version, with `more` among its steps.
fn package(version: u32, more: &str) -> String {
    format!(
        "extension: reach\ntitle: Reach\nblueprints:\n  round: |\n    blueprint: reach/round\n    version: {version}\n    \
         params:\n      tone: {{ type: string, default: plain }}\n    steps:\n      made: {{ do: noop }}\n{more}"
    )
}

/// A factory with the package's first version installed and a project.
async fn shop() -> Rig {
    let rig = Rig::new().await;
    rig.send("/api/extensions", json!({ "source": package(1, "") })).await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    rig
}

async fn key(rig: &Rig, scopes: &[&str]) -> String {
    let made = rig.send("/api/credentials", json!({ "name": "narrow", "scopes": scopes })).await;
    made["token"].as_str().unwrap().to_string()
}

/// Every line of a summary's part of this kind, those under others among them.
fn lines(summary: &Value, kind: &str) -> Vec<String> {
    fn all(lines: &Value, found: &mut Vec<String>) {
        for line in lines.as_array().into_iter().flatten() {
            found.push(line["text"].as_str().unwrap_or_default().to_string());
            all(&line["under"], found);
        }
    }
    let parts = summary["parts"].as_array().cloned().unwrap_or_default();
    let mut found = Vec::new();
    for part in parts.iter().filter(|part| part["kind"] == kind) {
        all(&part["lines"], &mut found);
    }
    found
}

/// The summary of `name` as it is found for the shop.
async fn summary(rig: &Rig, token: &str, name: &str) -> Value {
    let body = json!({ "name": name, "project": "shop" });
    let (status, answer) = rig.call(Method::POST, "/api/blueprints/summary", Some(token), Some(body)).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    answer["summary"].clone()
}

async fn write(rig: &Rig, path: &str, text: &str) {
    let body = json!({ "path": path, "content": text });
    let (status, answer) =
        rig.call(Method::PUT, "/api/projects/shop/file", Some(&rig.token), Some(body)).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
}

#[tokio::test]
async fn a_fork_keeps_what_it_was_made_from_to_the_letter_and_says_that_it_was_made_and_taken_away() {
    let rig = shop().await;
    let ships = rig.node().app.engine.library().shipped("reach/round", None).unwrap();
    let mark = rig.mark();
    let made = rig.send(OWN, json!({ "blueprint": "reach/round" })).await;
    let file = format!("@blueprints/{}", brand::blueprint_file("reach/round"));
    assert_eq!((&made["path"], &made["project"]), (&json!(file), &json!("shop")), "{made}");
    assert_eq!(made["origin"]["version"], "1", "{made}");
    assert!(made["origin"]["source"].is_null(), "the text is the default's to be read as: {made}");

    // Kept with the saying of it: which blueprint, which text to the letter, who and when.
    let said = rig.event("project.forked", |event| event.data["project"] == "shop").await;
    assert!(said.seq > mark);
    assert_eq!((&said.data["blueprint"], &said.data["version"]), (&json!("reach/round"), &json!("1")));
    assert_eq!(said.data["path"], json!(file));
    assert_eq!(said.data["revision"], json!(blueprint::revision(&ships)));
    let engine = &rig.node().app.engine;
    let origin = engine.forked_from("shop", "reach/round").await.expect("it was kept");
    assert_eq!(origin.source, ships, "to the letter");
    assert_eq!((origin.path.as_str(), origin.by_name.as_str()), (file.as_str(), "Ada"));
    assert_eq!(said.data["by"], json!(origin.by));
    // Where the lists say: a project's, in the journal that keeps what is run for it.
    crate::apart::held_to_the_lists(&rig.node().app.journal).await;

    // It is the default as it was forked, and says so wherever it is asked.
    let say = rig.send(OWN, json!({ "blueprint": "reach/round", "say": true })).await;
    assert_eq!(say["origin"]["by_name"], "Ada", "{say}");
    let nowhere = "Nowhere: it is the default as it was forked.";
    assert_eq!(lines(&json!({ "parts": say["said"] }), "differs"), [nowhere]);
    let fork = summary(&rig, &rig.token, "reach/round").await;
    assert!(lines(&fork, "is")[0].starts_with("Forked from `reach/round` version 1 by Ada on "), "{fork}");
    assert_eq!(lines(&fork, "differs"), [nowhere]);
    assert!(lines(&fork, "moved").is_empty() && lines(&fork, "back").len() == 1, "{fork}");

    // Taken away: said, and nothing is kept of it. One made there again is another fork.
    let mark = rig.mark();
    let back = rig.send(OWN, json!({ "blueprint": "reach/round", "back": true })).await;
    assert_eq!(back["removed"], json!(file));
    let gone = rig.event("project.unforked", |event| event.data["project"] == "shop").await;
    assert!(gone.seq > mark);
    assert_eq!(
        (&gone.data["blueprint"], &gone.data["version"], &gone.data["lost"]),
        (&json!("reach/round"), &json!("1"), &json!(false))
    );
    assert!(rig.node().app.engine.forked_from("shop", "reach/round").await.is_none());
    let journal = rig.node().app.journal.clone();
    for one in journal.every() {
        assert_eq!(one.kv_list(conductor::forked::NAMESPACE).await.unwrap(), [], "in `{}`", one.name());
    }
    // What ships is what is found there again, and it says how it is changed.
    let default = summary(&rig, &rig.token, "reach/round").await;
    assert!(!lines(&default, "is")[0].contains("fork"), "{default}");
    assert_eq!(lines(&default, "change").len(), 2, "what it may be told, and a fork: {default}");
}

#[tokio::test]
async fn words_a_blueprint_links_are_forked_under_their_own_name_where_they_are_read_and_said_as_words() {
    let rig = shop().await;
    let name = "std/prompts/lane-round.md";
    let ships = rig.node().app.engine.library().shipped(name, Some("shop")).unwrap();
    let made = rig.send(OWN, json!({ "blueprint": name })).await;
    // Under its own name, which is where a project's own words are read: never as a blueprint's file.
    let file = format!("@blueprints/{name}");
    assert_eq!(made["path"], json!(file), "{made}");
    let place = rig.placed("shop");
    assert_eq!(std::fs::read_to_string(place.join(name)).unwrap(), ships);
    assert!(!place.join(brand::blueprint_file(name)).exists());
    // Words say no version, whatever a line of them looks like, and are the same as what ships.
    assert!(made["version"].is_null() && made["origin"]["version"].is_null(), "{made}");
    assert_eq!(made["differs"]["same"], true, "{made}");
    assert_eq!(made["differs"]["changed"], json!([]), "{made}");
    let said = rig.event("project.forked", |event| event.data["blueprint"] == name).await;
    assert_eq!(said.data["path"], json!(file));

    // Changed: what runs in the project reads the project's words, and the fork says that they changed.
    let text = ships.replace("${why}", "${why} Be brief.\n\n${tone}");
    write(&rig, &file, &text).await;
    assert_eq!(rig.node().app.engine.library().load(name, Some("shop")).as_deref(), Some(text.as_str()));
    let say = rig.send(OWN, json!({ "blueprint": name, "say": true })).await;
    assert_eq!(say["path"], json!(file), "{say}");
    assert_eq!(say["differs"]["same"], false, "{say}");
    // The fork set against what ships, as its summary sets it against what it was forked from.
    assert_eq!(say["differs"]["changed"], json!(["its words changed", "the blank `${tone}` is new"]));
    assert!(say["differs"]["was"].is_null() && say["differs"]["now"].is_null(), "{say}");
    let parts = json!({ "parts": say["said"] });
    assert_eq!(lines(&parts, "differs"), ["Its words changed.", "The blank `${tone}` is new."], "{say}");
    assert!(lines(&parts, "moved").is_empty(), "what ships is what it was forked from: {say}");
    assert!(lines(&parts, "back")[0].contains("What the fork's text says is then gone"), "{say}");

    // A line laid out another way is other words, not the same text laid out.
    write(&rig, &file, &ships.replacen("\n\n", "\n", 1)).await;
    let say = rig.send(OWN, json!({ "blueprint": name, "say": true })).await;
    assert_eq!(say["differs"]["same"], false, "{say}");
    assert_eq!(lines(&json!({ "parts": say["said"] }), "differs"), ["Its words changed."], "{say}");

    // Given back as a blueprint's fork is: refused while it says other words, then taken away.
    let asked = json!({ "blueprint": name, "back": true });
    let (status, kept) = rig.call(Method::POST, OWN, Some(&rig.token), Some(asked)).await;
    assert_eq!(status, StatusCode::CONFLICT, "{kept}");
    let back = rig.send(OWN, json!({ "blueprint": name, "back": true, "force": true })).await;
    assert_eq!(back["removed"], json!(file), "{back}");
    assert!(!place.join(name).exists());
    assert!(rig.node().app.engine.forked_from("shop", name).await.is_none());
    assert_eq!(rig.node().app.engine.library().load(name, Some("shop")), Some(ships));
}

#[tokio::test]
async fn what_only_a_fork_has_is_said_to_be_new_where_it_is_said_against_what_ships_as_its_summary_says_it() {
    let rig = shop().await;
    // A step only the fork of a blueprint has.
    let ships = rig.node().app.engine.library().shipped("reach/round", None).unwrap();
    let made = rig.send(OWN, json!({ "blueprint": "reach/round" })).await;
    let typed = ships.replace("  made: { do: noop }", "  made: { do: noop }\n  more: { do: noop }");
    write(&rig, made["path"].as_str().unwrap(), &typed).await;
    let say = rig.send(OWN, json!({ "blueprint": "reach/round", "say": true })).await;
    assert_eq!((&say["differs"]["was"], &say["differs"]["now"]), (&json!("1"), &json!("1")), "{say}");
    assert_eq!(say["differs"]["changed"], json!(["the step `more` is new"]), "{say}");
    assert_eq!(lines(&json!({ "parts": say["said"] }), "differs"), ["The step `more` is new."], "{say}");
    let fork = summary(&rig, &rig.token, "reach/round").await;
    assert_eq!(lines(&fork, "differs"), ["The step `more` is new."], "{fork}");

    // A blank only the fork of words a blueprint links has.
    let name = "std/prompts/lane-round.md";
    let ships = rig.node().app.engine.library().shipped(name, Some("shop")).unwrap();
    let made = rig.send(OWN, json!({ "blueprint": name })).await;
    write(&rig, made["path"].as_str().unwrap(), &format!("{ships}\n${{tone}}\n")).await;
    let say = rig.send(OWN, json!({ "blueprint": name, "say": true })).await;
    assert_eq!(
        say["differs"]["changed"],
        json!(["its words changed", "the blank `${tone}` is new"]),
        "{say}"
    );
    let parts = json!({ "parts": say["said"] });
    assert_eq!(lines(&parts, "differs"), ["Its words changed.", "The blank `${tone}` is new."], "{say}");
}

#[tokio::test]
async fn a_fork_says_where_it_differs_and_how_the_default_has_moved_whatever_ships_by_then() {
    let mut rig = shop().await;
    let first = rig.node().app.engine.library().shipped("reach/round", None).unwrap();
    let made = rig.send(OWN, json!({ "blueprint": "reach/round" })).await;
    let file = made["path"].as_str().unwrap().to_string();
    // Changed by somebody, as any file of the project's is.
    let text = first.replace("default: plain", "default: warm");
    assert_ne!(text, first);
    write(&rig, &file, &text).await;
    let fork = summary(&rig, &rig.token, "reach/round").await;
    assert_eq!(lines(&fork, "differs"), ["The param `tone` is changed."]);
    assert!(lines(&fork, "moved").is_empty(), "the default is where it was: {fork}");
    assert!(lines(&fork, "back")[0].contains("What the fork's text says is then gone"), "{fork}");

    // The program brings another, and the daemon is started again.
    rig.send("/api/extensions", json!({ "source": package(2, "      tidy: { do: noop }\n") })).await;
    rig.restart().await;
    let later = rig.node().app.engine.library().shipped("reach/round", None).unwrap();
    assert_eq!(blueprint::version(&later).as_deref(), Some("2"));
    // What it was forked from is still that text, set beside the fork and beside what ships now.
    let origin = rig.node().app.engine.forked_from("shop", "reach/round").await.expect("it is still kept");
    assert_eq!(origin.source, first);
    let fork = summary(&rig, &rig.token, "reach/round").await;
    assert!(lines(&fork, "is")[0].starts_with("Forked from `reach/round` version 1 by Ada on "), "{fork}");
    assert_eq!(lines(&fork, "differs"), ["The param `tone` is changed."]);
    let moved = lines(&fork, "moved");
    assert_eq!(
        moved[..2],
        ["`reach/round` 1 to 2: what ships now is not what this was forked from.", "The step `tidy` is new."],
        "{moved:?}"
    );
    assert!(moved[2].starts_with("The fork takes none of it by itself."), "{moved:?}");
    // The terminal's words for the same are the same parts.
    let say = rig.send(OWN, json!({ "blueprint": "reach/round", "say": true })).await;
    assert_eq!(lines(&json!({ "parts": say["said"] }), "moved"), moved);
    // The fork set against what ships now, which has a step the fork has not.
    assert_eq!(say["differs"]["changed"], json!(["the param `tone` is changed", "the step `tidy` is gone"]));
    // And the default says that this project runs its own, where it is asked as what ships.
    let body = json!({ "source": later, "project": "shop", "ships": true });
    let default = rig.send("/api/blueprints/summary", body).await["summary"].clone();
    assert_eq!(
        lines(&default, "is")[0],
        format!(
            "This project runs its own fork of it, `{file}`: that file, and not this text, is what runs here \
             under the name."
        )
    );
    assert!(lines(&default, "change").last().unwrap().starts_with("Change this project's fork of it"));

    // Going back loses what the fork says, so it is refused until that is said, and then says so.
    let (status, kept) = rig
        .call(Method::POST, OWN, Some(&rig.token), Some(json!({ "blueprint": "reach/round", "back": true })))
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "{kept}");
    assert!(
        rig.node().app.engine.forked_from("shop", "reach/round").await.is_some(),
        "nothing was taken away"
    );
    rig.send(OWN, json!({ "blueprint": "reach/round", "back": true, "force": true })).await;
    let gone = rig.event("project.unforked", |event| event.data["project"] == "shop").await;
    assert_eq!(gone.data["lost"], true);
    assert!(rig.node().app.engine.forked_from("shop", "reach/round").await.is_none());
}

#[tokio::test]
async fn a_fork_from_before_says_only_what_its_texts_show_and_none_is_made_up_for_it() {
    let rig = shop().await;
    let first = rig.node().app.engine.library().shipped("reach/round", None).unwrap();
    // As an earlier program left one: the file, and nothing kept.
    let file = format!("@blueprints/{}", brand::blueprint_file("reach/round"));
    let text = first.replace("default: plain", "default: warm");
    write(&rig, &file, &text).await;
    assert!(rig.node().app.engine.forked_from("shop", "reach/round").await.is_none());
    // It calls itself what ships now calls itself: the version is its own text's, and said to be.
    let fork = summary(&rig, &rig.token, "reach/round").await;
    assert_eq!(
        lines(&fork, "is")[0],
        "Forked from `reach/round` version 1. That was before what a fork is made from was kept: by whom and \
         when is not known, and the version is the one its own text says."
    );
    assert_eq!(lines(&fork, "differs"), ["The param `tone` is changed."]);
    let say = rig.send(OWN, json!({ "blueprint": "reach/round", "say": true })).await;
    assert!(say["origin"].is_null(), "{say}");

    // The program brings another, and no text of the default calls itself what the fork does any more.
    rig.send("/api/extensions", json!({ "source": package(2, "      tidy: { do: noop }\n") })).await;
    let fork = summary(&rig, &rig.token, "reach/round").await;
    assert_eq!(
        lines(&fork, "is")[0],
        "Forked from `reach/round`. That was before what a fork is made from was kept: from which version, by \
         whom and when is not known."
    );
    let titles: Vec<&str> =
        fork["parts"].as_array().unwrap().iter().filter_map(|part| part["title"].as_str()).collect();
    assert!(titles.contains(&"Where it differs from the default as it ships now"), "{titles:?}");
    assert!(!titles.contains(&"The default has moved since"), "what it moved from is not known: {titles:?}");
    // Reading it wrote nothing down.
    assert!(rig.node().app.engine.forked_from("shop", "reach/round").await.is_none());
    for one in rig.node().app.journal.every() {
        assert_eq!(one.kv_list(conductor::forked::NAMESPACE).await.unwrap(), []);
    }
}

#[tokio::test]
async fn whoever_may_not_read_the_projects_files_is_handed_nothing_of_a_fork_or_what_it_was_made_from() {
    let rig = shop().await;
    let made = rig.send(OWN, json!({ "blueprint": "reach/round" })).await;
    let file = made["path"].as_str().unwrap().to_string();
    let text =
        std::fs::read_to_string(rig.placed("shop").join(file.trim_start_matches("@blueprints/"))).unwrap();
    let ships = rig.node().app.engine.library().shipped("reach/round", None).unwrap();
    // Somebody who may read blueprints and not projects is told what the texts say, as before, and no more.
    let reader = key(&rig, &["blueprints:read"]).await;
    let plainly =
        |summary: &Value| !summary.to_string().contains("\"opens\"") && lines(summary, "differs").is_empty();
    let by_name = summary(&rig, &reader, "reach/round").await;
    assert!(plainly(&by_name) && lines(&by_name, "back").is_empty(), "{by_name}");
    for said in [json!({ "path": file }), json!({ "ships": true })] {
        let mut body =
            json!({ "source": if said["ships"] == true { &ships } else { &text }, "project": "shop" });
        body.as_object_mut().unwrap().extend(said.as_object().unwrap().clone());
        for route in ["/api/blueprints/summary", "/api/blueprints/check"] {
            let (status, answer) = rig.call(Method::POST, route, Some(&reader), Some(body.clone())).await;
            assert_eq!(status, StatusCode::OK, "{answer}");
            let summary = &answer["summary"];
            assert!(!summary.to_string().contains("fork of it, `"), "{route} {said}: {summary}");
            assert!(lines(summary, "differs").is_empty() && lines(summary, "back").is_empty(), "{summary}");
            assert!(!lines(summary, "is")[0].starts_with("Forked from"), "{route} {said}: {summary}");
        }
    }
    // Nor does the route say or take anything for them, as before; and no event of it is theirs to hear.
    for asked in [json!({ "say": true }), json!({ "back": true }), json!({})] {
        let mut body = asked.clone();
        body["blueprint"] = json!("reach/round");
        let (status, refused) = rig.call(Method::POST, OWN, Some(&reader), Some(body)).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{asked}: {refused}");
    }
    rig.event("project.forked", |event| event.data["project"] == "shop").await;
    let recent = "/api/events/recent?topics=project.forked,project.unforked&limit=100";
    let (status, heard) = rig.call(Method::GET, recent, Some(&reader), None).await;
    assert_eq!(status, StatusCode::OK, "{heard}");
    assert_eq!(heard["events"], json!([]), "it is said of a file of the project's");
    // Whoever may read the project is told, by the same asking.
    let member = key(&rig, &["blueprints:read", "projects:read"]).await;
    let (_, heard) = rig.call(Method::GET, recent, Some(&member), None).await;
    assert_eq!(heard["events"].as_array().map(Vec::len), Some(1), "{heard}");
    let fork = summary(&rig, &member, "reach/round").await;
    assert!(lines(&fork, "is")[0].starts_with("Forked from `reach/round` version 1 by Ada"), "{fork}");
}

#[tokio::test]
async fn where_a_text_stands_is_said_only_where_whoever_asks_says_what_the_text_is() {
    let rig = shop().await;
    let ships = rig.node().app.engine.library().shipped("reach/round", None).unwrap();
    let checked = |body: Value| async {
        let (status, answer) =
            rig.call(Method::POST, "/api/blueprints/check", Some(&rig.token), Some(body)).await;
        assert_eq!(status, StatusCode::OK, "{answer}");
        answer["summary"].clone()
    };
    // A text and nothing said of it: what it says, and no more. So for something running, and a text being written.
    let alone = checked(json!({ "source": ships, "project": "shop" })).await;
    assert!(lines(&alone, "change").is_empty() && !alone.to_string().contains("\"opens\""), "{alone}");
    // What ships, for no project and for one: how it is changed, the last of it a fork.
    for project in [None, Some("shop")] {
        let default = checked(json!({ "source": ships, "project": project, "ships": true })).await;
        let change = lines(&default, "change");
        assert!(change[0].starts_with("Tell it something else"), "{change:?}");
        assert!(change.last().unwrap().starts_with("Fork it:"), "{change:?}");
    }
    // A name that does not ship has no default to stand beside, whatever is said of it.
    let own = "blueprint: shop/own\nsteps:\n  one: { do: noop }\n";
    let said = checked(json!({ "source": own, "project": "shop", "ships": true })).await;
    assert!(lines(&said, "change").is_empty(), "{said}");

    // Forked: the file says what it was forked from as it is written in, before it is saved.
    let made = rig.send(OWN, json!({ "blueprint": "reach/round" })).await;
    let file = made["path"].as_str().unwrap();
    let typed = ships.replace("  made: { do: noop }", "  made: { do: noop }\n  more: { do: noop }");
    assert_ne!(typed, ships);
    let fork = checked(json!({ "source": typed, "project": "shop", "path": file })).await;
    assert!(lines(&fork, "is")[0].starts_with("Forked from `reach/round` version 1 by Ada"), "{fork}");
    assert_eq!(lines(&fork, "differs"), ["The step `more` is new."]);
    let line = &fork["parts"][1]["lines"][0];
    assert_eq!(line["at"], "main/more", "it is shown where it is written: {line}");
    // Another file of the project that says the same name is not the fork.
    let beside = checked(json!({ "source": typed, "project": "shop", "path": "notes/round.bp" })).await;
    assert!(lines(&beside, "differs").is_empty() && !beside.to_string().contains("\"opens\""), "{beside}");

    // In a project that is not trusted a file of its repository does not stand in for what ships, and
    // is not said to be what runs; a fork is made all the same, where the factory keeps them.
    let folder = rig.dir.join("taken-up");
    std::fs::create_dir_all(folder.join("blueprints/reach")).unwrap();
    std::fs::write(folder.join("blueprints").join(brand::blueprint_file("reach/round")), &typed).unwrap();
    rig.send("/api/projects", json!({ "name": "Yard", "path": folder })).await;
    let default = checked(json!({ "source": ships, "project": "yard", "ships": true })).await;
    assert!(!lines(&default, "is")[0].contains("runs its own fork"), "{default}");
    let (status, refused) = rig
        .call(
            Method::POST,
            "/api/projects/yard/own",
            Some(&rig.token),
            Some(json!({ "blueprint": "reach/round" })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{refused}");
    assert_eq!(rig.node().app.engine.forked_from("yard", "reach/round").await.unwrap().path, file);
    let default = checked(json!({ "source": ships, "project": "yard", "ships": true })).await;
    assert!(lines(&default, "is")[0].contains("runs its own fork"), "{default}");
}

#[tokio::test]
async fn a_fork_taken_away_as_the_daemon_stopped_leaves_no_record_that_stands() {
    let mut rig = shop().await;
    let made = rig.send(OWN, json!({ "blueprint": "reach/round" })).await;
    let file = made["path"].as_str().unwrap().to_string();
    let on_disk = rig.placed("shop").join(file.trim_start_matches("@blueprints/"));
    // Stopped after it was marked as leaving and before its file went: the file is there,
    // and what is said of it is what is said of one nothing was kept for.
    let engine = rig.node().app.engine.clone();
    assert!(engine.fork_leaving("shop", "reach/round", true).await.is_some());
    rig.restart().await;
    assert!(on_disk.is_file());
    assert!(rig.node().app.engine.forked_from("shop", "reach/round").await.is_none());
    let fork = summary(&rig, &rig.token, "reach/round").await;
    assert!(lines(&fork, "is")[0].contains("before what a fork is made from was kept"), "{fork}");
    // Stopped after its file went and before that was said: a file somebody puts there later is no
    // fork of that making, and asking for it to be taken away again ends what was begun.
    std::fs::remove_file(&on_disk).unwrap();
    std::fs::write(&on_disk, "blueprint: reach/round\nsteps:\n  theirs: { do: noop }\n").unwrap();
    let fork = summary(&rig, &rig.token, "reach/round").await;
    assert!(lines(&fork, "is")[0].starts_with("Forked from `reach/round`. That was before"), "{fork}");
    std::fs::remove_file(&on_disk).unwrap();
    let mark = rig.mark();
    let (status, none) = rig
        .call(Method::POST, OWN, Some(&rig.token), Some(json!({ "blueprint": "reach/round", "back": true })))
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "there is no file to take away: {none}");
    let gone = rig.event("project.unforked", |event| event.data["project"] == "shop").await;
    assert!(gone.seq > mark);
    for one in rig.node().app.journal.every() {
        assert_eq!(one.kv_list(conductor::forked::NAMESPACE).await.unwrap(), [], "in `{}`", one.name());
    }
    // A file that could not be taken away leaves what was kept standing.
    rig.send(OWN, json!({ "blueprint": "reach/round" })).await;
    let engine = rig.node().app.engine.clone();
    engine.fork_leaving("shop", "reach/round", true).await;
    assert!(engine.forked_from("shop", "reach/round").await.is_none());
    engine.fork_leaving("shop", "reach/round", false).await;
    assert!(engine.forked_from("shop", "reach/round").await.is_some());
}

#[tokio::test]
async fn what_a_fork_was_made_from_goes_into_a_copy_of_the_factory_with_the_fork() {
    let shared = crate::snapshots::Shared::new();
    let prod = shared.factory("prod").await;
    prod.send("/api/extensions", json!({ "source": package(1, "") })).await;
    prod.send("/api/projects", json!({ "name": "Shop" })).await;
    let ships = prod.node().app.engine.library().shipped("reach/round", None).unwrap();
    let made = prod.send(OWN, json!({ "blueprint": "reach/round" })).await;
    let file = made["path"].as_str().unwrap().to_string();
    let text = ships.replace("default: plain", "default: warm");
    write(&prod, &file, &text).await;
    let to = shared.snapshot("first");
    crate::snapshots::taken(&prod, &to).await;

    let copy = shared.copy("staging", &to).await;
    let origin =
        copy.node().app.engine.forked_from("shop", "reach/round").await.expect("it came with the copy");
    assert_eq!(origin.source, ships, "to the letter");
    assert_eq!(origin.by_name, "Ada");
    // The copy has the project's files, and the fork says there what it said where it was copied from.
    assert!(copy.node().app.engine.library().is_own("reach/round", Some("shop")));
    let body = json!({ "source": text, "project": "shop", "path": file });
    let fork = copy.send("/api/blueprints/summary", body).await["summary"].clone();
    assert!(lines(&fork, "is")[0].starts_with("Forked from `reach/round` version 1 by Ada"), "{fork}");
    assert_eq!(lines(&fork, "differs"), ["The param `tone` is changed."]);
    crate::apart::held_to_the_lists(&copy.node().app.journal).await;
}

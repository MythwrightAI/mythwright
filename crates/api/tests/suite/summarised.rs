//! A blueprint said in plain words, for everything that ships and by the
//! routes that hand a summary out.
//!
//! The function is the language's (`blueprint::summary`), and its cases
//! are in that crate's suite. What is held here is what only a whole
//! program can say: that every blueprint that ships, with this program's
//! actions and events and what its packages bring, comes to a summary
//! somebody could read, and that a summary is handed to whoever may read
//! the text it is of and to nobody else.

use std::path::{Path, PathBuf};

use blueprint::summary::forked::Standing;
use blueprint::summary::{Known, Line, summary, summary_as};
use blueprint::{ActionSpec, EventSpec};
use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;

fn top() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap()
}

fn blueprints_under(dir: &Path, found: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap().flatten() {
        let path = entry.path();
        if path.is_dir() {
            blueprints_under(&path, found);
        } else if brand::is_blueprint_path(&path.to_string_lossy()) {
            found.push(path);
        }
    }
}

/// Every blueprint that ships, by where it is: the standard ones, the
/// examples, and what each package brings; with the actions the packages
/// bring, whether or not this factory has them installed.
fn shipped() -> (Vec<(String, String)>, Vec<ActionSpec>) {
    let (packages, _) = agents::extension::load_all(&top().join("extensions"));
    let (mut texts, mut brought) = (Vec::new(), Vec::new());
    for (name, package) in &packages {
        brought.extend(package.actions.values().map(|action| action.spec.clone()));
        for (called, text) in &package.blueprints {
            texts.push((format!("extensions/{name}: {called}"), text.clone()));
        }
    }
    let mut files = Vec::new();
    blueprints_under(&top().join("blueprints"), &mut files);
    blueprints_under(&top().join("examples"), &mut files);
    for file in files {
        texts.push((file.display().to_string(), std::fs::read_to_string(&file).unwrap()));
    }
    (texts, brought)
}

fn every<'a>(lines: &'a [Line], found: &mut Vec<&'a Line>) {
    for line in lines {
        found.push(line);
        every(&line.under, found);
    }
}

/// A line without what it sets apart as written (`a name`), which may be
/// anything: what is left is the line's own words.
fn own_words(text: &str) -> String {
    text.split('`').step_by(2).collect::<Vec<_>>().join(" ")
}

/// On one line, as a summary says whatever a text says of itself.
fn on_a_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// What a text says of itself in its own words: of the whole, of each
/// param and of each step. A summary says these as they are written, and
/// what somebody wrote is theirs to have written as they liked.
fn authored(read: &blueprint::Blueprint) -> Vec<String> {
    fn of_steps(flow: &blueprint::Flow, found: &mut Vec<String>) {
        for step in &flow.steps {
            found.extend(step.description.as_deref().map(blueprint::summary::says));
            of_steps(&step.inner, found);
        }
    }
    let mut found: Vec<String> = read.description.iter().map(|said| on_a_line(said)).collect();
    found.extend(read.params.iter().filter_map(|param| param.description.as_deref()).map(on_a_line));
    of_steps(&read.flow, &mut found);
    for trigger in &read.triggers {
        of_steps(&trigger.flow, &mut found);
    }
    found.retain(|said| !said.is_empty());
    // The longest first, so that one which begins another is not taken out of it.
    found.sort_by_key(|said| std::cmp::Reverse(said.len()));
    found
}

/// What is wrong with a line somebody is to read, if anything is.
/// `authored` is what the text says in its own words, which is left out
/// of what is held to being no value as a text writes one.
fn unreadable(line: &Line, actions: &[ActionSpec], authored: &[String]) -> Option<String> {
    let words = own_words(&line.text);
    if line.text.trim().is_empty() {
        return Some("it is empty".into());
    }
    if line.text.contains("${") {
        return Some("it holds a hole nobody filled".into());
    }
    let ours = authored.iter().fold(line.text.clone(), |text, said| text.replace(said.as_str(), ""));
    if ["{\"", "\":", "[{", "}]", "null", "\\n"].iter().any(|raw| ours.contains(raw)) || ours.contains('\n') {
        return Some("it holds a value as the text writes it".into());
    }
    if line.text.contains("an action this was read without") || words.contains("worked out when it runs") {
        return Some("it says something it could not say".into());
    }
    // An action's name where its sentence should be: a dotted name among
    // the line's own words. A name of one word is a word of the language.
    let named = |spec: &&ActionSpec| {
        spec.name.contains('.')
            && words
                .split(|letter: char| !(letter.is_alphanumeric() || ".-_".contains(letter)))
                .any(|word| word.trim_matches('.') == spec.name)
    };
    actions
        .iter()
        .find(named)
        .map(|spec| format!("it names the action {} where a sentence should be", spec.name))
}

#[tokio::test]
async fn every_blueprint_that_ships_is_said_in_words_with_no_blank_no_id_and_nothing_as_its_text_writes_it() {
    let rig = Rig::new().await;
    let registry = rig.node().app.engine.registry().clone();
    let (texts, brought) = shipped();
    assert!(texts.len() >= 50, "the blueprints that ship were found: {}", texts.len());
    let mut actions = registry.all_specs();
    for spec in brought {
        if !actions.iter().any(|known| known.name == spec.name) {
            actions.push(spec);
        }
    }
    let events = registry.events();
    let events: &[EventSpec] = events.specs();
    let known = Known { actions: &actions, events, scopes: kernel::principal::SCOPES };

    let mut wrong = Vec::new();
    let (mut lines, mut makes, mut commands, mut secrets, mut tables) = (0, 0, 0, 0, 0);
    for (of, text) in &texts {
        // Each is read once. Whether it checks against this device is
        // another test's (`walkthrough.rs`).
        let read =
            blueprint::check(text, &blueprint::NoCatalog).blueprint.unwrap_or_else(|| panic!("{of} reads"));
        // As what it is: one that ships, which says last how what it does is changed.
        let ways = blueprint::summary::forked::Ways::default();
        let ships = Standing::Ships { own: None, part_of: &[], ways: &ways };
        let said = summary_as(&read, &known, Some(&ships));
        let authored = authored(&read);
        let change = said.parts.last().filter(|part| part.kind == "change").map(|part| part.lines.as_slice());
        let change = change.unwrap_or_else(|| panic!("{of}: says last how what it does is changed"));
        let leads = |line: &Line| line.opens.as_ref().map(|opens| opens.what.clone());
        assert_eq!(
            change.last().and_then(leads).as_deref(),
            Some("forking"),
            "{of}: the last of them is a fork"
        );
        // The tables whose rows are offered are those it makes whose rows set what something
        // does, each by its name, and no other: what the action says of itself (`a table`, and
        // that one which sits beside another is one), never a list of tables. A board's, a
        // library's, the features' and a package's own are what they hold, and are not offered.
        let rows: Vec<&str> = change
            .iter()
            .flat_map(|line| line.under.iter())
            .filter_map(|line| line.opens.as_ref().filter(|opens| opens.what == "table")?.name.as_deref())
            .collect();
        // By the name it is known by: `of` is where its file is.
        match read.name.as_str() {
            "std/fleet" => assert_eq!(rows, ["agents", "stages", "segments", "actions"], "{of}"),
            "std/told" => assert_eq!(rows, ["tells"], "{of}"),
            "std/chain" => assert_eq!(rows, ["chain"], "{of}"),
            // A card's A/B tests: which contenders work it and how they are judged.
            "std/ab" => assert_eq!(rows, ["contests"], "{of}"),
            _ => assert!(rows.is_empty(), "{of} offers to change the rows of {rows:?}"),
        }
        tables += rows.len();
        assert_eq!(said.name, read.name);
        assert!(said.parts.iter().any(|part| part.kind == "is"), "{of}: says what it is");
        assert!(said.parts.iter().any(|part| part.kind == "may"), "{of}: says what it may do");
        for part in &said.parts {
            if part.lines.is_empty() {
                wrong.push(format!("{of}: `{}` is a heading with nothing under it", part.title));
            }
            let mut all = Vec::new();
            every(&part.lines, &mut all);
            lines += all.len();
            makes += usize::from(part.kind == "makes") * all.len();
            commands += all.iter().filter(|line| line.mark.as_deref() == Some("command")).count();
            secrets += all.iter().filter(|line| line.mark.as_deref() == Some("secret")).count();
            for line in all {
                // What a page lays out in place of its words reads as its words do, each by itself.
                let pieces = line.title.iter().chain(&line.label).chain(&line.items).chain(&line.from);
                let pieces = pieces.chain(&line.only).chain(&line.tags).chain(&line.named).chain(&line.by);
                for said in std::iter::once(line.clone())
                    .chain(pieces.map(|piece| Line { text: piece.clone(), ..Line::default() }))
                {
                    if let Some(why) = unreadable(&said, &actions, &authored) {
                        wrong.push(format!("{of}, under `{}`: {why}:\n    {}", part.title, said.text));
                    }
                }
            }
        }
        // Said again from the same text, it is the same: nothing in it goes by an order nobody wrote.
        assert_eq!(summary_as(&read, &known, Some(&ships)), said, "{of}");
        // And but for that last part it is what the text alone comes to.
        let alone = summary(&read, &known);
        assert_eq!(said.parts[..said.parts.len() - 1], alone.parts[..], "{of}");
    }
    assert_eq!(tables, 7, "the tables whose rows are offered were found");
    assert!(wrong.is_empty(), "{} lines of what ships do not read:\n{}", wrong.len(), wrong.join("\n"));
    // And it found what there is to find: things made, commands run, secrets reached.
    assert!(lines > 1500, "the summaries have lines: {lines}");
    assert!(
        makes > 60 && commands >= 5 && secrets >= 2,
        "{makes} made, {commands} commands, {secrets} secrets, {lines} lines"
    );
}

/// What the test above goes by is not so loose that anything passes.
#[test]
fn a_line_that_holds_a_hole_an_actions_name_or_a_value_as_written_is_told_from_one_that_reads() {
    let actions = vec![ActionSpec::new("thing.make", "Make a thing."), ActionSpec::new("emit", "Say.")];
    let line = |text: &str| Line { text: text.to_string(), ..Line::default() };
    for reads in [
        "Make a thing.",
        "Say `thing.make` to whoever asks.",
        "A table called `board`.",
        "Emit nothing; the thing. Make it so.",
    ] {
        assert_eq!(unreadable(&line(reads), &actions, &[]), None, "{reads}");
    }
    // What the text says of itself is its author's, written as they liked.
    let theirs = "The variables, such as {\"api\": \"x\"}.".to_string();
    assert_eq!(
        unreadable(&line(&format!("{theirs} Unless it is said: nothing.")), &actions, &[theirs]),
        None
    );
    for (bad, why) in [
        ("A table called ${params.table}.", "hole"),
        ("Does thing.make.", "names the action"),
        ("Unless it is said: `{\"a\": 1}`.", "as the text writes it"),
        ("Unless it is said: `two\nlines`.", "as the text writes it"),
        ("  ", "empty"),
    ] {
        let found = unreadable(&line(bad), &actions, &[]).unwrap_or_else(|| panic!("{bad} does not read"));
        assert!(found.contains(why), "{bad}: {found}");
    }
}

/// A text of somebody's own that says each kind of thing a summary has a
/// part for.
const WATCH: &str = "\
blueprint: watch
description: Keep a table of what was seen, and say so.
permissions: [projects, channels, shell]
params:
  table: { type: string, default: seen, description: What the table is called. }
steps:
  seen:
    do: collection.ensure
    with: { name: \"${params.table}\", fields: { title: { type: text } } }
  look:
    description: Look at what changed. It reads and writes nothing.
    do: shell
    with: { cmd: [git, status] }
on:
  moved:
    event: board.item.moved
    do: channel.post
    with: { channel: general, body: moved }
";

fn part<'a>(summary: &'a Value, kind: &str) -> &'a Value {
    let parts = summary["parts"].as_array().expect("a summary has parts");
    parts.iter().find(|part| part["kind"] == kind).unwrap_or_else(|| panic!("no part `{kind}` in {summary}"))
}

fn texts(part: &Value) -> Vec<&str> {
    part["lines"].as_array().unwrap().iter().filter_map(|line| line["text"].as_str()).collect()
}

#[tokio::test]
async fn a_summary_is_handed_out_for_a_text_that_is_sent_and_for_a_blueprint_by_its_name() {
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;

    // A text that is sent, as the editor's page sends what is being written.
    let sent = rig.send("/api/blueprints/summary", json!({ "source": WATCH, "project": "shop" })).await;
    assert_eq!((sent["ok"].as_bool(), sent["name"].as_str()), (Some(true), Some("watch")), "{sent}");
    let summary = &sent["summary"];
    let kinds: Vec<&str> =
        summary["parts"].as_array().unwrap().iter().filter_map(|part| part["kind"].as_str()).collect();
    assert_eq!(
        kinds,
        ["is", "makes", "does", "hears", "may"],
        "each part that has something to say, in order"
    );
    assert_eq!(texts(part(summary, "makes")), ["A table called what the param `table` says."]);
    // A line says what it is about as the graph names it, and what the text calls that.
    let made = &part(summary, "makes")["lines"][0];
    assert_eq!((made["at"].as_str(), made["key"].as_str()), (Some("main/seen"), Some("seen")));
    assert_eq!(
        texts(part(summary, "does")),
        [
            "Declare a collection of records for a project: what they are made of, how they are looked at, and \
             where they are kept.",
            "Look at what changed.",
        ],
        "a step by its action's own sentence, and one that says what it is for by that, as far as its first \
         full stop"
    );
    let heard = &part(summary, "hears")["lines"][0];
    assert!(
        heard["text"].as_str().unwrap().starts_with("When this happens: a card was moved to another column"),
        "{heard}"
    );
    assert_eq!(heard["at"], "on.moved/@");
    assert_eq!(texts(&json!({ "lines": heard["under"] })), ["Post a message to a channel."]);
    // What somebody deciding to apply it most needs to see is marked.
    let may = part(summary, "may")["lines"].as_array().unwrap();
    let command = may.iter().find(|line| line["mark"] == "command").expect("it says a command is run");
    assert_eq!(command["text"], "It runs a command: `git status`.");
    assert_eq!(command["at"], "main/look");
    let scopes: Vec<&str> = may.iter().filter_map(|line| line["key"].as_str()).collect();
    assert_eq!(scopes, ["projects:write", "channels:write", "shell", "look"]);

    // The check's answer, which the page draws its graph from, carries the
    // same summary of the same text: the two are never of different texts.
    let checked = rig.send("/api/blueprints/check", json!({ "source": WATCH, "project": "shop" })).await;
    assert_eq!(&checked["summary"], summary);
    assert!(checked["graph"]["nodes"].as_array().unwrap().iter().any(|node| node["id"] == "main/seen"));

    // One that ships, by its name, with nothing applied anywhere.
    let board = rig.send("/api/blueprints/summary", json!({ "name": "std/board" })).await;
    assert_eq!(board["ok"], true, "{board}");
    assert_eq!(texts(part(&board["summary"], "makes"))[0], "A table called `board`.");
    assert!(rig.read("/api/instances").await["instances"].as_array().unwrap().is_empty());

    // A project's own, by the name it has there: found as an apply finds it.
    let own = json!({ "path": format!("@blueprints/{}", brand::blueprint_file("watch")), "content": WATCH });
    let (status, answer) =
        rig.call(Method::PUT, "/api/projects/shop/file", Some(&rig.token), Some(own)).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    let named = rig.send("/api/blueprints/summary", json!({ "name": "watch", "project": "shop" })).await;
    assert_eq!(&named["summary"], summary);
    let (status, answer) = rig.post("/api/blueprints/summary", json!({ "name": "watch" })).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "it is the project's, and no other's: {answer}");
    assert!(
        answer["error"]["message"].as_str().unwrap().contains("there is no blueprint called `watch`"),
        "{answer}"
    );
    let (status, answer) = rig.post("/api/blueprints/summary", json!({ "project": "shop" })).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{answer}");
    assert!(answer["error"]["message"].as_str().unwrap().contains("give `source`"), "{answer}");
}

#[tokio::test]
async fn a_text_that_does_not_check_has_no_summary_and_is_answered_with_what_is_wrong_with_it() {
    let rig = Rig::new().await;
    let wrong = WATCH.replace("do: shell", "do: shel");
    for path in ["/api/blueprints/summary", "/api/blueprints/check"] {
        let answer = rig.send(path, json!({ "source": wrong })).await;
        assert_eq!(answer["ok"], false, "{path}: {answer}");
        assert_eq!(answer["summary"], Value::Null, "{path}: {answer}");
        let said: Vec<&str> = answer["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|one| one["message"].as_str())
            .collect();
        assert!(said.iter().any(|one| one.contains("there is no action called `shel`")), "{path}: {said:?}");
    }
    // One that does not read at all is the same, in the reader's words.
    let answer = rig.send("/api/blueprints/summary", json!({ "source": "blueprint: x\nsteps: [\n" })).await;
    assert_eq!((answer["ok"].as_bool(), &answer["summary"]), (Some(false), &Value::Null), "{answer}");
    assert_eq!(answer["diagnostics"][0]["code"], "syntax");
    // A warning keeps no summary from anybody: a text that lists no permissions checks.
    let warned = WATCH.replace("permissions: [projects, channels, shell]\n", "");
    let answer = rig.send("/api/blueprints/summary", json!({ "source": warned })).await;
    assert_eq!(answer["ok"], true, "{answer}");
    assert!(answer["summary"]["parts"].is_array() && !answer["diagnostics"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn whoever_may_not_read_blueprints_is_handed_no_summary_of_a_text_or_of_a_name() {
    let rig = Rig::new().await;
    let narrow =
        rig.send("/api/credentials", json!({ "name": "narrow", "scopes": ["channels", "board"] })).await;
    let token = narrow["token"].as_str().unwrap();
    for body in [
        json!({ "name": "std/board" }),
        json!({ "source": WATCH }),
        // Refused before anything is said of the text, or of whether there is such a blueprint.
        json!({ "name": "no/such" }),
        json!({ "source": "not a blueprint" }),
        json!({}),
    ] {
        let (status, answer) =
            rig.call(Method::POST, "/api/blueprints/summary", Some(token), Some(body.clone())).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{body}: {answer}");
        let said = answer.to_string();
        assert!(said.contains("blueprints:read"), "{said}");
        assert!(
            !said.contains("parts") && !said.contains("table") && !said.contains("no blueprint"),
            "{said}"
        );
    }
    let (status, _) =
        rig.call(Method::POST, "/api/blueprints/summary", None, Some(json!({ "name": "std/board" }))).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "and nobody who is not signed in");
    // Whoever holds reading alone is handed it: it says nothing the text does not.
    let reader =
        rig.send("/api/credentials", json!({ "name": "reader", "scopes": ["blueprints:read"] })).await;
    let (status, answer) = rig
        .call(
            Method::POST,
            "/api/blueprints/summary",
            reader["token"].as_str(),
            Some(json!({ "name": "std/board" })),
        )
        .await;
    assert_eq!((status, answer["ok"].as_bool()), (StatusCode::OK, Some(true)), "{answer}");
}

/// What is running, and what has run, as the daemon lists them.
async fn standing(rig: &Rig) -> (Value, Value) {
    (rig.read("/api/instances").await, rig.read("/api/runs?limit=50").await)
}

#[tokio::test]
async fn a_summary_only_reads_and_nothing_of_a_blueprint_that_is_run_changes_for_it() {
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    // Something that runs and is over, so that what is said after it can
    // only be of what is asked next.
    let once = "blueprint: once\nsteps:\n  hello: { do: log, with: { message: hi } }\n";
    let applied = rig.send("/api/apply", json!({ "source": once, "project": "shop", "wait": true })).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
    // An apply answers what it always did: no summary is made for one.
    assert!(applied.get("summary").is_none(), "{applied}");
    let id = applied["instance"].as_str().expect("the instance it started").to_string();
    rig.event("instance.finished", |event| event.source.instance.as_deref() == Some(id.as_str())).await;

    let before = standing(&rig).await;
    let mark = rig.mark();
    for _ in 0..3 {
        let said = rig.send("/api/blueprints/summary", json!({ "source": once, "project": "shop" })).await;
        assert_eq!(said["ok"], true, "{said}");
        let said =
            rig.send("/api/blueprints/summary", json!({ "name": "std/board", "project": "shop" })).await;
        assert_eq!(said["ok"], true, "{said}");
    }
    // Nothing was said, and nothing that is running or has run is another
    // thing than it was: no instance, no run, no text taken.
    let after = journal::EventQuery { after: mark, limit: 1000, ..Default::default() };
    let said: Vec<String> = rig.said(after).await.into_iter().map(|event| event.topic.clone()).collect();
    assert_eq!(said, Vec::<String>::new());
    assert_eq!(standing(&rig).await, before);
}

/// Every line of a summary's part of this kind, with what it opens, those under others among them.
fn change_lines(summary: &Value) -> Vec<Line> {
    let parts: Vec<blueprint::summary::Part> =
        serde_json::from_value(summary["parts"].clone()).expect("a summary's parts");
    let mut found = Vec::new();
    for part in parts.iter().filter(|part| part.kind == "change") {
        let mut all = Vec::new();
        every(&part.lines, &mut all);
        found.extend(all.into_iter().cloned());
    }
    found
}

#[tokio::test]
async fn every_standard_blueprint_asked_for_a_project_says_where_what_it_does_is_changed_in_lines_that_read()
{
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    let library = rig.node().app.engine.library().clone();
    let actions = rig.node().app.engine.registry().all_specs();
    let names: Vec<String> = library
        .names(None)
        .into_iter()
        .filter(|name| name.starts_with("std/") && !conductor::library::is_linked(name))
        .collect();
    assert!(names.len() >= 40, "the standard blueprints were found: {}", names.len());
    let mut wrong = Vec::new();
    let mut said = std::collections::BTreeMap::new();
    for name in &names {
        let answer = rig.send("/api/blueprints/summary", json!({ "name": name, "project": "shop" })).await;
        let lines = change_lines(&answer["summary"]);
        assert!(!lines.is_empty(), "{name}: says how what it does is changed: {answer}");
        let text = library.load(name, None).unwrap();
        let read = blueprint::check(&text, &blueprint::NoCatalog).blueprint.unwrap();
        let authored = authored(&read);
        for line in &lines {
            if let Some(why) = unreadable(line, &actions, &authored) {
                wrong.push(format!("{name}: {why}:\n    {}", line.text));
            }
        }
        said.insert(name.clone(), lines);
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
    let opens = |line: &Line| {
        line.opens.as_ref().map(|opens| (opens.what.clone(), opens.name.clone().unwrap_or_default()))
    };
    let segments = Some(("table".to_string(), "segments".to_string()));

    // One of the usual things a round goes through: a row of `segments` first, with the command that adds one.
    for (name, at) in [
        ("std/work", "at 10 in what `std/lane-round` goes through"),
        ("std/keep", "at 20"),
        ("std/send", "at 30"),
        ("std/merge", "at 20 in what `std/reviewing` goes through"),
    ] {
        let first = &said[name][0];
        assert!(
            first.text.starts_with(
                "Lay something before it, in its place or after it by a row of the project's `segments` table"
            ),
            "{name}: {}",
            first.text
        );
        assert!(first.text.contains(at), "{name}: {}", first.text);
        assert!(
            first.text.contains(&format!("`{} record add shop segments title=", brand::CLI)),
            "{name}: {}",
            first.text
        );
        assert_eq!(opens(first), segments, "{name}");
    }
    // What goes through the list says what it does as it ships and the same row.
    let round = &said["std/lane-round"][0];
    assert!(round.text.starts_with("Change what it goes through by a row of the project's `segments` table: as it ships it does `std/work` at 10, `std/keep` at 20 and `std/send` at 30"), "{}", round.text);
    // A rule says how a project writes its own, and is not said to be applied with something else.
    for rule in ["std/dispatch", "std/signin"] {
        let texts: Vec<&str> = said[rule].iter().map(|line| line.text.as_str()).collect();
        assert!(texts[0].starts_with(&format!("Write the project's own: it is asked and never run, so a file of the project's own under its name, `@blueprints/{}`", brand::blueprint_file(rule))), "{rule}: {texts:?}");
        assert!(!texts.iter().any(|text| text.starts_with("Tell it something else")), "{rule}: {texts:?}");
    }
    // A text that links files says each, with the project's own file read in its place.
    assert!(
        said["std/review"].iter().any(|line| line.text
            == "`@blueprints/std/prompts/review/asks.md`, in place of `std/prompts/review/asks.md`"),
        "{:?}",
        said["std/review"]
    );
    // The fleet says its three tables with what each is, and the files what it runs links, by what links them.
    let fleet = &said["std/fleet"];
    for (table, is) in [
        ("agents", "Who is in the fleet."),
        ("stages", "What is done with a card."),
        ("segments", "What more is done with a card where it is"),
    ] {
        let line = fleet
            .iter()
            .find(|line| opens(line) == Some(("table".to_string(), table.to_string())))
            .unwrap_or_else(|| panic!("`{table}` in {fleet:?}"));
        assert!(line.text.starts_with(&format!("`{table}`: {is}")), "{}", line.text);
    }
    assert!(
        fleet.iter().any(|line| line.text.starts_with("Change the words what it runs puts together:")),
        "{fleet:?}"
    );
    assert!(
        fleet.iter().any(|line| opens(line) == Some(("default".to_string(), "std/work".to_string()))
            && line.text.starts_with("`std/work`: ")),
        "{fleet:?}"
    );
}

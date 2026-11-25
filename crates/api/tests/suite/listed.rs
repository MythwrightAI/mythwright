//! Which of the blueprints that ship runs which: the survey of everything
//! that ships, and what the daemon hands out of it.
//!
//! A blueprint is found by its name by everything that runs, so nothing
//! is renamed or moved to put the list in order: what is listed first is
//! what somebody applies, and under each its parts, the blueprints it
//! runs. Which is which is worked out from the texts and from what each
//! action says of itself (`blueprint::names`, over the library:
//! `texts::listed`). What is held here is what only everything that
//! ships can say: that every blueprint is either one somebody applies or
//! reached from one that is, and that the ones at the top are the ones
//! the product means somebody to apply, each said here with why.

use std::path::{Path, PathBuf};

use blueprint::ActionSpec;
use blueprint::names::{self, How};
use conductor::library::listed::{APPLIED, Listed, listed};
use conductor::{Library, MemoryLibrary};
use reqwest::Method;
use serde_json::{Value, json};

use crate::Rig;

fn top() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap()
}

/// What somebody applies of the standard blueprints, each with why it
/// is somebody's to apply. The first group is what nothing else names;
/// the second is `texts::listed::APPLIED`, said there too, which
/// something else that ships runs as well. A package says its own, in a
/// file of its `contract/` folder ([`APPLIED_OF_A_PACKAGE`]): nothing
/// here names a package.
const SOMEBODY_APPLIES: &[(&str, &str)] = &[
    ("std/ab", "gives a project A/B tests of its cards, from the board's page"),
    ("std/board", "gives a project its board; it is what a new project starts with"),
    ("std/bot", "a bot of somebody's own, which answers in a channel with what a command prints"),
    (
        "std/discuss",
        "has a channel's agents talk things over in its threads: applied for a channel, from its menu",
    ),
    ("std/docs", "gives a project a library: a folder's documents"),
    ("std/environments", "gives a project its environments, as a new project may start with"),
    ("std/features", "gives a project its features, which end as cards of its board"),
    ("std/flags", "gives a project its feature flags, as a new project may start with"),
    ("std/fleet", "a project's fleet: what its own `fleet` text calls, and what a new project starts with"),
    ("std/log-feed", "attached by a text of somebody's own to what runs services, to follow their logs"),
    ("std/mockups", "gives a project its mockups"),
    ("std/pipeline", "runs a project's pipelines as the rows of its triggers say"),
    ("std/profile", "says what is true of this factory's profile, for every blueprint here to read"),
    (
        "std/project",
        "sets a project up with what it starts with: applied for one as it is added, and as it is given something more",
    ),
    ("std/rolling", "runs a command on each device in turn"),
    ("std/site", "writes a project's library out as web pages"),
    ("std/snapshot", "writes a copy of this factory to a folder, to start another from"),
    ("std/store", "makes a database and hosts it as a store service, for tables to be kept in"),
    ("std/topics", "gives the factory its topics: applied as the first topic is made, by whoever manages it"),
    ("std/table", "gives a project a table of its own, from New… and then Table in the side panel"),
    ("std/timed", "gives a project its table of how long each card took, beside its board"),
    ("std/watch", "says in a channel what happens to something that keeps a history"),
    // Applied though something that ships runs them too.
    ("std/agent", "applied for each agent added by hand; a role calls it for each of its agents"),
    ("std/plan", "applied by hand for one plan; a fleet calls it for every plan of a folder"),
    (
        "std/pool",
        "started from a text of somebody's own for whatever is heavy; a fleet has one for each stage",
    ),
    (
        "std/slot",
        "hosts a service, called from a text of somebody's own; the store and the log feed each use it",
    ),
];

/// Where a package says which of its blueprints somebody applies, each
/// on a line with why.
const APPLIED_OF_A_PACKAGE: &str = "contract/applied.txt";

/// What somebody applies of everything that ships, each with why: the
/// standard ones, and what each package that brings blueprints says.
fn somebody_applies() -> Vec<(String, String)> {
    let mut said: Vec<(String, String)> =
        SOMEBODY_APPLIES.iter().map(|(name, why)| ((*name).to_string(), (*why).to_string())).collect();
    let (packages, _) = agents::extension::load_all(&top().join("extensions"));
    for (package, brought) in &packages {
        if brought.blueprints.is_empty() {
            continue;
        }
        let file = top().join("extensions").join(package).join(APPLIED_OF_A_PACKAGE);
        let text = std::fs::read_to_string(&file).unwrap_or_else(|_| {
            panic!(
                "the package `{package}` brings blueprints and has no `{APPLIED_OF_A_PACKAGE}`: a line for each \
                 of them that somebody applies, its name and then why"
            )
        });
        for line in text.lines().filter(|line| !line.starts_with('#') && !line.trim().is_empty()) {
            let (name, why) = line.split_once(char::is_whitespace).unwrap_or((line, ""));
            said.push((name.to_string(), why.trim().to_string()));
        }
    }
    said
}

/// Everything that ships: the standard blueprints and what each package
/// brings, installed here or not, with the actions this program and the
/// packages describe.
async fn shipped() -> (MemoryLibrary, Vec<ActionSpec>) {
    let rig = Rig::new().await;
    let mut actions = rig.node().app.engine.registry().all_specs();
    let library = conductor::standard();
    let (packages, _) = agents::extension::load_all(&top().join("extensions"));
    for package in packages.values() {
        for action in package.actions.values() {
            if !actions.iter().any(|known| known.name == action.spec.name) {
                actions.push(action.spec.clone());
            }
        }
        for (name, text) in &package.blueprints {
            library.put(name, text);
        }
    }
    (library, actions)
}

/// The survey as a table: each blueprint with what names it and how, and
/// then what somebody applies with how many parts each has.
fn table(library: &MemoryLibrary, actions: &[ActionSpec], listed: &Listed) -> String {
    let all = library.names(None);
    let known = |name: &str| all.iter().any(|one| one == name);
    let mut by: Vec<(String, Vec<String>)> = all.iter().map(|name| (name.clone(), Vec::new())).collect();
    for name in &all {
        for named in names::of(&library.load(name, None).unwrap(), &known, actions) {
            let how = match &named.how {
                How::Attaches => "uses".to_string(),
                How::Calls => "call".to_string(),
                How::Spawns => "spawn".to_string(),
                How::Default(param) => format!("default of `{param}`"),
                How::Asks(action) => format!("`{action}` reaches it"),
                How::Writes => "a value".to_string(),
            };
            let line = by.iter_mut().find(|(one, _)| *one == named.name).unwrap();
            line.1.push(format!("{name} ({how})"));
        }
    }
    let mut out = String::from("what names each:\n");
    for (name, named) in &by {
        let named = if named.is_empty() { "nothing".to_string() } else { named.join(", ") };
        out.push_str(&format!("  {name} <- {named}\n"));
    }
    out.push_str("what somebody applies:\n");
    for top in listed.tops() {
        let parts = listed.parts(&top);
        out.push_str(&format!("  {top} · {} parts: {}\n", parts.len(), parts.join(" ")));
    }
    out
}

#[tokio::test]
async fn every_blueprint_that_ships_is_one_somebody_applies_or_a_part_of_one_and_those_applied_are_the_ones_meant()
 {
    let (library, actions) = shipped().await;
    let found = listed(&library, &actions);
    let table = table(&library, &actions, &found);
    println!("{table}");
    assert!(found.runs.len() >= 50, "the blueprints that ship were found: {}", found.runs.len());

    // Nothing is hidden by being nobody's part and nobody's to apply.
    let unreached = names::unreached(&found.runs, &found.applied);
    assert!(
        unreached.is_empty(),
        "these blueprints name each other in a ring that nothing outside it names, so a list has them nowhere: \
         {unreached:?}. One of them is somebody's to apply (add it to `texts::listed::APPLIED` with why), or \
         something that is applied runs one of them and does not say so\n{table}"
    );

    // What is at the top is what the product means somebody to apply.
    let meant = somebody_applies();
    for top in found.tops() {
        assert!(
            meant.iter().any(|(name, _)| *name == top),
            "`{top}` is listed among what somebody applies, because no blueprint that ships names it. If it is a \
             part, name it from what runs it: a `call`, a `spawn` or a `uses`, the default of a param, its name \
             as a value, or, where only an action knows it, the action saying so (`ActionSpec::reaching`). If \
             somebody is to apply it, say so with why: a standard one in `SOMEBODY_APPLIES` in this test, a \
             package's in its `{APPLIED_OF_A_PACKAGE}`\n{table}"
        );
    }
    for (name, why) in &meant {
        assert!(!why.is_empty(), "{name} says why somebody applies it");
        assert!(
            found.tops().iter().any(|top| top == name),
            "`{name}` is said to be something somebody applies, and it is not listed so: something that ships \
             names it now, or it is gone. If it is still applied by itself, say so in `texts::listed::APPLIED`; \
             if it is a part now, take its line away\n{table}"
        );
    }

    // A line of what is applied though something runs it is needed, and is of something there.
    for (name, why) in APPLIED {
        assert!(!why.is_empty(), "{name} says why");
        assert!(
            found.runs.contains_key(*name),
            "`{name}` of `texts::listed::APPLIED` is no blueprint that ships"
        );
        assert!(
            !found.part_of(name).is_empty(),
            "`{name}` is on `texts::listed::APPLIED`, and nothing that ships runs it: it is listed among what \
             somebody applies without the line, which is to be taken off"
        );
    }

    // What the ways of naming are for, each where it was found.
    let parts = |of: &str| found.parts(of);
    let fleet = parts("std/fleet");
    for (part, how) in [
        ("std/arrangement", "a step calls it"),
        ("std/told", "it is attached"),
        ("std/review", "the default of a param"),
        ("std/working", "a value in the table of what a row's word comes to, two texts down"),
        ("std/work", "a value in the list of what a round does"),
        ("std/dispatch", "an action reaches it, and no text writes it"),
        ("std/signin", "the same"),
        ("std/pool", "applied by itself, and a part all the same"),
    ] {
        assert!(fleet.iter().any(|one| one == part), "`{part}` is a part of the fleet ({how}): {fleet:?}");
    }
    assert_eq!(parts("std/board"), ["std/cards"]);
    assert_eq!(parts("std/slot"), ["std/canary-stage"]);
    assert_eq!(found.part_of("std/slot"), ["std/log-feed", "std/store"]);
    assert_eq!(found.part_of("std/cards"), ["std/board"]);
    assert!(found.part_of("std/fleet").is_empty());
}

#[tokio::test]
async fn the_list_of_blueprints_says_what_each_that_ships_runs_and_everything_it_said_before() {
    let rig = Rig::new().await;
    let answer = rig.read("/api/blueprints").await;
    let all = answer["blueprints"].as_array().unwrap();
    let one = |name: &str| {
        all.iter().find(|one| one["name"] == name).unwrap_or_else(|| panic!("{name} is listed: {answer}"))
    };
    // Everything it handed, it hands: the names, and of each what it is
    // started with and gives back.
    assert_eq!(answer["names"].as_array().unwrap().len(), all.len());
    for said in all {
        for key in ["name", "file", "description", "params", "outputs"] {
            assert!(said.get(key).is_some(), "{said} says `{key}`");
        }
        assert!(said["runs"].is_array(), "one that is no project's says what it runs: {said}");
    }
    let runs = |name: &str| -> Vec<&str> {
        one(name)["runs"].as_array().unwrap().iter().filter_map(Value::as_str).collect()
    };
    assert_eq!(runs("std/board"), ["std/cards"]);
    assert_eq!(runs("std/cards"), [] as [&str; 0]);
    assert_eq!(
        runs("std/fleet"),
        ["std/arrangement", "std/chain", "std/plan", "std/proposal", "std/review", "std/run", "std/told"],
        "each once, by name, in order"
    );
    assert_eq!(runs("std/work"), ["std/turn"]);
    assert_eq!(runs("std/turn"), ["std/dispatch", "std/signin"], "what an action it asks reaches");
    // Applied though something runs it: said of those, and of no other.
    let applied: Vec<&str> =
        all.iter().filter(|one| one["applied"] == true).filter_map(|one| one["name"].as_str()).collect();
    assert_eq!(applied, ["std/agent", "std/plan", "std/pool", "std/slot"]);
    assert!(all.iter().all(|one| one["applied"] == true || one.get("applied").is_none()));

    // A project's own blueprint is the project's: nothing is said of what it runs, though it calls one that
    // ships, and it makes nothing that ships a part.
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    let own = "blueprint: release\nsteps:\n  board:\n    call: std/board\n";
    let (status, saved) = rig
        .call(
            Method::PUT,
            "/api/projects/shop/file",
            Some(&rig.token),
            Some(json!({ "path": "@blueprints/release.bp", "content": own })),
        )
        .await;
    assert!(status.is_success(), "{saved}");
    let for_shop = rig.read("/api/blueprints?project=shop").await;
    let listed = for_shop["blueprints"].as_array().unwrap();
    let release = listed.iter().find(|one| one["name"] == "release").expect("the project's own is listed");
    assert_eq!(release["file"], "@blueprints/release.bp");
    assert!(release.get("runs").is_none() && release.get("applied").is_none(), "{release}");
    let board = listed.iter().find(|one| one["name"] == "std/board").unwrap();
    assert_eq!(board["runs"], json!(["std/cards"]));
    assert_eq!(rig.read("/api/blueprints").await, answer, "and the factory's list is as it was");
}

fn part<'a>(summary: &'a Value, kind: &str) -> &'a Value {
    let parts = summary["parts"].as_array().expect("a summary has parts");
    parts.iter().find(|part| part["kind"] == kind).unwrap_or_else(|| panic!("no part `{kind}` in {summary}"))
}

#[tokio::test]
async fn the_summary_of_one_that_ships_says_what_it_is_a_part_of_and_leads_there() {
    let rig = Rig::new().await;
    let is = |summary: &Value| part(summary, "is")["lines"].as_array().unwrap().clone();
    let said = |lines: &[Value]| -> Vec<Value> {
        lines
            .iter()
            .filter(|line| line["text"].as_str().is_some_and(|text| text.starts_with("A part of")))
            .cloned()
            .collect()
    };

    // A part of one: said after what it says it is, and the line opens what runs it.
    let cards = rig.send("/api/blueprints/summary", json!({ "name": "std/cards" })).await;
    let lines = is(&cards["summary"]);
    assert_eq!(lines[1]["text"], "A part of `std/board`.", "{lines:?}");
    assert_eq!(lines[1]["opens"], json!({ "what": "default", "name": "std/board" }));
    assert_eq!(said(&lines).len(), 1);

    // A part of several, here one that is applied by itself too: each on a line of its own, which opens it.
    let slot = rig.send("/api/blueprints/summary", json!({ "name": "std/slot" })).await;
    let lines = said(&is(&slot["summary"]));
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0]["text"], "A part of 2 blueprints:");
    assert!(lines[0].get("opens").is_none());
    assert_eq!(
        lines[0]["under"],
        json!([
            { "text": "`std/log-feed`", "at": null, "key": null, "mark": null, "under": [], "opens": { "what": "default", "name": "std/log-feed" } },
            { "text": "`std/store`", "at": null, "key": null, "mark": null, "under": [], "opens": { "what": "default", "name": "std/store" } },
        ])
    );

    // One that nothing runs says nothing of it, and nothing else of any summary is another.
    let board = rig.send("/api/blueprints/summary", json!({ "name": "std/board" })).await;
    assert!(said(&is(&board["summary"])).is_empty());
    // A text nothing was said of is said as it always was, whatever it is called; and the check's answer for
    // one that is said to ship carries the same line.
    let text = rig.node().app.engine.library().load("std/cards", None).unwrap();
    let plainly = rig.send("/api/blueprints/summary", json!({ "source": text })).await;
    assert!(said(&is(&plainly["summary"])).is_empty());
    let mut without = cards["summary"].clone();
    let lines = without["parts"][0]["lines"].as_array_mut().unwrap();
    lines.remove(1);
    let change = without["parts"].as_array_mut().unwrap().pop().unwrap();
    assert_eq!(change["kind"], "change");
    assert_eq!(
        without, plainly["summary"],
        "but for the line and for how it is changed, it is the text's own"
    );
    let checked = rig.send("/api/blueprints/check", json!({ "source": text, "ships": true })).await;
    assert_eq!(checked["summary"], cards["summary"]);
}

//! Every word of the language and every action a person writes in a
//! blueprint of their own is shown by a whole blueprint, as a node hands
//! the language out: each checks against every action there is with no
//! warning, is laid out as the formatter lays it out, and asks what it is
//! the example of. And where the blueprints there are use each thing is
//! handed out beside it.

use serde_json::Value;

use crate::Rig;

/// The actions that are shown by no example, each by how its name begins,
/// with why: none is a step somebody writes in a blueprint of their own.
const UNSHOWN: &[(&str, &str)] = &[
    ("work.", "a block a fleet's stage asks of a card's row, handed the row whole"),
    ("review.", "a block a fleet's stage asks of a review, handed the row whole"),
    ("segments.", "a block a fleet's stage asks of its list of things done"),
    ("turn.", "a block `std/turn` asks of the word an agent's turn ended with"),
    ("column.", "a block a fleet's stage asks of its own row"),
    ("tickets.", "a block a feature's plan asks, handed the plan and the board"),
    ("sides.", "a block a package's blueprint asks, keeping two sides in step"),
    ("place.make", "a block a card's round asks of the place it is worked in"),
    ("place.keep", "a block a card's round asks of the place it is worked in"),
    ("place.send", "a block a card's round asks of the place it is worked in"),
    ("place.merge", "a block a card's round asks of the place it is worked in"),
    ("place.clear", "a block a card's round asks of the place it is worked in"),
    ("place.changed", "a block a card's round asks of the place it is worked in"),
    ("place.head", "a block a card's round asks of the place it is worked in"),
    ("place.take", "a block a card's round asks of the place it is worked in"),
    ("place.follows", "a block a card's round asks of the place it is worked in"),
    ("place.tidied", "a block a card's round asks of what clearing its place came to"),
    ("place.unkept", "a block a card's round asks of what could not be kept in its place"),
    ("rollout.", "a block a rolling update asks of its own state"),
    ("pool.turn", "the block `std/pool` asks; a pool is applied, and asked by its events"),
    ("tells.whom", "a block a fleet's blueprint asks of who is told what"),
    (
        "discussion.",
        "what a channel's discussions are kept and decided by, asked by `std/discuss` and the texts it starts",
    ),
    ("chain.", "a block a fleet's blueprint asks of who is next for a card that waits"),
    ("timed.", "a block `std/timed` asks of a card's history and of a board's columns"),
    ("triggers.", "a block `std/pipeline` asks of the rows of a project's triggers"),
    ("flags.due", "a block `std/flags` asks of the rows of a project's flags"),
    ("contest.", "a block a card's A/B test asks of its contest, its contenders and what its judges said"),
    ("record.change", "a block a package's blueprint asks of a change it heard"),
    ("channel.answer", "a block the factory's bot asks of what a command came to"),
    ("system.answer_as", "the factory's bot's, asked as whoever spoke to it"),
    ("agent.present", "a block a fleet's blueprint asks of who is in its channel"),
    ("agent.counted", "a block a fleet's arrangement asks of a role's agents"),
    ("agent.news", "a block a fleet's blueprint asks of an agent's turns"),
    ("agent.round", "a block a fleet's stage asks of a card's round"),
    ("board.after", "a block `std/cards` asks of what a card waits on"),
    ("board.came", "a block `std/cards` asks of a card's move"),
    ("board.held", "a block `std/cards` asks of the cards that wait"),
    ("board.hold", "a stage says of a card its column holds, once it has asked the block it asks"),
    ("board.said", "a block `std/cards` asks of what was said on a card"),
    ("board.settle", "a block a board asks of itself as the daemon starts"),
    ("board.waits", "a block `std/cards` asks of whether a card may begin"),
];

fn text(of: &Value, key: &str) -> String {
    of[key].as_str().unwrap_or_default().to_string()
}

/// Every example the language is handed out with, with what it is of.
fn examples(language: &Value) -> Vec<(String, String)> {
    let mut all = Vec::new();
    let each = |of: &Value| of.as_array().cloned().unwrap_or_default();
    for shape in each(&language["shapes"]) {
        all.push((format!("the mapping `{}`", text(&shape, "name")), text(&shape, "example")));
        for field in each(&shape["fields"]) {
            all.push((
                format!("`{}` of `{}`", text(&field, "name"), text(&shape, "name")),
                text(&field, "example"),
            ));
        }
    }
    for name in each(&language["names"]) {
        all.push((format!("the name `{}`", text(&name, "name")), text(&name, "example")));
    }
    for function in each(&language["functions"]) {
        all.push((format!("the function `{}`", text(&function, "name")), text(&function, "example")));
    }
    for action in each(&language["actions"]) {
        let name = text(&action, "name");
        if action["example"].is_string() {
            all.push((format!("the action `{name}`"), text(&action, "example")));
        }
    }
    all
}

#[tokio::test]
async fn every_example_checks_against_every_action_there_is_with_no_warning_and_is_laid_out() {
    let rig = Rig::new().await;
    let engine = &rig.node().app.engine;
    let language = rig.read("/api/blueprints/language").await;
    let all = examples(&language);
    assert!(all.len() > 150, "{}", all.len());
    let mut wrong = Vec::new();
    for (what, example) in &all {
        if example.is_empty() {
            wrong.push(format!("{what} is handed out with no example"));
            continue;
        }
        let said: Vec<String> = engine
            .check(example, None)
            .diagnostics
            .iter()
            .map(|problem| format!("{}: {}", problem.code, problem.message))
            .collect();
        if !said.is_empty() {
            wrong.push(format!("the example of {what} is not clean: {said:?}\n{example}"));
        }
        if blueprint::format(example, &Default::default()).ok().as_deref() != Some(example.as_str()) {
            wrong.push(format!("the example of {what} is not laid out as the formatter lays it out"));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n\n"));
}

#[tokio::test]
async fn every_action_a_person_writes_is_shown_by_a_blueprint_that_asks_it_and_those_that_are_not_say_why() {
    let rig = Rig::new().await;
    let language = rig.read("/api/blueprints/language").await;
    for action in language["actions"].as_array().unwrap() {
        let name = text(action, "name");
        let unshown = UNSHOWN.iter().find(|(begins, _)| name.starts_with(begins));
        match (action["example"].as_str(), unshown) {
            (Some(example), None) => {
                let used = blueprint::used::used(&[("example".into(), example.to_string())]);
                assert!(
                    used.actions.contains_key(&name),
                    "the example of `{name}` does not ask it:\n{example}"
                );
            }
            (Some(_), Some((begins, _))) => {
                panic!("`{name}` has an example, and is said here to have none ({begins})")
            }
            (None, None) => panic!(
                "`{name}` is shown by no example: say `.example(...)` beside it, a whole blueprint that asks it, \
                 or say here why a person writes it in no blueprint of their own"
            ),
            (None, Some(_)) => {}
        }
    }
    for (begins, _) in UNSHOWN {
        let some = language["actions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|action| text(action, "name").starts_with(begins));
        assert!(some, "no action begins `{begins}` any more: take its line out");
    }
}

#[tokio::test]
async fn where_the_blueprints_there_are_use_each_thing_is_handed_out_the_shortest_first() {
    let rig = Rig::new().await;
    let used = rig.read("/api/blueprints/language/used").await;
    let retry = &used["words"]["step/retry"]["places"];
    assert_eq!(retry[0]["blueprint"], "std/rolling-step", "{retry}");
    assert!(retry[0]["line"].as_u64().unwrap() > 1);
    assert!(used["words"].get("handler/poll").is_none(), "nothing that ships polls: {}", used["words"]);
    let posted = &used["actions"]["channel.post"];
    let places = posted["places"].as_array().unwrap();
    assert_eq!(places.len(), blueprint::used::MOST);
    assert!(posted["more"].as_u64().unwrap() > 0, "{posted}");
    assert!(!used["names"]["event"]["places"].as_array().unwrap().is_empty());
}

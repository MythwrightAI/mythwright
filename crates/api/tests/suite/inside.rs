//! What actions say is written inside their arguments, as a node hands it
//! out: held to the tables it is said by, to the notes, and to the copy
//! the client's tests read.

use std::path::{Path, PathBuf};

use blueprint::Within;
use blueprint::inside::Row;
use serde_json::Value;

use crate::Rig;

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Every argument that says anything of what is written inside it, with
/// its action's name and what it says.
fn said(actions: &Value) -> Vec<(String, String, Within, &Value)> {
    let mut found = Vec::new();
    for action in actions.as_array().unwrap() {
        for arg in action["args"].as_array().unwrap() {
            if let Some(within) = arg.get("within") {
                let name = |of: &Value| of["name"].as_str().unwrap().to_string();
                found.push((name(action), name(arg), read(within), within));
            }
        }
    }
    found
}

/// What was handed out, as the program keeps it: read back through the
/// rows it is written as.
fn read(within: &Value) -> Within {
    fn rows(within: &Value, under: &str, into: &mut Vec<(String, String, String)>) {
        let text = |of: &Value, key: &str| of[key].as_str().unwrap_or_default().to_string();
        let all = |of: &Value| of.as_array().cloned().unwrap_or_default();
        let under = match within["each"].as_str() {
            Some(each) => {
                into.push((format!("{under}*"), "each".into(), each.into()));
                format!("{under}*.")
            }
            None => under.to_string(),
        };
        for word in all(&within["words"]) {
            into.push((format!("{under}{}", text(&word, "name")), "word".into(), text(&word, "help")));
        }
        for key in all(&within["keys"]) {
            let path = format!("{under}{}", text(&key, "name"));
            into.push((path.clone(), text(&key, "type"), text(&key, "help")));
            for word in all(&key["words"]) {
                into.push((format!("{path}.{}", text(&word, "name")), "word".into(), text(&word, "help")));
            }
        }
        for key in all(&within["keys"]) {
            if !key["within"].is_null() {
                rows(&key["within"], &format!("{under}{}.", text(&key, "name")), into);
            }
        }
    }
    let mut found = Vec::new();
    rows(within, "", &mut found);
    let borrowed: Vec<Row> =
        found.iter().map(|(path, kind, says)| (path.as_str(), kind.as_str(), says.as_str())).collect();
    Within::of(&borrowed)
}

#[tokio::test]
async fn an_action_hands_out_what_it_says_is_written_inside_an_argument_as_its_table_says_it() {
    let rig = Rig::new().await;
    let offered = rig.read("/api/actions").await;
    let said = said(&offered["actions"]);
    let tables: &[(&str, &str, &[Row])] = &[
        ("collection.ensure", "fields", kernel::inside::FIELDS),
        ("collection.ensure", "views", kernel::inside::VIEWS),
        ("rows.read", "fill", kernel::inside::GIVEN),
        ("rows.read", "giving", kernel::inside::GIVEN),
        ("rows.read", "counting", kernel::inside::COUNTING),
        ("rows.read", "gathering", kernel::inside::GATHERING),
        ("rows.read", "saying", kernel::inside::SAYING),
        ("service.start", "health", hosting::HEALTH),
    ];
    for (action, argument, table) in tables {
        let handed = said.iter().find(|(of, arg, ..)| of == action && arg == argument);
        let (.., within, raw) =
            handed.unwrap_or_else(|| panic!("`{argument}` of `{action}` says what is inside it"));
        assert_eq!(*within, Within::of(table), "`{argument}` of `{action}`, as it is handed out: {raw}");
    }
    // The language is handed to an editor with the same, in the one read.
    let language = rig.read("/api/blueprints/language").await;
    assert_eq!(self::said(&language["actions"]).len(), said.len());
    // Whatever says anything says a sentence for each key, and what each thing by a name is.
    for (action, argument, within, _) in &said {
        for (path, kind, says) in within.rows() {
            assert!(
                kind == "word" || !says.is_empty(),
                "`{argument}.{path}` of `{action}` says nothing of itself"
            );
        }
    }
    // An argument that says nothing is handed out as it always was.
    let name = &offered["actions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|one| one["name"] == "collection.ensure")
        .unwrap()["args"][0];
    let keys: Vec<&str> = name.as_object().unwrap().keys().map(String::as_str).collect();
    assert_eq!(keys, ["help", "name", "required", "type"]);
}

/// One row of the notes' table: an action, where in its arguments
/// something is written, what kind of thing it is, and its sentence.
fn row(action: &str, argument: &str, path: &str, kind: &str, says: &str) -> String {
    format!("| `{action}` | `{argument}.{path}` | {kind} | {says} |")
}

/// The gate for the notes: under *What is written inside an argument*,
/// `docs/blueprints.md` has a row for each thing an action says, as it
/// says it, and no row for what none says. The test gives the rows.
#[tokio::test]
async fn the_notes_list_what_is_written_inside_each_argument_as_its_action_says_it() {
    let rig = Rig::new().await;
    let offered = rig.read("/api/actions").await;
    let mut rows = Vec::new();
    for (action, argument, within, _) in said(&offered["actions"]) {
        for (path, kind, says) in within.rows() {
            rows.push(row(&action, &argument, &path, &kind, &says));
        }
    }
    if std::env::var_os("SAY_ROWS").is_some() {
        println!("{}", rows.join("\n"));
    }
    let docs = std::fs::read_to_string(repo().join("docs/blueprints.md")).unwrap();
    let heading = "\n### What is written inside an argument\n";
    let begins = docs.find(heading).unwrap_or_else(|| panic!("docs/blueprints.md has the heading{heading}"));
    let section = &docs[begins + heading.len()..];
    let section = &section[..section.find("\n## ").unwrap_or(section.len())];
    let missing: Vec<&String> =
        rows.iter().filter(|row| !section.lines().any(|line| line == row.as_str())).collect();
    assert!(
        missing.is_empty(),
        "docs/blueprints.md is out of step with what actions say is written inside their arguments. Its table \
         (under *What is written inside an argument*) should have these rows:\n{}",
        missing.iter().map(|row| row.as_str()).collect::<Vec<_>>().join("\n")
    );
    let stale: Vec<&str> = section
        .lines()
        .filter(|line| line.starts_with("| `") && !rows.iter().any(|row| row == line))
        .collect();
    assert!(
        stale.is_empty(),
        "docs/blueprints.md lists, under *What is written inside an argument*, what no action says:\n{}",
        stale.join("\n")
    );
}

/// The client's tests of what it explains and offers inside an argument
/// are run against a copy of the actions that say something there, kept
/// beside them. This holds the copy to what a node hands out, so those
/// tests cannot pass against words no action says any more.
#[tokio::test]
async fn the_copy_of_what_actions_say_is_written_inside_their_arguments_is_what_they_say() {
    let rig = Rig::new().await;
    let language = rig.read("/api/blueprints/language").await;
    let telling: Vec<&Value> = language["actions"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|action| action["args"].as_array().unwrap().iter().any(|arg| arg.get("within").is_some()))
        .collect();
    let described = format!("{}\n", serde_json::to_string_pretty(&telling).unwrap());
    let path = repo().join("ui/src/blueprint/inside.json");
    if std::env::var_os("WRITE_INSIDE").is_some() {
        std::fs::write(&path, &described).unwrap();
    }
    let kept = std::fs::read_to_string(&path).unwrap_or_default();
    assert!(
        kept == described,
        "ui/src/blueprint/inside.json is not what the actions here say is written inside their arguments. \
         Write it again with `WRITE_INSIDE=1 cargo test -p api the_copy_of_what_actions_say`, then run the \
         client's tests"
    );
}

//! The package that came with an installation, against the copy of its
//! name that is installed: where the two differ, as a table of cases, and
//! the one that came taken in place of the copy because somebody asked.

use agents::came::{Differs, differs};
use kernel::{Principal, PrincipalKind};
use serde_json::{Value, json};

use crate::{Rig, ada};

fn places(found: &[&str]) -> Vec<String> {
    found.iter().map(|place| place.to_string()).collect()
}

#[test]
fn where_the_one_that_came_differs_from_the_copy_is_said_by_place_and_in_words() {
    // The copy, the one that came, then what the one that came says
    // besides, says another way and does not say, and the words.
    let cases: Vec<(Value, Value, [&[&str]; 3], &str)> = vec![
        (json!({ "title": "A" }), json!({ "title": "A" }), [&[], &[], &[]], ""),
        (
            json!({ "agent": { "wall": { "keeps": [".a"] } } }),
            json!({ "agent": { "wall": { "keeps": [".a"], "signin": [".a/auth"] } } }),
            [&["agent.wall.signin"], &[], &[]],
            "says `agent.wall.signin` besides",
        ),
        // A list is one thing, the same or not, and so is a text.
        (
            json!({ "agent": { "args": [["run"]] }, "blueprints": { "serve": "a\nb\n" } }),
            json!({ "agent": { "args": [["run", "--json"]] }, "blueprints": { "serve": "a\nc\n" } }),
            [&[], &["agent.args", "blueprints.serve"], &[]],
            "says something else of `agent.args` and `blueprints.serve`",
        ),
        // What only the copy says: an earlier package's, or its owner's own.
        (
            json!({ "title": "A", "agent": { "models": { "list": [] } } }),
            json!({ "title": "A", "agent": {} }),
            [&[], &[], &["agent.models"]],
            "does not say `agent.models`",
        ),
        // A mapping where the other has something else is another thing said of one place.
        (
            json!({ "program": "tool", "install": "Get it." }),
            json!({ "program": { "name": "tool" }, "actions": { "status": {} }, "title": "T" }),
            [&["actions", "title"], &["program"], &["install"]],
            "says `actions` and `title` besides; says something else of `program`; does not say `install`",
        ),
        // Many are named as far as a sentence carries, and then counted.
        (
            json!({}),
            json!({ "a": 1, "b": 1, "c": 1, "d": 1, "e": 1, "f": 1, "g": 1, "h": 1 }),
            [&["a", "b", "c", "d", "e", "f", "g", "h"], &[], &[]],
            "says `a`, `b`, `c`, `d`, `e`, `f` and 2 more besides",
        ),
    ];
    for (copy, came, [besides, another, gone], words) in cases {
        let found = differs(&copy, &came);
        let meant = Differs { besides: places(besides), another: places(another), gone: places(gone) };
        assert_eq!(found, meant, "{copy} against {came}");
        assert_eq!(found.in_words(), words, "{copy} against {came}");
        assert_eq!(found.is_empty(), words.is_empty());
        assert_eq!(found.to_json()["words"], words);
        // The other way round, what one says besides the other does not say.
        let back = differs(&came, &copy);
        assert_eq!((back.besides, back.gone, back.another), (found.gone, found.besides, found.another));
    }
}

/// A package whose program is nowhere: nothing here runs one.
fn manifest(keeps: &str) -> String {
    format!(
        "extension: sample\ntitle: Sample\ndescription: Not a real agent.\nagent:\n  program: no-such-program-here\n  \
         input: ${{ turn.prompt }}\n  output:\n    - reply: ${{ event.text }}\n  wall:\n    keeps: {keeps}\n"
    )
}

fn member() -> Principal {
    Principal {
        kind: PrincipalKind::User,
        id: "bo".into(),
        name: "Bo".into(),
        scopes: vec!["agents".into(), "channels".into()],
        key: None,
    }
}

#[tokio::test]
async fn the_package_that_came_is_taken_in_place_of_the_copy_only_when_asked_and_says_where_they_differed() {
    let mut rig = Rig::new().await;
    let installed = manifest("[\".sample\"]");
    rig.agents.add_extension(&installed, &ada()).await.unwrap();
    let file = rig.dir.join("extensions/sample/extension.yaml");
    let added = async |rig: &Rig| rig.events("extension.added").await.len();
    assert_eq!(added(&rig).await, 1);

    // Nothing of that name came with this installation.
    let none = rig.agents.take_what_came("sample", &ada()).await.err().unwrap();
    assert_eq!(none.kind, kernel::ErrorKind::NotFound);
    assert!(none.message.contains("no package called `sample` came with this installation"), "{none:?}");

    // One comes that says more, and another that is not installed.
    let came =
        installed.replace("keeps: [\".sample\"]", "keeps: [\".sample\"]\n    signin: [\".sample/auth\"]");
    assert_ne!(came, installed);
    std::fs::create_dir_all(rig.dir.join("offered/sample")).unwrap();
    std::fs::write(rig.dir.join("offered/sample/extension.yaml"), &came).unwrap();
    std::fs::create_dir_all(rig.dir.join("offered/other")).unwrap();
    let other = installed.replace("extension: sample", "extension: other");
    std::fs::write(rig.dir.join("offered/other/extension.yaml"), other).unwrap();
    let behind = rig.agents.behind_by();
    assert_eq!(behind.keys().collect::<Vec<_>>(), ["sample"]);
    assert_eq!(behind["sample"].in_words(), "says `agent.wall.signin` besides");

    // Nothing takes it unasked: not asking which are behind, and not a start.
    rig.restart().await;
    assert_eq!(rig.agents.behind_what_came(), ["sample"]);
    assert_eq!(std::fs::read_to_string(&file).unwrap(), installed, "the copy is as it was, to the byte");
    assert!(!rig.dir.join("extensions/sample/extension.was.yaml").exists());
    assert_eq!(added(&rig).await, 1, "and nothing was said to have been installed");

    // It is the owner's to take, as installing one is.
    let refused = rig.agents.take_what_came("sample", &member()).await.err().unwrap();
    assert_eq!(refused.kind, kernel::ErrorKind::Denied, "{refused:?}");
    assert_eq!(std::fs::read_to_string(&file).unwrap(), installed);
    // One that came and is not installed is installed, not updated.
    let absent = rig.agents.take_what_came("other", &ada()).await.err().unwrap();
    assert_eq!(absent.kind, kernel::ErrorKind::NotFound);
    assert!(
        absent.message.contains(&format!("`{} extension add other` installs it", brand::CLI)),
        "{absent:?}"
    );
    assert!(rig.agents.extension("other").is_none());

    // Asked, the one that came is installed, the copy is kept beside it, and it is said.
    let taken = rig.agents.take_what_came("sample", &ada()).await.unwrap();
    assert_eq!(taken.differs.unwrap().in_words(), "says `agent.wall.signin` besides");
    assert_eq!(taken.extension.wall.signin, [".sample/auth"]);
    assert_eq!(std::fs::read_to_string(&file).unwrap(), came);
    let beside = rig.dir.join("extensions/sample/extension.was.yaml");
    assert_eq!(std::fs::read_to_string(&beside).unwrap(), installed);
    assert!(rig.agents.behind_what_came().is_empty());
    assert_eq!(added(&rig).await, 2);

    // Asked again there is nothing to take: nothing is written and nothing said.
    let again = rig.agents.take_what_came("sample", &ada()).await.unwrap();
    assert!(again.differs.is_none());
    assert_eq!(std::fs::read_to_string(&beside).unwrap(), installed);
    assert_eq!(added(&rig).await, 2);

    // A copy its owner changed reads as an earlier one does: all that is
    // known is where the two differ, and taking the one that came
    // replaces it, with what was changed kept beside it and nowhere else.
    let changed = came.replace("title: Sample", "title: Sample\ninstall: Ask Ada for it.");
    rig.agents.add_extension(&changed, &ada()).await.unwrap();
    assert_eq!(rig.agents.behind_by()["sample"].in_words(), "does not say `install`");
    let taken = rig.agents.take_what_came("sample", &ada()).await.unwrap();
    assert_eq!(taken.differs.unwrap().gone, ["install"]);
    assert_eq!(std::fs::read_to_string(&file).unwrap(), came);
    assert_eq!(std::fs::read_to_string(&beside).unwrap(), changed);
}

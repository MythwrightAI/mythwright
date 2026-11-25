//! How a project's things are arranged in the side panel
//! (`kernel::outline`): what an outline is held to and what each change
//! comes to, case for case with the table the client reads.

use kernel::outline::{DEEPEST, LONGEST_NAME, Node, Op, apply, checked, placed_in};
use serde_json::{Value, json};

/// The table the client reads too.
fn table() -> Value {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../ui/src/outlinecases.json");
    let kept = std::fs::read_to_string(path).expect("the table of outlines is in the client's sources");
    serde_json::from_str(&kept).expect("the table is a mapping of cases")
}

#[test]
fn an_outline_is_kept_or_refused_in_its_sentence_case_for_case() {
    let table = table();
    let cases = table["checked"].as_array().unwrap();
    assert!(cases.len() > 20, "the table is there");
    for case in cases {
        let came = checked(&case[0]).err();
        assert_eq!(came.as_deref(), case[1].as_str(), "{}", case[0]);
    }
    assert_eq!(table["longest"], json!(LONGEST_NAME), "the client holds a name to what is kept");
    assert_eq!(table["deepest"], json!(DEEPEST));
}

#[test]
fn each_change_comes_to_what_its_case_says_or_is_refused_in_its_sentence() {
    let table = table();
    let cases = table["applied"].as_array().unwrap();
    assert!(cases.len() > 20, "the table is there");
    for case in cases {
        let kept = (!case[0].is_null()).then(|| checked(&case[0]).unwrap());
        let seen: Vec<String> =
            case[2].as_array().unwrap().iter().map(|one| one.as_str().unwrap().to_string()).collect();
        let came = Op::read(&case[1]).and_then(|op| apply(kept.as_ref(), &op, &seen));
        let meant = &case[3];
        match came {
            Err(refused) => assert_eq!(json!({ "refused": refused }), *meant, "{case}"),
            Ok(applied) => {
                let items = applied
                    .outline
                    .map(|outline| Value::Array(outline.items.iter().map(Node::to_value).collect()));
                let came = json!({ "items": items, "changed": applied.changed, "done": applied.done });
                assert_eq!(came, *meant, "{case}");
            }
        }
    }
}

#[test]
fn a_key_of_a_node_that_is_not_known_here_is_kept_as_it_was_through_a_change() {
    let kept = checked(&json!({ "items": [
        { "folder": "f1", "name": "Kept", "access": { "general": "invited" }, "items": [{ "thing": "board" }] },
    ] }))
    .unwrap();
    let op = Op::read(&json!({ "op": "rename", "folder": "f1", "name": "Private" })).unwrap();
    let came = apply(Some(&kept), &op, &[]).unwrap().outline.unwrap();
    assert_eq!(
        came.to_value()["items"],
        json!([{ "folder": "f1", "name": "Private", "items": [{ "thing": "board" }], "access": { "general": "invited" } }])
    );
}

#[test]
fn a_folder_is_given_a_number_never_given_before_in_the_outline() {
    let kept = checked(&json!({ "next": 1, "items": [{ "folder": "f4", "name": "Old" }] })).unwrap();
    assert_eq!(kept.next, 5, "past every folder there is, whatever was said");
    let make = Op::read(&json!({ "op": "make", "name": "New" })).unwrap();
    let made = apply(Some(&kept), &make, &[]).unwrap();
    assert_eq!(made.done, json!({ "made": "f5" }));
    // A folder removed leaves its number given.
    let outline = made.outline.unwrap();
    let removed = apply(Some(&outline), &Op::Remove { folder: "f5".into() }, &[]).unwrap().outline.unwrap();
    assert_eq!(apply(Some(&removed), &make, &[]).unwrap().done, json!({ "made": "f6" }));
}

#[test]
fn what_is_handed_to_somebody_who_may_not_read_a_thing_leaves_it_out_and_every_folder_in() {
    let kept = checked(&json!({ "items": [
        { "thing": "board" },
        { "folder": "f1", "name": "Insights", "items": [{ "thing": "table:secret" }, { "thing": "table:runs" }] },
    ] }))
    .unwrap();
    let shown = kept.kept_to(&|key| key != "table:secret");
    assert_eq!(
        shown.to_value()["items"],
        json!([{ "thing": "board" }, { "folder": "f1", "name": "Insights", "items": [{ "thing": "table:runs" }] }])
    );
    assert_eq!(kept.things(), ["board", "table:secret", "table:runs"]);
}

#[test]
fn what_a_thing_is_shown_as_goes_with_it_and_nobody_who_may_not_read_it_is_handed_it() {
    let kept = checked(&json!({ "items": [
        { "thing": "board", "name": "Roadmap" },
        { "folder": "f1", "name": "Kept", "items": [{ "thing": "library:secret", "name": "Plans for May" }] },
    ] }))
    .unwrap();
    let shown = kept.kept_to(&|key| key != "library:secret");
    assert_eq!(
        shown.to_value()["items"],
        json!([{ "thing": "board", "name": "Roadmap" }, { "folder": "f1", "name": "Kept", "items": [] }])
    );
    assert!(!shown.to_value().to_string().contains("Plans for May"));
    // Moved out of its folder with the folder removed, it is still shown as it was named.
    let removed = apply(Some(&kept), &Op::Remove { folder: "f1".into() }, &[]).unwrap().outline.unwrap();
    assert_eq!(
        removed.to_value()["items"],
        json!([{ "thing": "board", "name": "Roadmap" }, { "thing": "library:secret", "name": "Plans for May" }])
    );
}

fn path(said: &[&str]) -> Vec<String> {
    said.iter().map(|one| one.to_string()).collect()
}

#[test]
fn things_placed_by_a_path_make_each_folder_of_it_that_is_not_there_and_the_same_done_twice_is_nothing() {
    let things = path(&["table:test-runs", "table:deployments"]);
    let first = placed_in(None, &path(&["Insights", "Runs"]), &things).unwrap();
    assert!(first.changed);
    let outline = first.outline.unwrap();
    assert_eq!(
        outline.to_value()["items"],
        json!([{ "folder": "f1", "name": "Insights", "items": [
            { "folder": "f2", "name": "Runs", "items": [{ "thing": "table:test-runs" }, { "thing": "table:deployments" }] }
        ] }])
    );
    assert_eq!(first.done, json!({ "folder": "f2" }));
    // Asked again, whatever the capitals: nothing changes, and the folder is the one there.
    let again = placed_in(Some(&outline), &path(&["insights", "RUNS"]), &things).unwrap();
    assert_eq!(
        (again.changed, again.outline.as_ref(), again.done.clone()),
        (false, Some(&outline), json!({ "folder": "f2" }))
    );
    // A thing already in the folder stays where it is there; one that is not comes at its end.
    let more =
        placed_in(Some(&outline), &path(&["Insights", "Runs"]), &path(&["table:deployments", "board"]))
            .unwrap();
    assert_eq!(
        more.outline.unwrap().to_value()["items"][0]["items"][0]["items"],
        json!([{ "thing": "table:test-runs" }, { "thing": "table:deployments" }, { "thing": "board" }])
    );
    // Put at the top where no path is said, and a folder made with nothing put in it.
    let top = placed_in(Some(&outline), &[], &path(&["table:test-runs"])).unwrap();
    assert_eq!(top.outline.unwrap().to_value()["items"][1], json!({ "thing": "table:test-runs" }));
    let alone = placed_in(None, &path(&["Empty"]), &[]).unwrap();
    assert_eq!(
        alone.outline.unwrap().to_value()["items"],
        json!([{ "folder": "f1", "name": "Empty", "items": [] }])
    );
}

#[test]
fn things_placed_by_a_path_are_refused_in_the_sentences_of_a_change() {
    assert_eq!(
        placed_in(None, &[], &[]).err().as_deref(),
        Some("Say what is to be put: a thing's key, or the folder it is to be put in.")
    );
    assert_eq!(
        placed_in(None, &path(&[" "]), &path(&["board"])).err().as_deref(),
        Some("A folder is called something: say what this one is called.")
    );
    assert!(
        placed_in(None, &path(&["Insights"]), &path(&["tables:wrong"]))
            .unwrap_err()
            .contains("is no thing of a project")
    );
}

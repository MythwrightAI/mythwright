//! Whether an item is held where it is until something its column
//! offers is pressed: a table of cases over the rows, what the item
//! keeps of what was pressed and which coming of it this is.

use deciding::pressed::{Held, Hold, coming, held};
use serde_json::{Value, json};

fn rows() -> Vec<Value> {
    vec![
        json!({ "id": "shop-actions-1", "title": "Signed", "column": "ready", "who": "person", "holds": true }),
        json!({ "id": "shop-actions-2", "title": "Noted", "column": "ready", "holds": false }),
        json!({ "id": "shop-actions-3", "title": "Cleared", "column": "ready", "holds": true, "again": true }),
        json!({ "id": "shop-actions-4", "title": "Looked over", "column": "review", "holds": true }),
    ]
}

fn words(said: &[&str]) -> Vec<String> {
    said.iter().map(|word| word.to_string()).collect()
}

fn asked(here: &str, pressed: &[&str], moved: &[&str]) -> Held {
    Held {
        item: "shop-4".into(),
        here: here.into(),
        actions: rows(),
        pressed: words(pressed),
        moved: words(moved),
    }
}

#[test]
fn an_item_is_held_while_what_holds_in_its_column_stands_unpressed_at_this_coming() {
    let moved = ["backlog", "user", "", "3"];
    let back = ["doing", "blueprint", "Not yet.", "5"];
    let cases: Vec<(&str, Held, Option<Vec<String>>)> = vec![
        ("with no row, nothing holds it", Held { actions: vec![], ..asked("ready", &[], &moved) }, None),
        ("in a column no row is of", asked("doing", &[], &moved), None),
        (
            "nothing pressed: each row that holds",
            asked("ready", &[], &moved),
            Some(words(&["Signed", "Cleared"])),
        ),
        (
            "one pressed, the other still to be",
            asked("ready", &["pressed signed 3 10 user:ada"], &moved),
            Some(words(&["Cleared"])),
        ),
        (
            "both pressed at this coming: it goes on",
            asked("ready", &["pressed signed 3 10 user:ada", "pressed cleared 3 11 user:ada"], &moved),
            None,
        ),
        (
            "sent back and come again: what is for one coming is to be pressed again, what is for good stands",
            asked("ready", &["pressed signed 3 10 user:ada", "pressed cleared 3 11 user:ada"], &back),
            Some(words(&["Cleared"])),
        ),
        (
            "what holds nothing is not waited for",
            asked("ready", &["pressed noted 3 9 user:ada", "pressed signed 3 10 user:ada"], &moved),
            Some(words(&["Cleared"])),
        ),
        (
            "added where it is and never moved: the first coming",
            asked("ready", &["pressed signed 0 10 user:ada", "pressed cleared 0 11 user:ada"], &[]),
            None,
        ),
        (
            "what it keeps of a hold is no press",
            asked("ready", &["held 3", "pressed signed 3 10 user:ada"], &moved),
            Some(words(&["Cleared"])),
        ),
        ("another column's row", asked("review", &[], &back), Some(words(&["Looked over"]))),
    ];
    for (what, asked, meant) in cases {
        let came: Option<Hold> = held(&asked);
        assert_eq!(came.as_ref().map(|hold| hold.waits.clone()), meant, "{what}");
        if let Some(hold) = came {
            assert_eq!(hold.at, coming(&asked.moved), "{what}: which coming");
            assert!(hold.says.starts_with("shop-4 waits in "), "{what}: {}", hold.says);
            assert_eq!(hold.value()["waits"], json!(hold.waits));
        }
    }
    assert_eq!(
        coming(&words(&["yesterday"])),
        0,
        "a row that says something else of its moves is its first coming"
    );
}

//! What is being built of this program itself and may be turned on before it
//! is whole (`kernel::experiments`): each off unless turned on, a name the
//! list no longer has passed over, and one it never had refused.

use kernel::experiments::{Experiment, LIST, is_on, read, shown, turned};
use serde_json::json;

const TRYING: &[Experiment] = &[
    Experiment { name: "topics", card: "390", says: "Topics, piece by piece." },
    Experiment { name: "other", card: "1", says: "Something else." },
];

#[test]
fn an_experiment_is_off_until_turned_on_and_says_the_card_it_is_for() {
    let none: Vec<String> = Vec::new();
    assert_eq!(
        shown(TRYING, &none),
        [
            json!({ "name": "topics", "card": "390", "says": "Topics, piece by piece.", "on": false }),
            json!({ "name": "other", "card": "1", "says": "Something else.", "on": false }),
        ]
    );
    let on = turned(TRYING, &none, "topics", true).unwrap();
    assert_eq!(on, ["topics"]);
    assert!(is_on(&on, "topics") && !is_on(&on, "other"));
    // Turned on twice is on once; turned off, it is off.
    assert_eq!(turned(TRYING, &on, "topics", true).unwrap(), ["topics"]);
    assert_eq!(turned(TRYING, &on, "topics", false).unwrap(), Vec::<String>::new());
}

#[test]
fn a_name_the_list_no_longer_has_is_passed_over_and_one_it_never_had_is_refused() {
    let kept = read(Some(&json!(["gone", "topics"])));
    assert_eq!(kept, ["gone", "topics"]);
    assert_eq!(turned(TRYING, &kept, "other", true).unwrap(), ["topics", "other"]);
    assert!(turned(TRYING, &kept, "nothing", true).unwrap_err().contains("`topics`, `other`"));
    assert!(turned(LIST, &kept, "topics", true).unwrap_err().contains("there are none now"));
    assert_eq!(read(None), Vec::<String>::new());
    // None is listed today.
    assert!(LIST.is_empty());
}

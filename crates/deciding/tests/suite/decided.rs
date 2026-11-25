//! What becomes of an item that was looked at, as one answer for every
//! way it is heard of; and what a council named by a row is begun with.

use std::collections::BTreeSet;

use deciding::decided::{DECIDED_WORDS, Decided, decided, word};
use deciding::reviews::VerdictCame;
use deciding::segments::{Asked, Run, Said, council, rest_is_usual};
use serde_json::{Value, json};

fn heard(topic: &str) -> Decided {
    Decided {
        topic: topic.into(),
        item: "shop-7".into(),
        found: true,
        status: "review".into(),
        here: "review".into(),
        pass: "done".into(),
        fail: "doing".into(),
        by: "shop-reviewer-1".into(),
        words: "It adds up.".into(),
        approved: 1,
        seats: 1,
        usual: 10,
        ..Decided::default()
    }
}

#[test]
fn what_an_item_that_was_decided_comes_to_is_one_word_for_every_way_it_is_heard_of() {
    let kept = |usual: bool| {
        vec![
            json!({ "card": "shop-7", "review": "review-shop-7-9", "row": "r3", "order": 15, "usual": usual }),
        ]
    };
    let came =
        |what: &str, also: &str| Decided { came: what.into(), says: also.into(), ..heard("review.came") };
    // A looking that is over, heard after whatever it said: kept still
    // under its name, it said nothing of how it came out.
    let ended = |review: &str| Decided { review: review.into(), kept: kept(false), ..heard("review.ended") };
    let cases: Vec<(&str, Decided, &str)> = vec![
        ("approved, with nothing more to do, it goes on", heard("review.passed"), "on"),
        (
            "approved where what was done is put with the rest",
            Decided { merge: true, ..heard("review.passed") },
            "after",
        ),
        (
            "approved where the project lays things about the looking",
            Decided { lists: true, ..heard("passed") },
            "after",
        ),
        ("approved, moved since", Decided { status: "done".into(), ..heard("review.passed") }, "nothing"),
        ("approved, gone", Decided { found: false, ..heard("review.passed") }, "nothing"),
        (
            "approved with nowhere to go on to",
            Decided { pass: String::new(), ..heard("review.passed") },
            "nothing",
        ),
        ("sent back", heard("review.failed"), "back"),
        ("sent back, moved since", Decided { status: "doing".into(), ..heard("review.failed") }, "nothing"),
        ("nobody decided", heard("review.undecided"), "left"),
        ("what is done before anybody looks sent it back", came("back", "2 tests failed"), "back"),
        ("or left it", came("left", "shop-7: Tests could not be done"), "left"),
        ("or there was no looking in the list, and it goes on", came("on", ""), "on"),
        ("with nowhere to go on to", Decided { pass: String::new(), ..came("on", "") }, "nothing"),
        ("or it was moved meanwhile", came("nothing", ""), "nothing"),
        ("its looking ended having said nothing, and it is here still", ended("review-shop-7-9"), "waits"),
        (
            "its looking said how it came out before it ended, which was heard first",
            Decided { kept: vec![], ..ended("review-shop-7-9") },
            "nothing",
        ),
        ("another looking has taken its place", ended("review-shop-7-4"), "nothing"),
        (
            "its looking asked for another in its place, which is no verdict",
            Decided { topic: "review.due".into(), ..ended("review-shop-7-9") },
            "nothing",
        ),
        (
            "its looking ended having said nothing, moved since",
            Decided { status: "done".into(), ..ended("review-shop-7-9") },
            "nothing",
        ),
    ];
    let mut reached = BTreeSet::new();
    for (what, asked, word) in cases {
        let came = decided(&asked).unwrap_or_else(|why| panic!("{what}: {why}"));
        assert_eq!(came.word, word, "{what}");
        reached.insert(came.word);
    }
    assert_eq!(reached, DECIDED_WORDS.iter().copied().collect::<BTreeSet<_>>(), "a word no case reaches");

    // What goes with the word.
    let passed = decided(&Decided { merge: true, ..heard("review.passed") }).unwrap();
    assert!(passed.merges && !passed.lists && passed.approves, "{passed:?}");
    assert_eq!((passed.verdict.as_str(), passed.says.as_str()), ("passed", "approved by shop-reviewer-1"));
    assert_eq!(
        (passed.row.as_str(), passed.order),
        ("", 10),
        "a looking kept with no place is the usual one"
    );
    // Where it was among the things done is what whoever asks kept of it.
    let listed =
        decided(&Decided { lists: true, merge: true, kept: kept(false), ..heard("review.passed") }).unwrap();
    assert!(listed.lists && !listed.merges, "{listed:?}");
    assert_eq!((listed.row.as_str(), listed.order), ("r3", 15));
    // Kept as having only what is usual after it, what is usual is done by itself.
    let usual =
        decided(&Decided { lists: true, merge: true, kept: kept(true), ..heard("review.passed") }).unwrap();
    assert!(usual.merges && !usual.lists, "{usual:?}");
    let said = decided(&Decided { reports: true, ..heard("review.failed") }).unwrap();
    assert_eq!(said.told, "shop-7 was sent back to doing: It adds up.");
    assert_eq!((said.verdict.as_str(), said.approves, said.says.as_str()), ("failed", false, "It adds up."));
    let nobody = decided(&heard("review.undecided")).unwrap();
    assert!(nobody.says.starts_with("shop-7: those who looked at it neither approved it nor sent it back"));
    assert!(!nobody.keeps, "it is looked at again when it is said to be due");
    let waits = decided(&ended("review-shop-7-9")).unwrap();
    assert!(
        waits.says.starts_with("shop-7 is in review, and its review ended with nothing said"),
        "{waits:?}"
    );
    assert!(
        waits.verdict.is_empty() && !waits.approves && !waits.keeps && waits.told.is_empty(),
        "{waits:?}"
    );
    let left = decided(&came("left", "why")).unwrap();
    assert!(left.keeps && left.verdict.is_empty() && !left.approves, "{left:?}");
    // Whether it was sent back from here before, for whoever asks to forget that as it goes on.
    let back = vec![json!({ "card": "shop-7", "at": "12" })];
    assert!(decided(&Decided { back, ..heard("review.passed") }).unwrap().sent);
    assert!(!decided(&heard("review.passed")).unwrap().sent);
    // And what is none of these is refused in a sentence.
    assert!(decided(&heard("review.voted")).unwrap_err().contains("`topic` is what was heard of shop-7"));
    assert!(decided(&came("maybe", "")).unwrap_err().contains("came to `maybe`"));
}

#[test]
fn putting_what_was_done_with_the_rest_ends_in_a_word_as_any_thing_done_does() {
    assert_eq!(word(&VerdictCame::Merge), "");
    assert_eq!(word(&VerdictCame::On(String::new())), "on");
    assert_eq!(word(&VerdictCame::Back(String::new())), "back");
    assert_eq!(word(&VerdictCame::Stuck(String::new())), "left");
    assert_eq!(word(&VerdictCame::Nothing), "nothing");
}

#[test]
fn a_council_a_row_names_is_begun_with_what_the_row_says_and_the_rest_as_whoever_asks_says() {
    let own = json!({ "role": "reviewer", "picks": "least", "seats": 1, "needed": 1, "remote": "origin" });
    let own = own.as_object().unwrap();
    let cells = |said: Value| said.as_object().cloned().unwrap();
    let run = |cells| Run {
        title: "Council".into(),
        name: "std/review".into(),
        order: 15,
        spawned: true,
        cells,
        ..Run::default()
    };
    let with = council(&run(cells(json!({ "seats": "3", "needed": 2 }))), own).unwrap();
    assert_eq!(Value::Object(with), json!({ "role": "reviewer", "picks": "least", "seats": 3, "needed": 2 }));
    let with = council(&run(cells(json!({ "role": "auditor" }))), own).unwrap();
    assert_eq!(with["role"], "auditor");
    assert_eq!(with["seats"], 1);
    let why = council(&run(cells(json!({ "command": "make test" }))), own).unwrap_err();
    assert!(
        why.starts_with(
            "the row Council names std/review, which is begun and heard from, and says `command`"
        ),
        "{why}"
    );
    let why = council(&Run { with: vec!["fails=left".into()], ..run(cells(json!({}))) }, own).unwrap_err();
    assert!(why.contains("says `fails=left` in `with`"), "{why}");
    let why = council(&run(cells(json!({ "seats": "three" }))), own).unwrap_err();
    assert!(why.contains("says `seats` is `three`"), "{why}");

    // What becomes of work in hand at a restart goes with it only where
    // whoever asks says that what it begins is told so.
    let told = json!({ "role": "reviewer", "seats": 1, "after_restart": "leave", "restarts": 2, "restarting": true });
    let with = council(&run(cells(json!({}))), told.as_object().unwrap()).unwrap();
    assert_eq!((&with["after_restart"], &with["restarts"]), (&json!("leave"), &json!(2)));
    let untold = json!({ "role": "reviewer", "seats": 1, "after_restart": "leave", "restarts": 2 });
    let with = council(&run(cells(json!({}))), untold.as_object().unwrap()).unwrap();
    assert!(!with.contains_key("after_restart") && !with.contains_key("restarts"), "{with:?}");

    // Whether only what is usual comes after it.
    let row = |order: i64, labelled: &str| Said {
        id: "r".into(),
        title: "Notes".into(),
        order,
        does: "shop/notes".into(),
        labelled: labelled.into(),
        ..Said::default()
    };
    let asked = |rows| Asked { rows, item: "shop-7".into(), labels: vec!["docs".into()], ..Asked::default() };
    let at = Run { order: 10, ..Run::default() };
    assert!(rest_is_usual(&asked(vec![row(5, "")]), &at));
    assert!(!rest_is_usual(&asked(vec![row(15, "")]), &at));
    assert!(
        rest_is_usual(&asked(vec![row(15, "urgent")]), &at),
        "a row held to another label is not this item's"
    );
    assert!(!rest_is_usual(&asked(vec![row(15, "docs")]), &at));
}

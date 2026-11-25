//! What a column asks before a thing there is taken up: the rows read,
//! a press kept and read back, what stands, what still waits, who may.

use kernel::pressed::{
    Action, Asked, Pressed, Refused, Who, did, find, held_says, holding, may, named, pressing, rows, waits,
};
use serde_json::json;

fn signed() -> Action {
    Action::read(&json!({ "id": "shop-actions-1", "title": "Signed", "column": "ready", "who": "person", "holds": true }))
        .unwrap()
}

fn again() -> Action {
    Action { again: true, ..signed() }
}

#[test]
fn a_row_is_read_by_its_cells_and_known_by_its_name_or_what_it_is_called() {
    let action = signed();
    assert_eq!(action.name, "signed");
    assert_eq!(action.who, Who::Person);
    assert!(action.holds && !action.again);
    let said = Action::read(
        &json!({ "id": "x-1", "data": { "title": "Signed off", "name": "Off", "holds": "true" } }),
    )
    .unwrap();
    assert_eq!((said.name.as_str(), said.title.as_str(), said.holds), ("off", "Signed off", true));
    assert_eq!(named("Signed off!"), "signed-off");
    assert!(
        Action::read(&json!({ "id": "x-2", "title": "  " })).is_none(),
        "called nothing, it is nothing to press"
    );
    let all =
        rows(&[json!({ "id": "a", "title": "One" }), json!("no row"), json!({ "id": "b", "title": "Two" })]);
    assert_eq!(all.len(), 2);
    assert_eq!(find(&all, "two").map(|one| one.id.as_str()), Some("b"));
    assert_eq!(find(&all, "a").map(|one| one.name.as_str()), Some("one"), "by its id");
    assert_eq!(find(&all, "ONE").map(|one| one.id.as_str()), Some("a"));
    assert!(find(&all, "three").is_none());
}

#[test]
fn a_press_is_kept_as_words_and_read_back_the_same_the_latest_of_each_row_standing() {
    let action = signed();
    let pressed = Pressed::default().press(&action, 2, ("user:ada", "Ada"), 1000).unwrap();
    let pressed = pressed.hold(2).unwrap();
    let kept = pressed.kept();
    assert_eq!(kept, vec!["pressed signed 2 1000 user:ada Ada".to_string(), "held 2".to_string()]);
    assert_eq!(Pressed::read(&kept), pressed);
    let twice = [kept.clone(), vec!["pressed signed 3 2000 key:k".into(), "nonsense".into()]].concat();
    let read = Pressed::read(&twice);
    assert_eq!(read.presses.len(), 1, "one press of a row is kept, the latest");
    assert_eq!(
        (read.presses[0].by.as_str(), read.presses[0].name.as_str()),
        ("key:k", ""),
        "what it is called, where it was said"
    );
    let named = Pressed::read(&["pressed signed 3 2000 user:ada Ada Lovelace".to_string()]);
    assert_eq!(named.presses[0].name, "Ada Lovelace");
    assert!(pressed.hold(2).is_none(), "said to be held once for each coming");
    assert!(pressed.hold(3).is_some());
}

#[test]
fn a_press_for_good_stands_at_every_coming_and_one_for_a_coming_only_at_its_own() {
    let pressed = Pressed::default().press(&signed(), 2, ("user:ada", "Ada"), 1).unwrap();
    assert!(pressed.standing(&signed(), 5).is_some());
    let pressed = Pressed::default().press(&again(), 2, ("user:ada", "Ada"), 1).unwrap();
    assert!(pressed.standing(&again(), 2).is_some());
    assert!(pressed.standing(&again(), 5).is_none(), "sent back and come again, it is to be pressed again");
    assert!(pressed.press(&again(), 2, ("user:bob", "Bob"), 2).is_none(), "pressed twice is pressed once");
    assert!(pressed.press(&again(), 5, ("user:bob", "Bob"), 2).is_some());
}

#[test]
fn a_press_is_taken_back_only_at_the_coming_it_was_made_at() {
    let pressed = Pressed::default().press(&signed(), 2, ("user:ada", "Ada"), 1).unwrap();
    assert!(pressed.take_back(&signed(), 2).unwrap().presses.is_empty());
    assert!(pressed.take_back(&signed(), 3).unwrap_err().contains("where it was pressed"));
    assert!(Pressed::default().take_back(&signed(), 2).unwrap_err().contains("nothing to take back"));
}

#[test]
fn what_a_thing_waits_for_is_what_holds_in_its_column_and_stands_unpressed() {
    let other = Action { name: "cleared".into(), title: "Cleared".into(), ..signed() };
    let loose = Action { name: "noted".into(), title: "Noted".into(), holds: false, ..signed() };
    let elsewhere = Action { name: "done".into(), column: "review".into(), ..signed() };
    let all = vec![signed(), other.clone(), loose, elsewhere];
    let none = Pressed::default();
    let names = |left: Vec<&Action>| left.iter().map(|one| one.name.clone()).collect::<Vec<_>>();
    assert_eq!(names(waits(&all, "ready", &none, 0)), ["signed", "cleared"]);
    let one = none.press(&signed(), 0, ("user:ada", "Ada"), 1).unwrap();
    assert_eq!(names(waits(&all, "ready", &one, 0)), ["cleared"]);
    assert!(waits(&all, "doing", &none, 0).is_empty(), "nothing holds a column no row is of");
    assert!(waits(&[], "ready", &none, 0).is_empty(), "with no row, nothing waits");
}

#[test]
fn who_may_press_is_said_and_held_to() {
    assert!(may(&signed(), "user", false).is_ok());
    assert!(may(&signed(), "key", false).is_ok(), "a key somebody made is theirs");
    assert!(may(&signed(), "agent", true).unwrap_err().contains("an agent may not"));
    let scoped = Action { scope: "admin".into(), who: Who::Anyone, ..signed() };
    assert!(may(&scoped, "agent", true).is_ok());
    assert!(may(&scoped, "user", false).unwrap_err().contains("whoever holds `admin`"));
    assert_eq!(Action { scope: "admin".into(), ..signed() }.who_may(), "a person who holds `admin`");
    assert_eq!(Action { who: Who::Anyone, ..signed() }.who_may(), "anyone who may change it");
}

#[test]
fn what_is_said_of_a_press_and_of_a_held_thing() {
    assert_eq!(did(&signed(), false), "pressed Signed");
    assert_eq!(did(&signed(), true), "took back Signed");
    assert_eq!(did(&Action { says: "signed it off".into(), ..signed() }, false), "signed it off");
    let other = Action { title: "Cleared".into(), who: Who::Anyone, ..signed() };
    assert_eq!(
        held_says("shop-4", "ready", &[&signed()]),
        "shop-4 waits in ready for Signed, which a person presses. It is taken up when that is pressed."
    );
    assert_eq!(
        held_says("shop-4", "ready", &[&signed(), &other]),
        "shop-4 waits in ready for Signed, which a person presses; and Cleared, which anyone who may change it \
         presses. It is taken up when each is pressed."
    );
}

/// A press of `word` on `shop-1`, in `column` the first time it came there.
fn asked<'a>(column: &'a str, word: &'a str, back: bool, kind: &'a str) -> Asked<'a> {
    Asked { item: "shop-1", column, coming: 1, word, back, by: (kind, "user:ada", "Ada"), at: 5 }
}

#[test]
fn what_a_press_comes_to_by_each_way_it_can_go() {
    let cleared =
        Action::read(&json!({ "id": "a-2", "title": "Cleared", "column": "ready", "holds": true })).unwrap();
    let looked =
        Action::read(&json!({ "id": "a-3", "title": "Looked", "column": "review", "scope": "admin" }))
            .unwrap();
    let all = [signed(), cleared, looked];
    let none = Pressed::default();
    let holds = |_: &str| false;
    // Each refusal, with its sentence.
    let cases: [(Asked<'_>, Refused); 5] = [
        (
            asked("ready", "approved", false, "user"),
            Refused::Unoffered(
                "`approved` is not something shop-1 is offered; in ready it is offered Signed, Cleared"
                    .into(),
            ),
        ),
        (
            asked("doing", "approved", false, "user"),
            Refused::Unoffered(
                "nothing is pressed on shop-1 in doing: no row of what can be pressed is of that column"
                    .into(),
            ),
        ),
        (
            asked("ready", "looked", false, "user"),
            Refused::Unfit("Looked is pressed on a card in review, and shop-1 is in ready".into()),
        ),
        (
            asked("ready", "signed", false, "agent"),
            Refused::Denied("Signed is pressed by a person, and an agent may not press it".into()),
        ),
        (
            asked("ready", "signed", true, "user"),
            Refused::Unfit("Signed is not pressed, so there is nothing to take back".into()),
        ),
    ];
    for (asked, refused) in cases {
        assert_eq!(pressing(&all, &none, asked, holds), Err(refused));
    }
    assert!(matches!(
        pressing(&all, &none, asked("review", "looked", false, "user"), holds),
        Err(Refused::Denied(_))
    ));
    assert!(
        pressing(&all, &none, asked("review", "looked", false, "user"), |scope| scope == "admin").is_ok()
    );

    // Pressed: one of two that hold, then the last, which lets it go.
    let first = pressing(&all, &none, asked("ready", "signed", false, "user"), holds).unwrap();
    assert_eq!((first.released, first.waits.clone()), (false, vec!["Cleared".to_string()]));
    let one = first.now.unwrap();
    let last = pressing(&all, &one, asked("ready", "Cleared", false, "user"), holds).unwrap();
    assert!(last.released && last.waits.is_empty());
    // Pressed again it writes nothing, and lets nothing go.
    let again =
        pressing(&all, &last.now.clone().unwrap(), asked("ready", "cleared", false, "user"), holds).unwrap();
    assert!(again.now.is_none() && !again.released);
    // Taken back, it waits for it again.
    let back = pressing(&all, &last.now.unwrap(), asked("ready", "cleared", true, "user"), holds).unwrap();
    assert_eq!((back.released, back.waits), (false, vec!["Cleared".to_string()]));
}

#[test]
fn a_thing_is_said_to_be_held_once_for_each_coming_and_not_where_nothing_holds_it() {
    let all = [signed()];
    let (said, left) = holding(&all, "ready", &Pressed::default(), 1).unwrap();
    assert_eq!(left.iter().map(|one| one.title.as_str()).collect::<Vec<_>>(), ["Signed"]);
    assert!(holding(&all, "ready", &said, 1).is_none(), "said already, this time it came");
    assert!(holding(&all, "ready", &said, 2).is_some(), "it came again");
    assert!(holding(&all, "doing", &Pressed::default(), 1).is_none(), "nothing holds it there");
    let pressed = Pressed::default().press(&all[0], 1, ("user:ada", "Ada"), 5).unwrap();
    assert!(holding(&all, "ready", &pressed, 1).is_none(), "what held it is pressed");
}

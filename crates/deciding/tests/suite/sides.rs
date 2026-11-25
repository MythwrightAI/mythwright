//! Two sides of one thing kept in step (`deciding::sides`, asked as
//! `sides.cross` and `sides.owed`): tables of what is handed and what it
//! comes to, with no word left unreached and what makes no sense refused.

use std::collections::BTreeSet;

use deciding::sides::{CAME, OWED, owed_told, told};
use serde_json::{Map, Value, json};

fn handed(args: Value) -> Map<String, Value> {
    args.as_object().cloned().unwrap()
}

#[test]
fn what_crosses_between_two_sides_is_what_moved_since_they_last_agreed_and_where_both_did_the_side_that_was_said()
 {
    // here, there, last, prefer; then came, out, in, agreed.
    type Case = (Value, Value, Value, &'static str, &'static str, Value, Value, Value);
    let cases: Vec<Case> = vec![
        // Both say what was agreed: nothing crosses.
        (
            json!({ "title": "A cart", "closed": false }),
            json!({ "title": "A cart", "closed": false }),
            json!({ "title": "A cart", "closed": false }),
            "here",
            "same",
            json!({}),
            json!({}),
            json!({ "title": "A cart", "closed": false }),
        ),
        // This side moved: it goes out, whichever side is preferred.
        (
            json!({ "title": "A basket" }),
            json!({ "title": "A cart" }),
            json!({ "title": "A cart" }),
            "there",
            "out",
            json!({ "title": "A basket" }),
            json!({}),
            json!({ "title": "A basket" }),
        ),
        // The other side moved: it comes in, whichever is preferred.
        (
            json!({ "title": "A cart" }),
            json!({ "title": "A trolley" }),
            json!({ "title": "A cart" }),
            "here",
            "in",
            json!({}),
            json!({ "title": "A trolley" }),
            json!({ "title": "A trolley" }),
        ),
        // One field moved here and another there: each goes its way.
        (
            json!({ "title": "A basket", "closed": false }),
            json!({ "title": "A cart", "closed": true }),
            json!({ "title": "A cart", "closed": false }),
            "here",
            "both",
            json!({ "title": "A basket" }),
            json!({ "closed": true }),
            json!({ "title": "A basket", "closed": true }),
        ),
        // One field moved on both, to different things: the side said.
        (
            json!({ "title": "A basket" }),
            json!({ "title": "A trolley" }),
            json!({ "title": "A cart" }),
            "here",
            "out",
            json!({ "title": "A basket" }),
            json!({}),
            json!({ "title": "A basket" }),
        ),
        (
            json!({ "title": "A basket" }),
            json!({ "title": "A trolley" }),
            json!({ "title": "A cart" }),
            "there",
            "in",
            json!({}),
            json!({ "title": "A trolley" }),
            json!({ "title": "A trolley" }),
        ),
        // Both moved to the same thing: nothing crosses, and that is
        // what they agree on now.
        (
            json!({ "title": "A basket" }),
            json!({ "title": "A basket" }),
            json!({ "title": "A cart" }),
            "there",
            "same",
            json!({}),
            json!({}),
            json!({ "title": "A basket" }),
        ),
        // Never agreed, and they differ: the side said, for each field.
        (
            json!({ "title": "A cart", "body": "Customers need one." }),
            json!({ "title": "Cart", "body": "Customers need one." }),
            json!({}),
            "here",
            "out",
            json!({ "title": "A cart" }),
            json!({}),
            json!({ "title": "A cart", "body": "Customers need one." }),
        ),
        (
            json!({ "title": "A cart" }),
            json!({ "title": "Cart" }),
            Value::Null,
            "there",
            "in",
            json!({}),
            json!({ "title": "Cart" }),
            json!({ "title": "Cart" }),
        ),
        // What was sent and is now said by both is no echo: what is
        // agreed has only not been written down yet.
        (
            json!({ "title": "A basket" }),
            json!({ "title": "A basket" }),
            json!({ "title": "A cart" }),
            "here",
            "same",
            json!({}),
            json!({}),
            json!({ "title": "A basket" }),
        ),
        // Emptied, never said and said as nothing are one nothing.
        (
            json!({ "body": "" }),
            json!({ "body": null }),
            json!({}),
            "here",
            "same",
            json!({}),
            json!({}),
            json!({ "body": "" }),
        ),
        (
            json!({ "title": "A cart" }),
            json!({ "title": "A cart", "body": "" }),
            json!({ "title": "A cart", "body": null }),
            "there",
            "same",
            json!({}),
            json!({}),
            json!({ "title": "A cart", "body": null }),
        ),
        // Emptied here since they agreed: that goes out as nothing.
        (
            json!({ "body": "" }),
            json!({ "body": "Customers need one." }),
            json!({ "body": "Customers need one." }),
            "there",
            "out",
            json!({ "body": "" }),
            json!({}),
            json!({ "body": "" }),
        ),
        // A field only the other side says, which was agreed as nothing.
        (
            json!({ "title": "A cart" }),
            json!({ "title": "A cart", "closed": true }),
            json!({ "title": "A cart", "closed": null }),
            "here",
            "in",
            json!({}),
            json!({ "closed": true }),
            json!({ "title": "A cart", "closed": true }),
        ),
        // A flag put back as it was on this side, moved on the other.
        (
            json!({ "closed": false }),
            json!({ "closed": true }),
            json!({ "closed": false }),
            "here",
            "in",
            json!({}),
            json!({ "closed": true }),
            json!({ "closed": true }),
        ),
    ];
    let mut reached = BTreeSet::new();
    for (here, there, last, prefer, came, out, taken, agreed) in cases {
        let asked = json!({ "here": here, "there": there, "last": last, "prefer": prefer });
        let crossing = told(&handed(asked.clone())).unwrap_or_else(|why| panic!("{asked}: {why}"));
        assert_eq!(crossing.came.word(), came, "{asked}");
        assert_eq!(Value::Object(crossing.out), out, "what goes out of {asked}");
        assert_eq!(Value::Object(crossing.take), taken, "what comes in of {asked}");
        assert_eq!(Value::Object(crossing.agreed), agreed, "what is agreed of {asked}");
        assert!(!crossing.holds && !crossing.newly, "{asked}");
        reached.insert(came);
    }
    // Whose changes on the other side are taken, and what was said of
    // one that is held: what is asked, then came, out, in, agreed, and
    // whether a change is held once it is done and came to be just now.
    let agreed = json!({ "title": "A cart", "body": "Customers need one." });
    let theirs = json!({ "title": "A trolley", "body": "Customers need one." });
    let ours = json!({ "title": "A basket", "body": "Customers need one." });
    let both = json!({ "title": "A cart", "body": "They asked twice." });
    type Held = (Value, &'static str, Value, Value, Value, bool, bool);
    let held: Vec<Held> = vec![
        // Everyone's are taken, trusted or not.
        (
            json!({ "here": agreed, "there": theirs, "last": agreed, "prefer": "here", "takes": "everyone", "trusted": false }),
            "in",
            json!({}),
            json!({ "title": "A trolley" }),
            theirs.clone(),
            false,
            false,
        ),
        // Only the trusted's: one who is, as everyone's.
        (
            json!({ "here": agreed, "there": theirs, "last": agreed, "prefer": "here", "takes": "trusted", "trusted": true }),
            "in",
            json!({}),
            json!({ "title": "A trolley" }),
            theirs.clone(),
            false,
            false,
        ),
        // One who is not: it is held, from now, and nothing crosses.
        (
            json!({ "here": agreed, "there": theirs, "last": agreed, "prefer": "here", "takes": "trusted", "trusted": false }),
            "held",
            json!({}),
            json!({}),
            agreed.clone(),
            true,
            true,
        ),
        // Not even what moved here beside it: nothing of ours is
        // written over what is there while it waits.
        (
            json!({ "here": both, "there": theirs, "last": agreed, "prefer": "here", "takes": "trusted", "trusted": false }),
            "held",
            json!({}),
            json!({}),
            agreed.clone(),
            true,
            true,
        ),
        // What only goes out is not held, whoever wrote the other side.
        (
            json!({ "here": ours, "there": agreed, "last": agreed, "prefer": "here", "takes": "trusted", "trusted": false }),
            "out",
            json!({ "title": "A basket" }),
            json!({}),
            ours.clone(),
            false,
            false,
        ),
        // Nor is what both moved, where this side is the one taken.
        (
            json!({ "here": ours, "there": theirs, "last": agreed, "prefer": "here", "takes": "trusted", "trusted": false }),
            "out",
            json!({ "title": "A basket" }),
            json!({}),
            ours.clone(),
            false,
            false,
        ),
        // Where the other side is the one taken, it is.
        (
            json!({ "here": ours, "there": theirs, "last": agreed, "prefer": "there", "takes": "trusted", "trusted": false }),
            "held",
            json!({}),
            json!({}),
            agreed.clone(),
            true,
            true,
        ),
        // Still held: nothing crosses, and it is not said again.
        (
            json!({ "here": agreed, "there": theirs, "last": agreed, "prefer": "here", "takes": "trusted", "trusted": false, "held": "waiting" }),
            "held",
            json!({}),
            json!({}),
            agreed.clone(),
            true,
            false,
        ),
        // Still held though the other side is trusted by now, or
        // everyone's are taken: somebody is to say which stands.
        (
            json!({ "here": both, "there": theirs, "last": agreed, "prefer": "here", "takes": "everyone", "trusted": true, "held": "waiting" }),
            "held",
            json!({}),
            json!({}),
            agreed.clone(),
            true,
            false,
        ),
        // Held, and the two say the same again: there is nothing to
        // hold.
        (
            json!({ "here": theirs, "there": theirs, "last": agreed, "prefer": "here", "takes": "trusted", "trusted": false, "held": "waiting" }),
            "same",
            json!({}),
            json!({}),
            theirs.clone(),
            false,
            false,
        ),
        // Somebody said the other side stands: all that differs comes
        // in, whatever moved and whoever wrote it.
        (
            json!({ "here": both, "there": theirs, "last": agreed, "prefer": "here", "takes": "trusted", "trusted": false, "held": "take" }),
            "in",
            json!({}),
            json!({ "title": "A trolley", "body": "Customers need one." }),
            theirs.clone(),
            false,
            false,
        ),
        // Somebody said this side stands: all that differs goes out.
        (
            json!({ "here": both, "there": theirs, "last": agreed, "prefer": "there", "takes": "trusted", "trusted": false, "held": "keep" }),
            "out",
            json!({ "title": "A cart", "body": "They asked twice." }),
            json!({}),
            both.clone(),
            false,
            false,
        ),
        // Said of two that agree: nothing crosses, and nothing is held.
        (
            json!({ "here": agreed, "there": agreed, "last": agreed, "prefer": "here", "takes": "nobody", "held": "take" }),
            "same",
            json!({}),
            json!({}),
            agreed.clone(),
            false,
            false,
        ),
        // Nobody's are taken: what moved there alone is neither taken
        // nor written over, and what was agreed stands.
        (
            json!({ "here": agreed, "there": theirs, "last": agreed, "prefer": "there", "takes": "nobody" }),
            "same",
            json!({}),
            json!({}),
            agreed.clone(),
            false,
            false,
        ),
        // What moved here goes out, beside it.
        (
            json!({ "here": both, "there": theirs, "last": agreed, "prefer": "there", "takes": "nobody" }),
            "out",
            json!({ "body": "They asked twice." }),
            json!({}),
            both.clone(),
            false,
            false,
        ),
        // And where both moved one field, this side's, whichever side
        // was said: the other's is never taken.
        (
            json!({ "here": ours, "there": theirs, "last": agreed, "prefer": "here", "takes": "nobody" }),
            "out",
            json!({ "title": "A basket" }),
            json!({}),
            ours.clone(),
            false,
            false,
        ),
        (
            json!({ "here": ours, "there": theirs, "last": agreed, "prefer": "there", "takes": "nobody" }),
            "same",
            json!({}),
            json!({}),
            agreed.clone(),
            false,
            false,
        ),
    ];
    for (asked, came, out, taken, agreed, holds, newly) in held {
        let crossing = told(&handed(asked.clone())).unwrap_or_else(|why| panic!("{asked}: {why}"));
        assert_eq!(crossing.came.word(), came, "{asked}");
        assert_eq!(Value::Object(crossing.out), out, "what goes out of {asked}");
        assert_eq!(Value::Object(crossing.take), taken, "what comes in of {asked}");
        assert_eq!(Value::Object(crossing.agreed), agreed, "what is agreed of {asked}");
        assert_eq!((crossing.holds, crossing.newly), (holds, newly), "what is held of {asked}");
        reached.insert(came);
    }
    let all: BTreeSet<&str> = CAME.iter().copied().collect();
    assert_eq!(reached, all, "every word it can answer is reached by a case");
}

#[test]
fn done_again_with_what_was_agreed_written_down_nothing_crosses() {
    // Whatever crossed, the two sides as they then are, held against
    // what they were said to agree on, are the same.
    let starts = [
        (json!({ "title": "A basket", "closed": false }), json!({ "title": "A cart", "closed": true })),
        (json!({ "title": "A basket" }), json!({ "title": "A trolley", "body": "new" })),
        (json!({ "body": "" }), json!({ "body": "Customers need one." })),
    ];
    for (here, there) in starts {
        for prefer in ["here", "there"] {
            let last = json!({ "title": "A cart", "closed": false, "body": "Customers need one." });
            let asked = json!({ "here": here, "there": there, "last": last, "prefer": prefer });
            let first = told(&handed(asked)).unwrap();
            let (mut ours, mut theirs) = (handed(here.clone()), handed(there.clone()));
            theirs.extend(first.out.clone());
            ours.extend(first.take.clone());
            let again = json!({ "here": ours, "there": theirs, "last": first.agreed, "prefer": prefer });
            let second = told(&handed(again.clone())).unwrap();
            assert_eq!(second.came.word(), "same", "{again}");
            assert_eq!(second.agreed, first.agreed, "{again}");
        }
    }
}

#[test]
fn what_makes_no_sense_of_two_sides_is_refused_in_a_sentence() {
    let cases = [
        (
            json!({ "here": {}, "there": {} }),
            "`prefer` is which side is taken where both moved, `here` or `there`, and neither is said",
        ),
        (
            json!({ "here": {}, "there": {}, "prefer": "newest" }),
            "`prefer` is which side is taken where both moved, `here` or `there`, and is `newest`",
        ),
        (
            json!({ "there": {}, "prefer": "here" }),
            "`here` is one of the two sides, each field by its name, and none is said",
        ),
        (
            json!({ "here": {}, "prefer": "here" }),
            "`there` is one of the two sides, each field by its name, and none is said",
        ),
        (
            json!({ "here": "A cart", "there": {}, "prefer": "here" }),
            "`here` is fields by their names, and is \"A cart\"",
        ),
        (
            json!({ "here": {}, "there": {}, "last": [1], "prefer": "here" }),
            "`last` is fields by their names, and is [1]",
        ),
        (
            json!({ "here": {}, "there": {}, "prefer": "here", "takes": "friends" }),
            "`takes` is whose changes on the other side are taken, `everyone`, `trusted` or `nobody`, and is `friends`",
        ),
        (
            json!({ "here": {}, "there": {}, "prefer": "here", "takes": "trusted" }),
            "`trusted` is whether the other side as it stands was written by somebody trusted, which taking only \
             what such a one changed goes by, and it is not said",
        ),
        (
            json!({ "here": {}, "there": {}, "prefer": "here", "trusted": "yes" }),
            "`trusted` is yes or no, and is \"yes\"",
        ),
        (
            json!({ "here": {}, "there": {}, "prefer": "here", "held": "maybe" }),
            "`held` is what was said of a change that is held: nothing, `waiting`, `take` to have the other side \
             stand or `keep` to have this one; it is `maybe`",
        ),
    ];
    for (asked, why) in cases {
        assert_eq!(told(&handed(asked.clone())).unwrap_err(), why, "{asked}");
    }
}

#[test]
fn what_was_said_and_has_not_crossed_is_what_is_named_by_nothing_that_has_each_with_the_names_once_it_has() {
    // things, by, had, also, except; then came, ids, kept, and the
    // `kept` beside each of those.
    let said = json!([
        { "id": "c1", "by": "ada", "body": "First." },
        { "id": "c2", "by": "bo", "body": "Second.", "ours": true },
        { "id": "c3", "by": "cy", "body": "Third." },
        { "id": "c4", "by": "ada", "body": "Fourth.", "ours": false },
    ]);
    let cases: Vec<(Value, &str, Value, Value)> = vec![
        // Nothing has crossed: all of them, each with the names so far.
        (
            json!({ "things": said }),
            "some",
            json!(["c1", "c2", "c3", "c4"]),
            json!([["c1"], ["c1", "c2"], ["c1", "c2", "c3"], ["c1", "c2", "c3", "c4"]]),
        ),
        // Some have: the rest, with those that had kept before them.
        (
            json!({ "things": said, "had": ["c1", "c3"] }),
            "some",
            json!(["c2", "c4"]),
            json!([["c1", "c3", "c2"], ["c1", "c3", "c2", "c4"]]),
        ),
        // Those that crossed the other way are not to cross, and are not
        // kept among these.
        (
            json!({ "things": said, "had": ["c1"], "also": ["c3", "c4"] }),
            "some",
            json!(["c2"]),
            json!([["c1", "c2"]]),
        ),
        // One that says `true` in the field named is left out.
        (
            json!({ "things": said, "had": ["c1"], "except": "ours" }),
            "some",
            json!(["c3", "c4"]),
            json!([["c1", "c3"], ["c1", "c3", "c4"]]),
        ),
        // All have, or there is nothing: none.
        (json!({ "things": said, "had": ["c4", "c3", "c2", "c1"] }), "none", json!([]), json!([])),
        (json!({ "had": ["c1"] }), "none", json!([]), json!([])),
        (json!({ "things": [], "had": null }), "none", json!([]), json!([])),
        // Named by another field, and by a number.
        (
            json!({ "things": [{ "seq": 41 }, { "seq": 42 }], "by": "seq", "had": [41] }),
            "some",
            json!([42]),
            json!([[41, 42]]),
        ),
        // Said twice in what was handed, it is owed once; and a name
        // written down twice is kept once.
        (
            json!({ "things": [{ "id": "c9" }, { "id": "c9" }], "had": ["c1", "c1"] }),
            "some",
            json!(["c9"]),
            json!([["c1", "c9"]]),
        ),
    ];
    let mut reached = BTreeSet::new();
    for (asked, came, ids, kept) in cases {
        let owed = owed_told(&handed(asked.clone())).unwrap_or_else(|why| panic!("{asked}: {why}"));
        assert_eq!(owed.word(), came, "{asked}");
        assert_eq!(Value::Array(owed.names.clone()), ids, "{asked}");
        let beside: Vec<Value> = owed.those.iter().map(|thing| thing["kept"].clone()).collect();
        assert_eq!(Value::Array(beside), kept, "{asked}");
        // What is kept in the end is what the last of them says.
        if let Some(last) = owed.those.last() {
            assert_eq!(last["kept"], Value::Array(owed.kept.clone()), "{asked}");
        }
        // Each is handed on as it was said, with nothing lost.
        for thing in &owed.those {
            assert!(thing.get("id").is_some() || thing.get("seq").is_some(), "{thing}");
        }
        reached.insert(came);
    }
    let all: BTreeSet<&str> = OWED.iter().copied().collect();
    assert_eq!(reached, all, "every word it can answer is reached by a case");

    for (asked, why) in [
        (json!({ "things": "c1" }), "`things` is a list, and is \"c1\""),
        (json!({ "things": ["c1"] }), "each of `things` is fields by their names, and one is \"c1\""),
        (
            json!({ "things": [{ "body": "First." }] }),
            "each of `things` says its name in `id`, and one says none: {\"body\":\"First.\"}",
        ),
        (
            json!({ "things": [{ "id": "" }] }),
            "each of `things` says its name in `id`, and one says none: {\"id\":\"\"}",
        ),
        (json!({ "things": [], "had": "c1" }), "`had` is a list, and is \"c1\""),
    ] {
        assert_eq!(owed_told(&handed(asked.clone())).unwrap_err(), why, "{asked}");
    }
}

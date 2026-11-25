//! A pool that gives a place that is free to whoever said the highest
//! priority as it asked (`order: priority`), and of those as high to
//! whoever asked first.

use conductor::Apply;
use conductor::pool::{Came, Order, Priorities, kept_priorities, priority_of, ranked, turn_ranked};
use kernel::Source;
use serde_json::json;

use crate::handlers::active;
use crate::{Rig, soon};

const POOL: &str = include_str!("../../../../blueprints/std/pool.bp");

fn names(words: &[&str]) -> Vec<String> {
    words.iter().map(|word| word.to_string()).collect()
}

fn priorities(kept: &[(&str, f64)]) -> Priorities {
    kept.iter().map(|(who, priority)| ((*who).to_string(), *priority)).collect()
}

#[test]
fn of_those_waiting_the_highest_priority_is_given_a_place_first_and_of_those_as_high_the_first_to_ask() {
    // Who holds, who waits (in the order they asked), what each said, what
    // came, how many places; who holds and who waits after, and who was
    // given a place just now, in the order they were.
    type Case<'a> = (
        &'a [&'a str],
        &'a [&'a str],
        &'a [(&'a str, f64)],
        Came<'a>,
        i64,
        (&'a [&'a str], &'a [&'a str], &'a [&'a str]),
    );
    let cases: [Case<'_>; 9] = [
        // The higher goes first, whenever it asked.
        (&["a"], &["b", "c"], &[("b", 1.0), ("c", 5.0)], Came::GaveBack("a"), 1, (&["c"], &["b"], &["c"])),
        // Of those as high, the first to have asked.
        (&["a"], &["b", "c"], &[("b", 5.0), ("c", 5.0)], Came::GaveBack("a"), 1, (&["b"], &["c"], &["b"])),
        // One that said nothing said 0: below one that said 1, above one that said less than nothing.
        (&["a"], &["b", "c"], &[("c", 1.0)], Came::GaveBack("a"), 1, (&["c"], &["b"], &["c"])),
        (&["a"], &["b", "c"], &[("c", -2.0)], Came::GaveBack("a"), 1, (&["b"], &["c"], &["b"])),
        // One that asks with a place free is given it at once, whatever it said.
        (&[], &[], &[("a", 0.0)], Came::Asked("a"), 1, (&["a"], &[], &["a"])),
        // One that asks when none is free waits, and is ranked by what it said.
        (&["a"], &["b"], &[("b", 1.0), ("c", 3.0)], Came::Asked("c"), 1, (&["a"], &["b", "c"], &[])),
        // Several places come free at once: given in order of priority.
        (
            &[],
            &["b", "c", "d"],
            &[("b", 1.0), ("c", 3.0), ("d", 2.0)],
            Came::Nothing,
            2,
            (&["c", "d"], &["b"], &["c", "d"]),
        ),
        // One that holds a place and waits for its next is passed over,
        // however high, while it holds this one.
        (
            &["a"],
            &["a", "b"],
            &[("a", 9.0), ("b", 1.0)],
            Came::Asked("c"),
            2,
            (&["a", "b"], &["a", "c"], &["b"]),
        ),
        // Ended, it neither holds nor waits, and its place goes to the highest.
        (&["a"], &["b", "c", "a"], &[("c", 2.0)], Came::Ended("a"), 1, (&["c"], &["b"], &["c"])),
    ];
    for (held, waiting, said, came, limit, (holds, waits, given)) in cases {
        let (now_held, now_waiting, now_given) =
            turn_ranked(names(held), names(waiting), came, limit, true, Order::Priority, &priorities(said));
        assert_eq!(
            (now_held, now_waiting, now_given),
            (names(holds), names(waits), names(given)),
            "{held:?}, {waiting:?}, {said:?}, {came:?}"
        );
    }
    // While it gives none, nobody is given one, however high.
    let (held, waiting, given) = turn_ranked(
        vec![],
        names(&["b"]),
        Came::Nothing,
        1,
        false,
        Order::Priority,
        &priorities(&[("b", 9.0)]),
    );
    assert_eq!((held, waiting, given), (names(&[]), names(&["b"]), names(&[])));
    // What the other orders give is what they gave: the priorities are not theirs.
    for (order, first) in [(Order::First, "b"), (Order::Last, "d")] {
        let said = priorities(&[("b", 1.0), ("c", 9.0), ("d", 2.0)]);
        let (held, _, _) = turn_ranked(vec![], names(&["b", "c", "d"]), Came::Nothing, 1, true, order, &said);
        assert_eq!(held, names(&[first]), "{order:?}");
    }
    assert_eq!(Order::named("priority"), Ok(Order::Priority));
    assert!(Order::named("soonest").unwrap_err().contains("`priority`"));
}

#[test]
fn the_priority_each_said_is_kept_for_as_long_as_it_holds_or_waits_and_one_that_says_none_again_keeps_its_own()
 {
    // What is kept before, what came, what whoever asks says; what is ranked by.
    type Case<'a> = (&'a [(&'a str, f64)], Came<'a>, Option<f64>, &'a [(&'a str, f64)]);
    let cases: [Case<'_>; 5] = [
        (&[], Came::Asked("a"), Some(3.0), &[("a", 3.0)]),
        (&[("a", 3.0)], Came::Asked("a"), None, &[("a", 3.0)]),
        (&[("a", 3.0)], Came::Asked("a"), Some(1.0), &[("a", 1.0)]),
        // Only whoever asks says one; giving back says nothing of it.
        (&[("a", 3.0)], Came::GaveBack("a"), Some(7.0), &[("a", 3.0)]),
        (&[("a", 3.0)], Came::Nothing, Some(7.0), &[("a", 3.0)]),
    ];
    for (before, came, says, after) in cases {
        assert_eq!(
            ranked(priorities(before), came, says),
            priorities(after),
            "{before:?}, {came:?}, {says:?}"
        );
    }
    // After the turn, of whoever holds a place or waits for one and nobody else.
    let kept =
        kept_priorities(priorities(&[("a", 1.0), ("b", 2.0), ("c", 3.0)]), &names(&["a"]), &names(&["c"]));
    assert_eq!(kept, priorities(&[("a", 1.0), ("c", 3.0)]));
    assert_eq!((priority_of(&kept, "c"), priority_of(&kept, "b")), (3.0, 0.0));
}

/// Something that looks after many things, and asks for a place for each
/// by its name and with the priority the test says.
const ASKER: &str = "\
blueprint: asker
on:
  ask:
    event: asker.ask
    do: emit
    with: { topic: pool.asked, pool: jobs, as: \"${event.data.name}\", priority: \"${event.data.priority}\" }

  give_back:
    event: asker.done
    do: emit
    with: { topic: pool.released, pool: jobs, as: \"${event.data.name}\" }
";

#[tokio::test]
async fn a_pool_told_to_go_by_priority_gives_the_next_place_to_the_highest_of_those_waiting() {
    let rig = Rig::new().await;
    let pool = Apply {
        key: Some("jobs".into()),
        ..rig.request(POOL, json!({ "name": "jobs", "limit": 1, "order": "priority" }))
    };
    let pool = rig.engine.apply(pool).await.unwrap_or_else(|err| panic!("{}", err.message));
    active(&rig, &pool.instance).await;
    let asker = Apply { key: Some("asker".into()), ..rig.request(ASKER, json!({})) };
    let asker = rig.engine.apply(asker).await.unwrap_or_else(|err| panic!("{}", err.message));
    active(&rig, &asker.instance).await;
    let say = |topic: &'static str, name: &'static str, priority: &'static str| {
        let journal = rig.journal.clone();
        async move {
            let data = json!({ "name": name, "priority": priority });
            journal.emit(topic, Source::default(), data).await.unwrap();
        }
    };
    let granted = |name: &'static str| rig.event("pool.granted", move |event| event.data["as"] == name);
    let waits = |name: &'static str| {
        rig.event("pool.waiting", move |event| event.data["to"] == format!("key:{name}"))
    };

    // The first holds the place; then one of priority 1 and one of 5 wait,
    // the lower having asked first, and one that says none (0) after them.
    say("asker.ask", "first", "2").await;
    granted("first").await;
    for (name, priority) in [("low", "1"), ("high", "5"), ("none", "")] {
        say("asker.ask", name, priority).await;
        waits(name).await;
    }
    let state = rig.instance(&pool.instance).await.state;
    assert_eq!(state["priorities"], json!({ "key:first": 2.0, "key:low": 1.0, "key:high": 5.0 }));

    // Each place given back goes to the highest of those still waiting.
    let mut order = Vec::new();
    for (done, next) in [("first", "high"), ("high", "low"), ("low", "none")] {
        say("asker.done", done, "").await;
        soon(next, granted(next)).await;
        order.push(next);
    }
    assert_eq!(order, ["high", "low", "none"]);
    let state = rig.instance(&pool.instance).await.state;
    assert_eq!((state["held"].clone(), state["priorities"].clone()), (json!(["key:none"]), json!({})));
}

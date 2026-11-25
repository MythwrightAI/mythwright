//! `std/pool`: a limited number work at once, the rest wait their turn in
//! the order they asked, and a place is never held by something that is
//! gone.

use std::collections::HashMap;

use conductor::Apply;
use kernel::Source;
use serde_json::{Value, json};

use crate::handlers::active;
use crate::{Rig, soon};

const POOL: &str = include_str!("../../../../blueprints/std/pool.bp");

/// Something that asks the pool as soon as it is there, says when it is
/// told it may begin and when it is told it waits, and gives its place
/// back, or asks for another, when the test tells it to.
const WORKER: &str = "\
blueprint: worker
on:
  start:
    event: instance.active
    scope: own
    when: ${ event.source.instance == instance.id }
    do: emit
    with: { topic: pool.asked, pool: jobs }

  go:
    event: pool.granted
    for: { to: \"${instance.id}\" }
    do: emit
    with: { topic: worker.began, who: \"${instance.key}\" }

  waits:
    event: pool.waiting
    for: { to: \"${instance.id}\" }
    do: emit
    with: { topic: worker.waits, who: \"${instance.key}\", limit: ${event.data.limit}, why: \"${event.data.why}\" }

  done:
    event: worker.finish
    for: { who: \"${instance.key}\" }
    do: emit
    with: { topic: pool.released, pool: jobs }

  more:
    event: worker.more
    for: { who: \"${instance.key}\" }
    do: emit
    with: { topic: pool.asked, pool: jobs }
";

struct Floor {
    rig: Rig,
    pool: String,
    /// Each worker's key, by the instance it is.
    names: HashMap<String, String>,
}

impl Floor {
    async fn with(limit: u64) -> Floor {
        let rig = Rig::new().await;
        let pool = Floor::pool(&rig, limit).await;
        Floor { rig, pool, names: HashMap::new() }
    }

    async fn pool(rig: &Rig, limit: u64) -> String {
        Floor::pool_told(rig, json!({ "name": "jobs", "limit": limit })).await
    }

    async fn pool_told(rig: &Rig, params: Value) -> String {
        let asked = Apply { key: Some("jobs".into()), ..rig.request(POOL, params) };
        let applied = rig.engine.apply(asked).await.unwrap_or_else(|err| panic!("{}", err.message));
        active(rig, &applied.instance).await;
        applied.instance
    }

    /// Starts a worker, which asks at once.
    async fn worker(&mut self, key: &str) -> String {
        let asked = Apply { key: Some(key.into()), ..self.rig.request(WORKER, json!({})) };
        let applied = self.rig.engine.apply(asked).await.unwrap_or_else(|err| panic!("{}", err.message));
        self.names.insert(applied.instance.clone(), key.to_string());
        applied.instance
    }

    async fn say(&self, topic: &str, data: Value) {
        self.rig.journal.emit(topic, Source::default(), data).await.unwrap();
    }

    /// Waits until `who` has been told it may begin, this many times.
    async fn began(&self, who: &str, times: usize) {
        let seen = std::cell::Cell::new(0);
        self.rig
            .event("worker.began", |event| {
                seen.set(seen.get() + usize::from(event.data["who"] == who));
                seen.get() == times
            })
            .await;
    }

    /// Waits until `who` has been told it waits, and gives why.
    async fn waits(&self, who: &str) -> Value {
        self.rig.event("worker.waits", |event| event.data["who"] == who).await.data.clone()
    }

    /// Who has been told it may begin, in the order they were.
    async fn order(&self) -> Vec<String> {
        let began = self.rig.events("worker.began").await;
        began.iter().map(|event| event.data["who"].as_str().unwrap().to_string()).collect()
    }

    /// Who holds a place and who waits, by their keys.
    async fn stands(&self) -> (Vec<String>, Vec<String>) {
        let state = self.rig.instance(&self.pool).await.state;
        let named = |list: &Value| -> Vec<String> {
            let ids = list.as_array().unwrap().iter();
            ids.map(|id| self.names[id.as_str().unwrap()].clone()).collect()
        };
        (named(&state["held"]), named(&state["waiting"]))
    }
}

#[tokio::test]
async fn a_pool_gives_as_many_places_as_its_limit_and_the_rest_wait_their_turn_in_the_order_they_asked() {
    let mut floor = Floor::with(2).await;
    for who in ["a", "b", "c", "d"] {
        floor.worker(who).await;
        // Each has been answered before the next asks: a place, or that it waits.
        match who {
            "a" | "b" => floor.began(who, 1).await,
            _ => assert_eq!(floor.waits(who).await, json!({ "who": who, "limit": 2, "why": "" })),
        }
    }
    assert_eq!(floor.order().await, ["a", "b"], "two at once and no more");
    assert_eq!(floor.stands().await, (vec!["a".into(), "b".into()], vec!["c".into(), "d".into()]));

    // A place given back goes to whoever has waited longest.
    floor.say("worker.finish", json!({ "who": "b" })).await;
    floor.began("c", 1).await;
    assert_eq!(floor.stands().await, (vec!["a".into(), "c".into()], vec!["d".into()]));
    floor.say("worker.finish", json!({ "who": "a" })).await;
    floor.began("d", 1).await;
    assert_eq!(floor.order().await, ["a", "b", "c", "d"]);
    assert_eq!(floor.stands().await, (vec!["c".into(), "d".into()], vec![]));

    // What nothing that can end asked for is given to nobody.
    floor.say("pool.asked", json!({ "pool": "jobs" })).await;
    floor.say("worker.finish", json!({ "who": "c" })).await;
    crate::handlers::handled(&floor.rig, "on.released", 3).await;
    assert_eq!(floor.rig.instance(&floor.pool).await.state["waiting"], json!([]));
}

#[tokio::test]
async fn a_place_is_free_again_when_whoever_held_it_ends_and_one_that_ends_waiting_waits_no_longer() {
    let mut floor = Floor::with(1).await;
    let a = floor.worker("a").await;
    floor.began("a", 1).await;
    let b = floor.worker("b").await;
    floor.waits("b").await;
    floor.worker("c").await;
    floor.waits("c").await;

    // The one that waited next is stopped: it is not given a place it
    // could do nothing with.
    floor.rig.engine.stop(&b).await.unwrap();
    // The one that held the place is stopped without giving it back.
    floor.rig.engine.stop(&a).await.unwrap();
    floor.began("c", 1).await;
    assert_eq!(floor.order().await, ["a", "c"]);
    assert_eq!(floor.stands().await, (vec!["c".into()], vec![]));
}

#[tokio::test]
async fn one_that_asks_again_while_it_holds_a_place_holds_nobody_up_and_has_its_next_when_it_gave_this_back()
{
    let mut floor = Floor::with(2).await;
    floor.worker("a").await;
    floor.began("a", 1).await;
    // It asks for its next turn before it has given this one back.
    floor.say("worker.more", json!({ "who": "a" })).await;
    crate::handlers::handled(&floor.rig, "on.asked", 2).await;
    assert_eq!(floor.order().await, ["a"], "it does not hold two places");

    // The next to ask is not held up behind it: there is a place.
    floor.worker("b").await;
    floor.began("b", 1).await;
    // Given back, the place is its own again, since nobody else waits.
    floor.say("worker.finish", json!({ "who": "a" })).await;
    floor.began("a", 2).await;
    assert_eq!(floor.order().await, ["a", "b", "a"]);
    assert_eq!(floor.stands().await, (vec!["b".into(), "a".into()], vec![]));
}

#[tokio::test]
async fn a_pool_told_to_wait_gives_no_places_and_says_why_and_gives_them_out_when_it_goes_on() {
    let mut floor = Floor::with(2).await;
    floor.worker("a").await;
    floor.began("a", 1).await;
    floor.say("pool.pause", json!({ "pool": "jobs", "why": "the usage limit was reached" })).await;
    crate::handlers::handled(&floor.rig, "on.pause", 1).await;

    // There is a place, and nobody is given it: each is told why it waits.
    for who in ["b", "c", "d"] {
        floor.worker(who).await;
        assert_eq!(floor.waits(who).await["why"], "the usage limit was reached");
    }
    assert_eq!(floor.order().await, ["a"], "what was being done goes on, and nothing new begins");

    // Going on, it gives the one place there is, and tells the others what
    // they wait for now: a place, and no longer a reason.
    floor.say("pool.resume", json!({ "pool": "jobs" })).await;
    floor.began("b", 1).await;
    let now =
        floor.rig.event("worker.waits", |event| event.data["who"] == "d" && event.data["why"] == "").await;
    assert_eq!(now.data["limit"], 2);
    assert_eq!(floor.order().await, ["a", "b"]);
    assert_eq!(floor.stands().await, (vec!["a".into(), "b".into()], vec!["c".into(), "d".into()]));

    // Paused while some wait: they are told then, too, each of them. Which
    // of the two says so first is theirs to settle, and not the pool's.
    floor.say("pool.pause", json!({ "pool": "jobs" })).await;
    for who in ["c", "d"] {
        floor
            .rig
            .event("worker.waits", move |event| {
                event.data["who"] == who && event.data["why"] == "it was paused"
            })
            .await;
    }

    // And each time, it is said to everyone, for whatever shows what waits.
    let said = floor.rig.events("pool.paused,pool.open").await;
    let said: Vec<(&str, &Value)> = said.iter().map(|event| (event.topic.as_str(), &event.data)).collect();
    assert_eq!(
        said,
        [
            ("pool.paused", &json!({ "pool": "jobs", "why": "the usage limit was reached" })),
            ("pool.open", &json!({ "pool": "jobs", "limit": 2 })),
            ("pool.paused", &json!({ "pool": "jobs", "why": "it was paused" })),
        ]
    );
}

#[tokio::test]
async fn applied_again_with_more_places_a_pool_gives_them_to_those_waiting_and_keeps_who_held_what() {
    let mut floor = Floor::with(1).await;
    for who in ["a", "b", "c", "d"] {
        floor.worker(who).await;
        match who {
            "a" => floor.began(who, 1).await,
            _ => drop(floor.waits(who).await),
        }
    }
    // The same pool, by its key, with three places where it had one.
    assert_eq!(Floor::pool(&floor.rig, 3).await, floor.pool);
    floor.began("b", 1).await;
    floor.began("c", 1).await;
    let still =
        floor.rig.event("worker.waits", |event| event.data["who"] == "d" && event.data["limit"] == 3).await;
    assert_eq!(still.data["why"], "");
    // The two were given their places in one go. Which of them says first
    // that it began is theirs to settle, and not the pool's: the order the
    // pool gave the places in is the order it keeps them in.
    let mut began = floor.order().await;
    assert_eq!(began.remove(0), "a");
    began.sort();
    assert_eq!(began, ["b", "c"]);
    assert_eq!(floor.stands().await, (vec!["a".into(), "b".into(), "c".into()], vec!["d".into()]));
}

#[tokio::test]
async fn a_pool_is_as_it_was_after_a_restart_and_goes_on_giving_places() {
    let mut floor = Floor::with(1).await;
    floor.worker("a").await;
    floor.began("a", 1).await;
    floor.worker("b").await;
    floor.waits("b").await;

    floor.rig.restart().await;

    assert_eq!(floor.stands().await, (vec!["a".into()], vec!["b".into()]));
    floor.say("worker.finish", json!({ "who": "a" })).await;
    floor.began("b", 1).await;
    assert_eq!(floor.order().await, ["a", "b"]);
}

/// One piece of work as a blueprint of its own: it asks as it begins to
/// wait, works when it has its place, and ends.
const JOB: &str = "\
blueprint: job
params: { name: { type: string } }
steps:
  place:
    do: wait
    with:
      event: pool.granted
      where: event.data.to == instance.id
      say: { topic: pool.asked, pool: jobs, about: \"${params.name}\" }

  work: { needs: [place], do: test.gate, with: { name: \"${params.name}\" } }
outputs:
  waited: ${ steps.place.out.event.data.waited }
";

#[tokio::test]
async fn work_that_is_an_instance_of_its_own_asks_as_it_waits_and_its_place_is_free_when_it_ends() {
    let floor = Floor::with(1).await;
    let job =
        |name: &str| Apply { key: Some(name.to_string()), ..floor.rig.request(JOB, json!({ "name": name })) };
    let first = floor.rig.engine.apply(job("x")).await.unwrap();
    floor.rig.reached("x").await;
    let second = floor.rig.engine.apply(job("y")).await.unwrap();
    // The second is told that it waits, with what it said it was about.
    let waits = floor.rig.event("pool.waiting", |_| true).await;
    assert_eq!(
        waits.data,
        // Whose pool it is, is said too: nobody's, where nobody said.
        json!({ "pool": "jobs", "project": "", "to": second.instance, "about": "y", "limit": 1, "why": "" })
    );

    // The first ends, having given nothing back, and the second has its place.
    floor.rig.open("x");
    let run = floor.rig.finished(&first).await;
    assert_eq!(run.outputs["waited"], false);
    floor.rig.reached("y").await;
    floor.rig.open("y");
    let run = floor.rig.finished(&second).await;
    assert_eq!(run.outputs["waited"], true);
    // And with both gone the pool holds nothing.
    crate::handlers::handled(&floor.rig, "on.ended", 2).await;
    let state = floor.rig.instance(&floor.pool).await.state;
    assert_eq!((&state["held"], &state["waiting"]), (&json!([]), &json!([])));
}

#[tokio::test]
async fn a_pool_told_to_gives_a_place_to_whoever_asked_last() {
    let rig = Rig::new().await;
    let pool = Floor::pool_told(&rig, json!({ "name": "jobs", "limit": 1, "order": "last" })).await;
    let mut floor = Floor { rig, pool, names: HashMap::new() };
    floor.worker("a").await;
    floor.began("a", 1).await;
    for who in ["b", "c"] {
        floor.worker(who).await;
        floor.waits(who).await;
    }
    floor.say("worker.finish", json!({ "who": "a" })).await;
    floor.began("c", 1).await;
    assert_eq!(floor.stands().await, (vec!["c".into()], vec!["b".into()]));
}

fn names(words: &[&str]) -> Vec<String> {
    words.iter().map(|word| word.to_string()).collect()
}

#[test]
fn who_is_given_a_place_is_worked_out_from_who_holds_one_who_waits_and_what_has_just_happened() {
    use conductor::pool::{Came, Order, turn};
    // One asks where there is a place, and one where there is none.
    let (held, waiting, given) = turn(vec![], vec![], Came::Asked("a"), 1, true, Order::First);
    assert_eq!((held.clone(), waiting.clone(), given), (names(&["a"]), names(&[]), names(&["a"])));
    let (held, waiting, given) = turn(held, waiting, Came::Asked("b"), 1, true, Order::First);
    assert_eq!((held.clone(), waiting.clone(), given), (names(&["a"]), names(&["b"]), names(&[])));
    // Asking twice is waiting once.
    let (held, waiting, _) = turn(held, waiting, Came::Asked("b"), 1, true, Order::First);
    assert_eq!(waiting, names(&["b"]));
    // One that asks while it holds a place waits for its next, and is not
    // given the place it gives back before whoever waited longer.
    let (held, waiting, given) = turn(held, waiting, Came::Asked("a"), 2, true, Order::First);
    assert_eq!((held.clone(), waiting.clone(), given), (names(&["a", "b"]), names(&["a"]), names(&["b"])));
    let (held, waiting, given) = turn(held, waiting, Came::GaveBack("a"), 2, true, Order::First);
    assert_eq!((held, waiting, given), (names(&["b", "a"]), names(&[]), names(&["a"])));

    // Ended, it neither holds nor waits, and its place goes to the next:
    // the first to have asked, or the last where that is the order.
    let stands = || (names(&["a"]), names(&["b", "c", "a"]));
    let (held, waiting, given) = turn(stands().0, stands().1, Came::Ended("a"), 1, true, Order::First);
    assert_eq!((held, waiting, given), (names(&["b"]), names(&["c"]), names(&["b"])));
    let (held, waiting, given) = turn(stands().0, stands().1, Came::Ended("a"), 1, true, Order::Last);
    assert_eq!((held, waiting, given), (names(&["c"]), names(&["b"]), names(&["c"])));

    // While it gives none, nobody is given one; when it goes on, as many
    // as there are places for, and with no limit all who wait.
    let (held, waiting, given) = turn(vec![], names(&["a", "b", "c"]), Came::Nothing, 2, false, Order::First);
    assert_eq!((held, waiting.clone(), given), (names(&[]), names(&["a", "b", "c"]), names(&[])));
    let (held, left, given) = turn(vec![], waiting.clone(), Came::Nothing, 2, true, Order::First);
    assert_eq!((held, left, given), (names(&["a", "b"]), names(&["c"]), names(&["a", "b"])));
    let (held, left, _) = turn(vec![], waiting, Came::Nothing, 0, true, Order::First);
    assert_eq!((held, left), (names(&["a", "b", "c"]), names(&[])));
    assert!(Order::named("soonest").unwrap_err().contains("`first` or `last`"));
}

#[tokio::test]
async fn what_is_gone_holds_no_place_and_is_given_none_whether_or_not_the_pool_heard_it_end() {
    // Something asked and was gone before the pool heard it ask, as when a
    // conductor is stopped between the two: looked for, it is not there.
    let rig = Rig::new().await;
    let source = "\
blueprint: looks
steps:
  turn:
    do: pool.turn
    with:
      held: [in_gone]
      waiting: [in_gone_too, \"${instance.id}\"]
      asks: in_never_was
      limit: 1
";
    let run = rig.run(source, json!({})).await;
    let turn = &run.steps[0].output;
    let me = run.instance.clone();
    assert_eq!(
        (turn["held"].clone(), turn["waiting"].clone(), turn["given"].clone()),
        (json!([me]), json!([]), json!([me]))
    );
}

#[test]
fn whoever_asks_or_gives_back_is_the_name_it_does_so_as_or_else_the_instance_it_is() {
    use conductor::pool::{name_of, who};
    type Said<'a> = (Option<&'a str>, Option<&'a str>);
    let cases: [(Said<'_>, Option<&str>); 8] = [
        // By an instance, with no name said or an empty one.
        ((Some("in_7"), None), Some("in_7")),
        ((Some("in_7"), Some("")), Some("in_7")),
        // By a name: it is the name that asks, whatever instance says so,
        // and where nothing that is an instance does (said from outside).
        ((Some("in_7"), Some("shop-12")), Some("key:shop-12")),
        ((None, Some("shop-12")), Some("key:shop-12")),
        ((Some(""), Some("shop-12")), Some("key:shop-12")),
        // Nobody: nothing said at all, and what says nothing.
        ((None, None), None),
        ((Some(""), None), None),
        ((Some(""), Some("")), None),
    ];
    for ((instance, named), comes_to) in cases {
        assert_eq!(who(instance, named).as_deref(), comes_to, "{instance:?} as {named:?}");
    }
    // A pool from before wrote the name itself, as it is kept: handed in
    // place of an instance, it is taken as it is.
    assert_eq!(who(Some("key:shop-12"), None).as_deref(), Some("key:shop-12"));

    // The name one that holds or waits asked as; nothing, for an instance.
    for (kept, name) in [
        ("key:shop-12", "shop-12"),
        ("in_7", ""),
        ("", ""),
        // Only what stands first says it is a name, and the rest is the name as it was asked.
        ("key:review-key:3", "review-key:3"),
        ("in_key:7", ""),
        ("key:", ""),
    ] {
        assert_eq!(name_of(kept), name, "`{kept}`");
    }
}

#[test]
fn what_a_pool_says_of_a_turn_is_who_was_given_a_place_and_who_waits_each_with_its_name() {
    use conductor::pool::{Abouts, Came, Told, told};
    // Given, waiting and held as they stand after the turn, and what it
    // came of; nobody having said what it asked about.
    let said = |given: &[&str], waiting: &[&str], held: &[&str], came: Came<'_>| {
        told(&names(given), &names(waiting), &names(held), came, &Abouts::new(), &[])
    };
    // Nothing happened to anybody.
    let nothing = Told { granted: vec![], waiters: vec![], waits: false, about: String::new() };
    assert_eq!(said(&[], &[], &[], Came::Nothing), nothing);
    // The one that has just asked is given a place and did not wait.
    assert_eq!(
        said(&["in_7"], &[], &["in_7"], Came::Asked("in_7")),
        Told {
            granted: vec![json!({ "to": "in_7", "as": "", "waited": false })],
            waiters: vec![],
            waits: false,
            about: String::new()
        }
    );
    // Asked by a name, it is told by that name.
    assert_eq!(
        said(&["key:shop-12"], &[], &["key:shop-12"], Came::Asked("key:shop-12")).granted,
        [json!({ "to": "key:shop-12", "as": "shop-12", "waited": false })]
    );
    // The one that has just asked waits, where there is no place; those before it are not it.
    let full = said(&[], &["in_1", "key:shop-12"], &["in_0"], Came::Asked("key:shop-12"));
    assert_eq!(
        (full.waits, full.waiters),
        (
            true,
            vec![json!({ "to": "in_1", "about": "" }), json!({ "to": "key:shop-12", "about": "shop-12" })]
        )
    );
    // A place given back goes to one that waited; whoever gave it back is not among them.
    let next = said(&["key:shop-3", "in_2"], &["in_5"], &["key:shop-3", "in_2"], Came::GaveBack("in_0"));
    assert_eq!(
        next.granted,
        [
            json!({ "to": "key:shop-3", "as": "shop-3", "waited": true }),
            json!({ "to": "in_2", "as": "", "waited": true })
        ]
    );
    assert_eq!((next.waits, next.waiters), (false, vec![json!({ "to": "in_5", "about": "" })]));
    // One asks while another is given the place it gave back: only the other waited.
    let both = said(&["in_1", "in_9"], &[], &["in_1", "in_9"], Came::Asked("in_9"));
    assert_eq!(
        both.granted,
        [
            json!({ "to": "in_1", "as": "", "waited": true }),
            json!({ "to": "in_9", "as": "", "waited": false })
        ]
    );
    // One that waited, holding nothing, asks again and is given a place
    // in that turn (a name said to be due twice, the second asking come
    // as the pool went on after a pause): it waited, and is told its
    // turn came, or what it is about would go on saying that it waits.
    let asked_again = told(
        &names(&["key:shop-12"]),
        &[],
        &names(&["key:shop-12"]),
        Came::Asked("key:shop-12"),
        &Abouts::new(),
        &names(&["key:shop-12"]),
    );
    assert_eq!(asked_again.granted, [json!({ "to": "key:shop-12", "as": "shop-12", "waited": true })]);
    // One that asked and is given nothing because it is not among those waiting either (it was gone) does not wait.
    assert!(!said(&[], &["in_1"], &[], Came::Asked("in_gone")).waits);

    // A name asked again while it holds its place waits for its next, and
    // is kept among those waiting; it is at work, and is not said to wait,
    // to itself or among those who do.
    let again = said(&[], &["key:shop-12"], &["key:shop-12"], Came::Asked("key:shop-12"));
    assert_eq!(again, Told { granted: vec![], waiters: vec![], waits: false, about: "shop-12".into() });
    // The same of an instance, with somebody else who does wait.
    let beside = said(&[], &["in_5", "in_7"], &["in_7"], Came::Asked("in_7"));
    assert_eq!((beside.waits, beside.waiters), (false, vec![json!({ "to": "in_5", "about": "" })]));
    // It gives its place back and has its next at once, nobody else waiting: it did not wait.
    let own = said(&["key:shop-12"], &[], &["key:shop-12"], Came::GaveBack("key:shop-12"));
    assert_eq!(
        own,
        Told {
            granted: vec![json!({ "to": "key:shop-12", "as": "shop-12", "waited": false })],
            waiters: vec![],
            waits: false,
            about: "shop-12".into()
        }
    );
    // It gives its place back and somebody who asked before it has the
    // place: now it waits, and is told so, which it was not while it held.
    let behind = said(&["in_5"], &["key:shop-12"], &["in_5"], Came::GaveBack("key:shop-12"));
    assert_eq!(behind.granted, [json!({ "to": "in_5", "as": "", "waited": true })]);
    assert_eq!(
        (behind.waits, behind.waiters),
        (true, vec![json!({ "to": "key:shop-12", "about": "shop-12" })])
    );
    // One that ended is told nothing, whoever is left.
    let ended = said(&["in_5"], &["key:shop-12"], &["in_5", "key:shop-12"], Came::Ended("in_0"));
    assert_eq!((ended.waits, ended.waiters), (false, vec![]));
}

#[test]
fn what_whoever_waits_asked_about_is_kept_with_it_and_said_when_it_is_told_that_it_waits() {
    use conductor::pool::{Abouts, Came, about_of, kept_about, told};
    let abouts = |kept: &[(&str, &str)]| -> Abouts {
        kept.iter().map(|(who, about)| ((*who).to_string(), (*about).to_string())).collect()
    };
    // What is kept before, what came, what whoever asks says it is about,
    // who then holds and who waits; and what is kept after.
    type Case<'a> = (
        &'a [(&'a str, &'a str)],
        Came<'a>,
        Option<&'a str>,
        &'a [&'a str],
        &'a [&'a str],
        &'a [(&'a str, &'a str)],
    );
    let cases: [Case<'_>; 9] = [
        // One that asks and says what about is kept with it, holding or waiting.
        (&[], Came::Asked("in_7"), Some("scene 4"), &["in_7"], &[], &[("in_7", "scene 4")]),
        (
            &[("in_1", "scene 1")],
            Came::Asked("in_7"),
            Some("scene 4"),
            &["in_1"],
            &["in_7"],
            &[("in_1", "scene 1"), ("in_7", "scene 4")],
        ),
        // One that says nothing of what about has nothing kept.
        (&[], Came::Asked("in_7"), None, &["in_7"], &[], &[]),
        (&[], Came::Asked("in_7"), Some(""), &["in_7"], &[], &[]),
        // Asked again while it holds, saying nothing: it is about what it was. Saying another thing: that.
        (&[("in_7", "scene 4")], Came::Asked("in_7"), None, &["in_7"], &["in_7"], &[("in_7", "scene 4")]),
        (
            &[("in_7", "scene 4")],
            Came::Asked("in_7"),
            Some("scene 5"),
            &["in_7"],
            &["in_7"],
            &[("in_7", "scene 5")],
        ),
        // It gives its place back and still waits for its next: kept. It gives it back and is gone: not.
        (&[("in_7", "scene 4")], Came::GaveBack("in_7"), None, &["in_5"], &["in_7"], &[("in_7", "scene 4")]),
        (
            &[("in_7", "scene 4"), ("in_5", "scene 2")],
            Came::GaveBack("in_7"),
            None,
            &["in_5"],
            &[],
            &[("in_5", "scene 2")],
        ),
        // One that ended is forgotten; and one kept that neither holds nor waits any more, whatever came.
        (&[("in_7", "scene 4"), ("in_gone", "scene 9")], Came::Ended("in_7"), None, &[], &[], &[]),
    ];
    for (before, came, says, held, waiting, after) in cases {
        let kept = kept_about(abouts(before), came, says, &names(held), &names(waiting));
        assert_eq!(kept, abouts(after), "{before:?}, {came:?}, {says:?}");
    }

    // What it asked about is what it said, and otherwise the name it asked as; nothing for an instance that said nothing.
    let kept = abouts(&[("in_7", "scene 4"), ("key:shop-3", "the third card")]);
    for (who, about) in
        [("in_7", "scene 4"), ("key:shop-3", "the third card"), ("key:shop-9", "shop-9"), ("in_2", "")]
    {
        assert_eq!(about_of(&kept, who), about, "{who}");
    }

    // An instance that asked for its next while it held its place, gave it
    // back, and waits behind another: it is told so about what it asked
    // about when it asked, which nothing says at this moment but what was kept.
    let behind =
        told(&names(&["in_5"]), &names(&["in_7"]), &names(&["in_5"]), Came::GaveBack("in_7"), &kept, &[]);
    assert_eq!((behind.waits, behind.about.as_str()), (true, "scene 4"));
    assert_eq!(behind.waiters, [json!({ "to": "in_7", "about": "scene 4" })]);
    // Those told when a pool pauses or goes on are each told about their own.
    let all =
        told(&[], &names(&["in_7", "key:shop-3", "key:shop-9", "in_2"]), &[], Came::Nothing, &kept, &[]);
    assert_eq!(
        all.waiters,
        [
            json!({ "to": "in_7", "about": "scene 4" }),
            json!({ "to": "key:shop-3", "about": "the third card" }),
            json!({ "to": "key:shop-9", "about": "shop-9" }),
            json!({ "to": "in_2", "about": "" }),
        ]
    );
    assert_eq!(all.about, "");
}

#[tokio::test]
async fn a_pool_from_before_hands_the_names_it_kept_and_is_answered_as_one_that_says_the_name_apart() {
    // What a pool kept before its text said `asks_as`: names written as it
    // keeps them, among instances, with what it was paused for. The turn
    // is the same whichever way who asks is said.
    let rig = Rig::new().await;
    let source = |asks: &str| {
        format!(
            "\
blueprint: kept
steps:
  turn:
    do: pool.turn
    with:
      held: [\"key:shop-1\", in_gone]
      waiting: [\"key:shop-2\", \"${{instance.id}}\"]
      paused: \"\"
      limit: 2
      order: first
{asks}
"
        )
    };
    let before = rig.run(&source("      asks: \"key:shop-3\""), json!({})).await;
    let now = rig.run(&source("      asks: in_whoever\n      asks_as: shop-3"), json!({})).await;
    for run in [&before, &now] {
        let turn = &run.steps[0].output;
        // The one that is gone holds nothing; the first to wait is given
        // its place; the rest wait, the one that has just asked last.
        assert_eq!(turn["held"], json!(["key:shop-1", "key:shop-2"]), "{:?}", run.error);
        assert_eq!(turn["waiting"], json!([run.instance, "key:shop-3"]));
        assert_eq!(turn["given"], json!(["key:shop-2"]));
        assert_eq!(turn["who"], "key:shop-3");
        assert_eq!(turn["granted"], json!([{ "to": "key:shop-2", "as": "shop-2", "waited": true }]));
        assert_eq!(
            turn["waiters"],
            json!([{ "to": run.instance, "about": "" }, { "to": "key:shop-3", "about": "shop-3" }])
        );
        assert_eq!(turn["waits"], true);
    }

    // Given back by a name, said either way; and with nothing said of who, nothing happened.
    let gave = "blueprint: gave\nsteps:\n  turn:\n    do: pool.turn\n    with: { held: [\"key:shop-1\"], waiting: [], \
                gives_back: in_whoever, gives_back_as: shop-1 }\n";
    let run = rig.run(gave, json!({})).await;
    assert_eq!(
        (run.steps[0].output["held"].clone(), run.steps[0].output["who"].clone()),
        (json!([]), json!("key:shop-1"))
    );
    let paused = "blueprint: paused\nsteps:\n  turn:\n    do: pool.turn\n    with: { held: [], waiting: [\"key:shop-1\"], \
                  paused: \"it was paused\", limit: 1 }\n";
    let run = rig.run(paused, json!({})).await;
    let turn = &run.steps[0].output;
    assert_eq!(
        (turn["given"].clone(), turn["waiters"].clone(), turn["who"].clone(), turn["waits"].clone()),
        (json!([]), json!([{ "to": "key:shop-1", "about": "shop-1" }]), json!(""), json!(false))
    );
    // Two things said at once are refused, as they always were.
    let two = "blueprint: two\nsteps:\n  turn:\n    do: pool.turn\n    with: { held: [], waiting: [], asks_as: a, \
               gives_back_as: b }\n";
    let run = rig.run(two, json!({})).await;
    assert!(run.error.unwrap_or_default().contains("one thing happens at a time"));
}

/// Something that looks after many things and asks the pool for each by
/// its name, as it is told to: for a place, and to give one back.
const KEEPER: &str = "\
blueprint: keeper
on:
  ask:
    event: keeper.ask
    do: emit
    with: { topic: pool.asked, pool: jobs, as: \"${event.data.name}\", about: \"${event.data.name}\" }

  back:
    event: keeper.back
    do: emit
    with: { topic: pool.released, pool: jobs, as: \"${event.data.name}\" }
";

/// What the pool has said to whom, in order: each `pool.granted` as the
/// name and whether it waited, and each `pool.waiting` as what it is about.
async fn said_to_names(rig: &Rig) -> Vec<String> {
    let said = rig.events("pool.granted,pool.waiting").await;
    let line = |event: &kernel::Event| match event.topic.as_str() {
        "pool.granted" => {
            format!("{} may begin, waited {}", event.data["as"].as_str().unwrap(), event.data["waited"])
        }
        _ => format!("{} waits", event.data["about"].as_str().unwrap()),
    };
    said.iter().map(line).collect()
}

#[tokio::test]
async fn a_name_asked_again_while_it_holds_its_place_is_not_told_it_waits_until_it_has_given_that_back() {
    let floor = Floor::with(1).await;
    let keeper = floor.rig.apply(KEEPER, json!({})).await;
    active(&floor.rig, &keeper.instance).await;
    let ask = |name: &'static str| floor.say("keeper.ask", json!({ "name": name }));
    let back = |name: &'static str| floor.say("keeper.back", json!({ "name": name }));

    // Asked for, it has the one place. Asked for again while it holds it
    // (two things each found it due), it waits for its next, and nobody
    // is told that it waits: it is at work.
    ask("shop-1").await;
    ask("shop-1").await;
    crate::handlers::handled(&floor.rig, "on.asked", 2).await;
    assert_eq!(said_to_names(&floor.rig).await, ["shop-1 may begin, waited false"]);
    let state = floor.rig.instance(&floor.pool).await.state;
    assert_eq!((&state["held"], &state["waiting"]), (&json!(["key:shop-1"]), &json!(["key:shop-1"])));

    // Given back with nobody else waiting, the place is its own again, and it did not wait for it.
    back("shop-1").await;
    crate::handlers::handled(&floor.rig, "on.released", 1).await;
    assert_eq!(
        said_to_names(&floor.rig).await,
        ["shop-1 may begin, waited false", "shop-1 may begin, waited false"]
    );

    // Another asks and waits, and is told. Then the first is asked for
    // again while it holds its place: still nothing is said of it.
    ask("shop-2").await;
    ask("shop-1").await;
    crate::handlers::handled(&floor.rig, "on.asked", 4).await;
    assert_eq!(said_to_names(&floor.rig).await[2..], ["shop-2 waits"]);
    // It gives its place back, and the one that asked before it has it:
    // now it waits behind somebody, and is told so. Which of the two is
    // said first is not the pool's to settle: each is its own step.
    back("shop-1").await;
    crate::handlers::handled(&floor.rig, "on.released", 2).await;
    let mut both = said_to_names(&floor.rig).await[3..].to_vec();
    both.sort();
    assert_eq!(both, ["shop-1 waits", "shop-2 may begin, waited true"]);
    // And has its turn when that one is done, having waited.
    back("shop-2").await;
    crate::handlers::handled(&floor.rig, "on.released", 3).await;
    assert_eq!(said_to_names(&floor.rig).await[5..], ["shop-1 may begin, waited true"]);
}

#[tokio::test]
async fn a_pool_says_how_many_it_gives_places_to_when_that_changes_and_not_for_being_given_another_text() {
    let floor = Floor::with(1).await;
    // Another text that asks nothing else of it, with the places it had: as
    // a later program brings, or a fleet applied again with its steps run.
    let other = POOL.replacen("description: Let a limited number", "description: Lets a limited number", 1);
    assert_ne!(other, POOL);
    let again = |text: &str, limit: u64| Apply {
        key: Some("jobs".into()),
        ..floor.rig.request(text, json!({ "name": "jobs", "limit": limit }))
    };
    let applied = floor.rig.engine.apply(again(&other, 1)).await.unwrap();
    assert_eq!(applied.instance, floor.pool);
    // It took the text, and gave out what places there were to give.
    floor.rig.event("instance.updated", |_| true).await;
    let filled = floor.rig.event("run.finished", |event| event.data["flow"] == "on.fill").await.seq;
    assert!(floor.rig.events("pool.open").await.is_empty(), "nothing changed, and nothing is said");

    // With two places where it had one, it says so, once.
    floor.rig.engine.apply(again(&other, 2)).await.unwrap();
    floor.rig.event("run.finished", move |event| event.seq > filled && event.data["flow"] == "on.fill").await;
    let said: Vec<Value> =
        floor.rig.events("pool.open").await.iter().map(|event| event.data.clone()).collect();
    assert_eq!(said, [json!({ "pool": "jobs", "limit": 2 })]);
}

#[tokio::test]
async fn an_instance_that_waits_for_its_next_place_behind_another_is_told_so_about_what_it_asked_about() {
    // Workers that say what they ask about, each time they ask: what they are called.
    let about = WORKER.replace(
        "{ topic: pool.asked, pool: jobs }",
        "{ topic: pool.asked, pool: jobs, about: \"${instance.key}\" }",
    );
    assert_ne!(about, WORKER);
    let mut floor = Floor::with(1).await;
    for who in ["a", "b"] {
        let asked = Apply { key: Some(who.into()), ..floor.rig.request(&about, json!({})) };
        let applied = floor.rig.engine.apply(asked).await.unwrap_or_else(|err| panic!("{}", err.message));
        floor.names.insert(applied.instance, who.to_string());
        match who {
            "a" => floor.began(who, 1).await,
            _ => drop(floor.waits(who).await),
        }
    }
    // The first asks for its next place while it holds this one, and is
    // told nothing; it gives this one back, the other has it, and now it
    // is told that it waits: about what it said when it asked, though it
    // says nothing of that as it gives its place back.
    floor.say("worker.more", json!({ "who": "a" })).await;
    crate::handlers::handled(&floor.rig, "on.asked", 3).await;
    floor.say("worker.finish", json!({ "who": "a" })).await;
    floor.began("b", 1).await;
    floor.waits("a").await;
    let said = floor.rig.events("pool.waiting").await;
    let about: Vec<&str> = said.iter().map(|event| event.data["about"].as_str().unwrap()).collect();
    assert_eq!(about, ["b", "a"]);
    // Its turn comes when the other is done, and nothing is kept of either once both are.
    floor.say("worker.finish", json!({ "who": "b" })).await;
    floor.began("a", 2).await;
    floor.say("worker.finish", json!({ "who": "a" })).await;
    crate::handlers::handled(&floor.rig, "on.released", 3).await;
    assert_eq!(floor.rig.instance(&floor.pool).await.state["abouts"], json!({}));
}

#[tokio::test]
async fn a_pool_from_before_it_kept_what_was_asked_about_goes_on_where_it_is_and_keeps_it_from_then() {
    // The pool's text as it was before: no word of state for what was
    // asked about, and nothing handed or kept of it.
    let before = POOL
        .replace("  abouts: {}\n", "")
        .replace("          abouts: \"${steps.turn.out.abouts}\"\n", "")
        .replace("          about: \"${ event.data.about ?? '' }\"\n", "");
    assert!(!before.contains("abouts"), "{before}");
    let rig = Rig::new().await;
    let pool = |rig: &Rig, text: &str| Apply {
        key: Some("jobs".into()),
        ..rig.request(text, json!({ "name": "jobs", "limit": 1 }))
    };
    let applied = rig.engine.apply(pool(&rig, &before)).await.unwrap_or_else(|err| panic!("{}", err.message));
    active(&rig, &applied.instance).await;
    let mut floor = Floor { rig, pool: applied.instance, names: HashMap::new() };
    floor.worker("a").await;
    floor.began("a", 1).await;
    floor.worker("b").await;
    floor.waits("b").await;

    // It takes the text that keeps it, where it is: who held and who waited still do.
    let asked = pool(&floor.rig, POOL);
    let again = floor.rig.engine.apply(asked).await.unwrap_or_else(|err| panic!("{}", err.message));
    assert_eq!(again.instance, floor.pool);
    floor.rig.event("instance.updated", |_| true).await;
    assert_eq!(floor.stands().await, (vec!["a".into()], vec!["b".into()]));
    // And goes on: the place given back goes to the one that waited.
    floor.say("worker.finish", json!({ "who": "a" })).await;
    floor.began("b", 1).await;
    assert_eq!(floor.rig.instance(&floor.pool).await.state["abouts"], json!({}));
}

/// Something that looks after a thing of its own name and asks for its
/// place by that name: as soon as it is there, whether or not a pool is,
/// and again each time the pool says that it is listening.
const EARLY: &str = "\
blueprint: early
on:
  start:
    event: instance.active
    scope: own
    when: ${ event.source.instance == instance.id }
    do: emit
    with: { topic: pool.asked, pool: jobs, as: \"${instance.key}\" }

  again:
    event: pool.listening
    for: { pool: jobs }
    do: emit
    with: { topic: pool.asked, pool: jobs, as: \"${instance.key}\" }

  go:
    event: pool.granted
    for: { pool: jobs, as: \"${instance.key}\" }
    do: emit
    with: { topic: worker.began, who: \"${instance.key}\" }
";

#[tokio::test]
async fn a_pool_says_when_it_is_listening_and_who_it_has_so_what_asked_before_it_could_hear_asks_again() {
    let rig = Rig::new().await;
    let early = |key: &str| Apply { key: Some(key.into()), ..rig.request(EARLY, json!({})) };
    // It asks, and there is no pool to hear it: the order is this test's
    // own, and not how fast anything was.
    rig.engine.apply(early("a")).await.unwrap_or_else(|err| panic!("{}", err.message));
    let unheard = rig.event("pool.asked", |event| event.data["as"] == "a").await.seq;
    assert!(rig.events("pool.listening").await.is_empty());

    // The pool comes to be listening and says so, having nobody; what
    // had asked asks again, and is given the place nobody heard it ask for.
    let pool = Floor::pool(&rig, 1).await;
    let floor = Floor { rig, pool, names: HashMap::new() };
    floor.began("a", 1).await;
    let said = floor.rig.events("pool.listening").await;
    assert_eq!(said.len(), 1);
    assert!(said[0].seq > unheard);
    assert_eq!(said[0].data, json!({ "pool": "jobs", "held": [], "waiting": [] }));

    // Another asks once the pool is listening, is heard, and waits.
    let early = |key: &str| Apply { key: Some(key.into()), ..floor.rig.request(EARLY, json!({})) };
    floor.rig.engine.apply(early("b")).await.unwrap_or_else(|err| panic!("{}", err.message));
    floor.rig.event("pool.waiting", |event| event.data["to"] == "key:b").await;

    // Told to give its places in another order, the pool takes that where
    // it is and says again that it is listening, with who it has. Both ask
    // again: the one that waits is kept once, and the one that holds its
    // place waits for its next.
    let other = Apply {
        key: Some("jobs".into()),
        ..floor.rig.request(POOL, json!({ "name": "jobs", "limit": 1, "order": "last" }))
    };
    let again = floor.rig.engine.apply(other).await.unwrap_or_else(|err| panic!("{}", err.message));
    assert_eq!(again.instance, floor.pool);
    let said = floor.rig.event("pool.listening", move |event| event.seq > said[0].seq).await;
    assert_eq!(said.data, json!({ "pool": "jobs", "held": ["key:a"], "waiting": ["key:b"] }));
    let mut kept = floor.rig.follow_after(said.seq, "run.finished");
    let mut asked = 0;
    soon("both to have asked again and been heard", async {
        while asked < 2 {
            asked += usize::from(kept.next().await.unwrap().data["flow"] == "on.asked");
        }
    })
    .await;
    let state = floor.rig.instance(&floor.pool).await.state;
    assert_eq!(state["held"], json!(["key:a"]));
    assert_eq!(state["waiting"], json!(["key:b", "key:a"]));
    assert_eq!(floor.order().await, ["a"], "and nobody was given a second place for it");
}

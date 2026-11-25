//! A handler and a `wait` hear from several journals: an instance's own
//! project's and the factory's own, and every one for what says the
//! whole factory. A handler has a place in each journal it hears, moved
//! with the run an event of that journal starts, so that after a restart
//! nothing of either is handed over twice and nothing is missed; and a
//! journal made while it listens is one it reads from its beginning.

use journal::Journal;
use kernel::Source;
use serde_json::{Value, json};

use crate::stepped::Stepped;
use crate::{Rig, soon};

/// Counts each thing it hears, by which.
const HEARER: &str = r#"
blueprint: hearer
on:
  heard:
    event: s.thing
    steps:
      count: { do: test.count, with: { key: "heard-${event.data.n}" } }
"#;

/// The same, hearing the whole factory.
const EVERYWHERE: &str = r#"
blueprint: everywhere
on:
  heard:
    event: s.thing
    scope: fleet
    steps:
      count: { do: test.count, with: { key: "all-${event.data.n}" } }
"#;

/// Hears the whole factory, and waits at a gate over each thing before
/// it counts it.
const HELD_EVERYWHERE: &str = r#"
blueprint: held-everywhere
on:
  heard:
    event: s.thing
    scope: fleet
    steps:
      held: { do: test.gate, with: { name: "over-${event.data.n}" } }
      count: { do: test.count, with: { key: "all-${event.data.n}" } }
"#;

/// Waits for a thing twice, and counts what each wait was given.
const WAITER: &str = r#"
blueprint: waiter
steps:
  first: { do: wait, with: { event: s.thing } }
  one: { do: test.count, with: { key: "given-${steps.first.out.event.data.n}" } }
  second: { do: wait, with: { event: s.thing, say: { topic: s.waiting } } }
  two: { do: test.count, with: { key: "given-${steps.second.out.event.data.n}" } }
"#;

/// Starts a child for each thing that is said, called for the thing.
const PARENT: &str = r#"
blueprint: parent
on:
  heard:
    event: s.thing
    steps:
      go: { spawn: child, as: "kid-${event.data.n}", with: { n: "${event.data.n}" } }
"#;

/// The child as it is started: it hears nothing of the thing.
const CHILD: &str = r#"
blueprint: child
params: { n: 0 }
steps:
  claim: { do: test.count, with: { key: claimed } }
on:
  poke: { event: s.poke, do: test.count, with: { key: poked } }
"#;

/// The child once it hears the thing it is called for.
const CHILD_HEARS: &str = r#"
blueprint: child
params: { n: 0 }
steps:
  claim: { do: test.count, with: { key: claimed } }
on:
  poke: { event: s.poke, do: test.count, with: { key: poked } }
  again:
    event: s.thing
    for: { n: "${params.n}" }
    do: test.count
    with: { key: "again-${event.seq}" }
"#;

/// Counts the last of the things said close together, by which.
const QUIET: &str = r#"
blueprint: quiet
on:
  heard:
    event: s.thing
    debounce: 2s
    do: test.count
    with: { key: "quiet-${event.data.n}" }
"#;

/// A rig whose journal keeps its projects' beside it, and the journal of
/// the project `shop`.
async fn rig() -> (Rig, Journal) {
    let rig = Rig::new().await;
    rig.keeps_projects().await;
    let shop = rig.journal.own("shop").await.unwrap();
    (rig, shop)
}

/// Applies `source` for the project `shop`, and gives the instance
/// once its handlers are listening.
async fn applied(rig: &Rig, source: &str) -> String {
    let instance = begun(rig, source).await;
    rig.event("instance.active", |event| event.source.instance.as_deref() == Some(instance.as_str())).await;
    instance
}

/// Applies `source` for the project `shop` and gives the instance, its
/// steps under way.
async fn begun(rig: &Rig, source: &str) -> String {
    let asked = conductor::Apply { project: Some("shop".into()), ..rig.request(source, json!({})) };
    rig.engine.apply(asked).await.unwrap().instance
}

async fn say(journal: &Journal, data: Value) -> u64 {
    journal.emit("s.thing", Source::default(), data).await.unwrap().seq
}

/// Waits until each of `keys` has been counted once, and holds that none
/// was counted more.
async fn counted(rig: &Rig, keys: &[&str]) {
    rig.soon(&format!("{keys:?} to be counted"), async {
        while keys.iter().any(|key| rig.shared.count(key) == 0) {
            tokio::task::yield_now().await;
        }
    })
    .await;
    for _ in 0..50 {
        tokio::task::yield_now().await;
    }
    let more: Vec<&&str> = keys.iter().filter(|key| rig.shared.count(key) != 1).collect();
    assert!(more.is_empty(), "counted more than once: {more:?}");
}

/// Where the handler `heard` of `instance` has got to, by journal.
async fn places(rig: &Rig, instance: &str) -> Vec<(String, u64)> {
    places_of(rig, instance, "heard").await
}

/// Where the handler `handler` of `instance` has got to, by journal:
/// read from whichever database keeps the instance, which is its
/// project's own. One that keeps nothing that is run has no such table.
async fn places_of(rig: &Rig, instance: &str, handler: &'static str) -> Vec<(String, u64)> {
    let mut found = Vec::new();
    for db in rig.journal.dbs() {
        let instance = instance.to_string();
        let read = db.call(move |connection| {
            let mut statement = connection.prepare(
                "SELECT journal, seq FROM places WHERE instance = ?1 AND handler = ?2 ORDER BY journal",
            )?;
            let rows = statement.query_map([instance.as_str(), handler], |row| {
                Ok((row.get(0)?, row.get::<_, i64>(1)? as u64))
            })?;
            rows.collect::<rusqlite::Result<Vec<_>>>()
        });
        found.extend(read.await.unwrap_or_default());
    }
    found
}

#[tokio::test]
async fn a_handler_hears_its_projects_journal_and_the_factorys_with_a_place_in_each_and_nothing_twice_after_a_restart()
 {
    if crate::one_journal() {
        return;
    }
    let (mut rig, shop) = rig().await;
    let yard = rig.journal.own("yard").await.unwrap();
    let before =
        [say(&shop, json!({ "project": "shop", "n": 0 })).await, say(&rig.journal, json!({ "n": 0 })).await];
    let instance = applied(&rig, HEARER).await;
    // It begins now in each journal it hears, and has no place in a
    // journal it does not: another project's.
    let begins = places(&rig, &instance).await;
    assert_eq!(begins.len(), 2, "{begins:?}");
    assert!(begins[0].0 == "factory" && begins[0].1 >= before[1], "{begins:?}");
    // Its own project's journal keeps what is run for it too, so now
    // there is after its having been applied was said.
    assert!(begins[1].0 == "project:shop" && begins[1].1 >= before[0], "{begins:?}");

    let of_shop = say(&shop, json!({ "project": "shop", "n": 1 })).await;
    let of_nobody = say(&rig.journal, json!({ "n": 2 })).await;
    say(&yard, json!({ "project": "yard", "n": 3 })).await;
    counted(&rig, &["heard-1", "heard-2"]).await;
    assert_eq!((rig.shared.count("heard-0"), rig.shared.count("heard-3")), (0, 0));
    // Each place moved with the run its own journal's event started,
    // and neither by what the other journal said.
    assert_eq!(
        places(&rig, &instance).await,
        [("factory".to_string(), of_nobody), ("project:shop".into(), of_shop)]
    );

    // Said while no conductor is there, in each; and the conductor back.
    rig.engine.halt();
    rig.engine.halted().await;
    let later =
        [say(&shop, json!({ "project": "shop", "n": 4 })).await, say(&rig.journal, json!({ "n": 5 })).await];
    rig.restart().await;
    say(&shop, json!({ "project": "shop", "n": 6 })).await;
    counted(&rig, &["heard-1", "heard-2", "heard-4", "heard-5", "heard-6"]).await;
    assert_eq!(places(&rig, &instance).await[0], ("factory".to_string(), later[1]));
    assert_eq!(rig.shared.count("heard-3"), 0, "another project's journal is not read for it");
}

#[tokio::test]
async fn what_hears_the_whole_factory_hears_every_journal_and_one_made_while_it_listens() {
    if crate::one_journal() {
        return;
    }
    let (mut rig, shop) = rig().await;
    let instance = applied(&rig, EVERYWHERE).await;
    say(&shop, json!({ "project": "shop", "n": 1 })).await;
    counted(&rig, &["all-1"]).await;
    // A project is given a journal while the handler waits, and says
    // something in it at once.
    let loft = rig.journal.own("loft").await.unwrap();
    let first = say(&loft, json!({ "project": "loft", "n": 2 })).await;
    say(&rig.journal, json!({ "n": 3 })).await;
    counted(&rig, &["all-1", "all-2", "all-3"]).await;
    let kept = places(&rig, &instance).await;
    assert!(kept.contains(&("project:loft".to_string(), first)), "{kept:?}");
    // Started again, it goes on in each from where it was.
    rig.restart().await;
    say(&loft, json!({ "project": "loft", "n": 4 })).await;
    counted(&rig, &["all-1", "all-2", "all-3", "all-4"]).await;
}

#[tokio::test]
async fn what_hears_the_whole_factory_hears_a_journal_made_while_it_has_something_in_hand() {
    if crate::one_journal() {
        return;
    }
    let (rig, shop) = rig().await;
    applied(&rig, HELD_EVERYWHERE).await;
    say(&shop, json!({ "project": "shop", "n": 1 })).await;
    rig.reached("over-1").await;
    // A project is given a journal while the handler is in the middle
    // of a run, waiting on nothing that says so, and says something.
    let loft = rig.journal.own("loft").await.unwrap();
    rig.open("over-2");
    say(&loft, json!({ "project": "loft", "n": 2 })).await;
    rig.open("over-1");
    counted(&rig, &["all-1", "all-2"]).await;
}

#[tokio::test]
async fn a_wait_hears_what_its_projects_journal_says_and_what_was_said_there_while_the_conductor_was_away() {
    let (mut rig, shop) = rig().await;
    let instance = begun(&rig, WAITER).await;
    // Once it says it waits: a wait listens from there, and what is said
    // after its step began and before then may be before its place
    // ([`a_wait_listens_from_when_it_says_it_waits_and_not_from_when_its_step_began`]).
    rig.event("step.waiting", |event| event.source.step.as_deref() == Some("first")).await;
    say(&rig.journal.own("yard").await.unwrap(), json!({ "project": "yard", "n": 9 })).await;
    say(&shop, json!({ "project": "shop", "n": 1 })).await;
    counted(&rig, &["given-1"]).await;
    // Once it is listening, which it says, the conductor goes.
    rig.event("s.waiting", |_| true).await;
    // The conductor goes, and the thing is said with nobody waiting.
    rig.engine.halt();
    rig.engine.halted().await;
    say(&shop, json!({ "project": "shop", "n": 2 })).await;
    rig.restart().await;
    counted(&rig, &["given-1", "given-2"]).await;
    assert_eq!(rig.shared.count("given-9"), 0);
    rig.soon("the instance to be done", async {
        while rig.instance(&instance).await.status.is_live() {
            tokio::task::yield_now().await;
        }
    })
    .await;
}

/// Waits for a thing once, and counts what it was given.
const WAITS_ONCE: &str = r#"
blueprint: waits-once
steps:
  first: { do: wait, with: { event: s.thing } }
  one: { do: test.count, with: { key: "given-${steps.first.out.event.data.n}" } }
"#;

/// A wait whose step is let go forward `turns` turns, and then a thing
/// said by somebody who does not give way, so that it is kept with
/// whatever the step's run was answered not yet looked at; once the wait
/// says it waits, a second. Where the first came among what the step
/// said (`step.started` before it, `step.waiting` after it), and which
/// of the two the wait was given.
async fn said_as_it_began(turns: usize) -> (bool, bool, Vec<u64>) {
    let (rig, shop) = rig().await;
    let first_step = |event: &kernel::Event| event.source.step.as_deref() == Some("first");
    let mut stepped = Stepped::holding(&rig.journal);
    stepped.through(false, begun(&rig, WAITS_ONCE)).await.expect("the journal is open");
    for _ in 0..turns {
        stepped.turn(false).await;
    }
    let mut thing = Box::pin(say(&shop, json!({ "project": "shop", "n": 1 })));
    if stepped.at_once(thing.as_mut()).is_pending() {
        stepped.through(false, thing).await.expect("the journal is open");
    }
    let waits = rig.event("step.waiting", first_step);
    stepped.through(false, waits).await.expect("the journal is open");
    stepped
        .through(false, say(&shop, json!({ "project": "shop", "n": 2 })))
        .await
        .expect("the journal is open");
    let given =
        || [1, 2].into_iter().filter(|n| rig.shared.count(&format!("given-{n}")) > 0).collect::<Vec<_>>();
    let counted = std::future::poll_fn(|_| match given().is_empty() {
        true => std::task::Poll::Pending,
        false => std::task::Poll::Ready(()),
    });
    rig.soon("the wait to be given a thing", stepped.through(false, counted))
        .await
        .expect("the journal is open");
    drop(stepped);
    let events = rig.events("step.started,step.waiting,s.thing").await;
    let at = |wanted: &dyn Fn(&kernel::Event) -> bool| {
        events.iter().find(|event| wanted(event)).map(|event| event.seq)
    };
    let began = at(&|event| event.topic == "step.started" && first_step(event)).expect("the step began");
    let listened = at(&|event| event.topic == "step.waiting" && first_step(event)).expect("the wait waited");
    let thing = at(&|event| event.topic == "s.thing" && event.data["n"] == 1).expect("the thing was said");
    (began < thing, thing < listened, given())
}

#[tokio::test]
async fn a_wait_listens_from_when_it_says_it_waits_and_not_from_when_its_step_began() {
    // Every amount of turns, from the step not begun until the wait has
    // said that it waits. At one of them the thing is kept after the
    // step said it began and before the wait took its place, which is
    // after: a test that said what it waited for as soon as the step
    // began met that now and then, and waited for ever (`step.waiting`
    // is what says the wait is listening). What is said once it says so
    // is its, whatever came before.
    let mut came = false;
    for turns in 0..=200 {
        let (after_it_began, before_it_listened, given) =
            soon(&format!("{turns} turns"), said_as_it_began(turns)).await;
        match (after_it_began, before_it_listened) {
            (_, false) => {
                assert_eq!(given, [1], "{turns} turns: said once it was waiting, it is the wait's");
                assert!(came, "the order this is there for did not come about before the wait was listening");
                return;
            }
            (false, true) => assert_eq!(given, [2], "{turns} turns: said before the step began, it is not"),
            (true, true) => {
                // Kept before the wait took its place or after: which is
                // in what the turns made, and the second is heard either way.
                came |= given == [2];
                assert_eq!(given.len(), 1, "{turns} turns: it was given one thing, {given:?}");
            }
        }
    }
    panic!("the wait was not listening after 200 turns: the amounts no longer reach past it");
}

#[tokio::test]
async fn a_handler_a_text_brings_hears_the_event_that_brought_the_text_in_whichever_journal_keeps_it() {
    // Said in the project's journal, and then in the factory's own: each
    // time the child the parent starts takes a text that brings a
    // handler for exactly that thing, and is handed the thing once.
    for (which, project) in [(0, Some("shop")), (1, None)] {
        let (rig, shop) = rig().await;
        let said_in = if which == 0 { shop.clone() } else { rig.journal.clone() };
        let other = if which == 0 { rig.journal.clone() } else { shop.clone() };
        let data = |n: u64| match project {
            Some(project) => json!({ "project": project, "n": n }),
            None => json!({ "n": n }),
        };
        rig.library.put("parent", PARENT);
        rig.library.put("child", CHILD);
        applied(&rig, PARENT).await;
        say(&said_in, data(1)).await;
        counted(&rig, &["claimed"]).await;
        let kid = rig
            .soon("the child to be there", async {
                loop {
                    let filter = conductor::InstanceFilter { live_only: true, ..Default::default() };
                    let live = rig.engine.instances(filter).await.unwrap();
                    if let Some(kid) = live.into_iter().find(|one| one.key == "kid-1") {
                        return kid.id;
                    }
                    tokio::task::yield_now().await;
                }
            })
            .await;
        rig.event("instance.active", |event| event.source.instance.as_deref() == Some(kid.as_str())).await;
        // Something said in the other journal meanwhile, numbered where
        // it may be: a place just before the event is a place in the
        // journal that keeps the event, and in no other.
        let elsewhere = other.emit("s.other", Source::default(), json!({})).await.unwrap().seq;

        rig.library.put("child", CHILD_HEARS);
        let brought = say(&said_in, data(1)).await;
        counted(&rig, &[&format!("again-{brought}")]).await;
        assert_eq!(rig.shared.count("claimed"), 1, "its steps were not run again");
        // Its place in the journal that keeps the event moved with the
        // run; in the other it began where that journal then was.
        let kept: Vec<(String, u64)> = places_of(&rig, &kid, "again").await;
        let place =
            |journal: &Journal| kept.iter().find(|(name, _)| name == journal.name()).map(|(_, at)| *at);
        assert_eq!(place(&said_in), Some(brought), "{kept:?}");
        assert!(place(&other).is_some_and(|at| at >= elsewhere), "{kept:?}");
        // And the next is one more.
        let next = say(&said_in, data(1)).await;
        counted(&rig, &[&format!("again-{brought}"), &format!("again-{next}")]).await;
    }
}

#[tokio::test]
async fn what_a_burst_let_go_of_in_one_journal_for_a_later_thing_in_another_is_not_handed_over_again() {
    if crate::one_journal() {
        return;
    }
    let (mut rig, shop) = rig().await;
    let instance = applied(&rig, QUIET).await;
    let sleeps = rig.clock.sleep_count();
    // One thing in the project's journal, and before it has been quiet
    // a later one in the factory's own: the later is what it runs with.
    let let_go = say(&shop, json!({ "project": "shop", "n": 1 })).await;
    rig.soon("the quiet to begin", rig.clock.sleeps_started(sleeps + 1)).await;
    let later = say(&rig.journal, json!({ "n": 2 })).await;
    rig.soon("the quiet to begin again", rig.clock.sleeps_started(sleeps + 2)).await;
    rig.clock.advance(std::time::Duration::from_secs(2));
    counted(&rig, &["quiet-2"]).await;
    assert_eq!(rig.shared.count("quiet-1"), 0);
    // Its place in each: just after what it ran with, and after what it
    // let go of, which is written though no run was started for it.
    assert_eq!(
        places(&rig, &instance).await,
        [("factory".to_string(), later), ("project:shop".to_string(), let_go)]
    );
    // Started again, neither is handed over a second time.
    rig.restart().await;
    let again = say(&shop, json!({ "project": "shop", "n": 3 })).await;
    rig.soon("the quiet to begin", rig.clock.sleeps_started(sleeps + 3)).await;
    rig.clock.advance(std::time::Duration::from_secs(2));
    counted(&rig, &["quiet-2", "quiet-3"]).await;
    assert_eq!(rig.shared.count("quiet-1"), 0, "what was let go of is not handed over after a restart");
    assert_eq!(places(&rig, &instance).await[1], ("project:shop".to_string(), again));
}

/// Says a thing when it is poked, naming no project: its own conductor
/// keeps what it says.
const SAYS: &str = r#"
blueprint: says
on:
  poked: { event: s.poke, do: emit, with: { topic: s.said, n: "${event.data.n}" } }
"#;

/// Stops whatever instance it is told of, whoever keeps it.
const STOPPER: &str = r#"
blueprint: stopper
on:
  told:
    event: s.stop
    scope: fleet
    steps:
      end: { do: stop, with: { instance: "${event.data.id}" } }
"#;

/// How many instances the database of `journal` keeps.
async fn kept_in(journal: &Journal) -> i64 {
    let read = journal
        .db()
        .call(|connection| connection.query_row("SELECT count(*) FROM instances", [], |row| row.get(0)));
    read.await.unwrap_or(0)
}

#[tokio::test]
async fn what_is_applied_for_a_project_is_kept_in_its_journal_by_its_own_conductor_and_asked_of_the_factorys()
{
    if crate::one_journal() {
        return;
    }
    let (mut rig, shop) = rig().await;
    let yard = rig.journal.own("yard").await.unwrap();
    let of_shop = applied(&rig, SAYS).await;
    let asked = conductor::Apply { project: Some("yard".into()), ..rig.request(HEARER, json!({})) };
    let of_yard = rig.engine.apply(asked).await.unwrap().instance;
    let asked = conductor::Apply { project: None, ..rig.request(EVERYWHERE, json!({})) };
    let of_nobody = rig.engine.apply(asked).await.unwrap().instance;
    // Each where it is kept, and nowhere else.
    assert_eq!((kept_in(&shop).await, kept_in(&yard).await, kept_in(&rig.journal).await), (1, 1, 1));
    // What its conductor says of it is in its own journal, and so is
    // what it says itself.
    let poked = json!({ "project": "shop", "n": 7 });
    rig.journal.emit("s.poke", Source::default(), poked).await.unwrap();
    rig.event("s.said", |event| event.data["n"] == json!(7)).await;
    let all = journal::EventQuery { limit: usize::MAX, ..Default::default() };
    let topics = |journal: Journal| {
        let all = all.clone();
        async move { journal.read(all).await.unwrap().into_iter().map(|event| event.topic).collect::<Vec<_>>() }
    };
    let in_shop = topics(shop.clone()).await;
    for topic in ["instance.started", "instance.active", "s.poke", "s.said"] {
        assert!(
            in_shop.iter().any(|said| said == topic),
            "{topic} is not in the project's journal: {in_shop:?}"
        );
    }
    // What it says of its own runs, which no text there hears, is passed
    // on and kept in no journal.
    let begun = rig.events("run.started").await;
    assert!(begun.iter().any(|said| said.source.instance.as_deref() == Some(of_shop.as_str())), "{begun:?}");
    assert!(!in_shop.iter().any(|said| said == "run.started"), "{in_shop:?}");
    assert!(!topics(rig.journal.clone()).await.iter().any(|topic| topic == "s.said" || topic == "s.poke"));

    // Asked of the factory's conductor, each is answered by whoever
    // keeps it: one, all of them, a project's, and whether it is there.
    assert_eq!(rig.engine.instance(&of_shop).await.unwrap().project.as_deref(), Some("shop"));
    assert_eq!(rig.engine.instance(&of_yard).await.unwrap().project.as_deref(), Some("yard"));
    let live = conductor::InstanceFilter { live_only: true, ..Default::default() };
    let mut all: Vec<String> =
        rig.engine.instances(live.clone()).await.unwrap().into_iter().map(|one| one.id).collect();
    all.sort();
    let mut meant = vec![of_shop.clone(), of_yard.clone(), of_nobody.clone()];
    meant.sort();
    assert_eq!(all, meant);
    let of = conductor::InstanceFilter { project: Some("yard".into()), ..live.clone() };
    let yards: Vec<String> = rig.engine.instances(of).await.unwrap().into_iter().map(|one| one.id).collect();
    assert_eq!(yards, std::slice::from_ref(&of_yard));
    for id in [&of_shop, &of_yard, &of_nobody] {
        assert!(rig.engine.is_there(id).await, "{id} is there");
    }
    assert!(!rig.engine.runs(&of_shop, 10).await.unwrap().is_empty());

    // Stopped through the factory's, it is stopped where it is kept;
    // and after a restart each is taken up by its own.
    rig.engine.stop(&of_yard).await.unwrap();
    assert!(!rig.engine.instance(&of_yard).await.unwrap().status.is_live());
    assert!(!rig.engine.is_there(&of_yard).await);
    rig.restart().await;
    assert!(rig.engine.is_there(&of_shop).await && rig.engine.is_there(&of_nobody).await);
    assert!(!rig.engine.is_there(&of_yard).await);
    rig.journal.emit("s.poke", Source::default(), json!({ "project": "shop", "n": 8 })).await.unwrap();
    rig.event("s.said", |event| event.data["n"] == json!(8)).await;
}

#[tokio::test]
async fn a_step_that_stops_an_instance_another_conductor_keeps_has_it_stopped_there() {
    let (rig, _shop) = rig().await;
    rig.journal.own("yard").await.unwrap();
    let asked = conductor::Apply { project: Some("yard".into()), ..rig.request(HEARER, json!({})) };
    let of_yard = rig.engine.apply(asked).await.unwrap().instance;
    // The stopper is the project `shop`'s, and hears the whole factory.
    applied(&rig, STOPPER).await;
    rig.journal.emit("s.stop", Source::default(), json!({ "id": of_yard })).await.unwrap();
    rig.soon("the other project's instance to be stopped", async {
        while rig.engine.instance(&of_yard).await.unwrap().status.is_live() {
            tokio::task::yield_now().await;
        }
    })
    .await;
    assert!(!rig.engine.is_there(&of_yard).await);
}

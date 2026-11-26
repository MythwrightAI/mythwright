//! What a choosing asks of the turns that are kept: how many each agent
//! was given is counted as turns are queued, and read from the turns
//! only as agents are opened and where turns are taken away; and what
//! each has in hand is asked of the index, an agent at a time. Neither
//! takes longer for the turns there have been.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use agents::{Order, TurnRequest};
use rusqlite::params;
use serde_json::{Value, json};

use crate::Rig;

fn work(slot: &str, words: &str) -> TurnRequest {
    TurnRequest {
        slot: slot.into(),
        kind: "task".into(),
        thread: format!("about:{words}"),
        prompt: words.into(),
        ..TurnRequest::default()
    }
}

/// How many turns each agent that could take work was given, and how
/// many it has in hand.
async fn counted(rig: &Rig) -> BTreeMap<String, (i64, usize)> {
    let could = rig.agents.could_take(Some("shop"), None, &[], Order::Least).await.unwrap();
    could.into_iter().map(|one| (one.slot.name.clone(), (one.given, one.in_hand))).collect()
}

/// Turns of `slot` as an earlier program kept them, ended long ago.
async fn kept_before(rig: &Rig, slot: &'static str, turns: usize) {
    let wrote = rig.journal.db().call(move |connection| {
        for n in 0..turns {
            connection.execute(
                "INSERT INTO turns (id, slot, kind, thread, prompt, options, status, created, finished)
                 VALUES (?1, ?2, 'task', 'about:before', 'a shelf', '[]', 'done', 1, 2)",
                params![format!("turn_before_{slot}_{n}"), slot],
            )?;
        }
        Ok(())
    });
    wrote.await.unwrap();
}

/// Turns of `slot`, an agent of the shop's, ended long ago, in the file
/// that keeps the turns of the shop's agents.
async fn kept_of_the_shops(rig: &Rig, slot: &'static str, turns: usize) {
    let wrote = rig.turns_of("shop").db().call(move |connection| {
        for n in 0..turns {
            connection.execute(
                "INSERT INTO turns (id, slot, kind, thread, prompt, options, status, created, finished, project)
                 VALUES (?1, ?2, 'task', 'about:before', 'a shelf', '[]', 'done', 1, 2, 'shop')",
                params![format!("turn_kept_{slot}_{n}"), slot],
            )?;
        }
        Ok(())
    });
    wrote.await.unwrap();
}

#[tokio::test]
async fn what_each_agent_was_given_is_counted_as_it_is_given_and_read_again_as_agents_are_opened() {
    let mut rig = Rig::new().await;
    rig.agent("one", "fake").await;
    rig.agent("two", "fake").await;
    for (slot, words) in [("one", "a shelf"), ("one", "a gate"), ("two", "a fence")] {
        rig.agents.enqueue(work(slot, words)).await.unwrap();
    }
    rig.turns_finished(3).await;
    let given =
        |one: i64, two: i64| BTreeMap::from([("one".to_string(), (one, 0)), ("two".to_string(), (two, 0))]);
    assert_eq!(counted(&rig).await, given(2, 1));
    // The one given least is first, and each in turn goes by the last.
    let least = rig.agents.could_take(Some("shop"), None, &[], Order::Least).await.unwrap();
    assert_eq!(least[0].slot.name, "two");
    let turn = rig.agents.could_take(Some("shop"), None, &[], Order::Turn).await.unwrap();
    assert_eq!(turn[0].slot.name, "one", "`two` was given work last");
    assert!(turn[0].last < turn[1].last);

    // A daemon that starts reads it from the turns that are kept, those
    // an earlier program kept among them.
    rig.stop();
    kept_before(&rig, "two", 4).await;
    rig.start().await;
    assert_eq!(counted(&rig).await, given(2, 5));

    // Turns taken away with their channel are no longer counted.
    let channel = rig.studio.channels.get(&crate::own(), "shop").await.unwrap().id;
    let said = TurnRequest { kind: "message".into(), channel: Some(channel.clone()), ..work("one", "hello") };
    rig.agents.enqueue(said).await.unwrap();
    rig.turns_finished(4).await;
    assert_eq!(counted(&rig).await, given(3, 5));
    assert_eq!(rig.agents.forget_channel(&channel).await.unwrap(), 1);
    assert_eq!(counted(&rig).await, given(2, 5));
}

#[tokio::test]
async fn a_choosing_asks_no_more_of_the_database_however_many_turns_have_been_kept() {
    let rig = Rig::new().await;
    rig.agent("one", "fake").await;
    rig.agent("two", "fake").await;
    // One at work, so that something is in hand.
    rig.agents.enqueue(work("one", "[gate:shelf] a shelf")).await.unwrap();
    rig.fake.saw(1).await;

    // What the studio does by itself on the same databases is no part of
    // choosing: the keeper of records' histories reads every table as one
    // is declared, in its own time. It is stopped, and what it asked
    // before is done by the time the counting begins, which is asked of
    // each database's thread after it.
    rig.studio.halt();
    // How much the databases do, counted by each: the factory's own,
    // which keeps who the agents are, and the shop's, which keeps what
    // its agents were asked.
    let done = Arc::new(AtomicU64::new(0));
    for db in rig.journal.dbs() {
        let counting = done.clone();
        let told = db.call(move |connection| {
            connection.progress_handler(
                1,
                Some(move || {
                    counting.fetch_add(1, Ordering::SeqCst);
                    false
                }),
            )
        });
        told.await.unwrap();
    }
    let chosen = async || {
        let before = done.load(Ordering::SeqCst);
        let could = counted(&rig).await;
        (done.load(Ordering::SeqCst) - before, could)
    };

    let (few, could) = chosen().await;
    assert_eq!(could["one"].1, 1, "{could:?}");
    // Kept where its turn in hand is, and in the factory's own as an
    // earlier program kept them.
    for slot in ["one", "two"] {
        kept_of_the_shops(&rig, slot, 1_000).await;
        kept_before(&rig, slot, 1_000).await;
    }
    // A table declared from another thread while this one does nothing:
    // whatever hears it here is polled only once this one waits on the
    // choosing, and what it reads is then counted with it (349 steps
    // beside 279, with the keeper not stopped).
    let collections = rig.studio.collections.clone();
    let spec = studio::collections::Spec::read(&json!({ "title": { "type": "text" } }), &Value::Null, None);
    let system = kernel::Principal::system();
    let declaring = collections.ensure("shop", "aside", spec.unwrap(), None, &system);
    std::thread::scope(|scope| {
        scope.spawn(|| {
            let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
            runtime.block_on(declaring).unwrap();
        });
    });
    let (many, could) = chosen().await;
    assert_eq!(could["one"].1, 1, "{could:?}");
    // The same to within what a deeper index costs to look into: read
    // through, four thousand turns are tens of thousands of steps.
    assert!(
        many <= few + 100,
        "choosing took {few} steps of the database with a turn kept and {many} with four thousand"
    );
    rig.fake.open("shelf");
    rig.turns_finished(1).await;
}

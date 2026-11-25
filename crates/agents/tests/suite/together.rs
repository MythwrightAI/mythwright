//! What is written of a turn and what is said of it are one write: no
//! stop leaves a turn written as queued, in hand or ended with nobody
//! told; a turn whose answer was kept when the daemon stopped is not
//! asked again, and says its answer and that it ended once; and one
//! called off before it began says that it finished.

use std::sync::{Arc, Mutex};

use agents::TurnRequest;
use kernel::Source;
use rusqlite::params;
use serde_json::json;

use crate::{Rig, ada, soon};

/// Something said to `builder` in its channel, which it answers there.
fn said_to_it(words: &str) -> TurnRequest {
    TurnRequest {
        slot: "builder".into(),
        kind: "message".into(),
        thread: "about:a shelf".into(),
        prompt: words.into(),
        channel: Some("shop".into()),
        ..TurnRequest::default()
    }
}

/// Every turn there is, with how its row stands.
async fn rows(rig: &Rig) -> Vec<(String, String)> {
    let read = |connection: &mut rusqlite::Connection| {
        let mut statement = connection.prepare("SELECT id, status FROM turns ORDER BY rowid")?;
        statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?.collect::<rusqlite::Result<Vec<_>>>()
    };
    rig.of_turns(read).await.into_iter().flatten().collect()
}

/// What was said of turns, each as its topic and the turn it is of.
async fn told(rig: &Rig) -> Vec<(String, String)> {
    let said = rig.events("agent.turn.*").await;
    said.into_iter()
        .map(|event| (event.topic.clone(), event.data["turn"].as_str().unwrap_or_default().to_string()))
        .collect()
}

/// Waits until nothing is asked of any database any more but by this.
async fn rested(rig: &Rig) {
    let dbs = rig.journal.dbs();
    let done = || dbs.iter().map(journal::Db::done).sum::<u64>();
    let mut still = 0;
    while still < 20 {
        let before = done();
        for db in &dbs {
            db.call(|_| Ok(())).await.unwrap();
        }
        for _ in 0..20 {
            tokio::task::yield_now().await;
        }
        still = if done() == before + dbs.len() as u64 { still + 1 } else { 0 };
    }
}

/// A factory with `builder` in it, in which nothing is said after the
/// `place`th thing that is said once the play begins, and whatever was
/// being written down is written all the same: how a daemon really
/// stops. Nothing at all is said of the play where `place` is none.
async fn stopped_after(place: usize) -> Rig {
    let rig = Rig::new().await;
    rig.agent("builder", "fake").await;
    let counted = Arc::new(Mutex::new((false, 0usize)));
    rig.journal.shut_after_saying(move |event| {
        let mut counted = counted.lock().unwrap();
        if event.topic == "test.begins" {
            counted.0 = true;
            return place == 0;
        }
        // What is passed on and not kept is no place of this.
        if counted.0 && event.seq != 0 {
            counted.1 += 1;
        }
        counted.0 && counted.1 == place && event.seq != 0
    });
    rig.journal.emit("test.begins", Source::actor("system"), json!({})).await.unwrap();
    rig
}

#[tokio::test]
async fn no_stop_leaves_a_turn_written_and_not_said() {
    // Played whole first, for the places there are: each thing said
    // from when the turn is asked for until it has ended.
    let rig = Rig::new().await;
    rig.agent("builder", "fake").await;
    let before = rig.events("").await.len();
    rig.fake.will_say("I put up a shelf.");
    let turn = rig.agents.enqueue(said_to_it("Put up a shelf.")).await.unwrap();
    rig.agents.wait(&turn).await.unwrap();
    rested(&rig).await;
    let played: Vec<String> =
        rig.events("").await.into_iter().skip(before).map(|event| event.topic).collect();
    let at =
        |topic: &str| played.iter().position(|said| said == topic).unwrap_or_else(|| panic!("{played:?}"));
    // Its answer is said where it was asked before it is said to have
    // ended, so there is a place between the two; and every event of the
    // turn's says whose it is, as its row does.
    assert!(at("agent.turn.queued") < at("agent.turn.started"), "{played:?}");
    assert!(at("agent.turn.started") < at("channel.message"), "{played:?}");
    assert!(at("channel.message") < at("agent.turn.finished"), "{played:?}");
    for event in rig.events("agent.turn.*").await {
        assert_eq!(event.data["project"], "shop", "{} says whose it is", event.topic);
    }
    assert_eq!(rows(&rig).await, [(turn.clone(), "done".to_string())]);
    drop(rig);

    // Then stopped at each place, and before anything of it was said.
    let mut answered_and_unsaid = 0;
    for place in 0..=played.len() {
        let rig = stopped_after(place).await;
        rig.fake.will_say("I put up a shelf.");
        let agents = rig.agents.clone();
        let asking = tokio::spawn(async move { agents.enqueue(said_to_it("Put up a shelf.")).await });
        soon("the journal to come to its place", rig.journal.shut()).await;
        rested(&rig).await;

        let (rows, told) = (rows(&rig).await, told(&rig).await);
        let said = |topic: &str, turn: &str| told.iter().any(|(said, of)| said == topic && of == turn);
        let stopped = format!("stopped after {place} of {played:?}: rows {rows:?}, said {told:?}");
        for (turn, status) in &rows {
            // A row is there only where it was said to be queued; in hand
            // only where it was said to have started; ended only where it
            // was said to have finished.
            assert!(said("agent.turn.queued", turn), "a turn written and never said to be queued; {stopped}");
            let begun = status != "queued";
            assert_eq!(
                begun,
                said("agent.turn.started", turn),
                "a turn's row and what was said of it; {stopped}"
            );
            let ended = status == "done" || status == "failed";
            assert_eq!(
                ended,
                said("agent.turn.finished", turn),
                "a turn's row and what was said of it; {stopped}"
            );
            answered_and_unsaid += usize::from(status == "answered");
        }
        // And nothing was said of a turn that is not written down.
        for (topic, turn) in &told {
            assert!(
                rows.iter().any(|(id, _)| id == turn),
                "`{topic}` said of a turn there is not; {stopped}"
            );
        }
        asking.abort();
    }
    // The one thing that is two writes was among the places: stopped as
    // its answer was said, the turn keeps its answer and is still in hand.
    assert!(answered_and_unsaid >= 1, "no stop fell between an answer being said and its turn ending");
}

/// A turn of `builder`'s as a daemon leaves it that stops after keeping
/// its answer and before saying the turn ended.
async fn left_answered(rig: &Rig, answer: &str) -> i64 {
    let (at, answer) = (rig.journal.now_ms(), answer.to_string());
    // In the file that keeps the turns of the shop's agents.
    let wrote = rig.turns_of("shop").db().call(move |connection| {
        connection.execute(
            "INSERT INTO turns (id, slot, kind, thread, prompt, channel, options, status, reply, created,
                                started, finished, project)
             VALUES ('turn_answered', 'builder', 'message', 'about:a shelf', 'Put up a shelf.', 'shop', '[]',
                     'answered', ?1, ?2, ?2, ?2, 'shop')",
            params![answer, at],
        )
    });
    wrote.await.unwrap();
    at
}

async fn says_it(rig: &Rig, answer: &str) -> usize {
    rig.transcript().await.iter().filter(|line| line.ends_with(answer)).count()
}

#[tokio::test]
async fn a_turn_found_answered_says_its_answer_and_that_it_ended_once_and_its_agent_is_not_asked_again() {
    let mut rig = Rig::new().await;
    rig.agent("builder", "fake").await;
    rig.stop();
    left_answered(&rig, "I put up a shelf.").await;
    assert_eq!(says_it(&rig, "I put up a shelf.").await, 0);

    rig.start().await;
    let ended = rig.event("agent.turn.finished", |event| event.data["turn"] == "turn_answered").await;
    assert_eq!((&ended.data["ok"], &ended.data["project"]), (&json!(true), &json!("shop")));
    // Its answer is there by the time it is said to have ended, once.
    assert_eq!(says_it(&rig, "I put up a shelf.").await, 1);
    assert_eq!(rig.agents.turn("turn_answered").await.unwrap().status, "done");
    assert!(rig.fake.prompts().is_empty(), "its agent was asked again: {:?}", rig.fake.prompts());
    // And a daemon that starts after that finds nothing to go on with.
    rig.restart().await;
    rested(&rig).await;
    let finished = rig.events("agent.turn.finished").await;
    assert_eq!(finished.len(), 1, "{finished:?}");
    assert_eq!(says_it(&rig, "I put up a shelf.").await, 1);
}

#[tokio::test]
async fn an_answer_that_was_said_before_the_daemon_stopped_is_not_said_again() {
    let mut rig = Rig::new().await;
    rig.agent("builder", "fake").await;
    rig.stop();
    left_answered(&rig, "I put up a shelf.").await;
    // The daemon got as far as saying it where it was asked.
    let builder = rig.agents.slot("builder").await.unwrap();
    let place = studio::Place::default();
    rig.studio.channels.post_at("shop", &builder.principal(), "I put up a shelf.", place).await.unwrap();

    rig.start().await;
    rig.event("agent.turn.finished", |event| event.data["turn"] == "turn_answered").await;
    assert_eq!(says_it(&rig, "I put up a shelf.").await, 1);
    assert!(rig.fake.prompts().is_empty());
}

#[tokio::test]
async fn a_turn_called_off_before_it_began_is_written_as_ended_and_says_that_it_finished() {
    let rig = Rig::new().await;
    rig.agent("builder", "fake").await;
    let work = |words: &str| TurnRequest {
        slot: "builder".into(),
        kind: "task".into(),
        thread: format!("about:{words}"),
        prompt: words.into(),
        ..TurnRequest::default()
    };
    let first = rig.agents.enqueue(work("[gate:shelf] a shelf")).await.unwrap();
    rig.fake.saw(1).await;
    let second = rig.agents.enqueue(work("a gate")).await.unwrap();
    rig.agents.cancel(&second).await;

    let ended = rig.event("agent.turn.finished", |event| event.data["turn"] == second.as_str()).await;
    assert_eq!(
        (&ended.data["ok"], &ended.data["called_off"], &ended.data["project"]),
        (&json!(false), &json!(true), &json!("shop"))
    );
    assert_eq!(ended.data["error"], "cancelled before it started");
    assert_eq!(rig.agents.turn(&second).await.unwrap().status, "failed");
    // Called off again, it says nothing more; and the one at work goes on.
    rig.agents.cancel(&second).await;
    rig.fake.open("shelf");
    rig.agents.wait(&first).await.unwrap();
    let finished = rig.events("agent.turn.finished").await;
    assert_eq!(finished.len(), 2, "{finished:?}");
    assert_eq!(rig.fake.prompts().len(), 1, "the one called off was never begun");
    let _ = ada();
}

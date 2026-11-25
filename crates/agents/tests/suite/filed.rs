//! Turns kept in several files: an agent of a project has its turns in
//! the project's own journal, one that is nobody's in the factory's, and
//! one given to another project in both. Which turn of all each is, is
//! what puts them in order, across files and across a start.

use agents::{Order, SlotSpec, TurnRequest};
use serde_json::json;

use crate::{Rig, ada};

/// An agent called `name`, of `project` or of none, in the role `helper`.
async fn agent(rig: &Rig, name: &str, project: Option<&str>) {
    let spec = SlotSpec {
        name: name.into(),
        adapter: Some("fake".into()),
        project: project.map(str::to_string),
        role: Some("helper".into()),
        ..SlotSpec::default()
    };
    rig.agents.ensure(spec, &ada()).await.unwrap();
}

/// Work asked of `slot` in the conversation `thread`, waited for: its turn.
async fn asked(rig: &Rig, slot: &str, thread: &str) -> String {
    let request = TurnRequest {
        slot: slot.into(),
        kind: "task".into(),
        thread: thread.into(),
        prompt: format!("in {thread}"),
        ..TurnRequest::default()
    };
    let id = rig.agents.enqueue(request).await.unwrap();
    rig.agents.wait(&id).await.unwrap();
    id
}

/// Who the conversation `thread` is held with.
async fn held(rig: &Rig, thread: &str) -> Option<String> {
    rig.agents.holding(thread).await.unwrap().map(|slot| slot.name)
}

/// The number each turn there is was given, by its id, in every file.
async fn numbers(rig: &Rig) -> Vec<(String, Option<i64>)> {
    let read = |connection: &mut rusqlite::Connection| {
        let mut statement = connection.prepare("SELECT id, nth FROM turns")?;
        statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?.collect::<rusqlite::Result<Vec<_>>>()
    };
    rig.of_turns(read).await.into_iter().flatten().collect()
}

#[tokio::test]
async fn a_conversation_with_turns_in_two_files_is_held_with_whoever_was_asked_last_and_so_it_is_after_a_start()
 {
    let mut rig = Rig::new().await;
    rig.studio.projects.create("Shop", None, &ada()).await.unwrap();
    agent(&rig, "shop-helper", Some("shop")).await;
    agent(&rig, "none-helper", None).await;
    // One of the shop's and one that is nobody's are asked in the same
    // conversations: the first's turns are kept in the shop's own
    // journal, the second's in the factory's.
    let first = asked(&rig, "shop-helper", "seat:1").await;
    asked(&rig, "none-helper", "seat:1").await;
    asked(&rig, "none-helper", "seat:2").await;
    asked(&rig, "shop-helper", "seat:2").await;
    if journal::apart() != Some(false) {
        assert!(!rig.turns_of("shop").db().is(rig.journal.db()));
    }
    assert_eq!(held(&rig, "seat:1").await.as_deref(), Some("none-helper"));
    assert_eq!(held(&rig, "seat:2").await.as_deref(), Some("shop-helper"));
    // Who the other conversations of a kind are held with, each by its
    // last turn, whichever file keeps that.
    assert_eq!(rig.agents.beside("seat:", "seat:2").await.unwrap(), ["none-helper"]);
    assert_eq!(rig.agents.beside("seat:", "seat:1").await.unwrap(), ["shop-helper"]);
    assert_eq!(rig.agents.beside("seat:", "seat:3").await.unwrap(), ["none-helper", "shop-helper"]);
    // Each in turn, for work that is no project's, so that neither is
    // nearer: the one that was given work longest ago comes first.
    let in_turn = async |rig: &Rig| {
        let could = rig.agents.could_take(None, Some("helper"), &[], Order::Turn).await.unwrap();
        could.into_iter().map(|one| (one.slot.name, one.given)).collect::<Vec<_>>()
    };
    assert_eq!(in_turn(&rig).await, [("none-helper".to_string(), 2), ("shop-helper".to_string(), 2)]);

    // Every turn has a number of its own, in the order they were asked.
    let given = numbers(&rig).await;
    let mut all: Vec<i64> =
        given.iter().map(|(_, nth)| nth.expect("a turn queued now says its number")).collect();
    let highest = *all.iter().max().unwrap();
    all.sort_unstable();
    all.dedup();
    assert_eq!(all.len(), 4, "{given:?}");
    assert_eq!(given.iter().find(|(id, _)| *id == first).unwrap().1, all.first().copied());

    // A daemon that starts reads none of it from memory, and comes to
    // the same; and what is asked next is numbered above all there is.
    rig.restart().await;
    assert_eq!(held(&rig, "seat:1").await.as_deref(), Some("none-helper"));
    assert_eq!(held(&rig, "seat:2").await.as_deref(), Some("shop-helper"));
    assert_eq!(in_turn(&rig).await, [("none-helper".to_string(), 2), ("shop-helper".to_string(), 2)]);
    let next = asked(&rig, "none-helper", "seat:2").await;
    let given = numbers(&rig).await;
    assert!(given.iter().find(|(id, _)| *id == next).unwrap().1 > Some(highest), "{given:?}");
    assert_eq!(held(&rig, "seat:2").await.as_deref(), Some("none-helper"));
    assert_eq!(in_turn(&rig).await, [("shop-helper".to_string(), 2), ("none-helper".to_string(), 3)]);
}

#[tokio::test]
async fn a_turn_from_before_says_no_number_and_comes_before_every_turn_asked_since() {
    let mut rig = Rig::new().await;
    rig.studio.projects.create("Shop", None, &ada()).await.unwrap();
    agent(&rig, "shop-helper", Some("shop")).await;
    agent(&rig, "none-helper", None).await;
    rig.stop();
    // As a program from before kept them, in the one file there was:
    // the last of them held the conversation with the agent that is
    // nobody's.
    for (n, slot) in ["shop-helper", "none-helper"].into_iter().enumerate() {
        let wrote = rig.journal.db().call(move |connection| {
            connection.execute(
                "INSERT INTO turns (id, slot, kind, thread, prompt, options, status, created, finished)
                 VALUES (?1, ?2, 'task', 'seat:1', 'in seat:1', '[]', 'done', 1, 2)",
                rusqlite::params![format!("turn_before_{n}"), slot],
            )
        });
        wrote.await.unwrap();
    }
    rig.start().await;
    assert_eq!(held(&rig, "seat:1").await.as_deref(), Some("none-helper"));
    // Asked of the shop's own now, its turn is kept in another file,
    // where it is the first row there is: it is the last all the same.
    let now = asked(&rig, "shop-helper", "seat:1").await;
    assert_eq!(held(&rig, "seat:1").await.as_deref(), Some("shop-helper"));
    let its: Vec<String> =
        rig.agents.turns("shop-helper", 10).await.unwrap().into_iter().map(|turn| turn.id).collect();
    assert_eq!(its, [now.clone(), "turn_before_0".to_string()]);
    let given = numbers(&rig).await;
    assert_eq!(
        given.iter().filter(|(_, nth)| nth.is_none()).count(),
        2,
        "nothing is written in a row from before"
    );
    assert!(given.iter().find(|(id, _)| *id == now).unwrap().1 > Some(2), "{given:?}");
}

#[tokio::test]
async fn a_message_has_one_turn_for_an_agent_though_the_agent_was_given_to_another_project_since() {
    let mut rig = Rig::new().await;
    rig.agent("builder", "fake").await;
    rig.studio.projects.create("Yard", None, &ada()).await.unwrap();
    rig.fake.will_say("It is up.");
    rig.say("@builder put up a shelf").await;
    rig.turns_finished(1).await;

    // The agent is given to another project, so that what it is asked
    // from now on is kept in another file; and a daemon starts that has
    // every message to hear again, as one does that was stopped before
    // it had kept where it had heard to.
    let spec = SlotSpec {
        name: "builder".into(),
        adapter: Some("fake".into()),
        project: Some("yard".into()),
        channel: Some("shop".into()),
        ..SlotSpec::default()
    };
    rig.agents.ensure(spec, &ada()).await.unwrap();
    rig.stop();
    rig.journal.kv_set("agents", "cursor", &json!(0)).await.unwrap();
    rig.start().await;
    rig.fake.will_say("And the gate.");
    rig.say("@builder and a gate").await;
    rig.turns_finished(2).await;

    // Heard twice, the first message was answered once.
    let turns = rig.agents.turns("builder", 10).await.unwrap();
    let asked: Vec<&str> = turns.iter().map(|turn| turn.prompt.as_str()).collect();
    assert_eq!(turns.len(), 2, "{asked:?}");
    assert_eq!(rig.fake.prompts().len(), 2);
    if journal::apart() != Some(false) {
        let kept = async |project: &str| {
            let read = rig.turns_of(project).db().call(|connection| {
                connection.query_row("SELECT COUNT(*) FROM turns", [], |row| row.get::<_, i64>(0))
            });
            read.await.unwrap()
        };
        assert_eq!((kept("shop").await, kept("yard").await), (1, 1));
    }
}

#[tokio::test]
async fn a_project_whose_agents_turns_the_factorys_journal_holds_is_given_a_journal_that_keeps_none_of_them()
{
    if crate::one_journal() {
        return;
    }
    let rig = Rig::new().await;
    // As a factory from before holds a project that was taken away: an
    // agent of it, and what the agent was asked, in the factory's own.
    agent(&rig, "yard-helper", None).await;
    let wrote = rig.journal.db().call(|connection| {
        connection.execute("UPDATE slots SET project = 'yard' WHERE name = 'yard-helper'", [])?;
        connection.execute(
            "INSERT INTO turns (id, slot, kind, thread, prompt, options, status, created, finished)
             VALUES ('turn_before', 'yard-helper', 'task', 'seat:1', 'in seat:1', '[]', 'done', 1, 2)",
            [],
        )
    });
    wrote.await.unwrap();

    // Added again, it has a journal of its own for what is run for it,
    // and its turns stay where they are: what its agent is asked now is
    // kept beside what it was asked before, and both are its.
    rig.studio.projects.create("Yard", None, &ada()).await.unwrap();
    assert_eq!(rig.journal.keeps_of("yard"), journal::keeps::RUNS);
    assert!(rig.turns_of("yard").db().is(rig.journal.db()));
    let now = asked(&rig, "yard-helper", "seat:1").await;
    let its: Vec<String> =
        rig.agents.turns("yard-helper", 10).await.unwrap().into_iter().map(|turn| turn.id).collect();
    assert_eq!(its, [now, "turn_before".to_string()]);
    // One with nothing there is made as a journal is made now.
    rig.studio.projects.create("Shop", None, &ada()).await.unwrap();
    assert_eq!(rig.journal.keeps_of("shop"), journal::keeps::MADE_AT);
    assert!(!rig.turns_of("shop").db().is(rig.journal.db()));
}

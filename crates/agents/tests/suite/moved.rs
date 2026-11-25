//! What the agents hand whoever moves a project of a factory from before
//! to a journal of its own (`agents::moved`): the turns that are the
//! project's, by what each says and never by whose its agent is now,
//! written where they go saying their number and their project, in the
//! order they were asked; and which of them are in the way of a move.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use agents::moved::in_hand;
use agents::{SlotSpec, TurnRequest};
use journal::moved::Row;
use kernel::ManualClock;
use rusqlite::Connection;

use crate::{Rig, ada};

/// A folder of this test's own, taken away when the test is over.
struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn scratch(label: &str) -> Scratch {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("agents-{label}-{}", &kernel::id::secret()[..12]));
    std::fs::create_dir_all(&dir).unwrap();
    Scratch(dir)
}

/// A journal's file called `name` with the agents' tables in it, as a
/// daemon makes them, closed.
async fn made(file: &Path, name: &str) {
    let journal = journal::moved::made(file, name, Arc::new(ManualClock::new())).await.unwrap();
    agents::apart::make(journal.db()).await.unwrap();
    journal.db().close();
    journal.db().closed().await;
}

/// An agent of `project`, or of none, as the list of agents says now.
fn agent(connection: &Connection, name: &str, project: Option<&str>) {
    let wrote = connection.execute(
        "INSERT INTO slots (name, adapter, project, instructions, scopes, permission_mode, active, created, updated)
         VALUES (?1, 'fake', ?2, '', '[]', 'edit', 1, 1, 1)",
        rusqlite::params![name, project],
    );
    wrote.unwrap();
}

/// A turn of `slot`: saying `project` and which turn of all it is, or
/// neither, as one from before does.
fn turn(connection: &Connection, id: &str, slot: &str, said: Option<(&str, i64)>, status: &str) {
    let (project, nth) = (said.map(|(project, _)| project), said.map(|(_, nth)| nth));
    let wrote = connection.execute(
        "INSERT INTO turns (id, slot, kind, thread, prompt, options, status, created, project, nth)
         VALUES (?1, ?2, 'task', 'seat:1', 'in seat:1', '{}', ?3, 1, ?4, ?5)",
        rusqlite::params![id, slot, status, project, nth],
    );
    wrote.unwrap();
}

/// The turns a file holds, in the order of its rows: each with the
/// number and the project it says.
fn turns(file: &Path) -> Vec<(String, Option<i64>, Option<String>)> {
    let connection = Connection::open(file).unwrap();
    let mut statement = connection.prepare("SELECT id, nth, project FROM turns ORDER BY rowid").unwrap();
    let rows = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?))).unwrap();
    rows.collect::<rusqlite::Result<_>>().unwrap()
}

#[tokio::test]
async fn a_turn_in_hand_is_in_the_way_of_moving_the_project_it_says_whoever_its_agent_is_of_by_now() {
    let dir = scratch("in-hand");
    let factory = dir.0.join("journal.db");
    made(&factory, "factory").await;
    let connection = Connection::open(&factory).unwrap();
    // An agent that was the shop's and was given to the yard since, one
    // that is nobody's, and one that was taken away.
    agent(&connection, "helper", Some("yard"));
    agent(&connection, "nobodys", None);
    turn(&connection, "turn_waits", "helper", Some(("shop", 1)), "queued");
    turn(&connection, "turn_answered", "helper", Some(("shop", 2)), "answered");
    turn(&connection, "turn_of_one_gone", "gone", Some(("shop", 3)), "running");
    turn(&connection, "turn_before", "helper", None, "queued");
    turn(&connection, "turn_done", "helper", Some(("shop", 5)), "done");
    turn(&connection, "turn_nobodys", "nobodys", Some(("", 6)), "running");

    // What is in the way of moving the shop is what says it is the
    // shop's and has not ended, though no agent is the shop's any more;
    // and a turn from before goes by its agent as that is now.
    let of = |project: &str| -> Vec<(String, String)> {
        let found = in_hand(&connection, project).unwrap();
        found.into_iter().map(|turn| (turn.turn, turn.status)).collect()
    };
    let (text, pair) = (str::to_string, |id: &str, status: &str| (id.to_string(), status.to_string()));
    assert_eq!(
        of("shop"),
        [
            pair("turn_answered", "answered"),
            pair("turn_of_one_gone", "running"),
            pair("turn_waits", "queued")
        ]
    );
    assert_eq!(of("yard"), [pair("turn_before", "queued")]);
    let agents: Vec<String> =
        in_hand(&connection, "shop").unwrap().into_iter().map(|turn| turn.agent).collect();
    assert_eq!(agents, ["helper", "gone", "helper"].map(text));
}

#[tokio::test]
async fn turns_moved_with_a_project_say_their_number_and_whose_they_are_in_the_order_they_were_asked() {
    let dir = scratch("turns");
    let (factory, file) = (dir.0.join("journal.db"), dir.0.join("shop.db"));
    made(&factory, "factory").await;
    {
        let connection = Connection::open(&factory).unwrap();
        agent(&connection, "shop-helper", Some("shop"));
        agent(&connection, "nobodys", None);
        // Was the shop's, and is the yard's now.
        agent(&connection, "given", Some("yard"));
        // From before a turn said anything: each is of its agent's
        // project as that is now.
        turn(&connection, "before_shops", "shop-helper", None, "done");
        turn(&connection, "before_nobodys", "nobodys", None, "done");
        turn(&connection, "before_given", "given", None, "done");
        // Since: each is whose it says, whoever its agent is of now.
        turn(&connection, "said_by_given", "given", Some(("shop", 10)), "done");
        turn(&connection, "said_shops", "shop-helper", Some(("shop", 11)), "failed");
        turn(&connection, "said_yards", "shop-helper", Some(("yard", 12)), "done");
        turn(&connection, "said_nobodys", "nobodys", Some(("", 13)), "done");
        turn(&connection, "said_of_one_gone", "gone", Some(("shop", 14)), "done");
    }
    made(&file, "project:shop").await;
    let tables: Vec<_> = agents::moved::tables().into_iter().map(|(_, kept)| kept).collect();
    assert!(agents::moved::tables().iter().all(|(level, _)| *level == journal::keeps::TURNS));
    let before = turns(&factory);

    let carried = journal::moved::carry(&factory, &file, "shop", &tables, &[]).unwrap();
    assert_eq!((carried[0].table.as_str(), carried[0].moves, carried[0].all), ("turns", 4, 8));
    assert_eq!(turns(&factory), before, "written, and nothing decided");
    let row = Row {
        name: "project:shop".into(),
        project: "shop".into(),
        file: "shop.db".into(),
        keeps: journal::keeps::TURNS,
        made: 5,
    };
    let note = serde_json::json!({ "moved": "shop" });
    let kept = ("factory", "moved_apart");
    journal::moved::decide(&factory, &row, &tables, &[], &carried, kept, &note, std::time::Instant::now())
        .unwrap();

    // The shop's are in its file, in the order they were asked: the one
    // from before under the number of the row it was, and saying whose
    // it is, since the list of agents it was read through is not there.
    let shop = |id: &str, nth: i64| (id.to_string(), Some(nth), Some("shop".to_string()));
    assert_eq!(
        turns(&file),
        [
            shop("before_shops", 1),
            shop("said_by_given", 10),
            shop("said_shops", 11),
            shop("said_of_one_gone", 14)
        ]
    );
    // And every other is where it was, as it was.
    let went = turns(&file);
    let left: Vec<_> =
        before.into_iter().filter(|(id, ..)| !went.iter().any(|(there, ..)| there == id)).collect();
    assert_eq!(turns(&factory), left);
    let stayed: Vec<String> = turns(&factory).into_iter().map(|(id, ..)| id).collect();
    assert_eq!(stayed, ["before_nobodys", "before_given", "said_yards", "said_nobodys"]);
    // Everything else of a turn went as it was.
    let connection = Connection::open(&file).unwrap();
    let failed: (String, String, String) = connection
        .query_row("SELECT slot, status, thread FROM turns WHERE id = 'said_shops'", [], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })
        .unwrap();
    assert_eq!(failed, ("shop-helper".to_string(), "failed".to_string(), "seat:1".to_string()));
}

#[tokio::test]
async fn turns_left_in_the_factorys_file_of_a_project_that_keeps_its_own_go_among_those_there_in_the_order_asked()
 {
    let dir = scratch("strays");
    let (factory, file) = (dir.0.join("journal.db"), dir.0.join("shop.db"));
    made(&factory, "factory").await;
    made(&file, "project:shop").await;
    {
        // The factory's file: two turns that say they are the shop's, of
        // an agent that is the yard's by now, the first from before turns
        // were numbered (it is the second row there); and the yard's
        // own, which stays.
        let connection = Connection::open(&factory).unwrap();
        agent(&connection, "given", Some("yard"));
        turn(&connection, "yards", "given", Some(("yard", 1)), "done");
        connection
            .execute(
                "INSERT INTO turns (id, slot, kind, thread, prompt, options, status, created, project)
                 VALUES ('second', 'given', 'task', 'seat:1', 'in seat:1', '{}', 'done', 1, 'shop')",
                [],
            )
            .unwrap();
        turn(&connection, "fourth", "given", Some(("shop", 7)), "done");
        // The shop's own file, which keeps its agents' turns: two that
        // were asked before and after those.
        let connection = Connection::open(&file).unwrap();
        turn(&connection, "third", "shop-helper", Some(("shop", 5)), "done");
        turn(&connection, "fifth", "shop-helper", Some(("shop", 9)), "done");
    }
    // The line is handed over whatever the project's journal keeps.
    let tables: Vec<_> = agents::moved::tables().into_iter().map(|(_, kept)| kept).collect();
    assert!(tables.iter().all(|kept| kept.strays));
    let carried = journal::moved::carry(&factory, &file, "shop", &tables, &[]).unwrap();
    assert_eq!((carried[0].moves, carried[0].all), (2, 3));
    let row = Row {
        name: "project:shop".into(),
        project: "shop".into(),
        file: "shop.db".into(),
        keeps: journal::keeps::TURNS,
        made: 5,
    };
    let note = serde_json::json!({ "moved": "shop" });
    let kept = ("factory", "moved_apart");
    journal::moved::decide(&factory, &row, &tables, &[], &carried, kept, &note, std::time::Instant::now())
        .unwrap();
    // Every turn of the shop's is in its file, the rows in the order
    // of their numbers: a file's newest row is its highest number.
    let shop = |id: &str, nth: i64| (id.to_string(), Some(nth), Some("shop".to_string()));
    assert_eq!(turns(&file), [shop("second", 2), shop("third", 5), shop("fourth", 7), shop("fifth", 9)]);
    assert_eq!(turns(&factory), [("yards".to_string(), Some(1), Some("yard".to_string()))]);
}

#[tokio::test]
async fn a_turn_the_factorys_file_keeps_for_a_project_is_read_though_its_agent_was_given_away_and_the_project_has_its_own_file()
 {
    if crate::one_journal() {
        return;
    }
    let mut rig = Rig::new().await;
    rig.studio.projects.create("Yard", None, &ada()).await.unwrap();
    // As a factory from before holds a project that was taken away: a
    // turn that says it is the shop's, in the factory's own file, of an
    // agent that was given to the yard since. Nothing in the list of
    // agents says the shop ever had one.
    let spec = |project: &str| SlotSpec {
        name: "helper".into(),
        adapter: Some("fake".into()),
        project: Some(project.into()),
        role: Some("helper".into()),
        ..SlotSpec::default()
    };
    rig.agents.ensure(spec("yard"), &ada()).await.unwrap();
    rig.stop();
    let wrote = rig.journal.db().call(|connection| {
        connection.execute(
            "INSERT INTO turns (id, slot, kind, thread, prompt, options, status, created, finished, project)
             VALUES ('turn_shops', 'helper', 'task', 'seat:1', 'in seat:1', '[]', 'done', 1, 2, 'shop')",
            [],
        )
    });
    wrote.await.unwrap();
    rig.start().await;

    // Added again, the shop is given a journal as one is made now,
    // which keeps its agents' turns: the question that would have it
    // keep less asks the agents that are the shop's, and none is.
    rig.studio.projects.create("Shop", None, &ada()).await.unwrap();
    assert_eq!(rig.journal.keeps_of("shop"), journal::keeps::MADE_AT);
    assert!(!rig.turns_of("shop").db().is(rig.journal.db()));

    // The turn is where it was, and is read wherever a turn is asked
    // for: by its id, among its agent's, and for who holds the
    // conversation it was asked in; and what its agent is asked next
    // comes after it.
    assert_eq!(rig.agents.turn("turn_shops").await.unwrap().slot, "helper");
    let its = async |rig: &Rig| -> Vec<String> {
        rig.agents.turns("helper", 10).await.unwrap().into_iter().map(|turn| turn.id).collect()
    };
    assert_eq!(its(&rig).await, ["turn_shops"]);
    assert_eq!(rig.agents.holding("seat:1").await.unwrap().map(|slot| slot.name).as_deref(), Some("helper"));
    let request = TurnRequest {
        slot: "helper".into(),
        kind: "task".into(),
        thread: "seat:1".into(),
        prompt: "in seat:1".into(),
        ..TurnRequest::default()
    };
    let next = rig.agents.enqueue(request).await.unwrap();
    rig.agents.wait(&next).await.unwrap();
    assert_eq!(its(&rig).await, [next.clone(), "turn_shops".to_string()]);
    // And so it is for a daemon that starts and reads it all again;
    // and one that finds such a turn still waiting there takes it up
    // where it is, and ends it there.
    rig.stop();
    let wrote = rig.journal.db().call(|connection| {
        connection.execute(
            "INSERT INTO turns (id, slot, kind, thread, prompt, options, status, created, project, nth)
             VALUES ('turn_waits', 'helper', 'task', 'seat:1', 'in seat:1', '[]', 'queued', 3, 'shop', 3)",
            [],
        )
    });
    wrote.await.unwrap();
    rig.start().await;
    assert_eq!(rig.agents.wait("turn_waits").await.unwrap().status, "done");
    assert_eq!(its(&rig).await, ["turn_waits".to_string(), next, "turn_shops".to_string()]);
    assert_eq!(rig.agents.turn("turn_shops").await.unwrap().status, "done");
    let kept = async |journal: &journal::Journal| -> Vec<(String, String)> {
        let read = journal.db().call(|connection| {
            let mut statement = connection.prepare("SELECT id, status FROM turns ORDER BY rowid")?;
            statement
                .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
                .collect::<rusqlite::Result<Vec<_>>>()
        });
        read.await.unwrap()
    };
    let done = |id: &str| (id.to_string(), "done".to_string());
    assert_eq!(kept(&rig.journal).await, [done("turn_shops"), done("turn_waits")]);
    assert!(kept(&rig.turns_of("shop")).await.is_empty());
}

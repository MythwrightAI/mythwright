//! A project of a factory from before is moved whole (`api::moved`): with
//! what is run for it and what was said of it go its tables, their
//! records, each record's history, what a scan last saw of its files and
//! what its agents were asked. One journey, in one factory: a project
//! with a board and a fleet that has worked cards, moved through both
//! cuts of a move; and beside it a project at each level a journal can
//! have been made at, each taken the rest of the way by the same asking
//! (of one made now, a turn the factory's file was left holding for it,
//! of an agent that is another project's), and one from before that
//! nobody asked for, which is as it was.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use api::moved::Stop;
use journal::keeps;
use serde_json::{Value, json};

use crate::Rig;
use crate::journals::{card, history};
use crate::lanes::{at_rest, finished, git, move_card, moved, worked};
use crate::moved::{kept_less, listed};

/// The agents of the journey: the shop's two workers, and one brought
/// up by hand for each of the projects beside it that are moved.
const AGENTS: [&str; 5] = ["shop-worker-1", "shop-worker-2", "one-helper", "three-helper", "two-helper"];
/// The projects of the journey, in the order of their names.
const PROJECTS: [&str; 5] = ["one", "shop", "three", "two", "yard"];
/// Those of them that have a board.
const BOARDS: [&str; 4] = ["one", "shop", "two", "yard"];

/// A node whose agents can be given `writer`, which says it changed a
/// file, and where its data is.
async fn rig() -> (Rig, PathBuf) {
    let mut data = PathBuf::new();
    let rig = Rig::with("moved-whole", |config, _| {
        let writes = ["swallow", "append", "CHANGES.md", "changed", "say", "I changed a file."];
        config.adapters.push(("writer".into(), crate::standin::argv(&writes)));
        data = config.data_dir.clone();
    });
    (Rig::owned(rig.await).await, data)
}

/// Does something to the factory's own database while the daemon is
/// stopped, as a program from before left it.
fn left(data: &Path, sql: &str) {
    let connection = journal::copy::Connection::open(data.join("journal.db")).unwrap();
    connection.execute_batch(sql).unwrap_or_else(|err| panic!("{sql}: {err}"));
}

/// What a statement counts in a database's file, read while no daemon
/// has it.
fn count(file: &Path, sql: &str) -> i64 {
    let connection = journal::copy::Connection::open(file).unwrap();
    connection.query_row(sql, [], |row| row.get(0)).unwrap_or_else(|err| panic!("{sql}: {err}"))
}

/// Has the agent `<project>-helper` asked something, as work of the
/// project's, and waits for it to have answered: a turn of its own.
async fn asked(rig: &Rig, project: &str, n: usize) {
    let name = format!("{project}-helper");
    rig.send("/api/agents", json!({ "name": name, "adapter": "writer", "project": project })).await;
    let source = format!(
        "blueprint: asks-{n}\nsteps:\n  ask: {{ do: agent.task, with: {{ slot: {name}, prompt: \"Say hello.\" }} }}\n"
    );
    let applied = rig.send("/api/apply", json!({ "source": source, "project": project, "wait": true })).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
}

/// The turns an agent was given, the newest first: each by its id.
async fn turns(rig: &Rig, agent: &str) -> Vec<String> {
    let turns = rig.read(&format!("/api/agents/{agent}/turns")).await;
    turns["turns"].as_array().unwrap().iter().map(|turn| turn["id"].as_str().unwrap().to_string()).collect()
}

/// What whoever looks at the factory sees of what is kept for its
/// projects.
#[derive(Debug, PartialEq)]
struct Seen {
    /// Each project's cards, with the column each is in.
    cards: BTreeMap<String, Vec<(String, String)>>,
    /// Each card's history, as it is handed out.
    histories: BTreeMap<String, Value>,
    /// Each agent's turns, the newest first.
    turns: BTreeMap<String, Vec<String>>,
    /// How many lines of history were ever said to be written.
    lines: usize,
}

/// What is seen, waited for as long as a request may take and no longer,
/// saying so where it never came.
async fn seen(rig: &Rig) -> Seen {
    crate::in_time("what whoever looks at the factory sees of its projects", seeing(rig)).await
}

async fn seeing(rig: &Rig) -> Seen {
    let (mut cards, mut histories, mut asked) = (BTreeMap::new(), BTreeMap::new(), BTreeMap::new());
    for project in BOARDS {
        let board = rig.read(&format!("/api/board/{project}")).await;
        let mut on: Vec<(String, String)> = Vec::new();
        for card in board["items"].as_array().unwrap() {
            let id = card["id"].as_str().unwrap().to_string();
            histories
                .insert(id.clone(), rig.read(&format!("/api/records/{id}/history")).await["history"].clone());
            on.push((id, card["status"].as_str().unwrap().to_string()));
        }
        on.sort();
        cards.insert(project.to_string(), on);
    }
    for agent in AGENTS {
        asked.insert(agent.to_string(), turns(rig, agent).await);
    }
    let lines = crate::soon("the lines of history said", rig.count_of("collection.history")).await;
    Seen { cards, histories, turns: asked, lines }
}

/// Waits until no run is under way and no agent has a turn in hand, and
/// says what still was where that never came.
async fn rested(rig: &Rig) {
    let mut over = rig.follow_after(rig.mark(), "run.finished,agent.turn.finished");
    let mut still = Vec::new();
    let rested = tokio::time::timeout(crate::PATIENCE, async {
        loop {
            let (runs, turns, under_way) = crate::observed::at_work(rig).await;
            if runs + turns == 0 {
                return;
            }
            still = under_way;
            over.next().await.unwrap();
        }
    })
    .await;
    assert!(rested.is_ok(), "timed out waiting for nothing to be under way; still: {}", still.join(", "));
}

/// How often the factory is stopped and looked at before the journey says
/// that something stays in the way of a move.
const LOOKS: usize = 10;

/// How much of each kind the factory's own database holds of `project`
/// that would go with it, leaving out what there is none of.
fn would(asked: &api::moved::Looked, project: &str) -> BTreeMap<String, u64> {
    let of = asked.projects.iter().find(|one| one.id == project).unwrap();
    api::moved::went(&of.rows, false).into_iter().filter(|(_, rows)| *rows > 0).collect()
}

#[tokio::test]
async fn a_project_is_moved_with_its_cards_their_history_and_its_agents_turns_from_whichever_level_it_was_at()
{
    if crate::one_journal() {
        return;
    }
    let (mut rig, data) = rig().await;
    let mut dir = PathBuf::new();
    for name in ["Shop", "Yard", "One", "Two", "Three"] {
        let made = rig.send("/api/projects", json!({ "name": name })).await;
        if name == "Shop" {
            dir = PathBuf::from(made["path"].as_str().unwrap());
        }
    }
    rig.stop().await;
    // As a factory an earlier program made holds them: `shop`, `yard`
    // and `one` with no journal but the factory's, and `two` with one
    // that keeps its tables and not its agents' turns. And two turns:
    // one the shop's worker was given before a turn said whose it is, or
    // which it is, with the worker it was given to, which was there
    // then (a turn of a name nobody has is an agent's that was taken
    // away, and no later one's of the name); and one that says it is of
    // `three`, whose journal keeps turns, of an agent that was `three`'s
    // then and is another project's by now.
    kept_less(&data, "two", keeps::RECORDS);
    left(
        &data,
        "DELETE FROM journals WHERE project IN ('shop', 'yard', 'one');
         INSERT INTO slots (name, adapter, project, instructions, scopes, permission_mode, active, created, updated)
             VALUES ('shop-worker-1', 'worker', 'shop', '', '[]', 'edit', 0, 1, 1),
                    ('one-helper', 'writer', 'three', '', '[]', 'edit', 0, 1, 1);
         INSERT INTO turns (id, slot, kind, thread, prompt, options, status, created, finished)
             VALUES ('turn_before', 'shop-worker-1', 'task', 'lane:shop-0', 'before', '[]', 'done', 1, 2);
         INSERT INTO turns (id, slot, kind, thread, prompt, options, status, created, finished, project)
             VALUES ('turn_given', 'one-helper', 'task', 'seat:1', 'before', '[]', 'done', 1, 2, 'three');",
    );
    for project in ["shop", "yard", "one"] {
        journal::moved::put_away(&data.join("journals").join(format!("{project}.db")));
    }
    rig.start().await;

    // A board for each but the one made now, with a card that was
    // added and taken in hand; and for the two whose journals keep less
    // than one made now, an agent that was asked something. `one` is
    // taken away and added again once it has a board: it is given a
    // journal that keeps what is run for it and no more, as the
    // factory's file holds its table.
    rig.board("one").await;
    let (status, gone) = rig.call(reqwest::Method::DELETE, "/api/projects/one", Some(&rig.token), None).await;
    assert_eq!(status, reqwest::StatusCode::OK, "{gone}");
    rig.send("/api/projects", json!({ "name": "One" })).await;
    let journal = rig.node().app.journal.clone();
    let levels = |journal: &journal::Journal| PROJECTS.map(|project| journal.keeps_of(project));
    assert_eq!(levels(&journal), [keeps::RUNS, keeps::NOTHING, keeps::TURNS, keeps::RECORDS, keeps::NOTHING]);
    let mut cards = BTreeMap::new();
    for project in ["one", "two", "yard"] {
        if project != "yard" {
            asked(&rig, project, 1).await;
        }
        if project != "one" {
            rig.board(project).await;
        }
        cards.insert(project, card(&rig, project).await);
    }
    asked(&rig, "three", 1).await;

    // The shop: a repository, a board, and a fleet of two who work, the
    // one given least so far first, with a person to review. One card
    // is worked and done, one is worked and waits for a person, and one
    // has something said on it. Said last, so that every line of every
    // history before it is written when its own is.
    git(&dir, &["init", "--quiet", "."]);
    git(&dir, &["config", "user.name", "Ada"]);
    git(&dir, &["config", "user.email", "ada@example.com"]);
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "--quiet", "-m", "first"]);
    rig.board("shop").await;
    let stages = json!([
        { "title": "work", "column": "ready", "does": "work", "role": "worker", "pass": "review", "picks": "least" },
        {
            "title": "review", "column": "review", "does": "review", "role": "reviewer", "seats": 0,
            "needed": 0, "pass": "done", "fail": "doing", "rounds": 2, "merge": true, "in_flight": "finish",
        },
    ]);
    crate::lanes::fleet(&rig, json!({ "workers": 2, "stages": stages })).await;
    // Each is said to wait for a person before anybody moves it on: a
    // card a person finishes first was never waited on, and nobody is
    // told of it.
    let waited = async |id: &str| {
        let id = id.to_string();
        rig.event("card.waits", move |event| event.data["card"] == id.as_str()).await;
    };
    let done = worked(&rig, "Basket").await;
    waited(&done).await;
    finished(&rig, &done).await;
    let waits = worked(&rig, "Till").await;
    waited(&waits).await;
    let spoken = rig.send("/api/board/shop/items", json!({ "title": "Sign" })).await["id"]
        .as_str()
        .unwrap()
        .to_string();
    let mark = rig.mark();
    rig.send(&format!("/api/records/{spoken}/comments"), json!({ "body": "Above the door." })).await;
    rig.event("run.finished", |event| event.seq > mark && event.data["flow"] == "on.said").await;
    assert_eq!(history(&rig, &spoken, 2).await, ["board.item.created", "record.commented"]);
    // And what a scan sees of the shop's files is written down.
    rig.send("/api/projects/shop/scan", json!({})).await;
    // Something addressed to the owner that opens the shop's channel: a
    // line of their inbox, which is the factory's and no project's.
    let channels = rig.read("/api/channels").await;
    let of_shop = |channel: &&Value| channel["project"] == "shop";
    let channel = channels["channels"].as_array().unwrap().iter().find(of_shop).unwrap()["id"].clone();
    let owner = rig.read("/api/me").await["actor"].as_str().unwrap().to_string();
    let line = studio::inbox::Tell {
        owner,
        kind: studio::inbox::Kind::Told,
        about: "shop".into(),
        place: channel.as_str().unwrap().into(),
        from: "bot:factory".into(),
        why: "told".into(),
        says: Some("the shop is ready".into()),
        said_by: None,
        parts: None,
    };
    rig.node().app.studio.channels.inbox.tell(line).await.unwrap().unwrap();

    // Stopped at rest, with what there is to see written down.
    let (factory, journals) = (data.join("journal.db"), data.join("journals"));
    let mut looks = Vec::new();
    let (before, asked_first) = loop {
        rested(&rig).await;
        let before = seen(&rig).await;
        rig.stop().await;
        let asked = api::moved::look(&data, None).unwrap();
        let in_the_way: Vec<String> =
            asked.projects.iter().flat_map(|project| project.in_the_way.clone()).collect();
        if in_the_way.is_empty() {
            break (before, asked);
        }
        // Something had only just been heard: let it finish.
        looks.push(in_the_way.join("; "));
        assert!(
            looks.len() < LOOKS,
            "something stayed in the way of a move, each time it was looked at:\n{}",
            looks.join("\n")
        );
        rig.start().await;
    };
    let given = |agent: &str| before.turns[agent].len();
    assert_eq!(given("shop-worker-1") + given("shop-worker-2"), 3, "{:?}", before.turns);
    assert_eq!(before.cards["shop"].len(), 3, "{:?}", before.cards);

    // Asked first, it says what of each kind would go, by what the
    // project's journal does not keep yet, and changes nothing.
    let state: Vec<(bool, bool)> =
        asked_first.projects.iter().map(|project| (project.moves(), project.apart())).collect();
    assert_eq!(state, [(true, false); 5], "none is kept apart whole, the one made now neither");
    let shops = would(&asked_first, "shop");
    let above = ["collections", "records", "record_history", "project_files", "turns"];
    for table in ["instances", "runs", "events"].iter().chain(&above) {
        assert!(shops.contains_key(*table), "nothing of `{table}` would go with the shop: {shops:?}");
        assert!(!api::moved::rows_of(table).contains('_'), "`{table}` is said to nobody in words");
    }
    assert_eq!(shops["turns"], 3);
    // What is said of `one` is in its journal already, and what was
    // run for it since; its tables and its turns are not.
    let ones = would(&asked_first, "one");
    assert!(above.iter().all(|table| *table == "project_files" || ones.contains_key(*table)), "{ones:?}");
    assert!(!ones.contains_key("events"), "{ones:?}");
    assert_eq!(would(&asked_first, "two"), BTreeMap::from([("turns".to_string(), 1)]));
    // And of the one made now, the turn that says it is its own.
    assert_eq!(would(&asked_first, "three"), BTreeMap::from([("turns".to_string(), 1)]));
    assert_eq!(listed(&data).len(), 3);

    // What the factory's file and a project's hold of the shop.
    let shop = journals.join("shop.db");
    let held = |file: &Path| {
        [
            "SELECT count(*) FROM collections WHERE project = 'shop'",
            "SELECT count(*) FROM records WHERE project = 'shop'",
            "SELECT count(*) FROM record_history WHERE record LIKE 'shop-%'",
            "SELECT count(*) FROM project_files WHERE project = 'shop'",
            "SELECT count(*) FROM turns WHERE slot LIKE 'shop-worker-%'",
        ]
        .map(|sql| count(file, sql))
    };
    let was = held(&factory);
    assert!(was.iter().all(|rows| *rows > 0), "{was:?}");

    // Cut off with the project's file written and nothing decided: the
    // shop is as it was, every row of it in the factory's file.
    let cut = async |project: &str, stop| api::moved::run(&data, Some(project), stop, |_| {}).await.unwrap();
    assert!(cut("shop", Some(Stop::Written)).await.is_empty());
    assert_eq!(listed(&data).len(), 3);
    assert_eq!((held(&factory), held(&shop)), (was, was));
    // Cut off with the move decided: it is moved, and the factory's
    // file holds none of it.
    assert!(cut("shop", Some(Stop::Decided)).await.is_empty());
    assert!(listed(&data).contains(&("shop".to_string(), "shop.db".to_string(), keeps::MADE_AT, true)));
    assert_eq!((held(&factory), held(&shop)), ([0; 5], was));
    // Nothing of anybody's inbox went with it: the line that opens its
    // channel is in the factory's file (beside what its fleet told, a
    // card that waits in review for a person), and its own has no such
    // table.
    assert_eq!(count(&factory, "SELECT count(*) FROM inbox WHERE why = 'told'"), 1);
    assert_eq!(count(&shop, "SELECT count(*) FROM sqlite_master WHERE name = 'inbox'"), 0);
    // Its turns are in its file in the order they were asked: the one
    // from before first, under the number of the row it was, and every
    // one says whose it is.
    {
        let connection = journal::copy::Connection::open(&shop).unwrap();
        let mut statement = connection.prepare("SELECT id, nth, project FROM turns ORDER BY rowid").unwrap();
        let rows = statement.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, Option<i64>>(1)?, row.get::<_, Option<String>>(2)?))
        });
        let rows: Vec<_> = rows.unwrap().collect::<rusqlite::Result<_>>().unwrap();
        assert_eq!(rows[0], ("turn_before".to_string(), Some(1), Some("shop".to_string())), "{rows:?}");
        assert!(rows.windows(2).all(|pair| pair[0].1 < pair[1].1), "{rows:?}");
        assert!(rows.iter().all(|(_, nth, project)| nth.is_some() && project.as_deref() == Some("shop")));
    }

    // The same for a project whose journal keeps what is run: its file
    // is written beside the one it has, and its row names the new one
    // once it is decided.
    let ones = "SELECT count(*) FROM records WHERE project = 'one'";
    assert!(cut("one", Some(Stop::Written)).await.is_empty());
    assert!(listed(&data).contains(&("one".to_string(), "one.db".to_string(), keeps::RUNS, false)));
    assert_eq!((count(&factory, ones), count(&journals.join("one.2.db"), ones)), (1, 1));
    assert!(cut("one", Some(Stop::Decided)).await.is_empty());
    assert!(listed(&data).contains(&("one".to_string(), "one.2.db".to_string(), keeps::MADE_AT, false)));
    assert_eq!((count(&factory, ones), count(&journals.join("one.2.db"), ones)), (0, 1));
    // One whose journal keeps its tables has its agents' turns moved.
    // One made now has the turn the factory's file held for it written
    // among its own, before the one its agent was asked since, as it
    // was asked before it. And nobody asked for the yard.
    let moved_two = cut("two", None).await;
    assert_eq!(moved_two.iter().map(|moved| moved.file.as_str()).collect::<Vec<_>>(), ["two.2.db"]);
    let moved_three = cut("three", None).await;
    assert_eq!(moved_three.iter().map(|moved| moved.file.as_str()).collect::<Vec<_>>(), ["three.2.db"]);
    {
        let connection = journal::copy::Connection::open(journals.join("three.2.db")).unwrap();
        let mut statement = connection.prepare("SELECT id, slot, nth FROM turns ORDER BY rowid").unwrap();
        let rows = statement.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, i64>(2)?))
        });
        let rows: Vec<_> = rows.unwrap().collect::<rusqlite::Result<_>>().unwrap();
        assert_eq!(rows.len(), 2, "{rows:?}");
        assert_eq!((rows[0].0.as_str(), rows[0].1.as_str(), rows[0].2), ("turn_given", "one-helper", 2));
        assert!(rows[1].1 == "three-helper" && rows[1].2 > 2, "{rows:?}");
    }
    // The file `one` had before, which its row no longer names, was
    // taken away by the move as it was asked again.
    assert!(!journals.join("one.db").exists() && journals.join("one.2.db").exists());
    let rows_of_yard = "SELECT count(*) FROM records WHERE project = 'yard'";
    assert_eq!(count(&factory, rows_of_yard), 1);
    let now: BTreeMap<String, i64> =
        listed(&data).into_iter().map(|(project, _, keeps, _)| (project, keeps)).collect();
    assert_eq!(now.into_values().collect::<Vec<_>>(), [keeps::MADE_AT; 4], "all but the yard");
    let looked = api::moved::look(&data, None).unwrap();
    let state: Vec<bool> = looked.projects.iter().map(|project| project.apart()).collect();
    assert_eq!(state, [true, true, true, true, false]);

    // Started, the daemon says each move once; and everything there
    // was to see is as it was: every
    // card where it was, each card's history line for line, each
    // agent's turns in their order, and no line of history written
    // again by whoever keeps them.
    rig.start().await;
    rig.count("system.moved_apart", 4).await;
    assert_eq!(seen(&rig).await, before);
    // The owner's box as it was: the line that opens the shop's channel,
    // and one for each card that waits in review for a person, told once.
    let inbox = rig.read("/api/inbox").await;
    let lines = inbox["lines"].as_array().unwrap();
    let why = |why: &str| lines.iter().filter(|line| line["why"] == why && line["count"] == 1).count();
    assert_eq!((&inbox["unread"], &lines[0]["says"]), (&json!(3), &json!("the shop is ready")), "{inbox}");
    assert_eq!((why("told"), why("review")), (1, 2), "{inbox}");
    // And so it is once everything a start has the fleet look at is
    // over: nothing found a card to say anything of.
    rested(&rig).await;
    assert_eq!(seen(&rig).await, before);
    let journal = rig.node().app.journal.clone();
    assert_eq!(levels(&journal), [keeps::TURNS, keeps::TURNS, keeps::TURNS, keeps::TURNS, keeps::NOTHING]);
    crate::apart::rows_are_kept_where_their_lines_say(&rig).await;
    crate::apart::held_to_the_lists(&journal).await;

    // A card made ready after the move is heard once by the stage that
    // works: it is taken once, by the worker that was given least
    // before the move, and its turn comes after every one before.
    let least = match given("shop-worker-1") < given("shop-worker-2") {
        true => "shop-worker-1",
        false => "shop-worker-2",
    };
    assert_ne!(given("shop-worker-1"), given("shop-worker-2"), "{:?}", before.turns);
    let mark = rig.mark();
    move_card(&rig, &spoken, "ready", None).await;
    let reviewed = moved(&rig, &spoken, "review", mark).await;
    at_rest(&rig, reviewed).await;
    let taken = journal::EventQuery {
        topics: "board.item.moved".into(),
        after: mark,
        limit: 1_000,
        ..Default::default()
    };
    let taken = rig.said(taken).await;
    let taken =
        taken.iter().filter(|event| event.data["id"] == spoken.as_str() && event.data["to"] == "doing");
    assert_eq!(taken.count(), 1);
    for worker in ["shop-worker-1", "shop-worker-2"] {
        let (now, was) = (turns(&rig, worker).await, &before.turns[worker]);
        match worker == least {
            true => assert_eq!(now[1..], was[..], "the turn asked since comes first: {now:?}"),
            false => assert_eq!(&now, was),
        }
    }
    // The card that waited for a person is finished by one.
    finished(&rig, &waits).await;
    // And beside the shop: each card is moved on and has one line
    // more, and each agent's next turn comes after the one before.
    for project in ["one", "two", "yard"] {
        let id = &cards[project];
        move_card(&rig, id, "review", None).await;
        let lines = history(&rig, id, 3).await;
        assert_eq!(lines, ["board.item.created", "board.item.moved", "board.item.moved"], "{project}");
    }
    for project in ["one", "two", "three"] {
        asked(&rig, project, 2).await;
        let (helper, was) = (format!("{project}-helper"), &before.turns[&format!("{project}-helper")]);
        let now = turns(&rig, &helper).await;
        assert_eq!((now.len(), &now[1..]), (was.len() + 1, &was[..]), "{project}");
    }
    // The agent that is `one`'s was given the turn that says `three`
    // before its own, and still says so.
    assert_eq!(before.turns["one-helper"].last().map(String::as_str), Some("turn_given"));
    // What was written since is where each project's rows are kept: its
    // own file for those that were moved, the factory's for the yard.
    let held = crate::apart::rows_are_kept_where_their_lines_say(&rig).await;
    for project in ["one", "shop", "two"] {
        for table in ["records", "record_history", "turns"] {
            let rows = held.iter().find(|(of, kept, _)| of == project && *kept == table);
            assert!(rows.is_some_and(|(_, _, rows)| *rows > 0), "`{table}` of `{project}`: {held:?}");
        }
    }
    {
        let rows = held.iter().find(|(of, kept, _)| of == "three" && *kept == "turns");
        assert!(rows.is_some_and(|(_, _, rows)| *rows == 3), "the turns of `three`: {held:?}");
    }
    assert_eq!(count(&rig.kept_in("yard"), rows_of_yard), 1);
}

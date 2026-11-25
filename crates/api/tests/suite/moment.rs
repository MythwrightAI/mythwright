//! A copy of a factory is of one moment: every file of it, the factory's
//! own and each project's journal, is as it was at the same instant,
//! though every one of them was being written to as the copy was taken.
//! So the copy is a factory that could have existed: nothing in one of
//! its files rests on something another of its files does not hold yet.
//! A row and what is said of it are one write in one file, so the copy
//! never holds one without the other: not a card, not a line of its
//! history, not a turn of an agent's. And a factory started from it has
//! each journal, every card, and every turn, those that were in hand
//! ended as a copy ends them; and goes on above every number the copy
//! came with.

use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;

use journal::copy::Connection;
use kernel::Source;
use serde_json::{Value, json};

use crate::snapshots::{Shared, snapshot};
use crate::{Rig, soon};

/// Keeps what it last heard struck, and says that it heard it: what is
/// struck is nobody's and is kept in the factory's own journal, and what
/// this keeps and says is its project's, in the project's.
const TALLY: &str = r#"
blueprint: tally
state: { heard: 0 }
on:
  struck:
    event: s.struck
    steps:
      kept: { do: set, with: { heard: "${event.data.n}" } }
      said: { do: emit, with: { topic: s.heard, n: "${event.data.n}" } }
"#;

/// Strikes `n` for the whole factory, and waits for each tally to say
/// it heard.
async fn strike(rig: &Rig, n: u64, tallies: &[String]) {
    let mark = rig.mark();
    rig.send("/api/emit/s.struck", json!({ "n": n })).await;
    for tally in tallies {
        rig.event("s.heard", |event| {
            event.seq > mark && event.data["n"] == json!(n) && event.source.instance.as_ref() == Some(tally)
        })
        .await;
    }
}

/// Asks the agent `slot` of the shop something, as work of the shop's,
/// in a blueprint of its own called after `n`: what was applied.
async fn ask(rig: &Rig, slot: &str, n: u64, wait: bool) -> Value {
    let source = format!(
        "blueprint: asks-{n}\nsteps:\n  ask: {{ do: agent.task, with: {{ slot: {slot}, prompt: \"Say hello.\" }} }}\n"
    );
    rig.send("/api/apply", json!({ "source": source, "project": "shop", "wait": wait })).await
}

/// Adds a card to the shop's board and takes it in hand: its id.
async fn card(rig: &Rig, title: &str) -> String {
    let made = rig.send("/api/board/shop/items", json!({ "title": title })).await;
    let id = made["id"].as_str().unwrap().to_string();
    crate::lanes::move_card(rig, &id, "doing", None).await;
    id
}

/// What a file of a copy holds of cards and of turns, rows and what was
/// said of them: held to each other, since each row and what is said
/// of it were one write in that file.
struct Rows {
    /// Each card, with the column its row says.
    cards: BTreeMap<String, String>,
    /// How many lines of history each card has.
    lines: BTreeMap<String, usize>,
    /// Each turn: its agent, what its row says became of it, and why.
    turns: BTreeMap<String, (String, String, Option<String>)>,
}

fn pairs(connection: &Connection, sql: &str) -> Vec<(String, String)> {
    let mut statement = connection.prepare(sql).unwrap();
    let rows = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?))).unwrap();
    rows.collect::<rusqlite::Result<_>>().unwrap()
}

/// Reads the cards and the turns of the copy's file `file`, and fails
/// where a row is there without what was said of it or the other way
/// round.
fn rows(file: &Path) -> Rows {
    let connection = Connection::open(file).unwrap();
    let called = file.file_name().unwrap().to_string_lossy().into_owned();
    // A card is there if and only if it was said to be added; and it is
    // in the column it was last said to be moved to.
    let cards: BTreeMap<String, String> = pairs(
        &connection,
        "SELECT id, json_extract(data, '$.status') FROM records WHERE collection = 'board'",
    )
    .into_iter()
    .collect();
    let mut said: BTreeMap<String, String> = BTreeMap::new();
    let of_cards = "SELECT json_extract(data, '$.id'), coalesce(json_extract(data, '$.to'), json_extract(data, '$.status'))
                    FROM events WHERE topic IN ('board.item.created', 'board.item.moved') ORDER BY seq";
    for (id, column) in pairs(&connection, of_cards) {
        said.insert(id, column);
    }
    assert_eq!(cards, said, "{called}: the cards it holds, and the cards it says it holds");
    // A line of a card's history is there if and only if it was said to
    // be written, and the event it is a line of is there too.
    let lines =
        pairs(&connection, "SELECT record, CAST(seq AS TEXT) FROM record_history ORDER BY record, seq");
    let news = "SELECT json_extract(data, '$.record'), CAST(json_extract(data, '$.seq') AS TEXT) FROM events
                WHERE topic = 'collection.history' ORDER BY 1, json_extract(data, '$.seq')";
    assert_eq!(lines, pairs(&connection, news), "{called}: the lines of history it holds, and those it says");
    let events: BTreeSet<String> =
        texts(&connection, "SELECT CAST(seq AS TEXT) FROM events").into_iter().collect();
    for (record, seq) in &lines {
        assert!(
            events.contains(seq),
            "{called} holds a line of {record} for the event {seq}, which it lacks"
        );
    }
    // A turn is there if and only if it was said to be asked for; one
    // that was said to have ended says how; and one that was in hand
    // was ended by the copy, and nothing says it ended.
    let mut turns = BTreeMap::new();
    for (id, slot) in pairs(&connection, "SELECT id, slot FROM turns") {
        let about = format!("SELECT status, coalesce(error, '') FROM turns WHERE id = '{id}'");
        let (status, error) = pairs(&connection, &about).remove(0);
        turns.insert(id, (slot, status, Some(error).filter(|error| !error.is_empty())));
    }
    let of_topic = |topic: &str| -> BTreeSet<String> {
        let sql = format!("SELECT json_extract(data, '$.turn') FROM events WHERE topic = '{topic}'");
        texts(&connection, &sql).into_iter().collect()
    };
    let asked: BTreeSet<String> = turns.keys().cloned().collect();
    assert_eq!(asked, of_topic("agent.turn.queued"), "{called}: the turns it holds, and those it says");
    let ended = of_topic("agent.turn.finished");
    for (id, (slot, status, error)) in &turns {
        let by_the_copy = error.as_deref() == Some(agents::copy::NOT_TAKEN_UP);
        assert!(["done", "failed"].contains(&status.as_str()), "{called}: a turn of {slot} is {status}");
        assert_eq!(
            ended.contains(id),
            !by_the_copy,
            "{called}: the turn {id} of {slot} is {status} ({error:?})"
        );
    }
    let mut history: BTreeMap<String, usize> = BTreeMap::new();
    for (record, _) in lines {
        *history.entry(record).or_default() += 1;
    }
    Rows { cards, lines: history, turns }
}

/// One file of a copy, read: what a check of the copy goes by.
struct Held {
    /// What the file is called in the copy.
    file: String,
    /// What the journal in it is called.
    journal: String,
    events: BTreeSet<i64>,
    instances: BTreeSet<String>,
    /// Each run: its instance, and the number of the event that set it
    /// off where one did.
    runs: Vec<(String, Option<i64>)>,
    /// The greatest thing struck that it holds, and that it holds heard.
    struck: i64,
    heard: i64,
    /// What its tallies keep as last heard.
    kept: i64,
}

fn texts(connection: &Connection, sql: &str) -> Vec<String> {
    let mut statement = connection.prepare(sql).unwrap();
    statement.query_map([], |row| row.get(0)).unwrap().collect::<rusqlite::Result<_>>().unwrap()
}

fn number(connection: &Connection, sql: &str) -> i64 {
    connection.query_row(sql, [], |row| row.get(0)).unwrap()
}

fn held(file: &Path, called: &str, journal: &str) -> Held {
    let connection = Connection::open(file).unwrap();
    let of_topic = |topic: &str| {
        let greatest = format!(
            "SELECT coalesce(max(CAST(json_extract(data, '$.n') AS INTEGER)), 0) FROM events WHERE topic = '{topic}'"
        );
        number(&connection, &greatest)
    };
    let runs = {
        let mut statement =
            connection.prepare("SELECT instance, json_extract(event, '$.seq') FROM runs").unwrap();
        statement
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap()
    };
    let numbers = texts(&connection, "SELECT CAST(seq AS TEXT) FROM events");
    Held {
        file: called.to_string(),
        journal: journal.to_string(),
        events: numbers.iter().map(|seq| seq.parse().unwrap()).collect(),
        instances: texts(&connection, "SELECT id FROM instances").into_iter().collect(),
        runs,
        struck: of_topic("s.struck"),
        heard: of_topic("s.heard"),
        kept: number(
            &connection,
            "SELECT coalesce(max(CAST(json_extract(state, '$.heard') AS INTEGER)), 0) FROM instances
             WHERE blueprint = 'tally'",
        ),
    }
}

#[tokio::test]
async fn a_copy_taken_while_two_projects_write_is_of_one_moment_and_a_factory_is_started_from_it() {
    if crate::one_journal() {
        return;
    }
    let shared = Shared::new();
    // The copy is held as each project's journal is about to be written,
    // until the test lets it go: by then every file is to have been read
    // as it was, whatever is written meanwhile.
    let (holds, held_at) = tokio::sync::mpsc::unbounded_channel::<String>();
    let (go, gone) = mpsc::channel::<()>();
    let gone = Mutex::new(gone);
    let registry = shared.registry();
    // Two programs to run agents on: one that answers at once, and one
    // that waits at a hold until it is let go.
    let pipe = Arc::new(Mutex::new(PathBuf::new()));
    let at = pipe.clone();
    let prod = Rig::with("api-moment-prod", move |config, dir| {
        let hold = dir.join("hold").to_string_lossy().into_owned();
        let waits = ["swallow", "held", &hold, "waits", "say", "Hello."];
        config.adapters.push(("waits".into(), crate::standin::argv(&waits)));
        config.adapters.push(("answers".into(), crate::standin::argv(&["swallow", "say", "Hello."])));
        *at.lock().unwrap() = dir.join("hold");
        config.profile = Some("prod".into());
        config.registry = Some(registry);
        config.copying = Some(Arc::new(move |file: &str| {
            let _ = holds.send(file.to_string());
            // Let go by the test, or by the test being over.
            let _ = gone.lock().unwrap().recv_timeout(Duration::from_secs(60));
        }));
    })
    .await;
    let prod = Rig::owned(prod).await;
    let mut held_at = held_at;

    let mut tallies = Vec::new();
    for (name, project) in [("Shop", "shop"), ("Yard", "yard")] {
        prod.send("/api/projects", json!({ "name": name })).await;
        let applied =
            prod.send("/api/apply", json!({ "source": TALLY, "project": project, "wait": true })).await;
        assert_eq!(applied["finished"]["status"], "ok", "{applied}");
        tallies.push(applied["instance"].as_str().unwrap().to_string());
    }
    // Something said in a project's own journal by the node itself, too.
    let shop = prod.node().app.journal.of_project("shop").unwrap();
    shop.emit("kept.here", Source::actor("system"), json!({ "project": "shop" })).await.unwrap();
    let mut n = 0;
    for _ in 0..2 {
        n += 1;
        strike(&prod, n, &tallies).await;
    }
    // And the shop has a board with a card, an agent that has answered
    // something, and one that has a turn in hand and another waiting
    // behind it, each asked for by a run that waits for it: its tables,
    // its cards and its agents' turns are kept in its own file.
    let pipe = pipe.lock().unwrap().clone();
    crate::hold(&pipe);
    prod.board("shop").await;
    let first = card(&prod, "Before").await;
    for (name, adapter) in [("shop-quick", "answers"), ("shop-held", "waits")] {
        prod.send("/api/agents", json!({ "name": name, "adapter": adapter, "project": "shop" })).await;
    }
    assert_eq!(ask(&prod, "shop-quick", 100, true).await["finished"]["status"], "ok");
    let mark = prod.mark();
    let waiting: Vec<String> =
        [ask(&prod, "shop-held", 101, false).await, ask(&prod, "shop-held", 102, false).await]
            .iter()
            .map(|applied| applied["instance"].as_str().unwrap().to_string())
            .collect();
    prod.event("agent.turn.started", |event| event.seq > mark && event.data["slot"] == "shop-held").await;
    soon("the second turn to wait behind the first", async {
        let mut queued = prod.follow_after(mark, "agent.turn.queued");
        for _ in 0..2 {
            while queued.next().await.unwrap().data["slot"] != "shop-held" {}
        }
    })
    .await;

    // Both projects go on writing, struck again and again, while the
    // copy is taken; and at the first project's file the copy is held
    // while more is struck and heard, something is said in a project's
    // own journal, and a project is added.
    let to = shared.snapshot("prod-copy");
    let (done, mut meanwhile, mut files, mut later) = (Cell::new(false), Vec::new(), Vec::new(), Vec::new());
    let (finished, ()) = tokio::join!(
        async {
            let finished = snapshot(&prod, &to).await;
            done.set(true);
            finished
        },
        async {
            while !done.get() && files.len() < 2 {
                n += 1;
                strike(&prod, n, &tallies).await;
                // A card is added and moved, and an agent asked and
                // answered, as the copy is taken.
                card(&prod, &format!("During {n}")).await;
                ask(&prod, "shop-quick", n, true).await;
                let Ok(file) = held_at.try_recv() else { continue };
                if files.is_empty() {
                    for _ in 0..2 {
                        n += 1;
                        strike(&prod, n, &tallies).await;
                        meanwhile.push(n as i64);
                    }
                    later.push(card(&prod, "Later").await);
                    let asked = ask(&prod, "shop-quick", 1_000, true).await;
                    later.push(asked["instance"].as_str().unwrap().to_string());
                    shop.emit("said.later", Source::actor("system"), json!({ "project": "shop" }))
                        .await
                        .unwrap();
                    prod.send("/api/projects", json!({ "name": "Den" })).await;
                }
                files.push(file);
                go.send(()).unwrap();
            }
        }
    );
    assert_eq!(finished["status"], "ok", "{finished}");
    assert_eq!(files, ["journals/shop.db", "journals/yard.db"], "the copy was held at each project's file");

    // The copy's files, read: the factory's own, and each journal it lists.
    let factory = Connection::open(to.join("database.db")).unwrap();
    let listed = journal::listed(&factory).unwrap();
    let mut there: Vec<String> = std::fs::read_dir(to.join("journals"))
        .unwrap()
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    there.sort();
    // A journal is listed where its file is there, and the other way
    // round: the project added meanwhile has neither.
    let names: Vec<&str> = listed.iter().map(|one| one.file.as_str()).collect();
    assert_eq!(names, ["shop.db", "yard.db"], "what the copy lists");
    assert_eq!(there, ["shop.db", "yard.db"], "what the copy holds");
    let mut all = vec![held(&to.join("database.db"), "database.db", journal::FACTORY)];
    for one in &listed {
        all.push(held(&to.join("journals").join(&one.file), &one.file, &one.name));
    }
    let (own, projects) = all.split_first().unwrap();

    // No event is numbered above what the copy's giver has given.
    let mark = number(
        &factory,
        "SELECT CAST(value AS INTEGER) FROM kv WHERE namespace = 'journal' AND key = 'drawn'",
    );
    for file in &all {
        let last = file.events.last().copied().unwrap_or(0);
        assert!(last <= mark, "{} holds an event numbered {last}, and the copy's mark is {mark}", file.file);
    }
    // No run is of an instance the copy lacks, or was set off by an
    // event the copy does not hold.
    let instances: BTreeSet<&String> = all.iter().flat_map(|file| &file.instances).collect();
    let events: BTreeSet<i64> = all.iter().flat_map(|file| file.events.iter().copied()).collect();
    for file in &all {
        for (instance, event) in &file.runs {
            assert!(
                instances.contains(instance),
                "{} holds a run of {instance}, which the copy lacks",
                file.file
            );
            let said = event.is_none_or(|event| event <= 0 || events.contains(&event));
            assert!(said, "{} holds a run set off by the event {event:?}, which the copy lacks", file.file);
        }
    }
    // No place that was kept is past the last event of the journal it
    // is a place in: how far each was read for the history of records.
    let read_to = texts(&factory, "SELECT value FROM kv WHERE namespace = 'history' AND key = 'seq'");
    let read_to = read_to.first().map(|kept| serde_json::from_str::<Value>(kept).unwrap());
    let read_to = read_to.and_then(|kept| journal::Places::from_kept(&kept)).unwrap_or_default();
    for file in &all {
        let (place, last) = (read_to.of(&file.journal) as i64, file.events.last().copied().unwrap_or(0));
        assert!(
            place <= last,
            "a place in `{}` is {place}, and the last event there is {last}",
            file.journal
        );
    }
    // And in the test's own words: each project heard, and keeps, only
    // what the factory's own file says was struck; both had heard
    // something; and what was struck while the copy was held at a file,
    // and heard by both, is in no file of it.
    assert!(own.struck >= 2, "the copy holds what was struck before it was asked for: {}", own.struck);
    for project in projects {
        assert!(
            project.heard <= own.struck && project.kept <= own.struck,
            "{} heard {} and keeps {}, and the factory's file says {} was the last struck",
            project.file,
            project.heard,
            project.kept,
            own.struck
        );
        assert!(project.heard >= 2 && project.kept >= 2, "{} heard {}", project.file, project.heard);
    }
    let struck = meanwhile[0];
    let latest = all.iter().map(|file| file.struck.max(file.heard).max(file.kept)).max().unwrap();
    assert!(latest < struck, "{struck} was struck after every file was read, and the copy holds {latest}");
    // The shop's cards and its agents' turns are in its own file, each
    // row with what was said of it and neither without the other; the
    // factory's file holds none of them; and what was added and asked
    // while the copy was held is in neither.
    let shops = rows(&to.join("journals").join("shop.db"));
    let (others, yards) = (rows(&to.join("database.db")), rows(&to.join("journals").join("yard.db")));
    assert!(
        others.cards.is_empty() && others.turns.is_empty(),
        "the factory's own file holds the shop's rows"
    );
    assert!(yards.cards.is_empty() && yards.turns.is_empty());
    assert_eq!(shops.cards.get(&first).map(String::as_str), Some("doing"), "{:?}", shops.cards);
    assert!(!shops.cards.contains_key(&later[0]), "a card added after every file was read is in the copy");
    assert!(all.iter().all(|file| !file.instances.contains(&later[1])), "what was asked after it, too");
    // The turn that was in hand and the one that waited behind it were
    // ended by the copy, and so were the runs that waited for them.
    let in_hand: Vec<&(String, String, Option<String>)> =
        shops.turns.values().filter(|(slot, ..)| slot == "shop-held").collect();
    assert_eq!(in_hand.len(), 2, "{:?}", shops.turns);
    for (_, status, error) in in_hand {
        assert_eq!((status.as_str(), error.as_deref()), ("failed", Some(agents::copy::NOT_TAKEN_UP)));
    }
    drop(factory);

    // Whoever waited at the pipe is let go, one and then the other, so
    // that the factory the copy was taken of ends what it has in hand.
    for nth in 1..=2 {
        let mark = prod.mark();
        crate::let_go("an agent to be waiting at its pipe", &pipe).await;
        prod.event("agent.turn.finished", |event| event.seq > mark && event.data["slot"] == "shop-held")
            .await;
        if nth == 1 {
            prod.event("agent.turn.started", |event| event.seq > mark && event.data["slot"] == "shop-held")
                .await;
        }
    }

    // A factory started from it comes up, with what both projects ran,
    // and goes on numbering above everything it came with.
    let dev = shared.copy("dev", &to).await;
    let called: Vec<String> =
        dev.node().app.journal.every().iter().map(|journal| journal.name().to_string()).collect();
    assert_eq!(called, ["factory", "project:shop", "project:yard"]);
    // What a project's journal held as it was read is there, and what
    // it was told since is not.
    let all = journal::EventQuery { limit: usize::MAX, ..Default::default() };
    let of_shop = dev.node().app.journal.of_project("shop").unwrap().read(all).await.unwrap();
    let said: Vec<&str> = of_shop.iter().map(|event| event.topic.as_str()).collect();
    assert!(said.contains(&"kept.here") && !said.contains(&"said.later"), "{said:?}");
    let came = dev.read("/api/instances?limit=1000").await;
    let came: Vec<&str> =
        came["instances"].as_array().unwrap().iter().map(|one| one["id"].as_str().unwrap()).collect();
    for tally in &tallies {
        assert!(came.contains(&tally.as_str()), "{tally} is not among {came:?}");
    }
    // Every card the copy held is on the shop's board, where it was,
    // with its history; every turn is there for its agent; and nothing
    // is in hand: the turns that were are ended, as the runs that waited
    // for them are, and no agent was asked anything by the copy.
    let board = dev.read("/api/board/shop").await;
    let on_it: BTreeMap<String, String> = board["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|card| (card["id"].as_str().unwrap().to_string(), card["status"].as_str().unwrap().to_string()))
        .collect();
    assert_eq!(on_it, shops.cards);
    for (card, lines) in &shops.lines {
        let history = dev.read(&format!("/api/records/{card}/history")).await;
        assert_eq!(history["history"].as_array().map(Vec::len), Some(*lines), "{card}: {history}");
    }
    for agent in ["shop-quick", "shop-held"] {
        let asked = dev.read(&format!("/api/agents/{agent}/turns")).await;
        let asked: BTreeMap<String, String> = asked["turns"]
            .as_array()
            .unwrap()
            .iter()
            .map(|turn| {
                (turn["id"].as_str().unwrap().to_string(), turn["status"].as_str().unwrap().to_string())
            })
            .collect();
        let held: BTreeMap<String, String> = shops
            .turns
            .iter()
            .filter(|(_, (slot, ..))| slot == agent)
            .map(|(id, (_, status, _))| (id.clone(), status.clone()))
            .collect();
        assert_eq!(asked, held, "the turns of {agent}");
    }
    assert_eq!(crate::observed::at_work(&dev).await.0, 0, "a run is under way in the copy");
    assert_eq!(crate::observed::at_work(&dev).await.1, 0, "a turn is in hand in the copy");
    for instance in &waiting {
        let one = dev.read(&format!("/api/instances/{instance}")).await;
        assert_ne!(one["instance"]["status"], "active", "{one}");
        assert_ne!(one["instance"]["status"], "starting", "{one}");
    }
    let highest = events.last().copied().unwrap().max(mark) as u64;
    let mut given = BTreeSet::new();
    for journal in dev.node().app.journal.every() {
        let project = journal::project_of(journal.name()).unwrap_or_default().to_string();
        let said = journal.emit("kept.here", Source::actor("system"), json!({ "project": project }));
        let said = soon("a journal of the copy to say something", said).await.unwrap();
        assert!(
            said.seq > highest,
            "`{}` gave {}, and the copy came with {highest}",
            journal.name(),
            said.seq
        );
        assert!(given.insert(said.seq), "{} was given twice", said.seq);
    }
    assert_eq!(given.len(), 3);
}

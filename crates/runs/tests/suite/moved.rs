//! A project of a journal from before, moved to a file of its own
//! (`journal::moved`, handed the tables kept here): every row of the
//! project goes and no other, by the lines every row is held to; a move
//! that is not decided leaves the journal as it was and a file nobody
//! names; and one is not decided over a journal that was written to
//! meanwhile.

use std::path::Path;
use std::sync::Arc;

use journal::moved::{Counted, Row};
use kernel::ManualClock;
use runs::{StepInfo, StepStatus, Store};
use rusqlite::Connection;

use crate::before::{closed, from_before, opened};
use crate::{scratch, step, told_apart};

/// How many runs long over the journal from before holds.
const ENDED: usize = 40;
/// Where what is done is kept for a daemon to say.
const KEPT: (&str, &str) = ("factory", "moved_apart");
/// The run of the project that is under way in the journal from before.
const UNDER_WAY: &str = "shop-fleet-stage.heard";

/// A journal from before at `path`, brought up to the tables this
/// program keeps and closed, as a factory that was started once by this
/// program and stopped.
async fn stopped(path: &Path) {
    let journal = from_before(path, ENDED).await;
    Store::open(&journal).await.unwrap();
    closed(journal).await;
}

/// The file a project's journal is to be kept in, made as one is made
/// now with the runner's tables in it, and closed.
async fn made(file: &Path, name: &str) {
    let journal = journal::moved::made(file, name, Arc::new(ManualClock::new())).await.unwrap();
    Store::open(&journal).await.unwrap();
    closed(journal).await;
}

fn row(file: &str) -> Row {
    Row { name: "project:shop".into(), project: "shop".into(), file: file.into(), keeps: 1, made: 5 }
}

/// How many rows of each table a file holds, every one of them held to
/// its table's line.
fn held(path: &Path) -> Vec<(String, usize)> {
    told_apart(&Connection::open(path).unwrap()).unwrap()
}

fn of(counted: &[Counted], table: &str) -> u64 {
    counted.iter().find(|counted| counted.table == table).map(|counted| counted.moves).unwrap()
}

#[tokio::test]
async fn a_project_moved_out_of_a_journal_from_before_takes_every_row_of_its_own_and_no_other() {
    let dir = scratch("moved");
    let (factory, file) = (dir.0.join("journal.db"), dir.0.join("shop.db"));
    stopped(&factory).await;
    // The stage's run went on under this program, which says of each
    // step which of its run's it was to begin: two in one millisecond,
    // not called in the order they began.
    let journal = opened(&factory).await;
    let store = Store::open(&journal).await.unwrap();
    for name in ["zulu", "alpha"] {
        let began = StepInfo { step: name.into(), ..step(UNDER_WAY, 1, StepStatus::Running) };
        store.put_step_saying(began, || (Vec::new(), Vec::new()), Vec::new()).await.unwrap();
    }
    closed(journal).await;
    let before = held(&factory);
    // What is run for it, and what was said of it.
    let mut tables = runs::moved::tables();
    tables.push(runs::moved::said());

    made(&file, "project:shop").await;
    let carried = journal::moved::carry(&factory, &file, "shop", &tables, &[]).unwrap();
    // Three instances of the project, each with its first version; the
    // run of the fleet's steps, the stage's that is under way and every
    // run long over, each with a step, and the two the stage's began
    // since; how far the stage's list had come; the one place a live
    // handler had.
    let went = [
        ("instances", 3),
        ("instance_versions", 3),
        ("runs", 2 + ENDED as u64),
        ("steps", 4 + ENDED as u64),
        ("step_elements", 1),
        ("places", 1),
        ("alive", 2),
        ("under_way", 1),
    ];
    for (table, rows) in went {
        assert_eq!(of(&carried, table), rows, "{table}");
    }
    // Written, and nothing decided: the journal holds what it held.
    assert_eq!(held(&factory), before);

    let note = serde_json::json!({ "moved": "shop" });
    let at = journal::moved::decide(
        &factory,
        &row("shop.db"),
        &tables,
        &[],
        &carried,
        KEPT,
        &note,
        std::time::Instant::now(),
    )
    .unwrap();

    // Every row of both files is whose its table's line says, and each
    // file holds its own: the project's, and what is left.
    let (there, left) = (held(&file), held(&factory));
    for ((table, rows), (now, was)) in went.iter().zip(left.iter().zip(&before)) {
        assert_eq!(there.iter().find(|(name, _)| name == table).unwrap().1 as u64, *rows, "{table} there");
        assert_eq!(now.1 as u64, was.1 as u64 - rows, "{table} left");
    }
    let connection = Connection::open(&factory).unwrap();
    for counted in journal::moved::count(&connection, "shop", &tables).unwrap() {
        assert_eq!(counted.moves, 0, "`{}` still holds rows of the project", counted.table);
    }
    for counted in journal::moved::count(&Connection::open(&file).unwrap(), "shop", &tables).unwrap() {
        assert_eq!(counted.moves, counted.all, "`{}` holds rows that are not the project's", counted.table);
    }

    // The journal says where the project is kept, how much of it, that
    // it was moved and where the giver of numbers stood; and what was
    // done is kept for the daemon that next starts to say.
    let listed = journal::listed(&connection).unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(
        (listed[0].project.as_str(), listed[0].file.as_str(), listed[0].keeps),
        ("shop", "shop.db", 1)
    );
    assert_eq!(listed[0].moved, Some(("factory".to_string(), at)));
    let said: String = connection
        .query_row("SELECT value FROM kv WHERE namespace = ?1 AND key = ?2", [KEPT.0, KEPT.1], |row| {
            row.get(0)
        })
        .unwrap();
    let said: serde_json::Value = serde_json::from_str(&said).unwrap();
    assert_eq!((&said[0]["moved"], &said[0]["at"]), (&serde_json::json!("shop"), &serde_json::json!(at)));
    assert!(said[0]["took_ms"].is_u64() && said.as_array().unwrap().len() == 1, "{said}");
    drop(connection);

    // Opened as the project's journal, by what is asked of any: what is
    // alive, what is under way and each handler's place are found by
    // project, the place where it was in the journal it was a place in;
    // and a run that said nothing says whose it is where it went.
    let journal = journal::moved::made(&file, "project:shop", Arc::new(ManualClock::new())).await.unwrap();
    let store = Store::open(&journal).await.unwrap();
    let alive: Vec<String> =
        store.alive_of(Some("shop")).await.unwrap().into_iter().map(|instance| instance.id).collect();
    assert_eq!(alive, ["shop-fleet", "shop-fleet-stage"]);
    let under_way: Vec<String> =
        store.under_way_of(Some("shop")).await.unwrap().into_iter().map(|run| run.id).collect();
    assert_eq!(under_way, ["shop-fleet-stage.heard"]);
    let places = store.places_of(Some("shop")).await.unwrap();
    assert_eq!(places.len(), 1);
    assert_eq!((places[0].journal.as_str(), places[0].seq), ("factory", 41));
    let unsaid = journal.db().call(|connection| {
        connection.query_row("SELECT count(*) FROM runs WHERE project IS NOT 'shop'", [], |row| {
            row.get::<_, i64>(0)
        })
    });
    assert_eq!(unsaid.await.unwrap(), 0);
    // A run's steps are read there in the order they began: the one a
    // program from before wrote, which says nothing of that, and then
    // those that say, whatever they are called.
    let stage = store.run(UNDER_WAY).await.unwrap().unwrap();
    let steps: Vec<&str> = stage.steps.iter().map(|step| step.step.as_str()).collect();
    assert_eq!(steps, ["hello", "zulu", "alpha"]);
    closed(journal).await;

    // And the journal it came out of goes on with what is left: what is
    // nobody's and another project's, found as before.
    let journal = opened(&factory).await;
    let store = Store::open(&journal).await.unwrap();
    assert!(store.alive_of(Some("shop")).await.unwrap().is_empty());
    assert_eq!(store.alive_of(None).await.unwrap().len(), 1);
    assert_eq!(store.under_way_of(None).await.unwrap().len(), 1);
    closed(journal).await;
}

#[tokio::test]
async fn a_move_that_was_not_decided_leaves_the_journal_as_it_was_and_a_file_that_is_cleared() {
    let dir = scratch("moved-cut");
    let (factory, folder) = (dir.0.join("journal.db"), dir.0.join("journals"));
    stopped(&factory).await;
    let before = held(&factory);
    let tables = runs::moved::tables();

    // Nothing is taken for nobody's where there is nothing.
    assert!(journal::moved::unnamed(&factory, &folder).is_empty());
    assert_eq!(journal::moved::clear(&factory, &folder), None);

    let file = folder.join(journal::moved::file_for(&folder, "shop"));
    assert_eq!(file, folder.join("shop.db"));
    made(&file, "project:shop").await;
    journal::moved::carry(&factory, &file, "shop", &tables, &[]).unwrap();

    // Cut off here: the journal is as it was, and the file is one that
    // no row names, which whoever next holds the folder takes away.
    assert_eq!(held(&factory), before);
    assert!(journal::listed(&Connection::open(&factory).unwrap()).unwrap().is_empty());
    assert_eq!(journal::moved::unnamed(&factory, &folder), std::slice::from_ref(&file));
    let cleared = journal::moved::clear(&factory, &folder).expect("a file that no row names");
    assert_eq!(cleared.0, 1);
    assert!(cleared.1 > 0);
    assert!(!file.exists());
    assert_eq!(journal::moved::clear(&factory, &folder), None);

    // Asked again it is done, and the file a row names is nobody's to
    // clear: not it, and not its log beside it.
    made(&file, "project:shop").await;
    let carried = journal::moved::carry(&factory, &file, "shop", &tables, &[]).unwrap();
    journal::moved::decide(
        &factory,
        &row("shop.db"),
        &tables,
        &[],
        &carried,
        KEPT,
        &serde_json::json!({}),
        std::time::Instant::now(),
    )
    .unwrap();
    std::fs::write(folder.join("left.db"), b"what a move left").unwrap();
    std::fs::write(folder.join("notes.txt"), b"somebody's").unwrap();
    assert_eq!(journal::moved::clear(&factory, &folder), Some((1, 16)));
    assert!(file.exists());
    assert!(folder.join("notes.txt").exists());
    // A project that has a file is given another to be moved into.
    assert_eq!(journal::moved::file_for(&folder, "shop"), "shop.2.db");
}

#[tokio::test]
async fn a_move_is_not_decided_over_a_journal_that_was_written_to_since_its_rows_were_carried() {
    let dir = scratch("moved-since");
    let (factory, file) = (dir.0.join("journal.db"), dir.0.join("shop.db"));
    stopped(&factory).await;
    let tables = runs::moved::tables();
    made(&file, "project:shop").await;
    let carried = journal::moved::carry(&factory, &file, "shop", &tables, &[]).unwrap();

    // Something that should not have had it open wrote a run of the
    // project's meanwhile.
    Connection::open(&factory)
        .unwrap()
        .execute(
            "INSERT INTO runs (id, instance, flow, revision, status, event, started, project)
             VALUES ('shop-old.since', 'shop-old', 'on.heard', 'r1', 'ok', 'null', 9, 'shop')",
            [],
        )
        .unwrap();
    let before = held(&factory);
    let refused = journal::moved::decide(
        &factory,
        &row("shop.db"),
        &tables,
        &[],
        &carried,
        KEPT,
        &serde_json::json!({}),
        std::time::Instant::now(),
    )
    .expect_err("a row that was not carried");
    assert!(refused.message.contains("was written to meanwhile"), "{}", refused.message);
    assert!(refused.message.contains("Nothing was moved"), "{}", refused.message);
    // Nothing of it was written: no row names the file, and every row
    // is where it was.
    assert_eq!(held(&factory), before);
    assert!(journal::listed(&Connection::open(&factory).unwrap()).unwrap().is_empty());
}

/// Everything a file keeps in its small durable state under `name`:
/// each key with what is kept.
fn kept(path: &Path, name: &str) -> Vec<(String, String)> {
    let connection = Connection::open(path).unwrap();
    let mut statement =
        connection.prepare("SELECT key, value FROM kv WHERE namespace = ?1 ORDER BY key").unwrap();
    let rows = statement.query_map([name], |row| Ok((row.get(0)?, row.get(1)?))).unwrap();
    rows.collect::<rusqlite::Result<_>>().unwrap()
}

fn keys(path: &Path, name: &str) -> Vec<String> {
    kept(path, name).into_iter().map(|(key, _)| key).collect()
}

#[tokio::test]
async fn what_is_kept_beside_the_rows_goes_with_its_instance_or_its_project_and_a_text_where_a_tree_is_held_to_it()
 {
    use runs::beside::{
        ASKED, AT, ENVIRONMENT, FITS, FORKED, OWED, PINNED, POLL, PROJECT_ENVIRONMENTS, TEXTS, VOUCHED, Whose,
    };
    let dir = scratch("moved-beside");
    let (factory, file) = (dir.0.join("journal.db"), dir.0.join("shop.db"));
    stopped(&factory).await;
    // Another crate's list, read as the runner's own is.
    const READS: &str = "compared.reads";
    let theirs: &[(&str, Whose, &str)] = &[(READS, Whose::Instance, "what a thing reads is its starter's")];
    let mut lines = runs::moved::tables();
    lines.extend(runs::moved::beside(&[runs::beside::KEPT, theirs]));
    // In the one list with what was said, as a project is moved.
    lines.push(runs::moved::said());
    let placed = runs::moved::placed();

    // As a program from before kept it: all of it in the one file, for
    // the project's instances, for another project's and for nobody's.
    let keep = |path: &Path, all: &[(&str, &str, &str)]| {
        let connection = Connection::open(path).unwrap();
        for (name, key, value) in all {
            connection
                .execute("INSERT INTO kv (namespace, key, value) VALUES (?1, ?2, ?3)", [name, key, value])
                .unwrap();
        }
    };
    let text = r#"{"source":"blueprint: kid\n"}"#;
    keep(
        &factory,
        &[
            // Held to one text, and able to be put back on two more.
            (
                PINNED,
                "shop-fleet",
                r#"{"at":1,"texts":{"kid":"r-kid"},"before":{"at":0,"texts":{"kid":"r-was"},"own":"r-own"}}"#,
            ),
            (PINNED, "none-watch", r#"{"at":1,"texts":{"kid":"r-kid","other":"r-none"}}"#),
            (PINNED, "yard-done", r#"{"at":1,"texts":{"kid":"r-yard"}}"#),
            (TEXTS, "r-kid", text),
            (TEXTS, "r-was", text),
            (TEXTS, "r-own", text),
            (TEXTS, "r-none", text),
            (TEXTS, "r-yard", text),
            (TEXTS, "kid", text),
            (VOUCHED, "shop-fleet", "[]"),
            (VOUCHED, "none-watch", "[]"),
            (POLL, "shop-fleet-stage/read", r#"{"content":"two"}"#),
            (POLL, "none-watch/read", r#"{"content":"two"}"#),
            (AT, "shop-fleet-stage/struck", r#"{"at":"02:00 UTC","last":1}"#),
            (AT, "none-watch/struck", r#"{"at":"02:00 UTC","last":1}"#),
            // The run an asking of a step's began, by the step's instance.
            (ASKED, "shop-fleet\none", r#""run-one""#),
            (ASKED, "none-watch\none", r#""run-two""#),
            (FITS, "shop\nkid", r#"{"revision":"r-kid"}"#),
            (FITS, "shop\nother", r#"{"revision":"r-none"}"#),
            (FITS, "yard\nkid", r#"{"revision":"r-yard"}"#),
            (FITS, "\nkid", r#"{"revision":"r-kid"}"#),
            // What a fork of each project's was made from.
            (FORKED, "shop\nstd/board", r#"{"blueprint":"std/board"}"#),
            (FORKED, "yard\nstd/board", r#"{"blueprint":"std/board"}"#),
            // And each project's environments.
            (PROJECT_ENVIRONMENTS, "shop\nstaging", r#"{"name":"staging"}"#),
            (PROJECT_ENVIRONMENTS, "yard\nprod", r#"{"name":"prod"}"#),
            (READS, "shop-fleet\nkid-1", "{}"),
            (READS, "none-watch\nkid-1", "{}"),
            (ENVIRONMENT, "variables", "{}"),
        ],
    );
    let noted = keys(&factory, OWED.0);
    assert_eq!(noted, [OWED.1], "a journal from before with something alive notes it");
    // The file it goes to is one its own conductor kept something in:
    // a text by the same revision, and its own last text that checked;
    // and something is left there under the key of an instance whose
    // rows it never had.
    made(&file, "project:shop").await;
    keep(
        &file,
        &[
            (TEXTS, "r-kid", text),
            (FITS, "shop\nkid", r#"{"revision":"its-own"}"#),
            (POLL, "shop-fleet-stage/read", r#"{"content":"one"}"#),
        ],
    );

    let carried = journal::moved::carry(&factory, &file, "shop", &lines, &placed).unwrap();
    let said = |counted: &[Counted]| -> Vec<(String, u64)> {
        let beside = counted.iter().filter(|counted| counted.beside);
        beside.map(|counted| (counted.table.clone(), counted.moves)).collect()
    };
    let goes = [
        (PINNED, 1),
        (TEXTS, 3),
        (VOUCHED, 1),
        (POLL, 1),
        (AT, 1),
        (ASKED, 1),
        (FITS, 2),
        (FORKED, 1),
        (PROJECT_ENVIRONMENTS, 1),
        (READS, 1),
    ];
    assert_eq!(said(&carried), goes.map(|(name, kept)| (name.to_string(), kept)));

    // Not decided over a file something was kept in meanwhile.
    keep(&factory, &[(POLL, "shop-fleet/other", "{}")]);
    let decide = || {
        let began = std::time::Instant::now();
        journal::moved::decide(
            &factory,
            &row("shop.db"),
            &lines,
            &placed,
            &carried,
            KEPT,
            &serde_json::json!({}),
            began,
        )
    };
    let refused = decide().expect_err("something kept that was not carried");
    assert!(refused.message.contains("`poll` holds 2 rows of `shop`"), "{}", refused.message);
    assert_eq!(keys(&factory, PINNED), ["none-watch", "shop-fleet", "yard-done"]);
    Connection::open(&factory).unwrap().execute("DELETE FROM kv WHERE key = 'shop-fleet/other'", []).unwrap();
    decide().unwrap();

    // What was an instance's of the project, and the project's, is in
    // its file and not in the factory's; another project's, nobody's
    // and the factory's own are where they were.
    assert_eq!(keys(&file, PINNED), ["shop-fleet"]);
    assert_eq!(keys(&factory, PINNED), ["none-watch", "yard-done"]);
    assert_eq!(
        (keys(&file, VOUCHED), keys(&factory, VOUCHED)),
        (vec!["shop-fleet".into()], vec!["none-watch".to_string()])
    );
    // What was kept for an instance is its conductor's, which had the
    // instance until now: it stands over what the file had.
    assert_eq!(
        kept(&file, POLL),
        [("shop-fleet-stage/read".to_string(), r#"{"content":"two"}"#.to_string())]
    );
    assert_eq!(keys(&factory, POLL), ["none-watch/read"]);
    assert_eq!(keys(&file, READS), ["shop-fleet\nkid-1"]);
    assert_eq!(keys(&factory, READS), ["none-watch\nkid-1"]);
    assert_eq!(keys(&factory, FITS), ["\nkid", "yard\nkid"]);
    // What a fork was made from goes with the project whose file the fork is.
    assert_eq!(
        (keys(&file, FORKED), keys(&factory, FORKED)),
        (vec!["shop\nstd/board".into()], vec!["yard\nstd/board".to_string()])
    );
    // And so do its environments.
    assert_eq!(
        (keys(&file, PROJECT_ENVIRONMENTS), keys(&factory, PROJECT_ENVIRONMENTS)),
        (vec!["shop\nstaging".into()], vec!["yard\nprod".to_string()])
    );
    // What its own conductor had kept under the project's name stands.
    assert_eq!(
        kept(&file, FITS),
        [
            ("shop\nkid".to_string(), r#"{"revision":"its-own"}"#.to_string()),
            ("shop\nother".to_string(), r#"{"revision":"r-none"}"#.to_string())
        ]
    );
    // Each text a tree of the project is held to, or can be put back
    // on, is kept with it, once; and the factory's has every one still,
    // for what is left.
    assert_eq!(keys(&file, TEXTS), ["r-kid", "r-own", "r-was"]);
    assert_eq!(keys(&factory, TEXTS), ["kid", "r-kid", "r-none", "r-own", "r-was", "r-yard"]);
    // A journal's note of itself, and what is the factory's, stay.
    assert_eq!((keys(&factory, OWED.0), keys(&file, OWED.0)), (noted, Vec::new()));
    assert_eq!((keys(&factory, ENVIRONMENT).len(), keys(&file, ENVIRONMENT).len()), (1, 0));
    let connection = Connection::open(&factory).unwrap();
    for counted in journal::moved::count(&connection, "shop", &lines).unwrap() {
        assert_eq!(counted.moves, 0, "`{}` still holds something of the project's", counted.table);
    }
}

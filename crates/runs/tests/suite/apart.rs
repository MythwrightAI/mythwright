//! Whose each row is: every row of every table says its project or is
//! reached from one that does; a tree of instances is of one project, and
//! what says whose a row is never changes; and what is asked by project
//! reads nothing of another project's, however much the others have.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use journal::Journal;
use runs::{InstanceFilter, InstanceStatus, RunInfo, StepStatus, Store};
use serde_json::{Value, json};

use crate::{kept, of_project, run, said, says, step, told_apart};

/// The run of an instance's steps, under way, named after it.
fn main_of(instance: &str) -> RunInfo {
    run(&format!("{instance}.main"), instance)
}

/// Something of `project` that is alive, called `name`, with all that
/// is kept for it: a tree of two, each with a run under way, a step of
/// it under way in the middle of a list, and a handler with a place.
/// And something of the same project that is over, with a run that ended.
async fn made(store: &Store, journal: &Journal, project: Option<&str>, name: &str) {
    let prefix = project.unwrap_or("none");
    let (root, child, over) =
        (format!("{prefix}-{name}"), format!("{prefix}-{name}-child"), format!("{prefix}-{name}-over"));
    for (id, parent) in [(&root, None), (&child, Some(root.as_str())), (&over, None)] {
        store.insert_instance(of_project(id, project, parent), None, Vec::new()).await.unwrap();
        let of = main_of(id);
        store.insert_run(of.clone()).await.unwrap();
        let begun = step(&of.id, 1, StepStatus::Running);
        store.put_step_saying(begun, || (Vec::new(), Vec::new()), Vec::new()).await.unwrap();
        store.begin_element(&of.id, "hello", 0, 0).await.unwrap();
        store.set_place(id, "heard", journal.name(), 7).await.unwrap();
    }
    let ended = crate::ending(&main_of(&over).id, &over, false);
    store.end_run(runs::Ending { becomes: None, ..ended }).await.unwrap();
    store.stop_instance(&over, 9, Vec::new()).await.unwrap();
}

#[tokio::test]
async fn a_row_of_every_table_says_its_project_or_is_reached_from_one_that_does() {
    let (journal, store) = kept().await;
    for project in [Some("shop"), Some("yard"), None] {
        made(&store, &journal, project, "thing").await;
    }
    let found = journal.db().call(|connection| told_apart(connection)).await.unwrap();
    // Three of each project and of nobody: nine instances, each with its
    // first version, a run, a step and a place; and how far a list has
    // come for the six runs still under way.
    let rows = |table: &str| found.iter().find(|(name, _)| name == table).map(|(_, rows)| *rows);
    for table in ["instances", "instance_versions", "runs", "steps", "places"] {
        assert_eq!(rows(table), Some(9), "{table}");
    }
    // What is alive and under way is kept beside them, by project.
    for table in ["step_elements", "alive", "under_way"] {
        assert_eq!(rows(table), Some(6), "{table}");
    }

    // Asked of one row, it is the same answer.
    assert_eq!(store.project_of("shop-thing-child").await.unwrap(), Some(Some("shop".to_string())));
    assert_eq!(store.project_of("none-thing").await.unwrap(), Some(None));
    assert_eq!(store.project_of("nothing").await.unwrap(), None);
    assert_eq!(store.project_of_run("yard-thing-over.main").await.unwrap(), Some(Some("yard".to_string())));
    assert_eq!(store.project_of_run("none-thing.main").await.unwrap(), Some(None));
    assert_eq!(store.project_of_run("nothing").await.unwrap(), None);
}

#[tokio::test]
async fn every_table_the_runner_keeps_says_how_its_rows_are_told_apart() {
    let (journal, _) = kept().await;
    let made = journal.db().call(|connection| {
        let mut statement = connection
            .prepare("SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%'")?;
        statement.query_map([], |row| row.get::<_, String>(0))?.collect::<rusqlite::Result<Vec<_>>>()
    });
    let mut made: Vec<String> = made
        .await
        .unwrap()
        .into_iter()
        .filter(|table| !journal::copy::TABLES.contains(&table.as_str()))
        .collect();
    made.sort();
    let mut told: Vec<&str> = runs::apart::TABLES.iter().map(|(table, _)| *table).collect();
    told.sort_unstable();
    assert_eq!(
        made, told,
        "a table kept for what runs has a line of `runs::apart::TABLES` that says how its rows are told \
         apart by project: a project's rows are to be moved to its own journal, and not worked out"
    );
    // And each goes into a copy, or is emptied there, by being said:
    // with the table places were kept in before, which is no longer made.
    let mut copied = runs::copy::TABLES.to_vec();
    copied.retain(|table| *table != "trigger_cursors");
    copied.sort_unstable();
    assert_eq!(copied, told);
}

#[tokio::test]
async fn an_instance_is_of_its_parents_project_or_is_not_written() {
    let (journal, store) = kept().await;
    store.insert_instance(of_project("shop-root", Some("shop"), None), None, Vec::new()).await.unwrap();
    store.insert_instance(of_project("none-root", None, None), None, Vec::new()).await.unwrap();

    // Asked for as another project's, or as nobody's, under something of
    // a project; and as a project's under something that is nobody's.
    for (project, parent) in [(Some("yard"), "shop-root"), (None, "shop-root"), (Some("shop"), "none-root")] {
        let started = says("instance.started", "astray");
        let refused = store.insert_instance(of_project("astray", project, Some(parent)), None, started).await;
        let refused = refused.expect_err("an instance of one project under one of another");
        assert_eq!(refused.kind, kernel::ErrorKind::Invalid);
        assert!(refused.message.contains("a tree of instances is never of two"), "{}", refused.message);
        assert!(refused.message.contains(parent), "{}", refused.message);
    }
    // Nothing of it was written, and nothing said.
    assert!(store.instance("astray").await.unwrap().is_none());
    assert!(store.versions("astray", 10).await.unwrap().is_empty());
    assert!(said(&journal).await.is_empty());

    // As its parent is, it is written.
    let child = of_project("shop-child", Some("shop"), Some("shop-root"));
    assert!(store.insert_instance(child, None, Vec::new()).await.unwrap());
    let child = of_project("none-child", None, Some("none-root"));
    assert!(store.insert_instance(child, None, Vec::new()).await.unwrap());
    assert_eq!(store.project_of("shop-child").await.unwrap(), Some(Some("shop".to_string())));
}

#[tokio::test]
async fn what_says_whose_a_row_is_is_written_once_and_the_database_refuses_another() {
    let (journal, store) = kept().await;
    made(&store, &journal, Some("shop"), "thing").await;
    made(&store, &journal, Some("yard"), "thing").await;
    let tried = |sql: &'static str| journal.db().call(move |connection| connection.execute(sql, []));
    for (sql, of) in [
        ("UPDATE instances SET project = 'yard' WHERE id = 'shop-thing'", "an instance is of the project"),
        ("UPDATE instances SET project = NULL WHERE id = 'shop-thing'", "an instance is of the project"),
        ("UPDATE runs SET instance = 'yard-thing' WHERE id = 'shop-thing.main'", "a run is of the instance"),
        ("UPDATE runs SET project = 'yard' WHERE id = 'shop-thing.main'", "a run is of the instance"),
        ("UPDATE runs SET project = NULL WHERE id = 'shop-thing.main'", "a run is of the instance"),
        ("UPDATE places SET project = 'yard' WHERE instance = 'shop-thing'", "a place is of the project"),
        ("UPDATE alive SET project = 'yard' WHERE instance = 'shop-thing'", "what is alive is kept from"),
        ("UPDATE under_way SET project = 'yard' WHERE run = 'shop-thing.main'", "what is under way is kept"),
    ] {
        let refused = tried(sql).await.expect_err(sql);
        assert!(refused.message.contains(of), "{sql}: {}", refused.message);
    }
    // Nor is anything written that says whose it is wrongly, or not at
    // all, by whatever writes it: a run and a place of something of one
    // project, an instance under something of another.
    for (sql, of) in [
        (
            "INSERT INTO runs (id, instance, flow, revision, status, event, started, project)
             VALUES ('shop-thing.astray', 'shop-thing', 'main', 'r1', 'running', 'null', 2, 'yard')",
            "a run says whose it is as it is written",
        ),
        (
            "INSERT INTO runs (id, instance, flow, revision, status, event, started)
             VALUES ('shop-thing.unsaid', 'shop-thing', 'main', 'r1', 'running', 'null', 2)",
            "a run says whose it is as it is written",
        ),
        (
            "INSERT INTO places (instance, handler, journal, seq, project)
             VALUES ('shop-thing', 'astray', 'factory', 1, '')",
            "a place says whose it is as it is first kept",
        ),
        (
            "INSERT INTO instances
                 (id, blueprint, revision, source, params, state, status, parent, key, project, principal,
                  created, updated)
             VALUES ('astray', 'thing', 'r1', '', '{}', '{}', 'active', 'shop-thing', 'astray', 'yard', '{}', 1, 1)",
            "a tree of instances is never of two",
        ),
    ] {
        let refused = tried(sql).await.expect_err(sql);
        assert!(refused.message.contains(of), "{sql}: {}", refused.message);
    }
    journal.db().call(|connection| told_apart(connection)).await.unwrap();

    // Everything else that is written of an instance leaves it whose it
    // was: what it keeps, what it holds, another text, its ending.
    store.set_instance_state("shop-thing", kernel::Fields::new(), 5).await.unwrap();
    store.set_instance_holds("shop-thing", vec!["service:web".into()], 5).await.unwrap();
    store.rename_instance("shop-thing-child", "other", 5).await.unwrap();
    let text = runs::Definition {
        revision: "r2".into(),
        source: "name: greet\ndescription: again\n".into(),
        params: kernel::Fields::new(),
        principal: None,
        state: None,
        before: None,
        heard: None,
    };
    store.take_text("shop-thing", text, Vec::new(), 6, Vec::new()).await.unwrap();
    store.fail_instance("shop-thing-child", "it could not".into(), false, 7, Vec::new()).await.unwrap();
    assert_eq!(store.instance("shop-thing-child").await.unwrap().unwrap().status, InstanceStatus::Failed);
    journal.db().call(|connection| told_apart(connection)).await.unwrap();
}

/// How much the database has done, counted by the database: it tells
/// this as it goes, once for each small thing it does to answer.
pub(crate) async fn counting(journal: &Journal) -> Arc<AtomicU64> {
    let done = Arc::new(AtomicU64::new(0));
    let counted = done.clone();
    journal
        .db()
        .call(move |connection| {
            connection.progress_handler(
                1,
                Some(move || {
                    counted.fetch_add(1, Ordering::SeqCst);
                    false
                }),
            )
        })
        .await
        .unwrap();
    done
}

/// What is asked of a store by project, each thing with how much the
/// database did to answer it and what it answered, for the project `shop`
/// and for what is nobody's. And what is asked of everything, last.
async fn asked(store: &Store, done: &AtomicU64) -> Vec<(&'static str, u64, Value)> {
    let mut answers = Vec::new();
    let mut note = |what: &'static str, before: u64, answer: Value| {
        answers.push((what, done.load(Ordering::SeqCst) - before, answer));
    };
    let at = || done.load(Ordering::SeqCst);
    for project in [Some("shop"), None] {
        let before = at();
        let alive = store.alive_of(project).await.unwrap();
        note("alive", before, json!(alive.iter().map(|one| &one.id).collect::<Vec<_>>()));

        let before = at();
        let under_way = store.under_way_of(project).await.unwrap();
        note("under way", before, json!(under_way.iter().map(|one| &one.id).collect::<Vec<_>>()));

        let before = at();
        let places = store.places_of(project).await.unwrap();
        let places: Vec<_> =
            places.iter().map(|at| (&at.instance, &at.handler, &at.journal, at.seq)).collect();
        note("places", before, json!(places));

        let before = at();
        let key = format!("{}-thing", project.unwrap_or("none"));
        let found = store.live_instance_by_key(None, &key, project).await.unwrap();
        note("by its name", before, json!(found.map(|one| one.id)));
    }
    // Asked as the engine asks, of a project's instances that are alive.
    let before = at();
    let filter = InstanceFilter { live_only: true, project: Some("shop".into()), ..Default::default() };
    let alive = store.instances(filter).await.unwrap();
    note("alive, listed", before, json!(alive.iter().map(|one| &one.id).collect::<Vec<_>>()));

    let before = at();
    let every = store.unfinished_runs().await.unwrap();
    note("every run under way", before, json!(every.len()));
    answers
}

#[tokio::test]
async fn asking_for_one_projects_share_reads_no_row_of_anothers() {
    // Two factories with the same of one project and of nobody's, and
    // beside it a little of another project's in one and a great deal in
    // the other, alive and over.
    let mut came = Vec::new();
    for others in [2, 400] {
        let (journal, store) = kept().await;
        made(&store, &journal, Some("shop"), "thing").await;
        made(&store, &journal, None, "thing").await;
        for n in 0..others {
            made(&store, &journal, Some("yard"), &format!("thing{n}")).await;
        }
        let done = counting(&journal).await;
        came.push(asked(&store, &done).await);
    }
    let (few, many) = (&came[0], &came[1]);

    // What is a project's is its own, and what is nobody's is nobody's.
    let answer = |what: &str, nth: usize| {
        few.iter().filter(|(asked, ..)| *asked == what).nth(nth).map(|(_, _, answer)| answer.clone()).unwrap()
    };
    assert_eq!(answer("alive", 0), json!(["shop-thing", "shop-thing-child"]));
    assert_eq!(answer("alive", 1), json!(["none-thing", "none-thing-child"]));
    assert_eq!(answer("under way", 0), json!(["shop-thing.main", "shop-thing-child.main"]));
    assert_eq!(answer("under way", 1), json!(["none-thing.main", "none-thing-child.main"]));
    assert_eq!(answer("by its name", 0), json!("shop-thing"));
    assert_eq!(answer("by its name", 1), json!("none-thing"));
    assert_eq!(answer("alive, listed", 0), json!(["shop-thing-child", "shop-thing"]));
    // A place is kept for what is over too, until its instance's handlers are dropped.
    let places = answer("places", 0);
    assert_eq!(places.as_array().map(Vec::len), Some(3), "{places}");
    assert_eq!(places[0], json!(["shop-thing", "heard", journal::FACTORY, 7]));

    // The database did as much for each, to the last thing it does,
    // whether the other project had a little or a great deal: nothing of
    // the other's was read. What is asked of everything is what grew.
    for ((what, little, answer), (_, much, with_many)) in few.iter().zip(many) {
        if *what == "every run under way" {
            assert!(much > &(little * 20), "{what}: {little} and {much}: is the database's work counted?");
            continue;
        }
        assert_eq!(answer, with_many, "{what}");
        assert!(*little > 0, "{what}: nothing was counted");
        assert_eq!(little, much, "{what}: more was read where another project had more");
    }
}

//! The journals of a factory: its own, and one for each project that has
//! one, each a file with a thread of its own, kept beside the factory's.
//! Which projects have one is written down in the factory's database; a
//! file says what it is called and is held to it; they draw from one
//! giver, shut together, and are measured in the factory's.

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use journal::{Db, EventQuery, Journal, MetricQuery};
use kernel::{Clock, ManualClock, Source};
use serde_json::json;

use crate::scratch;

fn clock() -> Arc<dyn Clock> {
    Arc::new(ManualClock::new())
}

/// The factory's journal in `dir`, keeping its projects' in a folder there.
pub(crate) async fn factory(dir: &Path) -> Journal {
    let opened = Journal::open(Db::open(&dir.join("journal.db")).unwrap(), clock(), "dev").await.unwrap();
    opened.keeps_projects_in(dir.join("journals")).await.unwrap();
    opened
}

async fn say(journal: &Journal) -> u64 {
    journal.emit("said", Source::default(), json!({})).await.unwrap().seq
}

pub(crate) async fn numbers(journal: &Journal) -> Vec<u64> {
    let all = EventQuery { limit: usize::MAX, ..Default::default() };
    journal.read(all).await.unwrap().iter().map(|event| event.seq).collect()
}

/// Closes every journal of a factory and waits for their files to be let go.
pub(crate) async fn closed(factory: Journal) {
    for db in factory.dbs() {
        db.close();
        db.closed().await;
    }
}

#[tokio::test]
async fn a_project_is_given_a_journal_of_its_own_in_a_file_of_its_own_and_has_it_when_started_again() {
    if crate::one_journal() {
        return;
    }
    let dir = scratch("beside");
    let of_factory = factory(&dir).await;
    assert_eq!(of_factory.every().len(), 1, "a factory with no project has its own journal and no other");

    let shop = of_factory.own("shop").await.unwrap();
    assert_eq!((shop.name(), of_factory.name()), ("project:shop", journal::FACTORY));
    assert!(dir.join("journals").join("shop.db").is_file());
    assert!(!shop.db().is(of_factory.db()), "a database, and a thread, of its own");
    // Asked again it is the one there is; and it is the one that keeps
    // what is said of the project, as the factory's keeps what is nobody's.
    assert!(of_factory.own("shop").await.unwrap().db().is(shop.db()));
    assert!(of_factory.of(Some("shop")).db().is(shop.db()));
    assert!(of_factory.of(None).db().is(of_factory.db()));
    // What is kept of a project beside its events (its records) is
    // asked for by the project all the same, and is in its own journal
    // too, which was made keeping it; in the factory's for a project
    // nobody gave one, and for what is nobody's.
    assert!(of_factory.records_of(Some("shop")).db().is(shop.db()));
    assert!(of_factory.records_of(Some("nobody-gave-one")).db().is(of_factory.db()));
    assert!(of_factory.records_of(None).db().is(of_factory.db()));
    assert_eq!(of_factory.every_records().len(), 2);
    // A project nobody gave one has none: what is said of it is kept in
    // the factory's own.
    assert!(of_factory.of_project("from-before").is_none());
    assert!(of_factory.of(Some("from-before")).db().is(of_factory.db()));

    // Each keeps what is said in it, and no number is in both.
    let given = [say(&shop).await, say(&of_factory).await, say(&shop).await];
    assert_eq!(numbers(&shop).await, [given[0], given[2]]);
    assert_eq!(numbers(&of_factory).await, [given[1]]);
    assert!(given[0] != given[1] && given[2] == given[0] + 1);
    let other = of_factory.own("other").await.unwrap();
    let names: Vec<String> = of_factory.every().iter().map(|journal| journal.name().to_string()).collect();
    assert_eq!(names, ["factory", "project:other", "project:shop"]);
    assert_eq!(of_factory.dbs().len(), 3);
    assert!(of_factory.dbs()[2].is(of_factory.db()), "the factory's own is closed last");
    say(&other).await;
    closed(of_factory).await;

    // Started again, each is opened under its name with what it kept,
    // and goes on above all that any of them was given.
    let of_factory = factory(&dir).await;
    let shop = of_factory.of_project("shop").expect("the project's journal is opened with the factory's");
    assert_eq!(numbers(&shop).await, [given[0], given[2]]);
    assert_eq!(of_factory.every().len(), 3);
    let highest = given.iter().copied().max().unwrap();
    assert!(say(&shop).await > highest && say(&of_factory).await > highest);
}

#[tokio::test]
async fn a_journal_that_was_told_no_folder_keeps_what_is_said_of_every_project_itself() {
    let alone = Journal::memory(clock()).await.unwrap();
    assert!(alone.own("shop").await.unwrap().db().is(alone.db()));
    assert!(alone.of(Some("shop")).db().is(alone.db()));
    assert_eq!(alone.every().len(), 1);

    // One in memory that is told to keeps its projects' in memory too.
    // (A program told to make none makes none there either, and what is
    // above is all of this that is played by it.)
    if journal::apart() == Some(false) {
        return;
    }
    let of_factory = Journal::memory(clock()).await.unwrap();
    of_factory.keeps_projects_in_memory().await.unwrap();
    let shop = of_factory.own("shop").await.unwrap();
    assert!(!shop.db().is(of_factory.db()));
    assert_eq!([say(&shop).await, say(&of_factory).await].len(), 2);
    // And only a factory's own keeps others beside it.
    let refused = shop.keeps_projects_in_memory().await.unwrap_err();
    assert!(refused.message.contains("only a factory's own"), "{}", refused.message);
    assert!(of_factory.own("").await.is_err());
}

#[tokio::test]
async fn a_program_told_its_projects_are_kept_in_one_journal_makes_none_and_gives_the_factorys_own() {
    if journal::apart() != Some(false) {
        println!("not played: it is of a program told to keep its projects in one journal");
        return;
    }
    let dir = scratch("beside-one");
    let of_factory = factory(&dir).await;
    // Whoever adds a project asks for its journal, and is given the
    // factory's own, whatever one that is made would have been made to
    // keep: as a factory from before, where nobody asked.
    let shop = of_factory.own("shop").await.unwrap();
    let yard = of_factory.own_keeping("yard", journal::keeps::RUNS).await.unwrap();
    assert!(shop.db().is(of_factory.db()) && yard.db().is(of_factory.db()));
    assert!(of_factory.of_project("shop").is_none() && of_factory.rows_of("shop").is_none());
    assert_eq!((of_factory.every().len(), of_factory.every_records().len()), (1, 1));
    // Nothing is written down of it and no file is made: it is a
    // project nobody gave a journal, as every one of a factory from
    // before is.
    let listed = of_factory.db().call(|connection| journal::listed(connection)).await.unwrap();
    assert!(listed.is_empty());
    let files = std::fs::read_dir(dir.join("journals")).map_or(0, |files| files.count());
    assert_eq!(files, 0);
    // And what is said of the project is kept there, with what is nobody's.
    let said = [
        of_factory.emit("said", Source::default(), json!({ "project": "shop" })).await.unwrap().seq,
        say(&of_factory).await,
    ];
    assert_eq!(numbers(&of_factory).await, said);
    closed(of_factory).await;
}

#[tokio::test]
async fn projects_whose_names_cannot_be_files_as_they_are_each_have_a_file_of_their_own() {
    if crate::one_journal() {
        return;
    }
    let dir = scratch("named");
    let of_factory = factory(&dir).await;
    let ids = ["shop", "Shop", "a/b", "a=2fb", "../up", "sh op", "="];
    for id in ids {
        let journal = of_factory.own(id).await.unwrap();
        assert_eq!(journal.name(), journal::of_project(id));
    }
    let mut files: Vec<String> = std::fs::read_dir(dir.join("journals"))
        .unwrap()
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| name.ends_with(".db"))
        .collect();
    files.sort();
    assert_eq!(files.len(), ids.len(), "{files:?}");
    assert!(files.iter().all(|name| !name.contains('/') && !name.starts_with('.')), "{files:?}");
    assert_eq!(of_factory.every().len(), ids.len() + 1);
}

#[tokio::test]
async fn a_journals_file_says_what_it_is_called_and_one_put_in_anothers_place_is_refused() {
    if crate::one_journal() {
        return;
    }
    let dir = scratch("swapped");
    let of_factory = factory(&dir).await;
    for project in ["shop", "other"] {
        say(&of_factory.own(project).await.unwrap()).await;
    }
    closed(of_factory).await;
    let journals = dir.join("journals");
    let (shop, other, aside) = (journals.join("shop.db"), journals.join("other.db"), journals.join("aside"));
    std::fs::rename(&shop, &aside).unwrap();
    std::fs::rename(&other, &shop).unwrap();
    std::fs::rename(&aside, &other).unwrap();

    let opened = Journal::open(Db::open(&dir.join("journal.db")).unwrap(), clock(), "dev").await.unwrap();
    let refused = opened.keeps_projects_in(&journals).await.unwrap_err();
    assert!(refused.message.contains("this is the journal `project:"), "{}", refused.message);
    assert!(refused.message.contains("put in another's place"), "{}", refused.message);
    closed(opened).await;

    // Nor is a project's opened as a factory's own.
    let as_factory = Journal::open(Db::open(&shop).unwrap(), clock(), "dev").await;
    let refused = as_factory.err().expect("refused");
    assert!(refused.message.contains("it was opened as `factory`"), "{}", refused.message);
}

#[tokio::test]
async fn a_factory_is_not_started_without_the_journal_of_a_project_that_had_one() {
    if crate::one_journal() {
        return;
    }
    let dir = scratch("missing");
    let of_factory = factory(&dir).await;
    say(&of_factory.own("shop").await.unwrap()).await;
    closed(of_factory).await;
    for file in ["shop.db", "shop.db-wal", "shop.db-shm"] {
        let _ = std::fs::remove_file(dir.join("journals").join(file));
    }
    let opened = Journal::open(Db::open(&dir.join("journal.db")).unwrap(), clock(), "dev").await.unwrap();
    let refused = opened.keeps_projects_in(dir.join("journals")).await.unwrap_err();
    assert!(refused.message.contains("the journal of the project `shop` is not where it was kept"));
    assert!(!dir.join("journals").join("shop.db").exists(), "and none is made in its place");
}

#[tokio::test]
async fn a_journal_made_and_never_listed_is_the_one_opened_when_the_project_is_given_one_again() {
    let dir = scratch("unlisted");
    let of_factory = factory(&dir).await;
    let shop = of_factory.own("shop").await.unwrap();
    let said = say(&shop).await;
    // As a daemon stopped between making the file and writing its row
    // leaves it: the file is there, and nothing names it.
    of_factory.db().call(|connection| connection.execute("DELETE FROM journals", [])).await.unwrap();
    closed(of_factory).await;

    let of_factory = factory(&dir).await;
    assert!(of_factory.of_project("shop").is_none());
    let shop = of_factory.own("shop").await.unwrap();
    assert_eq!(numbers(&shop).await, [said]);
    assert!(say(&shop).await > said);
}

#[tokio::test]
async fn the_journals_of_a_factory_shut_together_at_a_place_one_of_them_was_told() {
    if crate::one_journal() {
        return;
    }
    let patience = Duration::from_secs(40);
    for place_in_the_projects in [false, true] {
        let dir = scratch("shut-together");
        let of_factory = factory(&dir).await;
        let shop = of_factory.own("shop").await.unwrap();
        of_factory.shut_after(|event| event.topic == "the.place");
        let (at, other) = match place_in_the_projects {
            true => (&shop, &of_factory),
            false => (&of_factory, &shop),
        };
        say(other).await;
        at.emit("the.place", Source::default(), json!({})).await.unwrap();
        tokio::time::timeout(patience, other.shut()).await.expect("the other journal shut with it");
        assert!(at.is_shut() && other.is_shut());
        // Nothing is said in either after it, and nothing written down:
        // each one's database does nothing more.
        for journal in [at, other] {
            let late = tokio::spawn({
                let journal = journal.clone();
                async move { say(&journal).await }
            });
            let wrote = tokio::spawn({
                let journal = journal.clone();
                async move { journal.kv_set("late", "written", &json!(true)).await }
            });
            for _ in 0..50 {
                tokio::task::yield_now().await;
            }
            assert!(!late.is_finished() && !wrote.is_finished());
            late.abort();
            wrote.abort();
        }
        // But that the daemon stopped, which whoever stops it says.
        let stopped = of_factory.emit_last("system.stopped", Source::default(), json!({}));
        tokio::time::timeout(patience, stopped).await.expect("said though shut").unwrap();
        closed(of_factory).await;

        let of_factory = factory(&dir).await;
        let shop = of_factory.of_project("shop").unwrap();
        let (at, other) = match place_in_the_projects {
            true => (&shop, &of_factory),
            false => (&of_factory, &shop),
        };
        let topics = |journal: &Journal| {
            let journal = journal.clone();
            async move {
                let all = journal.read(EventQuery::default()).await.unwrap();
                all.iter().map(|event| event.topic.clone()).collect::<Vec<_>>()
            }
        };
        assert!(topics(at).await.starts_with(&["the.place".to_string()]));
        assert_eq!(topics(other).await.iter().filter(|topic| *topic == "said").count(), 1);
        assert_eq!(of_factory.kv_get("late", "written").await.unwrap(), None);
        assert_eq!(shop.kv_get("late", "written").await.unwrap(), None);
    }
}

#[tokio::test]
async fn what_is_measured_through_a_projects_journal_is_read_in_the_factorys_by_its_name() {
    if crate::one_journal() {
        return;
    }
    let dir = scratch("measured");
    let of_factory = factory(&dir).await;
    let shop = of_factory.own("shop").await.unwrap();
    say(&shop).await;
    shop.measure("build", &[("project", "shop")], 12.0, true);
    of_factory.flush_metrics().await.unwrap();
    shop.flush_metrics().await.unwrap();

    let since_ms = of_factory.now_ms() - 60_000;
    let asked = |name: &str, by: &str| MetricQuery {
        name: name.into(),
        since_ms,
        group_by: vec![by.into()],
        ..Default::default()
    };
    let built = of_factory.query_metrics(asked("build", "project")).await.unwrap();
    assert_eq!(built.len(), 1, "what a project's journal was asked to measure is the factory's to read");
    assert_eq!(built[0].total.count, 1);
    // The work of each database, by which journal's it is.
    let waited = of_factory.query_metrics(asked("db.wait", "db")).await.unwrap();
    let of: Vec<&str> = waited.iter().map(|series| series.labels["db"].as_str()).collect();
    assert_eq!(of, ["journal", "project:shop"]);
    // And how long the project's stood still for its first run of numbers.
    let drew = of_factory.query_metrics(asked("db.draw", "db")).await.unwrap();
    assert_eq!(drew[0].total.count, 1);
    // Nothing is kept in the project's own file of what was measured.
    let kept: i64 =
        shop.db().call(|c| c.query_row("SELECT count(*) FROM metrics", [], |row| row.get(0))).await.unwrap();
    assert_eq!(kept, 0);
    assert!(shop.query_metrics(asked("build", "project")).await.unwrap().is_empty());
}

#[tokio::test]
async fn what_events_are_held_to_is_held_to_in_every_journal_of_a_factory() {
    let of_factory = Journal::memory(clock()).await.unwrap();
    of_factory.keeps_projects_in_memory().await.unwrap();
    let early = of_factory.own("early").await.unwrap();
    of_factory.hold(Arc::new(|topic, _| (topic == "wrong").then(|| "that is wrong".to_string())));
    let late = of_factory.own("late").await.unwrap();
    for journal in [&early, &late] {
        journal.emit("wrong", Source::default(), json!({})).await.unwrap();
    }
    assert_eq!(of_factory.faults(), ["that is wrong"]);
    assert_eq!(late.faults(), of_factory.faults());
}

#[tokio::test]
async fn a_program_from_before_there_were_journals_for_projects_refuses_a_factory_this_one_opened() {
    let dir = scratch("refused-before");
    let path = dir.join("journal.db");
    let of_factory = Journal::open(Db::open(&path).unwrap(), clock(), "dev").await.unwrap();
    closed(of_factory).await;
    // What the journal's tables were to the program before this: three
    // steps, of which nothing is done to a database that is past them.
    static BEFORE: &[&str] = &["SELECT 1", "SELECT 1", "SELECT 1"];
    let db = Db::open(&path).unwrap();
    let refused = db.migrate("journal", BEFORE).await.unwrap_err();
    assert!(refused.message.contains("newer than this build understands"), "{}", refused.message);
}

#[tokio::test]
async fn a_journal_this_program_makes_keeps_what_is_run_for_its_project_and_what_is_said_of_it_and_more() {
    if crate::one_journal() {
        return;
    }
    let dir = scratch("beside-rows");
    let of_factory = factory(&dir).await;
    let shop = of_factory.own("shop").await.unwrap();
    // Its row says so, and it is what is asked for what is run for it.
    let listed = of_factory.db().call(|connection| journal::listed(connection)).await.unwrap();
    assert_eq!(
        listed.iter().map(|one| (one.project.as_str(), one.keeps)).collect::<Vec<_>>(),
        [("shop", journal::keeps::MADE_AT)]
    );
    assert!(listed.iter().all(|one| one.moved.is_none()), "made apart, and moved from nowhere");
    assert!(of_factory.rows_of("shop").is_some_and(|keeps| keeps.db().is(shop.db())));
    assert!(of_factory.rows_of("yard").is_none(), "a project with no journal of its own");
    let keepers: Vec<String> = of_factory.keepers().into_iter().map(|(project, _)| project).collect();
    assert_eq!(keepers, ["shop"]);

    // What is said of the project, said to the factory's own, is kept
    // in the project's; what names no project, or one kept in the
    // factory's, is the factory's own.
    let said = |project: Option<&'static str>| {
        let of_factory = of_factory.clone();
        async move {
            let data = project.map_or(json!({}), |project| json!({ "project": project }));
            of_factory.emit("said", Source::default(), data).await.unwrap().seq
        }
    };
    let (of_shop, of_yard, of_nobody) =
        (said(Some("shop")).await, said(Some("yard")).await, said(None).await);
    assert_eq!(numbers(&shop).await, [of_shop]);
    assert_eq!(numbers(&of_factory).await, [of_yard, of_nobody]);

    // And when the factory is opened again.
    closed(of_factory).await;
    let of_factory = factory(&dir).await;
    assert!(of_factory.rows_of("shop").is_some());
    let again = of_factory.emit("said", Source::default(), json!({ "project": "shop" })).await.unwrap().seq;
    assert_eq!(numbers(&of_factory.of_project("shop").unwrap()).await, [of_shop, again]);
    closed(of_factory).await;
}

#[tokio::test]
async fn a_journal_listed_by_a_program_that_kept_nothing_in_it_keeps_nothing_that_is_run_or_said() {
    if crate::one_journal() {
        return;
    }
    let dir = scratch("beside-listed");
    let of_factory = factory(&dir).await;
    of_factory.own("shop").await.unwrap();
    closed(of_factory).await;
    // As a program from before listed it: its row says nothing of what
    // is run, which is the column's own default.
    {
        let connection = rusqlite::Connection::open(dir.join("journal.db")).unwrap();
        assert_eq!(connection.execute("UPDATE journals SET keeps = 0", []).unwrap(), 1);
    }
    let of_factory = factory(&dir).await;
    let shop = of_factory.of_project("shop").expect("it is still the project's journal");
    assert!(of_factory.rows_of("shop").is_none() && of_factory.keepers().is_empty());
    // What is said of the project stays where what is run for it is.
    let said = of_factory.emit("said", Source::default(), json!({ "project": "shop" })).await.unwrap().seq;
    assert_eq!((numbers(&of_factory).await, numbers(&shop).await), (vec![said], vec![]));
    // Asked for again, it is the journal there is, and is not marked.
    of_factory.own("shop").await.unwrap();
    assert!(of_factory.rows_of("shop").is_none());
    closed(of_factory).await;
}

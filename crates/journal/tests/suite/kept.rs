//! An event is kept in the journal that keeps the row it is said of.
//!
//! A project's journal is made to keep so much of its project (a level):
//! nothing, what is run for it and what is said of it, or its records as
//! well. A row and what is said of it are one write where one journal
//! keeps both, and where two do the row is written and then the event
//! said, as everything once was. What is the factory's for good is kept
//! there whatever project it names.

use journal::{Journal, keeps};
use kernel::Source;
use serde_json::json;

use crate::beside::{closed, factory, numbers};
use crate::scratch;

/// Something a crate above the journal keeps, in each database that is
/// to keep it.
async fn table(journal: &Journal) {
    let made = "CREATE TABLE IF NOT EXISTS things (id TEXT PRIMARY KEY)";
    journal.db().call(move |connection| connection.execute_batch(made)).await.unwrap();
}

async fn things(journal: &Journal) -> Vec<String> {
    let read = journal.db().call(|connection| {
        let mut statement = connection.prepare("SELECT id FROM things ORDER BY id")?;
        let rows = statement.query_map([], |row| row.get(0))?;
        rows.collect::<rusqlite::Result<Vec<String>>>()
    });
    read.await.unwrap()
}

/// Writes a thing of `project` where the project's records are kept,
/// and says so, as whoever keeps records does. Whether it was done.
async fn made(of_factory: &Journal, project: Option<&'static str>, id: &'static str) -> bool {
    let wrote = of_factory.records_of(project).write(move |writing| {
        writing.transaction().execute("INSERT INTO things (id) VALUES (?1)", [id])?;
        let data =
            project.map_or(json!({ "thing": id }), |project| json!({ "thing": id, "project": project }));
        writing.say("thing.made", Source::default(), data)?;
        Ok(())
    });
    wrote.await.is_ok()
}

/// A factory with a project at each level: `old` listed by a program
/// that kept nothing in its journal, `shop` whose journal keeps what is
/// run and said, and `yard` whose journal keeps its records as well.
async fn levels(label: &str) -> (Journal, Journal, Journal, Journal) {
    let of_factory = factory(&scratch(label)).await;
    let old = of_factory.own_keeping("old", keeps::NOTHING).await.unwrap();
    let shop = of_factory.own_keeping("shop", keeps::RUNS).await.unwrap();
    let yard = of_factory.own_keeping("yard", keeps::RECORDS).await.unwrap();
    table(&of_factory).await;
    table(&yard).await;
    (of_factory, old, shop, yard)
}

#[tokio::test]
async fn a_row_and_what_is_said_of_it_are_kept_by_the_journal_that_keeps_the_row_or_written_and_then_said() {
    if crate::one_journal() {
        return;
    }
    let (of_factory, old, shop, yard) = levels("kept-levels").await;
    // Which journal keeps the records of each.
    assert!(of_factory.records_of(None).db().is(of_factory.db()));
    assert!(of_factory.records_of(Some("old")).db().is(of_factory.db()));
    assert!(of_factory.records_of(Some("shop")).db().is(of_factory.db()));
    assert!(of_factory.records_of(Some("yard")).db().is(yard.db()));
    assert_eq!(of_factory.every_records().len(), 2, "the factory's own, and the one that keeps records");

    for (project, id) in [(None, "a"), (Some("old"), "b"), (Some("shop"), "c"), (Some("yard"), "d")] {
        assert!(made(&of_factory, project, id).await, "{project:?}");
    }
    // Nobody's, and a project from before: the factory's own keeps the
    // row and what is said of it. A project whose journal keeps what is
    // said of it and not its records: the row in the factory's, what is
    // said in its own. One whose journal keeps its records: both there.
    assert_eq!(things(&of_factory).await, ["a", "b", "c"]);
    assert_eq!(things(&yard).await, ["d"]);
    assert_eq!(numbers(&of_factory).await.len(), 2);
    assert_eq!(numbers(&old).await.len(), 0);
    assert_eq!(numbers(&shop).await.len(), 1);
    assert_eq!(numbers(&yard).await.len(), 1);
    closed(of_factory).await;
}

#[tokio::test]
async fn where_it_cannot_be_said_nothing_is_written_of_one_write_and_the_row_stays_where_they_are_two() {
    if crate::one_journal() {
        return;
    }
    let (of_factory, _old, shop, yard) = levels("kept-refused").await;
    let refuse = "CREATE TRIGGER unsaid BEFORE INSERT ON events WHEN NEW.topic = 'thing.made'
                  BEGIN SELECT RAISE(FAIL, 'database or disk is full'); END;";
    for journal in [&of_factory, &shop, &yard] {
        journal.db().call(move |connection| connection.execute_batch(refuse)).await.unwrap();
    }
    // One write: neither is kept.
    assert!(!made(&of_factory, None, "a").await);
    assert!(!made(&of_factory, Some("old"), "b").await);
    assert!(!made(&of_factory, Some("yard"), "d").await);
    assert!(things(&of_factory).await.is_empty() && things(&yard).await.is_empty());
    // Two journals: the row is written, and nobody was told. Whoever
    // looks for a row written and never said finds this one.
    assert!(made(&of_factory, Some("shop"), "c").await);
    assert_eq!(things(&of_factory).await, ["c"]);
    for journal in [&of_factory, &shop, &yard] {
        assert!(numbers(journal).await.is_empty());
    }
    closed(of_factory).await;
}

#[tokio::test]
async fn what_is_the_factorys_for_good_is_kept_there_whatever_project_it_names() {
    if crate::one_journal() {
        return;
    }
    let (of_factory, _old, shop, _yard) = levels("kept-here").await;
    let data = json!({ "project": "shop" });
    // By itself.
    let here = of_factory.emit_here("channel.spoken", Source::default(), data.clone()).await.unwrap().seq;
    // And with what it is said of.
    let wrote = of_factory.write(move |writing| {
        writing.transaction().execute("INSERT INTO things (id) VALUES ('said')", [])?;
        writing.say_here("channel.spoken", Source::default(), json!({ "project": "shop" }))
    });
    let (with, kept) = wrote.await.unwrap();
    assert_eq!(kept.len(), 1);
    assert_eq!(numbers(&of_factory).await, [here, with]);
    // Where the same said plainly is the project's.
    let routed = of_factory.emit("spoken", Source::default(), data.clone()).await.unwrap().seq;
    assert_eq!(numbers(&shop).await, [routed]);
    // And so it is said plainly, with a row or only passed on: which
    // topics stay is asked where every event's journal is.
    let plainly = of_factory.emit("channel.spoken", Source::default(), data.clone()).await.unwrap().seq;
    let wrote = of_factory.write(move |writing| writing.say("channel.spoken", Source::default(), data));
    let (written, _) = wrote.await.unwrap();
    assert_eq!(numbers(&of_factory).await, [here, with, plainly, written]);
    assert_eq!(numbers(&shop).await, [routed]);
    let (mut of_shop, mut of_all) = (shop.follow(0, ""), of_factory.follow(written, ""));
    of_factory.emit_ephemeral("channel.typing", Source::default(), json!({ "project": "shop" }));
    of_factory.emit_ephemeral("typing", Source::default(), json!({ "project": "shop" }));
    assert_eq!(of_all.next().await.unwrap().topic, "channel.typing");
    let passed_on: Vec<String> = vec![of_shop.next().await.unwrap(), of_shop.next().await.unwrap()]
        .into_iter()
        .map(|event| event.topic.clone())
        .collect();
    assert_eq!(passed_on, ["spoken", "typing"]);
    closed(of_factory).await;
}

/// Whether `journal`'s database has a table called `table`.
async fn has(journal: &Journal, table: &'static str) -> bool {
    let asked = journal.db().call(move |connection| {
        connection.query_row(
            "SELECT EXISTS (SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1)",
            [table],
            |row| row.get(0),
        )
    });
    asked.await.unwrap()
}

/// Whether the factory's own database holds shelves of `project`: of
/// the one called `before`, in this test.
fn held(_: &rusqlite::Connection, project: &str) -> rusqlite::Result<bool> {
    Ok(project == "before")
}

#[tokio::test]
async fn a_journal_is_made_with_the_tables_of_what_it_is_made_to_keep_and_one_that_keeps_less_is_given_none()
{
    if crate::one_journal() {
        return;
    }
    const SHELVES: &[&str] = &["CREATE TABLE shelves (id TEXT PRIMARY KEY);"];
    let dir = scratch("kept-made");
    let of_factory = factory(&dir).await;
    let early = of_factory.own_keeping("early", keeps::RECORDS).await.unwrap();
    let less = of_factory.own_keeping("less", keeps::RUNS).await.unwrap();
    // Said by the crate that keeps such tables, as it is opened: a
    // journal there already that keeps as much has them from then.
    of_factory.makes(keeps::RECORDS, "shelves", SHELVES, Some(held)).await.unwrap();
    assert!(has(&early, "shelves").await && !has(&less, "shelves").await);
    assert!(!has(&of_factory, "shelves").await, "nothing is made in the factory's own by this");
    // One made from then on is made with them where it is to keep as
    // much, and the level a journal is made at now is one that does.
    let own = of_factory.own("shop").await.unwrap();
    let more = of_factory.own_keeping("more", keeps::TURNS).await.unwrap();
    let none = of_factory.own_keeping("none", keeps::NOTHING).await.unwrap();
    assert!(has(&own, "shelves").await && has(&more, "shelves").await && !has(&none, "shelves").await);
    assert_eq!(of_factory.keeps_of("shop"), keeps::MADE_AT);
    // A project the factory's own database holds such rows of, as it
    // holds those of a project from before, is given a journal that
    // keeps what is run for it and no more: its rows stay where they
    // are, and are not looked for in a file that has none.
    let before = of_factory.own("before").await.unwrap();
    assert_eq!(of_factory.keeps_of("before"), keeps::RUNS);
    assert!(!has(&before, "shelves").await);
    assert!(of_factory.records_of(Some("before")).db().is(of_factory.db()));
    closed(of_factory).await;

    // Opened again, each is what its row says and has what it had:
    // saying it again makes nothing in a file whose row says less.
    let of_factory = factory(&dir).await;
    of_factory.makes(keeps::RECORDS, "shelves", SHELVES, Some(held)).await.unwrap();
    let of = |project: &str| of_factory.of_project(project).unwrap();
    assert!(has(&of("early"), "shelves").await && has(&of("shop"), "shelves").await);
    assert!(!has(&of("less"), "shelves").await && !has(&of("none"), "shelves").await);
    closed(of_factory).await;
}

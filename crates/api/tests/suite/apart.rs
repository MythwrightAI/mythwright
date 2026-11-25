//! What is run for a project is kept in the project's own journal, by a
//! conductor of the project's own, and is asked for through the
//! factory's as it always was: applied, read, listed, stopped, and taken
//! up when the daemon is started again. A project whose journal a
//! program from before listed, and one with no journal of its own, are
//! run by the factory's own conductor, with their rows in the factory's
//! database: nothing is moved by the daemon having started.

use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::Rig;

/// Counts what it hears said of its own project, and keeps one thing.
pub(crate) const COUNTER: &str = r#"
blueprint: counter
state: { heard: 0 }
steps:
  kept: { do: set, with: { heard: 0 } }
on:
  thing:
    event: s.thing
    steps:
      one: { do: set, with: { heard: "${event.data.n}" } }
      said: { do: emit, with: { topic: s.heard, n: "${event.data.n}" } }
"#;

pub(crate) async fn rig(label: &str) -> (Rig, PathBuf) {
    let mut data = PathBuf::new();
    let rig = Rig::with(label, |config, _| data = config.data_dir.clone()).await;
    (Rig::owned(rig).await, data)
}

/// How many rows `table` has in the database `file`, read while the
/// daemon runs: none, where the file has no such table.
pub(crate) fn rows(file: &Path, table: &str) -> i64 {
    let connection = journal::copy::Connection::open(file).unwrap();
    connection.query_row(&format!("SELECT count(*) FROM {table}"), [], |row| row.get(0)).unwrap_or(0)
}

/// Applies the counter for `project` and gives its instance and the run
/// of its steps, which went well.
pub(crate) async fn counter(rig: &Rig, project: &str) -> (String, String) {
    let asked = json!({ "source": COUNTER, "project": project, "wait": true });
    let applied = rig.send("/api/apply", asked).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
    let text = |key: &str| applied[key].as_str().unwrap().to_string();
    (text("instance"), text("run"))
}

/// Says a thing of `project` from outside, and waits for the counter
/// there to say it heard it.
pub(crate) async fn thing(rig: &Rig, project: &str, n: u64) {
    let mark = rig.mark();
    rig.send("/api/emit/s.thing", json!({ "project": project, "n": n })).await;
    rig.event("s.heard", |event| event.seq > mark && event.data["n"] == json!(n)).await;
}

/// What the counter `id` last heard, as its state says.
pub(crate) async fn heard(rig: &Rig, id: &str) -> Value {
    rig.read(&format!("/api/instances/{id}")).await["instance"]["state"]["heard"].clone()
}

pub(crate) fn ids(listed: &Value) -> Vec<String> {
    let all = listed["instances"].as_array().unwrap();
    all.iter().map(|one| one["id"].as_str().unwrap().to_string()).collect()
}

#[tokio::test]
async fn what_is_run_for_a_project_is_kept_in_its_own_journal_and_is_asked_for_as_it_always_was() {
    if crate::one_journal() {
        return;
    }
    let (mut rig, data) = rig("apart-own").await;
    for name in ["Shop", "Yard"] {
        rig.send("/api/projects", json!({ "name": name })).await;
    }
    let (shop, run) = counter(&rig, "shop").await;
    let (yard, _) = counter(&rig, "yard").await;
    // Nobody's: the factory's own conductor runs it.
    let asked = json!({ "source": COUNTER.replace("counter", "nobodys"), "wait": true });
    let nobodys = rig.send("/api/apply", asked).await["instance"].as_str().unwrap().to_string();

    // Each project's rows are in its own file, and the factory's has
    // what is nobody's and nothing of theirs.
    let (factory, of) = (data.join("journal.db"), |project: &str| data.join("journals").join(project));
    assert_eq!((rows(&of("shop.db"), "instances"), rows(&of("yard.db"), "instances")), (1, 1));
    assert_eq!(rows(&factory, "instances"), 1, "only what is nobody's is in the factory's own");
    assert!(rows(&of("shop.db"), "runs") >= 1 && rows(&of("shop.db"), "steps") >= 1);
    // And so is what the runner says of what it runs.
    let journal = rig.node().app.journal.clone();
    let started = journal::EventQuery { topics: "instance.started".into(), ..Default::default() };
    let said = |journal: journal::Journal| {
        let started = started.clone();
        async move {
            let all = journal.read(started).await.unwrap();
            all.into_iter().filter_map(|event| event.source.instance).collect::<Vec<_>>()
        }
    };
    assert_eq!(said(journal.of_project("shop").unwrap()).await, std::slice::from_ref(&shop));
    assert_eq!(said(journal.of_project("yard").unwrap()).await, std::slice::from_ref(&yard));
    assert_eq!(said(journal.clone()).await, std::slice::from_ref(&nobodys));

    // Asked through the factory's as ever: one, its run, a project's,
    // and all of them.
    let one = rig.read(&format!("/api/instances/{shop}")).await;
    assert_eq!(one["instance"]["project"], "shop", "{one}");
    assert_eq!(rig.read(&format!("/api/runs/{run}")).await["instance"], json!(shop));
    assert_eq!(ids(&rig.read("/api/instances?project=shop").await), std::slice::from_ref(&shop));
    let mut all = ids(&rig.read("/api/instances?limit=1000").await);
    all.sort();
    let mut meant = vec![shop.clone(), yard.clone(), nobodys.clone()];
    meant.sort();
    assert_eq!(all, meant);

    // Each hears its own project's and not the other's.
    thing(&rig, "shop", 1).await;
    thing(&rig, "yard", 2).await;
    assert_eq!((heard(&rig, &shop).await, heard(&rig, &yard).await), (json!(1), json!(2)));

    // Started again, each is taken up by its own conductor and goes on.
    rig.restart().await;
    thing(&rig, "shop", 3).await;
    assert_eq!((heard(&rig, &shop).await, heard(&rig, &yard).await), (json!(3), json!(2)));

    // And stopped through the factory's: written where it is kept.
    rig.send(&format!("/api/instances/{yard}/stop"), json!({})).await;
    let stopped = rig.read(&format!("/api/instances/{yard}")).await;
    assert_eq!(stopped["instance"]["status"], "stopped", "{stopped}");
    let mut live = ids(&rig.read("/api/instances?live=true&limit=1000").await);
    live.sort();
    let mut meant = vec![shop, nobodys];
    meant.sort();
    assert_eq!(live, meant);
}

#[tokio::test]
async fn a_project_whose_journal_says_nothing_of_what_is_run_stays_with_the_factorys_conductor() {
    if crate::one_journal() {
        return;
    }
    let (mut rig, data) = rig("apart-before").await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    rig.stop().await;
    // As a program from before listed a journal: it says nothing of
    // what is run, and nothing was ever run there.
    {
        let connection = journal::copy::Connection::open(data.join("journal.db")).unwrap();
        assert_eq!(connection.execute("UPDATE journals SET keeps = 0", []).unwrap(), 1);
    }
    rig.start().await;
    let (shop, _) = counter(&rig, "shop").await;
    let (factory, own) = (data.join("journal.db"), data.join("journals").join("shop.db"));
    assert_eq!((rows(&factory, "instances"), rows(&own, "instances")), (1, 0));
    thing(&rig, "shop", 1).await;
    // Nothing moves it by the daemon having started, and it goes on.
    rig.restart().await;
    assert_eq!((rows(&factory, "instances"), rows(&own, "instances")), (1, 0));
    thing(&rig, "shop", 2).await;
    let one = rig.read(&format!("/api/instances/{shop}")).await;
    assert_eq!(one["instance"]["state"]["heard"], json!(2), "{one}");
    // A project added now is run by a conductor of its own, beside it.
    rig.send("/api/projects", json!({ "name": "Yard" })).await;
    let (yard, _) = counter(&rig, "yard").await;
    assert_eq!(rows(&data.join("journals").join("yard.db"), "instances"), 1);
    assert_eq!(ids(&rig.read("/api/instances?project=yard").await), [yard]);
}

#[tokio::test]
async fn a_project_given_such_a_journal_while_something_of_it_runs_is_not_run_by_two() {
    if crate::one_journal() {
        return;
    }
    let (mut rig, data) = rig("apart-both").await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    let marked = |rows: i64| {
        let connection = journal::copy::Connection::open(data.join("journal.db")).unwrap();
        assert_eq!(connection.execute("UPDATE journals SET keeps = ?1", [rows]).unwrap(), 1);
    };
    rig.stop().await;
    marked(0);
    rig.start().await;
    let (shop, _) = counter(&rig, "shop").await;
    // Its journal comes to say that it keeps what is run, as that of a
    // project taken away and added again does, with the counter alive
    // in the factory's own rows.
    rig.stop().await;
    marked(1);
    rig.start().await;
    // Applied again, it is the one there is that answers: nothing is
    // started beside it by a second conductor.
    let asked = json!({ "source": COUNTER, "project": "shop", "wait": true });
    let again = rig.send("/api/apply", asked.clone()).await;
    assert_eq!((again["instance"].as_str(), again["created"].as_bool()), (Some(shop.as_str()), Some(false)));
    thing(&rig, "shop", 1).await;
    assert_eq!(ids(&rig.read("/api/instances?project=shop&live=true").await), std::slice::from_ref(&shop));
    let (factory, own) = (data.join("journal.db"), data.join("journals").join("shop.db"));
    assert_eq!((rows(&factory, "instances"), rows(&own, "instances")), (1, 0));

    // Once nothing of it is alive there as the daemon starts, what is
    // applied for it is run by its own; what ran before is still read.
    rig.send(&format!("/api/instances/{shop}/stop"), json!({})).await;
    rig.restart().await;
    let now = rig.send("/api/apply", asked).await["instance"].as_str().unwrap().to_string();
    assert_ne!(now, shop);
    assert_eq!((rows(&factory, "instances"), rows(&own, "instances")), (1, 1));
    thing(&rig, "shop", 2).await;
    assert_eq!(heard(&rig, &now).await, json!(2));
    let mut all = ids(&rig.read("/api/instances?project=shop").await);
    all.sort();
    let mut meant = vec![shop, now];
    meant.sort();
    assert_eq!(all, meant);
}

/// Everything a journal keeps in its small durable state: each name
/// with its key.
async fn kept(journal: &journal::Journal) -> Vec<(String, String)> {
    let read = journal.db().call(|connection| {
        let mut statement = connection.prepare("SELECT namespace, key FROM kv ORDER BY namespace, key")?;
        let rows = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
    });
    read.await.unwrap()
}

/// Whether the journal's own rows hold the instance `id`: not, where
/// it keeps nothing that is run.
async fn runs_it(journal: &journal::Journal, id: &str) -> bool {
    let id = id.to_string();
    let read = journal.db().call(move |connection| {
        connection.query_row("SELECT EXISTS (SELECT 1 FROM instances WHERE id = ?1)", [id], |row| row.get(0))
    });
    read.await.unwrap_or(false)
}

#[tokio::test]
async fn what_is_kept_beside_a_projects_rows_is_in_its_own_file_and_each_name_a_file_keeps_is_whose_the_lists_say()
 {
    use runs::beside::{PINNED, VOUCHED};
    if crate::one_journal() {
        return;
    }
    // A fleet, which reaches texts that ship and was applied for a
    // project by somebody: its tree is held to those texts and to the
    // project's files as that somebody vouched for them.
    let (rig, _at) = crate::fleets::rig().await;
    crate::fleets::shop(&rig, json!({})).await;
    let journal = rig.node().app.journal.clone();
    let own = journal.of_project("shop").unwrap();
    let names = |all: &[(String, String)], name: &str| all.iter().filter(|(kept, _)| kept == name).count();
    let (in_own, in_factory) = (kept(&own).await, kept(&journal).await);
    for name in [PINNED, VOUCHED] {
        assert!(names(&in_own, name) > 0, "the project's journal keeps nothing under `{name}`: {in_own:?}");
        assert_eq!(names(&in_factory, name), 0, "the factory's journal keeps `{name}` for a project's tree");
    }

    // And every name, in every file, is whose the lists say.
    held_to_the_lists(&journal).await;
}

/// Holds every name that every journal of a factory keeps something
/// under to what the lists say of it (the runner's and the
/// comparisons', which with the studio's own place among a journal's
/// events are all that may be in a project's file beside the
/// journal's own): what is the factory's is in the factory's
/// journal alone, what is an instance's is where that instance's rows
/// are, and what is a project's is in that project's journal.
pub(crate) async fn held_to_the_lists(journal: &journal::Journal) {
    use runs::beside::{Whose, instance_of, project_of};
    let lists = [runs::beside::KEPT, comparisons::copy::KEPT].concat();
    for one in journal.every() {
        let project = journal::project_of(one.name()).map(str::to_string);
        for (name, key) in kept(&one).await {
            let at = format!("`{name}` `{key}` in the journal `{}`", one.name());
            let listed = lists.iter().find(|(listed, ..)| *listed == name);
            match (listed, &project) {
                _ if name == "journal" => {}
                // What another crate keeps is the factory's.
                (None, None) => {}
                // Where the keeper of histories has read to in a
                // project's journal is kept there, with its lines.
                (None, Some(_)) if studio::copy::KEPT_BESIDE.contains(&name.as_str()) => {}
                (None, Some(_)) => panic!("{at} is on no list of what is kept beside a project's rows"),
                (Some((_, Whose::Factory, _)), _) => assert!(project.is_none(), "{at} is the factory's"),
                (Some((_, Whose::Instance, _)), _) => {
                    assert!(
                        runs_it(&one, instance_of(&key)).await,
                        "{at} is not where its instance's rows are"
                    );
                }
                // What is run for a project whose journal does not keep
                // it is the factory's to keep, and so is this.
                (Some((_, Whose::Project, _)), _) => {
                    let of = project_of(&key).filter(|of| journal.rows_of(of).is_some());
                    assert_eq!(of, project.as_deref(), "{at}");
                }
                (Some((_, Whose::Conductor, _)), _) => {}
            }
        }
    }
}

/// Every table a crate above the runner keeps, with how its rows are
/// told apart by project, and how much a project's journal has to keep
/// for the table's rows to be kept there: nothing, for one that is only
/// ever in the factory's own.
fn lists() -> Vec<(&'static str, journal::apart::Told, Option<i64>)> {
    let apart = [studio::apart::KEPT_APART, agents::apart::KEPT_APART].concat();
    let all = [studio::apart::TABLES, agents::apart::TABLES, hosting::apart::TABLES, topics::APART].concat();
    all.into_iter()
        .map(|(table, told)| (table, told, apart.iter().find(|(kept, _)| *kept == table).map(|(_, at)| *at)))
        .collect()
}

/// Reads every row of every table the lists name, in every file of the
/// factory, and holds each to where its table's line says it is kept: a
/// project's file holds only that project's rows, of the tables its
/// journal was made to keep, and the factory's own holds none of those
/// of such a project. Gives how many rows each project's file held, by
/// table. Asked at the end of a journey that is played anyway
/// (`whose.rs`), and of anything that moves a project's rows.
pub(crate) async fn rows_are_kept_where_their_lines_say(rig: &Rig) -> Vec<(String, &'static str, usize)> {
    let factory = rig.node().app.journal.clone();
    let mut held = Vec::new();
    for journal in factory.every() {
        let own = journal.name().strip_prefix("project:").map(str::to_string);
        for (table, told, apart) in lists() {
            let read = format!("SELECT {} FROM {table} kept", told.reads());
            let rows = journal.db().call(move |connection| {
                let made: bool = connection.query_row(
                    "SELECT EXISTS (SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1)",
                    [table],
                    |row| row.get(0),
                )?;
                if !made {
                    return Ok(Vec::new());
                }
                let mut statement = connection.prepare(&read)?;
                statement.query_map([], |row| row.get::<_, Option<String>>(0))?.collect()
            });
            let rows: Vec<Option<String>> = rows.await.unwrap();
            match &own {
                // A project's own file: its rows, of what it was made to keep.
                Some(project) => {
                    let keeps = apart.is_some_and(|at| factory.keeps_of(project) >= at);
                    let astray: Vec<&Option<String>> =
                        rows.iter().filter(|whose| !keeps || whose.as_deref() != Some(project)).collect();
                    assert!(astray.is_empty(), "`{table}` in the journal of `{project}` holds {astray:?}");
                    held.push((project.clone(), table, rows.len()));
                }
                // The factory's own: nothing a project's journal keeps.
                None => {
                    let astray: Vec<&String> = rows
                        .iter()
                        .flatten()
                        .filter(|whose| apart.is_some_and(|at| factory.keeps_of(whose) >= at))
                        .collect();
                    assert!(
                        astray.is_empty(),
                        "`{table}` in the factory's own journal holds rows of {astray:?}"
                    );
                }
            }
        }
    }
    held
}

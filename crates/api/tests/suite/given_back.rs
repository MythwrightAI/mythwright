//! What a node gives back when it stops.
//!
//! A daemon that stops is a process that ends, and the system takes back
//! whatever it held. A node stopped inside a process that goes on (a test
//! that starts a daemon again on what it left, a simulation that does so
//! at every step) has to give those things back itself: the system gives
//! one user only so many ways of being told that files changed, and a
//! process that kept one for every node it ever started used them up, for
//! itself and for everything else running as that user.
#![cfg(target_os = "linux")]

use std::os::unix::fs::MetadataExt;
use std::path::Path;
use std::time::Duration;

use serde_json::json;

use crate::{Rig, soon};

/// The folders under `dir`, itself among them, as the system numbers them.
fn folders(dir: &Path, into: &mut Vec<String>) {
    let Ok(here) = std::fs::metadata(dir) else { return };
    into.push(format!("ino:{:x} ", here.ino()));
    for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
            folders(&entry.path(), into);
        }
    }
}

/// How many of this process's ways of being told of changes are following
/// a folder under `dir`, as the system itself says.
fn following(dir: &Path) -> usize {
    let mut under = Vec::new();
    folders(dir, &mut under);
    std::fs::read_dir("/proc/self/fdinfo")
        .unwrap()
        .flatten()
        .filter(|entry| {
            std::fs::read_to_string(entry.path()).is_ok_and(|said| {
                said.lines().any(|line| {
                    line.starts_with("inotify") && under.iter().any(|folder| line.contains(folder))
                })
            })
        })
        .count()
}

/// Has the node follow its own folder, and says whether the system let it.
/// It gives one user only so many ways of being told, and the suites run
/// many nodes at once: where it gave this node none there is nothing of
/// that kind to give back, and nothing is waited for that will not be said.
fn follows(rig: &Rig) -> bool {
    let followed = rig.node().app.hosting.watch().folder(&rig.dir).is_some();
    if !followed {
        eprintln!("the system gave this node no way of following a folder: only its databases are looked at");
    }
    followed
}

#[tokio::test]
async fn a_node_that_is_stopped_follows_nothing_more_and_has_closed_its_databases() {
    let mut rig = Rig::new().await;
    if follows(&rig) {
        assert_eq!(following(&rig.dir), 1, "a node follows its files with one watch");
    }
    let journal = rig.node().app.journal.clone();

    rig.stop().await;

    // The watch is closed by the thread that reads it, a moment after it is told.
    soon("the node to have given back what it followed", async {
        while following(&rig.dir) > 0 {
            tokio::task::yield_now().await;
        }
    })
    .await;
    let refused = journal.read(journal::EventQuery::default()).await.unwrap_err();
    assert!(refused.message.contains("the database thread has stopped"), "{refused:?}");

    // Started again on what it left, it follows it again, with one watch.
    rig.start().await;
    if follows(&rig) {
        assert_eq!(following(&rig.dir), 1);
    }
}

/// How many files this process holds open under `dir` whose names end so,
/// as the system itself says.
fn open_under(dir: &Path, ending: &str) -> usize {
    std::fs::read_dir("/proc/self/fd")
        .unwrap()
        .flatten()
        .filter(|entry| {
            std::fs::read_link(entry.path())
                .is_ok_and(|to| to.starts_with(dir) && to.to_string_lossy().ends_with(ending))
        })
        .count()
}

/// Which files this process holds open under `dir`, each once however
/// often it is open.
fn held_under(dir: &Path) -> std::collections::BTreeSet<std::path::PathBuf> {
    std::fs::read_dir("/proc/self/fd")
        .unwrap()
        .flatten()
        .filter_map(|entry| std::fs::read_link(entry.path()).ok())
        .filter(|to| to.starts_with(dir))
        .collect()
}

#[tokio::test]
async fn a_node_that_is_let_go_of_holds_no_file_of_its_own_any_more_stopped_or_not() {
    let says =
        "blueprint: says\npermissions: [shell]\nsteps:\n  say: { do: shell, with: { cmd: [echo, hello] } }\n";
    for stopped in [false, true] {
        let mut rig = Rig::new().await;
        // Something is run, so that a step's log is open beside the databases.
        let said = rig.send("/api/apply", json!({ "source": says, "wait": true })).await;
        assert_eq!(said["finished"]["status"], "ok", "{said}");
        assert!(open_under(&rig.dir, ".db") >= 2, "its journal and what it searches");
        assert!(open_under(&rig.dir, ".log") >= 1, "the log of the step that spoke");
        // And enough is written for the journal's log to have been brought
        // back into its database: by a thread of its own, which has the
        // file open as well, and is given back with the rest.
        {
            let journal = rig.node().app.journal.clone();
            soon("the journal's log to be brought back", async {
                // Each time something else: what is written again as it
                // was is no write at all.
                let mut times = 0;
                while journal.db().brought_back() == 0 {
                    times += 1;
                    let filled = json!(format!("{times} {}", "written ".repeat(200)));
                    journal.kv_set("given-back", "filled", &filled).await.unwrap();
                }
            })
            .await;
            journal.kv_set("given-back", "filled", &json!(null)).await.unwrap();
            assert!(open_under(&rig.dir, "journal.db") >= 2, "the writer and whoever brings its log back");
        }

        // Stopped in order, or put away as a test's node is when the test
        // ends, with nobody having stopped it.
        match stopped {
            // Stopped is when its files are free, and not a moment before:
            // a daemon started again on this data opens them at once.
            true => {
                rig.stop().await;
                assert_eq!(open_under(&rig.dir, ""), 0, "a daemon that has stopped still holds a file");
            }
            false => drop(rig.node.take()),
        }

        // Its databases are closed by the threads that have them, a moment
        // after they are told.
        soon("the node to have given back its files", async {
            while open_under(&rig.dir, "") > 0 {
                tokio::task::yield_now().await;
            }
        })
        .await;
    }
}

/// How many threads of this process are called `name`, as the system says.
fn threads_called(name: &str) -> usize {
    std::fs::read_dir("/proc/self/task")
        .unwrap()
        .flatten()
        .filter(|task| {
            std::fs::read_to_string(task.path().join("comm")).is_ok_and(|comm| comm.trim() == name)
        })
        .count()
}

#[tokio::test]
async fn a_node_holds_three_files_for_the_journal_of_each_project_and_gives_them_back() {
    if crate::one_journal() {
        return;
    }
    for stopped in [false, true] {
        let mut data = std::path::PathBuf::new();
        let rig = Rig::with("given-back-journals", |config, _| data = config.data_dir.clone()).await;
        let mut rig = Rig::owned(rig).await;
        let journals = data.join("journals");
        assert_eq!(open_under(&journals, ""), 0);
        let before = held_under(&data);
        for name in ["Shop", "Yard", "Loft"] {
            rig.send("/api/projects", json!({ "name": name })).await;
        }
        // What a journal of its own costs a node: its database, the log
        // beside it and what the two are read through, each a file held
        // open; and a thread, which is counted below with every other
        // node's of this process and so only said to be there.
        assert_eq!(open_under(&journals, ".db"), 3, "a database for each project");
        assert_eq!(open_under(&journals, ""), 9, "and with each its log and what both are read through");
        // Nothing else of the node holds a file for a project's being
        // there. Held is open still once the node has gone on: a file of
        // the project's own is open for an instant as it is written and
        // as it is first read (the hosted gate counted one in that
        // instant), and a database the node had open before may open its
        // file again, or the log beside it, which how much was written
        // decides. Neither is a file kept open for a project.
        let beside = |held: &Path| {
            let held = held.to_string_lossy().into_owned();
            let of = held.trim_end_matches("-wal").trim_end_matches("-shm");
            before.contains(Path::new(of))
        };
        let mut since = Vec::new();
        for _ in 0..20_000 {
            since = held_under(&data)
                .into_iter()
                .filter(|held| !held.starts_with(&journals) && !before.contains(held) && !beside(held))
                .collect();
            if since.is_empty() {
                break;
            }
            tokio::task::yield_now().await;
        }
        assert!(since.is_empty(), "held since the projects were added, and no project's journal: {since:?}");
        assert!(
            threads_called("journal-db") >= 5,
            "a thread for each database: three projects', the factory's and what it searches"
        );

        match stopped {
            true => {
                rig.stop().await;
                assert_eq!(open_under(&data, ""), 0, "a daemon that has stopped still holds a file");
            }
            false => drop(rig.node.take()),
        }
        soon("the node to have given back its journals' files", async {
            while open_under(&data, "") > 0 {
                tokio::task::yield_now().await;
            }
        })
        .await;
    }
}

#[tokio::test]
async fn a_node_that_is_let_go_of_speaks_for_no_port_any_more_stopped_or_not() {
    // A service that is given a port has it spoken for by a socket its
    // factory holds (`hosting::ports`), which is a file of this process
    // and a number kept from every other factory on the machine.
    let hosts = "\
blueprint: hosts
permissions: [fleet]
steps:
  up: { do: service.start, with: { name: kept, cmd: IDLES, restart: never } }
"
    .replace("IDLES", &crate::standin::cmd(&["idle"]));
    for stopped in [false, true] {
        let mut rig = Rig::new().await;
        let said = rig.send("/api/apply", json!({ "source": hosts, "wait": true })).await;
        assert_eq!(said["finished"]["status"], "ok", "{said}");
        let hosting = rig.node().app.hosting.clone();
        let port = hosting.services()[0].port;
        assert_eq!(hosting.spoken_for(), [port]);
        assert!(hosting::ports::speak_for(port).is_none(), "the system holds it for the node");

        match stopped {
            true => rig.stop().await,
            false => drop(rig.node.take()),
        }
        // Its service goes on, and whatever finds it speaks for its port.
        assert!(hosting.spoken_for().is_empty(), "a node that is gone keeps a port from the next");
    }
}

/// Keeps the database's thread at a piece of work until it is let go:
/// what says it has come to the work, and what lets it go.
fn kept_at_work(db: &journal::Db) -> (tokio::sync::oneshot::Receiver<()>, std::sync::mpsc::Sender<()>) {
    let (entered, enters) = tokio::sync::oneshot::channel();
    let (go, kept) = std::sync::mpsc::channel::<()>();
    // Apart, so that nothing asked before it waits for it.
    drop(db.call_apart(move |_| {
        let _ = entered.send(());
        // Let go, or the test over: either ends the wait.
        let _ = kept.recv();
        Ok(())
    }));
    (enters, go)
}

#[tokio::test]
async fn a_node_stopped_while_a_database_is_at_work_waits_for_its_file_and_says_which_it_gave_up_on() {
    if crate::one_journal() {
        return;
    }
    let mut rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;

    // The order is made, and not left to a disk that is slow: the thread
    // of a project's database is kept at a piece of work, as it is by
    // what it was in the middle of or by putting its file away on a
    // machine with much else to write, while the node is stopped. Given
    // up on after two seconds with nothing said, such a database was what
    // the node started next was refused by: the database is locked.
    let (factory, shop) = {
        let journal = &rig.node().app.journal;
        (journal.db().clone(), journal.of_project("shop").unwrap().db().clone())
    };
    let (enters, go) = kept_at_work(&shop);
    soon("the project's database to come to the piece of work", enters).await.unwrap();
    let node = rig.node.take().expect("the node is running");
    {
        let stopping = node.stop();
        tokio::pin!(stopping);
        // Every database was asked to close before any is waited for:
        // the factory's own is put away while the project's is at work.
        soon("the factory's own database to be put away while the project's is at work", async {
            tokio::select! {
                biased;
                left = &mut stopping => panic!("the node stopped while a database was at work: {left:?}"),
                () = factory.closed() => {}
            }
        })
        .await;
        assert!(open_under(&rig.dir, "shop.db") > 0, "the project's database is still at work");
        go.send(()).expect("the database is kept at the piece of work");
        let left = soon("the node to stop once its database is let go", stopping).await;
        assert_eq!(left, Vec::<String>::new(), "every database was put away");
    }
    assert_eq!(open_under(&rig.dir, ""), 0, "a node that has stopped still holds a file");
    rig.start().await;

    // And a wait that runs out says what was waited for: which database,
    // and where the work its thread is at was asked for.
    let shop = rig.node().app.journal.of_project("shop").unwrap().db().clone();
    let (enters, go) = kept_at_work(&shop);
    soon("the project's database to come to the piece of work", enters).await.unwrap();
    let node = rig.node.take().expect("the node is running");
    let left = soon("a node to stop that waits for nothing", node.stop_within(Duration::ZERO)).await;
    let named = left.iter().find(|left| left.contains("journals/shop.db: "));
    let named = named.unwrap_or_else(|| panic!("the project's database is not named: {left:?}"));
    assert!(named.contains("doing the work asked at crates/api/tests/suite/given_back.rs:"), "{named}");
    go.send(()).expect("the database is kept at the piece of work");
    soon("the node to have given back its files", async {
        while open_under(&rig.dir, "") > 0 {
            tokio::task::yield_now().await;
        }
    })
    .await;
    rig.start().await;
}

#[tokio::test]
async fn a_node_that_is_let_go_of_is_given_back_whole_with_its_journals_and_its_engine_stopped_or_not() {
    // Each was held for the life of the process by something it held in
    // turn: the journal by what its events are held to, which held a copy
    // of every action; the engine by the gateway; the studio by itself,
    // through what reads a board's columns and what a card may be given
    // to; and the journal again by what keeps the histories of records,
    // which nothing ended. Some two megabytes for every node, and the
    // suite that starts hundreds came to seven gigabytes.
    for stopped in [false, true] {
        let mut rig = Rig::new().await;
        rig.send("/api/projects", json!({ "name": "Shop" })).await;
        rig.board("shop").await;
        let app = &rig.node().app;
        let journals = [app.journal.downgrade(), app.journal.of(Some("shop")).downgrade()];
        let engine = app.engine.downgrade();
        if stopped {
            rig.stop().await;
        }
        drop(rig);
        soon("the node's journals and its engine to be given back", async {
            while !journals.iter().all(journal::Downgraded::is_given_back) || engine.upgrade().is_some() {
                tokio::task::yield_now().await;
            }
        })
        .await;
    }
}

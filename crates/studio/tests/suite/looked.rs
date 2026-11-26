//! A look over a project's folder (`Projects::scan`) and a file written
//! through the factory at the same time: what changed is said once, by
//! whoever wrote it.
//!
//! A look walks the folder, then has the database's thread compare what
//! it found with what is written down, looking again at each file the two
//! disagree about. A write puts its file on the disk, then writes it down
//! on the same thread. Here the thread is held, so that the look waits
//! there with its walk done while the write puts its file on the disk and
//! waits behind it: the order a look came in when a run ended, or a
//! library's folder said it had changed, just as a plan was written
//! through the factory. The look then found the plan on the disk and not
//! written down, and said it had been changed by nobody before the write
//! said it was the agent's; what heard the first added the plan's cards
//! in nobody's name.

use std::sync::mpsc;
use std::time::Duration;

use journal::Db;

use crate::{Rig, ada, agent};

const PLAN: &str = "docs/plans/cart.md";

/// Waits, a turn of the runtime at a time, until `so` is true.
async fn until(what: &str, so: impl Fn() -> bool) {
    let waited = tokio::time::timeout(Duration::from_secs(60), async {
        while !so() {
            tokio::task::yield_now().await;
        }
    });
    waited.await.unwrap_or_else(|_| panic!("never came about: {what}"));
}

/// Holds `db`'s thread until what is handed back is told to go on.
async fn hold(db: &Db) -> (mpsc::Sender<()>, tokio::task::JoinHandle<()>) {
    let (go, told) = mpsc::channel::<()>();
    let (holding, held) = tokio::sync::oneshot::channel::<()>();
    let work = db.call(move |_| {
        let _ = holding.send(());
        let _ = told.recv();
        Ok(())
    });
    let done = tokio::spawn(async move { work.await.unwrap() });
    held.await.expect("the database's thread was never held");
    (go, done)
}

#[tokio::test]
async fn a_file_written_while_a_look_at_the_folder_waits_for_the_database_is_said_to_be_changed_by_its_writer_alone()
 {
    let rig = Rig::new().await;
    let projects = rig.studio.projects.clone();
    projects.create("Shop", None, &ada()).await.unwrap();
    projects.write("shop", PLAN, "# The cart\n", &ada()).await.unwrap();
    // Changed behind the factory's back, so that the look finds it changed.
    let file = projects.dir_of("shop").unwrap().join("docs").join("plans").join("cart.md");
    std::fs::write(&file, "# The cart, as it was left\n").unwrap();
    let from = rig.events("project.file.changed").await.len();

    let db = rig.journal.records_of(Some("shop")).db().clone();
    let (go, held) = hold(&db).await;
    let looked = tokio::spawn({
        let projects = projects.clone();
        async move { projects.scan("shop").await.unwrap() }
    });
    until("the look has walked the folder and waits for the database", || db.waiting() == 1).await;
    let architect = agent("shop-architect-1", &["projects:write"]);
    let wrote = tokio::spawn({
        let projects = projects.clone();
        async move { projects.write("shop", PLAN, "# The cart, as the architect planned it\n", &architect).await }
    });
    until("the write is on the disk and waits for the database", || db.waiting() == 2).await;
    go.send(()).unwrap();
    held.await.unwrap();
    looked.await.unwrap();
    wrote.await.unwrap().unwrap();

    let said: Vec<(String, Option<String>)> = rig.events("project.file.changed").await[from..]
        .iter()
        .filter(|event| event.data["path"] == PLAN)
        .map(|event| (event.data["change"].as_str().unwrap().to_string(), event.source.actor.clone()))
        .collect();
    assert_eq!(said, [("modified".to_string(), Some("agent:shop-architect-1".to_string()))]);
    assert_eq!(projects.read("shop", PLAN).await.unwrap(), "# The cart, as the architect planned it\n");
    // Written down as it is: the next look finds nothing to say of it.
    let again = projects.scan("shop").await.unwrap();
    assert!(again.iter().all(|change| change.path != PLAN), "{again:?}");
}

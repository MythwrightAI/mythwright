//! A step that adds a row, adds a card or says something on a record,
//! stopped anywhere, comes to the same as one never stopped: the row, the
//! card and what was said are each there once. Each of the three is the
//! same done twice (`record.add`, `board.add`, `record.comment`): told
//! that it is being done again, it looks for what its step left, by the
//! run, the step and which of the things done under the step's name it
//! is, in what was said of it since the step began.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use serde_json::{Value, json};

use crate::Rig;

const TWICE: &str = r#"blueprint: twice
permissions: [projects, board, records]
state:
  each: [one, two]
steps:
  table:
    do: collection.ensure
    with: { name: notes, fields: { title: { type: text } } }
  left_row:
    needs: [table]
    do: record.add
    with: { collection: notes, fields: { title: Once } }
  left_card:
    needs: [left_row]
    do: board.add
    with: { title: Once }
  left_said:
    needs: [left_card]
    do: record.comment
    with: { id: "${steps.left_card.out.id}", body: Said once }
  left_many:
    needs: [left_said]
    for_each: ${state.each}
    do: board.add
    with: { title: "Each ${item}" }
"#;

/// The steps that leave something, by their names: names no blueprint
/// that ships gives a step or a handler, so that what is counted is theirs.
const LEAVING: [&str; 4] = ["left_row", "left_card", "left_said", "left_many"];

async fn shop() -> Rig {
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    rig.board("shop").await;
    rig
}

fn of_leaving(event: &kernel::Event) -> bool {
    event.source.step.as_deref().is_some_and(|step| LEAVING.contains(&step))
}

/// Applies the text, with the daemon stopped after the `stop`-th thing
/// said of a step that leaves something, where one is given, and started
/// again; and waits for the run of its steps to end. What the run came
/// to, how many things were said of those steps all told, and what the
/// daemon was stopped after.
async fn played(stop: Option<usize>, saying: bool) -> (Rig, Value, usize, String) {
    let mut rig = shop().await;
    let said = Arc::new(AtomicUsize::new(0));
    let at = Arc::new(Mutex::new(String::new()));
    let (counted, stopped) = (said.clone(), at.clone());
    let journal = rig.node().app.journal.clone();
    let at_place = move |event: &kernel::Event| {
        if !of_leaving(event) {
            return false;
        }
        let nth = counted.fetch_add(1, Ordering::SeqCst) + 1;
        if stop == Some(nth) {
            let step = event.source.step.clone().unwrap_or_default();
            *stopped.lock().unwrap() = format!("{} of {step}", event.topic);
        }
        stop == Some(nth)
    };
    // Shut whole at the place, or with what was being written kept and
    // nothing more said: the two ways a daemon stops.
    match saying {
        true => journal.shut_after_saying(at_place),
        false => journal.shut_after(at_place),
    }
    let apply = json!({ "source": TWICE, "project": "shop" });
    match stop {
        Some(_) => {
            tokio::select! {
                _ = rig.post("/api/apply", apply) => {}
                () = journal.shut() => {}
            }
            crate::soon(
                &format!("the daemon to come to thing {} said of the steps", stop.unwrap_or(0)),
                journal.shut(),
            )
            .await;
            rig.restart().await;
        }
        None => {
            rig.send("/api/apply", apply).await;
        }
    }
    let ended = rig
        .event("run.finished", |event| event.data["blueprint"] == "twice" && event.data["flow"] == "main")
        .await;
    let came = ended.data.clone();
    let count = said.load(Ordering::SeqCst);
    let stopped = at.lock().unwrap().clone();
    (rig, came, count, stopped)
}

async fn titles(rig: &Rig) -> Vec<String> {
    let board = rig.read("/api/board/shop").await;
    let mut titles: Vec<String> = board["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|card| card["title"].as_str().unwrap().to_string())
        .collect();
    titles.sort_unstable();
    titles
}

async fn rows(rig: &Rig) -> usize {
    let notes = rig.read("/api/collections/shop/notes/records").await;
    notes["records"].as_array().map_or(0, Vec::len)
}

async fn comments(rig: &Rig) -> usize {
    let said = journal::EventQuery { topics: "record.commented".into(), limit: 1000, ..Default::default() };
    rig.said(said).await.iter().filter(|event| event.data["body"] == "Said once").count()
}

#[tokio::test]
async fn a_row_a_card_and_what_is_said_on_one_are_each_left_once_wherever_the_daemon_stops() {
    let (rig, came, places, _) = played(None, false).await;
    assert_eq!(came["status"], "ok", "{came}");
    let usual = (titles(&rig).await, rows(&rig).await, comments(&rig).await);
    assert_eq!(usual.0, ["Each one", "Each two", "Once"], "{usual:?}");
    assert_eq!((usual.1, usual.2), (1, 1), "{usual:?}");
    assert!(places >= 8, "each step that leaves something says something of it: {places}");
    drop(rig);
    let mut stopped = Vec::new();
    for (stop, saying) in (1..=places).flat_map(|stop| [(stop, false), (stop, true)]) {
        let (rig, came, _, at) = played(Some(stop), saying).await;
        assert_eq!(came["status"], "ok", "stopped after {at} (saying {saying}): {came}");
        let now = (titles(&rig).await, rows(&rig).await, comments(&rig).await);
        assert_eq!(now, usual, "stopped after {at} (saying {saying})");
        stopped.push(at);
    }
    // Among the places, those just after each thing was left, with what
    // says the step has ended not yet written: where it is found.
    for left in [
        "record.created of left_row",
        "board.item.created of left_card",
        "record.commented of left_said",
        "board.item.created of left_many",
    ] {
        assert!(stopped.iter().any(|at| at == left), "stopped after {left}: {stopped:?}");
    }
}

const FILED: &str = r#"blueprint: filed
permissions: [projects, records]
steps:
  table:
    do: collection.ensure
    with: { name: notes, store: files, fields: { title: { type: text } } }
  left_row:
    needs: [table]
    do: record.add
    with: { collection: notes, fields: { title: Once } }
"#;

#[tokio::test]
async fn a_row_of_a_table_kept_as_files_is_found_where_it_was_said_and_not_added_again_where_it_may_be_there_unsaid()
 {
    // Stopped as the row is said: it is there, said, and found.
    let mut rig = shop().await;
    let journal = rig.node().app.journal.clone();
    journal.shut_after(|event| {
        event.topic == "record.created" && event.source.step.as_deref() == Some("left_row")
    });
    tokio::select! {
        _ = rig.post("/api/apply", json!({ "source": FILED, "project": "shop" })) => {}
        () = journal.shut() => {}
    }
    crate::soon("the daemon to come to where it is stopped", journal.shut()).await;
    rig.restart().await;
    let ended = rig.event("run.finished", |event| event.data["blueprint"] == "filed").await;
    assert_eq!(ended.data["status"], "ok", "{}", ended.data);
    assert_eq!(rows(&rig).await, 1);

    // Stopped as the step begins: whether a file was written that nothing
    // said cannot be told, and nothing is added a second time.
    let mut rig = shop().await;
    let journal = rig.node().app.journal.clone();
    journal.shut_after(|event| {
        event.topic == "step.started" && event.source.step.as_deref() == Some("left_row")
    });
    tokio::select! {
        _ = rig.post("/api/apply", json!({ "source": FILED, "project": "shop" })) => {}
        () = journal.shut() => {}
    }
    crate::soon("the daemon to come to where it is stopped", journal.shut()).await;
    rig.restart().await;
    let ended = rig.event("run.finished", |event| event.data["blueprint"] == "filed").await;
    assert_eq!(ended.data["status"], "failed", "{}", ended.data);
    let why = ended.data["error"].as_str().unwrap_or_default();
    assert!(why.contains("keeps its records outside the factory's database"), "{why}");
    assert_eq!(rows(&rig).await, 0);
}

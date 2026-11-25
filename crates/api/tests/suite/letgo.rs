//! How long the detail of runs' steps is kept, and where their output is
//! written (`api::letgo`): said by the owner, let go of by its age with
//! what runs came to kept, held back where something is still owed it,
//! written and read in a folder of the owner's, and taken into a copy as
//! far as a copy is another factory.

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use journal::EventQuery;
use kernel::ManualClock;
use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;

const DAY: Duration = Duration::from_secs(24 * 3600);

/// A step that prints, and so writes its file.
fn work(key: &str) -> Value {
    let source = format!(
        "blueprint: work\npermissions: [shell]\nsteps:\n  say:\n    do: shell\n    with: {{ cmd: {} }}\n",
        crate::standin::cmd(&["say", "it was done"])
    );
    json!({ "source": source, "key": key, "wait": true })
}

/// A node on a clock the test moves, which stands where the wall's does
/// as it starts: a file is old by when it was last written, which is the
/// wall's to say.
async fn rig() -> (Rig, ManualClock) {
    let clock = ManualClock::new();
    let wall = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as i64;
    clock.advance(Duration::from_millis((wall - kernel::Clock::now_ms(&clock)).max(0) as u64));
    let ticking = clock.clone();
    let rig =
        Rig::owned(Rig::with("api-letgo", move |config, _| config.clock = Some(Arc::new(ticking))).await)
            .await;
    (rig, clock)
}

/// Applies `work` under `key`, and gives back its run.
async fn worked(rig: &Rig, key: &str) -> Value {
    let applied = rig.send("/api/apply", work(key)).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
    let instance = applied["instance"].as_str().unwrap();
    let runs = rig.read(&format!("/api/runs?instance={instance}")).await;
    runs["runs"][0].clone()
}

/// Has the file `run`'s step printed into say it was last written at
/// the test's own now: a file's age is when it was last written, and the
/// wall's clock does not move with the test's.
fn written_now(rig: &Rig, run: &str, clock: &ManualClock) {
    let file = rig.dir.join("data").join("logs").join("runs").join(format!("{run}.say.log"));
    let at = std::time::UNIX_EPOCH + Duration::from_millis(kernel::Clock::now_ms(clock) as u64);
    std::fs::File::options().append(true).open(file).unwrap().set_modified(at).unwrap();
}

fn id(run: &Value) -> String {
    run["id"].as_str().unwrap().to_string()
}

/// What the journals keep on `topic` of `run`: what is only passed on
/// was never kept, and has nothing to be let go of.
async fn said_of(rig: &Rig, topic: &str, run: &str) -> usize {
    let query = EventQuery { topics: topic.into(), limit: 100_000, ..Default::default() };
    let kept = rig.node().app.journal.read_every(query).await.unwrap();
    kept.iter().filter(|event| event.source.run.as_deref() == Some(run)).count()
}

async fn log(rig: &Rig, run: &str) -> Value {
    rig.read(&format!("/api/runs/{run}/steps/say/log")).await
}

#[tokio::test]
async fn what_runs_steps_said_and_printed_is_let_go_after_the_days_said_and_what_they_came_to_is_kept() {
    let (rig, clock) = rig().await;
    // Unsaid, everything is kept for ever.
    let said = rig.read("/api/system/logs").await;
    assert_eq!((said["days"].clone(), said["folder"].clone()), (Value::Null, Value::Null), "{said}");

    // What the runner says of its steps and runs is kept only where
    // something could be owed it (`conductor::owed`); here it is all kept,
    // as where something heard it, so that there is something to let go.
    rig.node().app.engine.keeps_all_it_says();
    let old = id(&worked(&rig, "old").await);
    assert_eq!(log(&rig, &old).await["lines"][0]["text"], "it was done");
    let talk = rig.send("/api/channels", json!({ "name": "talk" })).await;
    let talk = talk["id"].as_str().unwrap().to_string();
    rig.send(&format!("/api/channels/{talk}/messages"), json!({ "body": "still here" })).await;
    clock.advance(2 * DAY);
    let new = id(&worked(&rig, "new").await);
    written_now(&rig, &new, &clock);

    let answer = rig.send("/api/system/logs", json!({ "days": 1 })).await;
    assert_eq!(answer["days"], 1, "{answer}");
    let went = &answer["let_go"];
    assert!(went["events"].as_u64().unwrap() >= 3 && went["files"] == 1, "{answer}");
    // Of the old run, what was said of its steps and what they printed;
    // what it came to stays, and so does everything of the new one.
    for topic in ["run.started", "step.started", "step.finished"] {
        assert_eq!(said_of(&rig, topic, &old).await, 0, "{topic}");
        assert_eq!(said_of(&rig, topic, &new).await, 1, "{topic}");
    }
    assert_eq!(said_of(&rig, "run.finished", &old).await, 1);
    let run = rig.read(&format!("/api/runs/{old}")).await;
    assert_eq!(run["status"], "ok", "{run}");
    assert_eq!(run["steps"][0]["status"], "ok", "{run}");
    assert!(run["let_go"].as_str().unwrap().contains("let go after a day"), "{run}");
    assert!(rig.read(&format!("/api/runs/{new}")).await.get("let_go").is_none());
    let printed = log(&rig, &old).await;
    assert_eq!(printed["lines"], json!([]), "{printed}");
    assert!(printed["let_go"].as_str().unwrap().contains("let go after a day"), "{printed}");
    assert_eq!(log(&rig, &new).await["lines"][0]["text"], "it was done");
    // What was said in a channel is no log.
    let talk = rig.read(&format!("/api/channels/{talk}/messages")).await;
    assert!(talk.to_string().contains("still here"), "{talk}");

    // From then on, on the daemon's own timer: an hour on, once more.
    let mark = rig.mark();
    clock.advance(2 * DAY);
    let gone = rig.event("system.logs.let_go", |event| event.seq > mark).await;
    assert_eq!(gone.data["files"], 1, "{}", gone.data);
    assert!(gone.data["events"].as_u64().unwrap() >= 3, "{}", gone.data);
    assert_eq!(said_of(&rig, "step.started", &new).await, 0);
    assert_eq!(said_of(&rig, "run.finished", &new).await, 1);

    // Said for ever again, nothing more goes.
    let answer = rig.send("/api/system/logs", json!({ "days": "always" })).await;
    assert_eq!((answer["days"].clone(), answer["let_go"].clone()), (Value::Null, Value::Null), "{answer}");
}

#[tokio::test]
async fn what_something_alive_has_not_yet_been_handed_is_not_let_go() {
    let (rig, clock) = rig().await;
    let hold = rig.dir.join("hold-heard");
    crate::hold(&hold);
    // It hears every step that ends, one at a time, and the first one it
    // hears it is held over.
    let heard = format!(
        "blueprint: hears\npermissions: [shell]\none_at_a_time: true\non:\n  ended:\n    event: step.finished\n    \
         do: shell\n    with: {{ cmd: {} }}\n",
        crate::standin::cmd(&["held", &hold.to_string_lossy(), "heard"])
    );
    let applied = rig.send("/api/apply", json!({ "source": heard, "key": "hears", "wait": true })).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
    let hears = applied["instance"].as_str().unwrap().to_string();
    let first = id(&worked(&rig, "first").await);
    rig.event("step.started", |event| event.source.instance.as_deref() == Some(hears.as_str())).await;
    // What it is owed and has not been handed: the end of another run.
    let owed = id(&worked(&rig, "owed").await);
    clock.advance(2 * DAY);

    let answer = rig.send("/api/system/logs", json!({ "days": 1 })).await;
    assert!(answer["let_go"]["held"].is_string(), "{answer}");
    assert_eq!(said_of(&rig, "step.finished", &first).await, 0, "what it had been handed goes");
    assert_eq!(said_of(&rig, "step.finished", &owed).await, 1, "what it is owed stays");

    // Let go, it hears it from the journal.
    crate::let_go("the first it heard to be held", &hold).await;
    crate::pass(&hold, 4);
    // It hears one at a time, so the first of its runs to end is the one
    // it was held over: each that ends is looked at in turn, until one
    // was set off by the end it was owed. Taken for that, the first was
    // read before the owed one had begun, now and then (once in a
    // playing of every suite told one journal).
    let mut after = 0;
    let mut runs = Value::Null;
    for _ in 0..8 {
        let ended = rig
            .event("run.finished", |event| {
                event.seq > after
                    && event.data["flow"] == "on.ended"
                    && event.data["status"] == "ok"
                    && event.source.instance.as_deref() == Some(hears.as_str())
            })
            .await;
        after = ended.seq;
        runs = rig.read(&format!("/api/runs?instance={hears}")).await;
        let had = |run: &Value| {
            run["event"]["source"]["run"].as_str() == Some(owed.as_str()) && run["status"] == "ok"
        };
        if runs["runs"].as_array().unwrap().iter().any(had) {
            return;
        }
    }
    panic!("the end of the run it was owed was never handed to it: {runs}");
}

#[tokio::test]
async fn a_folder_said_for_logs_is_written_in_from_then_on_and_what_was_written_before_is_read_where_it_is() {
    let (rig, _) = rig().await;
    let before = id(&worked(&rig, "before").await);
    let folder = rig.dir.join("elsewhere");
    let answer = rig.send("/api/system/logs", json!({ "folder": folder })).await;
    let whole = answer["folder"].as_str().unwrap().to_string();
    assert!(Path::new(&whole).join("runs").is_dir(), "{answer}");
    let there = id(&worked(&rig, "there").await);
    assert!(Path::new(&whole).join("runs").join(format!("{there}.say.log")).is_file());
    // Back to the data folder: what was written in each is read there.
    let answer = rig.send("/api/system/logs", json!({ "folder": "usual" })).await;
    assert_eq!(
        (answer["folder"].clone(), answer["before"].clone()),
        (Value::Null, json!([whole])),
        "{answer}"
    );
    let after = id(&worked(&rig, "after").await);
    let data = rig.dir.join("data").join("logs").join("runs");
    assert!(
        data.join(format!("{after}.say.log")).is_file() && data.join(format!("{before}.say.log")).is_file()
    );
    for run in [&before, &there, &after] {
        assert_eq!(log(&rig, run).await["lines"][0]["text"], "it was done", "{run}");
    }

    // A folder that is not said whole, or where nothing can be written,
    // is refused in a sentence and nothing changes.
    let file = rig.dir.join("a-file");
    std::fs::write(&file, "").unwrap();
    for (folder, says) in [
        ("logs/here".to_string(), "said whole"),
        (file.join("logs").to_string_lossy().into_owned(), "could not be made"),
    ] {
        let (status, refused) = rig.post("/api/system/logs", json!({ "folder": folder })).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");
        assert!(crate::held::said(&refused).contains(says), "{refused}");
    }
    assert_eq!(rig.read("/api/system/logs").await["folder"], Value::Null);
    // Neither is anybody's but whoever manages the factory to say or read.
    let member = crate::held::key(&rig, &["blueprints", "projects", "channels"]).await;
    for (method, body) in [(Method::GET, None), (Method::POST, Some(json!({ "days": 1 })))] {
        let (status, _) = rig.call(method, "/api/system/logs", Some(&member), body).await;
        assert_eq!(status, StatusCode::FORBIDDEN);
    }
}

#[tokio::test]
async fn a_copy_of_the_factory_keeps_how_long_its_logs_are_kept_and_writes_its_own_where_it_is() {
    let shared = crate::snapshots::Shared::new();
    let prod = shared.factory("prod").await;
    let folder = shared.0.join("prod-logs");
    prod.send("/api/system/logs", json!({ "days": 3, "folder": folder })).await;
    let to = shared.snapshot("prod-copy");
    crate::snapshots::taken(&prod, &to).await;
    let dev = shared.copy("dev", &to).await;
    let said = dev.read("/api/system/logs").await;
    assert_eq!(
        (said["days"].clone(), said["folder"].clone(), said["before"].clone()),
        (json!(3), Value::Null, json!([])),
        "{said}"
    );
}

#[tokio::test]
async fn a_fleet_that_is_up_holds_back_nothing_said_of_steps_it_is_not_owed() {
    let (rig, _at) = crate::fleets::rig().await;
    crate::fleets::shop(&rig, json!({})).await;
    // Its apply having answered, its stages are still coming up: each
    // has its column looked at (`std/due`) in a handler's run, and what
    // is written down and not yet in hand holds everything back, since
    // its text is known nowhere (the runner's suite's `letgo.rs` makes
    // that moment). At rest, everything the fleet ran is over, or alive
    // and in hand, and none of its texts hears or waits for what is said
    // of steps: it all goes, as soon as anything is old enough.
    crate::simulation::rested(&rig).await;
    let answer = rig.send("/api/system/logs", json!({ "days": 1 })).await;
    let went = &answer["let_go"];
    assert_eq!((went["events"].clone(), went["held"].clone()), (json!(0), Value::Null), "{answer}");
}

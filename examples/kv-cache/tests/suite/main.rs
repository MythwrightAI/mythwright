//! The KV cache example, end to end: the blueprint in
//! `examples/blueprints/kv-cache.bp` hosting the real binary through the
//! standard `std/slot` blueprint.
//!
//! These are the tests of the rollout story as a whole. Stage timing runs on
//! a manual clock, so a rollout with three twenty-second stages takes a few
//! hundred milliseconds; the processes and the traffic are real.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use conductor::{
    Applied, Apply, Engine, InstanceInfo, InstanceStatus, Layers, MemoryLibrary, Registry, RunInfo, RunStatus,
};
use hosting::{Hosting, Status};
use journal::{Journal, Logs};
use kernel::{Event, ManualClock, Principal, PrincipalKind};
use serde_json::{Value, json};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

const BLUEPRINT: &str = include_str!("../../../blueprints/kv-cache.bp");
const BINARY: &str = env!("CARGO_BIN_EXE_kv-cache");

struct Rig {
    engine: Engine,
    hosting: Hosting,
    clock: ManualClock,
    journal: Journal,
    logs: Logs,
    dir: PathBuf,
    /// What was said, kept or passed on, for the test to read.
    order: Arc<journal::Order>,
}

impl Drop for Rig {
    fn drop(&mut self) {
        self.engine.halt();
        self.hosting.halt();
        // A test that fails part-way must not leave its caches running:
        // whatever this rig's factory started and is still there is
        // ended, each with its group.
        for pid in hosting::end_under(&self.dir) {
            eprintln!(
                "the test was over with a program its factory started still running; its group {pid} was ended"
            );
        }
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

async fn soon<T>(what: &str, work: impl Future<Output = T>) -> T {
    match tokio::time::timeout(Duration::from_secs(30), work).await {
        Ok(value) => value,
        Err(_) => panic!("timed out waiting for: {what}"),
    }
}

async fn build(journal: &Journal, logs: &Logs, dir: &std::path::Path) -> (Engine, Hosting) {
    let hosting = Hosting::new(journal.clone(), logs.clone(), dir.join(".run")).await.unwrap();
    // The factory here is inside the test's program, which cannot be
    // started again as the wrapper of what the factory starts: its own
    // process stands in for one.
    hosting.stands_between();
    let mut registry = Registry::new();
    hosting.register(&mut registry);
    let library = Layers::new().then(Arc::new(MemoryLibrary::new())).then(Arc::new(conductor::standard()));
    let engine = Engine::new(journal.clone(), logs.clone(), registry, Arc::new(library)).await.unwrap();
    hosting.attach(&engine);
    let workdir = dir.to_path_buf();
    engine.set_workdirs(Arc::new(move |_| Some(workdir.clone())));
    (engine, hosting)
}

impl Rig {
    async fn new() -> Rig {
        let clock = ManualClock::new();
        let journal = Journal::memory(Arc::new(clock.clone())).await.unwrap();
        let order = Arc::new(journal::Order::default());
        journal.orders_by(order.clone());
        let dir =
            PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("kv-{}", &kernel::id::secret()[..12]));
        std::fs::create_dir_all(&dir).unwrap();
        let logs = Logs::new(dir.join(".logs"));
        let (engine, hosting) = build(&journal, &logs, &dir).await;
        Rig { engine, hosting, clock, journal, logs, dir, order }
    }

    async fn restart(&mut self) {
        self.engine.halt();
        self.hosting.halt();
        let (engine, hosting) = build(&self.journal, &self.logs, &self.dir).await;
        hosting.resume().await.unwrap();
        engine.resume().await.unwrap();
        self.engine = engine;
        self.hosting = hosting;
    }

    /// Applies the example blueprint for `version`, without waiting.
    async fn apply(&self, version: &str, fault: &str) -> Applied {
        let user = Principal {
            kind: PrincipalKind::User,
            id: "ada".into(),
            name: "Ada".into(),
            scopes: vec!["*".into()],
            key: None,
        };
        let params = json!({ "version": version, "fault": fault, "build": false, "binary": BINARY });
        let request = Apply {
            source: Some(BLUEPRINT.to_string()),
            params: kernel::fields(params),
            ..Apply::new("kv-cache", user)
        };
        self.engine.apply(request).await.unwrap_or_else(|err| panic!("apply failed: {}", err.message))
    }

    async fn finished(&self, applied: &Applied) -> RunInfo {
        soon("the run to finish", self.engine.wait_run(applied.run.as_ref().expect("a run was started")))
            .await
            .unwrap()
    }

    /// Deploys `version` where nothing was running and returns the address's port.
    async fn deploy_first(&self, version: &str) -> (Applied, u16) {
        let applied = self.apply(version, "").await;
        let run = self.finished(&applied).await;
        assert_eq!(run.status, RunStatus::Ok, "{:?}", run.error);
        (applied, run.outputs["port"].as_u64().unwrap() as u16)
    }

    async fn event(&self, topic: &str, wanted: impl Fn(&Event) -> bool) -> Arc<Event> {
        let mut follow = self.order.follow_every(&self.journal, topic);
        soon(&format!("an event on `{topic}`"), async {
            loop {
                let event = follow.next().await.unwrap();
                if wanted(&event) {
                    return event;
                }
            }
        })
        .await
    }

    /// Waits until the stage that gives the candidate `weight` is being
    /// watched: it has said so, and is parked on the clock. Not by counting
    /// the sleeps since the rollout began: the wait for the candidate to be
    /// healthy starts one only if it is not healthy already, and how soon
    /// it is depends on the machine.
    async fn watching(&self, weight: u64) {
        self.event("slot.shifted", |event| event.data["weight"] == weight).await;
        soon("the stage to start watching", async {
            loop {
                let started = self.clock.sleep_count();
                if self.clock.sleeping() > 0 {
                    return;
                }
                self.clock.sleeps_started(started + 1).await;
            }
        })
        .await;
    }

    async fn events(&self, topics: &str) -> Vec<Event> {
        let query = journal::EventQuery { topics: topics.to_string(), limit: 10_000, ..Default::default() };
        self.order.read_every(&self.journal, query).await.unwrap()
    }

    async fn instance(&self, id: &str) -> InstanceInfo {
        self.engine.instance(id).await.unwrap()
    }
}

/// Sends one command through the stable address and returns the reply.
async fn ask(port: u16, command: &str) -> Result<String, String> {
    let mut stream =
        TcpStream::connect(("127.0.0.1", port)).await.map_err(|err| format!("connect: {err}"))?;
    stream.write_all(format!("{command}\nQUIT\n").as_bytes()).await.map_err(|err| format!("send: {err}"))?;
    let mut reply = String::new();
    soon("a reply", stream.read_to_string(&mut reply)).await.map_err(|err| format!("read: {err}"))?;
    if reply.is_empty() {
        Err("the connection was closed without a reply".into())
    } else {
        Ok(reply.trim().to_string())
    }
}

/// Asks for the version `count` times; every request must be answered.
async fn versions(port: u16, count: usize) -> Vec<String> {
    let mut seen = Vec::new();
    for _ in 0..count {
        let reply = ask(port, "VERSION").await.expect("no request may fail, whatever the rollout is doing");
        seen.push(reply.trim_start_matches("VERSION ").to_string());
    }
    seen
}

fn share(seen: &[String], version: &str) -> usize {
    seen.iter().filter(|seen| *seen == version).count()
}

#[tokio::test]
async fn the_first_apply_starts_the_cache_behind_a_stable_address() {
    let rig = Rig::new().await;
    let (applied, port) = rig.deploy_first("v1").await;

    assert_eq!(ask(port, "VERSION").await.unwrap(), "VERSION v1");
    assert_eq!(ask(port, "SET greeting hello there").await.unwrap(), "OK");
    assert_eq!(ask(port, "GET greeting").await.unwrap(), "VALUE hello there");
    assert_eq!(ask(port, "GET missing").await.unwrap(), "NIL");

    let promoted = rig.event("slot.promoted", |_| true).await;
    // A first version is watched through no stage: there is nothing to
    // compare it with.
    assert_eq!(
        promoted.data,
        json!({ "slot": "kv-cache", "service": "kv-cache-g1", "generation": 1, "stages": 0, "unwatched": 0 })
    );
    assert!(rig.events("slot.shifted").await.is_empty(), "nothing to shift from on a first deploy");

    // The tree: the example's instance, and under it the slot that holds
    // the router and the service.
    let root = rig.instance(&applied.instance).await;
    assert_eq!(root.status, InstanceStatus::Active);
    let slot = rig
        .engine
        .instances(conductor::InstanceFilter { parent: Some(root.id.clone()), ..Default::default() })
        .await
        .unwrap()
        .remove(0);
    assert_eq!(
        (slot.blueprint.as_str(), slot.key.as_str(), slot.status),
        ("std/slot", "slot", InstanceStatus::Active)
    );
    assert_eq!(slot.holds, ["router:kv-cache", "service:kv-cache-g1"]);
    assert_eq!(slot.state["stable"], "kv-cache-g1");

    let services = rig.hosting.services();
    assert_eq!((services[0].name.as_str(), services[0].status), ("kv-cache-g1", Status::Healthy));
}

#[tokio::test]
async fn a_new_version_is_rolled_out_in_stages_and_no_request_fails() {
    let rig = Rig::new().await;
    let (first, port) = rig.deploy_first("v1").await;

    let applied = rig.apply("v2", "").await;
    assert_eq!(applied.instance, first.instance, "applying again updates the instance that is there");

    for (weight, expected_v2) in [(10, 2), (50, 10), (100, 20)] {
        rig.watching(weight).await;
        let seen = versions(port, 20).await;
        assert_eq!(share(&seen, "v2"), expected_v2, "at {weight}%: {seen:?}");
        assert_eq!(share(&seen, "v1"), 20 - expected_v2);
        rig.clock.advance(Duration::from_secs(20));
    }

    let run = rig.finished(&applied).await;
    assert_eq!(run.status, RunStatus::Ok, "{:?}", run.error);
    assert_eq!(run.outputs["port"], port, "the address did not move");
    assert_eq!(share(&versions(port, 5).await, "v2"), 5);

    let promoted: Vec<Value> =
        rig.events("slot.promoted").await.into_iter().map(|event| event.data["service"].clone()).collect();
    assert_eq!(promoted, ["kv-cache-g1", "kv-cache-g2"]);
    rig.event("service.stopped", |event| event.data["service"] == "kv-cache-g1").await;
    let services = rig.hosting.services();
    assert_eq!(services.len(), 1, "the old version was retired: {services:?}");
    assert_eq!((services[0].name.as_str(), services[0].status), ("kv-cache-g2", Status::Healthy));

    // Each stage reported what it saw.
    let stages = rig
        .engine
        .instances(conductor::InstanceFilter {
            blueprint: Some("std/canary-stage".into()),
            ..Default::default()
        })
        .await
        .unwrap();
    let mut seen: Vec<(i64, i64, i64)> = Vec::new();
    for stage in &stages {
        let outputs = rig.engine.runs(&stage.id, 1).await.unwrap().remove(0).outputs;
        seen.push((
            outputs["weight"].as_i64().unwrap(),
            outputs["connections"].as_i64().unwrap(),
            outputs["failures"].as_i64().unwrap(),
        ));
    }
    seen.sort();
    assert_eq!(seen, [(10, 2, 0), (50, 10, 0), (100, 20, 0)]);
}

#[tokio::test]
async fn a_version_that_never_becomes_healthy_is_rejected_and_the_old_one_keeps_serving() {
    let rig = Rig::new().await;
    let (first, port) = rig.deploy_first("v1").await;

    let before = rig.clock.sleep_count();
    let applied = rig.apply("v2", "refuse").await;
    soon("the readiness deadline to start", rig.clock.sleeps_started(before + 1)).await;
    rig.clock.advance(Duration::from_secs(60));
    let run = rig.finished(&applied).await;

    assert_eq!(run.status, RunStatus::Failed);
    assert_eq!(run.error.as_deref(), Some("step `host` failed: step `ready` failed: timed out after 1m"));
    assert_eq!(share(&versions(port, 5).await, "v1"), 5, "v1 never stopped serving");

    let rejected = rig.event("slot.rejected", |_| true).await;
    assert_eq!(
        (rejected.data["candidate"].as_str(), rejected.data["reason"].as_str()),
        (Some("kv-cache-g2"), Some("timed out after 1m"))
    );
    let told = rig.event("kv.rollout_failed", |_| true).await;
    assert_eq!(told.data["version"], "v2");

    let root = rig.instance(&first.instance).await;
    assert_eq!(
        (root.status, root.params["version"].as_str()),
        (InstanceStatus::Active, Some("v1")),
        "back on the definition that works"
    );
    assert!(root.error.as_deref().unwrap().starts_with("update failed, still running the previous version"));
    let services = rig.hosting.services();
    assert_eq!(
        services.iter().map(|s| s.name.as_str()).collect::<Vec<_>>(),
        ["kv-cache-g1"],
        "the candidate was cleaned up"
    );
    assert!(rig.events("slot.shifted").await.is_empty(), "it was never sent any traffic");
}

#[tokio::test]
async fn a_version_that_fails_under_traffic_is_rolled_back_without_a_failed_request() {
    let rig = Rig::new().await;
    let (first, port) = rig.deploy_first("v1").await;

    // Healthy at first, then crashes once it has served a connection. It
    // serves that one and no other: it has stopped listening by the time it
    // answers, so the next sent to it is refused however long its process
    // takes to be gone.
    let applied = rig.apply("v2", "after:1").await;
    rig.watching(10).await;

    let seen = versions(port, 20).await;
    // On Windows the system goes on trying a connection that was refused
    // for about two seconds before it says so, and a program that ended
    // badly is started again after one: a connection sent to it while it
    // was gone reaches it once it is back (seen there: the sixteenth of
    // these was answered after a second and a half, by the new version
    // started again, which then ended again). So there it may answer
    // once for each time it was started; on every other system the
    // connection is refused at once and the router has covered for it
    // long before.
    let answered = share(&seen, "v2");
    assert!(
        if cfg!(windows) { (1..=2).contains(&answered) } else { answered == 1 },
        "it answered once, then died; the router covered for it: {seen:?}"
    );
    rig.event("service.exited", |event| event.data["service"] == "kv-cache-g2").await;
    rig.clock.advance(Duration::from_secs(20));
    let run = rig.finished(&applied).await;

    assert_eq!(run.status, RunStatus::Failed);
    let rolled_back = rig.event("slot.rolled_back", |_| true).await;
    assert_eq!(rolled_back.data["candidate"], "kv-cache-g2");
    assert!(
        rolled_back.data["reason"].as_str().unwrap().contains("did not pass observation"),
        "{}",
        rolled_back.data
    );

    assert_eq!(share(&versions(port, 10).await, "v1"), 10, "all traffic is back on v1");
    let root = rig.instance(&first.instance).await;
    assert_eq!((root.status, root.params["version"].as_str()), (InstanceStatus::Active, Some("v1")));
    rig.event("service.stopped", |event| event.data["service"] == "kv-cache-g2").await;
    assert_eq!(rig.hosting.services().iter().map(|s| s.name.as_str()).collect::<Vec<_>>(), ["kv-cache-g1"]);
    assert_eq!(rig.events("slot.shifted").await.len(), 1, "it stopped at the first stage");
}

#[tokio::test]
async fn the_cache_keeps_serving_through_a_conductor_restart_and_can_still_be_rolled_out_after() {
    let mut rig = Rig::new().await;
    let (first, port) = rig.deploy_first("v1").await;
    ask(port, "SET kept yes").await.unwrap();

    rig.restart().await;
    assert_eq!(ask(port, "GET kept").await.unwrap(), "VALUE yes", "same process, same memory, same address");
    // Said with what ended, if anything did: a cache started again is one
    // whose program ended, and how it ended is why.
    let ended: Vec<Value> = rig.events("service.exited").await.into_iter().map(|event| event.data).collect();
    assert_eq!(rig.events("service.started").await.len(), 1, "adopted, not started again; ended: {ended:?}");

    let applied = rig.apply("v2", "").await;
    assert_eq!(applied.instance, first.instance);
    for weight in [10, 50, 100] {
        rig.watching(weight).await;
        rig.clock.advance(Duration::from_secs(20));
    }
    assert_eq!(rig.finished(&applied).await.status, RunStatus::Ok);
    assert_eq!(share(&versions(port, 5).await, "v2"), 5);
}

// What follows is the log feed's, and Unix's with it.
#[cfg(unix)]
const LOGGED: &str = include_str!("../../../blueprints/logged-service.bp");

/// Waits, without sleeping, until the file at `path` contains `text`.
#[cfg(unix)]
async fn file_contains(path: &std::path::Path, text: &str) -> String {
    soon(&format!("{} to contain `{text}`", path.display()), async {
        loop {
            let content = std::fs::read_to_string(path).unwrap_or_default();
            if content.contains(text) {
                return content;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
}

/// Unix's alone: `std/log-feed` is the one blueprint that ships which
/// has a line of shell of its own (it follows a file with `tail`, for
/// which no program is on every system: `xtask/src/portable.rs` lists
/// it), so on Windows the feed is never started.
#[cfg(unix)]
#[tokio::test]
async fn a_log_feed_is_a_component_of_its_own_that_changes_without_touching_the_service() {
    let rig = Rig::new().await;
    let user = Principal {
        kind: PrincipalKind::User,
        id: "ada".into(),
        name: "Ada".into(),
        scopes: vec!["*".into()],
        key: None,
    };
    let apply = |to: &str| {
        let params = json!({ "binary": BINARY, "to": to });
        let request = Apply {
            source: Some(LOGGED.to_string()),
            params: kernel::fields(params),
            ..Apply::new("logged-service", user.clone())
        };
        rig.engine.apply(request)
    };

    let applied = apply("").await.unwrap();
    let run = rig.finished(&applied).await;
    assert_eq!(run.status, RunStatus::Ok, "{:?}", run.error);

    // The service's output arrives in the feed's file, in the project.
    let shipped = file_contains(&rig.dir.join("cache-g1.feed.log"), "listening on").await;
    assert!(shipped.contains("kv-cache v1 listening on 127.0.0.1:"), "{shipped}");
    rig.event("service.healthy", |event| event.data["service"] == "cache-g1-feed-g1").await;
    // And the handler that built the feed has finished: changing the
    // attachment under a handler still at work stops what it was building,
    // which is a different story from the one told here.
    rig.event("run.finished", |event| {
        event.data["blueprint"] == "std/log-feed" && event.data["flow"] == "on.started"
    })
    .await;
    let cache = rig.hosting.services().into_iter().find(|service| service.name == "cache-g1").unwrap();

    // Ship the logs somewhere else. Only the feed is replaced.
    let elsewhere = rig.dir.join("elsewhere.log");
    let updated = apply(&format!("cat >> '{}'", elsewhere.display())).await.unwrap();
    assert_eq!(updated.instance, applied.instance);
    rig.finished(&updated).await;
    // The attachment was re-applied with the new destination; its handler
    // builds the new feed the next time it hears of the service, so nudge it
    // the way a restart of the service would.
    rig.journal
        .emit("service.started", kernel::Source::instance(&cache.instance), json!({ "service": "cache-g1" }))
        .await
        .unwrap();
    file_contains(&elsewhere, "listening on").await;
    // The new feed writes before the old one is retired: the slot says it
    // has taken over only once the one before it is gone.
    rig.event("slot.promoted", |event| event.data["service"] == "cache-g1-feed-g2").await;

    let after = rig.hosting.services().into_iter().find(|service| service.name == "cache-g1").unwrap();
    assert_eq!(
        (after.pid, after.restarts),
        (cache.pid, 0),
        "the service itself was never touched; it last ended with {:?}",
        after.last_exit
    );
    let names: Vec<String> = rig.hosting.services().into_iter().map(|service| service.name).collect();
    assert_eq!(names, ["cache-g1", "cache-g1-feed-g2"], "one feed, the new one: {names:?}");
}

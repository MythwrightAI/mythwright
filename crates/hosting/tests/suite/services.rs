use conductor::{InstanceStatus, RunStatus};
use hosting::Status;
use serde_json::json;

use crate::standin::cmd;
use crate::{Rig, alive, gone, line, pids, soon, step};

/// A service that records its pid, says it is up, and idles.
fn idle() -> String {
    let serves = cmd(&["write", "${params.name}.pid", "%:pid%", "say", "up on %PORT%", "idle"]);
    format!(
        "\
blueprint: idle
permissions: [fleet]
params: {{ name: idle }}
steps:
  start:
    do: service.start
    with:
      name: ${{params.name}}
      cmd: {serves}
"
    )
}

/// A service that does nothing, for good.
fn idles() -> String {
    cmd(&["idle"])
}

#[tokio::test]
async fn a_service_runs_until_the_instance_that_owns_it_is_stopped() {
    let rig = Rig::new().await;
    let mut lines = rig.lines();
    let applied = rig.apply(&idle(), json!({})).await;
    let run = rig.finished(&applied).await;
    let started = &step(&run, "start").output;
    let port = started["port"].as_u64().unwrap();
    assert!(port > 0 && started["address"] == format!("127.0.0.1:{port}"));
    line(&mut lines, &format!("up on {port}")).await;

    rig.event("service.healthy", |event| event.data["service"] == "idle").await;
    let instance = rig.engine.instance(&applied.instance).await.unwrap();
    assert_eq!(
        (instance.status, instance.holds.as_slice()),
        (InstanceStatus::Active, &["service:idle".to_string()][..])
    );
    let info = &rig.hosting.services()[0];
    assert_eq!(
        (info.name.as_str(), info.status, info.instance.as_str()),
        ("idle", Status::Healthy, applied.instance.as_str())
    );
    let pid = pids(&rig.path("idle.pid"));
    assert!(alive(pid[0]));

    rig.engine.stop(&applied.instance).await.unwrap();
    rig.event("service.stopped", |_| true).await;
    gone(&pid).await;
    assert!(rig.hosting.services().is_empty());
    assert_eq!(
        rig.events("service.*").await.iter().map(|e| e.topic.as_str()).collect::<Vec<_>>(),
        ["service.started", "service.healthy", "service.stopped"]
    );
}

#[tokio::test]
async fn health_is_checked_and_changes_are_announced() {
    let rig = Rig::new().await;
    // The check is a line for the system's shell, as a user writes one:
    // here the stand-in's file with what it is to look for, which either
    // shell reads the same way.
    let (serves, healthy) = (idles(), crate::standin::word(&crate::standin::line(&["has", "healthy"])));
    let applied = rig
        .apply(
            &format!(
                "\
blueprint: checked
permissions: [fleet]
steps:
  start:
    do: service.start
    with:
      name: web
      cmd: {serves}
      health: {{ cmd: {healthy}, interval: 100ms, unhealthy_after: 2 }}
  ready: {{ do: service.wait_healthy, with: {{ service: web, timeout: 60s }} }}
  report: {{ do: service.status, with: {{ service: web }} }}
"
            ),
            json!({}),
        )
        .await;
    rig.event("service.started", |_| true).await;
    assert_eq!(rig.hosting.services()[0].status, Status::Starting, "not healthy until a check passes");

    std::fs::write(rig.path("healthy"), "").unwrap();
    let run = rig.finished(&applied).await;
    assert_eq!(step(&run, "ready").output["healthy"], true);
    assert_eq!(step(&run, "report").output["status"], "healthy");

    std::fs::remove_file(rig.path("healthy")).unwrap();
    let unhealthy = rig.event("service.unhealthy", |_| true).await;
    assert_eq!(unhealthy.data["service"], "web");
    assert!(
        unhealthy.data["reason"].as_str().unwrap().starts_with("the check exited with"),
        "{}",
        unhealthy.data
    );

    std::fs::write(rig.path("healthy"), "").unwrap();
    rig.event("service.healthy", |event| event.seq > unhealthy.seq).await;

    let checks = rig
        .journal
        .query_metrics(journal::MetricQuery { name: "service.health".into(), ..Default::default() })
        .await
        .unwrap();
    assert!(
        checks[0].total.count >= 3 && checks[0].total.errors >= 2,
        "every check is measured: {:?}",
        checks[0].total
    );
    rig.engine.stop(&applied.instance).await.unwrap();
}

#[tokio::test]
async fn waiting_on_a_service_that_dies_says_how_it_died() {
    let rig = Rig::new().await;
    let dies = cmd(&["err", "cannot bind", "exit", "7"]);
    let run = rig
        .run(
            &format!(
                "\
blueprint: stillborn
permissions: [fleet]
steps:
  start: {{ do: service.start, with: {{ name: dud, cmd: {dies}, restart: never, health: {{ tcp: true }} }} }}
  ready: {{ do: service.wait_healthy, with: {{ service: dud }} }}
"
            ),
            json!({}),
        )
        .await;
    assert_eq!(run.status, RunStatus::Failed);
    assert_eq!(step(&run, "ready").error.as_deref(), Some("`dud` is exited: it exited with code 7"));
    let logged = rig.engine.logs().tail("services", "dud", 5);
    assert_eq!(logged[0].text, "cannot bind", "its output is in the service's log");
}

#[tokio::test]
async fn a_crashing_service_is_restarted_within_a_budget_and_then_given_up_on() {
    let rig = Rig::new().await;
    let crashes = cmd(&["append", "lives", "life", "exit", "1"]);
    let applied = rig
        .apply(
            &format!(
                "\
blueprint: crashy
permissions: [fleet]
steps:
  start:
    do: service.start
    with: {{ name: crashy, cmd: {crashes}, max_restarts: 2, restart_backoff: 10ms }}
"
            ),
            json!({}),
        )
        .await;
    let failed = rig.event("service.failed", |_| true).await;
    assert_eq!(failed.data["reason"], "restarted 2 times within an hour; giving up");
    assert_eq!(
        std::fs::read_to_string(rig.path("lives")).unwrap().lines().count(),
        3,
        "the first life and two restarts"
    );
    let attempts: Vec<u64> =
        rig.events("service.restarting").await.iter().map(|e| e.data["attempt"].as_u64().unwrap()).collect();
    assert_eq!(attempts, [1, 2]);
    assert_eq!(rig.hosting.services()[0].status, Status::Failed);
    rig.engine.stop(&applied.instance).await.unwrap();
}

#[tokio::test]
async fn a_service_whose_process_ended_is_not_healthy_while_it_waits_to_be_started_again() {
    let rig = Rig::new().await;
    // Healthy as it starts, since nothing checks it; then it ends, and is
    // to be started again only after a long wait.
    let ends = cmd(&["exit", "1"]);
    let applied = rig
        .apply(
            &format!(
                "\
blueprint: brief
permissions: [fleet]
steps:
  start:
    do: service.start
    with: {{ name: brief, cmd: {ends}, restart_backoff: 10m }}
"
            ),
            json!({}),
        )
        .await;
    rig.event("service.healthy", |event| event.data["service"] == "brief").await;
    rig.event("service.restarting", |event| event.data["service"] == "brief").await;
    let info = &rig.hosting.services()[0];
    assert_eq!((info.status, info.last_exit), (Status::Starting, Some(1)), "{info:?}");
    assert!(info.status.is_up(), "it is still to run");
    rig.engine.stop(&applied.instance).await.unwrap();
    rig.event("service.stopped", |event| event.data["service"] == "brief").await;
}

#[tokio::test]
async fn what_a_projects_service_says_of_itself_is_kept_with_what_else_is_said_of_the_project() {
    let rig = Rig::new().await;
    // The project has a journal of its own, which keeps what is run for
    // it and what is said of it.
    rig.journal.keeps_projects_in_memory().await.unwrap();
    let shop = rig.journal.own("shop").await.unwrap();
    // Its check is a line either system's shell reads, and ends well.
    let source = format!(
        "\
blueprint: owned
permissions: [fleet]
steps:
  start:
    do: service.start
    with:
      name: web
      cmd: {}
      health: {{ cmd: \"exit 0\", interval: 100ms }}
",
        idles()
    );
    let request = conductor::Apply { project: Some("shop".into()), ..rig.request(&source, json!({})) };
    let applied = rig.engine.apply(request).await.unwrap();
    let said = async |topic: &'static str| {
        let mut follow = shop.follow(0, topic);
        soon(&format!("`{topic}` in the project's journal"), follow.next()).await.unwrap().seq
    };
    // What the service says names no project: the instance that owns it
    // says it. It is in the project's journal all the same, after the
    // instance was said to start and before it is said to be over, and
    // none of it is in the factory's.
    let (began, started, healthy) =
        (said("instance.started").await, said("service.started").await, said("service.healthy").await);
    assert!(began < started && started < healthy, "{began} {started} {healthy}");
    rig.engine.stop(&applied.instance).await.unwrap();
    let (stopped, over) = (said("service.stopped").await, said("instance.finished").await);
    assert!(healthy < stopped && stopped < over, "{healthy} {stopped} {over}");
    // (Played with every project kept in the factory's one journal,
    // that is the journal all of it is in, and this is all that is held.)
    if journal::apart() == Some(false) {
        return;
    }
    let of_factory = rig.events("").await;
    let services: Vec<&str> = of_factory
        .iter()
        .map(|event| event.topic.as_str())
        .filter(|topic| topic.starts_with("service."))
        .collect();
    assert!(services.is_empty(), "{services:?}");
}

#[tokio::test]
async fn a_service_started_for_a_project_is_that_projects_and_still_is_after_a_restart() {
    let mut rig = Rig::new().await;
    // Its check is a line either system's shell reads, and ends well.
    let source = format!(
        "\
blueprint: owned
permissions: [fleet]
steps:
  start:
    do: service.start
    with:
      name: web
      cmd: {}
      health: {{ cmd: \"exit 0\", interval: 100ms }}
",
        idles()
    );
    let request = conductor::Apply { project: Some("shop".into()), ..rig.request(&source, json!({})) };
    let applied = rig.engine.apply(request).await.unwrap();
    rig.event("service.healthy", |_| true).await;
    assert_eq!(rig.hosting.services()[0].project.as_deref(), Some("shop"));

    let checks = async |project: &str| {
        let labels = vec![("project".to_string(), project.to_string())];
        let query = journal::MetricQuery { name: "service.health".into(), labels, ..Default::default() };
        rig.journal.query_metrics(query).await.unwrap().first().map_or(0, |series| series.total.count)
    };
    assert!(checks("shop").await >= 1, "its checks are measured as the project's");
    assert_eq!(checks("other").await, 0);

    rig.restart().await;
    assert_eq!(rig.hosting.services()[0].project.as_deref(), Some("shop"), "it is remembered");
    rig.engine.stop(&applied.instance).await.unwrap();
    rig.event("service.stopped", |_| true).await;
}

#[tokio::test]
async fn a_service_survives_a_restart_of_the_conductor_and_is_adopted_not_started_twice() {
    let mut rig = Rig::new().await;
    let mut lines = rig.lines();
    let applied = rig.apply(&idle(), json!({ "name": "steady" })).await;
    rig.finished(&applied).await;
    rig.event("service.healthy", |_| true).await;
    line(&mut lines, "up on").await;
    let before = rig.hosting.services().remove(0);

    rig.restart().await;
    let after = rig.hosting.services().remove(0);
    assert_eq!((after.pid, after.port, after.status), (before.pid, before.port, Status::Healthy));
    assert!(alive(after.pid));
    assert_eq!(rig.events("service.started").await.len(), 1, "it was not started again");

    // The adopted service is still under control.
    rig.engine.stop(&applied.instance).await.unwrap();
    rig.event("service.stopped", |_| true).await;
    gone(&pids(&rig.path("steady.pid"))).await;
}

#[tokio::test]
async fn a_service_that_died_while_the_conductor_was_down_is_reported_not_assumed() {
    let mut rig = Rig::new().await;
    let mut lines = rig.lines();
    let applied = rig.apply(&idle(), json!({ "name": "fragile" })).await;
    rig.finished(&applied).await;
    rig.event("service.healthy", |_| true).await;
    // The pid file is written just before this line is printed.
    line(&mut lines, "up on").await;
    let pid = pids(&rig.path("fragile.pid"));

    rig.engine.halt();
    rig.hosting.halt();
    // Its whole group is ended without being asked, as the factory ends one.
    hosting::group::stop(pid[0], true).unwrap();
    gone(&pid).await;
    rig.restart().await;

    let exited = rig.event("service.exited", |_| true).await;
    assert_eq!(exited.data["while"], "the conductor was down");
    assert_eq!(rig.hosting.services()[0].status, Status::Exited);
}

#[tokio::test]
async fn starting_the_same_service_again_is_a_no_op_and_a_different_one_is_refused() {
    let rig = Rig::new().await;
    let first = rig.apply(&idle(), json!({})).await;
    let port = step(&rig.finished(&first).await, "start").output["port"].clone();

    let again =
        rig.engine.apply(conductor::Apply { force: true, ..rig.request(&idle(), json!({})) }).await.unwrap();
    assert_eq!(
        step(&rig.finished(&again).await, "start").output["port"],
        port,
        "the same service, untouched"
    );
    assert_eq!(rig.events("service.started").await.len(), 1);

    let other = format!(
        "blueprint: other\npermissions: [fleet]\nsteps:\n  start: {{ do: service.start, with: {{ name: idle, cmd: {} }} }}\n",
        idles()
    );
    let run = rig.run(&other, json!({})).await;
    assert_eq!(
        step(&run, "start").error.as_deref(),
        Some("a different service called `idle` is already running; stop it first or use another name")
    );
    rig.engine.stop(&first.instance).await.unwrap();
}

/// A service that says things of its own in what it prints, and idles.
fn talker() -> String {
    let says = [
        "starting up",
        r#"::measure {"name": "checkout.done", "value": 12.5}"#,
        r#"::measure {"name": "checkout.done", "value": 20, "ok": false, "labels": {"page": "cart"}}"#,
        r#"::event {"topic": "shop.paid", "order": "o-17", "total": 40}"#,
        r#"::measure {"name": "step", "value": 1}"#,
        r#"::event {"topic": "instance.failed", "error": "not mine to say"}"#,
        "::measure not json at all",
        "all said",
    ];
    let words: Vec<&str> = says.iter().flat_map(|line| ["say", line]).chain(["idle"]).collect();
    format!(
        "
blueprint: talker
permissions: [fleet]
steps:
  host:
    labels: {{ variant: b }}
    task:
      start:
        do: service.start
        with:
          name: checkout
          cmd: {}
",
        cmd(&words)
    )
}

#[tokio::test]
async fn a_service_says_measurements_and_events_of_its_own_in_what_it_prints() {
    let rig = Rig::new().await;
    let mut lines = rig.lines();
    let applied = rig.apply(&talker(), json!({})).await;
    rig.finished(&applied).await;
    line(&mut lines, "all said").await;

    // An event of its own, said for the instance that owns it.
    let paid = rig.event("shop.paid", |_| true).await;
    assert_eq!(paid.data, json!({ "order": "o-17", "total": 40 }));
    assert_eq!(paid.source.instance.as_deref(), Some(applied.instance.as_str()));

    // Its measurements, by the labels the step that started it gave, by
    // what it said of each, and by which service it is.
    rig.journal.flush_metrics().await.unwrap();
    let by = |group: &[&str]| journal::MetricQuery {
        name: "checkout.done".into(),
        group_by: group.iter().map(|name| name.to_string()).collect(),
        ..Default::default()
    };
    let all = rig.journal.query_metrics(by(&[])).await.unwrap();
    assert_eq!(
        (all[0].total.count, all[0].total.errors, all[0].total.max),
        (2, 1, 20.0),
        "{:?}",
        all[0].total
    );
    let variants = rig.journal.query_metrics(by(&["variant", "service"])).await.unwrap();
    assert_eq!(variants.len(), 1, "{variants:?}");
    assert_eq!(
        (
            variants[0].labels.get("variant").map(String::as_str),
            variants[0].labels.get("service").map(String::as_str)
        ),
        (Some("b"), Some("checkout"))
    );
    let pages = rig.journal.query_metrics(by(&["page"])).await.unwrap();
    assert!(
        pages.iter().any(|series| series.labels.get("page").map(String::as_str) == Some("cart")),
        "{pages:?}"
    );

    // What is the system's to say or to measure is not taken from it, and
    // it is told so where its output is read.
    assert!(rig.events("instance.failed").await.is_empty());
    let system =
        rig.journal.query_metrics(journal::MetricQuery { name: "step".into(), ..Default::default() });
    let steps = system.await.unwrap();
    assert!(steps.iter().all(|series| !series.labels.contains_key("service")), "{steps:?}");
    let log = std::fs::read_to_string(&rig.hosting.services()[0].log).unwrap_or_default();
    for told in [
        "not taken: `step` is a name the system measures under",
        "not taken: `instance.failed` is in a namespace only the system may publish under",
        "not taken: what follows `::measure` is a mapping written as JSON",
    ] {
        assert!(log.contains(told), "{told}\n{log}");
    }

    rig.engine.stop(&applied.instance).await.unwrap();
    rig.event("service.stopped", |_| true).await;
}

#[tokio::test]
async fn a_program_that_takes_its_port_and_its_name_as_arguments_is_told_them_there() {
    let rig = Rig::new().await;
    // No shell reads a variable for a program started from a list of
    // arguments; one that wants its port after `--port=` is given it there.
    let serves = cmd(&[
        "write",
        "told",
        "{name}",
        "append",
        "told",
        "--port={port}",
        "append",
        "told",
        "{port} again",
        "idle",
    ]);
    let applied = rig
        .apply(
            &format!(
                "\
blueprint: argued
permissions: [fleet]
steps:
  start:
    do: service.start
    with:
      name: web-g2
      cmd: {serves}
"
            ),
            json!({}),
        )
        .await;
    rig.finished(&applied).await;
    let port = rig.hosting.services()[0].port;
    assert!(port > 0);
    // Each of its arguments, a line each: the whole of one, the end of
    // one and the beginning of one.
    let told = crate::soon("the program to say what it was told", async {
        loop {
            match tokio::fs::read_to_string(rig.path("told")).await {
                Ok(told) if told.lines().count() == 3 && told.ends_with('\n') => return told,
                _ => tokio::task::yield_now().await,
            }
        }
    })
    .await;
    assert_eq!(told, format!("web-g2\n--port={port}\n{port} again\n"));
    // What is kept of the service is what was written, so that started
    // again on another port it is told that one.
    let kept = rig.hosting.services().remove(0);
    assert_eq!(kept.name, "web-g2");
}

#[tokio::test]
async fn a_service_is_stopped_when_its_program_has_ended_and_not_when_it_has_only_been_asked() {
    let rig = Rig::new().await;
    // A program that takes a moment to put its things away when it is
    // asked to stop, as one that is finishing what it has in hand does.
    // Being asked is a signal, which only Unix has, and what hears one
    // and goes on is a line of its shell. Where a stop is an ending and
    // nothing is asked (Windows), the program is one that only goes on,
    // and nothing is put away.
    let tidies = if cfg!(unix) {
        "[sh, -c, 'trap \"sleep 0.3; echo away > put-away; exit 0\" TERM; echo $$ > up; i=0; while [ $i -lt 12000 ]; do i=$((i+1)); sleep 0.05 & wait $!; done']".to_string()
    } else {
        cmd(&["write", "up", "%:pid%", "idle"])
    };
    let applied = rig
        .apply(
            &format!(
                "\
blueprint: tidy
permissions: [fleet]
steps:
  start:
    do: service.start
    with:
      name: tidy
      restart: never
      cmd: {tidies}
"
            ),
            json!({}),
        )
        .await;
    rig.finished(&applied).await;
    let up = crate::soon("the program to be up", async {
        loop {
            match pids(&rig.path("up")).first() {
                Some(pid) => return *pid,
                None => tokio::task::yield_now().await,
            }
        }
    })
    .await;

    rig.engine.stop(&applied.instance).await.unwrap();
    rig.event("service.stopped", |_| true).await;
    // By the time it is said to have stopped, it has: what it does on its
    // way out is done, and nothing of it is still running.
    #[cfg(unix)]
    assert!(rig.path("put-away").exists(), "it was given the time to put its things away");
    assert!(!alive(up), "it was said to have stopped while it was still running");
}

#[tokio::test]
async fn what_a_factory_left_running_is_ended_for_its_test_and_another_factorys_is_left_alone() {
    let (rig, other) = (Rig::new().await, Rig::new().await);
    for (rig, name) in [(&rig, "left"), (&other, "others")] {
        let mut lines = rig.lines();
        let applied = rig.apply(&idle(), json!({ "name": name })).await;
        rig.finished(&applied).await;
        rig.event("service.healthy", |event| event.data["service"] == name).await;
        // The pid file is written just before this line is printed, and a
        // service with no check is healthy as soon as it is started.
        line(&mut lines, "up on").await;
    }
    let (left, others) = (pids(&rig.path("left.pid")), pids(&other.path("others.pid")));
    // The factory is stopped, and leaves its service running, on purpose:
    // nothing watches the service from here on, so nothing starts it again.
    rig.engine.halt();
    rig.hosting.halt();
    assert!(alive(left[0]));

    // What a test's rig asks as the test is over: every program a factory
    // in this folder started that is still there is ended, and is over by
    // the time that is answered.
    assert_eq!(hosting::end_under(&rig.dir), left);
    assert!(!alive(left[0]));
    // Asked again there is nothing to end, and another factory's service
    // is as it was.
    assert_eq!(hosting::end_under(&rig.dir), Vec::<u32>::new());
    assert!(alive(others[0]));
}

/// What is said of the words a health check takes (`hosting::HEALTH`,
/// which `service.start` says of its `health`) is held to the reading of
/// one: a word that is read and not said, or said and not read, fails.
#[test]
fn what_is_said_of_a_health_check_is_every_word_one_is_read_by_and_no_other() {
    let table = hosting::HEALTH;
    let owned: Vec<(String, String, String)> = table
        .iter()
        .map(|(path, kind, says)| (path.to_string(), kind.to_string(), says.to_string()))
        .collect();
    assert_eq!(blueprint::Within::of(table).rows(), owned, "every row is read, in the order it is written");
    assert!(table.iter().all(|(_, _, says)| !says.is_empty()), "each says a sentence");
    // The reading, in the source: each word it asks the mapping for.
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/src/service.rs");
    let source = std::fs::read_to_string(path).unwrap();
    let begins =
        source.find("let health = match args.get(\"health\")").expect("where a health check is read");
    let stretch = &source[begins..begins + source[begins..].find("let restart").expect("where it ends")];
    let mut read = std::collections::BTreeSet::new();
    for ask in ["health.get(", "millis(health, "] {
        for (at, _) in stretch.match_indices(ask) {
            let word = stretch[at + ask.len()..].strip_prefix('"').and_then(|rest| rest.split('"').next());
            read.extend(word.map(str::to_string));
        }
    }
    let said: std::collections::BTreeSet<String> = table.iter().map(|(path, ..)| path.to_string()).collect();
    assert_eq!(said, read, "the keys `hosting::HEALTH` says, and those a health check is read by");
}

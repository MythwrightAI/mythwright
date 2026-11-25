use std::sync::Arc;

use blueprint::{ActionSpec, ValueType};
use conductor::{ActionError, Apply, InstanceStatus, Registry, RunStatus};
use kernel::Source;
use serde_json::json;

use crate::{Rig, Shared, action};

/// What the tests here ask that no other does: `test.reads`, which gives
/// what the test last noted under its `key` once the test opens the gate
/// of that name, as a source a poller asks answers when it is asked.
pub fn stand_ins(registry: &mut Registry, shared: &Arc<Shared>) {
    action(
        registry,
        ActionSpec::new(
            "test.reads",
            "Give what the test last noted under `key`, once it opens the gate of that name.",
        )
        .required("key", ValueType::String, "what is read, and the gate it waits at")
        .idempotent(),
        shared,
        |cx, args, shared| async move {
            let key = args["key"].as_str().unwrap();
            let gate = shared.gate(key);
            cx.emit("test.reached", json!({ "gate": key })).await?;
            tokio::select! {
                permit = gate.acquire() => permit.unwrap().forget(),
                () = cx.cancel().cancelled() => return Err(ActionError::cancelled()),
            }
            Ok(json!({ "said": shared.noted(key).last() }))
        },
    );
}

const WATCHER: &str = "\
blueprint: board-watcher
state: { seen: 0 }
on:
  ready:
    event: board.item.moved
    when: ${ event.data.to == \"ready\" }
    steps:
      note: { do: set, with: { seen: ${ state.seen + 1 } } }
      tell: { do: emit, with: { topic: lane.wanted, item: ${ event.data.id } } }
";

pub(crate) async fn active(rig: &Rig, instance: &str) {
    rig.event("instance.active", |event| event.source.instance.as_deref() == Some(instance)).await;
}

async fn moved(rig: &Rig, id: u64, to: &str) {
    rig.journal.emit("board.item.moved", Source::default(), json!({ "id": id, "to": to })).await.unwrap();
}

/// Waits until `count` runs of `flow` have finished.
pub(crate) async fn handled(rig: &Rig, flow: &str, count: usize) {
    let mut follow = rig.follow("run.finished");
    let mut seen = 0;
    rig.soon(&format!("{count} runs of {flow}"), async {
        while seen < count {
            if follow.next().await.unwrap().data["flow"] == flow {
                seen += 1;
            }
        }
    })
    .await;
}

#[tokio::test]
async fn a_handler_runs_for_each_event_that_matches() {
    let rig = Rig::new().await;
    let applied = rig.apply(WATCHER, json!({})).await;
    active(&rig, &applied.instance).await;
    assert_eq!(
        rig.instance(&applied.instance).await.status,
        InstanceStatus::Active,
        "handlers keep it alive"
    );

    moved(&rig, 7, "doing").await;
    moved(&rig, 8, "ready").await;
    moved(&rig, 9, "ready").await;
    handled(&rig, "on.ready", 2).await;

    let wanted: Vec<u64> =
        rig.events("lane.wanted").await.iter().map(|event| event.data["item"].as_u64().unwrap()).collect();
    assert_eq!(wanted, [8, 9]);
    assert_eq!(rig.instance(&applied.instance).await.state["seen"], 2);

    let runs = rig.engine.runs(&applied.instance, 10).await.unwrap();
    let flows: Vec<&str> = runs.iter().rev().map(|run| run.flow.as_str()).collect();
    assert_eq!(flows, ["main", "on.ready", "on.ready"]);
    assert_eq!(
        runs[0].event["data"],
        json!({ "id": 9, "to": "ready" }),
        "a handler's run records what fired it"
    );
}

#[tokio::test]
async fn runs_of_one_handler_never_overlap() {
    let rig = Rig::new().await;
    let applied = rig
        .apply(
            "blueprint: serial\non:\n  job:\n    event: job.wanted\n    do: test.gate\n    with: { name: \"job-${event.data.n}\" }\n",
            json!({}),
        )
        .await;
    active(&rig, &applied.instance).await;
    for n in [1, 2] {
        rig.journal.emit("job.wanted", Source::default(), json!({ "n": n })).await.unwrap();
    }
    rig.reached("job-1").await;
    assert_eq!(rig.shared.calls(), ["run:test.gate"], "the second waits for the first");
    rig.open("job-2");
    rig.open("job-1");
    handled(&rig, "on.job", 2).await;
    assert_eq!(rig.shared.calls().len(), 2);
}

#[tokio::test]
async fn a_handler_does_not_react_to_its_own_events() {
    let rig = Rig::new().await;
    let applied = rig
        .apply("blueprint: parrot\non:\n  echo:\n    event: \"chatter.*\"\n    do: emit\n    with: { topic: chatter.reply }\n", json!({}))
        .await;
    active(&rig, &applied.instance).await;
    rig.journal.emit("chatter.hello", Source::default(), json!({})).await.unwrap();
    handled(&rig, "on.echo", 1).await;
    // By the time a second outside event is handled, the first reply has
    // been seen and passed over.
    rig.journal.emit("chatter.again", Source::default(), json!({})).await.unwrap();
    handled(&rig, "on.echo", 2).await;
    assert_eq!(
        rig.engine.runs(&applied.instance, 10).await.unwrap().len(),
        3,
        "main, and one run per outside event"
    );
    assert_eq!(rig.events("chatter.reply").await.len(), 2);
}

#[tokio::test]
async fn scope_own_limits_a_handler_to_its_own_tree() {
    let rig = Rig::new().await;
    let parent = rig
        .apply(
            "blueprint: owner\non:\n  mine:\n    event: thing.happened\n    scope: own\n    do: test.count\n    with: { key: mine }\n",
            json!({}),
        )
        .await;
    active(&rig, &parent.instance).await;
    let shout = "blueprint: shouter\nsteps:\n  say: { do: emit, with: { topic: thing.happened } }\n";

    // An unrelated instance.
    rig.run(shout, json!({})).await;
    // A child of the owner.
    let child = Apply { parent: Some(parent.instance.clone()), ..rig.request(shout, json!({})) };
    let child = rig.engine.apply(child).await.unwrap();
    rig.finished(&child).await;
    handled(&rig, "on.mine", 1).await;
    assert_eq!(rig.shared.count("mine"), 1);
}

#[tokio::test]
async fn events_that_happen_while_the_conductor_is_down_are_handled_once_when_it_returns() {
    let mut rig = Rig::new().await;
    let applied = rig.apply(WATCHER, json!({})).await;
    active(&rig, &applied.instance).await;

    rig.engine.halt();
    moved(&rig, 1, "ready").await;
    moved(&rig, 2, "ready").await;
    rig.restart().await;
    handled(&rig, "on.ready", 2).await;

    rig.restart().await;
    moved(&rig, 3, "ready").await;
    handled(&rig, "on.ready", 3).await;
    let wanted: Vec<u64> =
        rig.events("lane.wanted").await.iter().map(|event| event.data["item"].as_u64().unwrap()).collect();
    assert_eq!(wanted, [1, 2, 3], "each exactly once, in order");
    assert_eq!(rig.instance(&applied.instance).await.state["seen"], 3);
}

#[tokio::test]
async fn a_handler_run_in_progress_is_finished_after_a_restart_not_started_again() {
    let mut rig = Rig::new().await;
    let applied = rig
        .apply("blueprint: careful\non:\n  job:\n    event: job.wanted\n    do: test.durable\n    with: { name: job }\n", json!({}))
        .await;
    active(&rig, &applied.instance).await;
    rig.journal.emit("job.wanted", Source::default(), json!({})).await.unwrap();
    rig.reached("job").await;

    rig.restart().await;
    rig.open("job");
    handled(&rig, "on.job", 1).await;

    let runs = rig.engine.runs(&applied.instance, 10).await.unwrap();
    assert_eq!(runs.len(), 2, "main and one handler run");
    let run = rig.engine.run(&runs[0].id).await.unwrap();
    assert_eq!(run.status, RunStatus::Ok);
    assert_eq!(run.steps[0].output, json!({ "resumed_from": { "pid": 4242 } }));
    assert_eq!((rig.shared.count("durable:run"), rig.shared.count("durable:resume")), (1, 1));
}

#[tokio::test]
async fn an_every_handler_runs_on_a_timer() {
    let rig = Rig::new().await;
    let since = rig.clock.sleep_count();
    let applied =
        rig.apply("blueprint: heartbeat\non:\n  beat:\n    every: 30s\n    do: test.count\n    with: { key: beats }\n", json!({})).await;
    active(&rig, &applied.instance).await;
    rig.advance_after(since, 1, "30s").await;
    handled(&rig, "on.beat", 1).await;
    rig.advance_after(since, 2, "30s").await;
    handled(&rig, "on.beat", 2).await;
    assert_eq!(rig.shared.count("beats"), 2);
}

#[tokio::test]
async fn a_poller_fires_only_when_what_it_reads_changes() {
    let rig = Rig::new().await;
    let since = rig.clock.sleep_count();
    // `test.count` returns a new number every time it is asked.
    let applied = rig
        .apply(
            "\
blueprint: upstream
on:
  moved:
    poll: { every: 30s, do: test.count, with: { key: polls } }
    do: test.echo
    with: { now: ${ event.data.count }, before: ${ event.previous.count } }
",
            json!({}),
        )
        .await;
    active(&rig, &applied.instance).await;
    // The first reading is taken as it begins to listen; a poller adds up
    // to a tenth of its interval as jitter to the next.
    rig.advance_after(since, 1, "33s").await;
    handled(&rig, "on.moved", 1).await;

    assert_eq!(rig.shared.count("polls"), 2, "the first reading is only a baseline");
    let run = &rig.engine.runs(&applied.instance, 1).await.unwrap()[0];
    assert_eq!(rig.engine.run(&run.id).await.unwrap().steps[0].output, json!({ "now": 2, "before": 1 }));

    let polls = rig
        .journal
        .query_metrics(journal::MetricQuery { name: "poll".into(), ..Default::default() })
        .await
        .unwrap();
    assert_eq!(polls[0].total.count, 2, "every poll is measured");
}

#[tokio::test]
async fn a_poller_that_sees_no_change_stays_quiet() {
    let rig = Rig::new().await;
    let since = rig.clock.sleep_count();
    let applied = rig
        .apply(
            "blueprint: still\non:\n  moved:\n    poll: { every: 30s, do: test.echo, with: { v: 1 } }\n    do: test.count\n    with: { key: fired }\n",
            json!({}),
        )
        .await;
    active(&rig, &applied.instance).await;
    for poll in 1..=3 {
        rig.advance_after(since, poll, "33s").await;
    }
    rig.soon("the third poll to finish", rig.clock.sleeps_started(since + 4)).await;
    assert_eq!(rig.shared.count("fired"), 0);
}

#[tokio::test]
async fn debounce_collapses_a_burst_into_one_run_with_the_latest_event() {
    let rig = Rig::new().await;
    let since = rig.clock.sleep_count();
    let applied = rig
        .apply(
            "blueprint: rebuild\non:\n  changed:\n    event: file.changed\n    debounce: 2s\n    do: test.echo\n    with: { path: ${ event.data.path } }\n",
            json!({}),
        )
        .await;
    active(&rig, &applied.instance).await;
    for path in ["a.rs", "b.rs", "c.rs"] {
        rig.journal.emit("file.changed", Source::default(), json!({ "path": path })).await.unwrap();
    }
    // Each event restarts the quiet period, so three sleeps have begun.
    rig.advance_after(since, 3, "2s").await;
    handled(&rig, "on.changed", 1).await;

    let runs = rig.engine.runs(&applied.instance, 10).await.unwrap();
    assert_eq!(runs.len(), 2, "main and one debounced run");
    assert_eq!(rig.engine.run(&runs[0].id).await.unwrap().steps[0].output, json!({ "path": "c.rs" }));
}

#[tokio::test]
async fn what_an_editor_is_told_follows_an_event_is_what_a_handler_finds_there() {
    let rig = Rig::new().await;
    let applied = rig
        .apply(
            "\
blueprint: looks
state: { found: [] }
on:
  moved:
    event: board.item.moved
    steps:
      note: { do: set, with: { found: ${ keys(event) } } }
",
            json!({}),
        )
        .await;
    active(&rig, &applied.instance).await;
    rig.journal.emit("board.item.moved", Source::instance("elsewhere"), json!({ "id": 1 })).await.unwrap();
    handled(&rig, "on.moved", 1).await;

    let mut found: Vec<String> = rig.instance(&applied.instance).await.state["found"]
        .as_array()
        .unwrap()
        .iter()
        .map(|key| key.as_str().unwrap().to_string())
        .collect();
    found.sort();
    let mut described: Vec<&str> =
        blueprint::vocabulary::EVENT_VIEW.iter().map(|choice| choice.name).collect();
    described.sort_unstable();
    assert_eq!(found, described);
}

#[tokio::test]
async fn a_restart_in_the_middle_of_catching_up_loses_none_of_what_is_still_to_come() {
    let mut rig = Rig::new().await;
    let applied = rig
        .apply(
            "blueprint: careful\non:\n  job:\n    event: job.wanted\n    do: test.durable\n    with: { name: \"job-${event.data.n}\" }\n",
            json!({}),
        )
        .await;
    active(&rig, &applied.instance).await;

    rig.engine.halt();
    for n in [1, 2, 3] {
        rig.journal.emit("job.wanted", Source::default(), json!({ "n": n })).await.unwrap();
    }
    rig.restart().await;
    // The first of the three is being handled; the other two are still to come.
    rig.reached("job-1").await;
    rig.restart().await;
    for n in [1, 2, 3] {
        rig.open(&format!("job-{n}"));
    }
    handled(&rig, "on.job", 3).await;

    let runs = rig.engine.runs(&applied.instance, 10).await.unwrap();
    let jobs: Vec<u64> =
        runs.iter().rev().skip(1).map(|run| run.event["data"]["n"].as_u64().unwrap()).collect();
    assert_eq!(jobs, [1, 2, 3], "each exactly once, in order");
}

#[tokio::test]
async fn what_an_instance_says_as_it_ends_is_heard_below_its_parent_however_late_the_handler_gets_to_it() {
    let mut rig = Rig::new().await;
    // The child sets something, which its parent's handler hears and is
    // kept busy by, and then fails; so its failure is looked at only after
    // it has gone from what the conductor has in hand.
    rig.library.put(
        "doomed",
        "\
blueprint: doomed
state: { marked: false }
steps:
  go: { do: test.gate, with: { name: go } }
  mark: { do: set, with: { marked: true } }
  doom: { do: test.gate, with: { name: doom } }
  boom: { do: test.flaky, with: { message: it could not be done } }
",
    );
    let watcher = "\
blueprint: watcher
steps:
  kid: { spawn: doomed }
on:
  below:
    event: instance.*
    scope: own
    steps:
      busy: { when: ${ event.topic == \"instance.state\" }, do: test.gate, with: { name: busy } }
      heard: { do: test.count, with: { key: \"heard-${ event.topic }\" } }
";
    let applied = rig.apply(watcher, json!({})).await;
    active(&rig, &applied.instance).await;
    rig.reached("go").await;
    rig.open("go");
    rig.reached("busy").await;
    rig.reached("doom").await;
    rig.open("doom");
    let failed = rig.event("instance.failed", |_| true).await;
    let kid = failed.source.instance.clone().unwrap();
    assert_ne!(kid, applied.instance);

    // Started again, the conductor knows only what is still running: the
    // child is not among it, and its failure is still its parent's to hear.
    rig.restart().await;
    rig.event("run.finished", |event| {
        event.data["flow"] == "on.below" && event.seq > failed.seq && event.data["status"] == "ok"
    })
    .await;
    assert_eq!(rig.shared.count("heard-instance.failed"), 1, "the failure of what was below it was heard");
    assert_eq!(rig.instance(&kid).await.status, InstanceStatus::Failed);
    assert_eq!(rig.instance(&applied.instance).await.status, InstanceStatus::Active);
}

#[tokio::test]
async fn an_event_that_says_what_hearing_it_takes_is_handled_for_whoever_holds_that_and_for_nobody_else() {
    let rig = Rig::new().await;
    let listens = |name: &str| {
        format!(
            "blueprint: {name}\nsteps:\n  once: {{ do: test.count, with: {{ key: {name}-up }} }}\non:\n  said:\n    event: notes.said\n    do: test.count\n    with: {{ key: {name} }}\n"
        )
    };
    // One applied by somebody who was given the journal, and one who was not.
    let holding = |more: &[&str]| kernel::Principal {
        kind: kernel::PrincipalKind::User,
        id: "grace".into(),
        name: "Grace".into(),
        scopes: ["blueprints", "records"].iter().chain(more).map(|scope| scope.to_string()).collect(),
        key: None,
    };
    let trusted = rig.request(&listens("trusted"), json!({}));
    let trusted = rig
        .engine
        .apply(conductor::Apply { principal: holding(&["private:journal"]), ..trusted })
        .await
        .unwrap();
    let other = rig.request(&listens("other"), json!({}));
    let other = rig
        .engine
        .apply(conductor::Apply { principal: holding(&["private:letters"]), ..other })
        .await
        .unwrap();
    active(&rig, &trusted.instance).await;
    active(&rig, &other.instance).await;

    // Said of something kept private, and then of something that is not.
    let kept = Source::actor("user:ada").taking(Some("private:journal"));
    rig.journal
        .emit("notes.said", kept.clone(), json!({ "body": "the marmalade is under the stairs" }))
        .await
        .unwrap();
    rig.journal
        .emit("notes.said", Source::actor("user:ada"), json!({ "body": "it is raining" }))
        .await
        .unwrap();
    handled(&rig, "on.said", 3).await;
    assert_eq!((rig.shared.count("trusted"), rig.shared.count("other")), (2, 1));

    // What it takes is kept with the event, and is there when it is read again.
    let read = rig.events("notes.said").await;
    assert_eq!(read[0].source.takes.as_deref(), Some("private:journal"));
    assert_eq!(read[1].source.takes, None);
    assert!(rig.engine.hears(&holding(&["private"]), "nobody", &read[0]).await);
    assert!(!rig.engine.hears(&holding(&[]), "nobody", &read[0]).await);
    for instance in [&trusted.instance, &other.instance] {
        rig.engine.stop(instance).await.unwrap();
    }
}

#[tokio::test]
async fn what_the_runner_says_of_a_run_takes_what_the_event_that_set_it_off_took_as_it_goes_taken_up_and_left()
 {
    let mut rig = Rig::new().await;
    // Three that hear the same thing and are in the middle of it when the
    // conductor stops: one that keeps what it does, one that keeps
    // nothing, and one whose text has the handler no longer by then.
    let listens = |name: &str, word: &str, turn: &str| {
        format!(
            "blueprint: {name}\n{word}steps:\n  begin: {{ do: test.count, with: {{ key: {name} }} }}\non:\n  said:\n    event: notes.said\n{turn}    steps:\n      held: {{ do: test.gate, with: {{ name: {name}, lines: [\"${{event.data.body}}\"] }} }}\n      after: {{ needs: [held], do: fail, with: {{ message: \"no further with ${{event.data.body}}\" }} }}\n"
        )
    };
    let kept = rig.apply(&listens("kept", "", ""), json!({})).await;
    let plain = rig.apply(&listens("plain", "durable: false\n", ""), json!({})).await;
    let turned = rig.apply(&listens("turned", "", "    turn: \"${ event.data.row }\"\n"), json!({})).await;
    for applied in [&kept, &plain, &turned] {
        active(&rig, &applied.instance).await;
    }
    let of = |applied: &conductor::Applied| {
        let instance = applied.instance.clone();
        move |event: &kernel::Event| event.source.instance.as_deref() == Some(instance.as_str())
    };

    // Said of something kept private; each prints what it heard.
    let mut lines = rig.follow("log.line");
    let body = "the marmalade is under the stairs";
    let private = Source::actor("user:ada").taking(Some("private:journal"));
    rig.journal.emit("notes.said", private, json!({ "body": body, "row": "a" })).await.unwrap();
    for gate in ["kept", "plain", "turned"] {
        rig.reached(gate).await;
    }
    for _ in 0..3 {
        let line = rig.soon("a line each of them printed", lines.next()).await.unwrap();
        assert_eq!(line.data["text"], body, "{line:?}");
        assert_eq!(line.source.takes.as_deref(), Some("private:journal"), "{line:?}");
    }
    // The third takes a text with no such handler, where it is.
    let dropped = "blueprint: turned\nversion: 2\nsteps:\n  begin: { do: test.count, with: { key: turned } }\non:\n  poke: { event: turned.poke, do: noop }\non_update: []\n";
    rig.apply(dropped, json!({})).await;
    rig.event("instance.updated", of(&turned)).await;

    // The conductor is stopped and started. The first is taken up, the
    // second is over, and the run of the third is ended as one left.
    rig.restart().await;
    rig.open("kept");
    let ended = |applied: &conductor::Applied| {
        let of = of(applied);
        rig.event("run.finished", move |event| of(event) && event.data["flow"] == "on.said")
    };
    let (kept_ended, plain_ended, turned_ended) =
        (ended(&kept).await, ended(&plain).await, ended(&turned).await);
    assert!(plain_ended.data["error"].as_str().unwrap().contains("keeps nothing"), "{plain_ended:?}");
    assert!(turned_ended.data["error"].as_str().unwrap().contains("not in the text"), "{turned_ended:?}");
    let handled: Vec<&str> = [&kept_ended, &plain_ended, &turned_ended]
        .into_iter()
        .map(|event| event.source.run.as_deref().unwrap())
        .collect();

    // Everything kept that was said of those three runs takes what the
    // event did, said as they went, as one was taken up and as two were
    // ended; and what was said of a run no event set off takes nothing.
    let said = rig.events("run.*,step.*").await;
    let (of_those, of_others): (Vec<_>, Vec<_>) = said
        .iter()
        .filter(|event| event.source.run.is_some())
        .partition(|event| handled.contains(&event.source.run.as_deref().unwrap()));
    for event in &of_those {
        assert_eq!(event.source.takes.as_deref(), Some("private:journal"), "{event:?}");
    }
    assert!(!of_others.is_empty());
    for event in &of_others {
        assert_eq!(event.source.takes, None, "{event:?}");
    }
    let run_said = |run: &kernel::Event, topic: &str| {
        of_those.iter().any(|event| event.topic == topic && event.source.run == run.source.run)
    };
    for run in [&kept_ended, &plain_ended, &turned_ended] {
        assert!(run_said(run, "run.started"), "{said:?}");
    }
    assert!(run_said(&kept_ended, "run.resumed") && run_said(&kept_ended, "step.started"), "{said:?}");
    assert!(
        run_said(&turned_ended, "step.cancelled") && run_said(&turned_ended, "step.finished"),
        "{said:?}"
    );
    for instance in [&kept.instance, &turned.instance] {
        rig.engine.stop(instance).await.unwrap();
    }
}

/// What a poller is of: nobody's, or a project's with a journal of its
/// own, as one added now has.
async fn polled(project: bool) {
    let rig = Rig::new().await;
    let project = project.then_some("shop");
    if let Some(project) = project {
        rig.keeps_projects().await;
        rig.journal.own(project).await.unwrap();
    }
    let since = rig.clock.sleep_count();
    // `test.count` returns a new number every time it is asked.
    let source = "\
blueprint: upstream
on:
  moved:
    poll: { every: 30s, do: test.count, with: { key: polls } }
    steps:
      noted: { do: test.count, with: { key: fired } }
";
    let asked = Apply { project: project.map(str::to_string), ..rig.request(source, json!({})) };
    let instance = rig.engine.apply(asked).await.unwrap().instance;
    active(&rig, &instance).await;
    // The first reading is taken as it begins to listen, and a poller
    // adds up to a tenth of its interval as jitter to the next.
    for poll in 1..=2 {
        rig.advance_after(since, poll, "33s").await;
    }
    handled(&rig, "on.moved", 2).await;
    assert_eq!(rig.shared.count("polls"), 3, "the first reading is only a baseline");
    assert_eq!(rig.shared.count("fired"), 2, "each reading after it that changed set the handler off");
}

#[tokio::test]
async fn a_poller_of_nobody_s_is_set_off_by_what_it_reads_changing() {
    polled(false).await;
}

#[tokio::test]
async fn a_poller_of_a_project_with_a_journal_of_its_own_is_set_off_by_what_it_reads_changing() {
    if crate::one_journal() {
        return;
    }
    polled(true).await;
}

/// What a poller is of in a test of what it reads changing as its text
/// is taken where its instance is.
async fn taken_while_reading(project: bool) {
    let rig = Rig::new().await;
    let project = project.then_some("shop");
    if let Some(project) = project {
        rig.keeps_projects().await;
        rig.journal.own(project).await.unwrap();
    }
    let text = |version: u64| {
        format!(
            "\
blueprint: upstream
version: {version}
on:
  moved:
    poll: {{ every: 30s, do: test.reads, with: {{ key: answer }} }}
    steps:
      noted: {{ do: test.count, with: {{ key: fired }} }}
"
        )
    };
    let applied = |version: u64| Apply {
        project: project.map(str::to_string),
        ..rig.request(&text(version), json!({}))
    };
    let mut reading = rig.follow("test.reached");
    let since = rig.clock.sleep_count();
    rig.shared.note("answer", "a");
    let instance = rig.engine.apply(applied(1)).await.unwrap().instance;
    active(&rig, &instance).await;
    // The first reading, only a baseline, as it begins to listen.
    rig.soon("the first reading", reading.next()).await.unwrap();
    rig.open("answer");
    // The answer is another, and while the source is answering the
    // second reading the instance takes another text where it is: the
    // handler that is reading is told to take no more, and the one that
    // takes its place waits for it to end.
    rig.advance_after(since, 1, "33s").await;
    rig.soon("the second reading", reading.next()).await.unwrap();
    rig.shared.note("answer", "b");
    let taken = rig.engine.apply(applied(2)).await.unwrap();
    assert_eq!(taken.instance, instance);
    let updated = rig.event("instance.updated", |event| event.data["in_place"] == true).await;
    assert_eq!(updated.source.instance.as_deref(), Some(instance.as_str()), "the text was taken where it is");
    rig.open("answer");
    // The handler that read it sleeps once more before it sees it is to
    // take no more, and the one that took its place, which has a reading
    // to tell the next from, reads after its own.
    rig.advance_after(since, 3, "33s").await;
    rig.soon("the third reading", reading.next()).await.unwrap();
    rig.open("answer");
    handled(&rig, "on.moved", 1).await;
    assert_eq!(
        rig.shared.count("fired"),
        1,
        "the change that came as the text was taken set the handler off once"
    );
}

#[tokio::test]
async fn a_change_a_poller_read_as_its_text_was_taken_sets_off_the_handler_that_takes_its_place() {
    taken_while_reading(false).await;
}

#[tokio::test]
async fn a_change_a_poller_of_a_project_with_a_journal_of_its_own_read_as_its_text_was_taken_sets_off_the_handler_that_takes_its_place()
 {
    if crate::one_journal() {
        return;
    }
    taken_while_reading(true).await;
}

/// What a poller is of in a test of its source changing soon after it
/// begins to listen.
async fn changed_soon_after(project: bool) {
    let rig = Rig::new().await;
    let project = project.then_some("shop");
    if let Some(project) = project {
        rig.keeps_projects().await;
        rig.journal.own(project).await.unwrap();
    }
    let source = "\
blueprint: upstream
on:
  moved:
    poll: { every: 1m, do: test.reads, with: { key: answer } }
    steps:
      noted: { do: test.count, with: { key: fired } }
";
    let mut reading = rig.follow("test.reached");
    let since = rig.clock.sleep_count();
    rig.shared.note("answer", "a");
    let asked = Apply { project: project.map(str::to_string), ..rig.request(source, json!({})) };
    let instance = rig.engine.apply(asked).await.unwrap().instance;
    active(&rig, &instance).await;
    // The first reading is taken as it begins to listen, with no time
    // let pass: it is what a change is told from.
    rig.soon("the first reading", reading.next()).await.unwrap();
    rig.open("answer");
    rig.soon("the poller to wait for its next", rig.clock.sleeps_started(since + 1)).await;
    // The source changes well before a whole interval has passed, which
    // is when the first reading used to be taken, and so held the change
    // as what the next was told from.
    rig.shared.note("answer", "b");
    rig.advance_after(since, 1, "66s").await;
    rig.soon("the second reading", reading.next()).await.unwrap();
    rig.open("answer");
    handled(&rig, "on.moved", 1).await;
    assert_eq!(rig.shared.count("fired"), 1, "what changed after it began to listen set it off");
    let run = &rig.engine.runs(&instance, 1).await.unwrap()[0];
    assert_eq!(
        (&run.event["previous"]["said"], &run.event["data"]["said"]),
        (&json!("a"), &json!("b")),
        "it was told from the first reading"
    );
}

#[tokio::test]
async fn a_poller_reads_first_as_it_begins_to_listen_so_a_change_before_its_next_reading_sets_it_off() {
    changed_soon_after(false).await;
}

#[tokio::test]
async fn a_poller_of_a_project_with_a_journal_of_its_own_reads_first_as_it_begins_to_listen() {
    if crate::one_journal() {
        return;
    }
    changed_soon_after(true).await;
}

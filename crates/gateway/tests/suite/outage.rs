//! Links that drop, devices that are down, and daemons that restart.

use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;

use conductor::{RunStatus, StepStatus};

use crate::{Rig, soon, step};

const GATED: &str = "blueprint: gated\nsteps:\n  work: { do: test.gate, device: builder-1, with: { name: job } }\n  after: { do: test.where }\n";

#[tokio::test]
async fn a_link_that_drops_mid_step_costs_time_not_the_step() {
    let rig = Rig::linked().await;
    let applied = rig.hub.apply(GATED).await;
    rig.shared.reached("job", 1).await;

    rig.wire.cut();
    rig.until_unlinked().await;
    // The work finishes while nobody can be told.
    rig.shared.open("job");
    let status = rig.member.gateway.uplink().unwrap().status;
    assert_eq!(status.error.as_deref(), Some("the link to the hub was lost"));

    rig.wire.restore();
    rig.until_linked().await;
    let run = rig.hub.finished(&applied).await;
    assert_eq!(run.status, RunStatus::Ok, "{run:?}");
    assert_eq!(step(&run, "work").output["device"], "member-box");
    assert_eq!(
        rig.shared.starts("job"),
        ["member-box"],
        "it ran once; the result was kept until it could be delivered"
    );
    assert_eq!(
        rig.hub.comings_and_goings().await,
        ["device.invited", "device.added", "device.connected", "device.disconnected", "device.connected"]
    );
}

#[tokio::test]
async fn a_step_sent_while_the_link_is_down_is_delivered_when_it_comes_back() {
    let rig = Rig::linked().await;
    rig.wire.cut();
    rig.until_unlinked().await;

    let applied =
        rig.hub.apply("blueprint: patient\nsteps:\n  there: { do: test.where, device: builder-1 }\n").await;
    let waiting = soon("the step to say it is waiting", async {
        loop {
            let run = rig.hub.engine.runs(&applied.instance, 1).await.unwrap().remove(0);
            let log = rig.hub.engine.step_log(&run.id, "there", 0, 10);
            if let Some(line) = log.first() {
                break line.text.clone();
            }
            tokio::task::yield_now().await;
        }
    })
    .await;
    assert_eq!(waiting, "device `builder-1` is not linked; waiting for it");

    rig.wire.restore();
    rig.until_linked().await;
    let run = rig.hub.finished(&applied).await;
    assert_eq!(step(&run, "there").output["device"], "member-box");
}

#[tokio::test]
async fn a_device_that_stays_away_fails_the_step_so_the_blueprint_can_react() {
    let rig = Rig::linked().await;
    rig.hub.gateway.hub().set_grace(Duration::from_secs(90));
    rig.wire.cut();
    rig.until_unlinked().await;

    // The blueprint's own answer to a device being down: try elsewhere.
    let source = "blueprint: fallback\nsteps:\n  there: { do: test.where, device: builder-1, allow_failure: true }\n  here:\n    when: ${ steps.there.status != 'ok' }\n    do: test.where\n";
    let applied = rig.hub.apply(source).await;
    let run = soon("the step to give up on the device", async {
        loop {
            let run = rig.hub.engine.runs(&applied.instance, 1).await.unwrap().remove(0);
            if run.status != RunStatus::Running {
                break rig.hub.engine.run(&run.id).await.unwrap();
            }
            rig.clock.advance(Duration::from_secs(30));
            tokio::task::yield_now().await;
        }
    })
    .await;
    let there = step(&run, "there");
    assert_eq!(there.status, StepStatus::Failed);
    let error = there.error.clone().unwrap();
    assert!(
        error.starts_with("device `builder-1` has not been linked for 1m30s; last heard from 2026-01-01T"),
        "{error}"
    );
    assert_eq!(step(&run, "here").output["device"], "hub", "and the work was done here instead");
    assert_eq!(run.status, RunStatus::Ok);
}

#[tokio::test]
async fn a_member_that_restarts_is_sent_its_work_again() {
    let mut rig = Rig::linked().await;
    let applied = rig.hub.apply(GATED).await;
    rig.shared.reached("job", 1).await;

    // The member's daemon dies with the work unfinished, and a new one links.
    rig.member.restart().await;
    rig.until_unlinked().await;
    rig.member.gateway.start().await.unwrap();
    rig.until_linked().await;

    rig.shared.reached("job", 2).await;
    rig.shared.open("job");
    let run = rig.hub.finished(&applied).await;
    assert_eq!(run.status, RunStatus::Ok, "{run:?}");
    assert_eq!(rig.shared.starts("job").len(), 2, "what the member forgot, it was asked to do again");
    assert_eq!(
        *rig.shared.cancelled.lock().unwrap(),
        ["job"],
        "the first try was called off when its daemon stopped, and that was not mistaken for an answer"
    );
}

#[tokio::test]
async fn a_hub_that_restarts_finds_its_step_still_being_worked_on() {
    let mut rig = Rig::linked().await;
    let applied = rig.hub.apply(GATED).await;
    rig.shared.reached("job", 1).await;

    // The hub's daemon dies; the member carries on, and links to the new one.
    rig.hub.restart().await;
    rig.wire.rewire(&rig.hub.gateway);
    rig.hub.engine.resume().await.unwrap();
    rig.until_linked().await;

    rig.shared.open("job");
    let run = rig.hub.finished(&applied).await;
    assert_eq!(run.status, RunStatus::Ok, "{run:?}");
    assert_eq!(step(&run, "work").output["device"], "member-box");
    assert_eq!(step(&run, "after").output["device"], "hub");
    assert_eq!(rig.shared.starts("job"), ["member-box"], "the same command arriving again is not run again");
}

#[tokio::test]
async fn a_member_waits_longer_each_time_and_as_long_as_the_hub_asks() {
    let pauses: Vec<u64> = (1..=9).map(|failures| gateway::member::backoff(failures).as_secs()).collect();
    assert_eq!(pauses, [1, 2, 4, 8, 16, 32, 60, 60, 60]);

    // A hub that is busy says how long to stay away, and is obeyed.
    let rig = Rig::linked().await;
    *rig.wire.retry_after.lock().unwrap() = Some(Duration::from_secs(300));
    rig.wire.cut();
    rig.until_unlinked().await;
    let status = soon("the member to schedule its next attempt", async {
        loop {
            let status = rig.member.gateway.uplink().unwrap().status;
            if status.retry_at.is_some() && status.failures >= 2 {
                break status;
            }
            // The first attempt after losing the link is the ordinary one;
            // it is the refusal that carries the hub's request.
            rig.clock.advance(Duration::from_secs(2));
            tokio::task::yield_now().await;
        }
    })
    .await;
    let wait = status.retry_at.unwrap() - rig.member.journal.now_ms();
    assert!((298_000..=360_000).contains(&wait), "asked for five minutes, waiting {wait} ms");
    assert_eq!(status.error.as_deref(), Some("the wire is cut"));
    let attempts = rig.wire.attempts.load(Ordering::SeqCst);
    rig.clock.advance(Duration::from_secs(120));
    tokio::task::yield_now().await;
    assert_eq!(
        rig.wire.attempts.load(Ordering::SeqCst),
        attempts,
        "no attempt is made before the time asked for"
    );
}

/// A hub played by the test: it sends what the test tells it to, and takes
/// every result it is given.
struct Scripted {
    commands: std::sync::Mutex<Option<tokio::sync::mpsc::Receiver<gateway::Command>>>,
    taken: tokio::sync::watch::Sender<Vec<String>>,
}

impl gateway::Uplink for Scripted {
    fn open(
        &self,
    ) -> kernel::BoxFut<'static, Result<tokio::sync::mpsc::Receiver<gateway::Command>, gateway::LinkError>>
    {
        let commands = self.commands.lock().unwrap().take();
        Box::pin(async move { commands.ok_or_else(|| gateway::LinkError::new("only one link in this test")) })
    }

    fn report(
        &self,
        report: gateway::Report,
    ) -> kernel::BoxFut<'static, Result<gateway::Ack, gateway::LinkError>> {
        let ids: Vec<String> = report.results.iter().map(|result| result.id.clone()).collect();
        self.taken.send_modify(|taken| taken.extend(ids.clone()));
        Box::pin(async move { Ok(gateway::Ack { ids }) })
    }
}

#[tokio::test]
async fn a_copy_of_a_command_that_arrives_after_its_answer_was_taken_is_not_run_again() {
    let clock = kernel::ManualClock::new();
    let shared = Arc::new(crate::Shared::default());
    let member = crate::Device::new("member-box", &clock, &shared).await;
    let (send, commands) = tokio::sync::mpsc::channel(8);
    let hub = Arc::new(Scripted {
        commands: std::sync::Mutex::new(Some(commands)),
        taken: tokio::sync::watch::Sender::new(Vec::new()),
    });
    member.gateway.link(hub.clone(), vec!["*".into()], "test://hub");

    let command = |id: &str, action: &str| {
        gateway::Command::Run(Box::new(gateway::wire::RunCommand {
            id: id.to_string(),
            action: action.to_string(),
            args: kernel::fields(serde_json::json!({ "name": "once" })),
            principal: crate::ada(),
            project: None,
            origin: kernel::Source { step: Some("work".into()), ..kernel::Source::default() },
            attempt: 1,
            env: Default::default(),
            secrets: Default::default(),
        }))
    };
    let taken = |id: &'static str| {
        let mut taken = hub.taken.subscribe();
        async move {
            soon(
                &format!("the answer to `{id}` to be taken"),
                taken.wait_for(|all| all.iter().any(|one| one == id)),
            )
            .await
            .unwrap();
        }
    };

    // Done, answered, and the answer taken.
    shared.open("once");
    send.send(command("run_1/work/1", "test.gate")).await.unwrap();
    taken("run_1/work/1").await;
    // The hub had sent it twice (it does, whenever it is unsure the first
    // got through), and the second copy arrives only now.
    send.send(command("run_1/work/1", "test.gate")).await.unwrap();
    // Something after it, to know the copy has been dealt with.
    send.send(command("run_1/after/1", "test.where")).await.unwrap();
    taken("run_1/after/1").await;
    assert_eq!(shared.starts("once"), ["member-box"], "once is once");
}

#[tokio::test]
async fn an_answer_that_arrives_before_a_restarted_hub_is_ready_for_it_is_not_lost() {
    let mut rig = Rig::linked().await;
    let applied = rig.hub.apply(GATED).await;
    rig.shared.reached("job", 1).await;

    // The hub dies; while it is away the member finishes the work.
    rig.wire.cut();
    rig.hub.restart().await;
    rig.shared.open("job");
    rig.wire.rewire(&rig.hub.gateway);
    rig.wire.restore();
    rig.until_linked().await;

    // The member offers its answer to a hub that has not yet picked its
    // runs up again, so nothing there is waiting for it.
    soon("the member to report to the new hub", async {
        while rig.hub.gateway.devices().await.unwrap()[0].info.is_none() {
            tokio::task::yield_now().await;
        }
    })
    .await;
    let unclaimed = gateway::Report {
        results: vec![gateway::wire::Outcome {
            id: "run_x/none/1".into(),
            ok: true,
            output: 1.into(),
            error: None,
        }],
        ..gateway::Report::default()
    };
    let ack = rig.hub.gateway.hub().report("builder-1", unclaimed).await;
    assert!(ack.ids.is_empty(), "an answer nothing is waiting for is not taken, so it is not thrown away");

    // Now the hub resumes; the step asks again and gets the answer that
    // was kept for it, without the work being done a second time.
    rig.hub.engine.resume().await.unwrap();
    let run = rig.hub.finished(&applied).await;
    assert_eq!(run.status, RunStatus::Ok, "{run:?}");
    assert_eq!(step(&run, "work").output["device"], "member-box");
    assert_eq!(rig.shared.starts("job"), ["member-box"]);
}

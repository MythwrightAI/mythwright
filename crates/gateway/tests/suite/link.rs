//! Steps on another device, when the link is healthy.

use conductor::{InstanceFilter, RunStatus, StepStatus};
use serde_json::json;

use crate::{Rig, ada, soon, step};

#[tokio::test]
async fn a_step_for_a_member_runs_there_and_its_output_and_log_come_back() {
    let rig = Rig::linked().await;
    let run = rig
        .hub
        .run(
            "blueprint: spread\nsteps:\n  here: { do: test.where }\n  there: { do: test.where, device: builder-1, with: { say: careful } }\n",
        )
        .await;
    assert_eq!(run.status, RunStatus::Ok, "{run:?}");
    assert_eq!(step(&run, "here").output["device"], "hub");
    assert_eq!(
        step(&run, "there").output,
        json!({ "device": "member-box", "by": "user:ada", "attempt": 1 }),
        "it ran on the member, on the authority of whoever applied the blueprint"
    );

    // What the action wrote there is in the step's log here, as if it had run here.
    let lines: Vec<(char, String)> = rig
        .hub
        .engine
        .step_log(&run.id, "there", 0, 100)
        .into_iter()
        .map(|line| (line.stream, line.text))
        .collect();
    assert_eq!(lines, [('o', "hello from member-box".to_string()), ('e', "careful".to_string())]);

    assert_eq!(rig.hub.comings_and_goings().await, ["device.invited", "device.added", "device.connected"]);
    assert!(
        rig.member.topics("run.*").await.is_empty(),
        "the run belongs to the hub; the member only did the work"
    );
}

#[tokio::test]
async fn the_hub_knows_its_members_and_how_they_stand() {
    let rig = Rig::apart().await;
    let before = rig.hub.gateway.devices().await.unwrap();
    assert_eq!(before.len(), 1);
    assert_eq!(
        (before[0].name.as_str(), before[0].connected, before[0].last_seen),
        ("builder-1", false, None)
    );

    rig.link(vec!["*".into()]).await;
    let device = soon("the member to introduce itself", async {
        loop {
            let devices = rig.hub.gateway.devices().await.unwrap();
            if devices[0].info.is_some() {
                break devices[0].clone();
            }
            tokio::task::yield_now().await;
        }
    })
    .await;
    assert!(device.connected && device.since.is_some() && device.last_seen.is_some());
    let info = device.info.unwrap();
    assert!(info.cpus > 0 && !info.os.is_empty(), "{info:?}");
    let link = rig.member.gateway.uplink().unwrap();
    assert_eq!(
        (link.status.hub.as_deref(), link.status.device.as_deref(), link.url.as_str()),
        (Some("hub"), Some("builder-1"), "http://hub"),
        "the member learns what the hub calls itself and what it calls the member"
    );

    // Removed from the fleet: the link is closed and the name means nothing.
    rig.hub.gateway.remove("builder-1", &ada()).await.unwrap();
    assert!(rig.hub.gateway.devices().await.unwrap().is_empty());
    assert!(rig.hub.gateway.hub().linked_since("builder-1").is_none());
    let run = rig.hub.run("blueprint: late\nsteps:\n  there: { do: test.where, device: builder-1 }\n").await;
    assert_eq!(
        run.error.as_deref(),
        Some(
            "step `there` failed: there is no device called `builder-1` in this fleet; no devices have been added to it"
        )
    );
}

#[tokio::test]
async fn a_name_no_device_has_is_an_error_at_once_and_names_the_ones_there_are() {
    let rig = Rig::linked().await;
    let run = rig.hub.run("blueprint: typo\nsteps:\n  there: { do: test.where, device: bilder-1 }\n").await;
    assert_eq!(run.status, RunStatus::Failed);
    assert_eq!(
        run.error.as_deref(),
        Some("step `there` failed: there is no device called `bilder-1` in this fleet; it has: builder-1")
    );
}

#[tokio::test]
async fn a_device_is_invited_by_an_admin_under_a_sensible_name_for_a_while_and_somewhere() {
    let rig = Rig::apart().await;
    let at = || vec!["http://hub".to_string()];
    let visitor = kernel::Principal { scopes: vec!["fleet:read".into()], ..ada() };
    assert_eq!(
        rig.hub.gateway.invite("builder-2", None, at(), &visitor).await.unwrap_err().message,
        "Ada is not allowed to do this (needs `admin`)"
    );
    for bad in ["", "Builder 2", "-x", "local"] {
        let refused = rig.hub.gateway.invite(bad, None, at(), &ada()).await;
        assert!(refused.is_err(), "`{bad}` should not be a device name");
    }
    assert_eq!(
        rig.hub.gateway.invite("hub", None, at(), &ada()).await.unwrap_err().message,
        "`hub` is this device's own name"
    );
    // For a day unless said, and never for more than fourteen.
    let day = 24 * 3600 * 1000;
    let now = rig.hub.journal.now_ms();
    let first = rig.hub.gateway.invite("builder-2", None, at(), &ada()).await.unwrap();
    assert_eq!(first.expires, now + day);
    let long = std::time::Duration::from_secs(15 * 24 * 3600);
    let refused = rig.hub.gateway.invite("builder-2", Some(long), at(), &ada()).await.unwrap_err();
    assert!(refused.message.contains("no more than fourteen"), "{}", refused.message);
    // It says where the hub is, or there is no invitation.
    let nowhere = rig.hub.gateway.invite("builder-2", None, Vec::new(), &ada()).await.unwrap_err();
    assert!(nowhere.message.contains("--at http://host:port"), "{}", nowhere.message);
    // Each is a string of its own, and the hub keeps the secret of neither.
    let second = rig.hub.gateway.invite("builder-2", None, at(), &ada()).await.unwrap();
    assert!(first.invitation.starts_with("inv_") && first.invitation != second.invitation);
    let secret = gateway::Invitation::read(&second.invitation).unwrap().secret;
    let kept = rig.hub.journal.kv_list("gateway.invitations").await.unwrap();
    assert_eq!(kept.len(), 3, "the rig's own, and these two");
    assert!(!format!("{kept:?}").contains(&secret), "what is kept is a hash of it");
    // Invited, it is listed as a device that has yet to come.
    let devices = rig.hub.gateway.devices().await.unwrap();
    let waiting: Vec<(&str, bool)> =
        devices.iter().map(|device| (device.name.as_str(), device.invited.is_some())).collect();
    assert_eq!(waiting, [("builder-1", true), ("builder-2", true)]);
}

#[tokio::test]
async fn a_member_decides_what_its_hub_may_do_on_it() {
    let rig = Rig::apart().await;
    rig.link(vec!["fleet:read".into()]).await;
    let run = rig
        .hub
        .run(
            "blueprint: nosy\npermissions: [shell, fleet]\nsteps:\n  look: { do: system.check, device: builder-1 }\n  touch: { do: shell, device: builder-1, with: { run: \"echo hi\" } }\n",
        )
        .await;
    assert_eq!(step(&run, "look").status, StepStatus::Ok, "{run:?}");
    assert_eq!(
        step(&run, "touch").error.as_deref(),
        Some("device `builder-1` does not let its hub use `shell`; it allows: fleet:read")
    );
}

#[tokio::test]
async fn a_real_command_runs_on_the_member_and_reports_how_it_ended() {
    let rig = Rig::linked().await;
    let marker = rig.member.dir.join("was-here");
    let touches = crate::standin::cmd(&["write", &marker.to_string_lossy(), "made it", "say", "done"]);
    let breaks = crate::standin::cmd(&["err", "no such table", "exit", "3"]);
    let source = format!(
        "blueprint: remote-shell\npermissions: [shell]\nsteps:\n  touch: {{ do: shell, device: builder-1, with: {{ cmd: {touches} }} }}\n  break: {{ do: shell, device: builder-1, allow_failure: true, with: {{ cmd: {breaks} }} }}\n"
    );
    let run = rig.hub.run(&source).await;
    assert_eq!(step(&run, "touch").status, StepStatus::Ok, "{run:?}");
    assert_eq!(std::fs::read_to_string(&marker).unwrap().trim(), "made it");
    let failed = step(&run, "break");
    assert_eq!(
        failed.error.as_deref(),
        Some("exited with code 3: no such table"),
        "the member's words, not the gateway's"
    );
    let log: Vec<String> =
        rig.hub.engine.step_log(&run.id, "touch", 0, 10).into_iter().map(|line| line.text).collect();
    assert!(log.contains(&"done".to_string()), "{log:?}");
}

#[tokio::test]
async fn what_must_stay_running_on_a_member_is_started_as_a_blueprint_there() {
    let rig = Rig::linked().await;
    // A lone step cannot leave something behind: nothing there would own it.
    let run = rig.hub.run("blueprint: orphan\nsteps:\n  keep: { do: test.hold, device: builder-1 }\n").await;
    let error = step(&run, "keep").error.clone().unwrap();
    assert!(
        error.contains("needs an instance on this device to own it") && error.contains("blueprint.apply"),
        "{error}"
    );

    // Applying a blueprint there gives it an owner. (A doubled dollar sign
    // makes the expression the inner blueprint's to fill in, not this one's.)
    let source = "blueprint: placer\npermissions: [blueprints]\nsteps:\n  host:\n    do: blueprint.apply\n    device: builder-1\n    with:\n      key: web\n      params: { greeting: hello }\n      source: |\n        blueprint: web\n        params: { greeting: { type: string } }\n        steps:\n          hold: { do: test.hold }\n        outputs: { said: \"$${params.greeting}\" }\n";
    let run = rig.hub.run(source).await;
    assert_eq!(run.status, RunStatus::Ok, "{run:?}");
    let out = &step(&run, "host").output;
    assert_eq!(
        (&out["status"], &out["outputs"]["said"], &out["device"]),
        (&json!("ok"), &json!("hello"), &json!("member-box"))
    );
    let there = rig
        .member
        .engine
        .instances(InstanceFilter { live_only: true, ..InstanceFilter::default() })
        .await
        .unwrap();
    assert_eq!(there.len(), 1);
    assert_eq!(
        (there[0].blueprint.as_str(), there[0].key.as_str(), there[0].holds.clone()),
        ("web", "web", vec!["thing:1".to_string()])
    );
    assert!(
        rig.hub
            .engine
            .instances(InstanceFilter { live_only: true, ..InstanceFilter::default() })
            .await
            .unwrap()
            .is_empty()
    );

    // And it is stopped the same way.
    let stop = "blueprint: remover\npermissions: [blueprints]\nsteps:\n  stop: { do: blueprint.stop, device: builder-1, with: { key: web } }\n  again: { do: blueprint.stop, device: builder-1, with: { key: web } }\n";
    let run = rig.hub.run(stop).await;
    assert_eq!(
        (&step(&run, "stop").output["stopped"], &step(&run, "again").output["stopped"]),
        (&json!(true), &json!(false))
    );
    assert!(
        rig.member
            .engine
            .instances(InstanceFilter { live_only: true, ..InstanceFilter::default() })
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn a_step_that_runs_out_of_time_stops_the_work_on_the_member() {
    let rig = Rig::linked().await;
    let applied = rig
        .hub
        .apply("blueprint: slow\nsteps:\n  wait: { do: test.gate, device: builder-1, timeout: 5s, with: { name: stuck } }\n")
        .await;
    rig.shared.reached("stuck", 1).await;
    assert_eq!(rig.shared.starts("stuck"), ["member-box"]);

    rig.clock.advance(std::time::Duration::from_secs(5));
    let run = rig.hub.finished(&applied).await;
    assert_eq!(step(&run, "wait").error.as_deref(), Some("timed out after 5s"));
    soon("the member to be told to stop", async {
        while rig.shared.cancelled.lock().unwrap().is_empty() {
            tokio::task::yield_now().await;
        }
    })
    .await;
    assert_eq!(*rig.shared.cancelled.lock().unwrap(), ["stuck"]);
}

#[tokio::test]
async fn a_device_taken_out_stops_what_it_was_doing_for_the_hub_and_the_step_that_waited_ends_at_once() {
    let rig = Rig::linked().await;
    // Something the hub started on the member earlier, which stays there.
    let placed = "blueprint: placer\npermissions: [blueprints]\nsteps:\n  host:\n    do: blueprint.apply\n    device: builder-1\n    with:\n      key: web\n      source: |\n        blueprint: web\n        steps:\n          hold: { do: test.hold }\n";
    let run = rig.hub.run(placed).await;
    assert_eq!(run.status, RunStatus::Ok, "{run:?}");
    let web = step(&run, "host").output["instance"].as_str().unwrap().to_string();
    let applied = rig
        .hub
        .apply(
            "blueprint: slow\nsteps:\n  wait: { do: test.gate, device: builder-1, with: { name: stuck } }\n",
        )
        .await;
    rig.shared.reached("stuck", 1).await;

    rig.hub.gateway.remove("builder-1", &ada()).await.unwrap();
    // The step that waited on it is over now, and says why: no time has
    // passed, so it did not wait out how long a device may be away.
    let run = rig.hub.finished(&applied).await;
    let why = step(&run, "wait").error.clone().unwrap_or_default();
    assert!(
        why.contains("`builder-1` was taken out of this fleet while this step was waiting on it"),
        "{why}"
    );

    // The member is told by the hub itself, when it next asks to be let
    // in, that it is out: it stops what it was doing for that hub, keeps
    // no answer for it, and says so once.
    rig.until("the member to stop what it was doing", || !rig.shared.cancelled.lock().unwrap().is_empty())
        .await;
    assert_eq!(*rig.shared.cancelled.lock().unwrap(), ["stuck"]);
    let unjoined = async || {
        let asked =
            journal::EventQuery { topics: "device.unjoined".into(), limit: 100, ..Default::default() };
        rig.member.journal.read(asked).await.unwrap()
    };
    soon("the member to say it is no longer joined", async {
        while unjoined().await.is_empty() {
            tokio::task::yield_now().await;
        }
    })
    .await;
    let said = unjoined().await;
    assert_eq!(said.len(), 1, "{said:?}");
    assert_eq!(said[0].data["hub"], "http://hub");
    let why = said[0].data["why"].as_str().unwrap();
    assert!(why.contains("no longer one of its fleet") && why.contains("does not know the key"), "{why}");
    assert!(why.contains("device join <invitation>"), "it says how to join again: {why}");
    // What is said is what is so: the steps under way were stopped, and
    // what the hub had started there earlier is still running, is named,
    // and is the device's owner's to stop.
    assert!(why.contains("The steps it was doing for the hub were stopped"), "{why}");
    assert!(why.contains(&format!("{web} (web)")) && why.contains("stop <id>"), "{why}");
    let there = rig
        .member
        .engine
        .instances(InstanceFilter { live_only: true, ..InstanceFilter::default() })
        .await
        .unwrap();
    assert_eq!(
        there.iter().map(|one| one.id.as_str()).collect::<Vec<_>>(),
        [web.as_str()],
        "it was left running"
    );

    // It has forgotten the hub: it is joined to nothing, asks that hub
    // nothing more however long it is left, and whoever looks is told
    // what happened, until it joins again.
    assert!(rig.member.gateway.uplink().is_none());
    let tried = rig.wire.attempts.load(std::sync::atomic::Ordering::SeqCst);
    rig.clock.advance(std::time::Duration::from_secs(24 * 3600));
    for _ in 0..200 {
        tokio::task::yield_now().await;
    }
    assert_eq!(rig.wire.attempts.load(std::sync::atomic::Ordering::SeqCst), tried, "it asked again");
    assert_eq!(unjoined().await.len(), 1);
    let kept = rig.member.gateway.unjoined().await.expect("what happened is kept for whoever looks");
    assert_eq!(kept["hub"], "http://hub");
    assert!(kept["why"].as_str().unwrap().contains("no longer one of its fleet"), "{kept}");
    assert_eq!((&kept["started"][0]["id"], &kept["started"][0]["blueprint"]), (&json!(web), &json!("web")));
}

#[test]
fn events_are_read_from_the_stream_as_they_complete() {
    let mut buffer = String::from(
        ": still here\n\ndata: {\"type\":\"cancel\",\"id\":\"a\"}\n\nid: 7\ndata: one\ndata: two\n\ndata: unfin",
    );
    assert_eq!(gateway::http::take_events(&mut buffer), ["{\"type\":\"cancel\",\"id\":\"a\"}", "one\ntwo"]);
    assert_eq!(buffer, "data: unfin", "what has not finished arriving is kept");
    buffer.push_str("ished\n\n");
    assert_eq!(gateway::http::take_events(&mut buffer), ["unfinished"]);
    assert!(buffer.is_empty());
}

#[tokio::test]
async fn a_rolling_update_goes_one_device_at_a_time_and_stops_at_the_first_that_fails() {
    let rig = Rig::linked().await;
    let roll = |command: String| {
        let params =
            kernel::fields(json!({ "devices": ["builder-1", "hub"], "run": command, "check": "exit 0" }));
        let request = conductor::Apply {
            blueprint: "std/rolling".into(),
            source: None,
            params,
            key: None,
            parent: None,
            target: None,
            project: None,
            principal: ada(),
            force: true,
            take: false,
            labels: Default::default(),
            environment: None,
        };
        async {
            let applied = rig.hub.engine.apply(request).await.unwrap();
            rig.hub.finished(&applied).await
        }
    };

    // Each device does the work in its own directory; both are visited.
    let visits =
        |device: &crate::Device| std::fs::read_to_string(device.dir.join("visits")).unwrap_or_default();
    // What is rolled out is a line for each device's shell, which names
    // a program; what checks it, a line either system's shell reads.
    let run = roll(crate::standin::line(&["append", "visits", "visited"])).await;
    assert_eq!(run.status, RunStatus::Ok, "{run:?}");
    assert_eq!(run.outputs["devices"], 2);
    assert_eq!((visits(&rig.member).as_str(), visits(&rig.hub).as_str()), ("visited\n", "visited\n"));

    // The first device is the canary: when it fails, the second is left alone.
    let fails = ["append", "visits", "visited", "err", "the disk is full", "exit", "28"];
    let run = roll(crate::standin::line(&fails)).await;
    assert_eq!(run.status, RunStatus::Failed);
    let error = run.error.unwrap();
    assert!(error.contains("exited with code 28: the disk is full"), "{error}");
    assert_eq!(visits(&rig.member), "visited\nvisited\n", "the first device was tried");
    assert_eq!(visits(&rig.hub), "visited\n", "the one after it was never touched");
}

#[tokio::test]
async fn the_hub_hears_when_what_a_member_runs_changes() {
    let rig = Rig::linked().await;
    let told = |count: usize| {
        let mut follow = rig.hub.journal.follow(0, "device.changed");
        async move {
            soon(&format!("the hub to be told {count} time(s) that the member changed"), async {
                for _ in 0..count {
                    follow.next().await.unwrap();
                }
            })
            .await;
        }
    };
    // Once when it first says what it is...
    told(1).await;
    assert_eq!(rig.hub.gateway.devices().await.unwrap()[0].info.as_ref().unwrap().instances, 0);

    // ...and again as soon as it is running something new, without waiting
    // for its next routine report.
    rig.member.run("blueprint: resident\nsteps:\n  stay: { do: test.hold }\n").await;
    told(2).await;
    assert_eq!(rig.hub.gateway.devices().await.unwrap()[0].info.as_ref().unwrap().instances, 1);
}

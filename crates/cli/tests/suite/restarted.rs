//! A factory started again by a step of its own (`factory.restart`), kept
//! running by the real `up`: the daemon ends saying it is to come back,
//! what keeps it starts it again at once, and the run that asked goes on
//! in the daemon that comes back, with one daemon and one keeper all along.
//!
//! Unix's alone, as the daemons of `daemon.rs` are, which this is.
#![cfg(unix)]

use serde_json::Value;
use tokio::io::{AsyncBufReadExt, BufReader};

use crate::daemon::{Daemon, alive};
use crate::{command_in, soon};

const TAKEN_UP: &str = "\
blueprint: taken-up
permissions: [admin, emit:errand.after]
steps:
  restarted:
    do: factory.restart
  after:
    needs: [restarted]
    do: emit
    with: { topic: errand.after }
";

/// The daemon a keeper is keeping: its one child.
fn kept_by(keeper: u32) -> u32 {
    let children = std::process::Command::new("pgrep").args(["-P", &keeper.to_string()]).output().unwrap();
    String::from_utf8_lossy(&children.stdout)
        .trim()
        .parse()
        .expect("what keeps a daemon has one child: the daemon")
}

#[tokio::test(flavor = "multi_thread")]
async fn a_step_has_up_start_the_daemon_again_and_its_run_goes_on_in_the_one_that_comes_back() {
    let mut daemon = Daemon::start("up").await;
    daemon
        .run(&["register", "--email", "ada@example.com", "--name", "Ada", "--password", "correct horse"])
        .await
        .ok();
    let keeper = daemon.child.id().unwrap();
    let first = kept_by(keeper);
    let file = daemon.file(&brand::blueprint_file("taken-up"), TAKEN_UP);
    let applied = daemon.run(&["apply", &file, "--no-wait", "--json"]).await.ok();
    let applied: Value = serde_json::from_str(&applied).unwrap();
    let run = applied["run"].as_str().unwrap_or_else(|| panic!("{applied}")).to_string();

    // Started again by what kept it, the first one having ended.
    daemon.until("is running at").await;
    assert!(!alive(first), "the daemon that asked has ended");
    assert_ne!(kept_by(keeper), first);

    // The run goes on there: what comes after the step is said.
    let mut follow =
        command_in(&daemon.dir, &daemon.url, &["events", "--topics", "errand.after", "--follow"]);
    let mut follow = follow.stdout(std::process::Stdio::piped()).kill_on_drop(true).spawn().unwrap();
    let mut lines = BufReader::new(follow.stdout.take().unwrap()).lines();
    soon("what comes after the step to be said", async {
        while let Some(line) = lines.next_line().await.unwrap() {
            if line.contains("errand.after") {
                return;
            }
        }
        panic!("the events ended with nothing after the step");
    })
    .await;
    drop(follow);
    let ran: Value = serde_json::from_str(&daemon.run(&["run", &run, "--json"]).await.ok()).unwrap();
    let step =
        ran["steps"].as_array().unwrap().iter().find(|one| one["step"] == "restarted").cloned().unwrap();
    assert_eq!(
        (step["status"].as_str(), step["output"]["came"].as_str()),
        (Some("ok"), Some("restarted")),
        "{ran}"
    );
    let events =
        daemon.run(&["events", "--topics", "system.started,system.restarting", "--limit", "10"]).await.ok();
    let count = |topic: &str| events.lines().filter(|line| line.contains(topic)).count();
    assert_eq!((count("system.started"), count("system.restarting")), (2, 1), "{events}");
    // And what keeps it says why it started it again.
    assert!(daemon.said().contains("as a step asked"), "{}", daemon.said());
    daemon.stop().await;
}

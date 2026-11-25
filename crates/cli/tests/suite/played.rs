//! Playing a blueprint to see what it does: a daemon started with
//! `--handed` says what each step was handed, `logs <run>` reads a run
//! step by step, and `down --at` stops the daemon as a step begins.
//!
//! Unix's alone: the daemon here is the real program as a process of its
//! own (`daemon::Daemon`), stopped by a signal.
#![cfg(unix)]

use std::process::Stdio;

use tokio::io::{AsyncBufReadExt, BufReader};

use crate::daemon::Daemon;
use crate::{command_in, soon};

const GREETER: &str = "\
blueprint: greeter
permissions: [blueprints]
params:
  who: { default: Ada }
state:
  greeting: \"\"
steps:
  greet: { do: set, with: { greeting: \"hello, ${ params.who }\" } }
  child:
    needs: [greet]
    call: hello
    with: { who: \"${ params.who }\", said: \"${ state.greeting }\" }
";

const HELLO: &str = "\
blueprint: hello
permissions: []
params:
  who: {}
  said: {}
steps:
  tell: { do: emit, with: { topic: hello.said, to: \"${ params.who }\", text: \"${ params.said }\" } }
";

/// The ids of an instance's runs, the newest first.
async fn runs_of(daemon: &Daemon, instance: &str) -> Vec<String> {
    let runs = daemon.run(&["runs", "--instance", instance, "--json"]).await.ok();
    let runs: serde_json::Value = serde_json::from_str(&runs).unwrap();
    runs["runs"].as_array().unwrap().iter().map(|run| run["id"].as_str().unwrap().to_string()).collect()
}

/// The id of the one instance of a blueprint there is.
async fn instance_of(daemon: &Daemon, blueprint: &str) -> String {
    let all = daemon.run(&["instances", "--all", "--json"]).await.ok();
    let all: serde_json::Value = serde_json::from_str(&all).unwrap();
    let found =
        all["instances"].as_array().unwrap().iter().find(|instance| instance["blueprint"] == blueprint);
    found.unwrap_or_else(|| panic!("no instance of {blueprint}: {all}"))["id"].as_str().unwrap().to_string()
}

/// Follows what the daemon says until a line has all of `words`.
async fn said(daemon: &Daemon, topics: &str, words: &[&str]) {
    let mut follow = command_in(&daemon.dir, &daemon.url, &["events", "--topics", topics, "--follow"])
        .stdout(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    let mut lines = BufReader::new(follow.stdout.take().unwrap()).lines();
    soon(&format!("the daemon to say {words:?}"), async {
        loop {
            let line = lines.next_line().await.unwrap().expect("the events ended");
            if words.iter().all(|word| line.contains(word)) {
                return;
            }
        }
    })
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_blueprint_played_in_a_factory_that_says_what_is_handed_is_read_step_by_step_stopped_at_a_step_and_gone_on_with()
 {
    let mut daemon = Daemon::start_told("serve", None, None, &["--handed"]).await;
    daemon
        .run(&["register", "--email", "ada@example.com", "--name", "Ada", "--password", "correct horse"])
        .await
        .ok();
    // Bare: what is played here is a blueprint of its own, and not the fleet.
    daemon.run(&["project", "create", "Shop", "--bare"]).await.ok();
    let own = daemon.dir.join("data/blueprints/projects/shop");
    std::fs::create_dir_all(&own).unwrap();
    std::fs::write(own.join(brand::blueprint_file("hello")), HELLO).unwrap();
    let greeter = daemon.file(&brand::blueprint_file("greeter"), GREETER);

    // The daemon is told where to stop before anything is played.
    let told = daemon.run(&["down", "--at", "tell", "--of", "hello"]).await.ok();
    assert!(told.contains("will stop as a step called `tell` of hello begins"), "{told}");
    // Whoever applies may get no answer: the daemon is cut off at the
    // step, and answers nobody after it.
    let _ = daemon.run(&["apply", &greeter, "--project", "shop", "--param", "who=Grace", "--no-wait"]).await;
    assert!(soon("the daemon to stop at the step", daemon.child.wait()).await.unwrap().success());

    // Started again on what it had, it goes on from the step it was cut
    // off at, and the play comes to its end.
    daemon.again().await;
    // (That its run finished is passed on and not kept; that the instance
    // is over is kept, and is read by whoever follows after it was said.)
    said(&daemon, "instance.finished", &["\"key\":\"greeter\"", "\"status\":\"done\""]).await;
    let events = daemon.run(&["events", "--topics", "hello.said,system.stopped", "--limit", "10"]).await.ok();
    let topics: Vec<&str> = events.lines().filter_map(|line| line.split_whitespace().nth(1)).collect();
    assert_eq!(
        topics,
        ["system.stopped", "hello.said"],
        "said once, and after the daemon was back:\n{events}"
    );

    // The run, step by step: what each was handed as `${ }` came to, what
    // it gave, and that it was taken up again.
    let greeter = instance_of(&daemon, "greeter").await;
    let hello = instance_of(&daemon, "hello").await;
    let run = daemon.run(&["logs", &runs_of(&daemon, &greeter).await[0]]).await.ok();
    assert_eq!(
        run,
        format!(
            "greet (ok)
  # handed: {{\"greeting\":\"hello, Grace\"}}
  gave: {{\"greeting\":\"hello, Grace\"}}
child (ok)
  # handed to hello as child: {{\"said\":\"hello, Grace\",\"who\":\"Grace\"}}
  # attempt 1 (resumed after a restart)
  # handed to hello as child: {{\"said\":\"hello, Grace\",\"who\":\"Grace\"}}
  gave: {{\"instance\":\"{hello}\"}}\n"
        )
    );
    let theirs = daemon.run(&["logs", &runs_of(&daemon, &hello).await[0]]).await.ok();
    // What it was handed is written as the step is taken up again. Whether
    // it had been written before the daemon went is not held: the daemon
    // is cut off as the step begins, and the line is written just after.
    let handed = "  # attempt 1 (resumed after a restart)\n  # handed: \
                  {\"text\":\"hello, Grace\",\"to\":\"Grace\",\"topic\":\"hello.said\"}\n";
    assert!(theirs.starts_with("tell (ok)\n") && theirs.contains(handed), "{theirs}");
    // One step alone is as it was.
    let one = daemon.run(&["logs", &runs_of(&daemon, &greeter).await[0], "greet"]).await.ok();
    assert_eq!(one, "# handed: {\"greeting\":\"hello, Grace\"}\n");

    daemon.run(&["down"]).await.ok();
    assert!(soon("the daemon to exit", daemon.child.wait()).await.unwrap().success());
}

#[tokio::test(flavor = "multi_thread")]
async fn down_told_a_blueprint_and_no_step_says_what_is_missing() {
    let daemon = Daemon::start("serve").await;
    let refused = daemon.run(&["down", "--of", "hello"]).await;
    assert_ne!(refused.code, 0);
    assert!(refused.err.contains("name the step too"), "{}", refused.err);
    daemon.run(&["down"]).await.ok();
}

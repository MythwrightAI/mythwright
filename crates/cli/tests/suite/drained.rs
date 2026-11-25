//! A daemon stopped once what it has in hand is done (`down --drain`),
//! with the real program as the daemon and a stand-in as its agent's.
//!
//! Unix's alone: the daemon here is the real program as a process of its
//! own (`daemon::Daemon`), which the rig stops by a signal where a test
//! leaves it running.
#![cfg(unix)]

use std::path::PathBuf;
use std::process::Stdio;

use crate::daemon::Daemon;
use crate::standin::{Hold, argv};
use crate::{command_in, soon};

/// A daemon whose agent `builder` says it was asked and waits to be let
/// go at `hold`, and whose `helper` says it was asked and answers at once.
async fn factory(hold: &Hold) -> (Daemon, Asked) {
    // Where each says it was asked, a folder of the test's own: the
    // daemon's is made as it starts, after the programs are said.
    let dir = crate::scratch("drained-asked");
    let (builder, helper) = (dir.join("builder"), dir.join("helper"));
    let (builder, helper) = (builder.to_str().unwrap(), helper.to_str().unwrap());
    let waits =
        argv(&["swallow", "append", builder, "asked", "held", &hold.address(), "builder", "say", "done"]);
    let answers = argv(&["swallow", "append", helper, "asked", "say", "swept"]);
    let (waits, answers) = (format!("writer={}", waits.join(" ")), format!("sweeper={}", answers.join(" ")));
    let told = ["--walls", "off", "--command", &waits, "--command", &answers];
    let daemon = Daemon::start_told("serve", None, None, &told).await;
    daemon
        .run(&["register", "--email", "ada@example.com", "--name", "Ada", "--password", "correct horse"])
        .await
        .ok();
    daemon.run(&["project", "create", "--bare", "Shop"]).await.ok();
    daemon.run(&["channel", "create", "shop", "--project", "shop"]).await.ok();
    for (name, adapter) in [("builder", "writer"), ("helper", "sweeper")] {
        let add = [
            "agent",
            "add",
            name,
            "--adapter",
            adapter,
            "--project",
            "shop",
            "--channel",
            "shop",
            "--no-wall",
        ];
        daemon.run(&add).await.ok();
    }
    (daemon, Asked(dir))
}

/// The folder each agent's program says it was asked in, taken away
/// with the test.
struct Asked(PathBuf);

impl Asked {
    /// How often the agent's program was asked.
    fn of(&self, agent: &str) -> usize {
        std::fs::read_to_string(self.0.join(agent)).unwrap_or_default().lines().count()
    }
}

impl Drop for Asked {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// `down --drain` begun, and left to wait.
fn draining(daemon: &Daemon, within: &str) -> tokio::process::Child {
    let mut down = command_in(&daemon.dir, &daemon.url, &["down", "--drain", "--within", within]);
    down.stdout(Stdio::piped()).stderr(Stdio::piped()).kill_on_drop(true).spawn().unwrap()
}

/// Waits until `status` says the daemon drains.
async fn says_it_drains(daemon: &Daemon) -> String {
    soon("`status` to say that it drains", async {
        loop {
            let said = daemon.run(&["status"]).await.ok();
            if said.contains("Draining to stop") {
                return said;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
}

/// The statuses of an agent's turns, the newest first.
async fn turns(daemon: &Daemon, agent: &str) -> Vec<String> {
    let found = daemon.run(&["--json", "agent", "turns", agent]).await.ok();
    let found: serde_json::Value = serde_json::from_str(&found).unwrap();
    found["turns"]
        .as_array()
        .unwrap()
        .iter()
        .map(|turn| turn["status"].as_str().unwrap().to_string())
        .collect()
}

#[tokio::test(flavor = "multi_thread")]
async fn a_drained_stop_waits_for_the_turn_in_hand_begins_nothing_new_and_stops_once_it_has_ended() {
    let hold = Hold::new().await;
    let (mut daemon, asked) = factory(&hold).await;
    daemon.run(&["post", "shop", "@builder add a cart"]).await.ok();
    let held = hold.came().await;
    assert_eq!(held.who, "builder");

    let down = draining(&daemon, "10m");
    let said = says_it_drains(&daemon).await;
    assert!(said.contains("builder's turn"), "{said}");
    // Nothing new is begun: what is said to an agent waits, queued, for
    // the next start; an apply is refused, saying why.
    daemon.run(&["post", "shop", "@helper sweep up"]).await.ok();
    soon("helper's turn to be queued", async {
        while turns(&daemon, "helper").await.is_empty() {
            tokio::task::yield_now().await;
        }
    })
    .await;
    // And so is putting a tree back on what it ran before, which is an apply too.
    for back in [&[][..], &["--back"][..]] {
        let refused = daemon.run(&[&["apply", "std/board", "--project", "shop"][..], back].concat()).await;
        assert_ne!(refused.code, 0);
        assert!(
            refused.err.contains("the daemon is draining to stop, and begins nothing new"),
            "{}",
            refused.err
        );
    }
    // It goes on while the turn is in hand.
    assert!(daemon.child.try_wait().unwrap().is_none(), "the daemon stopped with a turn in hand");

    held.go().await;
    let down = soon("`down --drain` to end", down.wait_with_output()).await.unwrap();
    let out = String::from_utf8_lossy(&down.stdout);
    assert!(down.status.success(), "{out}\n{}", String::from_utf8_lossy(&down.stderr));
    assert!(out.contains("It waits for:\n  builder's turn"), "{out}");
    assert!(out.contains("drained after"), "{out}");
    soon("the daemon to stop", daemon.child.wait()).await.unwrap();
    // What was said to helper was never begun, and builder answered once.
    assert_eq!((asked.of("builder"), asked.of("helper")), (1, 0));

    // Started again, what waited is begun.
    daemon.again().await;
    assert_eq!(turns(&daemon, "builder").await, ["done"]);
    soon("helper's turn to be done", async {
        while turns(&daemon, "helper").await != ["done"] {
            tokio::task::yield_now().await;
        }
    })
    .await;
    assert_eq!(asked.of("helper"), 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_drained_stop_past_its_bound_says_what_it_cut_off_and_what_becomes_of_each() {
    let hold = Hold::new().await;
    let (mut daemon, asked) = factory(&hold).await;
    daemon.run(&["post", "shop", "@builder add a cart"]).await.ok();
    let held = hold.came().await;

    let down = draining(&daemon, "1s");
    let down = soon("`down --drain` to end", down.wait_with_output()).await.unwrap();
    let out = String::from_utf8_lossy(&down.stdout);
    assert!(down.status.success(), "{out}");
    assert!(out.contains("stopped at its bound"), "{out}");
    assert!(out.contains("builder's turn"), "{out}");
    assert!(out.contains("it is begun again by the next start"), "{out}");
    soon("the daemon to stop", daemon.child.wait()).await.unwrap();
    drop(held);

    // As it said: begun again, and asked a second time.
    daemon.again().await;
    let again = hold.came().await;
    again.go().await;
    soon("builder's turn to be done", async {
        while turns(&daemon, "builder").await != ["done"] {
            tokio::task::yield_now().await;
        }
    })
    .await;
    assert_eq!(asked.of("builder"), 2);
}

// What a stop does to an agent's program is asked of its slot as the
// daemon stops, so a slot changed while its turn is in hand holds for that
// turn: every turn's program is started by its wrapper.

/// The one turn's program's number, from its files.
fn program(daemon: &Daemon) -> u32 {
    let run = daemon.dir.join("data").join("run");
    let pid = std::fs::read_dir(run).unwrap().flatten().map(|entry| entry.path()).find(|path| {
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        name.starts_with("turn.") && name.ends_with(".pid")
    });
    std::fs::read_to_string(pid.expect("its program has files")).unwrap().trim().parse().unwrap()
}

/// `builder` said again whole, going on without the daemon or not.
async fn outliving(daemon: &Daemon, outlives: bool) {
    let said = if outlives { "--outlives" } else { "--no-outlives" };
    let add = [
        "agent",
        "add",
        "builder",
        "--adapter",
        "writer",
        "--project",
        "shop",
        "--channel",
        "shop",
        "--no-wall",
    ];
    daemon.run(&[&add[..], &[said]].concat()).await.ok();
}

/// Stops the daemon as `down` does, and waits until it has.
async fn down(daemon: &mut Daemon) {
    daemon.run(&["down"]).await.ok();
    soon("the daemon to stop", daemon.child.wait()).await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_slot_turned_to_outlive_the_daemon_mid_turn_has_its_program_go_on_through_a_stop_and_answer_once() {
    let hold = Hold::new().await;
    let (mut daemon, asked) = factory(&hold).await;
    daemon.run(&["post", "shop", "@builder add a cart"]).await.ok();
    let held = hold.came().await;
    let pid = program(&daemon);
    outliving(&daemon, true).await;

    down(&mut daemon).await;
    assert!(crate::daemon::alive(pid), "its program went on");
    daemon.again().await;
    held.go().await;
    soon("builder's turn to be done", async {
        while turns(&daemon, "builder").await != ["done"] {
            tokio::task::yield_now().await;
        }
    })
    .await;
    assert_eq!(asked.of("builder"), 1, "asked once, by the daemon that is gone");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_slot_turned_not_to_outlive_the_daemon_mid_turn_has_its_program_ended_by_the_stop_and_begun_again()
{
    let hold = Hold::new().await;
    let (mut daemon, asked) = factory(&hold).await;
    outliving(&daemon, true).await;
    daemon.run(&["post", "shop", "@builder add a cart"]).await.ok();
    let held = hold.came().await;
    let pid = program(&daemon);
    outliving(&daemon, false).await;

    down(&mut daemon).await;
    soon("its program to be ended as the daemon stopped", async {
        while crate::daemon::alive(pid) {
            tokio::task::yield_now().await;
        }
    })
    .await;
    drop(held);
    daemon.again().await;
    hold.came().await.go().await;
    soon("builder's turn to be done", async {
        while turns(&daemon, "builder").await != ["done"] {
            tokio::task::yield_now().await;
        }
    })
    .await;
    assert_eq!(asked.of("builder"), 2);
}

//! What a program prints of a secret it was handed, with the real program
//! as the daemon.
//!
//! This is here and not with the tests of hosting because what stands
//! between such a program and the files it writes to is this program,
//! started under a word of its own, and in those tests "this program" is
//! the test: there the factory reads the program's output itself. Here it
//! is as it is for anybody: the value never reaches a file.
//!
//! Unix's alone: the daemon here is the real program as a process of its
//! own (`daemon::Daemon`), and the programs that print what they were
//! handed are lines of a Unix shell.
#![cfg(unix)]

use std::path::Path;

use serde_json::Value;

use crate::daemon::Daemon;
use crate::soon;

/// What the programs are handed, which must be in nothing that is kept.
const VALUE: &str = "sk_live_51Hx9f8e7d6c5b4a";
const MARK: &str = "[the secret `stripe`]";

/// Every file under `dir` that holds `text`. A link is no file of the
/// folder and is not followed: what it names is looked at where it is,
/// where that is under the folder. The one link a daemon makes that
/// leads out of its data is to its own program (`bin`), which is
/// nothing it keeps, and is 140 megabytes: read through the link each
/// time this was asked, it was most of what these tests took, three and
/// a half seconds of a processor each.
fn holding(dir: &Path, text: &str) -> Vec<String> {
    let mut found = Vec::new();
    for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let path = entry.path();
        let held = |path: &Path| {
            let bytes = std::fs::read(path).unwrap_or_default();
            bytes.windows(text.len()).any(|part| part == text.as_bytes())
        };
        match entry.file_type() {
            Ok(kind) if kind.is_dir() => found.extend(holding(&path, text)),
            Ok(kind) if kind.is_file() && held(&path) => found.push(path.display().to_string()),
            _ => {}
        }
    }
    found
}

/// Stops every instance of a daemon when a test is over, however it
/// ended. A test that starts a service stops the instance that owns it,
/// and then there is nothing left for this to do; one that fails before
/// it gets there would leave a program running that nobody ends, since a
/// daemon leaves its services running when it goes, on purpose.
struct Stops<'a>(&'a Daemon);

impl Drop for Stops<'_> {
    fn drop(&mut self) {
        let dir = &self.0.dir;
        let ask = |args: &[&str]| {
            let asked = std::process::Command::new(crate::BINARY)
                .args(args)
                .env_clear()
                .envs(crate::kept_on_windows())
                .env("PATH", std::env::var("PATH").unwrap_or_default())
                .env("HOME", dir)
                .env("XDG_CONFIG_HOME", dir.join("config"))
                .env("XDG_DATA_HOME", dir.join("share"))
                .env(brand::env_var("URL"), &self.0.url)
                .stdin(std::process::Stdio::null())
                .output();
            asked.map(|said| String::from_utf8_lossy(&said.stdout).into_owned()).unwrap_or_default()
        };
        let live: Value = serde_json::from_str(&ask(&["instances", "--json"])).unwrap_or_default();
        for instance in live["instances"].as_array().into_iter().flatten() {
            if let Some(id) = instance["id"].as_str() {
                // Said, so that what a failed test printed shows what was stopped for it.
                eprintln!(
                    "the test was over with {id} still there; asked to stop it: {}",
                    ask(&["stop", id]).trim()
                );
            }
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_step_and_a_service_that_print_their_secret_have_its_name_written_down_and_its_value_in_no_file() {
    let daemon = Daemon::start("serve").await;
    let _stops = Stops(&daemon);
    daemon
        .run(&["register", "--email", "ada@example.com", "--name", "Ada", "--password", "correct horse"])
        .await
        .ok();
    let key = daemon.file("key.txt", VALUE);
    daemon.run(&["secret", "set", "stripe", "--file", &key]).await.ok();
    // The file it was read from is whoever set it's own, and is not the factory's keeping.
    std::fs::remove_file(&key).unwrap();

    // A step that prints its key and complains with it, one that fails with
    // it in its last line, and a service that prints it and says it in an
    // event of its own.
    let text = r#"blueprint: leaks
permissions: [fleet, shell, secrets]
env:
  STRIPE_KEY: { secret: stripe }
steps:
  pay:
    do: shell
    with:
      run: |
        echo "paid with $STRIPE_KEY"
        echo "warned: $STRIPE_KEY" >&2
  refused:
    needs: [pay]
    do: shell
    allow_failure: true
    with:
      cmd: [/bin/sh, -c, 'echo "refused: $STRIPE_KEY is not a key" >&2; exit 3']
  serve:
    needs: [refused]
    do: service.start
    with:
      name: checkout
      restart: never
      run: |
        echo "serving with $STRIPE_KEY"
        echo "::event {\"topic\": \"shop.paid\", \"with\": \"$STRIPE_KEY\"}"
        exec /bin/sleep 300
outputs:
  said: ${ steps.pay.out.stdout }
"#;
    let file = daemon.file(&brand::blueprint_file("leaks"), text);
    let applied = daemon.run(&["apply", &file, "--json"]).await.ok();
    assert!(!applied.contains(VALUE), "{applied}");
    let applied: Value = serde_json::from_str(&applied).unwrap();
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
    assert_eq!(applied["finished"]["outputs"]["said"], format!("paid with {MARK}"), "{applied}");

    // The run, read step by step: what each printed and gave, and why one failed.
    let run = applied["run"].as_str().unwrap();
    let logs = daemon.run(&["logs", run]).await.ok();
    for said in ["paid with", "warned:"] {
        assert!(logs.contains(&format!("{said} {MARK}")), "{logs}");
    }
    assert!(logs.contains(&format!("refused: {MARK} is not a key")), "{logs}");
    assert!(!logs.contains(VALUE), "{logs}");
    let shown = daemon.run(&["run", run, "--json"]).await.ok();
    assert!(shown.contains(&format!("refused: {MARK} is not a key")) && !shown.contains(VALUE), "{shown}");

    // The service's log is a file it goes on writing to by itself.
    let kept = daemon.dir.join("data");
    let serving = format!("serving with {MARK}");
    soon("the service to have said what it serves with", async {
        while holding(&kept, &serving).is_empty() {
            tokio::task::yield_now().await;
        }
    })
    .await;
    // What it said as an event of its own, and every other event there is.
    let events = soon("the service's own event", async {
        loop {
            let events = daemon.run(&["events", "--topics", "**", "--limit", "10000", "--json"]).await.ok();
            if events.contains("shop.paid") {
                return events;
            }
            tokio::task::yield_now().await;
        }
    })
    .await;
    assert!(events.contains(MARK) && !events.contains(VALUE), "{events}");

    // And nothing the factory keeps, or the command line beside it, holds
    // the value as it is: where it is kept it is sealed.
    daemon.run(&["stop", applied["instance"].as_str().unwrap()]).await.ok();
    assert_eq!(holding(&daemon.dir, VALUE), Vec::<String>::new());
    assert!(!holding(&kept, MARK).is_empty(), "the looking would have found it");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_service_handed_a_secret_is_stopped_when_its_program_has_ended_and_its_last_words_are_kept() {
    let daemon = Daemon::start("serve").await;
    let _stops = Stops(&daemon);
    daemon
        .run(&["register", "--email", "ada@example.com", "--name", "Ada", "--password", "correct horse"])
        .await
        .ok();
    let key = daemon.file("key.txt", VALUE);
    daemon.run(&["secret", "set", "stripe", "--file", &key]).await.ok();
    std::fs::remove_file(&key).unwrap();
    // One that takes a moment to put its things away when it is asked to
    // stop, and says its key as it goes. What stands between it and its
    // log is asked to stop in the same breath, and has to stay for that.
    let (up, away) = (daemon.dir.join("up"), daemon.dir.join("put-away"));
    let text = format!(
        "blueprint: tidy\npermissions: [fleet, shell, secrets]\nsteps:\n  start:\n    do: service.start\n    \
         env: {{ STRIPE_KEY: {{ secret: stripe }} }}\n    with:\n      name: tidy\n      restart: never\n      \
         cmd: [/bin/sh, -c, 'trap \"sleep 0.3; echo away with $STRIPE_KEY; echo away > {away}; exit 0\" TERM; \
         echo up > {up}; i=0; while [ $i -lt 6000 ]; do i=$((i+1)); sleep 0.05 & wait $!; done']\n",
        away = away.display(),
        up = up.display()
    );
    let file = daemon.file(&brand::blueprint_file("tidy"), &text);
    let applied = daemon.run(&["apply", &file, "--json"]).await.ok();
    let applied: Value = serde_json::from_str(&applied).unwrap();
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
    soon("the program to be up", async {
        while !up.exists() {
            tokio::task::yield_now().await;
        }
    })
    .await;

    daemon.run(&["stop", applied["instance"].as_str().unwrap()]).await.ok();
    soon("the service to be said to have stopped", async {
        loop {
            let events = daemon.run(&["events", "--topics", "service.stopped", "--json"]).await.ok();
            if events.contains("tidy") {
                return;
            }
            tokio::task::yield_now().await;
        }
    })
    .await;
    // By the time it is said to have stopped, it has: what it does on its
    // way out is done, and what it said on its way out is in its log,
    // without the key.
    assert!(away.exists(), "it was given the time to put its things away");
    let last = format!("away with {MARK}");
    assert!(!holding(&daemon.dir.join("data"), &last).is_empty(), "its last words were kept");
    assert_eq!(holding(&daemon.dir, VALUE), Vec::<String>::new());
}

#[tokio::test(flavor = "multi_thread")]
async fn the_owner_is_told_where_a_secret_is_named_and_what_still_names_one_that_was_taken_away() {
    let daemon = Daemon::start("serve").await;
    let _stops = Stops(&daemon);
    daemon
        .run(&["register", "--email", "ada@example.com", "--name", "Ada", "--password", "correct horse"])
        .await
        .ok();
    let key = daemon.file("key.txt", VALUE);
    daemon.run(&["secret", "set", "stripe", "--file", &key]).await.ok();
    daemon.run(&["agent", "add", "builder", "--secret", "PAY_KEY=stripe"]).await.ok();

    let listed = daemon.run(&["secret", "list", "--where"]).await.ok();
    assert!(listed.contains("stripe") && listed.contains("by Ada"), "{listed}");
    assert!(listed.contains("agent builder (PAY_KEY)"), "{listed}");
    // Without being asked where, it says which there are and no more.
    let plain = daemon.run(&["secret", "list"]).await.ok();
    assert!(plain.contains("stripe") && !plain.contains("builder"), "{plain}");

    let removed = daemon.run(&["secret", "remove", "stripe"]).await.ok();
    assert!(removed.contains("stripe is gone"), "{removed}");
    assert!(removed.contains("It is still named by agent builder (PAY_KEY)"), "{removed}");
    let listed = daemon.run(&["secret", "list", "--where"]).await.ok();
    assert!(listed.contains("not set") && listed.contains("agent builder (PAY_KEY)"), "{listed}");
}

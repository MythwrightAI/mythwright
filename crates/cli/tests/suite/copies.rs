//! A copy of one factory, to start another from, as a person asks for it:
//! a snapshot from the factory to copy, and `--from` to the new one.
//!
//! Every command here says which factory it means, by `--env`.
//!
//! Unix's alone: its factories are the real program as processes of their
//! own (`profiles::Home`), each stopped by a signal, and a link is
//! made as Unix makes one.
#![cfg(unix)]

use crate::daemon::signal;
use crate::profiles::{Home, own_address, path};
use crate::soon;

const SIGN_IN: [&str; 5] = ["login", "--email", "ada@example.com", "--password", "correct horse"];

/// A `prod` factory with something in it, and a snapshot of it: where the
/// snapshot is, and what asking for it printed.
async fn snapshot_of_prod(home: &Home) -> (std::path::PathBuf, String) {
    home.register("prod", "Ada").await;
    home.run(&["--profile", "prod", "project", "create", "--bare", "Riverside Shop"]).await.ok();
    home.run(&["--profile", "prod", "apply", "std/board", "--project", "riverside-shop"]).await.ok();
    home.run(&["--profile", "prod", "board", "add", "riverside-shop", "Ship", "it"]).await.ok();
    home.run(&["--profile", "prod", "channel", "create", "general"]).await.ok();
    home.run(&["--profile", "prod", "post", "general", "said", "in", "prod"]).await.ok();
    let to = home.0.join("prod-copy");
    let param = format!("to={}", to.display());
    let asked = home.run(&["--profile", "prod", "apply", "std/snapshot", "--param", &param]).await.ok();
    (to, asked)
}

#[tokio::test(flavor = "multi_thread")]
async fn a_factory_is_copied_by_asking_it_for_a_snapshot_and_starting_another_from_that() {
    let home = Home::new("copy");
    let cli = brand::CLI;
    let prod = home.start(&["--profile", "prod"]).await;
    let (to, asked) = snapshot_of_prod(&home).await;
    assert!(asked.contains(" ok in ") && asked.contains("\"snapshot\":\"snap_"), "{asked}");
    assert!(to.join("snapshot.json").is_file() && to.join("database.db").is_file());

    // The new factory says what it is a copy of, what did not come, and
    // what to do next.
    // It is stopped and started again where it was, so where that is, is
    // the test's own to say.
    let (_own, listen) = own_address();
    let mut dev = home.start(&["--profile", "dev2", "--from", path(&to), "--listen", &listen]).await;
    dev.until("Nothing is running here").await;
    let said = dev.seen.join("\n");
    assert!(said.contains("  This factory was started from a copy of the `prod` factory"), "{said}");
    assert!(
        said.contains(&format!(
            "sign in again with the password you have there (`{cli} --profile dev2 login`)"
        )),
        "{said}"
    );
    assert!(said.contains(&format!("`{cli} --profile dev2 apply std/board --project <project>`")), "{said}");
    assert_ne!(dev.url, prod.url);
    assert!(home.data_of("dev2").join("journal.db").is_file());
    assert!(home.data_of("dev2").join("projects/riverside-shop/README.md").is_file());

    // Nobody is signed in to it; the same person signs in with the
    // password they have, and finds what the other held, with nothing running.
    let nobody = home.run(&["--profile", "dev2", "whoami"]).await;
    assert_eq!(nobody.code, 1, "{}", nobody.out);
    assert!(nobody.err.contains(&format!("sign in first (`{cli} --profile dev2 login`)")), "{}", nobody.err);
    home.run(&[&["--profile", "dev2"], &SIGN_IN[..]].concat()).await.ok();
    assert!(home.run(&["--profile", "dev2", "whoami"]).await.ok().starts_with("Ada"));
    assert!(home.run(&["--profile", "dev2", "read", "general"]).await.ok().contains("said in prod"));
    assert!(home.run(&["--profile", "dev2", "board", "riverside-shop"]).await.ok().contains("Ship it"));
    assert_eq!(home.run(&["--profile", "dev2", "instances"]).await.ok().trim(), "nothing is running");
    let log = home.run(&["--profile", "dev2", "events", "--topics", "system.copied"]).await.ok();
    assert_eq!(log.lines().filter(|line| line.contains("system.copied")).count(), 1, "{log}");
    // What is done in the copy is done in the copy.
    home.run(&["--profile", "dev2", "apply", "std/board", "--project", "riverside-shop"]).await.ok();
    home.run(&["--profile", "dev2", "board", "add", "riverside-shop", "Tried", "in", "dev"]).await.ok();
    assert!(!home.run(&["--profile", "prod", "board", "riverside-shop"]).await.ok().contains("Tried in dev"));
    assert!(
        home.run(&["--profile", "prod", "whoami"]).await.ok().starts_with("Ada"),
        "still signed in there"
    );

    // It is a factory now. Nothing is copied into it again, and whatever
    // keeps a daemon running does not go on trying; started as any
    // factory is, it is where it was and says no more about the copy.
    dev.stop().await;
    for mode in ["serve", "up"] {
        let refused = home.command(&[mode, "--profile", "dev2", "--from", path(&to)]).output();
        let refused = soon("the daemon to refuse", refused).await.unwrap();
        assert_eq!(refused.status.code(), Some(2), "{mode}");
        let said = String::from_utf8_lossy(&refused.stderr);
        assert!(
            said.contains("already holds a factory, and a copy is only ever put where there is none"),
            "{said}"
        );
    }
    let again = home.start(&["--profile", "dev2"]).await;
    assert_eq!(again.url, dev.url);
    assert!(!again.seen.iter().any(|line| line.contains("started from a copy")), "{:?}", again.seen);
    assert!(home.run(&["--profile", "dev2", "board", "riverside-shop"]).await.ok().contains("Tried in dev"));
}

#[tokio::test(flavor = "multi_thread")]
async fn starting_from_a_snapshot_is_refused_without_a_name_under_the_same_name_and_from_what_is_not_one() {
    let home = Home::new("copy-refused");
    let _prod = home.start(&["--profile", "prod"]).await;
    let (to, _) = snapshot_of_prod(&home).await;
    let nothing = home.0.join("nothing-here");
    let other = home.0.join("elsewhere");

    let refusals: [(Vec<&str>, &str); 3] = [
        (vec!["--from", path(&to), "--data", path(&other)], "the new one needs a name of its own"),
        (
            vec!["--profile", "prod", "--from", path(&to), "--data", path(&other)],
            "cannot be called `prod` too",
        ),
        (vec!["--profile", "dev2", "--from", path(&nothing)], "is not a snapshot of a factory"),
    ];
    for (args, why) in refusals {
        for mode in ["serve", "up"] {
            let refused = home.command(&[&[mode], &args[..]].concat()).output();
            let refused = soon("the daemon to refuse", refused).await.unwrap();
            assert_eq!(refused.status.code(), Some(2), "{mode} {args:?}");
            let said = String::from_utf8_lossy(&refused.stderr);
            assert!(said.starts_with("error: ") && said.contains(why), "{mode} {args:?}: {said}");
        }
    }
    assert!(!other.exists() && !home.data_of("dev2").exists(), "and nothing was made for any of them");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_copy_kept_running_is_copied_once_however_often_its_daemon_is_started_again() {
    let home = Home::new("copy-up");
    let _prod = home.start(&["--profile", "prod"]).await;
    let (to, _) = snapshot_of_prod(&home).await;

    // Its daemon is started again where it was, so where that is, is the
    // test's own to say.
    let (_own, listen) = own_address();
    let mut dev = home.start_as("up", &["--profile", "dev2", "--from", path(&to), "--listen", &listen]).await;
    dev.until("Nothing is running here").await;
    home.run(&[&["--profile", "dev2"], &SIGN_IN[..]].concat()).await.ok();
    home.run(&["--profile", "dev2", "board", "add", "riverside-shop", "Tried", "in", "dev"]).await.ok();

    // The daemon dies. What keeps it running starts it again on the
    // factory that is there now, and does not ask for the copy a second
    // time: that would be refused, and nothing would be running.
    let supervisor = dev.child.id().unwrap();
    let children =
        std::process::Command::new("pgrep").args(["-P", &supervisor.to_string()]).output().unwrap();
    let daemon: u32 = String::from_utf8_lossy(&children.stdout)
        .trim()
        .parse()
        .expect("the supervisor has one child: the daemon");
    signal(daemon, "-KILL");
    dev.until("is running at").await;
    dev.until("  client  ").await;
    let board = home.run(&["--profile", "dev2", "board", "riverside-shop"]).await.ok();
    assert!(board.contains("Ship it") && board.contains("Tried in dev"), "{board}");
    let log =
        home.run(&["--profile", "dev2", "events", "--topics", "system.copied,system.started"]).await.ok();
    // (The journal came with the copy, the other factory's starts among it.)
    let count =
        |topic: &str| log.lines().filter(|line| line.contains(topic) && line.contains("dev2")).count();
    assert_eq!((count("system.copied"), count("system.started")), (1, 2), "{log}");

    signal(supervisor, "-TERM");
    assert!(soon("the supervisor to exit", dev.child.wait()).await.unwrap().success());
}

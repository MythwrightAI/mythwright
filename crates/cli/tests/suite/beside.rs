//! A factory has one daemon, and one thing that keeps it running: what a
//! second `up` is told beside either, and what becomes of a daemon whose
//! address is something else's (`api::folder`).
//!
//! Before this a second `up` started a daemon that could not listen,
//! again and again for as long as it was left, and took the address when
//! the factory was stopped on purpose.
//!
//! Unix's alone, as the daemons of `daemon.rs` are, which these are.
#![cfg(unix)]

use std::path::Path;

use crate::daemon::{Daemon, signal};
use crate::{Ended, Ran, command_in, scratch, soon};

/// What a refused daemon, and what would have kept it running, end with.
const REFUSED: i32 = 3;

/// `up` asked for the factory at `data`, listening at `listen`, by
/// somebody in the folder `home`: what it said and how it ended. One
/// that was not refused would go on for ever, and is ended with the test.
async fn up(home: &Path, data: &Path, listen: &str) -> Ran {
    let mut command = command_in(
        home,
        &format!("http://{listen}"),
        &["up", "--data", data.to_str().unwrap(), "--listen", listen, "--adapter", "builtin"],
    );
    command.env("JOURNAL_DISK", "unwaited").kill_on_drop(true);
    let output = soon("a second `up` to end", command.output()).await.unwrap();
    Ran {
        code: output.status.code().unwrap_or(-1),
        out: String::from_utf8_lossy(&output.stdout).into_owned(),
        err: String::from_utf8_lossy(&output.stderr).into_owned(),
    }
}

/// The daemon a keeper is keeping: its one child.
fn kept_by(keeper: u32) -> u32 {
    let children = std::process::Command::new("pgrep").args(["-P", &keeper.to_string()]).output().unwrap();
    String::from_utf8_lossy(&children.stdout)
        .trim()
        .parse()
        .expect("what keeps a daemon has one child: the daemon")
}

#[tokio::test(flavor = "multi_thread")]
async fn a_second_up_beside_a_daemon_that_is_kept_running_says_where_the_factory_is_and_ends() {
    let mut daemon = Daemon::start("up").await;
    let data = daemon.dir.join("data");
    let listen = daemon.url.trim_start_matches("http://").to_string();
    let keeper = daemon.child.id().unwrap();
    let first = kept_by(keeper);

    // The factory's first keeper, started before the factory had a
    // folder: the daemon the second one starts is told where the factory
    // is being served, and is not started again.
    let second = up(&daemon.dir, &data, &listen).await;
    assert_eq!(second.code, REFUSED, "{}{}", second.out, second.err);
    let running = format!("the factory at {} is already running, at {}", data.display(), daemon.url);
    assert!(second.err.contains(&running), "{}", second.err);
    assert!(
        second.err.contains(&format!("`{} --url {} down` stops it", brand::CLI, daemon.url)),
        "{}",
        second.err
    );
    assert!(second.err.contains("it is not started again"), "{}", second.err);
    assert!(!second.out.contains("is running at"), "nothing was started: {}", second.out);

    // The same told another address: it is the factory that is served
    // already, wherever a second daemon of it would listen.
    let elsewhere = up(&daemon.dir, &data, "127.0.0.1:0").await;
    assert_eq!(elsewhere.code, REFUSED, "{}{}", elsewhere.out, elsewhere.err);
    assert!(elsewhere.err.contains(&running), "{}", elsewhere.err);

    // A daemon that dies is still started again, by what kept it, which
    // the folder knows from then on: a second is told so at once, and
    // starts nothing.
    signal(first, "-KILL");
    daemon.until("is running at").await;
    let again = kept_by(keeper);
    assert_ne!(again, first);
    let second = up(&daemon.dir, &data, &listen).await;
    assert_eq!(second.code, REFUSED, "{}{}", second.out, second.err);
    assert!(second.err.contains(&running), "{}", second.err);
    assert!(
        second.err.contains(&format!("is kept running by another `{} up`", brand::CLI)),
        "{}",
        second.err
    );
    assert_eq!(kept_by(keeper), again, "the daemon there was is the one there is");
    assert!(
        daemon.run(&["status"]).await.ok().contains(" CPUs, "),
        "the owner its first start made is signed in here"
    );

    // Stopped on purpose, it is stopped: nothing is left to bring it back.
    assert!(daemon.run(&["down"]).await.ok().starts_with("the daemon is stopping"));
    assert!(soon("what kept it running to exit", daemon.child.wait()).await.unwrap().success());
    assert_eq!(daemon.run(&["status"]).await.code, 1, "nothing is listening any more");
    assert_eq!(api::folder::served(&data), None, "and nothing has its folder");
}

#[tokio::test(flavor = "multi_thread")]
async fn an_up_told_an_address_something_else_has_says_that_it_is_not_this_factorys_and_ends() {
    let home = scratch("address-taken");
    let _ended = Ended(&home);
    let data = home.join("data");
    // Something that is no factory, listening for as long as the test goes on.
    let taken = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let listen = taken.local_addr().unwrap().to_string();

    let refused = up(&home, &data, &listen).await;
    assert_eq!(refused.code, REFUSED, "{}{}", refused.out, refused.err);
    assert!(refused.err.contains(&format!("error: cannot listen on {listen}: ")), "{}", refused.err);
    assert!(refused.err.contains("Something that is not this factory has the address"), "{}", refused.err);
    assert!(refused.err.contains("`--listen 127.0.0.1:<port>`"), "what to do is said: {}", refused.err);
    // Once, and not again for as long as it was left.
    assert_eq!(refused.err.matches("cannot listen on").count(), 1, "{}", refused.err);
    assert!(refused.err.contains("it is not started again"), "{}", refused.err);
    assert!(!refused.err.contains("starting it again"), "{}", refused.err);
    assert!(!data.exists(), "and nothing was made for a factory that was not started");
    let _ = std::fs::remove_dir_all(&home);
}

/// What an `up` does beside another that keeps the factory's daemon,
/// with the test as that other: it holds what a keeper holds, for as
/// long as it says, so that each order is made and none is waited for.
#[tokio::test(flavor = "multi_thread")]
async fn an_up_beside_another_ends_where_that_ones_daemon_is_up_and_waits_where_it_is_not() {
    let mut daemon = Daemon::start("serve").await;
    let data = daemon.dir.join("data");
    let listen = daemon.url.trim_start_matches("http://").to_string();
    let keeper = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(data.join("keeper"))
        .unwrap();
    keeper.lock().unwrap();
    let kept = format!("is kept running by another `{} up`", brand::CLI);

    // Its daemon is up: this one is not needed, says where the factory
    // is and how it is stopped, and starts nothing.
    let second = up(&daemon.dir, &data, &listen).await;
    assert_eq!(second.code, REFUSED, "{}{}", second.out, second.err);
    assert!(second.err.contains(&format!("is already running, at {}", daemon.url)), "{}", second.err);
    assert!(second.err.contains(&kept), "{}", second.err);
    assert!(
        second.err.contains(&format!("`{} --url {} down` stops it", brand::CLI, daemon.url)),
        "{}",
        second.err
    );

    // Its daemon is not up: stopping, or about to be started again,
    // which nothing here says. This one waits, saying what for, and
    // ends when the other's daemon is up.
    let waits = "Waiting to see which";
    // Told when what it says is written down, and looking where the
    // system will not say: it is two seconds before an `up` says what it
    // waits for, and giving way and reading again all the while was the
    // whole work of a processor for as long, twice.
    let waiting = |daemon: &Daemon, times: usize| {
        let said = daemon.dir.join("said-on-stderr");
        async move {
            let watch = hosting::Watch::new();
            let mut written = watch.files(&[&said]).or_looking();
            soon("an `up` beside another to say what it waits for", async {
                while std::fs::read_to_string(&said).unwrap_or_default().matches(waits).count() < times {
                    written.next().await;
                }
            })
            .await;
            watch.close();
        }
    };
    daemon.stop().await;
    let mut beside = command_in(
        &daemon.dir,
        &daemon.url,
        &["up", "--data", data.to_str().unwrap(), "--listen", &listen, "--adapter", "builtin"],
    );
    let said = std::fs::OpenOptions::new().append(true).open(daemon.dir.join("said-on-stderr")).unwrap();
    beside
        .env("JOURNAL_DISK", "unwaited")
        .kill_on_drop(true)
        .stderr(said)
        .stdout(std::process::Stdio::null());
    let mut beside = beside.spawn().unwrap();
    waiting(&daemon, 1).await;
    daemon.begin("serve");
    daemon.until("is running at").await;
    let ended = soon("the `up` that waited to end", beside.wait()).await.unwrap();
    assert_eq!(ended.code(), Some(REFUSED), "{}", daemon.said());
    assert!(daemon.said().contains(&kept), "{}", daemon.said());

    // And goes on when the other has ended: asked to stop and then
    // started is how a factory is started again, and the second of
    // those does not fail for coming before the first is over.
    daemon.stop().await;
    daemon.begin("up");
    waiting(&daemon, 2).await;
    keeper.unlock().unwrap();
    daemon.until("is running at").await;
    assert!(
        daemon.run(&["status"]).await.ok().contains(" CPUs, "),
        "the owner its first start made is signed in here"
    );
    assert!(daemon.run(&["down"]).await.ok().starts_with("the daemon is stopping"));
    assert!(soon("what kept it running to exit", daemon.child.wait()).await.unwrap().success());
}

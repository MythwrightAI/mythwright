//! The session `login` keeps, read by every command beside one that
//! writes it again.
//!
//! A command that was sent with no credential is told to sign in
//! (`sign in first (`login`)`), and that is what a command said once,
//! with a session that worked a moment before and after it. A command
//! takes the kept session from its file as it starts, and nothing it does
//! tells a file that is not there from one that is being written: so the
//! file is never anything but whole, the session before or the one after.

use crate::{Rig, run_in};

const SIGN_IN: [&str; 5] = ["login", "--email", "ada@example.com", "--password", "correct horse"];

/// Where the session with the daemon found by its address is kept, under
/// the rig's own settings. Only the test that follows the file as the
/// system tells it of each change reads it, and that one is Linux's.
#[cfg(target_os = "linux")]
fn kept(rig: &Rig) -> std::path::PathBuf {
    rig.dir.join("config").join(brand::ID).join("session.json")
}

/// The order itself, as the system tells it: the session's file is only
/// ever put in place whole, by a file moved over it, and is never taken
/// away, made empty or written into where it is. Each of those is a
/// moment in which a command reading it finds no session.
#[cfg(target_os = "linux")]
#[tokio::test(flavor = "multi_thread")]
async fn a_session_kept_again_is_put_in_place_whole_and_its_file_is_never_without_one() {
    use notify::event::{EventKind, ModifyKind, RenameMode};
    use notify::{RecursiveMode, Watcher};

    let rig = Rig::signed_in().await;
    let file = kept(&rig);
    assert!(file.is_file(), "registering keeps a session at {}", file.display());
    let folder = file.parent().unwrap().to_path_buf();

    let (tell, heard) = std::sync::mpsc::channel();
    let mut watcher = notify::recommended_watcher(tell).unwrap();
    watcher.watch(&folder, RecursiveMode::NonRecursive).unwrap();
    for _ in 0..3 {
        rig.run(&SIGN_IN).await.ok();
    }
    // The system says what happened to a folder in the order it happened:
    // once it has said this was made, it has said everything before it.
    let last = folder.join("last-of-them");
    std::fs::write(&last, "").unwrap();
    let mut said = Vec::new();
    loop {
        let event = heard
            .recv_timeout(std::time::Duration::from_secs(40))
            .expect("the folder to be told of")
            .unwrap();
        if event.paths.contains(&last) {
            break;
        }
        if event.paths.contains(&file) {
            said.push(event.kind);
        }
    }
    let moved_in = |kind: &EventKind| {
        matches!(
            kind,
            EventKind::Modify(ModifyKind::Name(RenameMode::To | RenameMode::Both | RenameMode::Any))
        )
    };
    assert!(said.iter().filter(|kind| moved_in(kind)).count() >= 3, "{said:?}");
    // Read as a file is read: a command that looked at it, or the system
    // saying it was opened and let go of, is no change to it.
    let changed =
        |kind: &EventKind| matches!(kind, EventKind::Create(_) | EventKind::Remove(_) | EventKind::Modify(_));
    let others: Vec<_> = said.iter().filter(|kind| changed(kind) && !moved_in(kind)).collect();
    assert!(
        others.is_empty(),
        "the session's file was changed where it is, not put in place whole: {others:?}"
    );
    assert!(rig.run(&["whoami"]).await.ok().contains("Ada"));
}

/// The symptom, crowded: commands that read the kept session, side by
/// side with others that sign in again and so write it, each with a
/// session that works. None is told to sign in, and none fails to keep
/// what it signed in with.
#[tokio::test(flavor = "multi_thread")]
async fn commands_beside_a_sign_in_that_keeps_its_session_again_all_find_a_session() {
    let rig = Rig::signed_in().await;
    let mut failed = Vec::new();
    for _ in 0..6 {
        let mut ran = tokio::task::JoinSet::new();
        for nth in 0..12 {
            let (dir, url) = (rig.dir.clone(), rig.url.clone());
            let args: Vec<&str> = if nth % 4 == 0 { SIGN_IN.to_vec() } else { vec!["whoami"] };
            ran.spawn(async move { run_in(&dir, &url, &[], &args).await });
        }
        while let Some(done) = ran.join_next().await {
            let done = done.unwrap();
            if done.code != 0 {
                failed.push(done.err);
            }
        }
    }
    assert!(failed.is_empty(), "commands with a session that works failed:\n{}", failed.join("\n"));
}

/// Another way a kept session could read as none: the daemon stopped and
/// started again between two commands, as a chapter of a story does. The
/// sessions are the daemon's to keep on its disk and not in its memory,
/// so the command that comes first after it is back (the one the story
/// was refused, among them) is answered as the one before it was, each
/// time, by a daemon that is the real program as a process of its own.
#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn a_kept_session_is_taken_by_a_daemon_started_again_from_the_first_command_after_it() {
    let mut daemon = crate::daemon::Daemon::start("serve").await;
    daemon
        .run(&["register", "--email", "ada@example.com", "--name", "Ada", "--password", "correct horse"])
        .await
        .ok();
    for _ in 0..3 {
        daemon.again().await;
        let (events, me) = tokio::join!(
            daemon.run(&["events", "--topics", "slot.*", "--limit", "20"]),
            daemon.run(&["whoami"])
        );
        assert_eq!(events.code, 0, "{}\n{}", events.out, events.err);
        assert!(me.ok().contains("Ada"));
    }
}

/// What the story was refused for, made on purpose: two daemons signed
/// in to from one home by their addresses, the second after the first.
/// The session kept for the daemon found by its address is one, so the
/// second's took the first's place, and the first's next command went
/// with no credential. It is said for what it is, with the command that
/// signs in where it was asked.
#[tokio::test(flavor = "multi_thread")]
async fn a_session_kept_with_another_daemon_is_said_where_this_one_asks_for_a_sign_in() {
    let first = Rig::signed_in().await;
    assert!(first.run(&["events", "--topics", "slot.*", "--limit", "20"]).await.code == 0);
    let second = Rig::new().await;
    let register = ["register", "--email", "ada@example.com", "--name", "Ada", "--password", "correct horse"];
    run_in(&first.dir, &second.url, &[], &register).await.ok();

    let refused = first.run(&["events", "--topics", "slot.*", "--limit", "20"]).await;
    assert_ne!(refused.code, 0);
    let cli = brand::CLI;
    assert!(
        refused.err.contains(&format!(
            "the session kept on this machine is with {} and not {}",
            second.url, first.url
        )) && refused.err.contains(&format!("(`{cli} login --url {}`)", first.url)),
        "{}",
        refused.err
    );
}

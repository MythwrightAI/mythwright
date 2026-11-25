//! A program that was started to go on without the daemon, as the daemon
//! after it finds it.
//!
//! The daemon that started it is played here: a program is started, and
//! its number and the moment it was started are written where a wrapper
//! and its daemon would have written them. Whoever looks for it is another
//! daemon's code, given only the files.
//!
//! The program is the stand-in, the same on every system. Two things here
//! are Unix's alone and say so where they are: a look that is held at a
//! pipe which has a name among the files, and a program that is ended by
//! a signal, which is what tells one that was asked from one that was not.

use std::path::{Path, PathBuf};

use hosting::Watch;
use hosting::kept::{Files, Process};

use crate::soon;

fn folder(label: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("kept-{label}-{}", &kernel::id::secret()[..12]));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// A program an earlier daemon started under `files` in `dir`, which goes
/// on until what it reads is closed: as that daemon holds it, and what to
/// close. It waits for the daemon's word before it begins, as a wrapper
/// does.
///
/// Unix's: what it reads is held by a second hold on the same pipe, which
/// is a file's number there, and the look at it below waits at a pipe
/// with a name.
#[cfg(unix)]
fn left_at_work(dir: &Path, files: &Files) -> (Process, std::os::fd::OwnedFd) {
    use std::os::fd::AsFd;
    // It reads what it is handed to its end, which comes when the hold
    // on its pipe is let go.
    let child = tokio::process::Command::new(crate::STANDIN)
        .arg("swallow")
        .stdin(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let reads = child.stdin.as_ref().unwrap().as_fd().try_clone_to_owned().unwrap();
    std::fs::write(dir.join("turn.pid"), child.id().unwrap().to_string()).unwrap();
    (Process::started(child, files).unwrap(), reads)
}

/// Unix's alone: the look is held where it reads how the program ended by
/// that being a pipe with a name, which waits for whoever writes to it.
/// No other system has a file that does.
#[cfg(unix)]
#[tokio::test]
async fn a_program_that_ends_as_it_is_looked_for_is_there_with_how_it_ended_and_is_not_taken_for_gone() {
    let dir = folder("ends");
    let files = Files::new(&dir, "turn");
    let (mut theirs, reads) = left_at_work(&dir, &files);
    // How it ended is read from a pipe for this one look, so that the look
    // waits there until it is let go: what it finds of the program's
    // ending is decided here, and so is what happens meanwhile.
    let said = dir.join("turn.exit");
    assert!(std::process::Command::new("mkfifo").arg(&said).status().unwrap().success());
    let watch = Watch::new();
    let look = {
        let (files, watch) = (files.clone(), watch.clone());
        tokio::task::spawn_blocking(move || Process::found(&files, &watch).map(|found| found.is_there()))
    };
    // The look has come to how the program ended, and it has not ended.
    let pipe = said.clone();
    // Opened in a thread of its own: it waits for the look to come and
    // read, for as long as that takes, and a runtime waits for a thread
    // of its own as it is put away, so a look that never came would have
    // the test hang where it should fail.
    let (opened, looking) = tokio::sync::oneshot::channel();
    std::thread::spawn(move || {
        let _ = opened.send(std::fs::OpenOptions::new().write(true).open(pipe));
    });
    let looking = soon("the look to come to how the program ended", looking).await.unwrap().unwrap();
    // Now it ends, saying how, and is gone: all of it before the look goes on.
    drop(reads);
    soon("the program to end", theirs.wait()).await;
    std::fs::write(dir.join("turn.ended"), "0").unwrap();
    std::fs::rename(dir.join("turn.ended"), &said).unwrap();
    // The look is let go on having found nothing said, as it would have a
    // moment before the program said it.
    drop(looking);
    let there = soon("the look to be over", look).await.unwrap();
    assert_eq!(there, Some(true), "a program that ended having said how was taken for one that was cut off");
    // And how it ended is there for whoever takes it up.
    let mut found = Process::found(&files, &watch).unwrap();
    assert!(found.has_ended());
    assert_eq!(soon("how it ended to be read", found.wait()).await, Some(0));
    watch.close();
    let _ = std::fs::remove_dir_all(&dir);
}

/// A program started to go on without the daemon, which writes a file to
/// say it is at work and then does `then`: its command, and the file.
///
/// This program is the test's and cannot be started as the wrapper, so
/// what stands in for it is a thread of the test's process
/// (`Wraps::Here`). What the wrapper itself does (it waits to be told,
/// and starts nothing for a daemon that is gone first; it stays when it
/// is asked to stop) is held where this program is run:
/// `crates/cli/tests/suite/wrapped.rs`.
fn wrapped(dir: &Path, files: &Files, then: &str) -> (tokio::process::Command, PathBuf) {
    let at_work = dir.join("at-work");
    let program = [
        crate::STANDIN.into(),
        "write".into(),
        at_work.clone().into_os_string(),
        "there".into(),
        then.into(),
    ];
    (hosting::kept::wrapper(files, &[], &program).unwrap(), at_work)
}

fn files_in(dir: &Path) -> Files {
    Files::new(dir, "turn").wrapped(hosting::wrapped::Wraps::Here)
}

#[tokio::test]
async fn a_program_started_to_go_on_reads_what_it_was_handed_and_says_how_it_ended() {
    let dir = folder("told");
    let files = files_in(&dir);
    let (mut command, at_work) = wrapped(&dir, &files, "cat");
    hosting::kept::hand_in(&files, "Add a cart\n").unwrap();
    let mut process = hosting::kept::begin(&mut command, &files, &[]).unwrap();
    assert_eq!(soon("the program to end", process.wait()).await, Some(0));
    assert!(at_work.exists());
    assert_eq!(process.said().more(true), ["Add a cart"]);
    assert!(Process::found(&files, &Watch::new()).is_some_and(|found| found.has_ended()));
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn one_found_again_by_its_files_is_there_at_work_and_stopped_says_how_it_ended() {
    let dir = folder("found");
    let files = files_in(&dir);
    let (mut command, _) = wrapped(&dir, &files, "idle");
    let theirs = hosting::kept::begin(&mut command, &files, &[]).unwrap();
    // Another daemon's, given only the files.
    let watch = Watch::new();
    let mut found = Process::found(&files, &watch).unwrap();
    assert_eq!(found.pid(), theirs.pid());
    assert!(found.is_there() && !found.has_ended());
    // Asked to stop, the program ends as one asked does, and that is said.
    // What asks is a signal, and the code is the one a program ends with
    // that was sent it; where a stop is an ending and nothing is asked
    // (Windows), it has ended and how is said, whatever the code.
    soon("it to stop", found.stop()).await;
    let ended = soon("how it ended to be heard", found.wait()).await;
    #[cfg(unix)]
    assert_eq!(ended, Some(143));
    assert!(ended.is_some());
    assert!(found.is_there() && found.has_ended());
    watch.close();
    let _ = std::fs::remove_dir_all(&dir);
}

/// Unix's alone: that a program was ended without being asked is known by
/// the signal it was ended by, and where a stop is an ending (Windows)
/// nothing tells one stopped so from one that ended by itself.
#[cfg(unix)]
#[tokio::test]
async fn one_ended_without_being_asked_says_nothing_and_is_not_there_to_be_taken_up() {
    let dir = folder("ended");
    let files = files_in(&dir);
    let (mut command, _) = wrapped(&dir, &files, "idle");
    let mut theirs = hosting::kept::begin(&mut command, &files, &[]).unwrap();
    // The whole of it, wrapper and all, as the machine being started
    // again would end it.
    hosting::group::stop(theirs.pid(), true).unwrap();
    assert_eq!(soon("it to be gone", theirs.wait()).await, None);
    let found = Process::found(&files, &Watch::new()).unwrap();
    assert!(!found.has_ended(), "something said how a program ended that was ended with its wrapper");
    assert!(!found.is_there(), "a program that was cut off was taken for one to be taken up");
    let _ = std::fs::remove_dir_all(&dir);
}

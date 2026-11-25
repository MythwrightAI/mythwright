//! The wrapper: this program started in a program's place, to start it
//! and say how it ended (`hosting::wrapped`).
//!
//! This is here and not with the tests of hosting because the wrapper is
//! this program, and in those tests "this program" is the test's: there a
//! thread of the test's process stands in for it. Here it is the program
//! anybody runs, and the test is what a daemon is to it: it starts the
//! wrapper, writes down when, tells it, asks it to stop, and looks for it
//! again by its files as a daemon that is not its parent does.
//!
//! The programs wrapped here are lines of shell: they are the tests'
//! stand-ins for a user's program, and nothing of the factory's.
//!
//! Unix's alone: the programs wrapped are lines of a Unix shell, asked to
//! stop and killed by signals, and one waits at a pipe with a name.
#![cfg(unix)]

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use hosting::Watch;
use hosting::kept::{Files, Process};
use hosting::wrapped::Asked;

use crate::{BINARY, scratch, soon};

fn stand_in(does: &str, told: &[&Path]) -> Vec<OsString> {
    let mut program: Vec<OsString> = vec!["sh".into(), "-c".into(), does.into()];
    program.extend(told.iter().map(|path| path.as_os_str().to_owned()));
    program
}

/// A wrapper as a daemon starts one for a step or a service: the command,
/// with what the program prints going to `out` and `err` in `dir` and how
/// it ended to `exit`.
fn wrapper(dir: &Path, unsay: &[(&str, &str)], program: &[OsString]) -> tokio::process::Command {
    let unsay = unsay.iter().map(|(variable, said)| (variable.to_string(), said.to_string())).collect();
    wrapper_asked(
        dir,
        Asked { exit: dir.join("exit"), told: None, reads: None, unsay, script: false },
        program,
    )
}

/// The same, asked what `asked` says.
fn wrapper_asked(dir: &Path, asked: Asked, program: &[OsString]) -> tokio::process::Command {
    let mut command = hosting::wrapped::command(Path::new(BINARY), &asked, program);
    let file = |name: &str| std::fs::File::create(dir.join(name)).unwrap();
    // In the test's own folder, so that whatever it starts is of that
    // folder and is ended with the test (`crate::Ended`).
    command.stdout(file("out")).stderr(file("err")).current_dir(dir).kill_on_drop(true);
    command
}

fn read(dir: &Path, name: &str) -> String {
    std::fs::read_to_string(dir.join(name)).unwrap_or_default()
}

/// Waits, without sleeping, until the file holds the words.
async fn says(file: &Path, words: &str) {
    soon(&format!("{} to say `{words}`", file.display()), async {
        while !std::fs::read_to_string(file).is_ok_and(|said| said.contains(words)) {
            tokio::task::yield_now().await;
        }
    })
    .await;
}

/// A pipe a stand-in waits at until the test lets it go.
fn pipe(dir: &Path) -> PathBuf {
    let pipe = dir.join("held");
    assert!(std::process::Command::new("mkfifo").arg(&pipe).status().unwrap().success());
    pipe
}

fn let_go(pipe: &Path) {
    use std::io::Write;
    std::fs::OpenOptions::new().write(true).open(pipe).unwrap().write_all(b"go\n").unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_program_that_ends_well_and_one_that_ends_badly_are_each_said_to_have_ended_as_they_did() {
    let dir = scratch("wrapped-ends");
    let _ended = crate::Ended(&dir);
    let well = stand_in("echo done; echo careful >&2", &[]);
    let ended = soon("the wrapper to end", wrapper(&dir, &[], &well).status()).await.unwrap();
    assert_eq!(ended.code(), Some(0));
    assert_eq!((read(&dir, "exit").as_str(), read(&dir, "out").as_str()), ("0", "done\n"));
    assert_eq!(read(&dir, "err"), "careful\n");

    let badly = stand_in("echo no; exit 7", &[]);
    let ended = soon("the wrapper to end", wrapper(&dir, &[], &badly).status()).await.unwrap();
    // The wrapper ends as its program did, and says so where a daemon
    // that is not its parent looks.
    assert_eq!((ended.code(), read(&dir, "exit").as_str()), (Some(7), "7"));
    assert!(!dir.join("exit.saying").exists(), "what the code was written beside was left there");
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_wrapper_told_that_it_is_handed_a_users_script_has_the_systems_shell_read_it_whole() {
    let dir = scratch("wrapped-script");
    let _ended = crate::Ended(&dir);
    let asked = || Asked { exit: dir.join("exit"), script: true, ..Asked::default() };
    // Several lines, quotation marks of both kinds, and what a shell
    // makes of each: one word to the wrapper, and one to the shell.
    let script = "echo \"a  b\" 'c  d'\necho two | tr a-z A-Z\necho \"$0\" >&2; exit 3";
    let ended = soon("the wrapper to end", wrapper_asked(&dir, asked(), &[script.into()]).status()).await;
    assert_eq!((ended.unwrap().code(), read(&dir, "exit").as_str()), (Some(3), "3"));
    assert_eq!(read(&dir, "out"), "a  b c  d\nTWO\n");
    // The shell where a system keeps it, handed the script and nothing
    // after it.
    assert_eq!(read(&dir, "err"), "/bin/sh\n");

    // A script is one word. More than one is nothing a daemon wrote,
    // and nothing is started for it.
    let two = [OsString::from("echo one"), OsString::from("echo two")];
    let ended = soon("the wrapper to end", wrapper_asked(&dir, asked(), &two).status()).await.unwrap();
    assert_eq!((ended.code(), read(&dir, "out").as_str()), (Some(2), ""));
    assert!(read(&dir, "err").contains("is not a command to give it"), "{}", read(&dir, "err"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_program_that_is_not_there_is_said_to_have_ended_as_a_shell_says_it_and_why_is_written_down() {
    let dir = scratch("wrapped-absent");
    let _ended = crate::Ended(&dir);
    let absent = vec![OsString::from(dir.join("no-such-program"))];
    let ended = soon("the wrapper to end", wrapper(&dir, &[], &absent).status()).await.unwrap();
    assert_eq!((ended.code(), read(&dir, "exit").as_str()), (Some(127), "127"));
    let why = read(&dir, "err");
    assert!(why.starts_with("could not start `") && why.contains("no-such-program"), "{why}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_program_ended_by_a_signal_of_its_own_is_said_to_have_ended_with_what_a_shell_calls_it() {
    let dir = scratch("wrapped-signalled");
    let _ended = crate::Ended(&dir);
    // The program alone, and not its group: the wrapper is still there
    // to say so.
    let alone = stand_in("kill -KILL $$", &[]);
    let ended = soon("the wrapper to end", wrapper(&dir, &[], &alone).status()).await.unwrap();
    assert_eq!((ended.code(), read(&dir, "exit").as_str()), (Some(137), "137"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test(flavor = "multi_thread")]
async fn asked_to_stop_the_wrapper_stays_until_a_program_that_takes_its_time_has_ended_and_says_how() {
    let dir = scratch("wrapped-stays");
    let _ended = crate::Ended(&dir);
    let held = pipe(&dir);
    // Asked to stop, it says that it heard, and puts its things away for
    // as long as the test holds it there. What it was waiting on is its
    // own to end, and by what nothing takes in: asked in the instant
    // before that was started, the group was asked without it, and it
    // went on for its five minutes with nobody waiting for it (a shell
    // told to stop as it says it is ready left it five times in six);
    // and asked to stop as it is started, while it is still the shell,
    // it takes the asking as the shell does and goes on (one in fifty).
    let slow = stand_in(
        "trap 'echo asked; read go < \"$0\"; echo put away; kill -9 $! 2>/dev/null; exit 3' TERM; \
         echo ready; sleep 300 & wait $!",
        &[&held],
    );
    let mut child = wrapper(&dir, &[], &slow).spawn().unwrap();
    let leader = child.id().unwrap();
    says(&dir.join("out"), "ready").await;

    hosting::group::stop(leader, false).unwrap();
    says(&dir.join("out"), "asked").await;
    // The asking was said to the whole group, the wrapper with it, and
    // the wrapper is there: gone, the program would be taken for stopped.
    assert!(child.try_wait().unwrap().is_none(), "the wrapper went before its program had ended");
    assert!(!dir.join("exit").exists(), "how it ended was said before it had");

    let_go(&held);
    let ended = soon("the wrapper to end", child.wait()).await.unwrap();
    assert_eq!((ended.code(), read(&dir, "exit").as_str()), (Some(3), "3"));
    assert!(read(&dir, "out").ends_with("put away\n"), "{}", read(&dir, "out"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test(flavor = "multi_thread")]
async fn what_a_program_left_behind_it_is_ended_with_its_test_though_its_folder_is_gone_by_then() {
    let dir = scratch("wrapped-left");
    // A program that starts another and ends without it, as the one
    // above did when it was asked to stop a moment too soon.
    let leaves = stand_in("sleep 300 & echo $! > \"$0\"", &[&dir.join("left")]);
    let ended = soon("the wrapper to end", wrapper(&dir, &[], &leaves).status()).await.unwrap();
    assert_eq!(ended.code(), Some(0));
    let left: u32 = read(&dir, "left").trim().parse().unwrap();
    assert!(crate::daemon::alive(left));
    // And something that only names the folder, as somebody's own command
    // does who reads what a test left there: no program of this suite's,
    // and in a folder of its own.
    // (Something the shell does itself, so that it is the shell that
    // waits, started with the folder's name.)
    let mut reads = std::process::Command::new("sh")
        .args(["-c", "read line; exit 0"])
        .arg(&dir)
        .stdin(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    {
        // As every test here ends: its folder is taken away, and then
        // what was of the folder is ended, which is found all the same.
        let _ended = crate::Ended(&dir);
        let _ = std::fs::remove_dir_all(&dir);
    }
    soon("what was left behind to be gone", async {
        while crate::daemon::alive(left) {
            tokio::task::yield_now().await;
        }
    })
    .await;
    // What only named the folder was not the test's to end: it is there,
    // and goes when it is let go.
    assert!(reads.try_wait().unwrap().is_none(), "something that only named the folder was ended with it");
    drop(reads.stdin.take());
    assert!(reads.wait().unwrap().success());
}

#[tokio::test(flavor = "multi_thread")]
async fn what_a_program_prints_of_what_it_was_handed_reaches_its_files_as_the_name_and_never_the_value() {
    let dir = scratch("wrapped-unsaid");
    let _ended = crate::Ended(&dir);
    let prints =
        stand_in("echo \"my key is $REMOTE_TOKEN\"; echo \"refused: $REMOTE_TOKEN\" >&2; exit 4", &[]);
    let mut command = wrapper(&dir, &[("REMOTE_TOKEN", "[the secret `remote`]")], &prints);
    command.env("REMOTE_TOKEN", "tok-4f7a91c2d8e6");
    let ended = soon("the wrapper to end", command.status()).await.unwrap();
    // One process is the wrapper and what stands between the program and
    // its files: it says how the program ended once all of that is passed on.
    assert_eq!((ended.code(), read(&dir, "exit").as_str()), (Some(4), "4"));
    assert_eq!(read(&dir, "out"), "my key is [the secret `remote`]\n");
    assert_eq!(read(&dir, "err"), "refused: [the secret `remote`]\n");
    let _ = std::fs::remove_dir_all(&dir);
}

/// A wrapper as a daemon starts one for a program that is to go on
/// without it, whose program says it is at work and then does `then`:
/// the command, and the file that says so.
fn kept(dir: &Path, files: &Files, then: &str, told: &[&Path]) -> (tokio::process::Command, PathBuf) {
    let at_work = dir.join("at-work");
    let mut all = vec![at_work.as_path()];
    all.extend(told);
    let does = format!("echo there > \"$0\"; {then}");
    let mut command =
        hosting::kept::wrapper_by(Path::new(BINARY), files, &[], &stand_in(&does, &all)).unwrap();
    // In the test's own folder, as the wrapper of a step is: what it
    // starts is of that folder, and is ended with the test.
    command.current_dir(dir);
    (command, at_work)
}

#[tokio::test(flavor = "multi_thread")]
async fn a_program_is_started_only_once_the_moment_it_began_is_written_down_and_reads_what_it_was_handed() {
    let dir = scratch("wrapped-told");
    let _ended = crate::Ended(&dir);
    let files = Files::new(&dir, "turn");
    let (mut command, at_work) = kept(&dir, &files, "cat", &[]);
    hosting::kept::hand_in(&files, "Add a cart\n").unwrap();
    let child = command.spawn().unwrap();
    // The wrapper says which process it is, and waits.
    says(&dir.join("turn.pid"), &child.id().unwrap().to_string()).await;
    assert!(!at_work.exists(), "a program was started before its wrapper was told");
    let mut process = Process::started(child, &files).unwrap();
    assert_eq!(soon("the program to end", process.wait()).await, Some(0));
    assert!(at_work.exists());
    assert_eq!(process.said().more(true), ["Add a cart"]);
    assert!(Process::found(&files, &Watch::new()).is_some_and(|found| found.has_ended()));
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_daemon_gone_before_it_wrote_down_when_a_program_began_leaves_no_program_at_work() {
    let dir = scratch("wrapped-unsaid-moment");
    let _ended = crate::Ended(&dir);
    let files = Files::new(&dir, "turn");
    let (mut command, at_work) = kept(&dir, &files, "cat", &[]);
    let mut child = command.spawn().unwrap();
    // The daemon is killed here, before it has written anything down:
    // what it held of the wrapper is closed with it.
    drop(child.stdin.take());
    let ended = soon("the wrapper to end", child.wait()).await.unwrap();
    assert!(ended.success(), "{ended}");
    assert!(!at_work.exists(), "a program was started with nothing written down to find it by");
    // The daemon after it finds nothing to take up, and there is nothing
    // it would have had to stop.
    assert!(Process::found(&files, &Watch::new()).is_none());
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_wrapper_found_again_by_its_files_is_waited_for_by_a_daemon_that_is_not_its_parent() {
    let dir = scratch("wrapped-found");
    let _ended = crate::Ended(&dir);
    let held = pipe(&dir);
    let files = Files::new(&dir, "turn");
    let (mut command, at_work) = kept(&dir, &files, "read go < \"$1\"; echo finished; exit 5", &[&held]);
    let theirs = Process::started(command.spawn().unwrap(), &files).unwrap();
    says(&at_work, "there").await;

    // Another daemon's code, given only the files: it is no parent of the
    // wrapper, and hears of its end from what the wrapper writes.
    let watch = Watch::new();
    watch.folder(&dir);
    let mut found = Process::found(&files, &watch).unwrap();
    assert_eq!(found.pid(), theirs.pid());
    assert!(found.is_there() && !found.has_ended());
    let_go(&held);
    assert_eq!(soon("how it ended to be read from its files", found.wait()).await, Some(5));
    assert!(found.is_there() && found.has_ended());
    assert_eq!(found.said().more(true), ["finished"]);
    watch.close();
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test(flavor = "multi_thread")]
async fn one_killed_outright_wrapper_and_all_says_nothing_and_is_not_there_to_be_taken_up() {
    let dir = scratch("wrapped-killed");
    let _ended = crate::Ended(&dir);
    let files = Files::new(&dir, "turn");
    let (mut command, at_work) = kept(&dir, &files, "exec sleep 300", &[]);
    let mut theirs = Process::started(command.spawn().unwrap(), &files).unwrap();
    says(&at_work, "there").await;
    // The whole group, as the machine being started again ends it.
    hosting::group::stop(theirs.pid(), true).unwrap();
    assert_eq!(soon("the wrapper to be gone", theirs.wait()).await, None);
    let found = Process::found(&files, &Watch::new()).unwrap();
    assert!(!found.has_ended(), "something said how a program ended whose wrapper was ended with it");
    assert!(!found.is_there(), "a program that was cut off was taken for one to be taken up");
    // And a group that is gone was stopped: asking again is no failure.
    hosting::group::stop(theirs.pid(), true).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
}

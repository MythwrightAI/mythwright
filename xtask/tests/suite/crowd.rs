//! Crowding a test: what is asked, which test is meant, and what many
//! runs at once come to.
//!
//! The test program here is a stand-in, a script that answers as one
//! does: nothing is built. What is held is what a crowd done by hand got
//! wrong: a name that is no test's passed for a test that passed.

use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use xtask::crowd::{Asked, Came, Crowd, allowed, asked, held, kept, named, run, summary};

fn words(said: &[&str]) -> Vec<String> {
    said.iter().map(|word| word.to_string()).collect()
}

/// A folder of the test's own, with a stand-in for a test program in it.
struct Tree {
    root: PathBuf,
}

impl Drop for Tree {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

impl Tree {
    fn new(name: &str) -> Tree {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let unique = format!("xtask-{name}-{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::SeqCst));
        let tree = Tree { root: PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(unique) };
        fs::create_dir_all(&tree.root).unwrap();
        tree
    }

    /// A crowd of `times` runs, four at once, of a stand-in that does
    /// `script` each time it is run.
    fn crowd(&self, script: &str, times: usize) -> Crowd {
        let program = self.root.join("suite");
        // Written by a program of its own and not by this one. A file
        // this process has open to write is held, for an instant, by
        // whatever another test's thread is starting just then, and the
        // system will not run a file that anybody has open to write
        // ("Text file busy"): that failed a gate, once in some dozens.
        // The shell that writes it starts nothing meanwhile.
        let mut writes = Command::new("sh")
            .args(["-c", "cat > \"$1\" && chmod 755 \"$1\"", "sh"])
            .arg(&program)
            .stdin(Stdio::piped())
            .spawn()
            .unwrap();
        writes.stdin.take().unwrap().write_all(format!("#!/bin/sh\n{script}\n").as_bytes()).unwrap();
        assert!(writes.wait().unwrap().success(), "the stand-in could not be written");
        Crowd {
            program,
            dir: self.root.clone(),
            test: "module::a_test".to_string(),
            times,
            at_once: 4,
            // Held to nothing: whether this system has `taskset` is not
            // what is asked here.
            hold: None,
            limit: Duration::from_secs(120),
            keep: self.root.join("kept"),
        }
    }
}

/// What a test program prints for one test that passed.
const PASSES: &str = "echo 'running 1 test'; echo 'test module::a_test ... ok'; \
                      echo 'test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 9 filtered out'";

#[test]
fn what_is_asked_is_read_and_what_is_left_unsaid_is_the_usual() {
    let usual = asked(&words(&["journal", "given::a_test"])).unwrap();
    assert_eq!(
        usual,
        Asked {
            krate: "journal".into(),
            test: "given::a_test".into(),
            times: 40,
            at_once: 4,
            cpus: 2,
            limit: Duration::from_secs(600),
        }
    );
    let said = words(&["--times", "7", "api", "--cpus", "4", "a::b", "--at-once", "9", "--limit", "30"]);
    let said = asked(&said).unwrap();
    assert_eq!((said.krate.as_str(), said.test.as_str()), ("api", "a::b"));
    assert_eq!((said.times, said.at_once, said.cpus, said.limit), (7, 9, 4, Duration::from_secs(30)));

    // Asked wrongly, it says how it is asked and runs nothing.
    for wrong in [
        vec!["journal"],
        vec!["journal", "a", "b"],
        vec!["journal", "a", "--times"],
        vec!["journal", "a", "--times", "0"],
        vec!["journal", "a", "--times", "many"],
        vec!["journal", "a", "--threads", "4"],
    ] {
        let refused = asked(&words(&wrong)).unwrap_err();
        assert!(refused.contains("usage: cargo xtask crowd <crate> <test>"), "{wrong:?}: {refused}");
    }
}

#[test]
fn a_name_that_is_no_tests_whole_name_is_refused_with_the_tests_it_is_part_of() {
    let tests = words(&["pool::a_place_is_given_once", "pool::a_place_is_given_back", "runs::a_run_ends"]);
    assert_eq!(named(&tests, "runs::a_run_ends"), Ok("runs::a_run_ends"));

    // The module left off: a test program asked for that runs nothing,
    // and says that all is well.
    let near = named(&tests, "a_place_is_given").unwrap_err();
    assert!(near.contains("no test is called `a_place_is_given`"), "{near}");
    assert!(near.contains("  pool::a_place_is_given_once\n  pool::a_place_is_given_back"), "{near}");
    assert!(!near.contains("runs::"), "{near}");

    let nothing = named(&tests, "a_test_there_never_was").unwrap_err();
    assert!(nothing.contains("nothing would be run, which is not the test passing"), "{nothing}");
}

#[test]
fn a_crowd_is_held_to_the_first_of_the_processors_this_process_may_itself_use() {
    let status = "Name:\txtask\nCpus_allowed:\tff00\nCpus_allowed_list:\t8-15\nMems_allowed_list:\t0\n";
    let lane = allowed(status).unwrap();
    assert_eq!(lane, (8..=15).collect::<Vec<_>>());
    assert_eq!(held(&lane, 2), "8,9");
    assert_eq!(held(&lane, 4), "8,9,10,11");
    // No more than there are, and never none.
    assert_eq!(held(&[3, 5], 8), "3,5");
    assert_eq!(held(&lane, 0), "8");

    assert_eq!(allowed("Cpus_allowed_list:\t0-3,8,10-11\n"), Some(vec![0, 1, 2, 3, 8, 10, 11]));
    // A system that does not say, or says something else, is not guessed at.
    assert_eq!(allowed("Name:\txtask\n"), None);
    assert_eq!(allowed("Cpus_allowed_list:\tall\n"), None);
}

#[test]
fn every_run_is_counted_and_the_whole_output_of_each_that_failed_is_kept_and_of_none_that_passed() {
    let tree = Tree::new("crowd-counted");
    // The fifth run of them all fails, and says why on both of its outputs.
    let script = format!(
        "[ \"$1 $2 $3\" = 'module::a_test --exact --nocapture' ] || {{ echo \"asked for $*\"; exit 2; }}\n\
         [ \"$CARGO_MANIFEST_DIR\" = \"$PWD\" ] || {{ echo 'not run from its folder'; exit 2; }}\n\
         if [ \"$CROWD_RUN\" = 5 ] && [ ! -e passes ]; then\n\
           echo 'running 1 test'; echo \"thread 'module::a_test' panicked\" >&2; echo 'left: 1'\n\
           echo 'test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 9 filtered out'; exit 101\n\
         fi\n{PASSES}"
    );
    let crowd = tree.crowd(&script, 12);
    let told = Mutex::new(Vec::new());

    let came = run(&crowd, &|line| told.lock().unwrap().push(line.to_string())).unwrap();

    assert_eq!(came, Came { ran: 12, failed: vec![5], ended: Vec::new() });
    assert!(!came.ok());
    let failed = kept(&crowd.keep, came.failed[0]);
    let printed = fs::read_to_string(&failed).unwrap();
    assert!(
        printed.contains("thread 'module::a_test' panicked")
            && printed.contains("left: 1")
            && printed.contains("test result: FAILED"),
        "{printed}"
    );
    // Nothing is kept of the eleven that passed.
    let there: Vec<_> = fs::read_dir(&crowd.keep).unwrap().map(|entry| entry.unwrap().path()).collect();
    assert_eq!(there, vec![failed.clone()]);
    // And whoever waits was told of it as it ended.
    assert_eq!(*told.lock().unwrap(), vec![format!("run {} failed: {}", came.failed[0], failed.display())]);

    // Crowded again and passing every time, what the last crowd kept is
    // gone: what is there is this one's.
    fs::write(tree.root.join("passes"), "").unwrap();
    let came = run(&crowd, &|_| {}).unwrap();
    assert_eq!(came, Came { ran: 12, failed: Vec::new(), ended: Vec::new() });
    assert!(came.ok());
    assert_eq!(fs::read_dir(&crowd.keep).unwrap().count(), 0);
}

#[test]
fn a_run_that_ran_no_test_is_a_failure_though_the_program_ended_well() {
    let tree = Tree::new("crowd-nothing");
    // As a test program answers for a name it has no test of, and for a
    // test that is ignored.
    for nothing in [
        "echo 'running 0 tests'; echo 'test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 10 filtered out'",
        "echo 'running 1 test'; echo 'test module::a_test ... ignored'; \
         echo 'test result: ok. 0 passed; 0 failed; 1 ignored; 0 measured; 9 filtered out'",
    ] {
        let crowd = tree.crowd(nothing, 3);
        let came = run(&crowd, &|_| {}).unwrap();
        assert_eq!((came.ran, came.failed.as_slice()), (3, [1, 2, 3].as_slice()), "{nothing}");
        assert!(fs::read_to_string(kept(&crowd.keep, 2)).unwrap().contains("0 passed"));
    }
}

#[test]
fn a_run_that_does_not_end_is_ended_when_its_time_is_up_and_is_counted_apart_with_what_it_had_said() {
    let tree = Tree::new("crowd-ended");
    // It says it has begun and then reads from a pipe nobody writes to,
    // which the test holds open: it ends by itself when the test is over,
    // if nothing ended it before.
    let pipe = tree.root.join("pipe");
    assert!(std::process::Command::new("mkfifo").arg(&pipe).status().unwrap().success());
    let held_open = fs::OpenOptions::new().read(true).write(true).open(&pipe).unwrap();
    let mut crowd = tree.crowd("echo 'running 1 test'; exec cat pipe", 2);
    // No time at all: a run is ended as soon as it is looked at.
    crowd.limit = Duration::ZERO;

    let came = run(&crowd, &|_| {}).unwrap();

    assert_eq!((came.ran, came.failed.len(), came.ended.as_slice()), (2, 0, [1, 2].as_slice()));
    assert!(!came.ok());
    assert!(kept(&crowd.keep, 1).is_file());
    let asked = asked(&words(&["journal", "module::a_test", "--limit", "5"])).unwrap();
    assert_eq!(
        summary(&asked, Some("8,9"), &came),
        "module::a_test of journal: 2 runs, 4 at once, held to processors 8,9: 0 failed, 2 had not ended in 5 s"
    );
    drop(held_open);
}

#[test]
fn what_is_said_at_the_end_is_how_many_ran_how_and_how_many_failed() {
    let asked = asked(&words(&["api", "rows::a_card", "--times", "50", "--at-once", "6"])).unwrap();
    let came = Came { ran: 50, failed: vec![7, 31], ended: Vec::new() };
    assert_eq!(
        summary(&asked, Some("0,1"), &came),
        "rows::a_card of api: 50 runs, 6 at once, held to processors 0,1: 2 failed"
    );
    assert_eq!(
        summary(&asked, None, &Came { ran: 50, ..Default::default() }),
        "rows::a_card of api: 50 runs, 6 at once, held to no processors: 0 failed"
    );
}

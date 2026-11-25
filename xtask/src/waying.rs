//! `cargo xtask ways [<way> ...]`: every suite, played the other ways.
//!
//! The test programs are built as the gate builds them, in the one build
//! with every crate in it (`suites::built_whole`), so that what is played
//! is the programs the gate runs and nothing is compiled a second way.
//! Each way is then one run of all of them by the tests' runner, started
//! as a command of its own (`run-tests`, as the `offline` gate starts it)
//! so that it and every program it runs are told the way's word and no
//! other (`xtask::ways::told`). A way is played whatever the one before
//! it came to, so that one asking says everything that fails. One that
//! only some suites go by is a run of those (`xtask::ways::plays`).
//!
//! What a gate last kept of how long each test took is put back after
//! each way: `cargo xtask timings` says what the gate's own run took, and
//! a run told another word is not that.

use std::process::{Command, Stdio};
use std::time::Instant;

use xtask::ways::{self, Way};

use crate::crowding::standing;
use crate::repo::{Ran, Repo};
use crate::suites::{self, Program};

/// How many lines of what is wrong with a way are printed.
const MOST: usize = 60;

/// Plays every test program one way, and says what came of it: whether
/// it held, and what the runner said.
fn play(repo: &Repo, programs: &[Program], way: &Way) -> Result<suites::Outcome, String> {
    let this = std::env::current_exe()
        .map_err(|err| format!("this program could not find itself to run the tests: {err}"))?;
    let mut command = Command::new(this);
    command.args(suites::asked(programs)).current_dir(&repo.root);
    for (says, word) in ways::told(way) {
        match word {
            Some(word) => command.env(says, word),
            None => command.env_remove(says),
        };
    }
    let child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|err| format!("could not start the tests' runner: {err}"))?;
    let ran = suites::follow(child)?;
    let mut outcome = suites::heard(&ran);
    // A runner that ended badly and said nothing of why (it was ended,
    // or could not begin) has not passed.
    if !ran.ok && outcome.problems.is_empty() {
        outcome.problems.push(format!("the tests' runner ended badly, saying only: {}", ran.aside));
    }
    Ok(outcome)
}

pub fn run(repo: &Repo, args: &[String]) -> Result<bool, String> {
    let asked = ways::asked(args)?;
    println!("building the test programs, as the gate does");
    let programs = suites::built_whole(repo).map_err(|problems| problems.join("\n"))?;
    if programs.is_empty() {
        return Err("found no test programs to play".to_string());
    }
    let names: Vec<&str> = asked.iter().map(|way| way.name).collect();
    println!("{}: the suites, played {}", standing(&repo.root), names.join(", "));
    let kept = repo.tests_as_kept(Ran::Gate);
    let mut failed = Vec::new();
    for way in asked {
        let started = Instant::now();
        let of_it: Vec<Program> =
            programs.iter().filter(|program| ways::plays(way, &program.krate)).cloned().collect();
        let outcome = match of_it.is_empty() {
            true => Err(format!("no test program of {} was built", way.suites.join(", "))),
            false => play(repo, &of_it, way),
        };
        repo.keep_tests_as(Ran::Gate, kept.as_deref());
        let millis = started.elapsed().as_millis();
        let (problems, timed) = match outcome {
            Ok(outcome) => (outcome.problems, outcome.timed),
            Err(why) => (vec![why], "not played".to_string()),
        };
        println!("{}", ways::line(way, problems.is_empty(), millis, &timed));
        if !problems.is_empty() {
            for problem in problems.iter().take(MOST) {
                println!("         {problem}");
            }
            if problems.len() > MOST {
                println!("         ... and {} more", problems.len() - MOST);
            }
            println!("       played with {}={}: {}", way.says, way.word, way.what);
            failed.push(way.name);
        }
    }
    if failed.is_empty() {
        println!("the suites hold played {}", names.join(", "));
    } else {
        println!("failed: {}", failed.join(", "));
    }
    Ok(failed.is_empty())
}

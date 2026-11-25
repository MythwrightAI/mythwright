//! `cargo xtask <command>`: the project's own tooling.
//!
//! ```text
//! cargo xtask check            every gate
//! cargo xtask check --fast     the static gates only (under a second)
//! cargo xtask check --changed  compile and test only crates affected by local changes
//! cargo xtask rename --display "New Name" [--id new-name] [--cli nn]
//! cargo xtask timings          how long each gate has been taking, and
//!                              which tests were the slowest in the last run
//! cargo xtask crowd <crate> <test> [--times N] [--at-once M] [--cpus K]
//!                              one test run many times, several at once on
//!                              a few processors; what each run that failed
//!                              printed is kept under target/xtask/crowd
//! cargo xtask left [--none]    what is running that a test or a check here
//!                              started and nobody waits for; it ends nothing,
//!                              and with --none fails where there is anything
//! cargo xtask ways [one] [own] [gathered] [ungathered]
//!                              every suite played another way than the
//!                              gates play it: with every project in the
//!                              factory's own journal, with the runner's
//!                              plays each a project's with a journal of
//!                              its own, with whatever waits for a
//!                              database kept by one commit; and the
//!                              journal's with every piece of work kept
//!                              by a commit of its own; all four where
//!                              none is named
//! cargo xtask changes [--since <ref-or-date>]
//!                              how many commits needed a rebuild, week by week
//! cargo xtask install [--desktop] [--to DIR] [--bin DIR]
//!                              build it, put a copy in a folder of its own
//!                              and the command on your PATH; with
//!                              --desktop, the window too, and on Linux
//!                              its entry in the desktop's menu
//! cargo xtask install --link   link the command to where it is built
//!                              instead, so that it follows this checkout
//! cargo xtask package          build it as it is released and make the
//!                              folder someone downloads, under target/package:
//!                              this system's, or with --target another's,
//!                              for trying there; with --nightly YYYYMMDD,
//!                              a nightly of the commit checked out
//! cargo xtask notices [--check]
//!                              make THIRD-PARTY-NOTICES.md, the notices of
//!                              what a package bundles, after a dependency
//!                              changed; with --check, say whether the one
//!                              there is what would be made
//! ```
//!
//! The gates are the rules of the repository written as code. Most exist
//! because the previous codebase measured what it cost not to have them; the
//! reasons are in `docs/build-times.md` and beside each gate below.

mod buildtimes;
mod crowding;
mod gates;
mod general;
mod install;
mod leftover;
mod offline;
mod packing;
mod portable;
mod rename;
mod repo;
mod suites;
mod waying;

use std::process::ExitCode;
use std::time::Instant;

use repo::Repo;
use xtask::changes;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    // Asked by a look, of this program itself and nobody else: to leave
    // something behind, and to be what is left (`xtask::left::adopter`).
    match args.first().map(String::as_str) {
        Some(xtask::left::LEAVES) => {
            return xtask::left::leaves().map_or(ExitCode::FAILURE, |()| ExitCode::SUCCESS);
        }
        Some(xtask::left::WAITS) => {
            xtask::left::waits();
            return ExitCode::SUCCESS;
        }
        _ => {}
    }
    let repo = match Repo::find() {
        Ok(repo) => repo,
        Err(message) => return fail(&message),
    };
    match args.first().map(String::as_str) {
        Some("check") => check(&repo, &args[1..]),
        Some("rename") => match rename::run(&repo, &args[1..]) {
            Ok(()) => ExitCode::SUCCESS,
            Err(message) => fail(&message),
        },
        Some("timings") => timings(&repo),
        Some("crowd") => match crowding::run(&repo, &args[1..]) {
            Ok(true) => ExitCode::SUCCESS,
            Ok(false) => ExitCode::FAILURE,
            Err(message) => fail(&message),
        },
        Some("left") => match leftover::list(&repo, &args[1..]) {
            Ok(true) => ExitCode::SUCCESS,
            Ok(false) => ExitCode::FAILURE,
            Err(message) => fail(&message),
        },
        Some("ways") => match waying::run(&repo, &args[1..]) {
            Ok(true) => ExitCode::SUCCESS,
            Ok(false) => ExitCode::FAILURE,
            Err(message) => fail(&message),
        },
        // Asked for by the `offline` gate, of itself, where there is no network.
        Some("run-tests") => suites::inside(&repo, &args[1..]),
        Some("build-times") => match buildtimes::run(&repo, &args[1..]) {
            Ok(()) => ExitCode::SUCCESS,
            Err(message) => fail(&message),
        },
        Some("install") => match install::run_install(&repo, &args[1..]) {
            Ok(()) => ExitCode::SUCCESS,
            Err(message) => fail(&message),
        },
        Some("package") => match packing::run_package(&repo, &args[1..]) {
            Ok(()) => ExitCode::SUCCESS,
            Err(message) => fail(&message),
        },
        Some("changes") => match changes::run(&repo.root, &args[1..]) {
            Ok(()) => ExitCode::SUCCESS,
            Err(message) => fail(&message),
        },
        Some("notices") => match xtask::notices::run(&repo.root, &args[1..]) {
            Ok(()) => ExitCode::SUCCESS,
            Err(message) => fail(&message),
        },
        _ => {
            eprintln!(
                "usage: cargo xtask <check [--fast] [--changed] | build-times [--clean] | rename --display NAME [--id ID] [--cli CLI] | timings | crowd CRATE TEST [--times N] [--at-once M] [--cpus K] [--limit SECONDS] | left [--none] | ways [one] [own] [gathered] [ungathered] | changes [--since <ref-or-date>] | install [--desktop] [--to DIR] [--bin DIR] [--link] [--no-build] [--profile NAME] | package [--no-build] [--profile NAME] [--out DIR] [--target TRIPLE] [--cargo-build WORD] [--no-window] [--nightly YYYYMMDD] | notices [--check]>"
            );
            ExitCode::from(2)
        }
    }
}

fn fail(message: &str) -> ExitCode {
    eprintln!("error: {message}");
    ExitCode::FAILURE
}

fn check(repo: &Repo, args: &[String]) -> ExitCode {
    // The gates check the files git lists here. Where it lists none, every
    // one of them would pass having looked at nothing: a copy of the source
    // that is no repository, or a folder git ignores inside another one.
    if repo.files().is_empty() {
        return fail(
            "git lists no files here, and the gates check the files it lists: nothing would be checked. \
             This folder is not a repository of its own (`git init` makes it one), or everything in it is ignored",
        );
    }
    let fast = args.iter().any(|arg| arg == "--fast");
    let changed = args.iter().any(|arg| arg == "--changed");
    let (scope, because) =
        if changed { gates::Scope::changed(repo) } else { (gates::Scope::Everything, None) };
    match (&scope, because) {
        (gates::Scope::Crates(crates), _) => println!(
            "scope: {}",
            if crates.is_empty() { "no Rust crates affected".to_string() } else { crates.join(", ") }
        ),
        (gates::Scope::Everything, Some(file)) => {
            println!(
                "scope: everything, since {file} changed and is in no crate: any crate's tests may read it"
            )
        }
        (gates::Scope::Everything, None) => {}
    }

    let mut failed = Vec::new();
    for gate in gates::ALL {
        if fast && gate.slow {
            continue;
        }
        let started = Instant::now();
        let result = (gate.run)(repo, &scope);
        let millis = started.elapsed().as_millis();
        repo.record_timing(gate.name, millis, result.is_ok());
        match result {
            Ok(note) => println!("  ok   {:<10} {millis:>7} ms  {note}", gate.name),
            Err(problems) => {
                println!("  FAIL {:<10} {millis:>7} ms", gate.name);
                for problem in problems.iter().take(40) {
                    println!("         {problem}");
                }
                if problems.len() > 40 {
                    println!("         ... and {} more", problems.len() - 40);
                }
                println!("       why: {}", gate.why);
                failed.push(gate.name);
            }
        }
    }
    if failed.is_empty() {
        println!("all gates passed");
        ExitCode::SUCCESS
    } else {
        println!("failed: {}", failed.join(", "));
        ExitCode::FAILURE
    }
}

/// How many of the slowest tests `timings` names.
const SLOWEST: usize = 15;

/// Median and latest duration of each gate, from the local ledger, and
/// what the tests took the last time a gate ran them.
fn timings(repo: &Repo) -> ExitCode {
    let rows = repo.timings();
    if rows.is_empty() {
        println!("no gate has been timed here yet; `cargo xtask check` times each");
    } else {
        println!("{:<10} {:>6} {:>10} {:>10} {:>10}", "gate", "runs", "median ms", "latest ms", "worst ms");
    }
    for gate in gates::ALL {
        let mut millis: Vec<u128> =
            rows.iter().filter(|row| row.gate == gate.name).map(|row| row.millis).collect();
        let Some(latest) = millis.last().copied() else { continue };
        millis.sort_unstable();
        println!(
            "{:<10} {:>6} {:>10} {:>10} {:>10}",
            gate.name,
            millis.len(),
            millis[millis.len() / 2],
            latest,
            millis[millis.len() - 1]
        );
    }
    let whole = repo.tests(repo::Ran::Whole);
    if let Some(millis) = whole.whole {
        println!(
            "\nevery test, as `cargo xtask build-times` last ran them: one program after another, each on"
        );
        println!("every thread, which is what the budget holds. {millis} ms from end to end.");
        suites(&whole);
    }
    let kept = repo.tests(repo::Ran::Gate);
    if kept.suites.is_empty() {
        return ExitCode::SUCCESS;
    }
    println!("\nthe tests, as a gate last ran each crate's:");
    suites(&kept);
    let mut tests = kept.tests;
    tests.sort_by_key(|(_, took)| std::cmp::Reverse(took.millis));
    println!("\nthe slowest, each timed with the others running beside it (alone it takes less):");
    for (krate, took) in tests.iter().take(SLOWEST) {
        let played = kept.played.iter().find(|one| &one.krate == krate && one.test == took.test);
        let played =
            played.map_or(String::new(), |one| format!("  ({} places, {} plays)", one.places, one.plays));
        println!("{:>8} ms  {:<10} {}{played}", took.millis, krate, took.test);
    }
    ExitCode::SUCCESS
}

/// Each crate's tests of a run that was kept, the longest first: how many,
/// how long the program ran, which of its tests took longest (a program is
/// never over before that one is), and what its tests that play played.
fn suites(kept: &repo::Kept) {
    let mut suites: Vec<&repo::Suite> = kept.suites.iter().collect();
    suites.sort_by_key(|suite| std::cmp::Reverse(suite.millis));
    println!(
        "{:<12} {:>6} {:>10} {:>8} {:>8} {:>11}  its longest test",
        "crate", "tests", "ran ms", "places", "plays", "longest ms"
    );
    let count = |count: u64| if count == 0 { "-".to_string() } else { count.to_string() };
    for suite in suites {
        let (places, plays) = xtask::played::of(&kept.played, &suite.krate);
        let longest =
            kept.tests.iter().filter(|(krate, _)| krate == &suite.krate).max_by_key(|(_, took)| took.millis);
        let (millis, test) = longest.map_or((0, ""), |(_, took)| (took.millis, took.test.as_str()));
        println!(
            "{:<12} {:>6} {:>10} {:>8} {:>8} {:>11}  {test}",
            suite.krate,
            suite.tests,
            suite.millis,
            count(places),
            count(plays),
            millis
        );
    }
}

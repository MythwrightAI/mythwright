//! Build times: measured, written down, and held to a budget.
//!
//! The codebase before this one reached builds of fifty minutes without
//! any single change having caused it. Nothing here stops a build getting
//! slower; this makes it impossible not to notice. `cargo xtask build-times`
//! measures the same few things each time and adds a line to a history that
//! is kept with the source. Running every test is one of them: once the
//! code is built that is where a gate's time goes, and it grows the same
//! way, a test at a time. The `budget` gate fails when the last line is
//! over budget, and when the code has grown enough since that line was
//! written that it no longer says how long a build takes.

use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use xtask::clock;

use crate::gates::{GateResult, failures};
use crate::repo::{Ran, Repo, brand_value, line_count};
use crate::suites;

/// The history, kept with the source so that it outlives `cargo clean`.
const HISTORY: &str = "docs/build-history.csv";
/// What a build may take, and how much code may be added before it is
/// measured again.
const BUDGET: &str = "build-budget.toml";
const COLUMNS: &str = "date,rust_lines,test_lines,packages,rebuild_bottom_ms,rebuild_top_ms,test_binaries_ms,clean_dev_s,jobs,tests_ms,tests_by_crate_ms,rebuild_runner_ms";
/// A crate's tests are named in a line of the history where they took
/// this long; the rest are put together.
const NAMED_FROM_MS: u64 = 1000;

/// The crate everything is built on, and the one built on everything.
const BOTTOM: &str = "crates/kernel/src/lib.rs";
const TOP: &str = "crates/cli/src/main.rs";
/// The runner, which is the crate that comes nearest its ceiling of lines
/// again and again: what a change to it costs, it alone, so that its
/// growth shows as seconds and not only as lines.
const RUNNER: (&str, &str) = ("conductor", "crates/conductor/src/lib.rs");

struct Budget {
    rebuild_bottom_s: f64,
    rebuild_top_s: f64,
    clean_dev_s: f64,
    test_run_s: f64,
    packages: usize,
    remeasure_after_growth_percent: f64,
}

struct Row {
    date: String,
    rust_lines: usize,
    test_lines: usize,
    packages: usize,
    rebuild_bottom_ms: u128,
    rebuild_top_ms: u128,
    test_binaries_ms: u128,
    /// Only measured when asked for: it is the expensive one.
    clean_dev_s: Option<f64>,
    jobs: String,
    /// Running every test, once everything is built. A line from before
    /// this was measured has none, which is not the same as none at all.
    tests_ms: Option<u128>,
    /// What each crate's tests took of that, the longest first, as it is
    /// written (`conductor:163012 api:135100 others:16000`): the programs
    /// are run one after another, so the first of them is where a run of
    /// every test is to be made shorter. A line from before has none.
    tests_by_crate: String,
    /// A rebuild of the runner alone after a change to it, with nothing
    /// above it built. A line from before has none.
    rebuild_runner_ms: Option<u128>,
}

/// The crate whose tests took longest in a line of the history, and how
/// long: nothing for a line that does not say.
fn longest(by_crate: &str) -> Option<(&str, u64)> {
    by_crate
        .split_whitespace()
        .filter_map(|one| one.split_once(':'))
        .filter(|(krate, _)| *krate != "others")
        .filter_map(|(krate, millis)| Some((krate, millis.parse().ok()?)))
        .max_by_key(|(_, millis)| *millis)
}

fn budget(repo: &Repo) -> Result<Budget, String> {
    let text =
        repo.read(BUDGET).ok_or_else(|| format!("{BUDGET} is missing; it says what a build may take"))?;
    let number = |key: &str| -> Result<f64, String> {
        brand_value(&text, key)
            .and_then(|value| value.parse::<f64>().ok())
            .ok_or_else(|| format!("{BUDGET} needs a number for `{key}`"))
    };
    Ok(Budget {
        rebuild_bottom_s: number("rebuild_bottom_s")?,
        rebuild_top_s: number("rebuild_top_s")?,
        clean_dev_s: number("clean_dev_s")?,
        test_run_s: number("test_run_s")?,
        packages: number("packages")? as usize,
        remeasure_after_growth_percent: number("remeasure_after_growth_percent")?,
    })
}

/// Lines of Rust that are the product, and lines that test it.
fn lines(repo: &Repo) -> (usize, usize) {
    let (mut source, mut tests) = (0, 0);
    for file in repo.files().iter().filter(|file| file.ends_with(".rs")) {
        let count = line_count(&repo.root.join(file));
        if file.contains("/tests/") { tests += count } else { source += count }
    }
    (source, tests)
}

fn packages(repo: &Repo) -> usize {
    repo.read("Cargo.lock").map_or(0, |lock| lock.lines().filter(|line| line.starts_with("name = ")).count())
}

fn history(repo: &Repo) -> Vec<Row> {
    let text = repo.read(HISTORY).unwrap_or_default();
    text.lines()
        .skip(1)
        .filter_map(|line| {
            let cells: Vec<&str> = line.split(',').collect();
            Some(Row {
                date: cells.first()?.to_string(),
                rust_lines: cells.get(1)?.parse().ok()?,
                test_lines: cells.get(2)?.parse().ok()?,
                packages: cells.get(3)?.parse().ok()?,
                rebuild_bottom_ms: cells.get(4)?.parse().ok()?,
                rebuild_top_ms: cells.get(5)?.parse().ok()?,
                test_binaries_ms: cells.get(6)?.parse().ok()?,
                clean_dev_s: cells.get(7).and_then(|cell| cell.parse().ok()),
                jobs: cells.get(8).unwrap_or(&"").to_string(),
                tests_ms: cells.get(9).and_then(|cell| cell.parse().ok()),
                tests_by_crate: cells.get(10).unwrap_or(&"").to_string(),
                rebuild_runner_ms: cells.get(11).and_then(|cell| cell.parse().ok()),
            })
        })
        .collect()
}

/// Today's date, from the count of days since 1970.
fn today() -> String {
    let days =
        SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |since| since.as_secs() / 86_400) as i64;
    // Days to a civil date, by the usual arithmetic on 400-year cycles.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let day_of_era = z.rem_euclid(146_097);
    let year_of_era = (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_index + 2) / 5 + 1;
    let month = if month_index < 10 { month_index + 3 } else { month_index - 9 };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}")
}

fn cargo(repo: &Repo, args: &[&str]) -> Result<u128, String> {
    let started = Instant::now();
    let output = Command::new("cargo")
        .args(args)
        .current_dir(&repo.root)
        .output()
        .map_err(|err| format!("could not run cargo: {err}"))?;
    if !output.status.success() {
        let said = String::from_utf8_lossy(&output.stderr);
        return Err(format!("`cargo {}` failed:\n{}", args.join(" "), said.trim_end()));
    }
    Ok(started.elapsed().as_millis())
}

/// Runs every test as cargo does, one program after another on every
/// thread, and says how long that took from end to end and what each
/// crate's took of it. It is cargo that is timed, as it always was: what
/// each program took is read from what cargo and the programs print as
/// they go, and is kept where `cargo xtask timings` reads it, with what
/// each test took and what each test that plays played.
fn every_test(repo: &Repo) -> Result<(u128, String), String> {
    let programs = suites::built_whole(repo).map_err(|problems| problems.join("\n"))?;
    let played_to = repo.played_path();
    let _ = played_to.parent().map(std::fs::create_dir_all);
    let _ = std::fs::remove_file(&played_to);
    let started = Instant::now();
    let child = Command::new("cargo")
        .arg("test")
        .env(xtask::played::TO, &played_to)
        .current_dir(&repo.root)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|err| format!("could not run cargo: {err}"))?;
    let ran = suites::follow(child)?;
    let whole = started.elapsed().as_millis();
    let played = repo.played();
    if !ran.ok {
        return Err(format!("`cargo test` failed:\n{}", failures(&ran.text()).join("\n")));
    }
    // Each run of tests in what was printed is the next thing cargo said
    // it would run. Where the two do not come to as many, nothing says
    // whose a run was, and only the whole is kept.
    let runs = clock::runs(&ran.lines, clock::threads());
    let programs: Vec<(String, String)> =
        programs.into_iter().map(|program| (program.krate, program.path)).collect();
    let whose = clock::whose(&ran.aside, &programs);
    repo.record_whole(whole as u64);
    if runs.len() != whose.len() {
        println!(
            "cargo said it ran {} things and {} runs of tests were printed: what each crate's tests took was not read",
            whose.len(),
            runs.len()
        );
        return Ok((whole, String::new()));
    }
    let mut by_crate: Vec<(String, u64)> = Vec::new();
    for (krate, run) in whose.iter().zip(&runs) {
        let Some(krate) = krate else { continue };
        repo.record_tests(Ran::Whole, krate, run.millis, &run.tests, &played);
        by_crate.push((krate.clone(), run.millis));
    }
    by_crate.sort_by_key(|(_, millis)| std::cmp::Reverse(*millis));
    let others: u64 =
        by_crate.iter().filter(|(_, millis)| *millis < NAMED_FROM_MS).map(|(_, millis)| millis).sum();
    let mut said: Vec<String> = by_crate
        .iter()
        .filter(|(_, millis)| *millis >= NAMED_FROM_MS)
        .map(|(krate, millis)| format!("{krate}:{millis}"))
        .collect();
    if others > 0 {
        said.push(format!("others:{others}"));
    }
    Ok((whole, said.join(" ")))
}

/// Makes a file look changed, as an edit to it would.
fn touch(path: &Path) -> Result<(), String> {
    std::fs::File::options()
        .append(true)
        .open(path)
        .and_then(|file| file.set_modified(SystemTime::now()))
        .map_err(|err| format!("could not touch {}: {err}", path.display()))
}

/// Measures, adds a line to the history, shows the last few, and says
/// whether the budget is kept.
pub fn run(repo: &Repo, args: &[String]) -> Result<(), String> {
    let clean = args.iter().any(|arg| arg == "--clean");
    let budget = budget(repo)?;
    // Everything built first, so that what is timed is only what a change costs.
    println!("building everything first, so that only a change is timed");
    cargo(repo, &["build"])?;
    cargo(repo, &["test", "--no-run"])?;
    // Every test, with nothing left to compile: what a gate spends most of
    // its time on once the code is built.
    println!("running every test");
    let (tests_ms, tests_by_crate) = every_test(repo)?;

    touch(&repo.root.join(BOTTOM))?;
    let rebuild_bottom_ms = cargo(repo, &["build"])?;
    let test_binaries_ms = cargo(repo, &["test", "--no-run"])?;
    touch(&repo.root.join(TOP))?;
    let rebuild_top_ms = cargo(repo, &["build"])?;
    // The runner by itself: asked for alone, its dependencies may be built
    // with other features than the whole has, so it is built so once before
    // it is changed, and only the build after the change is timed.
    let (runner, runner_lib) = RUNNER;
    cargo(repo, &["build", "-p", runner])?;
    touch(&repo.root.join(runner_lib))?;
    let rebuild_runner_ms = cargo(repo, &["build", "-p", runner])?;

    let clean_dev_s = if clean {
        // Somewhere of its own, so that nobody's build is thrown away for it.
        let scratch = repo.root.join("target/build-times");
        let _ = std::fs::remove_dir_all(&scratch);
        println!("building from nothing in target/build-times; this is the slow one");
        let took = cargo(repo, &["build", "--target-dir", "target/build-times"])?;
        let _ = std::fs::remove_dir_all(&scratch);
        Some(took as f64 / 1000.0)
    } else {
        None
    };

    let (rust_lines, test_lines) = lines(repo);
    let jobs = std::env::var("CARGO_BUILD_JOBS").unwrap_or_else(|_| "all".into());
    let row = Row {
        date: today(),
        rust_lines,
        test_lines,
        packages: packages(repo),
        rebuild_bottom_ms,
        rebuild_top_ms,
        test_binaries_ms,
        clean_dev_s,
        jobs,
        tests_ms: Some(tests_ms),
        tests_by_crate,
        rebuild_runner_ms: Some(rebuild_runner_ms),
    };
    // The lines there are kept under the columns as they are now: one from
    // before a column was added has nothing in it.
    let kept: String = repo
        .read(HISTORY)
        .unwrap_or_default()
        .lines()
        .skip(1)
        .filter(|line| !line.trim().is_empty())
        .map(|line| format!("{line}\n"))
        .collect();
    let mut text = format!("{COLUMNS}\n{kept}");
    text.push_str(&format!(
        "{},{},{},{},{},{},{},{},{},{},{},{}\n",
        row.date,
        row.rust_lines,
        row.test_lines,
        row.packages,
        row.rebuild_bottom_ms,
        row.rebuild_top_ms,
        row.test_binaries_ms,
        row.clean_dev_s.map_or(String::new(), |seconds| format!("{seconds:.0}")),
        row.jobs,
        row.tests_ms.map_or(String::new(), |millis| millis.to_string()),
        row.tests_by_crate,
        row.rebuild_runner_ms.map_or(String::new(), |millis| millis.to_string())
    ));
    std::fs::write(repo.root.join(HISTORY), text)
        .map_err(|err| format!("could not write {HISTORY}: {err}"))?;

    show(&history(repo));
    match over(&budget, &row) {
        problems if problems.is_empty() => {
            println!("\nwithin budget. A line was added to {HISTORY}.");
            Ok(())
        }
        problems => Err(problems.join("\n")),
    }
}

fn show(rows: &[Row]) {
    println!(
        "\n{:<11} {:>10} {:>9} {:>16} {:>13} {:>16} {:>14} {:>10} {:>12}",
        "date",
        "rust lines",
        "packages",
        "bottom rebuild",
        "top rebuild",
        "runner rebuild",
        "test binaries",
        "from clean",
        "every test"
    );
    let seconds = |millis: u128| format!("{:.1} s", millis as f64 / 1000.0);
    for row in rows.iter().rev().take(12).rev() {
        println!(
            "{:<11} {:>10} {:>9} {:>16} {:>13} {:>16} {:>14} {:>10} {:>12}",
            row.date,
            row.rust_lines,
            row.packages,
            seconds(row.rebuild_bottom_ms),
            seconds(row.rebuild_top_ms),
            row.rebuild_runner_ms.map_or("-".to_string(), seconds),
            seconds(row.test_binaries_ms),
            row.clean_dev_s.map_or("-".to_string(), |s| format!("{s:.0} s")),
            row.tests_ms.map_or("-".to_string(), seconds)
        );
    }
    // Of the last one, what each crate's tests took: they are run one
    // after another, so this is where a run of every test goes.
    if let Some(last) = rows.last().filter(|last| !last.tests_by_crate.is_empty()) {
        let each: Vec<String> = last
            .tests_by_crate
            .split_whitespace()
            .filter_map(|one| one.split_once(':'))
            .filter_map(|(krate, millis)| Some(format!("{krate} {}", seconds(millis.parse().ok()?))))
            .collect();
        println!("\nevery test on {}, by crate: {}", last.date, each.join(", "));
    }
}

/// What of a measurement is over budget, each as a sentence.
fn over(budget: &Budget, row: &Row) -> Vec<String> {
    let mut problems = Vec::new();
    // What a build takes is looked into one way, and what the tests take another.
    let build = "`cargo build --timings`";
    let tests = "`cargo xtask timings` names the slowest tests of the last gate";
    let mut held = |what: &str, took_s: f64, may_s: f64, look: &str| {
        if took_s > may_s {
            problems.push(format!(
                "{what} took {took_s:.1} s on {}, and may take {may_s:.0} s. Find what made it slow ({look}) before raising the budget in {BUDGET}",
                row.date
            ));
        }
    };
    held(
        "a rebuild after a change to the bottom crate",
        row.rebuild_bottom_ms as f64 / 1000.0,
        budget.rebuild_bottom_s,
        build,
    );
    held(
        "a rebuild after a change to the top crate",
        row.rebuild_top_ms as f64 / 1000.0,
        budget.rebuild_top_s,
        build,
    );
    if let Some(clean) = row.clean_dev_s {
        held("a build from nothing", clean, budget.clean_dev_s, build);
    }
    if let Some(millis) = row.tests_ms {
        held("running every test", millis as f64 / 1000.0, budget.test_run_s, tests);
    }
    problems
}

/// The gate: costs nothing to run, and fails when the budget is broken or
/// when what was last measured is no longer the code there is.
pub fn gate(repo: &Repo) -> GateResult {
    let budget = budget(repo).map_err(|message| vec![message])?;
    let mut problems = Vec::new();
    let now = packages(repo);
    if now > budget.packages {
        problems.push(format!(
            "Cargo.lock names {now} packages and the budget is {}. Every one is compiled by everyone, on every clean build; if the new ones earn that, raise `packages` in {BUDGET}",
            budget.packages
        ));
    }
    let rows = history(repo);
    let Some(last) = rows.last() else {
        problems.push(format!(
            "no build has been measured yet; run `cargo xtask build-times`, which starts {HISTORY}"
        ));
        return Err(problems);
    };
    problems.extend(over(&budget, last));
    let (rust_lines, _) = lines(repo);
    let grown = (rust_lines as f64 - last.rust_lines as f64) / last.rust_lines.max(1) as f64 * 100.0;
    if grown > budget.remeasure_after_growth_percent {
        problems.push(format!(
            "there are {rust_lines} lines of Rust, {grown:.0}% more than when a build was last measured ({}, {} lines). Run `cargo xtask build-times` so that the history says what a build costs now",
            last.date, last.rust_lines
        ));
    }
    if problems.is_empty() {
        let most = longest(&last.tests_by_crate).map_or(String::new(), |(krate, millis)| {
            format!(" ({:.0} s of it `{krate}`)", millis as f64 / 1000.0)
        });
        let tests = last.tests_ms.map_or(String::new(), |millis| {
            format!(", {:.0} s to run every test{most}", millis as f64 / 1000.0)
        });
        Ok(format!(
            "{:.1} s to rebuild from the bottom{tests}, measured {} at {} lines; {now} of {} packages",
            last.rebuild_bottom_ms as f64 / 1000.0,
            last.date,
            last.rust_lines,
            budget.packages
        ))
    } else {
        Err(problems)
    }
}

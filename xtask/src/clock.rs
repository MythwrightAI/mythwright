//! How long each test took, worked out from what a test program prints.
//!
//! A test program does not say how long a test took: the option that has
//! it say so is not in the toolchain that is released. What it does print
//! is a line as each test finishes, and how it starts them is simple
//! enough to work back from. It takes them in the order of their names,
//! starts as many as it has threads, and starts one more each time one
//! finishes. So a test began when the run did, if it was among the first,
//! and otherwise when the one that made room for it finished; and it took
//! from then until its own line was printed.
//!
//! What that gives is how long a test took with the others running beside
//! it, which is more than it takes alone (five times more, measured, for
//! the tests that start a whole fleet). It says which tests are the slow
//! ones and is not held to a budget: what is held is how long every test
//! takes together (`cargo xtask build-times`).
//!
//! Every second counted is between two lines that were printed. Told the
//! wrong number of threads it puts time on the wrong test, and it never
//! makes any up.

/// A line a test program printed, and how long after the program was
/// started it arrived, in milliseconds.
pub type Line = (u64, String);

/// One test, and how long it took with the others beside it.
#[derive(Clone, Debug, PartialEq)]
pub struct Took {
    pub test: String,
    pub millis: u64,
}

/// One run of a test program: its tests, and how long it was from the
/// program saying it began to the last line of the run.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Run {
    pub tests: Vec<Took>,
    pub millis: u64,
}

/// The test a line says has finished, however it went.
fn finished(line: &str) -> Option<&str> {
    let (name, came_to) = line.strip_prefix("test ")?.split_once(" ... ")?;
    let came_to = came_to.split_whitespace().next()?.trim_end_matches(',');
    matches!(came_to, "ok" | "FAILED" | "ignored")
        .then(|| name.strip_suffix(" - should panic").unwrap_or(name))
}

/// Whether a line is a test program saying that it begins.
fn begins(line: &str) -> bool {
    line.strip_prefix("running ")
        .and_then(|rest| rest.strip_suffix(" tests").or(rest.strip_suffix(" test")))
        .is_some_and(|count| count.parse::<usize>().is_ok())
}

/// The runs in what was printed, in the order they were made, each test
/// with how long it took: the program ran `threads` tests at a time.
pub fn runs(lines: &[Line], threads: usize) -> Vec<Run> {
    let mut runs = Vec::new();
    // When the run being read began, and what finished in it and when.
    let mut began: Option<u64> = None;
    let mut done: Vec<(u64, &str)> = Vec::new();
    let mut last = 0;
    let mut close = |began: Option<u64>, done: &mut Vec<(u64, &str)>, last: u64| {
        if let Some(began) = began {
            runs.push(run(began, std::mem::take(done), last, threads.max(1)));
        }
    };
    for (at, line) in lines {
        let line = line.trim_end();
        if begins(line) {
            close(began, &mut done, last);
            began = Some(*at);
            last = *at;
        } else if began.is_some() {
            if let Some(test) = finished(line) {
                done.push((*at, test));
                last = *at;
            } else if line.starts_with("test result: ") {
                close(began.take(), &mut done, *at);
            }
        }
    }
    close(began, &mut done, last);
    runs
}

/// One run: it began at `began`, `done` is what finished and when, in the
/// order it was printed, and `ended` is its last line.
fn run(began: u64, done: Vec<(u64, &str)>, ended: u64, threads: usize) -> Run {
    // The order the program started them in: by name, as bytes.
    let mut order: Vec<&str> = done.iter().map(|(_, test)| *test).collect();
    order.sort_unstable();
    let tests = done
        .iter()
        .map(|(at, test)| {
            let place = order.binary_search(test).unwrap_or(0);
            // Among the first, it began with the run; otherwise when the
            // one that made room for it finished.
            let started = if place < threads { began } else { done[place - threads].0 };
            Took { test: test.to_string(), millis: at.saturating_sub(started) }
        })
        .collect();
    Run { tests, millis: ended.saturating_sub(began) }
}

/// How many tests a test program runs at a time when it is told nothing:
/// what `RUST_TEST_THREADS` says, or as many as this process may use
/// processors.
pub fn threads() -> usize {
    std::env::var("RUST_TEST_THREADS")
        .ok()
        .and_then(|said| said.parse().ok())
        .filter(|threads| *threads > 0)
        .unwrap_or_else(|| std::thread::available_parallelism().map_or(1, usize::from))
}

/// Whose each run of tests was, of those `cargo test` made, in the order
/// it made them: it says what it is about to run aside, a line for each
/// (`Running <target> (<program>)`, or `Doc-tests <crate>`), and those
/// lines are all that says. The crate, where the program is one of
/// `programs` (each a crate and where its test program is); nothing for
/// anything else cargo runs (what is written beside a program's own
/// code, what is written in its notes), which here has no tests in it.
pub fn whose(aside: &str, programs: &[(String, String)]) -> Vec<Option<String>> {
    aside
        .lines()
        .map(str::trim_start)
        .filter_map(|line| {
            if line.starts_with("Doc-tests ") {
                return Some(None);
            }
            let ran = line.strip_prefix("Running ")?;
            let path = ran.rsplit_once('(')?.1.trim_end_matches(')');
            Some(programs.iter().find(|(_, program)| program.ends_with(path)).map(|(krate, _)| krate.clone()))
        })
        .collect()
}

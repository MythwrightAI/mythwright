//! The test programs: built once, and run by the gates that hold them.
//!
//! `cargo test` builds every test program and then runs them, one after
//! another, each on as many threads as there are processors. The gates do
//! the two halves themselves, for three reasons.
//!
//! The two programs that drive the whole product are run where there is no
//! network (`offline`), and are run once: with cargo running everything
//! first, they were run twice, and that second run was two fifths of the
//! gate.
//!
//! A program that starts a daemon for each test gets no faster past a
//! dozen threads: the tests wait on one another inside the one process.
//! So where there are processors for it, the programs are run side by
//! side, eight threads each, and one with many tests is run as several
//! processes that each take a share of them. The API suite takes 21 s as
//! one process on thirty-two threads and 9 s as four on eight, for two
//! fifths less of the processors' time. On eight processors or fewer
//! nothing is side by side and nothing is shared out: it is as cargo does.
//!
//! And each line a program prints is given the time it arrived, which is
//! what says how long each test took (`xtask::clock`).
//!
//! Every program is built in the one build, with every crate in it. Asking
//! cargo for some crates' tests and not others changes which features
//! their dependencies are built with, and it then compiles a second copy
//! of every crate here.

use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitCode, Stdio};
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

use xtask::clock::{self, Line, Took};
use xtask::sharing::{self, Room};

use crate::gates::{Scope, failures};
use crate::repo::{self, Repo};

/// The crates whose tests start whole daemons and drive them as a person
/// would. Theirs are the tests `offline` runs.
pub const WHOLE_PRODUCT: &[&str] = &["api", "cli"];

/// Before a line the tests' runner prints for the gate that started it,
/// where the two are not one process: something that is wrong.
const PROBLEM: &str = "::problem ";
/// And before what it says of how long the tests took.
const TIMED: &str = "::timed ";

/// A test program that was built: whose it is, and where.
#[derive(Clone)]
pub struct Program {
    pub krate: String,
    /// The crate's own folder, which is where cargo runs its tests.
    pub dir: PathBuf,
    pub path: String,
}

/// What a program printed, each line with when it arrived.
pub struct Ran {
    pub ok: bool,
    pub lines: Vec<Line>,
    /// What it said aside, which is not timed.
    pub aside: String,
}

impl Ran {
    pub fn text(&self) -> String {
        let mut text: String = self.lines.iter().map(|(_, line)| format!("{line}\n")).collect();
        text.push_str(&self.aside);
        text
    }
}

/// The value of `"key":"value"` in a line of what cargo says of a build.
fn said<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let marker = format!("\"{key}\":\"");
    let rest = &line[line.find(&marker)? + marker.len()..];
    Some(&rest[..rest.find('"')?])
}

/// The test programs of what `scope` covers, built if they were not; none
/// where it covers nothing. Asked once for a run of the gates, which all
/// have the one scope: whoever asks again is told what the first was.
pub fn built(repo: &Repo, scope: &Scope) -> Result<Vec<Program>, Vec<String>> {
    static BUILT: OnceLock<Result<Vec<Program>, Vec<String>>> = OnceLock::new();
    BUILT.get_or_init(|| build(repo, scope)).clone()
}

fn build(repo: &Repo, scope: &Scope) -> Result<Vec<Program>, Vec<String>> {
    let Some(packages) = scope.package_args() else { return Ok(Vec::new()) };
    let output = Command::new("cargo")
        // What is built is said for a program to read; what is wrong with
        // the code is still said as it is to a person.
        .args(["test", "--no-run", "--message-format=json-render-diagnostics"])
        .args(&packages)
        .current_dir(&repo.root)
        .output()
        .map_err(|err| vec![format!("could not run cargo: {err}")])?;
    let text = String::from_utf8_lossy(&output.stdout);
    if !output.status.success() {
        return Err(failures(&String::from_utf8_lossy(&output.stderr)));
    }
    let members = repo.members();
    Ok(text
        .lines()
        .filter(|line| line.contains("\"kind\":[\"test\"]"))
        .filter_map(|line| {
            let path = said(line, "executable")?.replace("\\\\", "\\");
            let manifest = PathBuf::from(said(line, "manifest_path")?.replace("\\\\", "\\"));
            let dir = manifest.parent()?.to_path_buf();
            let krate = members
                .iter()
                .find(|member| repo.root.join(&member.dir) == dir)
                .map(|member| member.name.clone())
                .or_else(|| Some(dir.file_name()?.to_string_lossy().into_owned()))?;
            Some(Program { krate, dir, path })
        })
        .collect())
}

/// Waits for `child`, reading what it prints a line at a time and noting
/// when each line came.
pub fn follow(mut child: Child) -> Result<Ran, String> {
    let started = Instant::now();
    let printed = child.stdout.take().ok_or("the program's output was not kept")?;
    let aside = child.stderr.take();
    // Read beside the lines, so that a program that says much aside is
    // never held up by nobody reading it.
    let aside = std::thread::spawn(move || {
        let mut text = Vec::new();
        if let Some(mut aside) = aside {
            let _ = aside.read_to_end(&mut text);
        }
        String::from_utf8_lossy(&text).into_owned()
    });
    let mut lines = Vec::new();
    let mut reader = BufReader::new(printed);
    let mut line = Vec::new();
    loop {
        line.clear();
        match reader.read_until(b'\n', &mut line) {
            Ok(0) | Err(_) => break,
            Ok(_) => {
                let text = String::from_utf8_lossy(&line);
                lines.push((started.elapsed().as_millis() as u64, text.trim_end().to_string()));
            }
        }
    }
    let status = child.wait().map_err(|err| format!("could not wait for the program: {err}"))?;
    Ok(Ran { ok: status.success(), lines, aside: aside.join().unwrap_or_default() })
}

/// One process of tests: a program, or a share of its tests by name.
struct Job<'a> {
    program: &'a Program,
    only: Option<Vec<String>>,
    tests: usize,
    /// How many tests the program said it has, where it was asked.
    listed: Option<usize>,
}

/// What became of a job, and when it began and ended, in milliseconds from
/// when the first was started.
struct Done {
    ran: Result<Ran, String>,
    began: u64,
    ended: u64,
}

/// What came of running the tests: what is wrong, each a line, and what is
/// said of how long they took.
pub struct Outcome {
    pub problems: Vec<String>,
    pub timed: String,
}

/// Every test program there is, whatever the run of the gates covers: for
/// a gate that needs one its scope left out. Not kept, since it is asked
/// for at most once.
pub fn built_whole(repo: &Repo) -> Result<Vec<Program>, Vec<String>> {
    build(repo, &Scope::Everything)
}

/// Runs a program with these arguments and gives what it printed; or, where
/// it failed or could not be run, what it said.
pub fn asked_of(program: &Program, args: &[&str]) -> Result<String, String> {
    let output = command(program)
        .args(args)
        .output()
        .map_err(|err| format!("could not run {}: {err}", program.path))?;
    let printed = String::from_utf8_lossy(&output.stdout).into_owned();
    match output.status.success() {
        true => Ok(printed),
        false => Err(format!("{printed}{}", String::from_utf8_lossy(&output.stderr))),
    }
}

/// A program as cargo runs one: from its crate's folder, told where that is.
fn command(program: &Program) -> Command {
    let mut command = Command::new(&program.path);
    command.current_dir(&program.dir).env("CARGO_MANIFEST_DIR", &program.dir).stdin(Stdio::null());
    command
}

/// The names of a program's tests, in the order it would run them.
fn tests_of(program: &Program) -> Option<Vec<String>> {
    let output = command(program).args(["--list", "--format", "terse"]).output().ok()?;
    output.status.success().then(|| {
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter_map(|line| line.strip_suffix(": test"))
            .map(str::to_string)
            .collect()
    })
}

/// The processes to run for `programs`: a program with many tests is
/// several, where there is room for them (`sharing::shares`).
fn jobs(programs: &[Program], room: Room) -> Vec<Job<'_>> {
    let mut jobs = Vec::new();
    for program in programs {
        let tests = if room.shares > 1 { tests_of(program) } else { None };
        let listed = tests.as_ref().map(Vec::len);
        match tests.as_deref().and_then(|tests| sharing::shares(tests, room)) {
            Some(shares) => {
                for only in shares {
                    jobs.push(Job { program, tests: only.len(), only: Some(only), listed });
                }
            }
            None => jobs.push(Job { program, only: None, tests: listed.unwrap_or(0), listed }),
        }
    }
    jobs
}

fn run(job: &Job, threads: usize, played_to: &Path, root: &Path) -> Result<Ran, String> {
    let mut command = command(job.program);
    // How many tests at a time is said both ways: to the program's own
    // runner, and where its tests can read it, since a test that makes
    // plays several at a time makes as many as the program runs tests
    // and would otherwise take every processor for each process.
    command
        .arg(format!("--test-threads={threads}"))
        .env("RUST_TEST_THREADS", threads.to_string())
        .env(xtask::played::TO, played_to)
        // Whose tests these are, which whatever they start is told with
        // the rest: something left running says where it came from.
        .env(xtask::left::FROM, xtask::left::from(root, &job.program.krate));
    if let Some(only) = &job.only {
        command.arg("--exact").args(only);
    }
    let child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|err| format!("could not run {}: {err}", job.program.path))?;
    follow(child)
}

/// Runs every test of `programs`, as many processes side by side as there
/// are processors for, and keeps how long each test took for `cargo xtask
/// timings`. Every program is run whatever another came to, so that one
/// run says everything that fails.
pub fn run_all(repo: &Repo, programs: &[Program]) -> Outcome {
    let room = sharing::room(clock::threads());
    let (at_once, threads) = (room.at_once, room.threads);
    let jobs = jobs(programs, room);
    // The largest first, so that the small ones fill what room there is at
    // the end and not the other way about.
    let mut order: Vec<usize> = (0..jobs.len()).collect();
    order.sort_by_key(|index| std::cmp::Reverse(jobs[*index].tests));
    let waiting = Mutex::new(VecDeque::from(order));
    let done: Mutex<Vec<Option<Done>>> = Mutex::new(jobs.iter().map(|_| None).collect());
    // Where a suite that plays writes down how many places and plays
    // each of its tests made (`xtask::played`).
    let played_to = repo.played_path();
    let _ = played_to.parent().map(std::fs::create_dir_all);
    let _ = std::fs::remove_file(&played_to);
    let started = Instant::now();
    std::thread::scope(|scope| {
        for _ in 0..at_once.min(jobs.len()) {
            scope.spawn(|| {
                loop {
                    let Some(index) = waiting.lock().expect("the jobs that wait").pop_front() else { break };
                    let began = started.elapsed().as_millis() as u64;
                    let ran = run(&jobs[index], threads, &played_to, &repo.root);
                    let ended = started.elapsed().as_millis() as u64;
                    done.lock().expect("the jobs that are done")[index] = Some(Done { ran, began, ended });
                }
            });
        }
    });
    let done = done.into_inner().expect("the jobs that are done");
    let played = repo.played();

    let mut problems = Vec::new();
    let mut count = 0;
    let mut slowest: Option<(&str, Took)> = None;
    for program in programs {
        let of_it = || jobs.iter().zip(&done).filter(|(job, _)| std::ptr::eq(job.program, program));
        let mut tests: Vec<Took> = Vec::new();
        let mut wrong = Vec::new();
        for (_, done) in of_it() {
            match done.as_ref().map(|done| &done.ran) {
                Some(Ok(ran)) => {
                    if !ran.ok {
                        wrong.extend(failures(&ran.text()));
                    }
                    tests.extend(clock::runs(&ran.lines, threads).into_iter().flat_map(|run| run.tests));
                }
                Some(Err(err)) => wrong.push(err.clone()),
                None => wrong.push(format!("{} was not run", program.path)),
            }
        }
        // Shared out, every test is still run: a share that named a test
        // the program did not know would run nothing and say it passed.
        let listed = of_it().find_map(|(job, _)| job.listed);
        if let Some(listed) = listed.filter(|listed| wrong.is_empty() && *listed != tests.len()) {
            wrong.push(format!(
                "it has {listed} tests and {} were run: some were lost in sharing them between processes",
                tests.len()
            ));
        }
        if !wrong.is_empty() {
            problems.push(format!("in `{}`:", program.krate));
            problems.extend(wrong);
        }
        // From when the first of its processes began to when the last ended.
        let spans = || of_it().filter_map(|(_, done)| done.as_ref());
        let began = spans().map(|done| done.began).min().unwrap_or(0);
        let ended = spans().map(|done| done.ended).max().unwrap_or(0);
        count += tests.len();
        for took in &tests {
            if slowest.as_ref().is_none_or(|(_, slowest)| took.millis > slowest.millis) {
                slowest = Some((&program.krate, took.clone()));
            }
        }
        repo.record_tests(repo::Ran::Gate, &program.krate, ended.saturating_sub(began), &tests, &played);
    }
    let shape = if at_once > 1 {
        format!(" in {} processes, {at_once} at a time on {threads} threads each", jobs.len())
    } else {
        String::new()
    };
    let timed = match slowest {
        Some((krate, took)) => format!(
            "{count} tests{shape}; the slowest took {:.1} s beside the others ({krate}, {})",
            took.millis as f64 / 1000.0,
            took.test
        ),
        None => "no tests".into(),
    };
    Outcome { problems, timed }
}

/// The tests' runner as a command of its own (`run-tests`), for a gate that
/// has them run somewhere it is not itself: given each program's crate,
/// folder and path, it runs them and says what came of it in lines the gate
/// reads (`heard`).
pub fn inside(repo: &Repo, args: &[String]) -> ExitCode {
    let programs: Vec<Program> = args
        .as_chunks::<3>()
        .0
        .iter()
        .map(|[krate, dir, path]| Program {
            krate: krate.clone(),
            dir: PathBuf::from(dir),
            path: path.clone(),
        })
        .collect();
    let outcome = run_all(repo, &programs);
    for problem in &outcome.problems {
        println!("{PROBLEM}{problem}");
    }
    println!("{TIMED}{}", outcome.timed);
    if outcome.problems.is_empty() { ExitCode::SUCCESS } else { ExitCode::FAILURE }
}

/// What the runner is told, to run `programs` as a command of its own.
pub fn asked(programs: &[Program]) -> Vec<String> {
    let mut asked = vec!["run-tests".to_string()];
    for program in programs {
        asked.extend([program.krate.clone(), program.dir.display().to_string(), program.path.clone()]);
    }
    asked
}

/// What a runner that was a command of its own said came of it.
pub fn heard(ran: &Ran) -> Outcome {
    let said = |before: &str| -> Vec<String> {
        ran.lines.iter().filter_map(|(_, line)| line.strip_prefix(before)).map(str::to_string).collect()
    };
    Outcome { problems: said(PROBLEM), timed: said(TIMED).pop().unwrap_or_else(|| "no tests".into()) }
}

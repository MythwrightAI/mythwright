//! Crowding a test: one test of a crate's suite run many times, several
//! at once, held to a few processors.
//!
//! A test that fails once in fifty runs fails for an order of two things
//! that a machine with processors to spare never comes to. Run again by
//! itself it passes, and run with the whole gate it fails too seldom to be
//! looked at. What brings the order about is more of the test at once than
//! there are processors for, over and over; and what makes the failure
//! worth having is everything the failing run printed, kept.
//!
//! This is that, apart from the building (`xtask/src/crowding.rs`), so
//! that it can be asked directly with a stand-in for the test program.
//! What must hold: a name that is no test of the program's is an error and
//! never a pass (a test program asked for a name it does not have runs
//! nothing and says all is well); every run is counted; a run that ran no
//! test is a failure; and the whole output of each run that failed is
//! there afterwards, and of no run that passed.

use std::fs::{self, File};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

/// How it is asked for, said where it is asked wrongly.
pub const USAGE: &str =
    "cargo xtask crowd <crate> <test> [--times N] [--at-once M] [--cpus K] [--limit SECONDS]";

/// The variable a run is told which one it is in, from 1.
pub const RUN: &str = "CROWD_RUN";

/// What was asked for.
#[derive(Clone, Debug, PartialEq)]
pub struct Asked {
    pub krate: String,
    /// The test's whole name, as the program lists it.
    pub test: String,
    /// How many times it is run in all.
    pub times: usize,
    /// How many of those runs go on side by side.
    pub at_once: usize,
    /// How many processors all of them are held to together.
    pub cpus: usize,
    /// How long one run may take before it is ended.
    pub limit: Duration,
}

/// What `args` ask for. Left unsaid: forty runs, four at once on two
/// processors, and ten minutes for a run. More at once than there are
/// processors for is the point: that is what a busy machine is.
pub fn asked(args: &[String]) -> Result<Asked, String> {
    let mut asked = Asked {
        krate: String::new(),
        test: String::new(),
        times: 40,
        at_once: 4,
        cpus: 2,
        limit: Duration::from_secs(600),
    };
    let mut named = Vec::new();
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        if !arg.starts_with("--") {
            named.push(arg.clone());
            continue;
        }
        let count = args
            .next()
            .and_then(|said| said.parse::<usize>().ok())
            .filter(|count| *count > 0)
            .ok_or_else(|| format!("`{arg}` takes a number, one or more\nusage: {USAGE}"))?;
        match arg.as_str() {
            "--times" => asked.times = count,
            "--at-once" => asked.at_once = count,
            "--cpus" => asked.cpus = count,
            "--limit" => asked.limit = Duration::from_secs(count as u64),
            _ => return Err(format!("`{arg}` is not something a crowd is told\nusage: {USAGE}")),
        }
    }
    let [krate, test] = named.as_slice() else {
        return Err(format!("a crowd is of one test of one crate: say both\nusage: {USAGE}"));
    };
    (asked.krate, asked.test) = (krate.clone(), test.clone());
    Ok(asked)
}

/// The test of `tests` that `name` is: its whole name and nothing less. A
/// test program asked for a name it has no test of runs none and says so
/// as a pass, so a name that is not one is refused here, with the tests
/// that have it in their names where there are any: the module was
/// probably left off.
pub fn named<'a>(tests: &'a [String], name: &str) -> Result<&'a str, String> {
    if let Some(test) = tests.iter().find(|test| *test == name) {
        return Ok(test);
    }
    let near: Vec<&str> =
        tests.iter().filter(|test| test.contains(name)).map(String::as_str).take(8).collect();
    Err(match near.as_slice() {
        [] => format!("no test is called `{name}`: nothing would be run, which is not the test passing"),
        near => format!(
            "no test is called `{name}`: a test is named whole, with its module. Those with that in their \
             names:\n  {}",
            near.join("\n  ")
        ),
    })
}

/// The processors this process may run on, read from what the system says
/// of it (`/proc/self/status`): the line `Cpus_allowed_list`, which is
/// ranges and single ones (`0-3,8,10-11`). `None` where that is not said.
pub fn allowed(status: &str) -> Option<Vec<usize>> {
    let list = status.lines().find_map(|line| line.strip_prefix("Cpus_allowed_list:"))?;
    let mut allowed = Vec::new();
    for part in list.trim().split(',') {
        let (first, last) = part.split_once('-').unwrap_or((part, part));
        let (first, last) = (first.trim().parse::<usize>().ok()?, last.trim().parse::<usize>().ok()?);
        allowed.extend(first..=last);
    }
    (!allowed.is_empty()).then_some(allowed)
}

/// The processors a crowd is held to: the first `cpus` of those this
/// process may itself run on, so that one started where it is already
/// held to some (a lane of a shared machine) stays among them. As `taskset`
/// takes them.
pub fn held(allowed: &[usize], cpus: usize) -> String {
    let held: Vec<String> = allowed.iter().take(cpus.max(1)).map(usize::to_string).collect();
    held.join(",")
}

/// One crowd, ready to be run.
pub struct Crowd {
    /// The test program, and the folder it is run from: its crate's, as
    /// cargo runs it.
    pub program: PathBuf,
    pub dir: PathBuf,
    pub test: String,
    pub times: usize,
    pub at_once: usize,
    /// The processors every run is held to, as `taskset` takes them; none
    /// where the system has no `taskset`, and the runs are then held to
    /// nothing.
    pub hold: Option<String>,
    pub limit: Duration,
    /// Where the output of a run that failed is kept. Whatever was there
    /// is taken away first: what is found there afterwards is this crowd's.
    pub keep: PathBuf,
}

/// How one run went.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Went {
    Passed,
    Failed,
    /// It was still going when a run's time was up, and was ended.
    Ended,
}

/// What a crowd came to. Runs are numbered from 1.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Came {
    pub ran: usize,
    /// The runs that failed: the test failed, or the program ran no test.
    pub failed: Vec<usize>,
    /// The runs that had not ended when their time was up.
    pub ended: Vec<usize>,
}

impl Came {
    /// Whether every run passed.
    pub fn ok(&self) -> bool {
        self.failed.is_empty() && self.ended.is_empty()
    }
}

/// Where the whole output of run `nth` is, if it failed.
pub fn kept(keep: &std::path::Path, nth: usize) -> PathBuf {
    keep.join(format!("run-{nth}.out"))
}

/// Whether what a test program printed says that it ran one test and the
/// test passed. Its ending well is not enough: one that ran nothing, or
/// passed over a test that is ignored, ends well too.
fn passed(printed: &str) -> bool {
    printed.lines().any(|line| line.starts_with("test result: ok. 1 passed;"))
}

/// One run: the program asked for the one test, everything it prints
/// written to the run's file.
fn once(crowd: &Crowd, nth: usize) -> Result<Went, String> {
    let file = kept(&crowd.keep, nth);
    let out = File::create(&file).map_err(|err| format!("could not write {}: {err}", file.display()))?;
    let aside = out.try_clone().map_err(|err| format!("could not write {}: {err}", file.display()))?;
    let mut command = match &crowd.hold {
        Some(hold) => {
            let mut command = Command::new("taskset");
            command.args(["-c", hold]).arg(&crowd.program);
            command
        }
        None => Command::new(&crowd.program),
    };
    // Nothing is captured: a run that is ended has then said what it had
    // to say before it was.
    command.args([crowd.test.as_str(), "--exact", "--nocapture"]);
    command.current_dir(&crowd.dir).env("CARGO_MANIFEST_DIR", &crowd.dir);
    // Which run it is, for whatever a test writes down that is to be told
    // apart afterwards.
    command.env(RUN, nth.to_string());
    command.stdin(Stdio::null()).stdout(out).stderr(aside);
    let mut child =
        command.spawn().map_err(|err| format!("could not run {}: {err}", crowd.program.display()))?;
    let began = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if began.elapsed() >= crowd.limit => {
                let _ = child.kill();
                let _ = child.wait();
                return Ok(Went::Ended);
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(20)),
            Err(err) => return Err(format!("could not wait for run {nth}: {err}")),
        }
    };
    let printed = fs::read(&file).map(|bytes| String::from_utf8_lossy(&bytes).into_owned());
    if status.success() && passed(&printed.unwrap_or_default()) {
        let _ = fs::remove_file(&file);
        return Ok(Went::Passed);
    }
    Ok(Went::Failed)
}

/// Runs the crowd. `said` is told of each run that did not pass as it
/// ends, so that whoever waits for a long one sees that it has found
/// something.
pub fn run(crowd: &Crowd, said: &(dyn Fn(&str) + Sync)) -> Result<Came, String> {
    let _ = fs::remove_dir_all(&crowd.keep);
    fs::create_dir_all(&crowd.keep)
        .map_err(|err| format!("could not make {}: {err}", crowd.keep.display()))?;
    let next = AtomicUsize::new(1);
    let came = Mutex::new(Came::default());
    let wrong: Mutex<Option<String>> = Mutex::new(None);
    std::thread::scope(|scope| {
        for _ in 0..crowd.at_once.clamp(1, crowd.times.max(1)) {
            scope.spawn(|| {
                loop {
                    let nth = next.fetch_add(1, Ordering::SeqCst);
                    if nth > crowd.times || wrong.lock().expect("what went wrong").is_some() {
                        break;
                    }
                    let went = match once(crowd, nth) {
                        Ok(went) => went,
                        Err(err) => {
                            wrong.lock().expect("what went wrong").get_or_insert(err);
                            break;
                        }
                    };
                    let mut came = came.lock().expect("what the crowd came to");
                    came.ran += 1;
                    let file = kept(&crowd.keep, nth);
                    match went {
                        Went::Passed => {}
                        Went::Failed => {
                            came.failed.push(nth);
                            said(&format!("run {nth} failed: {}", file.display()));
                        }
                        Went::Ended => {
                            came.ended.push(nth);
                            said(&format!(
                                "run {nth} had not ended in {} s and was ended: {}",
                                crowd.limit.as_secs(),
                                file.display()
                            ));
                        }
                    }
                }
            });
        }
    });
    if let Some(wrong) = wrong.into_inner().expect("what went wrong") {
        return Err(wrong);
    }
    let mut came = came.into_inner().expect("what the crowd came to");
    came.failed.sort_unstable();
    came.ended.sort_unstable();
    Ok(came)
}

/// What is said of a crowd when it is over, in a line.
pub fn summary(asked: &Asked, hold: Option<&str>, came: &Came) -> String {
    let on = match hold {
        Some(hold) => format!("held to processors {hold}"),
        None => "held to no processors".to_string(),
    };
    let ended = match came.ended.len() {
        0 => String::new(),
        ended => format!(", {ended} had not ended in {} s", asked.limit.as_secs()),
    };
    format!(
        "{} of {}: {} runs, {} at once, {on}: {} failed{ended}",
        asked.test,
        asked.krate,
        came.ran,
        asked.at_once,
        came.failed.len()
    )
}

//! What the tests left running: listed for whoever asks, and held by the
//! gates that run the test programs.
//!
//! The working out is `xtask::left`, which is asked as functions; this is
//! the two places it is asked from. Neither ends anything. A gate that
//! ended what a test left would pass over the test that left it, and the
//! next thing left would be found by nobody.

use std::sync::OnceLock;
use std::time::{Duration, Instant};

use xtask::left::{self, Left, Looked};

use crate::gates::GateResult;
use crate::repo::Repo;

/// How long something found after the tests is given to be gone before it
/// is said to have been left, and how often it is looked at meanwhile. A
/// guard that ends what its test started has asked by the time the test
/// program is over, and what was asked is then on its way out: a signal
/// taken, files put away. Nothing says from outside that a process which
/// is not one's own has ended but its no longer being there, so that is
/// what is waited for, and the time is only the bound: nothing is waited
/// where nothing was found, which is every gate that passes.
const GONE_WITHIN: Duration = Duration::from_secs(3);
const LOOKED_EVERY: Duration = Duration::from_millis(25);

/// A look by this program, which finds out once who takes in what it
/// leaves behind.
fn look() -> Result<Looked, String> {
    static ADOPTER: OnceLock<Option<u32>> = OnceLock::new();
    left::look(*ADOPTER.get_or_init(|| left::adopter(&std::env::current_exe().ok()?)))
}

/// How `cargo xtask left` is asked.
pub const USAGE: &str = "cargo xtask left [--none] [--after PID] [--but NUMBERS] [--numbers]";

/// `cargo xtask left`: what is running that is this checkout's. It says,
/// and ends nothing. Asked `--none`, it fails where anything is left that
/// nobody waits for, having given what it found the time to be gone that
/// a gate gives.
///
/// The rest is for a check of the client, which looks as it begins and
/// as it exits (`ui/scripts/ends.mjs`): `--numbers` says only which
/// processes are left, each as its number and when it began; `--but`
/// leaves out those it is handed, said so; and `--after PID` counts what
/// came from that process as nobody's, since it is about to end.
pub fn list(repo: &Repo, args: &[String]) -> Result<bool, String> {
    let (mut none, mut numbers, mut after) = (false, false, None);
    let mut but: Vec<(u32, u64)> = Vec::new();
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--none" => none = true,
            "--numbers" => numbers = true,
            "--after" => {
                let pid = args.next().and_then(|pid| pid.parse::<u32>().ok());
                after = Some(pid.ok_or(format!("`--after` is followed by a process's number: {USAGE}"))?);
                none = true;
            }
            "--but" => {
                let said =
                    args.next().ok_or(format!("`--but` is followed by what `--numbers` said: {USAGE}"))?;
                but.extend(said.split(',').filter_map(|one| {
                    let (pid, began) = one.split_once(':')?;
                    Some((pid.parse().ok()?, began.parse().ok()?))
                }));
            }
            other => return Err(format!("`{other}` is not something `left` is asked: {USAGE}")),
        }
    }
    let began = Instant::now();
    let looked = match look() {
        Ok(looked) => looked,
        Err(why) => {
            if !numbers {
                println!("{why}");
            }
            return Ok(true);
        }
    };
    let found = left::lying(&looked, &repo.root, after);
    let took = began.elapsed().as_millis();
    let (nobodys, waited): (Vec<Left>, Vec<Left>) = found
        .into_iter()
        .filter(|left| !but.contains(&(left.seen.pid, left.seen.began)))
        .partition(|left| left.nobodys);
    if numbers {
        let said: Vec<String> =
            nobodys.iter().map(|left| format!("{}:{}", left.seen.pid, left.seen.began)).collect();
        println!("{}", said.join(","));
        return Ok(true);
    }
    // Where it is to fail for what is left, what is on its way out is not.
    let nobodys = if none { settled(nobodys) } else { nobodys };
    if nobodys.is_empty() {
        println!("nothing is left running here ({} processes looked at in {took} ms)", looked.seen.len());
    } else {
        println!(
            "left running here, with nobody waiting for it ({} processes looked at in {took} ms); nothing was ended:",
            looked.seen.len()
        );
        for left in &nobodys {
            println!("  {}", left::said(left, looked.at, &repo.root));
        }
    }
    if !waited.is_empty() && !none {
        println!("running from this checkout's target/, and somebody waits for it:");
        for left in &waited {
            println!("  {}", left::said(left, looked.at, &repo.root));
        }
    }
    Ok(nobodys.is_empty() || !none)
}

/// What is left of `found` once what was on its way out has gone.
fn settled(mut found: Vec<Left>) -> Vec<Left> {
    let began = Instant::now();
    loop {
        found.retain(|left| left::still(&left.seen));
        if found.is_empty() || began.elapsed() >= GONE_WITHIN {
            return found;
        }
        std::thread::sleep(LOOKED_EVERY);
    }
}

/// Runs the test programs with `run`, and fails for what they left
/// running: what is there afterwards, was not there before, is this
/// checkout's and has nobody waiting for it. What `run` came to is said
/// either way.
pub fn held(repo: &Repo, run: impl FnOnce() -> GateResult) -> GateResult {
    let began = Instant::now();
    let before = look();
    let mut looking = began.elapsed();
    let result = run();
    let began = Instant::now();
    let (before, after): (Looked, Looked) = match (before, look()) {
        (Ok(before), Ok(after)) => (before, after),
        // A system with nothing to read it from: the tests are still tests.
        _ => return result.map(|note| format!("{note}. What was left running was not looked for")),
    };
    let found = left::new(&before, &after, &repo.root);
    looking += began.elapsed();
    let found = settled(found);
    if found.is_empty() {
        // What the two looks took is said, since it is the gate's time.
        return result.map(|note| {
            format!("{note}. Nothing was left running (looked for in {} ms)", looking.as_millis())
        });
    }
    let mut problems = result.err().unwrap_or_default();
    problems.push("the tests left these running, and nothing here ended them:".into());
    problems.extend(found.iter().map(|left| format!("  {}", left::said(left, after.at, &repo.root))));
    problems.push(left::MUST.into());
    Err(problems)
}

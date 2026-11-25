//! `cargo xtask crowd <crate> <test>`: one test, many times, several at
//! once on a few processors.
//!
//! The test program is built as the gate builds it, in the one build with
//! every crate in it (`suites::built_whole`), so that what is crowded is
//! the program the gate runs and nothing is compiled a second way. What is
//! done with it is `xtask::crowd`.

use std::path::Path;
use std::process::{Command, Stdio};

use xtask::crowd::{self, Crowd};

use crate::repo::Repo;
use crate::suites;

/// What git says of where the work stands, for a result that names what
/// it is a result of: the commit, and whether anything is changed from it.
pub fn standing(root: &Path) -> String {
    let git = |args: &[&str]| -> Option<String> {
        let output = Command::new("git").args(args).current_dir(root).output().ok()?;
        output.status.success().then(|| String::from_utf8_lossy(&output.stdout).trim().to_string())
    };
    match (git(&["rev-parse", "--short", "HEAD"]), git(&["status", "--short"])) {
        (Some(commit), Some(changed)) if changed.is_empty() => format!("at {commit}"),
        (Some(commit), _) => format!("at {commit}, with changes that are not committed"),
        _ => "in a folder that is no repository".to_string(),
    }
}

/// The processors the runs are held to, as `taskset` takes them; or why
/// they are held to none.
fn hold(cpus: usize) -> Result<String, String> {
    let there = Command::new("taskset")
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success());
    if !there {
        return Err("this system has no `taskset`".to_string());
    }
    let allowed =
        std::fs::read_to_string("/proc/self/status").ok().and_then(|status| crowd::allowed(&status));
    match allowed {
        Some(allowed) => Ok(crowd::held(&allowed, cpus)),
        None => Err("this system does not say which processors a program may use".to_string()),
    }
}

pub fn run(repo: &Repo, args: &[String]) -> Result<bool, String> {
    let asked = crowd::asked(args)?;
    println!("building the test programs, as the gate does");
    let programs = suites::built_whole(repo).map_err(|problems| problems.join("\n"))?;
    let Some(program) = programs.iter().find(|program| program.krate == asked.krate) else {
        let mut crates: Vec<&str> = programs.iter().map(|program| program.krate.as_str()).collect();
        crates.sort_unstable();
        return Err(format!(
            "no crate called `{}` has a test program; those that have: {}",
            asked.krate,
            crates.join(", ")
        ));
    };
    let tests = suites::asked_of(program, &["--list", "--format", "terse"])
        .map_err(|said| format!("{} would not list its tests:\n{said}", program.path))?;
    let tests: Vec<String> =
        tests.lines().filter_map(|line| line.strip_suffix(": test")).map(str::to_string).collect();
    let test = crowd::named(&tests, &asked.test)?.to_string();
    let hold = match hold(asked.cpus) {
        Ok(hold) => Some(hold),
        Err(why) => {
            println!("{why}: the runs are held to no processors, and are crowded only by one another");
            None
        }
    };
    let folder: String =
        test.chars().map(|c| if c.is_alphanumeric() || c == '_' { c } else { '-' }).collect();
    let crowd = Crowd {
        program: program.path.clone().into(),
        dir: program.dir.clone(),
        test,
        times: asked.times,
        at_once: asked.at_once,
        hold,
        limit: asked.limit,
        keep: repo.root.join("target/xtask/crowd").join(&asked.krate).join(folder),
    };
    println!("{}: {}", standing(&repo.root), program.path);
    let came = crowd::run(&crowd, &|line| println!("{line}"))?;
    println!("{}", crowd::summary(&asked, crowd.hold.as_deref(), &came));
    if !came.ok() {
        println!("what each of those runs printed is in {}", crowd.keep.display());
    }
    Ok(came.ok())
}

//! What the tooling needs to know about the repository.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use xtask::clock::Took;
use xtask::played::Played;

pub struct Repo {
    pub root: PathBuf,
}

pub struct Timing {
    pub gate: String,
    pub millis: u128,
}

/// A crate's tests, as they were last run.
pub struct Suite {
    pub krate: String,
    pub millis: u64,
    pub tests: usize,
}

/// Which run of the tests something was kept of. The two are kept apart
/// because they are two figures: a gate runs the programs side by side
/// where there is room, or held to a few processors, and what the budget
/// holds is every program one after another on every thread.
#[derive(Clone, Copy)]
pub enum Ran {
    /// As a gate last ran each crate's.
    Gate,
    /// As `cargo xtask build-times` last ran them all, which is as cargo does.
    Whole,
}

/// What was kept of a run of the tests: each crate's suite with how long
/// it ran and how many tests it has, each test with how long it took,
/// what each test that plays played, and, of a run of them all, how long
/// that was from end to end.
#[derive(Default)]
pub struct Kept {
    pub suites: Vec<Suite>,
    pub tests: Vec<(String, Took)>,
    pub played: Vec<Played>,
    pub whole: Option<u64>,
}

/// A workspace member and what it depends on.
pub struct Member {
    pub name: String,
    /// Directory relative to the root.
    pub dir: String,
    pub manifest: String,
    /// Names from `[dependencies]`, `[dev-dependencies]` and `[build-dependencies]`.
    pub deps: Vec<Dep>,
}

pub struct Dep {
    pub name: String,
    pub section: String,
    /// The right-hand side as written.
    pub spec: String,
    pub line: usize,
}

impl Repo {
    /// The directory holding `brand.toml`, at or above the current one.
    pub fn find() -> Result<Repo, String> {
        let mut dir = std::env::current_dir().map_err(|err| err.to_string())?;
        loop {
            if dir.join("brand.toml").is_file() && dir.join("Cargo.toml").is_file() {
                return Ok(Repo { root: dir });
            }
            if !dir.pop() {
                return Err("run this from inside the repository (no brand.toml found)".into());
            }
        }
    }

    pub fn read(&self, path: &str) -> Option<String> {
        fs::read_to_string(self.root.join(path)).ok()
    }

    /// A value from `brand.toml`.
    pub fn brand(&self, key: &str) -> Option<String> {
        brand_value(&self.read("brand.toml")?, key)
    }

    /// Every file git would track: committed, staged or new, minus ignored.
    pub fn files(&self) -> Vec<String> {
        let output = Command::new("git")
            .args(["ls-files", "--cached", "--others", "--exclude-standard"])
            .current_dir(&self.root)
            .output();
        let Ok(output) = output else { return Vec::new() };
        let mut files: Vec<String> = String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter(|path| self.root.join(path).is_file())
            .map(str::to_string)
            .collect();
        files.sort();
        files.dedup();
        files
    }

    /// Files that differ from the last commit, plus new ones. With no commits
    /// yet, that is every file.
    pub fn changed_files(&self) -> Vec<String> {
        let git = |args: &[&str]| -> Option<String> {
            let output = Command::new("git").args(args).current_dir(&self.root).output().ok()?;
            output.status.success().then(|| String::from_utf8_lossy(&output.stdout).into_owned())
        };
        // Relative to here, not to the top of the repository: the application
        // may be one folder among several in it.
        let Some(diff) = git(&["diff", "--name-only", "--relative", "HEAD"]) else { return self.files() };
        let untracked = git(&["ls-files", "--others", "--exclude-standard"]).unwrap_or_default();
        let mut files: Vec<String> = diff.lines().chain(untracked.lines()).map(str::to_string).collect();
        files.sort();
        files.dedup();
        files
    }

    /// Workspace members, found by their manifests.
    pub fn members(&self) -> Vec<Member> {
        let mut members = Vec::new();
        for file in self.files() {
            let Some(dir) = file.strip_suffix("/Cargo.toml") else { continue };
            let Some(manifest) = self.read(&file) else { continue };
            let Some(name) = package_name(&manifest) else { continue };
            members.push(Member { name, dir: dir.to_string(), deps: dependencies(&manifest), manifest });
        }
        members
    }

    pub fn record_timing(&self, gate: &str, millis: u128, ok: bool) {
        let path = self.timings_path();
        let _ = path.parent().map(fs::create_dir_all);
        let now = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |elapsed| elapsed.as_secs());
        if let Ok(mut file) = fs::OpenOptions::new().create(true).append(true).open(path) {
            let _ = writeln!(file, "{now}\t{gate}\t{millis}\t{}", if ok { "ok" } else { "fail" });
        }
    }

    pub fn timings(&self) -> Vec<Timing> {
        let Ok(text) = fs::read_to_string(self.timings_path()) else { return Vec::new() };
        text.lines()
            .filter_map(|line| {
                let mut fields = line.split('\t');
                let (_, gate, millis) = (fields.next()?, fields.next()?, fields.next()?);
                Some(Timing { gate: gate.to_string(), millis: millis.parse().ok()? })
            })
            .collect()
    }

    fn timings_path(&self) -> PathBuf {
        self.root.join("target").join("xtask").join("gate-timings.tsv")
    }

    /// Keeps how long a crate's tests took the last time they were run
    /// this way, and what those that play played, in place of what was
    /// kept of that crate before.
    pub fn record_tests(&self, ran: Ran, krate: &str, millis: u64, tests: &[Took], played: &[Played]) {
        let path = self.tests_path(ran);
        let _ = path.parent().map(fs::create_dir_all);
        let kept = fs::read_to_string(&path).unwrap_or_default();
        let mut text: String = kept
            .lines()
            .filter(|line| line.split('\t').nth(1) != Some(krate))
            .map(|line| format!("{line}\n"))
            .collect();
        text.push_str(&format!("suite\t{krate}\t{millis}\t{}\n", tests.len()));
        for took in tests {
            text.push_str(&format!("test\t{krate}\t{}\t{}\n", took.millis, took.test));
        }
        for one in played.iter().filter(|one| one.krate == krate) {
            text.push_str(&format!("played\t{krate}\t{}\t{}\t{}\n", one.places, one.plays, one.test));
        }
        let _ = fs::write(path, text);
    }

    /// Keeps how long a run of every test took from end to end, and
    /// forgets every crate that was kept of the run before it: a crate
    /// that is gone, or whose tests were not read, is not left there
    /// from an earlier day beside the rest.
    pub fn record_whole(&self, millis: u64) {
        let path = self.tests_path(Ran::Whole);
        let _ = path.parent().map(fs::create_dir_all);
        let _ = fs::write(path, format!("whole\t-\t{millis}\t-\n"));
    }

    /// What was last kept of the tests, run this way.
    pub fn tests(&self, ran: Ran) -> Kept {
        let text = fs::read_to_string(self.tests_path(ran)).unwrap_or_default();
        let mut kept = Kept::default();
        for line in text.lines() {
            let mut fields = line.splitn(4, '\t');
            let (Some(kind), Some(krate), Some(count), Some(rest)) =
                (fields.next(), fields.next(), fields.next(), fields.next())
            else {
                continue;
            };
            let Ok(count) = count.parse::<u64>() else { continue };
            match kind {
                "suite" => kept.suites.push(Suite {
                    krate: krate.to_string(),
                    millis: count,
                    tests: rest.parse().unwrap_or(0),
                }),
                "test" => {
                    kept.tests.push((krate.to_string(), Took { test: rest.to_string(), millis: count }))
                }
                "played" => {
                    let Some((plays, test)) = rest.split_once('\t') else { continue };
                    let Ok(plays) = plays.parse::<u64>() else { continue };
                    kept.played.push(Played {
                        krate: krate.to_string(),
                        test: test.to_string(),
                        places: count,
                        plays,
                    });
                }
                "whole" => kept.whole = Some(count),
                _ => {}
            }
        }
        kept
    }

    /// What is kept of the tests run this way, as it is written down:
    /// for whoever is about to run them some other way than the one it
    /// was kept of, and puts it back afterwards ([`Repo::keep_tests_as`]).
    pub fn tests_as_kept(&self, ran: Ran) -> Option<String> {
        fs::read_to_string(self.tests_path(ran)).ok()
    }

    /// Puts back what was kept of the tests run this way; nothing, where
    /// nothing was.
    pub fn keep_tests_as(&self, ran: Ran, kept: Option<&str>) {
        let path = self.tests_path(ran);
        let _ = match kept {
            Some(kept) => fs::write(path, kept),
            None => fs::remove_file(path),
        };
    }

    /// Where a suite that plays is told to write down what it played, by
    /// this process: a file of its own, so that two runs side by side in
    /// one tree do not read each other's.
    pub fn played_path(&self) -> PathBuf {
        self.root.join("target").join("xtask").join(format!("played-{}.tsv", std::process::id()))
    }

    /// What was written down there, and the file taken away.
    pub fn played(&self) -> Vec<Played> {
        let path = self.played_path();
        let text = fs::read_to_string(&path).unwrap_or_default();
        let _ = fs::remove_file(path);
        xtask::played::read(&text)
    }

    fn tests_path(&self, ran: Ran) -> PathBuf {
        let file = match ran {
            Ran::Gate => "test-times.tsv",
            Ran::Whole => "every-test.tsv",
        };
        self.root.join("target").join("xtask").join(file)
    }
}

/// Reads `key = "value"` from the flat format `brand.toml` uses.
pub fn brand_value(text: &str, key: &str) -> Option<String> {
    text.lines().find_map(|line| {
        let (name, value) = line.split_once('=')?;
        (name.trim() == key).then(|| value.trim().trim_matches('"').to_string())
    })
}

fn package_name(manifest: &str) -> Option<String> {
    let mut in_package = false;
    for line in manifest.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_package = line == "[package]";
        } else if in_package && let Some(value) = line.strip_prefix("name") {
            return Some(value.trim_start_matches([' ', '=']).trim().trim_matches('"').to_string());
        }
    }
    None
}

fn dependencies(manifest: &str) -> Vec<Dep> {
    let mut deps = Vec::new();
    let mut section = String::new();
    for (index, raw) in manifest.lines().enumerate() {
        let line = raw.trim();
        if line.starts_with('[') {
            section = line.trim_matches(['[', ']']).to_string();
            // `[dependencies.foo]` style tables are not used here; the
            // manifest gate reports them.
            continue;
        }
        if !section.ends_with("dependencies") || line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((name, spec)) = line.split_once('=') {
            deps.push(Dep {
                name: name.trim().to_string(),
                section: section.clone(),
                spec: spec.trim().to_string(),
                line: index + 1,
            });
        }
    }
    deps
}

/// Lines in a file, or zero if it cannot be read as text.
pub fn line_count(path: &Path) -> usize {
    fs::read_to_string(path).map_or(0, |text| text.lines().count())
}

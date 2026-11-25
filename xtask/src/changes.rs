//! `cargo xtask changes`: of the changes made, how many needed a rebuild.
//!
//! The product rests on a claim: what changes often is configuration (a
//! blueprint, a package, the client), which is read while the daemon runs,
//! and not the code the daemon is compiled from. If that holds, the share
//! of commits that need the daemon built again falls as the product
//! settles. This reads the history and says what the share has been, a
//! week at a time, so that the claim is a number and not a belief.
//!
//! A commit is in one group, the first of these that a file it touched
//! fits:
//!
//! - `rebuild`: anything under `crates/`, `desktop/` or `xtask/`, and
//!   `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml` and `brand.toml`,
//!   which is not Rust and is compiled in all the same: the `brand`
//!   crate's build script turns it into constants;
//! - `client`: anything under `ui/`;
//! - `text`: anything under `blueprints/`, `extensions/` or `examples/`;
//! - `notes`: everything else.
//!
//! A commit that touched no file here is not a change here, and is not
//! counted. That is a merge, whose commits are counted already, one by
//! one; and where the repository holds more than the application, a
//! commit made wholly outside it.
//!
//! Nothing of the history is worked out here that git can be asked for:
//! which commits, the day of each, the files each touched. `--since` is
//! not read here either. Git is asked whether it names a commit. If it
//! does, what is counted is what the branch has that the commit has not;
//! if it does not, it is handed to `git log --since` as it is, and what
//! git makes of it as a date is what it means.

use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;
use std::process::{Command, Output};

const USAGE: &str = "usage: cargo xtask changes [--since <ref-or-date>]";

/// Where everything is compiled: the daemon, the desktop program, this tooling.
const COMPILED: &[&str] = &["crates/", "desktop/", "xtask/"];
/// What decides how all of it is compiled, and `brand.toml`, which is
/// compiled in: the `brand` crate's build script turns it into constants,
/// so a commit that touched it is a daemon to build again. These are the
/// ones at the top; a crate's own manifest is under its folder.
const MANIFESTS: &[&str] = &["Cargo.toml", "Cargo.lock", "rust-toolchain.toml", "brand.toml"];
const CLIENT: &[&str] = &["ui/"];
/// Blueprints, packages, and the examples.
const TEXT: &[&str] = &["blueprints/", "extensions/", "examples/"];

/// What is asked of git: every commit the branch has, as a line that is a
/// NUL and the day it was committed, and under it the files it touched.
///
/// No path has a NUL in it, so a line that begins with one is a commit's
/// and nothing else's. The day is the one the commit itself says
/// (`--date=short`), so that the same history is the same table wherever
/// it is read, and it is the day it was committed because that is the one
/// `--since` goes by. Paths are from this folder (`--relative`), for a
/// repository that holds more than the application, and are printed as
/// they are and not with their letters written as numbers (`quotePath`).
/// A file that was moved is two, where it was and where it is
/// (`--no-renames`): one moved out of a crate changed that crate.
const LOG: &[&str] = &[
    "-c",
    "core.quotePath=false",
    "log",
    "--name-only",
    "--no-renames",
    "--relative",
    "--date=short",
    "--format=%x00%cd",
];

/// What a commit asked of whoever runs the product, by the files it touched.
///
/// In the order they are tried, which is the order of the table's columns.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Group {
    /// The daemon has to be built again.
    Rebuild,
    /// The client is built again, and the daemon is not.
    Client,
    /// A blueprint, a package or an example: nothing is built.
    Text,
    /// Nothing that runs.
    Notes,
}

/// Whether any of `paths` is under one of `folders`.
fn under(paths: &[&str], folders: &[&str]) -> bool {
    paths.iter().any(|path| folders.iter().any(|folder| path.starts_with(*folder)))
}

/// The group of a commit that touched `paths`: the first one that any of
/// them fits. Paths are from the top of the application, as git writes them.
pub fn group(paths: &[&str]) -> Group {
    if under(paths, COMPILED) || paths.iter().any(|path| MANIFESTS.contains(path)) {
        Group::Rebuild
    } else if under(paths, CLIENT) {
        Group::Client
    } else if under(paths, TEXT) {
        Group::Text
    } else {
        Group::Notes
    }
}

/// A day of the calendar, counted from the first of January 1970.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Day(i64);

impl Day {
    /// Reads `2026-11-09`, which is how git writes a day, and nothing else.
    pub fn parse(text: &str) -> Option<Day> {
        let mut parts = text.split('-').map(|part| part.parse::<i64>().ok());
        let (year, month, day) = (parts.next()??, parts.next()??, parts.next()??);
        let sound = (0..=9999).contains(&year) && (1..=12).contains(&month) && (1..=31).contains(&day);
        if parts.next().is_some() || !sound {
            return None;
        }
        // A day the month does not have (the thirtieth of February) comes
        // back as a day of the next month, and so not as it was written.
        let read = Day(days_from_civil(year, month, day));
        (read.to_string() == text).then_some(read)
    }

    /// The Monday of the week this day is in. The first day counted was a
    /// Thursday, three days into its week.
    pub fn monday(self) -> Day {
        Day(self.0 - (self.0 + 3).rem_euclid(7))
    }
}

impl fmt::Display for Day {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (year, month, day) = civil_from_days(self.0);
        write!(f, "{year:04}-{month:02}-{day:02}")
    }
}

// The two below are the usual arithmetic for the calendar (Howard Hinnant's
// `days_from_civil` and `civil_from_days`), written out because this crate
// uses the standard library alone. A year is counted from March, so that
// the day a leap year adds is its last, and years are taken four hundred at
// a time, which is how long the calendar takes to repeat: 146,097 days.

/// Days from the first of January 1970 to a day of the calendar.
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let year_of_era = year.rem_euclid(400);
    let month_from_march = if month > 2 { month - 3 } else { month + 9 };
    let day_of_year = (153 * month_from_march + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    year.div_euclid(400) * 146_097 + day_of_era - 719_468
}

/// The year, month and day that is so many days from the first of January 1970.
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let days = days + 719_468;
    let day_of_era = days.rem_euclid(146_097);
    let year_of_era = (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_from_march = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_from_march + 2) / 5 + 1;
    let month = if month_from_march < 10 { month_from_march + 3 } else { month_from_march - 9 };
    let year = year_of_era + days.div_euclid(146_097) * 400 + i64::from(month <= 2);
    (year, month, day)
}

/// A commit, as far as the question goes: the day it was made, and its group.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Commit {
    pub day: Day,
    pub group: Group,
}

/// A path as git printed it, without the quote it puts before one that has
/// a quote, a backslash or a tab in it. What such a path begins with is
/// still where it is, which is all that is asked of it.
fn unquoted(path: &str) -> &str {
    path.strip_prefix('"').unwrap_or(path)
}

/// The commits in what git printed when asked as `LOG` asks.
pub fn read(log: &str) -> Result<Vec<Commit>, String> {
    let mut commits = Vec::new();
    // Before the first commit's line there is nothing.
    for commit in log.split('\0').skip(1) {
        let mut lines = commit.lines();
        let date = lines.next().unwrap_or_default();
        let Some(day) = Day::parse(date) else {
            return Err(format!(
                "git gave a commit the day `{date}`, which is not a day written the way 2026-11-09 is"
            ));
        };
        let paths: Vec<&str> = lines.filter(|line| !line.is_empty()).map(unquoted).collect();
        if !paths.is_empty() {
            commits.push(Commit { day, group: group(&paths) });
        }
    }
    Ok(commits)
}

/// How many commits were in each group, in the order of `Group`.
type Counts = [usize; 4];

/// The share that needed a rebuild, as a whole percent. Half a percent and
/// over is the next one up.
fn share(counts: Counts) -> String {
    let all: usize = counts.iter().sum();
    format!("{}%", (counts[Group::Rebuild as usize] * 200 + all) / (all * 2).max(1))
}

/// One row. The headings are a row too, so that each is over what it names.
fn row<T: fmt::Display>(name: &str, [rebuild, client, text, notes]: [T; 4], share: &str) -> String {
    format!("{name:<10} {rebuild:>7} {client:>7} {text:>7} {notes:>7} {share:>9}\n")
}

/// The table: a row for each week in which there was a commit, named by
/// the Monday it began and oldest first, and a last row for everything.
pub fn table(commits: &[Commit]) -> String {
    let mut weeks: BTreeMap<Day, Counts> = BTreeMap::new();
    let mut all: Counts = [0; 4];
    for commit in commits {
        weeks.entry(commit.day.monday()).or_default()[commit.group as usize] += 1;
        all[commit.group as usize] += 1;
    }
    let mut rows = row("week", ["rebuild", "client", "text", "notes"], "rebuild %");
    for (monday, counts) in weeks {
        rows.push_str(&row(&monday.to_string(), counts, &share(counts)));
    }
    rows.push_str(&row("all", all, &share(all)));
    rows
}

/// Where the count begins, when that is not the first commit.
enum Since {
    /// A commit, by any name git has for it.
    Commit(String),
    /// Anything else, which git reads as a date.
    Date(String),
}

impl Since {
    fn of(root: &Path, value: &str) -> Result<Since, String> {
        let value = value.to_string();
        if names_a_commit(root, &value)? { Ok(Since::Commit(value)) } else { Ok(Since::Date(value)) }
    }

    /// What `git log` is given, to begin there.
    fn range(&self) -> String {
        match self {
            Since::Commit(name) => format!("{name}..HEAD"),
            Since::Date(date) => format!("--since={date}"),
        }
    }
}

/// What `--since` was given, if it was given. Nothing else is taken, so
/// that a word mistyped is not quietly a count of everything.
fn asked_since(args: &[String]) -> Result<Option<&str>, String> {
    let value = match args {
        [] => return Ok(None),
        [flag, value] if flag == "--since" => value.as_str(),
        [only] => only.strip_prefix("--since=").unwrap_or_default(),
        _ => "",
    };
    if value.is_empty() {
        let given = args.join(" ");
        return Err(format!("`changes` was given `{given}`, which it cannot read; {USAGE}"));
    }
    Ok(Some(value))
}

/// Asks git something, in the repository.
fn git(root: &Path, args: &[&str]) -> Result<Output, String> {
    let asked = Command::new("git").args(args).current_dir(root).output();
    asked.map_err(|err| format!("the history is read with git, which could not be run: {err}"))
}

/// What git printed, or why it would not, in its own words.
fn printed(output: &Output) -> Result<String, String> {
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        Err(format!("git would not read the history: {}", String::from_utf8_lossy(&output.stderr).trim()))
    }
}

/// Whether git knows `name` as a commit: a branch, a tag, a hash, or any
/// of its ways of writing one.
fn names_a_commit(root: &Path, name: &str) -> Result<bool, String> {
    let commit = format!("{name}^{{commit}}");
    Ok(git(root, &["rev-parse", "--verify", "--quiet", commit.as_str()])?.status.success())
}

/// The commits of the branch this is run on, from `since` if that is given.
fn history(root: &Path, since: Option<&Since>) -> Result<Vec<Commit>, String> {
    // A branch with nothing on it yet has no history, which `git log` calls
    // a failure and is not one. Where there is no repository at all it is,
    // and git says so.
    if !names_a_commit(root, "HEAD")? {
        printed(&git(root, &["rev-parse", "--git-dir"])?)?;
        return Ok(Vec::new());
    }
    let range = since.map(Since::range);
    let mut args: Vec<&str> = LOG.to_vec();
    args.extend(range.as_deref());
    read(&printed(&git(root, &args)?)?)
}

/// What is said when there is nothing to count.
fn nothing(since: Option<&Since>) -> String {
    match since {
        None => "There are no commits here yet, so there is nothing to count.".to_string(),
        Some(Since::Commit(name)) => format!("There are no commits since `{name}`."),
        // Said, because a name mistyped is a date to git as well, and this
        // is then the only sign of it.
        Some(Since::Date(date)) => {
            let read = "It names no commit here, so git read it as a date.";
            format!("There are no commits since `{date}`. {read}")
        }
    }
}

/// `cargo xtask changes`, for the repository at `root`.
pub fn run(root: &Path, args: &[String]) -> Result<(), String> {
    let since = asked_since(args)?.map(|value| Since::of(root, value)).transpose()?;
    let commits = history(root, since.as_ref())?;
    if commits.is_empty() {
        println!("{}", nothing(since.as_ref()));
    } else {
        print!("{}", table(&commits));
    }
    Ok(())
}

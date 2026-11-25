//! `cargo xtask changes`: the group a commit is in, the table a week at a
//! time, and the history as git gives it.
//!
//! Which group, which week and what table are asked of the functions, of
//! commits written here. The tests at the end make small repositories:
//! what is asked of git, and what is made of its answer, are held to each
//! other only by asking it.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

use xtask::changes::{Commit, Day, Group, group, read, table};

const XTASK: &str = env!("CARGO_BIN_EXE_xtask");

fn commit(day: &str, group: Group) -> Commit {
    Commit { day: Day::parse(day).unwrap(), group }
}

/// What a table says, without the space that lines its columns up.
fn rows(table: &str) -> Vec<String> {
    table.lines().map(|line| line.split_whitespace().collect::<Vec<_>>().join(" ")).collect()
}

/// A history as git prints it for `changes`, with `@` written here for the
/// NUL that begins each commit's line.
fn printed(text: &str) -> String {
    text.replace('@', "\0")
}

#[test]
fn a_commit_that_touched_anything_compiled_needed_a_rebuild() {
    assert_eq!(group(&["crates/api/src/lib.rs"]), Group::Rebuild);
    assert_eq!(group(&["desktop/src/main.rs"]), Group::Rebuild);
    assert_eq!(group(&["xtask/src/gates.rs"]), Group::Rebuild);
    for manifest in ["Cargo.toml", "Cargo.lock", "rust-toolchain.toml"] {
        assert_eq!(group(&[manifest]), Group::Rebuild, "{manifest}");
    }
}

#[test]
fn brand_toml_is_compiled_in_so_a_commit_that_touched_it_needed_a_rebuild() {
    assert_eq!(group(&["brand.toml"]), Group::Rebuild);
    // A rename of the product touches it and the notes, and nothing else.
    assert_eq!(group(&["README.md", "brand.toml", "docs/architecture.md"]), Group::Rebuild);
    // The build reads the one at the top and no other.
    assert_eq!(group(&["docs/brand.toml"]), Group::Notes);
}

#[test]
fn a_commit_is_in_the_first_group_that_any_file_it_touched_fits() {
    let touched = ["README.md", "blueprints/std/board.bp", "ui/src/api.ts", "crates/cli/src/help.rs"];
    assert_eq!(group(&touched), Group::Rebuild);
    assert_eq!(group(&touched[..3]), Group::Client);
    assert_eq!(group(&touched[..2]), Group::Text);
    assert_eq!(group(&touched[..1]), Group::Notes);
}

#[test]
fn blueprints_packages_and_examples_are_text() {
    assert_eq!(group(&["blueprints/std/working.bp"]), Group::Text);
    assert_eq!(group(&["extensions/first/extension.yaml"]), Group::Text);
    assert_eq!(group(&["examples/scenarios/flaky-build.bp", "README.md"]), Group::Text);
    // An example that is a program is built for itself. The daemon is not.
    assert_eq!(group(&["examples/kv-cache/src/main.rs"]), Group::Text);
}

#[test]
fn everything_else_is_notes() {
    for path in ["README.md", "docs/build-times.md", ".gitignore", "build-budget.toml"] {
        assert_eq!(group(&[path]), Group::Notes, "{path}");
    }
    // A name that only begins like a folder's is not in it, and a manifest
    // that is not at the top decides nothing about the build.
    let near = ["uint.md", "cratesmanship/notes.md", "docs/crates/Cargo.toml"];
    assert_eq!(group(&near), Group::Notes);
    assert_eq!(group(&[]), Group::Notes);
}

#[test]
fn a_week_begins_on_its_monday_across_months_years_and_leap_days() {
    let weeks = [
        ("2026-11-09", "2026-11-09"),
        ("2026-11-12", "2026-11-09"),
        ("2026-11-15", "2026-11-09"),
        ("2026-11-16", "2026-11-16"),
        ("2026-11-01", "2026-10-26"),
        ("2027-01-01", "2026-12-28"),
        ("2028-03-05", "2028-02-28"),
        ("2024-02-29", "2024-02-26"),
        ("1970-01-01", "1969-12-29"),
    ];
    for (day, monday) in weeks {
        assert_eq!(Day::parse(day).unwrap().monday().to_string(), monday, "{day}");
    }
}

#[test]
fn only_a_day_of_the_calendar_written_as_git_writes_one_is_a_day() {
    let wrong = ["2026-02-30", "2026-13-01", "2026-11-00", "2026-11-9", "11/09/2026", ""];
    for not_a_day in wrong {
        assert_eq!(Day::parse(not_a_day), None, "{not_a_day}");
    }
    assert!(Day::parse("2028-02-29").is_some(), "a leap year has the day");
    assert_eq!(Day::parse("2027-02-29"), None, "and the year before it has not");
}

#[test]
fn the_table_is_a_row_a_week_oldest_first_and_a_last_row_for_everything() {
    let commits = [
        commit("2026-11-10", Group::Text),
        commit("2026-11-03", Group::Rebuild),
        commit("2026-11-12", Group::Rebuild),
        commit("2026-11-08", Group::Notes),
        commit("2026-11-09", Group::Client),
        commit("2026-11-11", Group::Text),
    ];
    let expected = [
        "week rebuild client text notes rebuild %",
        "2026-11-02 1 0 0 1 50%",
        "2026-11-09 1 1 2 0 25%",
        "all 2 1 2 1 33%",
    ];
    assert_eq!(rows(&table(&commits)), expected);
}

#[test]
fn the_columns_of_the_table_line_up() {
    let commits = [commit("2026-11-03", Group::Rebuild), commit("2026-11-10", Group::Text)];
    let drawn = table(&commits);
    let widths: Vec<usize> = drawn.lines().map(str::len).collect();
    assert!(widths.iter().all(|width| *width == widths[0]), "{drawn}");
}

/// The share the table gives when `rebuild` of `all` commits needed one.
fn share(rebuild: usize, all: usize) -> String {
    let mut commits = vec![commit("2026-11-09", Group::Notes); all];
    for one in commits.iter_mut().take(rebuild) {
        one.group = Group::Rebuild;
    }
    table(&commits).split_whitespace().next_back().unwrap().to_string()
}

#[test]
fn the_share_is_the_nearest_whole_percent_and_a_half_is_the_next_one_up() {
    assert_eq!(share(0, 3), "0%");
    assert_eq!(share(1, 3), "33%");
    assert_eq!(share(2, 3), "67%");
    assert_eq!(share(1, 8), "13%");
    assert_eq!(share(3, 3), "100%");
}

#[test]
fn the_history_is_read_a_commit_at_a_time_from_what_git_prints() {
    // A line for the commit, an empty one, and the files it touched.
    let log = printed("@2026-11-03\n\ncrates/api/src/lib.rs\nREADME.md\n@2026-11-04\n\ndocs/notes.md\n");
    let expected = [commit("2026-11-03", Group::Rebuild), commit("2026-11-04", Group::Notes)];
    assert_eq!(read(&log).unwrap(), expected);
    assert!(read("").unwrap().is_empty(), "and where git printed nothing there are none");
}

#[test]
fn a_commit_that_touched_no_file_here_is_not_counted() {
    // A merge is printed so, and a commit made wholly outside this folder.
    let log = printed("@2026-11-03\n@2026-11-04\n\ndocs/notes.md\n@2026-11-05\n");
    assert_eq!(read(&log).unwrap(), [commit("2026-11-04", Group::Notes)]);
}

#[test]
fn a_path_git_printed_in_quotes_is_still_grouped_by_where_it_is() {
    let log = printed("@2026-11-03\n\n\"crates/api/src/a\\tb.rs\"\n");
    assert_eq!(read(&log).unwrap(), [commit("2026-11-03", Group::Rebuild)]);
}

#[test]
fn a_day_git_did_not_write_as_one_is_refused_and_named() {
    let refused = read(&printed("@last Tuesday\n\nREADME.md\n")).unwrap_err();
    assert!(refused.contains("`last Tuesday`"), "{refused}");
}

/// A repository made for the test, with a folder in it that looks like
/// this one to the tooling: the history `changes` reads there is the one
/// made here, and not this repository's.
struct History {
    /// The top of the repository.
    root: PathBuf,
    /// Where the application is in it, which is where the tooling is run.
    home: PathBuf,
}

impl Drop for History {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn write(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

/// Runs `git` in `dir` as if at noon on `day`; a failure is the test's.
fn git(dir: &Path, day: &str, args: &[&str]) {
    let at = format!("{day} 12:00:00 +0000");
    let ran = Command::new("git")
        .args(args)
        .current_dir(dir)
        .env("GIT_AUTHOR_DATE", &at)
        .env("GIT_COMMITTER_DATE", &at)
        .output()
        .unwrap();
    assert!(ran.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&ran.stderr));
}

impl History {
    /// A repository that is the application and nothing else.
    fn new(name: &str) -> History {
        History::holding(name, "")
    }

    /// A repository with the application in its folder `app`, or at its
    /// top where `app` is empty.
    fn holding(name: &str, app: &str) -> History {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let unique = format!("log-{name}-{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::SeqCst));
        let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(unique);
        let home = root.join(app);
        // What the tooling knows its repository by. Neither is committed.
        write(&home.join("brand.toml"), "display = \"Sample\"\nid = \"sample\"\ncli = \"sm\"\n");
        write(&home.join("Cargo.toml"), "");
        // A repository of its own: without one, git run here finds the one
        // this test is in. Who commits is said in it, so that the test does
        // not lean on whoever runs it having told git who they are.
        git(&root, "2025-10-01", &["init", "--quiet", "."]);
        git(&root, "2025-10-01", &["config", "user.name", "Ada"]);
        git(&root, "2025-10-01", &["config", "user.email", "ada@example.com"]);
        History { root, home }
    }

    /// A commit made on `day` that touched `paths`, which are written from
    /// the top of the repository.
    fn commit(&self, day: &str, paths: &[&str]) {
        for path in paths {
            write(&self.root.join(path), &format!("changed on {day}\n"));
        }
        let mut add = vec!["add", "--"];
        add.extend_from_slice(paths);
        git(&self.root, day, &add);
        git(&self.root, day, &["commit", "--quiet", "-m", "a change"]);
    }

    /// Runs `cargo xtask changes` where the application is: whether it
    /// succeeded, and what it said.
    fn changes(&self, args: &[&str]) -> (bool, String) {
        let mut tool = Command::new(XTASK);
        tool.arg("changes").args(args).current_dir(&self.home);
        let output = tool.output().unwrap();
        let said =
            format!("{}{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
        (output.status.success(), said)
    }
}

#[test]
fn a_repository_with_no_commits_yet_is_told_so_and_that_is_no_failure() {
    let history = History::new("empty");
    let (ok, said) = history.changes(&[]);
    assert!(ok && said.contains("There are no commits here yet"), "{said}");
}

#[test]
fn the_commits_of_the_branch_are_counted_a_week_at_a_time() {
    let history = History::new("weeks");
    history.commit("2025-10-28", &["crates/api/src/lib.rs", "README.md"]);
    history.commit("2025-10-30", &["blueprints/std/board.bp"]);
    history.commit("2025-11-03", &["ui/src/api.ts"]);
    history.commit("2025-11-09", &["docs/notes.md"]);

    let (ok, said) = history.changes(&[]);
    assert!(ok, "{said}");
    let expected = [
        "week rebuild client text notes rebuild %",
        "2025-10-27 1 0 1 0 50%",
        "2025-11-03 0 1 0 1 0%",
        "all 1 1 1 1 25%",
    ];
    assert_eq!(rows(&said), expected);
}

#[test]
fn since_is_a_commit_where_git_knows_it_as_one_and_a_date_where_it_does_not() {
    let history = History::new("since");
    history.commit("2025-10-28", &["crates/api/src/lib.rs"]);
    git(&history.root, "2025-10-28", &["branch", "before"]);
    history.commit("2025-11-04", &["blueprints/std/board.bp"]);
    history.commit("2025-11-06", &["docs/notes.md"]);

    // A branch: what this one has that it has not.
    let (ok, said) = history.changes(&["--since", "before"]);
    assert!(ok, "{said}");
    assert_eq!(rows(&said)[1..], ["2025-11-03 0 0 1 1 0%", "all 0 0 1 1 0%"]);

    // A day and an hour, which only git reads.
    let (ok, said) = history.changes(&["--since=2025-11-05 00:00:00 +0000"]);
    assert!(ok, "{said}");
    assert_eq!(rows(&said)[1..], ["2025-11-03 0 0 0 1 0%", "all 0 0 0 1 0%"]);

    // A name git does not know is a date to it as well, and one that
    // leaves nothing. That is said, since nothing else shows the mistake.
    let (ok, said) = history.changes(&["--since", "befor"]);
    assert!(ok && said.contains("git read it as a date"), "{said}");
}

#[test]
fn where_the_repository_holds_more_only_what_touched_the_application_is_counted() {
    let history = History::holding("beside", "app");
    history.commit("2025-10-28", &["app/crates/api/src/lib.rs", "site/index.html"]);
    history.commit("2025-10-29", &["site/about.html"]);
    history.commit("2025-10-30", &["app/docs/notes.md"]);

    let (ok, said) = history.changes(&[]);
    assert!(ok, "{said}");
    assert_eq!(rows(&said)[1..], ["2025-10-27 1 0 0 1 50%", "all 1 0 0 1 50%"]);
}

#[test]
fn a_file_moved_out_of_a_crate_changed_the_crate() {
    let history = History::new("moved");
    history.commit("2025-10-28", &["crates/api/src/notes.md"]);
    // The same text under another name is, to git, the same file moved.
    fs::rename(history.root.join("crates/api/src/notes.md"), history.root.join("notes.md")).unwrap();
    git(&history.root, "2025-11-04", &["add", "--", "crates/api/src/notes.md", "notes.md"]);
    git(&history.root, "2025-11-04", &["commit", "--quiet", "-m", "moved"]);

    let (ok, said) = history.changes(&[]);
    assert!(ok, "{said}");
    let expected = ["2025-10-27 1 0 0 0 100%", "2025-11-03 1 0 0 0 100%", "all 2 0 0 0 100%"];
    assert_eq!(rows(&said)[1..], expected);
}

#[test]
fn anything_but_since_and_where_to_begin_is_refused_with_the_usage() {
    let history = History::new("usage");
    let wrong: [&[&str]; 4] = [&["--sicne", "main"], &["--since"], &["--since="], &["everything"]];
    for args in wrong {
        let (ok, said) = history.changes(args);
        assert!(!ok && said.contains("usage: cargo xtask changes"), "{args:?}: {said}");
    }
}

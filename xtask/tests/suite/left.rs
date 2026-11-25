//! What the tests left running, worked out from what two looks found.
//!
//! The cases are plain values: a look is a list of processes, each with
//! its parent, when it began, its program and its folder. The last tests
//! look for real, on a system that has `/proc`, at processes they start
//! themselves and let go by closing what those read from.

use std::path::Path;

use xtask::left::{FROM, Left, Looked, Seen, Stat, Why, ago, from, lying, new, said, stat};

const ROOT: &str = "/work/here";
/// Who takes in what has lost its parent, in these cases.
const ADOPTS: u32 = 40;
/// Whoever looks: the gate, started from a shell.
const GATE: u32 = 100;
const SHELL: u32 = 90;

fn root() -> &'static Path {
    Path::new(ROOT)
}

fn process(pid: u32, parent: u32, program: &str, folder: &str) -> Seen {
    Seen {
        pid,
        parent,
        began: u64::from(pid) * 10,
        ours: true,
        program: Some(program.into()),
        folder: Some(folder.into()),
        told: program.rsplit('/').next().unwrap_or(program).to_string(),
        ..Seen::default()
    }
}

/// What is there whatever the tests do: the system's first process, who
/// takes in what is left, a shell and the gate started from it.
fn usual() -> Vec<Seen> {
    vec![
        Seen { pid: 1, parent: 0, began: 1, ..Seen::default() },
        Seen { pid: ADOPTS, parent: 1, began: 2, ..Seen::default() },
        Seen { leads: true, ..process(SHELL, ADOPTS, "/usr/bin/zsh", ROOT) },
        process(GATE, SHELL, "/work/here/target/debug/xtask", ROOT),
    ]
}

fn looked(more: Vec<Seen>) -> Looked {
    let mut seen = usual();
    seen.extend(more);
    Looked { seen, me: GATE, adopts: vec![1, ADOPTS], at: 100_000 }
}

fn pids(left: &[Left]) -> Vec<u32> {
    left.iter().map(|left| left.seen.pid).collect()
}

#[test]
fn a_program_of_this_checkout_that_lost_its_parent_while_the_tests_ran_was_left() {
    let before = looked(vec![]);
    let after =
        looked(vec![process(500, ADOPTS, "/work/here/target/debug/api-standin", "/work/here/crates/api")]);
    let left = new(&before, &after, root());
    assert_eq!(pids(&left), [500]);
    assert_eq!(left[0].why, Why::Program);
    assert!(left[0].nobodys);
}

#[test]
fn nothing_is_left_where_nothing_is_new() {
    let all = looked(vec![process(500, ADOPTS, "/work/here/target/debug/factory", ROOT)]);
    assert_eq!(new(&all, &all, root()), []);
}

#[test]
fn what_was_there_before_the_tests_is_not_theirs() {
    // Left by an earlier gate, or started by hand: there before, and so
    // not what these tests did.
    let old = process(300, ADOPTS, "/work/here/target/debug/factory", ROOT);
    let before = looked(vec![old.clone()]);
    let after = looked(vec![old, process(500, ADOPTS, "/usr/bin/sleep", "/work/here/crates/cli")]);
    assert_eq!(pids(&new(&before, &after, root())), [500]);
}

#[test]
fn a_number_given_again_is_another_process() {
    // The process that had the number before the tests is gone, and one
    // the tests left has it now: when it began says which it is.
    let before = looked(vec![process(500, SHELL, "/usr/bin/cat", "/")]);
    let again = Seen { began: 99_000, ..process(500, ADOPTS, "/work/here/target/debug/api-standin", ROOT) };
    assert_eq!(pids(&new(&before, &looked(vec![again]), root())), [500]);
}

#[test]
fn another_checkouts_program_is_not_this_ones() {
    // Another checkout's gate is running at this moment, and what it
    // leaves has the same parent as what this one would.
    let before = looked(vec![]);
    let after = looked(vec![
        process(500, ADOPTS, "/work/there/target/debug/api-standin", "/work/there/crates/api"),
        // A folder that only begins as this checkout's does.
        process(501, ADOPTS, "/work/here-too/target/debug/factory", "/work/here-too"),
    ]);
    assert_eq!(new(&before, &after, root()), []);
}

#[test]
fn what_somebody_who_was_there_before_still_waits_for_is_theirs() {
    // A build started from another terminal while the gate runs: its
    // programs are under this checkout's target/, and its parent is
    // there, as it was before.
    let other = process(95, ADOPTS, "/usr/bin/zsh", ROOT);
    let before = looked(vec![other.clone()]);
    let after = looked(vec![
        other,
        process(600, 95, "/usr/bin/cargo", ROOT),
        process(601, 600, "/work/here/target/debug/build/x/build-script-build", ROOT),
        // And what the gate itself has running is the gate's.
        process(602, GATE, "/work/here/target/debug/deps/suite-1", "/work/here/crates/api"),
    ]);
    assert_eq!(new(&before, &after, root()), []);
}

#[test]
fn a_shell_somebody_opened_meanwhile_is_theirs_with_what_they_run_in_it() {
    // A terminal opened in the checkout while the gate runs: its shell
    // was started by what takes in what is left, as a terminal's is.
    let before = looked(vec![]);
    let after = looked(vec![
        Seen { leads: true, ..process(500, ADOPTS, "/usr/bin/zsh", ROOT) },
        process(501, 500, "/work/here/target/debug/factory", ROOT),
    ]);
    assert_eq!(new(&before, &after, root()), []);
}

#[test]
fn everything_under_what_was_left_is_left_with_it() {
    // A daemon a test started and never stopped, the wrapper it started,
    // and the program under that, which is the system's and is in no
    // folder of the checkout.
    let before = looked(vec![]);
    let after = looked(vec![
        process(500, ADOPTS, "/work/here/target/debug/factory", "/work/here/crates/cli"),
        process(501, 500, "/work/here/target/debug/factory", "/"),
        process(502, 501, "/usr/bin/sleep", "/"),
    ]);
    let left = new(&before, &after, root());
    assert_eq!(pids(&left), [500, 501, 502]);
    assert_eq!(left[2].why, Why::Under(501));
}

#[test]
fn a_program_of_the_systems_in_a_folder_of_the_checkout_was_left_and_one_elsewhere_was_not() {
    let before = looked(vec![]);
    let after = looked(vec![
        process(500, ADOPTS, "/usr/bin/sleep", "/work/here/crates/cli"),
        // Somebody's own, started meanwhile and let go of: nothing of it
        // is this checkout's.
        process(501, ADOPTS, "/usr/bin/sleep", "/home/somebody"),
    ]);
    let left = new(&before, &after, root());
    assert_eq!(pids(&left), [500]);
    assert_eq!(left[0].why, Why::Folder);
}

#[test]
fn what_a_test_program_started_says_whose_tests_they_were_wherever_it_is() {
    let before = looked(vec![]);
    let told = |krate: &str, checkout: &str| Some(from(Path::new(checkout), krate));
    let after = looked(vec![
        // In no folder of the checkout and no program of it: only what it
        // was told says where it came from.
        Seen { from: told("cli", ROOT), ..process(500, ADOPTS, "/usr/bin/sleep", "/") },
        // Told by another checkout's gate.
        Seen { from: told("cli", "/work/there"), ..process(501, ADOPTS, "/usr/bin/sleep", "/") },
        // What a program was started by is said for what it started too,
        // which was told nothing.
        Seen { from: told("api", ROOT), ..process(502, ADOPTS, "/work/here/target/debug/factory", ROOT) },
        process(503, 502, "/usr/bin/cat", "/"),
    ]);
    let left = new(&before, &after, root());
    assert_eq!(pids(&left), [500, 502, 503]);
    assert_eq!(left[0].why, Why::Told);
    let of: Vec<Option<&str>> = left.iter().map(|left| left.of.as_deref()).collect();
    assert_eq!(of, [Some("cli"), Some("api"), Some("api")]);
}

#[test]
fn a_parent_that_is_nowhere_to_be_seen_has_gone() {
    // Its parent ended between the reading of one and of the other.
    let before = looked(vec![]);
    let after = looked(vec![process(500, 499, "/work/here/target/debug/api-standin", ROOT)]);
    assert_eq!(pids(&new(&before, &after, root())), [500]);
}

#[test]
fn a_listing_holds_every_program_of_the_checkout_and_says_which_nobody_waits_for() {
    let looked = looked(vec![
        // A test program the gate is running now.
        process(200, GATE, "/work/here/target/debug/deps/suite-1", "/work/here/crates/api"),
        process(201, 200, "/work/here/target/debug/api-standin", "/work/here/crates/api"),
        // Left by a gate of an hour ago.
        process(150, ADOPTS, "/work/here/target/debug/api-standin", "/work/here/crates/api"),
        process(151, ADOPTS, "/usr/bin/sleep", "/work/here/crates/cli"),
        // Another checkout's.
        process(152, ADOPTS, "/work/there/target/debug/factory", "/work/there"),
        // Somebody's editor, in the folder with its parent there.
        process(160, SHELL, "/usr/bin/vim", ROOT),
    ]);
    let left = lying(&looked, root(), None);
    let listed: Vec<(u32, bool)> = left.iter().map(|left| (left.seen.pid, left.nobodys)).collect();
    assert_eq!(listed, [(150, true), (151, true), (200, false), (201, false)]);
}

#[test]
fn a_listing_leaves_out_whoever_looks_and_a_shell_that_is_in_the_folder() {
    // The shell is in the checkout and its parent is whoever takes in
    // what is left, as a shell's is; the gate's program is under target/.
    assert_eq!(lying(&looked(vec![]), root(), None), []);
    // A second shell in the folder, which whoever looks did not come from.
    let other = Seen { leads: true, ..process(95, ADOPTS, "/usr/bin/zsh", "/work/here/crates") };
    assert_eq!(lying(&looked(vec![other]), root(), None), []);
}

#[test]
fn a_listing_takes_what_the_systems_own_has_taken_in_for_left() {
    // Left from another session, whose own first process has it: not one
    // this look knows to take such in, and not readable by whoever looks.
    let theirs = Seen { pid: 70, parent: 1, began: 5, ..Seen::default() };
    let looked = looked(vec![theirs, process(300, 70, "/usr/bin/sleep", "/work/here/crates/cli")]);
    assert_eq!(pids(&lying(&looked, root(), None)), [300]);
}

#[test]
fn what_came_from_something_about_to_end_has_nobody_waiting_for_it() {
    // A check of the client, as it exits: its daemon was told to stop
    // and has not yet, with a service's wrapper and program under it;
    // the browser it drives is in the folder and is no program of the
    // checkout's, and another check's daemon is still that check's.
    let (check, other) = (700, 800);
    let looked = Looked {
        me: 720,
        ..looked(vec![
            process(check, SHELL, "/usr/bin/node", "/work/here/ui"),
            process(701, check, "/work/here/target/debug/factory", ROOT),
            process(702, 701, "/work/here/target/debug/factory", ROOT),
            process(703, 702, "/usr/bin/python3", "/"),
            process(710, check, "/opt/browser/chrome", "/work/here/ui"),
            process(720, check, "/work/here/target/debug/xtask", ROOT),
            process(other, SHELL, "/usr/bin/node", "/work/here/ui"),
            process(801, other, "/work/here/target/debug/factory", ROOT),
        ])
    };
    let listed = |after| -> Vec<(u32, bool)> {
        // Of the two checks: what else these cases have running is
        // another terminal's here.
        let of_a_check = |left: &&Left| left.seen.pid >= check;
        lying(&looked, root(), after)
            .iter()
            .filter(of_a_check)
            .map(|left| (left.seen.pid, left.nobodys))
            .collect()
    };
    assert_eq!(listed(None), [(701, false), (702, false), (703, false), (801, false)]);
    assert_eq!(listed(Some(check)), [(701, true), (702, true), (703, true), (801, false)]);
}

#[test]
fn a_line_says_which_process_how_long_whose_tests_what_and_where() {
    let seen = Seen {
        from: Some(from(root(), "api")),
        told: "/work/here/target/debug/api-standin idle".into(),
        began: 100_000 - 4_200,
        ..process(500, ADOPTS, "/work/here/target/debug/api-standin", "/work/here/crates/api")
    };
    let left = new(&looked(vec![]), &looked(vec![seen]), root());
    assert_eq!(
        said(&left[0], 100_000, root()),
        "500 for 42 s, from the tests of `api`: /work/here/target/debug/api-standin idle (in crates/api)"
    );
    // What it was started with is said in one line, whatever is in it.
    let script = Seen { told: "sh -c \nset -e\n  echo up".into(), ..left[0].seen.clone() };
    let line = said(&Left { seen: script, ..left[0].clone() }, 100_000, root());
    assert!(line.ends_with(": sh -c set -e echo up (in crates/api)"), "{line}");
    assert_eq!(ago(0), "0 s");
    assert_eq!(ago(11_900), "119 s");
    assert_eq!(ago(360_000), "60 min");
    assert_eq!(ago(5_400_000), "15 h");
}

#[test]
fn what_a_process_says_of_itself_is_read_past_a_name_with_brackets_in_it() {
    let line = "4242 (a (odd) name) S 17 4242 4000 34816 4242 4194304 1 0 0 0 0 0 0 0 20 0 1 0 987654 1 2 3";
    assert_eq!(
        stat(line),
        Some(Stat { state: 'S', parent: 17, session: 4000, terminal: 34816, began: 987_654 })
    );
    assert_eq!(stat("4242 (cut short) S 17"), None);
    assert_eq!(stat("nothing of the kind"), None);
    assert!(stat(&line.replace(") S ", ") Z ")).unwrap().over());
}

/// The looking itself, where there is something to read it from.
#[cfg(target_os = "linux")]
mod looking {
    use std::io::Read;
    use std::path::{Path, PathBuf};
    use std::process::{Child, ChildStdin, Command, Stdio};

    use xtask::left::{FROM, Looked, adopter, from, new, still};

    /// A look as the tooling's own program makes one.
    fn look() -> Result<Looked, String> {
        xtask::left::look(adopter(Path::new(env!("CARGO_BIN_EXE_xtask"))))
    }

    fn folder(name: &str) -> PathBuf {
        // A folder of this run's own: the test may be run many times at once.
        let folder =
            Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("left-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(folder.join("target")).unwrap();
        folder.canonicalize().unwrap()
    }

    /// Something left behind in `folder` as a test would leave it: a
    /// shell starts a program that waits to be let go, and ends. What is
    /// handed back is the program's number and what lets it go.
    fn left_behind(folder: &Path, krate: &str) -> (u32, ChildStdin) {
        let mut shell: Child = Command::new("sh")
            .args(["-c", "exec 3<&0; cat <&3 >/dev/null 2>&1 & echo $!"])
            .current_dir(folder)
            .env(FROM, from(folder, krate))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let held = shell.stdin.take().unwrap();
        let mut said = String::new();
        shell.stdout.take().unwrap().read_to_string(&mut said).unwrap();
        assert!(shell.wait().unwrap().success());
        (said.trim().parse().unwrap(), held)
    }

    fn gone(looked: &Looked, pid: u32) {
        let seen = looked.seen.iter().find(|seen| seen.pid == pid).unwrap();
        // It ends when what it reads from is closed, which has been done:
        // that it has is waited for as the gate waits, by looking.
        while still(seen) {
            std::thread::yield_now();
        }
    }

    #[test]
    fn something_left_behind_in_a_folder_is_found_and_says_whose_tests_left_it_and_nothing_once_it_is_let_go()
    {
        let folder = folder("found");
        let before = look().unwrap();
        let (pid, held) = left_behind(&folder, "api");
        let after = look().unwrap();
        let left = new(&before, &after, &folder);
        assert_eq!(left.iter().map(|left| left.seen.pid).collect::<Vec<_>>(), [pid], "{left:#?}");
        assert_eq!(left[0].of.as_deref(), Some("api"));
        assert!(left[0].seen.told.starts_with("cat"), "{:?}", left[0].seen);
        // And it is nobody else's: another folder's look finds nothing.
        let another = self::folder("another");
        assert_eq!(new(&before, &after, &another), []);

        drop(held);
        gone(&after, pid);
        assert_eq!(new(&before, &look().unwrap(), &folder), []);
        for made in [folder, another] {
            let _ = std::fs::remove_dir_all(made);
        }
    }

    #[test]
    fn something_a_test_still_waits_for_is_not_left() {
        let folder = folder("waited");
        let before = look().unwrap();
        // Started by this test and waited for by it, in the same folder.
        let mut child = Command::new("cat")
            .current_dir(&folder)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .spawn()
            .unwrap();
        let after = look().unwrap();
        assert!(after.seen.iter().any(|seen| seen.pid == child.id() && seen.parent == std::process::id()));
        assert_eq!(new(&before, &after, &folder), []);
        drop(child.stdin.take());
        child.wait().unwrap();
        let _ = std::fs::remove_dir_all(folder);
    }
}

// Held where the rest are, though it is of no look: what a test program
// is told is one word, and reads back as whose it is.
#[test]
fn what_a_test_program_is_told_names_its_crate_and_its_checkout() {
    assert_eq!(FROM, "TESTS_OF");
    assert_eq!(from(root(), "api"), "api@/work/here");
}

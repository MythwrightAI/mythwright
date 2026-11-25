//! The `general` gate: the runner and the building blocks are held to
//! naming no card, board, stage, lane or fleet.
//!
//! Each test makes a small repository and asks the real tool, with only
//! its static gates, what that gate says of it. Whether the other gates
//! pass in such a tree is not what is asked.

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

const XTASK: &str = env!("CARGO_BIN_EXE_xtask");

/// What the gate says of a repository that holds these files: whether it
/// passed, and its lines.
fn said(files: &[(&str, &str)]) -> (bool, String) {
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let unique = format!("xtask-general-{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::SeqCst));
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(unique);
    let put = |path: &str, text: &str| {
        let path = root.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    };
    put("brand.toml", "display = \"Sample\"\nid = \"sample\"\ncli = \"sm\"\n");
    put("Cargo.toml", "[workspace]\n");
    put(".gitignore", "target/\n");
    for (path, text) in files {
        put(path, text);
    }
    let git = |args: &[&str]| {
        let done = Command::new("git")
            .args(["-c", "user.name=test", "-c", "user.email=test@example.invalid"])
            .args(["-c", "core.hooksPath=/dev/null", "-c", "commit.gpgsign=false"])
            .args(args)
            .current_dir(&root)
            .output()
            .unwrap();
        assert!(done.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&done.stderr));
    };
    git(&["init", "--quiet"]);
    git(&["add", "."]);
    git(&["commit", "--quiet", "-m", "as it began"]);
    let done = Command::new(XTASK).args(["check", "--fast"]).current_dir(&root).output().unwrap();
    let all = String::from_utf8_lossy(&done.stdout).into_owned();
    let _ = fs::remove_dir_all(&root);
    // The gate's own line, and what is said under it until the next gate's.
    let mut lines = all.lines().skip_while(|line| !line.contains(" general "));
    let first = lines.next().unwrap_or_else(|| panic!("the gate said nothing:\n{all}")).to_string();
    let under: Vec<&str> = lines.take_while(|line| line.starts_with("     ")).collect();
    (first.trim_start().starts_with("ok"), format!("{first}\n{}", under.join("\n")))
}

#[test]
fn the_runner_may_speak_of_a_card_in_its_notes_and_never_in_its_code() {
    let notes = "//! A lane's steps claim its card.\npub fn runs() {} // as a stage's card would\n";
    let (passed, told) = said(&[("crates/conductor/src/lib.rs", notes)]);
    assert!(passed, "{told}");

    for code in
        ["pub fn card_of(id: &str) {}\n", "pub struct StageKind;\n", "pub const LANES: &str = \"x\";\n"]
    {
        let (passed, told) = said(&[("crates/blueprint/src/lib.rs", code)]);
        assert!(!passed, "{code}: {told}");
        assert!(told.contains("crates/blueprint/src/lib.rs:1: the runner names a"), "{told}");
    }
    // What the runner keeps on disk is the runner's code as much.
    let (passed, told) = said(&[("crates/runs/src/lib.rs", "pub fn card_of(id: &str) {}\n")]);
    assert!(!passed, "{told}");
    assert!(told.contains("crates/runs/src/lib.rs:1: the runner names a"), "{told}");
    // And so is what starts one blueprint several ways, which came out of it.
    let (passed, told) = said(&[("crates/comparisons/src/lib.rs", "pub fn card_of(id: &str) {}\n")]);
    assert!(!passed, "{told}");
    assert!(told.contains("crates/comparisons/src/lib.rs:1: the runner names a"), "{told}");
    // Any other crate may: what a board is, is somebody's to know.
    let (passed, told) = said(&[("crates/studio/src/lib.rs", "pub fn card_of(id: &str) {}\n")]);
    assert!(passed, "{told}");
}

#[test]
fn no_action_is_called_after_a_card_or_a_lane_and_none_takes_or_gives_one() {
    let general = "action place.make\n  takes item string needed\n  gives dir\n\naction board.move\n  takes id string\n";
    let (passed, told) = said(&[("contract/offered.txt", general)]);
    assert!(passed, "the board's own actions are let through, and said to be: {told}");
    assert!(told.contains("the actions of `board`"), "{told}");

    let (passed, told) = said(&[("contract/offered.txt", "action lane.place\n  takes item string\n")]);
    assert!(!passed && told.contains("the action `lane.place` is called after a lane"), "{told}");
    let takes = "action place.make\n  takes card string needed\n  gives stage\n";
    let (passed, told) = said(&[("contract/offered.txt", takes)]);
    assert!(!passed, "{told}");
    assert!(told.contains("`card` of the action `place.make` is called after a card"), "{told}");
    assert!(told.contains("`stage` of the action `place.make` is called after a stage"), "{told}");
}

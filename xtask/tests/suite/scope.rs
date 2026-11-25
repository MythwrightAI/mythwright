//! `cargo xtask check --changed`: what a change is taken to affect.
//!
//! Each test makes a small repository, changes something in it and asks
//! the real tool, with only its static gates, what it would check. Whether
//! those gates then pass in such a tree is not what is asked.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

const XTASK: &str = env!("CARGO_BIN_EXE_xtask");

struct Tree {
    root: PathBuf,
}

impl Drop for Tree {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn write(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

impl Tree {
    /// A repository with a crate, one built on it that reads a file of the
    /// client's, and what a repository has beside its crates; all of it
    /// committed.
    fn new(name: &str) -> Tree {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let unique = format!("xtask-{name}-{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::SeqCst));
        let tree = Tree { root: PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(unique) };
        let put = |path: &str, text: &str| write(&tree.root.join(path), text);
        put("brand.toml", "display = \"Sample\"\nid = \"sample\"\ncli = \"sm\"\n");
        put("Cargo.toml", "[workspace]\n");
        put("crates/low/Cargo.toml", "[package]\nname = \"low\"\n");
        put("crates/low/src/lib.rs", "pub fn low() {}\n");
        put(
            "crates/high/Cargo.toml",
            "[package]\nname = \"high\"\n\n[dependencies]\nlow = { workspace = true }\n",
        );
        put(
            "crates/high/src/lib.rs",
            "pub const STYLE: &str = include_str!(\"../../../ui/src/page.css\");\n",
        );
        put("crates/apart/Cargo.toml", "[package]\nname = \"apart\"\n");
        put("crates/apart/src/lib.rs", "pub fn apart() {}\n");
        put("ui/src/page.css", "body { margin: 0 }\n");
        put("ui/src/app.tsx", "export const app = 1;\n");
        put("docs/notes.md", "# Notes\n");
        put("extensions/first/extension.yaml", "extension: first\n");
        put("build-budget.toml", "packages = 1\n");
        // What the tool itself keeps is not a change, as in the repository.
        put(".gitignore", "target/\n");
        tree.git(&["init", "--quiet"]);
        tree.git(&["add", "."]);
        tree.git(&["commit", "--quiet", "-m", "as it began"]);
        tree
    }

    fn git(&self, args: &[&str]) {
        let done = Command::new("git")
            .args(["-c", "user.name=test", "-c", "user.email=test@example.invalid"])
            .args(["-c", "core.hooksPath=/dev/null", "-c", "commit.gpgsign=false"])
            .args(args)
            .current_dir(&self.root)
            .output()
            .unwrap();
        assert!(done.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&done.stderr));
    }

    fn change(&self, path: &str) -> &Tree {
        let path = self.root.join(path);
        let was = fs::read_to_string(&path).unwrap_or_default();
        write(&path, &format!("{was}\n"));
        self
    }

    /// What the tool says it would check, for what has changed.
    fn scope(&self) -> String {
        let done = Command::new(XTASK)
            .args(["check", "--fast", "--changed"])
            .current_dir(&self.root)
            .output()
            .unwrap();
        let said = String::from_utf8_lossy(&done.stdout).into_owned();
        said.lines()
            .find_map(|line| line.strip_prefix("scope: "))
            .unwrap_or_else(|| panic!("the tool did not say what it would check:\n{said}"))
            .to_string()
    }
}

#[test]
fn where_git_lists_no_files_nothing_is_checked_and_the_tool_says_so_and_does_not_pass() {
    // A copy of the source that is not a repository: every gate reads what
    // git lists, and would pass having looked at nothing.
    let tree = Tree::new("scope-no-repository");
    fs::remove_dir_all(tree.root.join(".git")).unwrap();
    let done = Command::new(XTASK).args(["check", "--fast"]).current_dir(&tree.root).output().unwrap();
    assert!(!done.status.success());
    let said = String::from_utf8_lossy(&done.stderr);
    assert!(said.contains("git lists no files here"), "{said}");
    assert!(!String::from_utf8_lossy(&done.stdout).contains(" ok "), "a gate ran over nothing");
}

#[test]
fn a_change_in_a_crate_affects_it_and_what_is_built_on_it_and_nothing_else() {
    let tree = Tree::new("scope-crate");
    assert_eq!(tree.scope(), "no Rust crates affected");
    assert_eq!(tree.change("crates/low/src/lib.rs").scope(), "high, low");
    let tree = Tree::new("scope-top");
    assert_eq!(tree.change("crates/high/src/lib.rs").scope(), "high");
}

#[test]
fn a_change_to_a_note_affects_everything_since_any_crates_tests_may_read_it() {
    // Tests hold the notes to the code, so a note that changed alone is
    // not a change nothing needs running for.
    let tree = Tree::new("scope-note");
    assert_eq!(
        tree.change("docs/notes.md").scope(),
        "everything, since docs/notes.md changed and is in no crate: any crate's tests may read it"
    );
    // And one that was not there before, which no list could have named.
    let tree = Tree::new("scope-new-note");
    assert!(tree.change("docs/another.md").scope().starts_with("everything, since docs/another.md"));
}

#[test]
fn a_change_to_anything_else_outside_the_crates_affects_everything() {
    for changed in ["extensions/first/extension.yaml", "build-budget.toml", "Cargo.toml", "brand.toml"] {
        let tree = Tree::new("scope-outside");
        assert!(
            tree.change(changed).scope().starts_with(&format!("everything, since {changed}")),
            "{changed}"
        );
    }
    // Beside a change in a crate it is still everything.
    let tree = Tree::new("scope-both");
    assert!(tree.change("crates/apart/src/lib.rs").change("docs/notes.md").scope().starts_with("everything"));
}

#[test]
fn the_clients_own_files_affect_no_crate_but_one_that_names_the_file_that_changed() {
    // The client has a gate of its own.
    let tree = Tree::new("scope-client");
    assert_eq!(tree.change("ui/src/app.tsx").scope(), "no Rust crates affected");
    // A file of the client's that a crate reads is that crate's too.
    let tree = Tree::new("scope-client-read");
    assert_eq!(tree.change("ui/src/page.css").scope(), "high");
    let tree = Tree::new("scope-client-both");
    assert_eq!(tree.change("ui/src/app.tsx").change("crates/apart/src/lib.rs").scope(), "apart");
}

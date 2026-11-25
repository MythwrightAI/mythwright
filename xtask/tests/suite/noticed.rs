//! `cargo xtask notices`, run in a tree made for the test: the file made
//! from what the tool and the client's build say, the same made twice;
//! what is said where the tool is not there, is another version, or
//! refuses a licence; and the programs' half kept without the tool where
//! nothing it was made from has changed.
//!
//! The tool and the client's build are stood in for by files of the
//! system's shell that write records, called `cargo` and `node` and
//! looked for before anything else: what each really writes is the file at
//! the top of the repository, which `notices.rs` holds.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

use xtask::notices;

const XTASK: &str = env!("CARGO_BIN_EXE_xtask");

/// A licence's text, long enough to be taken for one.
const TEXT: &str = "Permission is hereby granted, free of charge, to any person obtaining a copy\nof this software, to deal in the Software without restriction, and so on for as\nlong as a licence goes on, which is longer than two hundred letters by a good\nway, so that this one is taken for a licence's text and not for none at all.";

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

fn record(licence: &str, library: &str) -> String {
    format!(
        "%%licence%% {licence}\n%%name%% {licence}\n%%from%% file\n%%library%% {library}\n%%text%%\n{TEXT}\n%%end%%\n"
    )
}

/// The lock of a program built from one crate of somebody else's.
fn lock(version: &str) -> String {
    format!(
        "version = 4\n\n[[package]]\nname = \"cli\"\nversion = \"0.1.0\"\ndependencies = [\n \"theirs\",\n]\n\n[[package]]\nname = \"theirs\"\nversion = \"{version}\"\nsource = \"registry+https://example.org/index\"\nchecksum = \"abcd\"\n"
    )
}

impl Tree {
    /// A tree that looks like the repository to the tooling: a program
    /// built from one crate, a client built from one package, and
    /// stand-ins for the tool and the client's build that say so.
    fn new(name: &str) -> Tree {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let unique =
            format!("xtask-notices-{name}-{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::SeqCst));
        let tree = Tree { root: PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(unique) };
        let root = &tree.root;
        write(&root.join("brand.toml"), "display = \"Sample\"\nid = \"sample\"\ncli = \"sm\"\n");
        write(&root.join("Cargo.toml"), "[workspace]\nmembers = [\"crates/*\"]\n");
        write(
            &root.join("crates/cli/Cargo.toml"),
            "[package]\nname = \"cli\"\n\n[dependencies]\ntheirs = \"1\"\n",
        );
        write(&root.join("Cargo.lock"), &lock("1.0.0"));
        write(&root.join("about.toml"), "accepted = [\"MIT\"]\n");
        write(&root.join("about.hbs"), "what the tool writes by\n");
        write(&root.join("ui/node_modules/package/package.json"), "{}");
        write(
            &root.join("ui/package-lock.json"),
            "{\n  \"packages\": {\n    \"node_modules/package\": {\n      \"version\": \"3.0.0\"\n    },\n    \"node_modules/second\": {\n      \"version\": \"0.5.0\"\n    }\n  }\n}\n",
        );
        write(&root.join("said/crates"), &record("MIT", "theirs 1.0.0"));
        write(&root.join("said/packages"), &record("ISC", "package 3.0.0"));
        tree.tool("echo 'cargo-about 0.9.3'");
        // The client's build: asked in the client's folder, as it is.
        tree.program("node", "test \"$1 $2\" = 'build.mjs --notices' || exit 2\ncat ../said/packages");
        tree
    }

    /// A stand-in for a program, looked for before any other of its name.
    fn program(&self, name: &str, does: &str) {
        let file = self.root.join("first").join(name);
        write(&file, &format!("#!/bin/sh\n{does}\n"));
        fs::set_permissions(&file, fs::Permissions::from_mode(0o755)).unwrap();
    }

    /// A stand-in for cargo with the tool as `version` answers for it:
    /// asked which version it is, it does that; asked to make the notices,
    /// it writes what the tree says the crates are, having been asked as
    /// the tooling asks.
    fn tool(&self, version: &str) {
        let asked = "about -c never generate --frozen --fail --workspace -c about.toml about.hbs";
        self.program(
            "cargo",
            &format!(
                "if test \"$1 $2\" = 'about --version'; then {version}; exit $?; fi\ntest \"$*\" = '{asked}' || {{ echo \"asked: $*\" >&2; exit 2; }}\ncat said/crates"
            ),
        );
    }

    /// Runs `cargo xtask notices` in the tree.
    fn notices(&self, args: &[&str]) -> (bool, String) {
        let rest = std::env::var_os("PATH").unwrap_or_default();
        let all = std::iter::once(self.root.join("first")).chain(std::env::split_paths(&rest));
        let output = Command::new(XTASK)
            .arg("notices")
            .args(args)
            .current_dir(&self.root)
            .env("PATH", std::env::join_paths(all).unwrap())
            .output()
            .unwrap();
        let said =
            format!("{}{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
        (output.status.success(), said)
    }

    fn file(&self) -> String {
        fs::read_to_string(self.root.join(notices::FILE)).unwrap_or_default()
    }

    /// What does not hold of the file that is there, asked with no tool.
    fn held(&self) -> Vec<String> {
        notices::held(&self.root).unwrap()
    }
}

#[test]
fn the_file_is_made_from_what_the_tool_and_the_clients_build_say_and_is_the_same_made_twice() {
    let tree = Tree::new("made");
    let refused = notices::held(&tree.root).unwrap_err();
    assert!(refused.contains("`cargo xtask notices` makes it, and it is committed"), "{refused}");
    let (ok, said) = tree.notices(&[]);
    assert!(ok, "{said}");
    assert!(said.contains("1 crates (MIT)") && said.contains("1 packages of the client (ISC)"), "{said}");
    assert!(said.contains("wrote THIRD-PARTY-NOTICES.md"), "{said}");
    let file = tree.file();
    for wanted in
        ["- `theirs` 1.0.0\n", "- `package` 3.0.0\n", "### (`MIT`)", "### (`ISC`)", "Built into the programs"]
    {
        assert!(file.contains(wanted), "`{wanted}` in\n{file}");
    }
    assert_eq!(tree.held(), Vec::<String>::new());

    // Made again from the same, it is the same, and nothing is written.
    let (ok, said) = tree.notices(&[]);
    assert!(ok && said.contains("is what is there already: nothing to commit"), "{said}");
    assert_eq!(tree.file(), file);
    let (ok, said) = tree.notices(&["--check"]);
    assert!(ok && said.contains("is what is there already"), "{said}");

    // A crate updated with the notices not made again: said with no tool,
    // and by `--check`, which writes nothing.
    write(&tree.root.join("Cargo.lock"), &lock("1.1.0"));
    let problems = tree.held().join("\n");
    assert!(
        problems.contains("the crates the programs can be built from are not the ones it was made from"),
        "{problems}"
    );
    assert!(
        problems
            .contains("it names the crate `theirs` 1.0.0, which `Cargo.lock` does not hold at that version"),
        "{problems}"
    );
    write(&tree.root.join("said/crates"), &record("MIT", "theirs 1.1.0"));
    let (ok, said) = tree.notices(&["--check"]);
    assert!(!ok && said.contains("is not what `cargo xtask notices` makes of what is here now"), "{said}");
    assert_eq!(tree.file(), file);
    let (ok, said) = tree.notices(&[]);
    assert!(ok, "{said}");
    assert!(tree.file().contains("- `theirs` 1.1.0\n"));
    assert_eq!(tree.held(), Vec::<String>::new());

    // And it takes nothing else.
    let (ok, said) = tree.notices(&["--sideways"]);
    assert!(!ok && said.contains("`--sideways` is nothing `cargo xtask notices` takes"), "{said}");
}

#[test]
fn without_the_tool_the_programs_half_is_kept_while_nothing_it_was_made_from_has_changed() {
    let tree = Tree::new("kept");
    assert!(tree.notices(&[]).0);
    let before = notices::read(&tree.file());

    // No tool, and a package added to the client: the client's half is
    // made again and the programs' is as it was.
    tree.tool("echo 'error: no such command: `about`' >&2; exit 101");
    write(
        &tree.root.join("said/packages"),
        &format!("{}{}", record("ISC", "package 3.0.0"), record("MIT", "second 0.5.0")),
    );
    let (ok, said) = tree.notices(&[]);
    assert!(ok, "{said}");
    assert!(
        said.contains("`cargo-about` is not installed here: the programs' half is kept as it is"),
        "{said}"
    );
    let after = notices::read(&tree.file());
    assert_eq!(after.crates_half, before.crates_half);
    assert_eq!(after.marks, before.marks);
    assert!(tree.file().contains("- `second` 0.5.0\n"), "{}", tree.file());
    assert_eq!(tree.held(), Vec::<String>::new());

    // A crate updated: now the tool is what is missing, and how it is
    // come by is said whole.
    write(&tree.root.join("Cargo.lock"), &lock("2.0.0"));
    let kept = tree.file();
    let (ok, said) = tree.notices(&[]);
    assert!(!ok, "{said}");
    assert!(said.contains("`cargo-about` is not installed here."), "{said}");
    assert!(
        said.contains("\n  cargo install cargo-about --locked --version 0.9.3 --features cli\n"),
        "{said}"
    );
    assert!(said.contains("no build, test or package asks for it"), "{said}");
    assert_eq!(tree.file(), kept, "nothing was written");

    // Another version of the tool may tell a text otherwise, and is not
    // what the file is made with.
    tree.tool("echo 'cargo-about 0.8.4'");
    let (ok, said) = tree.notices(&[]);
    assert!(
        !ok && said
            .contains("the `cargo-about` here is `cargo-about 0.8.4`, and the file is made with 0.9.3."),
        "{said}"
    );
    assert!(said.contains(notices::INSTALL), "{said}");
}

#[test]
fn a_licence_that_is_not_accepted_makes_no_file_and_says_whose_it_is_to_accept() {
    let tree = Tree::new("refused");
    // The tool, refusing as it does: it names the crate and the licence.
    tree.program(
        "cargo",
        "if test \"$1 $2\" = 'about --version'; then echo 'cargo-about 0.9.3'; exit 0; fi\necho 'error: failed to satisfy license requirements' >&2\necho '   license = \"MPL-2.0\"' >&2\nexit 1",
    );
    let (ok, said) = tree.notices(&[]);
    assert!(!ok, "{said}");
    for wanted in [
        "`cargo about` could not make the notices:",
        "license = \"MPL-2.0\"",
        "A crate is under a licence that `about.toml` does not accept",
        "it is the maintainer's to accept, one crate at a time",
    ] {
        assert!(said.contains(wanted), "`{wanted}` in\n{said}");
    }
    assert!(!tree.root.join(notices::FILE).exists(), "no file was made");

    // And so does a client with no packages to read the notices from.
    let tree = Tree::new("uninstalled");
    fs::remove_dir_all(tree.root.join("ui/node_modules")).unwrap();
    let (ok, said) = tree.notices(&[]);
    assert!(!ok && said.contains("run `npm install` in ui/, then `cargo xtask notices` again"), "{said}");
    assert!(!tree.root.join(notices::FILE).exists());
}

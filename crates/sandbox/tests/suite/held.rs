//! A sandbox is handed nothing of the factory in use, by where its
//! code lives: this reads the crate's own source and its manifest for
//! anything that would let it hold such a thing.
//!
//! What the crate may stand on is listed, and what its code may not
//! name is looked for in every line that is code. A line that names
//! one for a reason is let through here, with the reason.

use std::path::Path;

/// The crates of this repository a sandbox may stand on. None of the
/// ones above it: not the agents, not the gateway, not the daemon.
const STANDS_ON: &[(&str, &str)] = &[
    ("blueprint", "the language: a text read, an action and an event as each says itself"),
    ("kernel", "plain values, a clock nobody else moves, ids"),
    ("journal", "a journal and a database, each made new in memory"),
    ("conductor", "a conductor made on that journal with the sandbox's own registry"),
    ("blocks", "three actions that work something out"),
    ("deciding", "the blocks a fleet's texts ask, each a function of what it is handed"),
    ("hosting", "the two blocks of a rollout, which no `Hosting` is needed for, and nothing else of it"),
    ("studio", "the tables a factory has, made new over the sandbox's journal"),
];

/// What no line of its code may name.
const NEVER: &[(&str, &str)] = &[
    ("api::", "the daemon"),
    ("agents::", "the agents"),
    ("gateway::", "other devices"),
    ("Hosting", "what starts programs and keeps secrets"),
    ("std::process", "a program started"),
    ("tokio::process", "a program started"),
    ("Command", "a program started"),
    ("std::net", "the network"),
    ("tokio::net", "the network"),
    ("TcpStream", "a socket"),
    ("TcpListener", "a socket"),
    ("UdpSocket", "a socket"),
    ("UnixStream", "a socket"),
    ("reqwest", "a request sent"),
    ("enable_io", "a runtime that can open a socket or wait on a program"),
    ("enable_all", "a runtime that can open a socket or wait on a program"),
    ("Db::open", "a database that is a file"),
    ("keeps_projects_in(", "journals that are files"),
    ("std::env", "what the daemon was started with"),
    ("kernel::git", "a repository"),
    ("secret", "a secret"),
    ("Own::", "the installation's own key"),
    ("link::", "the installation's own key"),
    ("Studio::open(", "a studio that may start a program or ask a service"),
];

/// A line that names one of those all the same, and why it may.
const LET_THROUGH: &[(&str, &str, &str)] = &[
    (
        "stood.rs",
        "pub secrets: Vec<String>,",
        "the names of the secrets a step would have been handed; a name is what a blueprint writes, and no \
         value is ever where a step's context is",
    ),
    (
        "stood.rs",
        "#[serde(skip_serializing_if = \"Vec::is_empty\")]",
        "no word of it: the line above the names",
    ),
    ("stood.rs", "secrets: cx.secrets().values().cloned().collect(),", "the same names, read from the step"),
    (
        "inside.rs",
        "kernel::id::secret()",
        "random letters, for the password of the made-up person: an account of the sandbox's own journal, which \
         nobody signs in to",
    ),
];

fn source() -> Vec<(String, String)> {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files: Vec<(String, String)> = std::fs::read_dir(src)
        .unwrap()
        .flatten()
        .map(|entry| {
            (entry.file_name().to_string_lossy().into_owned(), std::fs::read_to_string(entry.path()).unwrap())
        })
        .collect();
    files.sort();
    assert!(files.len() >= 6, "the crate's source was not found");
    files
}

#[test]
fn the_sandbox_s_code_names_nothing_it_could_reach_the_factory_in_use_by() {
    let mut found = Vec::new();
    for (file, text) in source() {
        for (at, line) in text.lines().enumerate() {
            let code = line.trim();
            if code.starts_with("//") {
                continue;
            }
            for (word, what) in NEVER {
                let let_through =
                    LET_THROUGH.iter().any(|(within, said, _)| *within == file && code.contains(said));
                if code.contains(word) && !let_through {
                    found.push(format!("{file}:{}: `{word}` ({what}): {code}", at + 1));
                }
            }
        }
    }
    assert!(
        found.is_empty(),
        "a sandbox is made of nothing of the factory in use, and its code names what it must not hold. Hand it \
         a plain value in `Made`, or, where the line is none of that, let it through here with the reason:\n{}",
        found.join("\n")
    );
    // Each line let through is still there: a reason for nothing is taken away.
    let files = source();
    for (within, said, _) in LET_THROUGH {
        let there = files.iter().any(|(file, text)| file == within && text.contains(said));
        assert!(there, "`{said}` is let through in {within} and is no longer there: take its line away");
    }
}

#[test]
fn the_sandbox_stands_on_no_crate_above_it() {
    let manifest = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml")).unwrap();
    let ours: Vec<String> = std::fs::read_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join(".."))
        .unwrap()
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    assert!(ours.contains(&"agents".to_string()) && ours.contains(&"api".to_string()), "{ours:?}");
    let depends = manifest.split("[dependencies]").nth(1).unwrap().split("\n[").next().unwrap();
    let named: Vec<&str> =
        depends.lines().filter_map(|line| line.split_once(" = ")).map(|(name, _)| name.trim()).collect();
    for name in named.iter().filter(|name| ours.iter().any(|one| one == *name)) {
        assert!(
            STANDS_ON.iter().any(|(allowed, _)| allowed == name),
            "the sandbox depends on `{name}`, which is not one of the crates it may stand on: {STANDS_ON:?}"
        );
    }
    for (allowed, _) in STANDS_ON {
        assert!(named.contains(allowed), "`{allowed}` is listed as stood on and is no dependency");
    }
}

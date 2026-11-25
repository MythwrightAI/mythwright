//! Nothing of the factory's own is a line of shell, where a program is
//! started or stopped.
//!
//! What starts a program for the factory, waits for it and says how it
//! ended is this program (`hosting::wrapped`), and what stops one is a
//! call to the system (`hosting::group`): neither is a shell that has to
//! be found, or `kill` run as a program, and both are the same wherever
//! this program is built. A shell is one system's. This reads the source
//! for the name of one, or of a program that stops another, written out
//! to be run, so that the next one is found before anything is run.
//!
//! What a user writes for a shell is still read by their system's
//! (`run:`, a health check's `cmd:`): that is theirs. And the tests'
//! stand-ins are lines of shell throughout: a crate's tests are not read.

use std::path::Path;

/// What is let through, and why: a file, the words on the line, the reason.
const LET_THROUGH: &[(&str, &str, &str)] = &[
    (
        "crates/hosting/src/script.rs",
        "let mut command = Command::new(\"/bin/sh\");",
        "a user's own command (`run:` of a step or a service, a health check's `cmd:`) is written for a shell, \
         and is read by the system's: `hosting::script::command`, which nothing of the factory's asks for itself",
    ),
    (
        "crates/hosting/src/script.rs",
        "pub const SHELL: &str = \"cmd.exe\";",
        "the same where the system is Windows: what its shell is called, for a user's own command and nothing else",
    ),
    (
        "crates/hosting/src/script.rs",
        "pub const READS: [&str; 3] = [\"/D\", \"/S\", \"/C\"];",
        "and what that shell is told so that it reads the user's line as it was written",
    ),
    (
        "crates/hosting/src/wall.rs",
        ".args([\"--\", \"/bin/sh\", \"-c\", \":\"])",
        "the trial of whether a wall can be held: a command that does nothing, started behind one by the \
         program that holds walls, which only Linux has. It starts no program of anybody's",
    ),
    (
        "crates/cli/src/serve.rs",
        "(\"cmd\", &[\"/C\", \"start\", \"\"])",
        "opening the client in a person's browser where the system is Windows: `start` is a word of that \
         shell and no program. It is asked once, by `up --open`, of nothing the factory runs",
    ),
    (
        "crates/search/src/lib.rs",
        "\"go\", \"sh\", \"sql\"",
        "a kind of file the search index reads, by how its name ends: nothing is run",
    ),
    (
        "xtask/src/offline.rs",
        "\"sh\", \"-c\", INSIDE",
        "the gate's own, and not the program's: the tests are run with the network cut off by the system's \
         `unshare`, on Linux, where the gate says it cannot when it cannot",
    ),
];

/// A shell, or a program that stops another, by the names one is run by.
const NAMES: &[&str] =
    &["sh", "bash", "zsh", "dash", "cmd.exe", "powershell", "pwsh", "kill", "pkill", "killall", "taskkill"];

/// Where a system keeps such a program, for one named by where it is.
const KEPT_IN: &[&str] = &["", "/bin/", "/usr/bin/", "/usr/local/bin/"];

/// What a shell that is not Unix's is told so that it reads a line: its
/// own name is also a word of the blueprint language (`cmd:`), so it is
/// found by this.
const READS_A_LINE: &[&str] = &["\"/C\"", "\"/c\""];

/// The lines of `text` that name a shell, or a program that stops
/// another, as something written out to be run: the line and what it says.
fn wrong_in(text: &str) -> Vec<(usize, String)> {
    let mut wrong = Vec::new();
    for (line, says) in text.lines().enumerate() {
        // What a comment says is not run.
        let code = says.trim_start();
        if code.starts_with("//") {
            continue;
        }
        let names = NAMES
            .iter()
            .flat_map(|name| KEPT_IN.iter().map(move |kept| format!("\"{kept}{name}\"")))
            .any(|written| code.contains(&written));
        if names || READS_A_LINE.iter().any(|written| code.contains(written)) {
            wrong.push((line + 1, code.to_string()));
        }
    }
    wrong
}

#[test]
fn nothing_of_the_factorys_own_starts_or_stops_a_program_with_a_shell_or_a_program_that_kills() {
    let top = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut wrong = Vec::new();
    let (mut looked, mut let_through) = (0, vec![0; LET_THROUGH.len()]);
    let mut folders = vec![top.join("crates"), top.join("xtask"), top.join("desktop"), top.join("examples")];
    while let Some(folder) = folders.pop() {
        for entry in std::fs::read_dir(&folder).unwrap().flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            if path.is_dir() {
                // A crate's tests start stand-ins for a user's programs,
                // which are lines of shell; what the factory starts them
                // with is in its source.
                if name != "tests" && name != "target" && name != "node_modules" {
                    folders.push(path);
                }
                continue;
            }
            if !name.ends_with(".rs") {
                continue;
            }
            looked += 1;
            let file = path.strip_prefix(&top).unwrap().to_string_lossy().replace('\\', "/");
            for (line, says) in wrong_in(&std::fs::read_to_string(&path).unwrap()) {
                match LET_THROUGH.iter().position(|(at, words, _)| *at == file && says.contains(words)) {
                    Some(listed) => let_through[listed] += 1,
                    None => wrong.push(format!("{file}:{line}: {says}")),
                }
            }
        }
    }
    assert!(looked > 200, "looked at only {looked} files: is this still where they are?");
    for ((file, words, _), found) in LET_THROUGH.iter().zip(&let_through) {
        assert_eq!(
            *found, 1,
            "`{words}` in {file} is let through and is there {found} times: put the list right"
        );
    }
    assert!(
        wrong.is_empty(),
        "these name a shell, or a program that stops another, to be run. A program is started for the factory by \
         its wrapper, which is this program (`hosting::wrapped`, `hosting::kept`), from a list of arguments; a \
         group is stopped by `hosting::group::stop`, and one process asked by `hosting::group::ask`. A user's \
         own script goes through `command` in `crates/hosting/src/script.rs`. What is neither is listed in \
         `LET_THROUGH` with the reason.\n{}",
        wrong.join("\n")
    );
}

#[test]
fn what_reads_the_source_for_a_shell_finds_each_way_of_writing_one() {
    let found = |text: &str| wrong_in(text).into_iter().map(|(line, _)| line).collect::<Vec<_>>();
    assert_eq!(found("Command::new(\"/bin/sh\").arg(\"-c\")"), [1]);
    assert_eq!(found("let shell = \"sh\";\nlet other = \"bash\";"), [1, 2]);
    assert_eq!(found("Command::new(\"kill\").args([\"-TERM\"])"), [1]);
    assert_eq!(found("for program in [\"/bin/kill\", \"kill\"] {"), [1]);
    assert_eq!(found("Command::new(\"taskkill\").args([\"/PID\"])"), [1]);
    assert_eq!(found("command.arg(\"/C\").arg(script);"), [1]);
    assert_eq!(found("(\"cmd\", &[\"/C\", \"start\"])"), [1]);
    // A word of the blueprint language, a comment, and a word that only
    // holds such a name are none of them a shell that is run.
    assert_eq!(found(".arg(\"cmd\", T::List, \"a program and its arguments\")"), [] as [usize; 0]);
    assert_eq!(found("// it was `Command::new(\"/bin/sh\")` once"), [] as [usize; 0]);
    assert_eq!(found("let topic = \"skill\"; let other = \"shell\";"), [] as [usize; 0]);
}

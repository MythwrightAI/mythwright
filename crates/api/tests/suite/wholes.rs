//! What the rest of the program may ask of the journal's database.
//!
//! One thread does all of that database's work, and has a transaction
//! open for each piece of it, which it may share among pieces that were
//! waiting together (`journal::gather`). So a piece of work begins no
//! transaction of its own, ends none, and asks nothing that has to be
//! outside one: what it writes that is one thing it writes in a whole
//! (`journal::whole`). The thread refuses the rest in a sentence when it
//! is asked (the journal's own suite holds that); this reads the source,
//! so that it is found before anything is run, by whoever writes the
//! next one without knowing.

use std::path::Path;

use crate::Rig;

/// What is let through, and why: a file, the words on the line, the reason.
const LET_THROUGH: &[(&str, &str, &str)] = &[(
    "crates/folders/src/snapshot.rs",
    "connection.unchecked_transaction()",
    "what is left out of a copy is taken out of the copy's own file, on a connection opened for it \
     (`journal::copy::open`, by both that call `leave_out`), and never on the journal's thread",
)];

/// What begins a transaction or a part of one, whatever it is asked of.
const BEGINS: &[&str] = &[
    "unchecked_transaction(",
    "transaction_with_behavior(",
    "Transaction::new",
    ".savepoint(",
    "savepoint_with_name(",
    "Savepoint::new",
];

/// A statement that begins or ends a transaction, or cannot be made
/// inside one, where it opens a text that is written out.
const STATEMENTS: &[&str] = &["BEGIN", "COMMIT", "ROLLBACK", "SAVEPOINT", "RELEASE", "END", "VACUUM"];

/// What `.transaction()` is asked of, at `at` in `text`: the name before
/// the dot, over the end of a line where a chain was broken there.
fn asked_of(text: &str, at: usize) -> &str {
    let before = text[..at].trim_end();
    let from = before
        .char_indices()
        .rev()
        .find(|(_, letter)| !(letter.is_alphanumeric() || *letter == '_'))
        .map_or(0, |(at, letter)| at + letter.len_utf8());
    &before[from..]
}

/// The places in `text` that begin a transaction of their own, end one,
/// or ask what cannot be asked inside one: the line and what it says.
fn wrong_in(text: &str) -> Vec<(usize, String)> {
    let line_of = |at: usize| text[..at].matches('\n').count();
    let mut wrong = Vec::new();
    // The whole a write was handed is called a transaction (`Writing`),
    // and reading it begins nothing: `writing.transaction()`.
    for (at, _) in text.match_indices(".transaction()") {
        if asked_of(text, at) != "writing" {
            wrong.push(line_of(at));
        }
    }
    for begins in BEGINS {
        wrong.extend(text.match_indices(begins).map(|(at, _)| line_of(at)));
    }
    for statement in STATEMENTS {
        let opens = format!("\"{statement}");
        for (at, _) in text.match_indices(&opens) {
            // The word itself, and not one that starts with it.
            let after = text[at + opens.len()..].chars().next();
            if after.is_none_or(|letter| !(letter.is_alphanumeric() || letter == '_' || letter == '-')) {
                wrong.push(line_of(at));
            }
        }
    }
    wrong.sort_unstable();
    wrong.dedup();
    let lines: Vec<&str> = text.lines().collect();
    wrong.into_iter().map(|line| (line + 1, lines[line].trim().to_string())).collect()
}

#[test]
fn nothing_outside_the_journal_begins_a_transaction_of_its_own_on_the_journals_connection() {
    let top = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut wrong = Vec::new();
    let (mut looked, mut let_through) = (0, 0);
    let mut folders = vec![top.join("crates"), top.join("xtask"), top.join("desktop")];
    while let Some(folder) = folders.pop() {
        for entry in std::fs::read_dir(&folder).unwrap().flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            if path.is_dir() {
                // The journal's own crate is where the thread and the
                // whole are. A crate's tests are run, and what they ask
                // of the thread is refused there in its sentence; what
                // they open for themselves is theirs.
                let journals = path.ends_with("crates/journal");
                if !journals && name != "tests" && name != "target" && name != "node_modules" {
                    folders.push(path);
                }
                continue;
            }
            if !name.ends_with(".rs") {
                continue;
            }
            looked += 1;
            let file = path.strip_prefix(&top).unwrap().to_string_lossy().replace('\\', "/");
            let file = file.trim_start_matches("../../").to_string();
            for (line, says) in wrong_in(&std::fs::read_to_string(&path).unwrap()) {
                match LET_THROUGH.iter().any(|(at, words, _)| *at == file && says.contains(words)) {
                    true => let_through += 1,
                    false => wrong.push(format!("{file}:{line}: {says}")),
                }
            }
        }
    }
    assert!(looked > 200, "looked at only {looked} files: is this still where they are?");
    assert_eq!(
        let_through,
        LET_THROUGH.len(),
        "something let through is no longer there: take it off the list"
    );
    assert!(
        wrong.is_empty(),
        "these begin or end a transaction of their own, or ask what cannot be asked inside one. The journal's \
         thread has a transaction open for every piece of work it does: write what is one thing in a whole, \
         `journal::whole(connection)`, and do what must be outside a transaction on a connection of its own. \
         One that is never asked of the journal's thread is listed in `LET_THROUGH` with the reason.\n{}",
        wrong.join("\n")
    );
}

#[test]
fn what_reads_the_source_for_a_transaction_of_its_own_finds_each_way_of_writing_one() {
    let found = |text: &str| wrong_in(text).into_iter().map(|(line, _)| line).collect::<Vec<_>>();
    // Each way of beginning one, on a line or over the end of one.
    assert_eq!(found("let all = connection.transaction()?;"), [1]);
    assert_eq!(found("let all = connection\n    .transaction()?;"), [2]);
    assert_eq!(found("connection.unchecked_transaction()?"), [1]);
    assert_eq!(found("connection.transaction_with_behavior(Immediate)?"), [1]);
    assert_eq!(found("let part = connection.savepoint()?;"), [1]);
    assert_eq!(found("rusqlite::Transaction::new(connection, Deferred)"), [1]);
    // Said as a statement, to begin one or to end the thread's own.
    assert_eq!(found("connection.execute_batch(\"BEGIN IMMEDIATE\")"), [1]);
    assert_eq!(found("connection.execute_batch(\"COMMIT\")"), [1]);
    assert_eq!(found("connection.execute(\"ROLLBACK;\", [])"), [1]);
    assert_eq!(found("connection.execute(\"VACUUM INTO ?1\", [to])"), [1]);
    // The whole a write was handed, and a whole asked for, begin nothing
    // of their own; nor does a word that only starts like a statement,
    // or a rule's own `BEGIN` in the middle of what makes it.
    assert_eq!(found("writing.transaction().execute(sql, [])?"), [0usize; 0]);
    assert_eq!(found("None => writing\n    .transaction()\n    .execute(sql, [])?"), [0usize; 0]);
    assert_eq!(found("let all = journal::whole(connection)?;"), [0usize; 0]);
    assert_eq!(found("say(\"ENDED\"); say(\"BEGINNING\"); say(\"END-OF-FILE\")"), [0usize; 0]);
    assert_eq!(found("\"CREATE TRIGGER full BEFORE INSERT ON events\n BEGIN SELECT 1; END;\""), [0usize; 0]);
}

#[tokio::test]
async fn nothing_a_factory_keeps_is_checked_only_as_a_commit_is_made() {
    // Work that waits for the database together may be kept by one
    // commit. Work that fails is refused where it is done and takes back
    // its own; a rule that is looked at only as the commit is made would
    // take back everybody's in it, for what one of them wrote.
    let rig = Rig::new().await;
    let deferred: Vec<String> = rig
        .node()
        .app
        .journal
        .db()
        .call(|connection| {
            let mut read = connection.prepare("SELECT name FROM sqlite_master WHERE sql LIKE '%DEFERR%'")?;
            read.query_map([], |row| row.get(0))?.collect()
        })
        .await
        .unwrap();
    assert!(deferred.is_empty(), "checked only at the commit: {deferred:?}");
}

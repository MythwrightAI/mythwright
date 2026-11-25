//! The steps a database's tables are made by only grow at the end.
//!
//! Each crate says how its tables are made as a list of steps, and a
//! database that has run a list's first steps runs only those after
//! (`journal::steps`). So a step that was there stays where it was,
//! word for word, and a new one is added after the last. A step written
//! in before the last was never run by a factory that had run them all:
//! it ran the last a second time, was refused that by its database, and
//! would not start; every test had passed, since a test makes its
//! databases new, and made new any order of steps does.
//!
//! So what each step of each list comes to is written down
//! (`contract/steps.txt`: the list, which step, and what its text comes
//! to), and a list that is not what it was fails here. The lists are
//! read from the databases a node makes, each of which keeps what the
//! steps it ran came to: nobody has to remember a list, and where tables
//! are made under a name that none of those databases has, that fails
//! too. A step added after the last, and a new list, are written by
//!
//!   WRITE_STEPS=1 cargo test -p api the_steps
//!
//! and committed with the step. A step is the text the database is
//! handed, whole: a comment in it, or how it is laid out, is part of it.
//! Where a step has to be changed all the same, somebody says why, and
//! the reason is kept (`contract/steps-changed.txt`), which is built
//! into the program so that a database that ran the step as it was is
//! not refused:
//!
//!   BREAK_STEPS="why" cargo test -p api the_steps

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use api::store::StoreServer;
use serde_json::json;

use crate::Rig;

const KEPT: &str = "contract/steps.txt";
const CHANGED: &str = "contract/steps-changed.txt";

/// What each step of each list comes to, by the list's name.
type Lists = BTreeMap<String, Vec<String>>;

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Every file under `folder` that is a database.
fn databases(folder: &Path) -> Vec<PathBuf> {
    let (mut found, mut folders) = (Vec::new(), vec![folder.to_path_buf()]);
    while let Some(folder) = folders.pop() {
        for entry in std::fs::read_dir(&folder).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                folders.push(path);
                continue;
            }
            let mut begins = [0u8; 16];
            let read = std::fs::File::open(&path)
                .and_then(|mut file| std::io::Read::read_exact(&mut file, &mut begins));
            if read.is_ok() && &begins == b"SQLite format 3\0" {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}

/// What the lists were that a node's databases were made by: a factory
/// with a project in it and a store beside it, stopped, and every
/// database in its folder asked what it keeps of the steps it ran. With
/// it, which file each list was read in.
async fn made() -> (Lists, BTreeMap<String, Vec<String>>) {
    let mut rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    // A store keeps its records in a database of its own, made when one
    // is served.
    let file = rig.dir.join("store").join("cards.db");
    let store = StoreServer::start(&file, "127.0.0.1:0".parse().unwrap()).await.unwrap();
    store.stop().await;
    rig.stop().await;
    // And the tables a project's own file is made with, as whatever
    // writes such a file with no daemon makes them: where the suite is
    // played with one journal, the project above was given none.
    let apart = journal::Db::open_unwaited(&rig.dir.join("apart").join("shop.db")).unwrap();
    studio::apart::make(&apart).await.unwrap();
    agents::apart::make(&apart).await.unwrap();
    apart.close();
    apart.closed().await;
    let (mut lists, mut read_in) = (Lists::new(), BTreeMap::<String, Vec<String>>::new());
    for file in databases(&rig.dir) {
        let within = file.strip_prefix(&rig.dir).unwrap().to_string_lossy().replace('\\', "/");
        let database = rusqlite::Connection::open(&file).unwrap();
        let mut rows = database.prepare("SELECT namespace, version, steps FROM schema_versions").unwrap();
        let rows = rows.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?, row.get::<_, Option<String>>(2)?))
        });
        for row in rows.unwrap() {
            let (list, ran, were) = row.unwrap();
            let were: Vec<String> = were.unwrap_or_default().split_whitespace().map(str::to_string).collect();
            assert_eq!(
                were.len() as i64,
                ran,
                "`{list}` in {within}: a database keeps what each step it ran was"
            );
            if let Some(elsewhere) = lists.get(&list) {
                assert_eq!(
                    &were, elsewhere,
                    "`{list}` in {within} is another list than in {:?}",
                    read_in[&list]
                );
            }
            lists.insert(list.clone(), were);
            read_in.entry(list).or_default().push(within.clone());
        }
    }
    (lists, read_in)
}

/// What is written down: what each step of each list came to.
fn kept(written: &str) -> Lists {
    let mut lists = Lists::new();
    for line in written.lines().filter(|line| !line.starts_with('#') && !line.trim().is_empty()) {
        let mut said = line.split(' ');
        let (list, step, mark) = (said.next().unwrap(), said.next().unwrap(), said.next().unwrap());
        let steps = lists.entry(list.to_string()).or_default();
        steps.push(mark.to_string());
        assert_eq!(
            step.parse(),
            Ok(steps.len()),
            "`{KEPT}` says the steps of `{list}` in their order: {line}"
        );
    }
    lists
}

const HEAD: &str = "\
# The steps every database's tables are made by: each list by the name
# its tables are made under, each of its steps in its order, and what
# the step's text comes to. A step that is here stays where it is, word
# for word, and a step is only ever added after the last: a database
# that has run a list's first steps runs only those after.
# Written by a test, never by hand. After adding a step at the end of a
# list, or a list:
#   WRITE_STEPS=1 cargo test -p api the_steps
# writes its line. Commit it with the step. A line is never changed
# that way: a step changed where it stood is written only with why
# (steps-changed.txt), and a line is taken out by hand only for a step
# no database anywhere was ever made by.
";

fn write(made: &Lists) {
    let mut written = HEAD.to_string();
    for (list, steps) in made {
        written.push('\n');
        for (at, mark) in steps.iter().enumerate() {
            written.push_str(&format!("{list} {} {mark}\n", at + 1));
        }
    }
    std::fs::write(repo().join(KEPT), written).expect("the steps written");
}

/// A step that is not the one that was there: which, and what the two
/// come to.
#[derive(Debug, PartialEq)]
struct Changed {
    list: String,
    step: usize,
    was: String,
    is: String,
}

/// What is wrong between the lists as they are and what they are held
/// to, a sentence for each list.
#[derive(Debug, Default)]
struct Found {
    /// What no reason makes right: a step taken out, a list gone.
    never: Vec<String>,
    /// A list with a step that is not the one that was there.
    wrong: Vec<String>,
    /// Each such step, for where it is said to be meant.
    changed: Vec<Changed>,
    /// What only has to be written down: steps added last, a new list.
    owed: Vec<String>,
}

const WHY: &str =
    "A database that has run the steps before runs only what comes after them, by how many it has run";

fn held(made: &Lists, kept: &Lists) -> Found {
    let mut found = Found::default();
    for (list, steps) in made {
        let Some(were) = kept.get(list) else {
            found
                .owed
                .push(format!("`{list}` is a list of {} that is held to nothing yet", count(steps.len())));
            continue;
        };
        if steps.len() < were.len() {
            found.never.push(format!(
                "`{list}` has {} and is held to {}: a step was taken out. {WHY}: one that ran {} is refused \
                 by a program that has fewer, as a database newer than the program. Put the step back where \
                 it was; what it made is undone by a step added after the last",
                count(steps.len()),
                were.len(),
                were.len()
            ));
            continue;
        }
        let differs: Vec<usize> = (0..were.len()).filter(|at| steps[*at] != were[*at]).collect();
        for at in &differs {
            let (list, was, is) = (list.clone(), were[*at].clone(), steps[*at].clone());
            found.changed.push(Changed { list, step: at + 1, was, is });
        }
        if let Some(first) = differs.first().copied() {
            let n = first + 1;
            let now = steps.iter().position(|step| *step == were[first]).map(|at| at + 1);
            let what = match now {
                Some(now) if now == n + 1 => format!(
                    "a step was written in before it, and the step that was there is now step {now}. {WHY}: \
                     the step written in is never run there, and the one pushed along is run a second time, \
                     which the database refuses, so that its program does not start, or takes without a \
                     word, leaving the new step undone for good. Add the step after the last"
                ),
                Some(now) => format!(
                    "the step that was there is now step {now}: it was moved. {WHY}: one that had run it \
                     where it was does not run it again where it is, and runs a second time whatever is now \
                     in a place it had not come to. Put it back where it was"
                ),
                None => format!(
                    "it was changed, and a comment or how it is laid out is part of it, since the text is \
                     what a database is handed. {WHY}: a step that was changed is not run again by any \
                     database that ran it, and what it now does is done only in one made new. Leave it as \
                     it was, and add a step after the last that does what is wanted"
                ),
            };
            found.wrong.push(format!(
                "`{list}`: step {n} of the {} it is held to is not the step that was there: {what}",
                were.len()
            ));
        } else if steps.len() > were.len() {
            found.owed.push(format!(
                "`{list}` has {} and is held to {}: what was added after the last is to be written down",
                count(steps.len()),
                were.len()
            ));
        }
    }
    for (list, were) in kept.iter().filter(|(list, _)| !made.contains_key(*list)) {
        found.never.push(format!(
            "`{list}` is held to {} and no database a node makes was made by it. A list whose tables are \
             still made is made by this test (`made`), so that its steps are read; one whose tables are no \
             longer made has its lines taken out by hand, and a database goes on keeping how many of its \
             steps it ran",
            count(were.len())
        ));
    }
    found
}

fn count(steps: usize) -> String {
    format!("{steps} step{}", if steps == 1 { "" } else { "s" })
}

/// What is let through of the names tables are made under being written
/// where the making is asked: a file, the words on the line, the reason.
const LET_THROUGH: &[(&str, &str, &str)] = &[
    (
        "crates/journal/src/set.rs",
        "journal.db().migrate(namespace, steps)",
        "a project's journal is made with the tables whoever keeps rows there said (`Journal::makes`), \
         under the name they said, which is read where they say it",
    ),
    (
        "crates/journal/src/set.rs",
        "journal.db().migrate(made.namespace, made.steps)",
        "the same, for a journal made after it was said",
    ),
];

/// The names tables are made under, as `text` writes each where it asks
/// for them to be made, and the lines that ask and write no name there.
fn asked_in(text: &str) -> (Vec<String>, Vec<(usize, String)>) {
    let (mut names, mut unnamed) = (Vec::new(), Vec::new());
    for asks in [".migrate(", ".makes("] {
        for (at, _) in text.match_indices(asks) {
            let line = text[..at].matches('\n').count();
            let whole = text.lines().nth(line).unwrap_or_default().trim();
            if whole.starts_with("//") {
                continue;
            }
            // What it is handed, as far as the call's end: the name is
            // the first text written out there.
            let handed = &text[at + asks.len()..];
            let handed = &handed[..handed.find(')').unwrap_or(handed.len())];
            let mut parts = handed.split('"');
            match (parts.next(), parts.next(), parts.next()) {
                (Some(_), Some(name), Some(_)) => names.push(name.to_string()),
                _ => unnamed.push((line + 1, whole.to_string())),
            }
        }
    }
    (names, unnamed)
}

/// Where tables are made, anywhere in the program, under a name none of
/// `made` is of, or under one the place does not write out: each a
/// sentence. And how many files were looked at.
fn made_unread(made: &Lists) -> (Vec<String>, usize) {
    let top = repo();
    let (mut wrong, mut looked, mut let_through) = (Vec::new(), 0, 0);
    let mut folders = vec![top.join("crates"), top.join("xtask"), top.join("desktop"), top.join("examples")];
    while let Some(folder) = folders.pop() {
        for entry in std::fs::read_dir(&folder).unwrap().flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            if path.is_dir() {
                // A crate's tests make tables of their own, of lists
                // made up to be wrong.
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
            let file = file.trim_start_matches("../../").to_string();
            let (names, unnamed) = asked_in(&std::fs::read_to_string(&path).unwrap());
            for name in names.iter().filter(|name| !made.contains_key(*name)) {
                wrong.push(format!(
                    "{file} makes tables under `{name}`, and no database this test's node made has them: \
                     have this test make them (`made`), so that the steps they are made by are held"
                ));
            }
            for (line, says) in unnamed {
                match LET_THROUGH.iter().any(|(at, words, _)| *at == file && says.contains(words)) {
                    true => let_through += 1,
                    false => wrong.push(format!(
                        "{file}:{line} makes tables under a name it does not write out there (`{says}`): \
                         write the name where the making is asked, so that it is read here"
                    )),
                }
            }
        }
    }
    if let_through != LET_THROUGH.len() {
        wrong.push("something let through is no longer there: take it off `LET_THROUGH`".to_string());
    }
    (wrong, looked)
}

fn today() -> String {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis();
    api::contract::day(now as i64)
}

/// A step changed where it stood, as it is written down with why.
fn said(day: &str, changed: &Changed, why: &str) -> String {
    let Changed { list, step, was, is } = changed;
    format!("{day}\t{list}\t{step}\t{was}\t{is}\t{}\n", why.trim().replace(['\t', '\n'], " "))
}

/// The gate.
#[tokio::test]
async fn the_steps_a_database_is_made_by_are_the_ones_that_were_there_with_any_more_added_last() {
    let (made, read_in) = made().await;
    // Every list there is: nothing here names one.
    let (unread, looked) = made_unread(&made);
    assert!(looked > 200, "looked at only {looked} files: is this still where they are?");
    assert!(unread.is_empty(), "{}", unread.join("\n"));
    let found = held(&made, &kept(&std::fs::read_to_string(repo().join(KEPT)).unwrap_or_default()));
    if std::env::var_os("SAY_STEPS").is_some() {
        for (list, steps) in &made {
            println!("{list}: {}, read in {}", count(steps.len()), read_in[list].join(", "));
        }
    }
    assert!(found.never.is_empty(), "{}", found.never.join(".\n"));
    let why = std::env::var("BREAK_STEPS").ok().filter(|why| !why.trim().is_empty());
    if let Some(why) = &why {
        // Said once for each step, with what it came to before and comes to now.
        let mut changes = std::fs::read_to_string(repo().join(CHANGED)).unwrap_or_default();
        let day = today();
        changes.extend(found.changed.iter().map(|changed| said(&day, changed, why)));
        std::fs::write(repo().join(CHANGED), changes).expect("the reasons written");
        write(&made);
        return;
    }
    assert!(
        found.wrong.is_empty(),
        "{}.\nA step is only ever added after the last (`{KEPT}`). Where one that is there has to be changed \
         where it stands, in its words and in nothing it does, say why, and the reason is kept (`{CHANGED}`): \
         a changed step is not run again by any database that ran it, and one that ran it as it was is taken \
         to have run it:\n  BREAK_STEPS=\"why\" cargo test -p api the_steps",
        found.wrong.join(".\n")
    );
    if std::env::var_os("WRITE_STEPS").is_some() {
        write(&made);
        return;
    }
    assert!(
        found.owed.is_empty(),
        "{}.\nWrite down what each step comes to (`{KEPT}`), and commit it with the step:\n  \
         WRITE_STEPS=1 cargo test -p api the_steps",
        found.owed.join(".\n")
    );
}

#[test]
fn what_is_wrong_with_a_list_of_steps_is_said_by_its_step_and_only_what_was_added_last_is_owed() {
    let of = |lists: &[(&str, &[&str])]| -> Lists {
        lists
            .iter()
            .map(|(list, steps)| (list.to_string(), steps.iter().map(|s| s.to_string()).collect()))
            .collect()
    };
    let kept = of(&[
        ("same", &["a", "b"]),
        ("longer", &["a", "b"]),
        ("written-in", &["a", "b", "rules"]),
        ("moved", &["a", "b", "c", "d"]),
        ("changed", &["a", "b", "c"]),
        ("shorter", &["a", "b", "c"]),
        ("gone", &["a"]),
    ]);
    let made = of(&[
        ("same", &["a", "b"]),
        ("longer", &["a", "b", "c", "d"]),
        ("written-in", &["a", "b", "beside", "rules"]),
        ("moved", &["a", "c", "d", "b"]),
        ("changed", &["a", "B", "c", "d"]),
        ("shorter", &["a", "b"]),
        ("new", &["a"]),
    ]);
    let found = held(&made, &kept);

    // What is only owed: steps after the last, and a list that is new.
    assert_eq!(found.owed.len(), 2, "{:?}", found.owed);
    assert!(found.owed[0].starts_with("`longer` has 4 steps and is held to 2"), "{:?}", found.owed);
    assert_eq!(found.owed[1], "`new` is a list of 1 step that is held to nothing yet");

    // What nothing makes right.
    assert_eq!(found.never.len(), 2, "{:?}", found.never);
    assert!(found.never[0].starts_with("`shorter` has 2 steps and is held to 3: a step was taken out"));
    assert!(
        found.never[1].starts_with("`gone` is held to 1 step and no database a node makes was made by it")
    );

    // And a step that is not the one that was there, said by the first
    // of its list: one in the order of the lists' names.
    assert_eq!(found.wrong.len(), 3, "{:?}", found.wrong);
    let changed = &found.wrong[0];
    assert!(changed.starts_with(
        "`changed`: step 2 of the 3 it is held to is not the step that was there: it was changed"
    ));
    assert!(
        changed.contains("a step that was changed is not run again by any database that ran it"),
        "{changed}"
    );
    let moved = &found.wrong[1];
    assert!(
        moved.starts_with(
            "`moved`: step 2 of the 4 it is held to is not the step that was there: the step that was there \
             is now step 4: it was moved"
        ),
        "{moved}"
    );
    let written_in = &found.wrong[2];
    assert!(
        written_in.starts_with(
            "`written-in`: step 3 of the 3 it is held to is not the step that was there: a step was written in \
             before it, and the step that was there is now step 4. A database that has run the steps before \
             runs only what comes after them, by how many it has run: the step written in is never run \
             there, and the one pushed along is run a second time"
        ),
        "{written_in}"
    );
    assert!(written_in.ends_with("Add the step after the last"), "{written_in}");

    // Each step that differs where it stood, for whoever says it is meant.
    let step = |list: &str, step, was: &str, is: &str| Changed {
        list: list.into(),
        step,
        was: was.into(),
        is: is.into(),
    };
    assert_eq!(
        found.changed,
        [
            step("changed", 2, "b", "B"),
            step("moved", 2, "b", "c"),
            step("moved", 3, "c", "d"),
            step("moved", 4, "d", "b"),
            step("written-in", 3, "rules", "beside"),
        ]
    );
    // As it is written down, and as the program reads it: the step that
    // was is the step that is.
    let line = said("2026-11-19", &found.changed[0], "a word of its\tcomment\n");
    assert_eq!(line, "2026-11-19\tchanged\t2\tb\tB\ta word of its comment\n");
    let has = vec!["a".to_string(), "B".to_string()];
    assert_eq!(journal::steps::parted("changed", &["a", "b"], &has, ""), Some(2));
    assert_eq!(journal::steps::parted("changed", &["a", "b"], &has, &line), None);
}

#[test]
fn what_is_kept_of_the_steps_is_read_back_by_list_and_a_name_is_read_where_tables_are_made() {
    let written = "# a head\n\nfirst 1 aa\nfirst 2 bb\n\nsecond 1 cc\n";
    let lists = kept(written);
    assert_eq!(lists["first"], ["aa", "bb"]);
    assert_eq!(lists["second"], ["cc"]);
    // Where a name is read that tables are made under.
    let source = "db.migrate(\"store\", SCHEMA).await?;\n\
                  // never a `.migrate(` of a project's database\n\
                  factory.makes(keeps::RECORDS, \"collections\", crate::tables::COLLECTIONS, held).await?;\n\
                  journal.db().migrate(namespace, steps).await?;\n";
    let (names, unnamed) = asked_in(source);
    assert_eq!(names, ["store", "collections"]);
    assert_eq!(unnamed, [(4, "journal.db().migrate(namespace, steps).await?;".to_string())]);
}

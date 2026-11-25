//! The other ways the suites are played: what each is called, and what a
//! test program is told to be played so.
//!
//! A program that is tested is told two things of the journal by a word
//! in its environment, each read in one place of the journal's crate:
//! whether a project that is added has a journal of its own
//! (`PROJECT_JOURNALS`, `journal::apart`), and how work that waits for a
//! database is gathered into commits (`JOURNAL_GATHERS`,
//! `journal::gather`). Told nothing it does what a daemon does, which is
//! how the gates play every suite, each time. A suite is also to hold
//! told each word: `one` is a factory an earlier program made, which
//! every upgrade meets, and nothing played the suites so for long enough
//! that thirty-two tests had come to fail there with nobody told, and
//! one of the journal's told to commit each piece of work by itself.
//!
//! The gates do not play them: a run of every test is held to a budget,
//! and a way here is that run again, but for one that the journal's
//! suite alone goes by. They are a command of their own
//! (`cargo xtask ways`), asked for before a change to where anything is
//! kept and by the hosted run, beside the gate.

/// One way of playing the suites that is not the usual.
#[derive(Debug, PartialEq)]
pub struct Way {
    /// What it is asked for by.
    pub name: &'static str,
    /// The variable a test program reads it from.
    pub says: &'static str,
    /// And the word it reads there.
    pub word: &'static str,
    /// What is played so, in a line.
    pub what: &'static str,
    /// The crates whose suites are played so: every one, where none is
    /// named.
    pub suites: &'static [&'static str],
}

/// Every one there is, in the order they are played where none is named.
pub const WAYS: &[Way] = &[
    Way {
        name: "one",
        says: "PROJECT_JOURNALS",
        word: "one",
        what: "every project kept in the factory's own journal, as a factory from before keeps them",
        suites: &[],
    },
    Way {
        name: "own",
        says: "PROJECT_JOURNALS",
        word: "own",
        what: "each play of the runner's suite applied for a project with a journal of its own",
        suites: &[],
    },
    Way {
        name: "gathered",
        says: "JOURNAL_GATHERS",
        word: "all",
        what: "whatever waits for a database kept by one commit, however little that is",
        suites: &[],
    },
    // The journal's suite alone: nothing else has a test that goes by
    // the word, and to every other suite it is what a daemon does but
    // where sixteen pieces of work wait at once, which the usual way and
    // `gathered` play from both sides.
    Way {
        name: "ungathered",
        says: "JOURNAL_GATHERS",
        word: "1",
        what: "every piece of a database's work kept by a commit of its own",
        suites: &["journal"],
    },
];

/// The ways that were asked for by name, in the order they were named
/// and each once; every one where none was named. A name that is no way
/// is refused with those there are: taken for nothing, the suites would
/// be played the usual way and said to hold another.
pub fn asked(args: &[String]) -> Result<Vec<&'static Way>, String> {
    if args.is_empty() {
        return Ok(WAYS.iter().collect());
    }
    let mut asked: Vec<&'static Way> = Vec::new();
    for name in args {
        let Some(way) = WAYS.iter().find(|way| way.name == name) else {
            let names: Vec<&str> = WAYS.iter().map(|way| way.name).collect();
            return Err(format!(
                "`{name}` is no way the suites are played; those there are: {}",
                names.join(", ")
            ));
        };
        if !asked.contains(&way) {
            asked.push(way);
        }
    }
    Ok(asked)
}

/// What a test program's environment is to say for it to be played this
/// way and no other: each variable any way is read from, with the word
/// for this one and nothing for the rest. So a word that whoever asks
/// has in their own environment is not played beside the one asked for.
pub fn told(way: &Way) -> Vec<(&'static str, Option<&'static str>)> {
    let mut told: Vec<(&'static str, Option<&'static str>)> = Vec::new();
    for other in WAYS {
        if !told.iter().any(|(says, _)| *says == other.says) {
            told.push((other.says, (other.says == way.says).then_some(way.word)));
        }
    }
    told
}

/// Whether the suite of the crate `krate` is played this way.
pub fn plays(way: &Way, krate: &str) -> bool {
    way.suites.is_empty() || way.suites.contains(&krate)
}

/// The line that says what came of a way: whether it held, what it is
/// called, how long it took and what the tests' runner said of them.
pub fn line(way: &Way, ok: bool, millis: u128, timed: &str) -> String {
    let mark = if ok { "ok  " } else { "FAIL" };
    format!("  {mark} {:<10} {millis:>7} ms  {timed}", way.name)
}

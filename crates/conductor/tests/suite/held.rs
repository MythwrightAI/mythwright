//! Every word of the language is run, and is stopped and started.
//!
//! A play of `plays.rs`, or of `plays/targeted.rs` beside it, holds the
//! words its blueprint is written with: which they are is read from the
//! text ([`blueprint::vocabulary::written`]),
//! so a play says what it is the case for by being written with it. Every
//! text there is played (one that is not is a constant nothing uses, which
//! does not build), and a play runs its blueprint with nothing in its way
//! before it stops it anywhere. A word of the vocabulary that no play is
//! written with fails here, by name; so does a built-in action.

use std::collections::BTreeSet;

use blueprint::vocabulary::{SHAPES, written};

/// The plays, as they are written: those of `plays.rs`, and of the files
/// of its modules.
const PLAYS: &[&str] = &[
    include_str!("plays.rs"),
    include_str!("plays/again.rs"),
    include_str!("plays/gathered.rs"),
    include_str!("plays/targeted.rs"),
    include_str!("plays/timed.rs"),
];

/// A word the conductor cannot play, with the suite that runs it and why
/// it is there. It is held to being written in a blueprint of that suite.
const ELSEWHERE: &[(&str, &str, &str, &str)] = &[
    (
        "step",
        "if_fail_ask_agent",
        include_str!("../../../agents/tests/suite/blueprints.rs"),
        "an agent is asked what to do about the failure, and the conductor knows of none",
    ),
    (
        "step",
        "else",
        include_str!("../../../agents/tests/suite/blueprints.rs"),
        "it is what is done when the agent that was asked cannot say",
    ),
];

/// Every blueprint written in `source` as a raw string.
fn texts(source: &str) -> Vec<&str> {
    source.split("r#\"").skip(1).filter_map(|rest| rest.split("\"#").next()).collect()
}

#[test]
fn every_word_of_the_language_is_in_a_blueprint_that_is_played_stopped_and_started() {
    let plays: Vec<&str> = PLAYS.iter().flat_map(|source| texts(source)).collect();
    assert!(plays.len() >= 30, "the plays were not found: only {} texts", plays.len());
    let mut held: BTreeSet<(&str, &str)> = BTreeSet::new();
    let mut done: BTreeSet<String> = BTreeSet::new();
    for text in &plays {
        let words = written(text);
        assert!(!words.is_empty(), "a text of the plays does not read:\n{text}");
        held.extend(words);
        // What its steps, hooks and handlers do, by the action's name.
        for line in text.lines() {
            for said in ["do: ", "emit: "] {
                if let Some(at) = line.find(said) {
                    let action = if said == "emit: " { "emit" } else { line[at + 4..].trim() };
                    done.insert(
                        action
                            .trim_end_matches([',', '}', ' '])
                            .split([',', ' '])
                            .next()
                            .unwrap()
                            .to_string(),
                    );
                }
            }
        }
    }
    let mut missing = Vec::new();
    for shape in SHAPES {
        for field in shape.fields {
            let elsewhere = ELSEWHERE.iter().find(|(of, word, ..)| (*of, *word) == (shape.name, field.name));
            match elsewhere {
                Some((_, word, suite, why)) => {
                    assert!(
                        !why.is_empty() && suite.contains(&format!("{word}: ")),
                        "`{word}` is not run where it is said to be"
                    );
                    assert!(
                        !held.contains(&(shape.name, field.name)),
                        "`{word}` is played here now: take it off the list"
                    );
                }
                None if held.contains(&(shape.name, field.name)) => {}
                None => missing.push(format!("`{}` of a {}", field.name, shape.name)),
            }
        }
    }
    for action in blueprint::intrinsics() {
        if !done.contains(&action.name) {
            missing.push(format!("the built-in action `{}`", action.name));
        }
    }
    assert!(
        missing.is_empty(),
        "no blueprint that is played, stopped and started (`plays.rs`) is written with: {}. Write the play.",
        missing.join(", ")
    );
}

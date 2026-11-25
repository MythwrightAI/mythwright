//! Every word of the language is read, refused when it is written wrongly,
//! and laid out the one way.
//!
//! The cases are in `tests/fixtures/words`, a file for each shape and a
//! case for each of its words: a text written with the word any old way,
//! that text laid out, the word written wrongly, and where and why the
//! reader refuses that. A case is headed by its word, and is held to being
//! written with it ([`blueprint::vocabulary::written`]) and to being
//! refused at the word or at what holds it, so the heading cannot come to
//! say what the case does not. A word of the vocabulary with no case fails here, by
//! name.
//!
//! `WRITE_WORDS=1` writes what the formatter lays out and what the reader
//! refuses into the files, for a new case. Read what it wrote before it is
//! kept: it is what is held from then on.

use std::collections::BTreeSet;
use std::path::Path;

use blueprint::vocabulary::{SHAPES, written};
use blueprint::{Settings, format, means_the_same};

use crate::check;

#[derive(Default)]
struct Case {
    word: String,
    written: String,
    laid: String,
    wrong: String,
    /// The line and column the reader points at, as `3:9`.
    at: String,
    why: String,
}

/// A file of cases as its opening words and its cases.
fn cases(text: &str) -> (String, Vec<Case>) {
    let (mut opening, mut found, mut part) = (String::new(), Vec::<Case>::new(), "");
    for line in text.split_inclusive('\n') {
        if let Some(word) = line.strip_prefix("=== ") {
            found.push(Case { word: word.trim().to_string(), ..Case::default() });
            part = "";
        } else if let Some(name) = line.strip_prefix("--- ") {
            let (name, rest) = name.trim().split_once(' ').unwrap_or((name.trim(), ""));
            part = ["written", "laid", "wrong", "refused"]
                .into_iter()
                .find(|known| *known == name)
                .unwrap_or("");
            assert!(!part.is_empty(), "`{name}` is no part of a case");
            found.last_mut().expect("a part is of a case").at = match part {
                "refused" => rest.to_string(),
                _ => continue,
            };
        } else if let Some(case) = found.last_mut() {
            match part {
                "written" => case.written.push_str(line),
                "laid" => case.laid.push_str(line),
                "wrong" => case.wrong.push_str(line),
                "refused" => case.why.push_str(line),
                _ => {}
            }
        } else {
            opening.push_str(line);
        }
    }
    for case in &mut found {
        // The line left empty between two cases is not part of either.
        case.why = case.why.trim().to_string();
    }
    (opening, found)
}

/// The first thing the reader refuses in a text: where, and why.
fn refused(text: &str) -> Option<(String, String)> {
    let checked = check(text);
    let first = checked.errors().next()?;
    Some((format!("{}:{}", first.span.start.line, first.span.start.col), first.message.clone()))
}

/// Whether line `line` of `text` is where `word` is written, or something
/// that line is under: the reader points at a word, or at what holds it
/// (the step whose action is not one, the handler whose event is not).
fn points_at(text: &str, word: &str, line: usize) -> bool {
    let lines: Vec<&str> = text.lines().collect();
    let set_in = |line: &str| line.len() - line.trim_start_matches(' ').len();
    lines.iter().enumerate().filter(|(_, written)| written.contains(&format!("{word}:"))).any(
        |(at, written)| {
            let mut within = set_in(written);
            (0..=at).rev().any(|above| {
                let holds = above == at || set_in(lines[above]) < within;
                within = within.min(set_in(lines[above]));
                holds && above + 1 == line
            })
        },
    )
}

fn laid(text: &str) -> String {
    format(text, &Settings::default()).unwrap_or_else(|left| panic!("{}\n{text}", left.message()))
}

#[test]
fn every_word_of_the_language_has_a_case_that_reads_is_laid_out_and_is_refused_written_wrongly() {
    let folder = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/words");
    let write = std::env::var_os("WRITE_WORDS").is_some();
    let (mut held, mut wrong) = (BTreeSet::new(), Vec::new());
    for shape in SHAPES {
        let file = folder.join(format!("{}.txt", shape.name));
        let text = std::fs::read_to_string(&file).unwrap_or_default();
        let (opening, mut found) = cases(&text);
        if write {
            let mut kept = opening;
            for case in &mut found {
                case.laid = laid(&case.written);
                if let Some((at, why)) = refused(&case.wrong) {
                    (case.at, case.why) = (at, why);
                }
                kept.push_str(&format!(
                    "=== {}\n--- written\n{}--- laid\n{}--- wrong\n{}--- refused {}\n{}\n\n",
                    case.word, case.written, case.laid, case.wrong, case.at, case.why
                ));
            }
            std::fs::write(&file, format!("{}\n", kept.trim_end())).unwrap();
        }
        for case in &found {
            let Some((of, word)) = case.word.split_once('.').filter(|(of, _)| *of == shape.name) else {
                wrong.push(format!("`{}` is not a word of a {}, and is in its file", case.word, shape.name));
                continue;
            };
            let mut fault = |what: String| wrong.push(format!("{}: {what}", case.word));
            if !written(&case.written).iter().any(|(shape, name)| (*shape, *name) == (of, word)) {
                fault("its case is not written with it".into());
            }
            if let Some((at, why)) = refused(&case.written) {
                fault(format!("its case does not read: {at}: {why}"));
            }
            let out = laid(&case.written);
            if out != case.laid {
                fault(format!("it is laid out as\n{out}and its case says\n{}", case.laid));
            }
            if !means_the_same(&case.written, &out) || laid(&out) != out {
                fault("laid out, it means something else or is not settled".into());
            }
            match refused(&case.wrong) {
                None => fault("written wrongly, it is not refused".into()),
                Some((at, why)) => {
                    let line = at.split(':').next().and_then(|line| line.parse::<usize>().ok()).unwrap_or(0);
                    if !points_at(&case.wrong, word, line) {
                        let there = case.wrong.lines().nth(line.saturating_sub(1)).unwrap_or_default();
                        fault(format!(
                            "written wrongly, what is refused is not it but {at} (`{}`): {why}",
                            there.trim()
                        ));
                    }
                    if (at.as_str(), why.as_str()) != (case.at.as_str(), case.why.as_str()) {
                        fault(format!(
                            "it is refused at {at}: {why}\nand its case says {}: {}",
                            case.at, case.why
                        ));
                    }
                }
            }
            held.insert((of.to_string(), word.to_string()));
        }
    }
    for shape in SHAPES {
        for field in shape.fields {
            if !held.contains(&(shape.name.to_string(), field.name.to_string())) {
                wrong.push(format!("`{}` of a {} has no case in {}.txt", field.name, shape.name, shape.name));
            }
        }
    }
    assert!(wrong.is_empty(), "{} of the language's words are not held:\n{}", wrong.len(), wrong.join("\n"));
}

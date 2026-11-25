//! A blueprint said in plain words: the one function, by its cases.
//!
//! A case is a small text in `tests/fixtures/summary` and, beside it, what
//! it comes to as a terminal prints it. `WRITE_SUMMARY=1` writes what the
//! function says now into the files, for a new case. Read what it wrote
//! before it is kept: it is what is held from then on.

use std::path::{Path, PathBuf};

use blueprint::summary::{Known, Line, Summary, plain, says, summary};
use blueprint::{ActionSpec, EventSpec, ValueType, means_the_same};

/// The actions the cases use, shaped like the real ones: one that makes
/// something, one that runs a command, one that only reads.
fn actions() -> Vec<ActionSpec> {
    use ValueType::*;
    vec![
        ActionSpec::new("thing.make", "Make sure a thing is there. One that is there is left as it is.")
            .making("a thing", "name")
            .required("name", String, "what it is called")
            .capability("things:write"),
        ActionSpec::new("thing.read", "Read a thing.")
            .required("id", String, "which")
            .capability("things:read"),
        ActionSpec::new("tool.run", "Run a command and wait for it to finish.")
            .runs("run")
            .runs("cmd")
            .arg("run", String, "a line a shell reads")
            .arg("cmd", List, "a program and its arguments")
            .capability("shell"),
        ActionSpec::new("table.make", "Make sure a table is there, made as it says.")
            .making("a table", "name")
            .modelled(blueprint::Modelled {
                parts: "fields",
                kind: ("type", "text"),
                of: "of",
                naming: &["record", "records"],
                required: "required",
                takes: ("takes", "each", "whoever writes a row holds each scope it then says"),
                unsaid: "unsaid",
                suggests: "suggests",
                beside: ("beside", "about"),
                by_hand: "by_hand",
            })
            .required("name", String, "what it is called")
            .required("fields", Object, "what its rows are made of")
            .arg("beside", String, "the table it sits beside")
            .arg("about", String, "what of that a row is about")
            .arg("by_hand", Bool, "whether its rows are written by hand")
            .capability("things:write"),
        ActionSpec::new("letter.send", "Send a letter.")
            .required("to", String, "whom")
            .capability("post:send"),
    ]
}

fn events() -> Vec<EventSpec> {
    use ValueType::*;
    vec![
        EventSpec::new("thing.made", "A thing was made. It is there from then on.")
            .field("id", String, "which"),
        EventSpec::new("thing.changed", "A thing was changed.").field("id", String, "which"),
        EventSpec::new("thing.removed", "A thing was taken away.").field("id", String, "which"),
    ]
}

const SCOPES: &[(&str, &str)] = &[
    ("things:read", "read things"),
    ("things:write", "make and change things"),
    ("blueprints:run", "start, apply and stop blueprint instances"),
    ("secrets:use", "use what the factory keeps secret, without being shown it: hand one to a program"),
    ("shell", "run commands on a device"),
    ("post", "send letters; `post:<word>` is one kind"),
];

fn said(source: &str) -> Summary {
    let (actions, events) = (actions(), events());
    let known = Known { actions: &actions, events: &events, scopes: SCOPES };
    let mut all = blueprint::ActionList::new(actions.clone());
    all.push(ActionSpec::new("unused", "Nothing."));
    let checked = blueprint::check(source, &all);
    let errors: Vec<String> = checked.errors().map(|one| one.message.clone()).collect();
    assert!(errors.is_empty(), "the case checks: {errors:?}\n{source}");
    summary(&checked.blueprint.expect("it reads"), &known)
}

fn cases() -> Vec<PathBuf> {
    let folder = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/summary");
    let mut found: Vec<PathBuf> = std::fs::read_dir(&folder)
        .unwrap()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ending| ending == "bp"))
        .collect();
    found.sort();
    found
}

#[test]
fn each_case_comes_to_the_words_kept_beside_it() {
    let cases = cases();
    assert!(cases.len() >= 6, "the cases were found: {}", cases.len());
    let mut wrong = Vec::new();
    for case in cases {
        let source = std::fs::read_to_string(&case).unwrap();
        let now = plain(&said(&source));
        let kept = case.with_extension("txt");
        if std::env::var_os("WRITE_SUMMARY").is_some() {
            std::fs::write(&kept, &now).unwrap();
        }
        let was = std::fs::read_to_string(&kept).unwrap_or_default();
        if was != now {
            wrong.push(format!("{}:\n--- kept\n{was}\n--- now\n{now}", case.display()));
        }
    }
    assert!(
        wrong.is_empty(),
        "a summary is not what is kept beside its case. If it is meant, write them again with \
         `WRITE_SUMMARY=1 cargo test -p blueprint each_case` and read what was written:\n{}",
        wrong.join("\n\n")
    );
}

#[test]
fn a_text_laid_out_another_way_or_given_a_comment_comes_to_the_same_summary() {
    for case in cases() {
        let source = std::fs::read_to_string(&case).unwrap();
        let laid = blueprint::format(&source, &blueprint::Settings::default()).expect("it is laid out");
        // Every line pushed down and in, a comment at its head and one after each line that ends in a colon.
        let noted: String = laid
            .lines()
            .map(
                |line| {
                    if line.ends_with(':') { format!("{line}  # said here\n") } else { format!("{line}\n") }
                },
            )
            .collect();
        let other = format!("# A comment at its head.\n\n\n{noted}");
        assert!(means_the_same(&source, &other), "{}: the other text means the same", case.display());
        assert_eq!(said(&source), said(&other), "{}", case.display());
        assert_eq!(said(&source), said(&laid), "{}", case.display());
    }
}

#[test]
fn no_line_of_any_case_holds_a_hole_or_is_empty_and_no_part_is_headed_with_nothing_under_it() {
    fn lines<'a>(of: &'a [blueprint::summary::Line], found: &mut Vec<&'a blueprint::summary::Line>) {
        for line in of {
            found.push(line);
            lines(&line.under, found);
        }
    }
    for case in cases() {
        let summary = said(&std::fs::read_to_string(&case).unwrap());
        for part in &summary.parts {
            assert!(!part.lines.is_empty(), "{}: `{}` has no line", case.display(), part.title);
            let mut all = Vec::new();
            lines(&part.lines, &mut all);
            for line in all {
                assert!(
                    !line.text.trim().is_empty(),
                    "{}: an empty line under `{}`",
                    case.display(),
                    part.title
                );
                assert!(!line.text.contains("${"), "{}: {}", case.display(), line.text);
            }
        }
    }
}

#[test]
fn a_summary_is_the_same_read_back_from_what_it_is_handed_out_as() {
    for case in cases() {
        let summary = said(&std::fs::read_to_string(&case).unwrap());
        let handed = serde_json::to_value(&summary).unwrap();
        assert_eq!(serde_json::from_value::<Summary>(handed).unwrap(), summary, "{}", case.display());
    }
}

/// The one rule for what a step says of itself, held to the table the
/// client's own rule is held to (`says` in `ui/src/graph/tile.ts`).
#[test]
fn what_a_step_says_of_itself_is_cut_as_the_client_cuts_it() {
    let table = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../ui/src/graph/says.json");
    let cases: Vec<(String, String)> =
        serde_json::from_str(&std::fs::read_to_string(table).unwrap()).expect("a list of pairs");
    assert!(cases.len() >= 12, "the table was found");
    for (description, cut) in cases {
        assert_eq!(says(&description), cut, "{description:?}");
    }
}

/// The actions that are the language's own are said by what a step
/// hands them, and each has a case: one added to the language with none
/// fails here by name, where it would be said by one sentence for all
/// of its steps.
#[test]
fn every_action_of_the_languages_own_has_a_case_that_says_it_by_what_it_is_handed() {
    let all: String = cases().iter().map(|case| std::fs::read_to_string(case).unwrap()).collect();
    for own in blueprint::intrinsics() {
        assert!(
            all.contains(&format!("do: {}\n", own.name)),
            "`{}` is an action of the language and no case of `tests/fixtures/summary` has a step that asks it",
            own.name
        );
    }
    let steps = cases().into_iter().find(|case| case.ends_with("steps.bp")).expect("the case of steps");
    let said = plain(&said(&std::fs::read_to_string(steps).unwrap()));
    for line in [
        "Say `shop.told`. [told]",
        "Keep `last` and `seen`. [kept]",
        "Wait 5 minutes. [paused]",
        "Say `shop.asked`, and wait until `shop.answered` is said, for 1 hour at most. [asked]",
        "Keep an entry in its list `open`. [held]",
        "Take an entry out of its list `open`. [let_go]",
        "Measure `shop.counted`. [counted]",
        "Do nothing. [nothing]",
        "Stop `helper` and everything it started. [child]",
        "Fail here, on purpose. Only when the param `count` is less than `0`. [wrong]",
        "End this run here, as failed. [over]",
    ] {
        assert!(said.contains(line), "`{line}` in:\n{said}");
    }
}

/// A line taken apart, as a page lays it out: every line that is, with
/// its parts each on a line under it, and what it is about.
fn shaped(summary: &Summary) -> String {
    fn lines(of: &[Line], depth: usize, out: &mut String) {
        for line in of {
            let pad = "  ".repeat(depth);
            let about =
                line.at.as_deref().or(line.key.as_deref()).map(|at| format!(" [{at}]")).unwrap_or_default();
            match (&line.label, &line.title) {
                (Some(label), _) => out.push_str(&format!("{pad}{label}:{about}\n")),
                (None, Some(title)) => out.push_str(&format!("{pad}{title}{about}\n")),
                (None, None) => out.push_str(&format!("{pad}= {}{about}\n", line.text)),
            }
            let said = [
                (
                    "title",
                    line.label.as_ref().and(line.title.as_ref()).into_iter().cloned().collect::<Vec<_>>(),
                ),
                ("items", line.items.clone()),
                ("from", line.from.iter().cloned().collect()),
                ("only", line.only.clone()),
                ("tags", line.tags.clone()),
                ("named", line.named.iter().cloned().collect()),
                ("by", line.by.iter().cloned().collect()),
                ("ending", line.ending.iter().cloned().collect()),
                (
                    "opens",
                    line.opens
                        .iter()
                        .map(|opens| format!("{} {}", opens.what, opens.name.as_deref().unwrap_or("")))
                        .collect(),
                ),
            ];
            let made = line.made.iter().map(|made| {
                let called = made.called.as_deref().map(|called| format!(" ({called})")).unwrap_or_default();
                format!("{} {}{called}", made.what, made.name.as_deref().unwrap_or(""))
            });
            let model = line.model.iter().flat_map(|model| {
                let fields = model.fields.iter().map(|field| {
                    let names =
                        field.names.as_deref().map(|names| format!(" -> {names}")).unwrap_or_default();
                    format!("{}: {}{names} {:?}", field.name, field.kind, field.about)
                });
                model
                    .as_said
                    .iter()
                    .cloned()
                    .chain(fields)
                    .chain(model.said.iter().cloned())
                    .collect::<Vec<_>>()
            });
            let said = said.into_iter().chain([("made", made.collect()), ("model", model.collect())]);
            for (what, words) in said {
                for word in words {
                    out.push_str(&format!("{pad}  {what}: {word}\n"));
                }
            }
            lines(&line.under, depth + 1, out);
        }
    }
    let mut out = format!("{}\n", summary.name);
    for part in &summary.parts {
        out.push_str(&format!("\n{}\n", part.title));
        lines(&part.lines, 1, &mut out);
    }
    out
}

/// What a page draws in place of a line's words is kept beside each case
/// too (`<case>.parts.txt`), written by the same `WRITE_SUMMARY`: a line
/// that has no parts is its words after `=`.
#[test]
fn each_case_is_taken_apart_as_is_kept_beside_it() {
    let mut wrong = Vec::new();
    for case in cases() {
        let now = shaped(&said(&std::fs::read_to_string(&case).unwrap()));
        let kept = case.with_extension("parts.txt");
        if std::env::var_os("WRITE_SUMMARY").is_some() {
            std::fs::write(&kept, &now).unwrap();
        }
        let was = std::fs::read_to_string(&kept).unwrap_or_default();
        if was != now {
            wrong.push(format!("{}:\n--- kept\n{was}\n--- now\n{now}", case.display()));
        }
    }
    assert!(
        wrong.is_empty(),
        "a summary is not taken apart as is kept beside its case. If it is meant, write them again with \
         `WRITE_SUMMARY=1 cargo test -p blueprint each_case` and read what was written:\n{}",
        wrong.join("\n\n")
    );
}

/// What a page lays out of a line says nothing its text does not: where
/// it is heard from and what holds it are sentences of its text, in
/// order; every step, hook and handler has a title; and no part of any
/// line holds a hole.
#[test]
fn a_lines_parts_say_what_its_text_says_and_every_step_and_handler_has_a_title() {
    /// Each line, and whether it is something done or heard, which has a title.
    fn every<'a>(of: &'a [Line], titled: bool, deep: bool, found: &mut Vec<(&'a Line, bool)>) {
        for line in of {
            found.push((line, titled && (deep || line.at.is_some())));
            every(&line.under, titled, true, found);
        }
    }
    for case in cases() {
        let summary = said(&std::fs::read_to_string(&case).unwrap());
        let mut all = Vec::new();
        for part in &summary.parts {
            every(&part.lines, ["does", "hears", "ends"].contains(&part.kind.as_str()), false, &mut all);
        }
        for (line, titled) in all {
            let of = format!("{}: {}", case.display(), line.text);
            let mut rest = line.text.as_str();
            for sentence in line.from.iter().chain(&line.only) {
                let at =
                    rest.find(sentence.as_str()).unwrap_or_else(|| panic!("{of}: `{sentence}` in order"));
                rest = &rest[at + sentence.len()..];
            }
            if titled {
                assert!(line.title.as_ref().is_some_and(|title| !title.trim().is_empty()), "{of}: a title");
            }
            if line.label.is_some() {
                assert!(line.title.is_some() != !line.items.is_empty(), "{of}: a fact is a title or a list");
            }
            let pieces = line.title.iter().chain(&line.label).chain(&line.items).chain(&line.from);
            let pieces = pieces.chain(&line.only).chain(&line.tags).chain(&line.named).chain(&line.by);
            let model = line.model.iter().flat_map(|model| {
                let fields = model
                    .fields
                    .iter()
                    .flat_map(|field| [&field.kind, &field.text].into_iter().chain(&field.about));
                model.as_said.iter().chain(fields).chain(&model.said)
            });
            let pieces = pieces
                .chain(model)
                .chain(line.made.iter().flat_map(|made| std::iter::once(&made.what).chain(&made.name)));
            for piece in pieces {
                assert!(!piece.trim().is_empty() && !piece.contains("${"), "{of}: `{piece}`");
            }
        }
    }
}

/// Each line of what is done as a blueprint ends says which ending it is,
/// so that a page that plays it can play each as what it is; no other
/// line says one.
#[test]
fn each_ending_says_which_it_is_and_nothing_else_says_one() {
    let ends = cases().into_iter().find(|case| case.ends_with("ends.bp")).expect("the case of endings");
    let summary = said(&std::fs::read_to_string(ends).unwrap());
    let part = summary.parts.iter().find(|part| part.kind == "ends").expect("what it does as it ends");
    let endings: Vec<&str> = part.lines.iter().map(|line| line.ending.as_deref().unwrap_or("")).collect();
    assert_eq!(endings, ["success", "fail", "finish", "update", "stop"]);
    fn none(of: &[Line]) -> bool {
        of.iter().all(|line| line.ending.is_none() && none(&line.under))
    }
    for case in cases() {
        let summary = said(&std::fs::read_to_string(&case).unwrap());
        for part in &summary.parts {
            let lines = if part.kind == "ends" {
                part.lines.iter().flat_map(|line| line.under.clone()).collect()
            } else {
                part.lines.clone()
            };
            assert!(none(&lines), "{}: only an ending's own line says which it is", case.display());
        }
    }
}

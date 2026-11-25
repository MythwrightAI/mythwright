//! What becomes of a contest and of each of its entries.
//!
//! Asked once the contenders have ended, before anything is judged, it
//! says whether there is anything to judge: two entries or more whose
//! work was kept are judged; with one, the contest's `alone` says (left
//! for a person, the usual, or that one taken with nothing judged); with
//! none, the contest is left, saying why. Asked again with what the
//! judging came to (a winner, a person to pick, or nothing) or with a
//! person's answer, it says each entry's verdict and what becomes of its
//! place (`take`: it becomes the item's; `keep`: its branch is kept;
//! `discard`: thrown away; `wait`: nothing yet), what the contest's row
//! says, and the word the segment that ran it ends in.
//!
//! What becomes of the work that did not win is the contest's `losers`
//! (`branch`, the usual: kept as a branch; or `discard`), and of a
//! judging that decided nothing its `undecided` (`person`, the usual: a
//! person is asked which; or `left`). Nothing is thrown away that nobody
//! decided against: a contest that is left keeps every branch.
//!
//! A shadow test names its incumbent (`incumbent`, a way): the incumbent's
//! work is the one taken, whatever the judges say, and nobody is asked to
//! pick. The others, its challengers, are judged beside it (`challengers:
//! judged`, the usual: each verdict is said and counted as any is) or only
//! counted (`counted`: nothing is judged, and what each spent and took is
//! what tells them apart). A challenger's place is kept or thrown away as
//! `losers` says, whatever its verdict, so that its work never becomes
//! the item's. An incumbent whose work could not be kept leaves the
//! contest for a person, every branch kept: no challenger is taken in its
//! place.

use blueprint::{ActionSpec, ValueType as T};
use conductor::{ActionError, Registry};
use serde_json::{Value, json};

use super::described::{self, NO, YES};
use super::{said, text};
use crate::add;

/// One entry, as the contest's row of it says.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Entry {
    pub way: String,
    pub letter: String,
    /// `kept` where its work was kept; anything else (`unkept`, `failed`)
    /// where it was not.
    pub state: String,
    /// Its program could see the other contenders' work: said with what
    /// the contest came to, and never left out.
    pub unhidden: bool,
    /// What a person asked is told of it, as its row says: the agent
    /// that worked it, its branch, what it changed as `git diff --stat`
    /// says it, and what its agent said ([`super::described`]).
    pub agent: String,
    pub branch: String,
    pub changed: String,
    pub said: String,
}

/// What a contest is told to decide by.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Asked {
    pub entries: Vec<Entry>,
    /// What the judging came to: empty where nothing was judged yet,
    /// `won` with the way that won, `person`, or `undecided`.
    pub judged: String,
    pub winner: String,
    /// What a person answered where one was asked: a letter or a way.
    pub answer: String,
    /// Why the judging came to nothing, or what a person is asked, as the
    /// judging said it.
    pub why: String,
    pub question: String,
    /// The settings: `losers`, `undecided`, `alone`.
    pub losers: String,
    pub undecided: String,
    pub alone: String,
    /// Who judged, to be said with the winner.
    pub by: String,
    /// The incumbent's way, in a shadow test; nothing otherwise.
    pub incumbent: String,
    /// What a shadow test's challengers are: `judged` (the usual) or `counted`.
    pub challengers: String,
}

/// What becomes of it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Outcome {
    /// `judge` (there is something to judge), `won`, `asks` (a person is
    /// asked) or `left`.
    pub came: &'static str,
    /// The word the segment that ran the contest ends in: `on`, `asks` or
    /// `left`; nothing while it is to be judged.
    pub ends: &'static str,
    /// What the contest's row says it is: `judging`, `came`, `asking` or `left`.
    pub state: &'static str,
    pub winner: String,
    /// For each entry, as handed: its verdict and what becomes of its place.
    pub entries: Vec<(&'static str, &'static str)>,
    /// The entries to judge, where there is something to judge.
    pub judge: Vec<usize>,
    pub question: String,
    pub why: String,
    /// A line for the item's history, where the contest came to something.
    pub said: String,
}

/// Every word [`outcome`] can answer.
pub const OUTCOME_WORDS: &[&str] = &["judge", "won", "asks", "left"];

/// What becomes of the contest `asked` is about.
pub fn outcome(asked: &Asked) -> Result<Outcome, String> {
    let one_of = |word: &str, setting: &str, words: &[&str]| -> Result<String, String> {
        let word = if word.is_empty() { words[0] } else { word };
        match words.contains(&word) {
            true => Ok(word.to_string()),
            false => Err(format!("`{setting}` is one of {}; it is `{word}`", words.join(", "))),
        }
    };
    let losers = one_of(&asked.losers, "losers", &["branch", "discard"])?;
    let undecided = one_of(&asked.undecided, "undecided", &["person", "left"])?;
    let alone = one_of(&asked.alone, "alone", &["person", "take"])?;
    let challengers = one_of(&asked.challengers, "challengers", &["judged", "counted"])?;
    let entries = &asked.entries;
    if entries.is_empty() {
        return Err("a contest's outcome is decided for its entries, and none is handed".into());
    }
    let kept: Vec<usize> = (0..entries.len()).filter(|at| entries[*at].state == "kept").collect();
    let lost = if losers == "discard" { "discard" } else { "keep" };
    let ways =
        |only: &[usize]| only.iter().map(|at| entries[*at].way.as_str()).collect::<Vec<_>>().join(" and ");
    let all: Vec<usize> = (0..entries.len()).collect();
    let apart: Vec<&str> =
        entries.iter().filter(|entry| entry.unhidden).map(|entry| entry.way.as_str()).collect();
    let apart = |said: String| match apart.is_empty() {
        true => said,
        false => format!(
            "{said}; not held apart, and so could see the other contenders' work: {}",
            apart.join(", ")
        ),
    };
    let not_kept = || {
        let each: Vec<String> = entries
            .iter()
            .filter(|entry| entry.state != "kept")
            .map(|entry| {
                format!("{} {}", entry.way, if entry.state.is_empty() { "ended" } else { &entry.state })
            })
            .collect();
        each.join(", ")
    };
    let waiting =
        |came: &'static str, ends: &'static str, state: &'static str, question: String, why: String| {
            Outcome {
                came,
                ends,
                state,
                entries: vec![("", "wait"); entries.len()],
                question,
                why,
                ..Default::default()
            }
        };
    let won = |at: usize, why: String, by: &str| {
        let verdicts = (0..entries.len())
            .map(|other| match (other == at, kept.contains(&other)) {
                (true, _) => ("won", "take"),
                (false, true) => ("lost", lost),
                (false, false) => ("", lost),
            })
            .collect();
        let by = if by.is_empty() { String::new() } else { format!(" by {by}") };
        Outcome {
            came: "won",
            ends: "on",
            state: "came",
            winner: entries[at].way.clone(),
            entries: verdicts,
            judge: Vec::new(),
            question: String::new(),
            said: apart(format!("its A/B test of {} was judged{by}: {}", ways(&all), entries[at].way)),
            why,
        }
    };
    let left = |why: String| Outcome {
        came: "left",
        ends: "left",
        state: "left",
        entries: vec![("", "keep"); entries.len()],
        said: apart(format!("its A/B test of {} came to nothing: {why}", ways(&all))),
        why,
        ..Default::default()
    };
    let asking = |lead: &str| described::question(entries, &kept, lead);
    // A shadow test: the incumbent goes on whatever is judged, and nobody is asked.
    if !asked.incumbent.is_empty() {
        let Some(at) = entries.iter().position(|entry| entry.way == asked.incumbent) else {
            return Err(format!("the incumbent `{}` is none of the contest's entries", asked.incumbent));
        };
        let incumbent = entries[at].way.as_str();
        if !kept.contains(&at) {
            return Ok(left(format!(
                "its incumbent {incumbent}'s work could not be kept ({}), and a shadow test takes no \
                 challenger's in its place",
                not_kept()
            )));
        }
        let rivals = kept.iter().any(|other| *other != at);
        let goes_on = |verdict: &dyn Fn(usize) -> &'static str, said: String, why: String| {
            let each = (0..entries.len())
                .map(|other| {
                    (
                        if kept.contains(&other) { verdict(other) } else { "" },
                        if other == at { "take" } else { lost },
                    )
                })
                .collect();
            let said =
                format!("its shadow test of {} {said}; its incumbent, {incumbent}, goes on", ways(&all));
            Outcome {
                came: "won",
                ends: "on",
                state: "came",
                winner: incumbent.to_string(),
                entries: each,
                said: apart(said),
                why: format!("a shadow test, whose incumbent's work is taken whatever is judged: {why}"),
                ..Default::default()
            }
        };
        let by = if asked.by.is_empty() { String::new() } else { format!(" by {}", asked.by) };
        return match asked.judged.as_str() {
            "" if challengers == "judged" && rivals => Ok(Outcome {
                came: "judge",
                state: "judging",
                entries: vec![("", "wait"); entries.len()],
                judge: kept.clone(),
                ..Default::default()
            }),
            "" if !rivals => Ok(goes_on(
                &|_| "",
                format!("had no challenger whose work was kept ({})", not_kept()),
                "no challenger's work was kept".into(),
            )),
            "" => Ok(goes_on(
                &|_| "",
                "was counted and not judged".into(),
                "its challengers are counted and not judged".into(),
            )),
            "won" => match kept.iter().copied().find(|other| entries[*other].way == asked.winner) {
                Some(won) => Ok(goes_on(
                    &|other| if other == won { "won" } else { "lost" },
                    format!("was judged{by}: {}", entries[won].way),
                    format!("the judges took {}", entries[won].way),
                )),
                None => Err(format!(
                    "the judging says `{}` won, which is no entry whose work was kept",
                    asked.winner
                )),
            },
            "person" | "undecided" => Ok(goes_on(
                &|_| "undecided",
                format!("was judged{by} and decided nothing"),
                match asked.why.is_empty() {
                    true => "the judges decided nothing".into(),
                    false => format!("the judges decided nothing ({})", asked.why),
                },
            )),
            other => Err(format!("`{other}` is not what a judging comes to: `won`, `person` or `undecided`")),
        };
    }
    // A person's answer stands over anything else: an entry by its letter
    // or its way; yes, for the one entry whose work was kept; or no.
    if !asked.answer.is_empty() {
        let answer = asked.answer.trim();
        let word = described::word(answer);
        let named = |at: &usize| entries[*at].letter == answer || entries[*at].way == answer;
        let lettered = |at: &usize| entries[*at].letter.eq_ignore_ascii_case(&word);
        let found = kept.iter().copied().find(named);
        let found = match found {
            Some(at) => Some(at),
            None if YES.contains(&word.as_str()) && kept.len() == 1 => Some(kept[0]),
            None if YES.contains(&word.as_str()) || NO.contains(&word.as_str()) => None,
            None => kept.iter().copied().find(lettered),
        };
        if let Some(at) = found {
            return Ok(won(at, "a person picked it".into(), "a person"));
        }
        if NO.contains(&word.as_str()) {
            let taken = match kept[..] {
                [only] => format!("{}'s work is not taken", entries[only].way),
                _ => "no entry's work is taken".into(),
            };
            return Ok(left(format!(
                "a person answered `{answer}`, and so {taken}: the card is left for a person"
            )));
        }
        let lead = match (YES.contains(&word.as_str()), kept.len()) {
            (_, 0) => String::new(),
            (true, _) => format!("`{answer}` says not which: the work of {} entries was kept.\n", kept.len()),
            (false, 1) => format!("`{answer}` is not yes or no, nor one of the entries. "),
            (false, _) => format!("`{answer}` is none of the entries whose work was kept.\n"),
        };
        if !kept.is_empty() {
            return Ok(waiting(
                "asks",
                "asks",
                "asking",
                asking(&lead),
                format!("`{answer}` was answered, and is none of the entries"),
            ));
        }
    }
    if kept.is_empty() {
        return Ok(left(format!(
            "none of its {} contenders' work could be kept ({})",
            entries.len(),
            not_kept()
        )));
    }
    match asked.judged.as_str() {
        "" => match kept[..] {
            [only] if alone == "take" => {
                Ok(won(only, format!("it was the only contender whose work was kept ({})", not_kept()), ""))
            }
            [_] => Ok(waiting(
                "asks",
                "asks",
                "asking",
                asking(""),
                "one contender's work was kept, and the contest says a person decides".into(),
            )),
            _ => Ok(Outcome {
                came: "judge",
                state: "judging",
                entries: vec![("", "wait"); entries.len()],
                judge: kept.clone(),
                ..Default::default()
            }),
        },
        "won" => match kept.iter().copied().find(|at| entries[*at].way == asked.winner) {
            Some(at) => Ok(won(at, String::new(), &asked.by)),
            None => {
                Err(format!("the judging says `{}` won, which is no entry whose work was kept", asked.winner))
            }
        },
        "person" => Ok(waiting("asks", "asks", "asking", asking(&asked.question), asked.why.clone())),
        "undecided" if undecided == "person" => Ok(waiting(
            "asks",
            "asks",
            "asking",
            asking(&format!("The judges decided nothing ({}). Which entry is taken?", asked.why)),
            asked.why.clone(),
        )),
        "undecided" => Ok(left(format!("the judges decided nothing ({})", asked.why))),
        other => Err(format!("`{other}` is not what a judging comes to: `won`, `person` or `undecided`")),
    }
}

/// Adds `contest.outcome` to what a blueprint can do.
pub fn register(registry: &mut Registry) {
    add(
        registry,
        ActionSpec::new(
            "contest.outcome",
            "Say what becomes of a contest and of each of its entries: something to judge, the entry taken, a person asked, or the contest left.",
        )
        .deciding()
        .required("entries", T::List, "the entries, each its `way`, `letter` and `state` (`kept` where its work was kept), and what a person asked is told of it: its `agent`, `branch`, what it `changed` (as `git diff --stat` says) and what its agent `said`; its row's `id`, `key` and `not_held_apart` are handed back with it")
        .arg("judged", T::String, "what the judging came to: `won`, `person` or `undecided`; nothing where nothing was judged yet")
        .arg("winner", T::String, "the way that won, where the judging says one did")
        .arg("answer", T::String, "what a person answered where one was asked: a letter or a way; yes (`y`, `take`, `ok`) for the one entry whose work was kept; or no (`n`, `leave`), which leaves the item for a person")
        .arg("answered", T::Bool, "whether `answer` is a person's answer; what was said is taken for one only where this says so (true where not said and `answer` is said)")
        .arg("said", T::List, "what the judges said of each entry, by its `way`: `said` (a judge's word each) or `words`, kept with it as `scores`")
        .arg("why", T::String, "why the judging came to nothing or asks a person, as it said")
        .arg("question", T::String, "what the judging said a person is asked")
        .arg("by", T::String, "who judged, to be said with the winner")
        .arg("losers", T::String, "what becomes of work that did not win: `branch` (kept as a branch; the usual) or `discard`")
        .arg("undecided", T::String, "what a judging that decided nothing comes to: `person` (the usual) or `left`")
        .arg("alone", T::String, "what one contender left comes to: `person` (the usual) or `take`")
        .arg("incumbent", T::String, "the incumbent's way, in a shadow test: its work is taken whatever is judged, and nobody is asked; nothing for a test of equals")
        .arg("challengers", T::String, "what a shadow test's challengers are: `judged` beside the incumbent (the usual) or only `counted`")
        .output("came", "`judge`, `won` (an entry taken: the winner, or a shadow test's incumbent), `asks` or `left`")
        .output("ends", "the word the segment ends in: `on`, `asks` or `left`; nothing while it is to be judged")
        .output("state", "what the contest's row says: `judging`, `came`, `asking` or `left`")
        .output("winner", "the way taken: in a shadow test its incumbent, whatever the judges took")
        .output("entries", "each entry's `way`, `verdict` (`won`, `lost`, `undecided` in a shadow test, or nothing) and what becomes of its place (`fate`: `take`, `keep`, `discard` or `wait`)")
        .output("judge", "the entries to judge, as handed, where there is something to judge")
        .output("question", "what a person is asked")
        .output("why", "why, where it asks or is left")
        .output("said", "a line for the item's history, where it came to something, saying which entries were not held apart")
        .output("take", "the place of the entry taken, by its key: what becomes the item's; nothing where none is")
        .output("agent", "the agent that worked the entry taken, as its row says")
        .output("worked", "the agents that worked an entry, as their rows say, each once: those that may not judge")
        .output("keep", "the places whose branches are kept, by their keys")
        .output("discard", "the places thrown away, by their keys")
        .idempotent(),
        |_, args| async move {
            let rows = args.get("entries").and_then(Value::as_array).cloned().unwrap_or_default();
            let entries: Vec<Entry> = rows
                .iter()
                .map(|one| Entry {
                    way: text(one, "way"),
                    letter: text(one, "letter"),
                    state: text(one, "state"),
                    unhidden: one.get("not_held_apart").and_then(Value::as_bool).unwrap_or(false),
                    agent: text(one, "agent"),
                    branch: text(one, "branch"),
                    changed: text(one, "changed"),
                    said: text(one, "said"),
                })
                .collect();
            let asked = Asked {
                entries: entries.clone(),
                judged: said(&args, "judged"),
                winner: said(&args, "winner"),
                answer: match args.get("answered").and_then(Value::as_bool) {
                    Some(false) => String::new(),
                    _ => said(&args, "answer"),
                },
                why: said(&args, "why"),
                question: said(&args, "question"),
                losers: said(&args, "losers"),
                undecided: said(&args, "undecided"),
                alone: said(&args, "alone"),
                by: said(&args, "by"),
                incumbent: said(&args, "incumbent"),
                challengers: said(&args, "challengers"),
            };
            let came = outcome(&asked).map_err(ActionError::new)?;
            // What the judges said of each, a line a judge, kept on its row.
            let of_judges = args.get("said").and_then(Value::as_array).cloned().unwrap_or_default();
            let scores = |way: &str| -> Vec<String> {
                let said = of_judges.iter().find(|one| text(one, "way") == way);
                let lines = |value: &Value| match value {
                    Value::Array(each) => each.iter().map(blueprint::expr::display).collect(),
                    Value::String(words) => words.lines().map(str::to_string).collect(),
                    _ => Vec::new(),
                };
                let lines: Vec<String> = said
                    .map(|one| lines(one.get("said").or_else(|| one.get("words")).unwrap_or(&Value::Null)))
                    .unwrap_or_default();
                lines.into_iter().filter(|line| !line.trim().is_empty()).collect()
            };
            let each: Vec<Value> = rows
                .iter()
                .zip(&entries)
                .zip(&came.entries)
                .map(|((row, entry), (verdict, fate))| {
                    json!({
                        "way": entry.way, "verdict": verdict, "fate": fate, "id": text(row, "id"),
                        "key": text(row, "key"), "scores": scores(&entry.way),
                    })
                })
                .collect();
            let judge: Vec<Value> = came.judge.iter().map(|at| rows[*at].clone()).collect();
            let keys = |wanted: &str| -> Vec<String> {
                let fated = rows.iter().zip(&came.entries).filter(|(_, (_, fate))| *fate == wanted);
                fated.map(|(row, _)| text(row, "key")).filter(|key| !key.is_empty()).collect()
            };
            // Who worked an entry, as its row says, each once: none of them judges.
            let mut worked: Vec<String> = rows.iter().map(|row| text(row, "agent")).filter(|one| !one.is_empty()).collect();
            worked.sort();
            worked.dedup();
            let taken = rows.iter().zip(&came.entries).find(|(_, (_, fate))| *fate == "take");
            let (take, agent) = taken.map_or((String::new(), String::new()), |(row, _)| (text(row, "key"), text(row, "agent")));
            Ok(json!({
                "came": came.came, "ends": came.ends, "state": came.state, "winner": came.winner, "entries": each,
                "judge": judge, "question": came.question, "why": came.why, "said": came.said, "take": take,
                "agent": agent, "keep": keys("keep"), "discard": keys("discard"), "worked": worked,
            }))
        },
    );
}

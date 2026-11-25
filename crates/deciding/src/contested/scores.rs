//! What judges who each score every entry come to, by the contest's rule.
//!
//! Each judge ends with a line an entry, its letter and the score it
//! gives (`A: 7`). The scores are kept as they were said, a judge's word
//! each, and what they come to is the rule the contest's owner chose
//! (`then`): a person picks with the scores beside them (`person`, the
//! usual); the entry with the highest mean wins (`highest`); or the entry
//! most judges scored above every other wins (`most`). Whatever the rule,
//! a tie is left for a person: no rule here picks between two that came
//! out alike. A rule is a case of this function, never an expression in
//! a text.

use blueprint::{ActionSpec, ValueType as T};
use conductor::{ActionError, Registry};
use serde_json::{Value, json};

use super::{Known, count, known, said, text};
use crate::add;

/// The rules there are, the usual first.
pub const RULES: &[&str] = &["person", "highest", "most"];

/// What one judge said: who, and its words.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Reply {
    pub by: String,
    pub text: String,
}

/// What the scores came to.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Scored {
    /// `won`, `person` (a person picks), or `undecided` (nobody scored).
    pub came: &'static str,
    pub winner: String,
    /// For each entry, as handed, each judge's score of it as it was said,
    /// empty where it gave none that could be read.
    pub scores: Vec<Vec<String>>,
    /// Why a person picks, or why nothing was decided; and what a person
    /// is asked, with the scores beside it.
    pub why: String,
    pub question: String,
    /// What could not be read, as `<judge>: <letter>`.
    pub unread: Vec<String>,
}

/// Every word [`scores`] can answer.
pub const SCORES_WORDS: &[&str] = &["won", "person", "undecided"];

/// The score `reply` gives the entry `letter`, out of `scale`: the
/// number on a line that begins with the letter and a colon.
fn score(reply: &str, letter: &str, scale: i64) -> Option<i64> {
    reply.lines().find_map(|line| {
        let (said, rest) = line.trim().split_once(':')?;
        if said.trim() != letter {
            return None;
        }
        let number: i64 = rest.split_whitespace().next()?.trim_end_matches(['.', ',']).parse().ok()?;
        (0..=scale).contains(&number).then_some(number)
    })
}

/// What `replies` scoring `entries` out of `scale` come to by `then`.
pub fn scores(entries: &[Known], replies: &[Reply], scale: i64, then: &str) -> Result<Scored, String> {
    if entries.is_empty() {
        return Err("scores are read for the entries judged, and none is handed".into());
    }
    if scale < 1 {
        return Err(format!("`scale` is the highest score there is, and is at least one; it is {scale}"));
    }
    let then = if then.is_empty() { "person" } else { then };
    if !RULES.contains(&then) {
        return Err(format!(
            "`{then}` is no rule scores come to by; a contest says `person` (the usual), `highest` or `most`"
        ));
    }
    let mut unread = Vec::new();
    // Each judge's score of each entry, where it could be read.
    let read: Vec<Vec<Option<i64>>> = replies
        .iter()
        .map(|reply| {
            entries
                .iter()
                .map(|entry| {
                    let got = score(&reply.text, &entry.letter, scale);
                    if got.is_none() {
                        unread.push(format!("{}: {}", reply.by, entry.letter));
                    }
                    got
                })
                .collect()
        })
        .collect();
    let scores: Vec<Vec<String>> = (0..entries.len())
        .map(|at| read.iter().map(|one| one[at].map(|n| n.to_string()).unwrap_or_default()).collect())
        .collect();
    let any = read.iter().flatten().any(Option::is_some);
    let beside = || {
        let each: Vec<String> = entries
            .iter()
            .zip(&scores)
            .map(|(entry, said)| {
                let said: Vec<&str> =
                    said.iter().map(|one| if one.is_empty() { "-" } else { one.as_str() }).collect();
                format!("{}: {}", entry.letter, said.join(", "))
            })
            .collect();
        each.join("; ")
    };
    let letters = || entries.iter().map(|entry| entry.letter.as_str()).collect::<Vec<_>>().join(", ");
    let ask = |why: String| Scored {
        came: "person",
        winner: String::new(),
        scores: scores.clone(),
        question: format!("The judges scored {}. Which entry is taken ({})?", beside(), letters()),
        why,
        unread: unread.clone(),
    };
    if !any {
        return Ok(Scored {
            came: "undecided",
            scores: scores.clone(),
            why: format!("none of the {} judges gave a score that could be read", replies.len()),
            unread: unread.clone(),
            ..Default::default()
        });
    }
    // The entries the rule puts first, as their places among those handed.
    let first: Vec<usize> = match then {
        "highest" => {
            // The greatest mean, held without dividing: a sum against a count.
            let means: Vec<(i64, i64)> = (0..entries.len())
                .map(|at| {
                    let each: Vec<i64> = read.iter().filter_map(|one| one[at]).collect();
                    (each.iter().sum(), each.len() as i64)
                })
                .collect();
            let greater = |a: (i64, i64), b: (i64, i64)| a.0 * b.1.max(1) > b.0 * a.1.max(1);
            let scored: Vec<usize> = (0..entries.len()).filter(|at| means[*at].1 > 0).collect();
            let best = scored.iter().copied().fold(None, |best: Option<usize>, at| match best {
                Some(was) if !greater(means[at], means[was]) => Some(was),
                _ => Some(at),
            });
            let best = best.map(|best| means[best]).unwrap_or((0, 0));
            scored.into_iter().filter(|at| !greater(best, means[*at])).collect()
        }
        "most" => {
            let mut tops = vec![0usize; entries.len()];
            for one in &read {
                let high = one.iter().flatten().max().copied();
                let at: Vec<usize> =
                    (0..entries.len()).filter(|at| one[*at].is_some() && one[*at] == high).collect();
                // A judge that scored two alike above the rest put neither first.
                if let [only] = at[..] {
                    tops[only] += 1;
                }
            }
            let most = tops.iter().copied().max().unwrap_or(0);
            match most {
                0 => Vec::new(),
                _ => (0..entries.len()).filter(|at| tops[*at] == most).collect(),
            }
        }
        _ => return Ok(ask("the contest says a person picks from the scores".into())),
    };
    match first[..] {
        [won] => Ok(Scored {
            came: "won",
            winner: entries[won].way.clone(),
            scores,
            why: String::new(),
            question: String::new(),
            unread,
        }),
        [] => Ok(ask("no judge put one entry above every other".into())),
        _ => {
            let tied: Vec<&str> = first.iter().map(|at| entries[*at].letter.as_str()).collect();
            Ok(ask(format!("{} came out alike, and a tie is a person's to settle", tied.join(" and "))))
        }
    }
}

/// Adds `contest.scores` to what a blueprint can do.
pub fn register(registry: &mut Registry) {
    add(
        registry,
        ActionSpec::new(
            "contest.scores",
            "Read what judges scored each entry, and say what that comes to by the contest's rule: the entry that won, a person to pick, or nothing.",
        )
        .deciding()
        .required("entries", T::List, "the entries judged, each its `way` and the `letter` it is scored by")
        .required("replies", T::List, "what each judge said: `by`, and its `text`, a line an entry such as `A: 7`")
        .arg("scale", T::Int, "the highest score there is; 10 if not said")
        .arg("then", T::String, "what the scores come to: `person` (a person picks; the usual), `highest` (the highest mean) or `most` (the entry most judges scored above every other); a tie is a person's whatever is said")
        .output("came", "`won`, `person`, or `undecided` where no score could be read")
        .output("winner", "the way that won")
        .output("scores", "each entry's `way` and every judge's score of it as it was said (`said`, empty for none)")
        .output("why", "why a person picks, or why nothing was decided")
        .output("question", "what a person is asked, with the scores beside it")
        .output("unread", "the scores that could not be read, as `<judge>: <letter>`")
        .idempotent(),
        |_, args| async move {
            let entries: Vec<Known> = known(&args, "entries");
            let said_ = args.get("replies").and_then(Value::as_array).cloned().unwrap_or_default();
            let replies: Vec<Reply> =
                said_.iter().map(|one| Reply { by: text(one, "by"), text: text(one, "text") }).collect();
            let scale = match count(&args, "scale") {
                0 => 10,
                scale => scale,
            };
            let came = scores(&entries, &replies, scale, &said(&args, "then")).map_err(ActionError::new)?;
            let scores: Vec<Value> = entries
                .iter()
                .zip(&came.scores)
                .map(|(entry, said)| json!({ "way": entry.way, "said": said }))
                .collect();
            Ok(json!({
                "came": came.came, "winner": came.winner, "scores": scores, "why": came.why,
                "question": came.question, "unread": came.unread,
            }))
        },
    );
}

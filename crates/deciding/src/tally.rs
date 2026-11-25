//! What a review comes to: the one thing a council works out.
//!
//! Those who look at a card each say what they make of it, one at a time
//! and in no order. After each, `std/review` asks here whether that
//! settles it: it keeps the votes, and says what was decided to whatever
//! started it. What is counted, and when enough is enough, is this
//! function of the votes there are and how many were to be cast.
//!
//! Which of the words an agent may choose is counted for the work and
//! which against is whoever asks's to say (`yes`, `no`): `approve` and
//! `send_back` where nothing is said, which is what a review's seat is
//! offered. Any other word is neither.
//!
//! Nothing here scores anybody: a vote is what its agent chose, and what
//! a review comes to is a count of those.

use blueprint::{ActionSpec, ValueType as T};
use conductor::{ActionError, Registry};
use serde_json::{Value, json};

use crate::add;

/// What one who looked said: who, what they chose, and their words.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Vote {
    pub by: String,
    pub decision: String,
    pub words: String,
}

/// What the votes are counted with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Counted<'a> {
    /// Every vote cast so far, this one last.
    pub votes: &'a [Vote],
    /// How many look, and how many have to approve: more than half,
    /// where none is said (`0`).
    pub seats: i64,
    pub needed: i64,
    /// The choice counted for the work, and the one counted against it.
    pub yes: &'a str,
    pub no: &'a str,
}

/// The choices counted where whoever asks says none.
pub const YES: &str = "approve";
pub const NO: &str = "send_back";

/// What the votes came to.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Tallied {
    /// `passed`, `failed`, `undecided`, or empty while it waits for the rest.
    pub came: &'static str,
    /// Who decided it, and what each of them said.
    pub by: String,
    pub words: String,
    /// How many chose the one, and how many the other, so far.
    pub approved: i64,
    pub against: i64,
}

/// Every word [`tally`] can answer: the last is that it waits.
pub const TALLY_WORDS: &[&str] = &["passed", "failed", "undecided", ""];

/// How a review stands with these votes cast, of `seats` that look and
/// `needed` to approve (none said: more than half).
///
/// It has passed when enough approve; failed when enough no longer can
/// and someone said what is wrong; and is undecided when all have spoken
/// and neither is so. Until then it waits for the rest, and has come to
/// nothing.
pub fn tally(asked: &Counted<'_>) -> Result<Tallied, String> {
    let (votes, seats, yes_word, no_word) = (asked.votes, asked.seats, asked.yes, asked.no);
    if seats < 1 {
        return Err(format!("`seats` is how many look at the card, and is at least one; it is {seats}"));
    }
    if yes_word.is_empty() || no_word.is_empty() || yes_word == no_word {
        return Err(format!(
            "what is counted for the work and what against it are two words; they are `{yes_word}` and \
             `{no_word}`"
        ));
    }
    let needed = if asked.needed > 0 { asked.needed } else { seats / 2 + 1 };
    let chose = |decision: &str| votes.iter().filter(|vote| vote.decision == decision).collect::<Vec<_>>();
    let (for_it, against_it) = (chose(yes_word), chose(no_word));
    let (yes, no) = (for_it.len() as i64, against_it.len() as i64);
    let cast = votes.len() as i64;
    let came = if yes >= needed {
        "passed"
    } else if yes + seats - cast >= needed {
        ""
    } else if no > 0 {
        "failed"
    } else if cast >= seats {
        "undecided"
    } else {
        ""
    };
    // Who decided it and what each of them said, as `<agent>: <words>`;
    // where one looked alone, its words as they are.
    let named = |votes: &[&Vote]| votes.iter().map(|vote| vote.by.as_str()).collect::<Vec<_>>().join(", ");
    let said = |votes: &[&Vote], between: &str| {
        let lines: Vec<String> = votes.iter().map(|vote| format!("{}: {}", vote.by, vote.words)).collect();
        lines.join(between)
    };
    let (by, words) = match came {
        "passed" => {
            let alone = votes.last().filter(|_| seats == 1).map(|vote| vote.words.clone());
            (named(&for_it), alone.unwrap_or_else(|| said(&for_it, "\n")))
        }
        "failed" => (named(&against_it), said(&against_it, "\n\n")),
        "undecided" => {
            let neither: Vec<&Vote> =
                votes.iter().filter(|vote| vote.decision != yes_word && vote.decision != no_word).collect();
            (String::new(), said(&neither, "\n"))
        }
        _ => (String::new(), String::new()),
    };
    Ok(Tallied { came, by, words, approved: yes, against: no })
}

fn text(args: &kernel::Fields, key: &str) -> Option<String> {
    args.get(key).and_then(Value::as_str).map(str::to_string).filter(|text| !text.is_empty())
}

/// Adds what is asked here to what a blueprint can do.
pub fn register(registry: &mut Registry) {
    add(
        registry,
        ActionSpec::new(
            "review.tally",
            "Count one more vote on a card and say whether that settles it: passed, failed, undecided, or not yet.",
        )
        .deciding()
        .required("votes", T::List, "the votes cast before this one, as this action last answered them")
        .required("by", T::String, "who has just looked")
        .required("decision", T::String, "what they chose: the choice counted for it, the one counted against it, or anything else for neither")
        .arg("words", T::String, "what they said")
        .required("seats", T::Int, "how many look at the card")
        .arg("needed", T::Int, "how many have to approve; more than half, if 0 or not said")
        .arg("yes", T::String, "the choice that is counted for the work; `approve`, if not said")
        .arg("no", T::String, "the choice that is counted against it; `send_back`, if not said")
        .output("votes", "the votes with this one among them, to keep and hand back with the next")
        .output("came", "`passed`, `failed`, `undecided`, or empty while it waits for the rest")
        .output("by", "who decided it: those who approved, or those who sent it back")
        .output("words", "what they said, each as `<agent>: <words>`")
        .output("approved", "how many approve so far")
        .output("against", "how many sent it back so far")
        .idempotent(),
        |_, args| async move {
            let mut votes = args.get("votes").and_then(Value::as_array).cloned().unwrap_or_default();
            let said = |key: &str| text(&args, key).unwrap_or_default();
            votes.push(json!({ "by": said("by"), "decision": said("decision"), "words": said("words") }));
            let of = |vote: &'_ Value, key: &str| vote[key].as_str().unwrap_or_default().to_string();
            let cast: Vec<Vote> = votes
                .iter()
                .map(|vote| Vote { by: of(vote, "by"), decision: of(vote, "decision"), words: of(vote, "words") })
                .collect();
            let (yes, no) = (text(&args, "yes"), text(&args, "no"));
            let counted = Counted {
                votes: &cast,
                seats: args.get("seats").and_then(Value::as_i64).unwrap_or(1),
                needed: args.get("needed").and_then(Value::as_i64).unwrap_or(0),
                yes: yes.as_deref().unwrap_or(YES),
                no: no.as_deref().unwrap_or(NO),
            };
            let came = tally(&counted).map_err(ActionError::new)?;
            Ok(json!({
                "came": came.came, "by": came.by, "words": came.words, "approved": came.approved,
                "against": came.against, "votes": votes,
            }))
        },
    );
}

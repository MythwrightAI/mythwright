//! What becomes of an item when those who looked at it have said, or
//! when the things done with it before anybody looks came to an end by
//! themselves: one answer for every way a text that has items reviewed
//! hears of it, so that the text does one thing for each word and says
//! each thing once.
//!
//! A text heard three verdicts in three handlers, each with its own
//! steps for the same few things (the item counted, said, moved, left).
//! Here they are one question: which of them was heard (`topic`), what
//! was said with it, and what the text keeps of the item; and the answer
//! is a word (`came`) with what goes with it. An item that passed and has
//! more to be done with it where it is (what was done put with the rest,
//! whatever a table's rows lay after the looking) comes to `after`, and
//! whoever asks goes through that; anything else is the word to act on.
//! [`decided`] is the function, with a table of cases that reaches every
//! word it can answer.
//!
//! That a looking ended having said nothing of how it came out is heard
//! here too (`ended`, which whoever asks says itself on hearing that the
//! looking is over), and not apart: what a looking says is said before it
//! ends, so heard by the one hearer in the order it was said, whatever it
//! said has been taken in by the time its ending is, and an ending is
//! never taken for one that said nothing because it was heard first.

use blueprint::{ActionSpec, ValueType as T};
use conductor::{ActionError, Registry};
use serde_json::{Value, json};

use crate::add;
use crate::reviews::{Verdict, VerdictCame, of, verdict};

/// Every word [`decided`] can answer.
pub const DECIDED_WORDS: &[&str] = &["after", "on", "back", "left", "waits", "nothing"];

/// What [`decided`] is asked.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Decided {
    /// What was heard: that it `passed`, `failed` or was left `undecided`
    /// (as a topic, `review.passed`, or the word alone); `came`, the
    /// things done before anybody looks having ended by themselves; or
    /// `ended`, a looking being over, said by whoever asks; or `due`, a
    /// looking asking for another in its place.
    pub topic: String,
    pub item: String,
    pub found: bool,
    pub status: String,
    pub here: String,
    pub pass: String,
    pub fail: String,
    /// What was done is put with the rest before an item goes on.
    pub merge: bool,
    /// Whoever asks has a list of things done with an item here, beyond
    /// what is usual; unless what it kept of the item's looking says
    /// that only what is usual comes after it (`usual` of the entry).
    pub lists: bool,
    pub reports: bool,
    pub by: String,
    pub words: String,
    pub approved: i64,
    pub against: i64,
    pub seats: i64,
    /// For `came`: the word the things done came to, and what was said.
    pub came: String,
    pub says: String,
    /// For `ended`: what the looking that is over was called.
    pub review: String,
    /// The items that have a looking under way, as whoever asks keeps
    /// them, each with where in the list its looking is (`row`, `order`).
    pub kept: Vec<Value>,
    /// Where a looking usually is in the list, for one kept without it.
    pub usual: i64,
    /// The items sent back from here, once for each time, as whoever
    /// asks keeps them.
    pub back: Vec<Value>,
}

/// What it comes to.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Came {
    pub word: &'static str,
    /// What is said as the item is moved, or the sentence where it is left.
    pub says: String,
    /// What is said where the work is reported; empty where nothing is.
    pub told: String,
    /// Which verdict this was, for whatever counts them; none for `came`.
    pub verdict: String,
    /// It was approved, and that is said of the item.
    pub approves: bool,
    /// For `after`: the list is gone through from where the looking was;
    /// or, there being no list, what was done is put with the rest.
    pub lists: bool,
    pub merges: bool,
    /// For `left`: the item is kept as one that was left, so that it is
    /// not begun for again until it is put here again.
    pub keeps: bool,
    /// Where the looking was in the list.
    pub row: String,
    pub order: i64,
    /// The item was sent back from here before, which whoever asks
    /// forgets once it goes on.
    pub sent: bool,
}

/// What an item comes to, by what was heard of it.
pub fn decided(asked: &Decided) -> Result<Came, String> {
    let which = asked.topic.rsplit('.').next().unwrap_or_default();
    let kept = asked.kept.iter().find(|entry| of(entry, &asked.item));
    let row = kept.and_then(|entry| entry["row"].as_str()).unwrap_or_default().to_string();
    let order =
        kept.and_then(|entry| entry["order"].as_i64()).filter(|order| *order != 0).unwrap_or(asked.usual);
    let lists = asked.lists && kept.is_none_or(|entry| entry["usual"] != true);
    let sent = asked.back.iter().any(|entry| of(entry, &asked.item));
    let came = Came { row, order, sent, ..Came::default() };
    let heard = |verdict: &str| Verdict {
        verdict: verdict.to_string(),
        item: asked.item.clone(),
        status: if asked.found { asked.status.clone() } else { String::new() },
        here: asked.here.clone(),
        pass: asked.pass.clone(),
        fail: asked.fail.clone(),
        merge: false,
        by: asked.by.clone(),
        words: asked.words.clone(),
        approved: asked.approved,
        against: asked.against,
        seats: asked.seats,
        reports: asked.reports,
        ..Verdict::default()
    };
    match which {
        "passed" => {
            let (first, told) = verdict(&heard("passed"))?;
            let came = Came { told, verdict: "passed".into(), approves: true, ..came };
            Ok(match first {
                VerdictCame::On(says) if lists => Came { word: "after", says, lists: true, ..came },
                VerdictCame::On(says) if asked.merge => Came { word: "after", says, merges: true, ..came },
                VerdictCame::On(says) => Came { word: "on", says, ..came },
                _ => Came { word: "nothing", ..came },
            })
        }
        "failed" => {
            let (first, told) = verdict(&heard("failed"))?;
            let came = Came { told, verdict: "failed".into(), ..came };
            Ok(match first {
                VerdictCame::Back(says) => Came { word: "back", says, ..came },
                _ => Came { word: "nothing", ..came },
            })
        }
        "undecided" => {
            let (first, _) = verdict(&heard("undecided"))?;
            Ok(Came { word: "left", says: first.words().to_string(), verdict: "undecided".into(), ..came })
        }
        "came" => {
            let says = asked.says.clone();
            match asked.came.as_str() {
                "on" if asked.pass.is_empty() => Ok(Came { word: "nothing", ..came }),
                "on" => Ok(Came { word: "on", says, ..came }),
                "back" => Ok(Came { word: "back", says, ..came }),
                "left" => Ok(Came { word: "left", says, keeps: true, ..came }),
                "" | "nothing" => Ok(Came { word: "nothing", ..came }),
                other => Err(format!(
                    "what was done with {} before anybody looked came to `{other}`, which is none of `on`, `back`, \
                     `left` or `nothing`",
                    asked.item
                )),
            }
        }
        // A looking that asks for another in its place: it is no longer
        // the one under way, which whoever asks takes off by its name,
        // before its ending is heard.
        "due" => Ok(Came { word: "nothing", ..came }),
        // Kept still under its name, it said nothing of how it came out:
        // what it said would have been heard first, and taken it off.
        "ended" => {
            let status = asked.status.as_str();
            Ok(match crate::reviews::over(&asked.kept, &asked.review, asked.found, status, &asked.here) {
                (_, Some(says)) => Came { word: "waits", says, ..came },
                _ => Came { word: "nothing", ..came },
            })
        }
        other => Err(format!(
            "`topic` is what was heard of {}: `review.passed`, `review.failed`, `review.undecided`, \
             `review.came`, `review.ended` or `review.due`; it is `{other}`",
            asked.item
        )),
    }
}

/// The word a thing done with an item ends with, for what putting what
/// was done with the rest came to: nothing yet, while it is still to be
/// asked; `left`, where something is in the way that a person puts right.
pub fn word(came: &VerdictCame) -> &'static str {
    match came {
        VerdictCame::Merge => "",
        VerdictCame::On(_) => "on",
        VerdictCame::Back(_) => "back",
        VerdictCame::Stuck(_) | VerdictCame::Left(_) => "left",
        VerdictCame::Nothing => "nothing",
    }
}

/// Adds what is asked here to what a blueprint can do.
pub fn register(registry: &mut Registry) {
    add(
        registry,
        ActionSpec::new(
            "review.decided",
            "Say what becomes of an item when those who looked at it have said, when the things done with it \
             before anybody looks ended by themselves, or when its looking ended having said nothing: more is \
             done with it where it is, it goes on, it goes back, it is left for a person, it waits for a person \
             to look at it, or nothing; with what is said of it and what is counted.",
        )
        .deciding()
        .required("topic", T::String, "what was heard: `review.passed`, `review.failed`, `review.undecided`, `review.came`, `review.ended`, or `review.due` from a looking that asks for another in its place")
        .required("item", T::String, "the item")
        .required("found", T::Bool, "whether it was found")
        .arg("status", T::String, "the column it is in")
        .required("here", T::String, "the column whose items are looked at")
        .arg("pass", T::String, "where an item goes on to")
        .arg("fail", T::String, "where an item is sent back to")
        .arg("merge", T::Bool, "what was done is put with the rest before the item goes on")
        .arg("lists", T::Bool, "whoever asks has a list of things done with an item here")
        .arg("reports", T::Bool, "what reviews come to is said where the work is reported")
        .arg("by", T::String, "who decided it")
        .arg("words", T::String, "what they said")
        .arg("approved", T::Int, "how many approved")
        .arg("against", T::Int, "how many sent it back")
        .arg("seats", T::Int, "how many looked")
        .arg("came", T::String, "for `review.came`: the word the things done came to")
        .arg("says", T::String, "and what was said with it")
        .arg("review", T::String, "for `review.ended` and `review.due`: what the looking that is over, or asks for another in its place, was called")
        .arg("kept", T::List, "the items that have a looking under way, as whoever asks keeps them")
        .arg("usual", T::Int, "where a looking usually is in the list, for one kept without it")
        .arg("back", T::List, "the items sent back from here, once for each time, as whoever asks keeps them")
        .arg("said", T::Object, "what was heard, whole, in place of saying each thing of it: `by`, `words`, `approved`, `against`, `seats`, `came`, `says`, `review`")
        .arg("own", T::Object, "what whoever asks says of itself, in place of saying each thing of it: `pass`, `fail`, `merge`, and `reports`, which is so where it says `verdicts`")
        .arg("title", T::String, "what the item is called, for what is done with it once it is approved")
        .arg("asking", T::Any, "a word that is this hearing's and no other's, for the same")
        .output("came", "`after`, `on`, `back`, `left`, `waits` or `nothing`")
        .output("says", "what is said as the item is moved, or the sentence where it is left or waits")
        .output("told", "what is said of it where the work is reported; empty where nothing is")
        .output("verdict", "which verdict it was, for whatever counts them; empty where it was none")
        .output("counts", "it was a verdict, and is counted")
        .output("approves", "it was approved, which is said of the item")
        .output("lists", "for `after`: the list is gone through from where the looking was")
        .output("merges", "for `after`: there being no list, what was done is put with the rest")
        .output("keeps", "for `left`: the item is kept as one that was left")
        .output("row", "where the looking was in the list: the row")
        .output("order", "and the order")
        .output("sent", "the item was sent back from here before")
        .output("journey", "for `after`: what is so of the item once it is approved, as a list gone through is handed it")
        .output("with", "for `after`: the same, as what puts approved work with the rest takes it by itself")
        .idempotent(),
        move |_, args| async move {
            // What was heard, handed whole, says each of these where
            // nothing else does.
            let args = crate::begun::with(&args, "own", &[("pass", "pass"), ("fail", "fail"), ("merge", "merge")]);
            let verdicts = args.get("own").is_some_and(|own| own["reports"] == "verdicts");
            let heard = args.get("said").and_then(Value::as_object);
            let said = |key: &str| args.get(key).or_else(|| heard.and_then(|heard| heard.get(key)));
            let text = |key: &str| said(key).and_then(Value::as_str).unwrap_or_default().to_string();
            let flag = |key: &str| args.get(key).and_then(Value::as_bool).unwrap_or(false);
            let count = |key: &str| said(key).and_then(Value::as_i64).unwrap_or(0);
            let asked = Decided {
                topic: text("topic"),
                item: text("item"),
                found: flag("found"),
                status: text("status"),
                here: text("here"),
                pass: text("pass"),
                fail: text("fail"),
                merge: flag("merge"),
                lists: flag("lists"),
                reports: args.get("reports").and_then(Value::as_bool).unwrap_or(verdicts),
                by: text("by"),
                words: text("words"),
                approved: count("approved"),
                against: count("against"),
                seats: said("seats").and_then(Value::as_i64).unwrap_or(1),
                came: text("came"),
                says: text("says"),
                review: text("review"),
                kept: args.get("kept").and_then(Value::as_array).cloned().unwrap_or_default(),
                usual: count("usual"),
                back: args.get("back").and_then(Value::as_array).cloned().unwrap_or_default(),
            };
            let came = decided(&asked).map_err(ActionError::new)?;
            let with = json!({
                "here": asked.here, "pass": asked.pass, "fail": asked.fail, "by": asked.by, "words": asked.words,
                "approved": asked.approved, "against": asked.against, "looked": asked.seats,
            });
            let mut journey = with.clone();
            journey["title"] = Value::from(text("title"));
            journey["asking"] = args.get("asking").cloned().unwrap_or(Value::Null);
            (journey["from"], journey["to"]) = (Value::from(asked.here.clone()), Value::from(asked.pass.clone()));
            journey.as_object_mut().expect("a mapping").remove("pass");
            Ok(json!({
                "journey": journey, "with": with,
                "came": came.word, "says": came.says, "told": came.told, "verdict": came.verdict,
                "counts": !came.verdict.is_empty(), "approves": came.approves, "lists": came.lists,
                "merges": came.merges, "keeps": came.keeps, "row": came.row, "order": came.order, "sent": came.sent,
            }))
        },
    );
}

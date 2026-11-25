//! What a council that talks it over came to in a stretch.
//!
//! The seats talk in a thread a stretch at a time, each turn ending with
//! the letter of the entry the seat would take, or with going on. After
//! each stretch, what each seat last said is looked at: where enough of
//! them say the same letter (`needed`, every seat where none is said) that
//! entry wins; where they do not and the council may still talk (fewer
//! turns than its most), it talks another stretch; and where it may not,
//! nothing was decided, which the contest's settings say what comes to.
//! A seat that ended with no letter, or with one no entry has, agrees with
//! nobody.

use std::collections::BTreeMap;

use blueprint::{ActionSpec, ValueType as T};
use conductor::{ActionError, Registry};
use serde_json::{Value, json};

use super::{Known, count, known, text};
use crate::add;

/// What the stretch came to.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Talked {
    /// `won`, `again` (another stretch) or `undecided`.
    pub came: &'static str,
    pub winner: String,
    /// How many seats said each letter, as `<letter> <count>`.
    pub said: String,
    pub why: String,
}

/// Every word [`talked`] can answer.
pub const TALKED_WORDS: &[&str] = &["won", "again", "undecided"];

/// What a stretch in which the seats last said `last` (a word a seat)
/// comes to, of `entries`, `needed` having to agree, after `turns` turns
/// of at most `most`.
pub fn talked(
    entries: &[Known],
    last: &[String],
    needed: i64,
    turns: i64,
    most: i64,
) -> Result<Talked, String> {
    if entries.is_empty() || last.is_empty() {
        return Err("a council's stretch is looked at for the entries judged and what each seat said, and one of them is not handed".into());
    }
    if needed < 0 || most < 1 {
        return Err(format!(
            "`needed` is how many seats have to agree (none said: every one) and `most` how many turns the \
             council may take, at least one; they are {needed} and {most}"
        ));
    }
    let needed = if needed == 0 { last.len() as i64 } else { needed };
    let mut by: BTreeMap<usize, i64> = BTreeMap::new();
    for word in last {
        if let Some(at) = entries.iter().position(|entry| entry.letter == word.trim()) {
            *by.entry(at).or_default() += 1;
        }
    }
    let said =
        by.iter().map(|(at, many)| format!("{} {many}", entries[*at].letter)).collect::<Vec<_>>().join(", ");
    let agreed: Vec<usize> = by.iter().filter(|(_, many)| **many >= needed).map(|(at, _)| *at).collect();
    if let [won] = agreed[..] {
        return Ok(Talked { came: "won", winner: entries[won].way.clone(), said, why: String::new() });
    }
    if turns < most {
        return Ok(Talked { came: "again", said, ..Default::default() });
    }
    let split = if said.is_empty() { "nobody said a letter".to_string() } else { said.clone() };
    Ok(Talked {
        came: "undecided",
        winner: String::new(),
        why: format!(
            "after {turns} turns, the most it may take, {needed} had not agreed on one entry ({split})"
        ),
        said,
    })
}

/// What one seat said in a stretch: who, the letter it would take (or
/// nothing yet), and its words.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Said {
    pub by: String,
    pub pick: String,
    pub words: String,
}

/// What the council has said so far, with this stretch's after it: what
/// each seat is shown of the others as it talks again. Each stretch under
/// a heading, each seat a line with the letter it would take.
pub fn heard(so_far: &str, stretch: i64, said: &[Said]) -> String {
    let lines: Vec<String> = said
        .iter()
        .map(|one| {
            let takes = match one.pick.trim() {
                "" => "would take none yet".to_string(),
                letter => format!("would take {letter}"),
            };
            format!("- {} ({takes}): {}", one.by, one.words.trim())
        })
        .collect();
    let this = format!("Stretch {stretch}:\n{}", lines.join("\n"));
    match so_far.trim() {
        "" => this,
        before => format!("{before}\n\n{this}"),
    }
}

/// Adds `contest.talked` to what a blueprint can do.
pub fn register(registry: &mut Registry) {
    add(
        registry,
        ActionSpec::new(
            "contest.talked",
            "Say what a council that talks it over came to after a stretch: the entry enough of its seats agreed on, another stretch, or nothing.",
        )
        .deciding()
        .required("entries", T::List, "the entries judged, each its `way` and the `letter` it is known by")
        .required("last", T::List, "what each seat last said: a letter, or anything else for none; or each seat's turn as `by`, `pick` and `words`")
        .arg("so_far", T::String, "what the council said before this stretch, as this gave it then")
        .arg("needed", T::Int, "how many seats have to agree on one entry; every one, if 0 or not said")
        .required("turns", T::Int, "how many turns the council has taken")
        .required("most", T::Int, "how many turns it may take")
        .output("came", "`won`, `again` (another stretch) or `undecided`")
        .output("winner", "the way that won")
        .output("said", "how many seats said each letter")
        .output("why", "why nothing was decided, where it was not")
        .output("next", "which stretch is next, where it is `again`")
        .output("transcript", "what the council has said so far, this stretch with it: what each seat is shown as it talks again")
        .output("verdicts", "each entry's `way`, its `verdict` (`won`, `lost` or `undecided`), and what the seats that would take it said (`words`)")
        .idempotent(),
        |_, args| async move {
            let entries: Vec<Known> = known(&args, "entries");
            let seats: Vec<Said> = args
                .get("last")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .map(|one| match one {
                    Value::String(word) => Said { pick: word.clone(), ..Default::default() },
                    other => Said {
                        by: text(other, "by"),
                        pick: match text(other, "pick") {
                            pick if pick.is_empty() => text(other, "word"),
                            pick => pick,
                        },
                        words: text(other, "words"),
                    },
                })
                .collect();
            let last: Vec<String> = seats.iter().map(|one| one.pick.clone()).collect();
            let turns = count(&args, "turns");
            let came = talked(&entries, &last, count(&args, "needed"), turns, count(&args, "most"))
                .map_err(ActionError::new)?;
            let transcript = heard(&super::said(&args, "so_far"), turns, &seats);
            let verdicts: Vec<Value> = entries
                .iter()
                .map(|entry| {
                    let verdict = match (came.came, entry.way == came.winner) {
                        ("won", true) => "won",
                        ("won", false) => "lost",
                        _ => "undecided",
                    };
                    let words: Vec<String> = seats
                        .iter()
                        .filter(|one| one.pick.trim() == entry.letter)
                        .map(|one| format!("{}: {}", one.by, one.words.trim()))
                        .collect();
                    json!({ "way": entry.way, "verdict": verdict, "words": words.join("\n") })
                })
                .collect();
            Ok(json!({
                "came": came.came, "winner": came.winner, "said": came.said, "why": came.why, "next": turns + 1,
                "transcript": transcript, "verdicts": verdicts,
            }))
        },
    );
}

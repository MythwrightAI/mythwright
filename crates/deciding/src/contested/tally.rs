//! What judges who each pick an entry come to.
//!
//! Each judge of a voting council looks for itself and picks the letter
//! of the entry it takes (or nothing, where it could not). What they come
//! to is a count of those: the entry enough of them picked, or, with
//! nobody saying how many are enough, the one picked most; nothing where
//! two are picked as often, where too few agree, or where nobody picked
//! an entry there is.

use std::collections::BTreeMap;

use blueprint::{ActionSpec, ValueType as T};
use conductor::{ActionError, Registry};
use serde_json::{Value, json};

use super::{Known, count, known, text};
use crate::add;

/// What one judge said: who, the letter picked (empty for none), and its words.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Pick {
    pub by: String,
    pub pick: String,
    pub words: String,
}

/// What the picks came to.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Tallied {
    /// `won` or `undecided`.
    pub came: &'static str,
    /// The way that won, where one did.
    pub winner: String,
    /// For each entry, as handed: its verdict (`won`, `lost`, `undecided`)
    /// and who picked it.
    pub verdicts: Vec<(&'static str, Vec<String>)>,
    /// Why nothing was decided, where it was not.
    pub why: String,
    /// The picks that named no entry, as `<judge>: <pick>`.
    pub astray: Vec<String>,
}

/// Every word [`tally`] can answer.
pub const TALLY_WORDS: &[&str] = &["won", "undecided"];

/// What `picks` of `entries` come to, `needed` of them having to agree on
/// one (none said: the one picked most, alone).
pub fn tally(entries: &[Known], picks: &[Pick], needed: i64) -> Result<Tallied, String> {
    if entries.is_empty() {
        return Err("the picks are counted for the entries judged, and none is handed".into());
    }
    if needed < 0 {
        return Err(format!(
            "`needed` is how many judges have to agree, and is not less than none; it is {needed}"
        ));
    }
    let mut by: BTreeMap<usize, Vec<String>> = BTreeMap::new();
    let mut astray = Vec::new();
    for pick in picks {
        match entries.iter().position(|entry| entry.letter == pick.pick) {
            Some(at) => by.entry(at).or_default().push(pick.by.clone()),
            None if pick.pick.is_empty() => {}
            None => astray.push(format!("{}: {}", pick.by, pick.pick)),
        }
    }
    let most = by.values().map(Vec::len).max().unwrap_or(0);
    let tops: Vec<usize> = by.iter().filter(|(_, who)| who.len() == most).map(|(at, _)| *at).collect();
    let picked = |at: usize| by.get(&at).cloned().unwrap_or_default();
    let undecided = |why: String| Tallied {
        came: "undecided",
        winner: String::new(),
        verdicts: (0..entries.len()).map(|at| ("undecided", picked(at))).collect(),
        why,
        astray: astray.clone(),
    };
    if most == 0 {
        return Ok(undecided(format!("none of the {} judges picked an entry there is", picks.len())));
    }
    let said = || {
        let each: Vec<String> =
            by.iter().map(|(at, who)| format!("{} {}", entries[*at].letter, who.len())).collect();
        each.join(", ")
    };
    if tops.len() > 1 {
        return Ok(undecided(format!("as many picked one entry as another ({})", said())));
    }
    if needed > 0 && (most as i64) < needed {
        return Ok(undecided(format!(
            "{needed} have to agree on one entry, and at most {most} did ({})",
            said()
        )));
    }
    let won = tops[0];
    Ok(Tallied {
        came: "won",
        winner: entries[won].way.clone(),
        verdicts: (0..entries.len()).map(|at| (if at == won { "won" } else { "lost" }, picked(at))).collect(),
        why: String::new(),
        astray,
    })
}

/// Adds `contest.tally` to what a blueprint can do.
pub fn register(registry: &mut Registry) {
    add(
        registry,
        ActionSpec::new(
            "contest.tally",
            "Say what judges who each picked an entry by its letter come to: the entry that won, or that nothing was decided and why.",
        )
        .deciding()
        .required("entries", T::List, "the entries judged, each its `way` and the `letter` it is picked by")
        .required("picks", T::List, "what each judge said: `by`, `pick` (a letter, or empty for none) and its `words`")
        .arg("needed", T::Int, "how many have to agree on one entry; the one picked most, alone, if 0 or not said")
        .output("came", "`won` or `undecided`")
        .output("winner", "the way that won")
        .output("verdicts", "each entry's `way`, its `verdict` (`won`, `lost` or `undecided`), who picked it (`by`) and what they said (`words`)")
        .output("why", "why nothing was decided, where it was not")
        .output("astray", "the picks that named no entry")
        .idempotent(),
        |_, args| async move {
            let entries: Vec<Known> = known(&args, "entries");
            let said = args.get("picks").and_then(Value::as_array).cloned().unwrap_or_default();
            let picks: Vec<Pick> = said
                .iter()
                .map(|one| Pick { by: text(one, "by"), pick: text(one, "pick").trim().to_string(), words: text(one, "words") })
                .collect();
            let came = tally(&entries, &picks, count(&args, "needed")).map_err(ActionError::new)?;
            let verdicts: Vec<Value> = entries
                .iter()
                .zip(&came.verdicts)
                .map(|(entry, (verdict, by))| {
                    let words: Vec<String> = picks
                        .iter()
                        .filter(|pick| by.contains(&pick.by))
                        .map(|pick| format!("{}: {}", pick.by, pick.words))
                        .collect();
                    json!({ "way": entry.way, "verdict": verdict, "by": by, "words": words.join("\n") })
                })
                .collect();
            Ok(json!({
                "came": came.came, "winner": came.winner, "verdicts": verdicts, "why": came.why, "astray": came.astray,
            }))
        },
    );
}

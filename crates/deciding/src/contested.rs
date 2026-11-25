//! What a contest comes to: one card worked several ways at once and
//! judged.
//!
//! A contest is a row of a table beside a board, about a card: which
//! contenders work it (each an agent, a role or a way of a comparison),
//! how they are judged and what becomes of the work that did not win.
//! The texts that run one (`std/contest` and what it calls) say what is
//! done and ask here what is decided, a block for each question:
//!
//! - which of a card's contests is to be done now ([`pending`]);
//! - what the contenders come to as ways of a comparison, the letters a
//!   judge knows them by, the places they work in and what is wrong with
//!   a setup ([`ways`]);
//! - what judges who each pick an entry come to ([`tally`]);
//! - what judges who each score every entry come to, by the contest's
//!   rule ([`scores`]);
//! - what a council that talks it over came to in a stretch ([`talked`]);
//! - and what becomes of each entry and of the contest ([`outcome`]),
//!   with what a person asked is told of each entry ([`described`]).
//!
//! Each is a function of plain values with its table of cases, for any
//! number of contenders. Nothing here scores anybody: a judge's pick or
//! score is kept as it was said, and a winner is only had from scores by
//! the rule the contest's owner chose, a tie always being left for a
//! person.

pub mod described;
pub mod outcome;
pub mod pending;
pub mod scores;
pub mod shown;
pub mod talked;
pub mod tally;
pub mod ways;

use conductor::Registry;
use serde_json::Value;

/// What an entry is known by where it is judged: its way, and the letter
/// a judge picks it by.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Known {
    pub way: String,
    pub letter: String,
}

/// What a judge is shown an entry as, the `at`th of all (from none): `A`
/// to `Z`, then `AA`, `AB` and on, as a spreadsheet's columns are called,
/// so that any number of entries has one each.
pub fn letter(at: usize) -> String {
    let mut at = at + 1;
    let mut said = Vec::new();
    while at > 0 {
        let rest = (at - 1) % 26;
        said.push(b'A' + rest as u8);
        at = (at - 1) / 26;
    }
    said.reverse();
    String::from_utf8(said).unwrap_or_default()
}

/// The entries handed to a block, as `ways` gave them or a row says them.
pub(crate) fn known(args: &kernel::Fields, key: &str) -> Vec<Known> {
    let each = args.get(key).and_then(Value::as_array).cloned().unwrap_or_default();
    each.iter().map(|one| Known { way: text(one, "way"), letter: text(one, "letter") }).collect()
}

pub(crate) fn text(of: &Value, key: &str) -> String {
    match of.get(key) {
        Some(Value::String(text)) => text.clone(),
        Some(Value::Number(number)) => number.to_string(),
        _ => String::new(),
    }
}

pub(crate) fn said(args: &kernel::Fields, key: &str) -> String {
    args.get(key).and_then(Value::as_str).unwrap_or_default().to_string()
}

pub(crate) fn count(args: &kernel::Fields, key: &str) -> i64 {
    args.get(key).and_then(Value::as_i64).unwrap_or(0)
}

/// Adds what is asked of a contest to what a blueprint can do.
pub fn register(registry: &mut Registry) {
    pending::register(registry);
    ways::register(registry);
    tally::register(registry);
    scores::register(registry);
    shown::register(registry);
    talked::register(registry);
    outcome::register(registry);
}

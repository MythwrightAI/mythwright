//! Where a word a choice offers stands among the others, as a number, and
//! what a number written before a field was a choice comes to.
//!
//! A pool that takes its askers by priority goes by a number each says
//! as it asks (`kernel::pool`), the higher first. A card's priority is a
//! word of a choice (`urgent`, `high`, `normal`, `low`), so the word comes
//! to a number here, by where it stands among what the choice offers as
//! they stand now: the first the highest. A table whose options somebody
//! put in another order (`of_chosen`) ranks by that order, with nothing
//! more done. Nothing chosen is the word the field says nothing stands for
//! (`unsaid`), where it says one, and otherwise below every word.
//!
//! The same field was a number before it was a choice, and a table that
//! ships is declared again over the rows it had: each number comes to the
//! word as far above the one nothing stands for as the number was above
//! nought, and as far below it for less than nought, the first or the
//! last where it goes past them (`word_of`). It was read the same way:
//! nothing was nought, and the higher went first.

use serde_json::Value;

/// Where `chosen` stands among `of`, as a number: the first of `of` is as
/// many as there are, and each after it one less, so the last is 1.
/// Nothing chosen, and a word `of` does not offer, stands where `unsaid`
/// does, or at 0 where there is no such word. A number is a value written
/// before the field was a choice, and stands where the word it comes to
/// does (`word_of`).
pub fn rank(chosen: &Value, of: &[String], unsaid: Option<&str>) -> i64 {
    let at = |word: &str| of.iter().position(|one| one == word).map(|place| (of.len() - place) as i64);
    let usual = unsaid.and_then(at).unwrap_or(0);
    match chosen {
        Value::String(word) => at(word).unwrap_or(usual),
        Value::Number(number) => {
            word_of(number.as_f64().unwrap_or(0.0), of, unsaid).and_then(|word| at(&word)).unwrap_or(usual)
        }
        _ => usual,
    }
}

/// The word of `of` a number written before the field was a choice comes
/// to: as many places above `unsaid` as the number is above nought (a
/// part of one being a place), and below it for less than nought, the
/// first or the last of `of` where it goes past them. Nothing where the
/// choice says nothing stands for no word: a number then has nowhere to be
/// counted from.
pub fn word_of(number: f64, of: &[String], unsaid: Option<&str>) -> Option<String> {
    let from = of.iter().position(|one| Some(one.as_str()) == unsaid)? as i64;
    let places = if number > 0.0 {
        number.ceil() as i64
    } else if number < 0.0 {
        number.floor() as i64
    } else {
        0
    };
    let at = from.saturating_sub(places).clamp(0, of.len() as i64 - 1);
    of.get(at as usize).cloned()
}

//! How many things at most are done for an item in one going through its
//! list, and what is said of one that has more to do than that.
//!
//! The list is gone through by a text that calls itself once for each
//! thing done (`std/through`), so nothing but this bounds how deep that
//! goes: whoever asks says how many at most (`most_done`, which a stage's
//! row says as `most`), and the asking that would call one more is
//! answered that the item is left for a person. What is begun and heard
//! from by whoever asks (a review) is not called here, and is not
//! counted.

/// What is said of an item that has more to do in one going through its
/// list than whoever goes through it does at once: what it is, what was
/// not begun, and the ways out, the last of them the command that raises
/// the number on the record that says it (`of`), where that was said.
pub fn beyond(item: &str, here: &str, title: &str, most: i64, of: &str) -> String {
    let things = if most == 1 { "1 thing".to_string() } else { format!("{most} things") };
    let raise = match of.trim() {
        "" => "or raise `most` on the row of `stages` that says how many are done".to_string(),
        of => format!("or raise how many are done: `{} record set {of} most={}`", brand::CLI, most + 1),
    };
    format!(
        "{item}: there is more to do with it in {here} than is gone through at once ({things} at most), and \
         {title} was not begun: it is left for a person. Take a row of `segments` away, put two of them in one \
         blueprint, {raise}"
    )
}

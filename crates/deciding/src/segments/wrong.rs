//! What is wrong with a list of things done as its rows say it.

use super::{Said, Usual};

/// The texts that go through such a list, each of which calls the things
/// in it: what ships for a stage's list (`std/segments`), for a card's
/// round (`std/lane-round`) and for the rest of either from its second
/// thing (`std/through`). Named as a thing done, one would go through
/// the list inside itself, a level of its tree deeper each time, until
/// the tree went as deep as one may. A round of a project's own, or one
/// kept from before under a name of its own, is a thing done like any.
pub const GOES_THROUGH: &[&str] = &["std/through", "std/segments", "std/lane-round"];

/// What is said of a row that names one of [`GOES_THROUGH`].
pub fn goes_through(title: &str, does: &str) -> String {
    format!(
        "The row {title} of `segments` names `{does}`, which is what goes through such a list itself: named \
         in it, it would go through the list inside itself, and it is not run. Name the thing to be done."
    )
}

/// What is wrong with a list as its rows say it, a sentence each: what a
/// row says that will never be done as written.
pub fn wrong(rows: &[Said], usual: &[Usual], here: &str, lists: bool) -> Vec<String> {
    let mut said = Vec::new();
    for row in rows {
        if !lists {
            said.push(format!(
                "The row {} of `segments` is for {here}, and what is done with a card in {here} is not a list \
                 that rows are laid into: it is not run. A stage that works, and one that reviews, run what \
                 their rows say.",
                row.title
            ));
            continue;
        }
        if GOES_THROUGH.contains(&row.does.as_str()) {
            said.push(goes_through(&row.title, &row.does));
            continue;
        }
        if row.does.is_empty() && !usual.iter().any(|usual| usual.order == row.order) {
            said.push(format!(
                "The row {} of `segments` names nothing to do (`does`), and nothing is usually done at {} in \
                 {here} for it to take out.",
                row.title, row.order
            ));
        }
    }
    said
}

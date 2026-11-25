//! What a comment on a selection of a document's text is about, as plain
//! values: the words it quotes, a few words either side of them, and the
//! headings the block they are in stands under.
//!
//! The document is a Markdown file, and other programs write it, so
//! nothing of a comment is kept in the file. What the comment was about
//! is kept with the comment, in the line of the record's history that it
//! is (`record.commented` carries it), and is found again by whoever
//! shows the document, by looking for the words in the text as it is
//! then (`ui/src/anchors.ts`). Words that are no longer there make the
//! comment one on the whole document that says what it was about: it is
//! never dropped.
//!
//! Each part is checked where it is written, and bounded: whoever says
//! something on a record writes what is kept for good in its history, and
//! a quote is a few words of a line, not a copy of the document.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{Error, Result};

/// The most characters quoted.
pub const MOST_QUOTED: usize = 400;
/// The most characters said either side of what is quoted.
pub const MOST_BESIDE: usize = 40;
/// The most headings a block is said to stand under.
pub const MOST_HEADINGS: usize = 6;
/// The most characters of one heading.
pub const MOST_HEADING: usize = 200;

/// The keys an anchor is written with, in the order they are described.
pub const KEYS: [&str; 4] = ["quote", "before", "after", "under"];

/// What is written inside an anchor, as an action says what is written
/// inside one of its arguments (`ActionSpec::within`).
pub const WITHIN: &[(&str, &str, &str)] = &[
    ("quote", "string", "the words commented on, as they read in the document, on one line"),
    (
        "before",
        "string",
        "a few words just before them in the same block, so that the same words said twice are told apart",
    ),
    ("after", "string", "a few words just after them in the same block"),
    (
        "under",
        "list",
        "the headings the block stands under, outermost first, so that words that moved are looked for there first",
    ),
];

/// What a comment on a selection of a document's text is about.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Anchor {
    /// The words commented on.
    pub quote: String,
    /// A few words just before them, in the same block; nothing at its start.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub before: String,
    /// A few words just after them, in the same block; nothing at its end.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub after: String,
    /// The headings the block stands under, outermost first.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub under: Vec<String>,
}

/// One part of an anchor as text: on one line, and no longer than it may be.
fn line(part: &str, value: &Value, most: usize) -> Result<String> {
    let Some(text) = value.as_str() else {
        return Err(Error::invalid(format!("`anchor.{part}` is words; it is {value}")));
    };
    if text.contains(['\n', '\r']) {
        return Err(Error::invalid(format!(
            "`anchor.{part}` is on one line; what a comment is about is a few words of one block, with what is \
             between lines said as a space"
        )));
    }
    let count = text.chars().count();
    if count > most {
        return Err(Error::invalid(format!(
            "`anchor.{part}` is {most} characters at most, and is {count}; a comment is about a few words, not a \
             copy of the document"
        )));
    }
    Ok(text.to_string())
}

/// Reads what a comment is about, as whoever says it writes it: nothing
/// where it is left out, which is a comment on the whole record.
/// Refused in a sentence where any part is not what it may be.
pub fn read(value: Option<&Value>) -> Result<Option<Anchor>> {
    let written = match value {
        None | Some(Value::Null) => return Ok(None),
        Some(Value::Object(written)) => written,
        Some(other) => {
            return Err(Error::invalid(format!(
                "`anchor` says which words of the document a comment is about: `quote`, with `before`, `after` \
                 and `under` beside it; it is {other}. Left out, the comment is on the whole document"
            )));
        }
    };
    if let Some(other) = written.keys().find(|key| !KEYS.contains(&key.as_str())) {
        return Err(Error::invalid(format!(
            "`anchor` takes `quote`, `before`, `after` and `under`, and not `{other}`"
        )));
    }
    let quote = line("quote", written.get("quote").unwrap_or(&Value::Null), MOST_QUOTED)?;
    if quote.trim().is_empty() {
        return Err(Error::invalid(
            "`anchor.quote` is the words a comment is about, and is something; a comment on the whole document \
             says no anchor",
        ));
    }
    let beside = |part: &str| match written.get(part) {
        None | Some(Value::Null) => Ok(String::new()),
        Some(value) => line(part, value, MOST_BESIDE),
    };
    let (before, after) = (beside("before")?, beside("after")?);
    let under = match written.get("under") {
        None | Some(Value::Null) => Vec::new(),
        Some(Value::Array(headings)) if headings.len() > MOST_HEADINGS => {
            return Err(Error::invalid(format!(
                "`anchor.under` is {MOST_HEADINGS} headings at most, the outermost first, and is {}",
                headings.len()
            )));
        }
        Some(Value::Array(headings)) => headings
            .iter()
            .map(|heading| {
                let heading = line("under", heading, MOST_HEADING)?;
                match heading.trim().is_empty() {
                    true => Err(Error::invalid("`anchor.under` is headings, and none of them is nothing")),
                    false => Ok(heading),
                }
            })
            .collect::<Result<_>>()?,
        Some(other) => {
            return Err(Error::invalid(format!(
                "`anchor.under` is the headings the block stands under, as a list, the outermost first; it is {other}"
            )));
        }
    };
    Ok(Some(Anchor { quote, before, after, under }))
}

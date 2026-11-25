//! What a thing done with an item gave out, read as the word it ended
//! with; and what a command that was run for one comes to. Functions of
//! what they are handed, each with its cases.

use serde_json::{Map, Value};

/// Every word [`ran`] can answer.
pub const RAN_WORDS: &[&str] = &["on", "back", "left"];

/// How much of what a command printed is said of it.
const TAIL: usize = 1200;

/// What a command that was run for an item comes to.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Ran {
    pub item: String,
    pub title: String,
    /// The command, as it was said.
    pub run: String,
    pub ok: bool,
    /// What becomes of the item where it did not go well: `back`, which
    /// is what nothing said means, or `left`.
    pub fails: String,
    pub printed: String,
    pub why: String,
    /// Why it was not run at all, where it was not: the item is left for
    /// a person, whatever its row says of failing.
    pub unrun: String,
}

/// The word a command's ending comes to, and what is said with it.
pub fn ran(asked: &Ran) -> Result<(&'static str, String), String> {
    let title = if asked.title.is_empty() { "what was run" } else { asked.title.as_str() };
    if asked.run.trim().is_empty() {
        let says = format!(
            "{}: {title} says no command to run (`command`), and it is left for a person",
            asked.item
        );
        return Ok(("left", says));
    }
    if !asked.unrun.trim().is_empty() {
        return Ok(("left", asked.unrun.trim().to_string()));
    }
    if asked.ok {
        return Ok(("on", String::new()));
    }
    let printed = asked.printed.trim();
    let from = printed.char_indices().rev().nth(TAIL).map_or(0, |(at, _)| at);
    let said = match (printed.is_empty(), asked.why.trim()) {
        (false, _) => printed[from..].trim().to_string(),
        (true, "") => "it printed nothing".to_string(),
        (true, why) => why.to_string(),
    };
    match asked.fails.as_str() {
        "" | "back" => Ok(("back", format!("`{}` did not pass: {said}", asked.run.trim()))),
        "left" => Ok((
            "left",
            format!(
                "{}: `{}` did not pass, and it is left for a person: {said}",
                asked.item,
                asked.run.trim()
            ),
        )),
        other => Err(format!(
            "{title} says `fails` is `{other}`; it is `back`, the item goes back with what was printed, or \
             `left`, it is left for a person"
        )),
    }
}

/// What a segment gives out that is not one of its words is handed on:
/// to whatever of the list comes after it and declares a param of that
/// name, and to whoever asks.
pub(super) const WORDS_GIVEN: &[&str] = &["came", "says", "why", "asker", "question"];

/// What a segment that ended gave out, as the words it ended with.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Gave {
    pub came: String,
    pub says: String,
    pub why: String,
    pub asker: String,
    pub question: String,
}

/// What a segment gave out, read as the word it ended with. One that
/// gives no word came to `on`. A whole round as it was before a round
/// was a list (it gives `available`, `asked` and `again`, and no word)
/// is read as what it is: it handed its item on itself, so nothing more
/// is done for the item; or its agent asked; or what was done could not
/// be kept and the item is to be given again, told why.
pub fn gave(out: &Map<String, Value>) -> Gave {
    let text = |key: &str| out.get(key).and_then(Value::as_str).unwrap_or_default().to_string();
    let flag = |key: &str| out.get(key).and_then(Value::as_bool).unwrap_or(false);
    let word = text("came");
    if !word.is_empty() || !out.contains_key("available") {
        let (says, why) = (text("says"), text("why"));
        // What is said as an item goes back is what it said, or why.
        let says = if word == "back" && says.is_empty() { why.clone() } else { says };
        return Gave { came: word, says, why, asker: text("asker"), question: text("question") };
    }
    let came = match (flag("asked"), flag("again")) {
        (true, _) => "asks",
        (false, true) => "back",
        (false, false) => "nothing",
    };
    Gave {
        came: came.into(),
        says: text("why"),
        why: text("why"),
        asker: text("asker"),
        question: text("question"),
    }
}

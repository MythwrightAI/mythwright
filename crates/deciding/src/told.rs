//! Whom something that waits for a person is told to, and in what words.
//!
//! A fleet tells of eight things: something asked a question (`asked`),
//! a card was left for a person (`left`), its agent could not do what it
//! needed and it was left for a person (`blocked`, which a row that tells
//! of `left` is told of too), a card waits for a person to review it
//! (`review`), a card waits for somebody to press what its column offers
//! (`held`), a plan waits to be accepted (`plan`), something of the
//! fleet failed (`failed`), and a command run for a card has run longer
//! than its row says it may before somebody is told (`slow`). Whom is rows
//! of a table: each says somebody (`whom`) and which of the things they
//! are told of (`of`; all of them, where a row says none). A row that
//! says nobody tells nobody, and so does a table with no row.
//!
//! [`whom`] is that: a function of the rows as they were read, which
//! thing happened, and what the event that said so carried. It gives
//! whom to tell, each once, and the sentence that says it, with what
//! the line is about. Who a word of `whom` names is not asked here:
//! whatever tells them (`inbox.tell`) says, and refuses one that names
//! nobody. A blueprint asks it as `tells.whom`, and then tells.

use blueprint::{ActionSpec, ValueType as T};
use conductor::{ActionError, Registry};
use serde_json::{Map, Value, json};

use crate::add;

blueprint::came! {
    /// What happened, that somebody may be told of.
    pub enum Of {
        /// Something asked a question, and waits for an answer.
        Asked => "asked",
        /// A card was left for a person.
        Left => "left",
        /// Its agent could not do what it needed, and it was left for a
        /// person: a card left that says so (`as: blocked`).
        Blocked => "blocked",
        /// A card waits for a person to review it: nobody is there to look,
        /// or too few.
        Review => "review",
        /// A card waits for somebody to press what its column offers.
        Held => "held",
        /// A plan waits to be accepted.
        Plan => "plan",
        /// Something of the fleet failed.
        Failed => "failed",
        /// A command run for a card has run longer than its row says it
        /// may before somebody is told, and goes on.
        Slow => "slow",
    }
}

/// How much of a command a sentence says: its first line, so long.
const COMMAND_SAID: usize = 120;

/// What is told, and to whom.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Telling {
    /// Which thing it is: why the line is there.
    pub of: Of,
    /// Whom, as each row says them, each once and in the rows' order.
    pub to: Vec<String>,
    /// The sentence.
    pub says: String,
    /// The id of what it is about, or nothing.
    pub about: String,
    /// Which telling it is, where that is so whichever run tells it: a
    /// command run again after a stop is told of once. Nothing otherwise.
    pub once: String,
    /// Of something asked: what is asked, in the words it was asked in,
    /// to be said where it was asked; and what is said where nobody is
    /// asked it, there (`unasked`) and where the fleet reports (`untold`).
    /// Nothing otherwise.
    pub asks: String,
    pub unasked: String,
    pub untold: String,
}

/// What is said where something was asked and no row names anybody to
/// ask it of: on what was asked about (`unasked`), and where the fleet
/// reports, naming it (`untold`).
fn asked_of_nobody(thing: &str, who: &str, project: &str) -> (String, String) {
    let project = if project.is_empty() { "<project>".to_string() } else { project.to_string() };
    let how = format!(
        "no row of the project's `tells` names anybody to tell of a question. Answer it in its Comments; and \
         say who is asked from now on with `{cli} record add {project} tells title=lead whom=@<somebody> of=asked`.",
        cli = brand::CLI
    );
    let asked = match who.is_empty() {
        false => format!("{thing}: {who} asked something, and nobody is asked it: {how}"),
        true => format!("{thing}: something was asked, and nobody is asked it: {how}"),
    };
    (format!("Nobody is asked this: {how}"), asked)
}

/// The first of `keys` that `said` says something under, as text.
fn word<'a>(said: &'a Map<String, Value>, keys: &[&str]) -> &'a str {
    keys.iter()
        .filter_map(|key| said.get(*key).and_then(Value::as_str))
        .map(str::trim)
        .find(|text| !text.is_empty())
        .unwrap_or_default()
}

/// Whether a row's `of` holds `thing`: it does where it says nothing. A
/// card left because its agent could not do what it needed is a card
/// left, and a row that tells of `left` is told of it.
fn tells_of(row: &Map<String, Value>, thing: Of) -> bool {
    let words: Vec<&str> = match row.get("of") {
        Some(Value::Array(words)) => words.iter().filter_map(Value::as_str).collect(),
        // One word, where a cell holds text and no list.
        Some(Value::String(one)) => one.split(',').collect(),
        _ => Vec::new(),
    };
    let mut words = words.into_iter().map(str::trim).filter(|word| !word.is_empty()).peekable();
    let also = match thing {
        Of::Blocked => Some(Of::Left.word()),
        _ => None,
    };
    words.peek().is_none()
        || words.any(|word| {
            word.eq_ignore_ascii_case(thing.word())
                || also.is_some_and(|also| word.eq_ignore_ascii_case(also))
        })
}

/// Whom `of` is told to and what is said: `rows` are the rows of who is
/// told what, as they were read; `said`, what the event that said so
/// carried; `from`, the instance that said it; `project`, whose it is.
pub fn whom(
    rows: &[Value],
    of: &str,
    said: &Map<String, Value>,
    from: &str,
    project: &str,
) -> Result<Telling, String> {
    let Some(of) = Of::ALL.iter().copied().find(|one| one.word() == of) else {
        return Err(format!("`of` is which thing somebody is told of, {}; it was given `{of}`", Of::words()));
    };
    // A card left that says why it was is told of as that.
    let of = match (of, word(said, &["as"])) {
        (Of::Left, "blocked") => Of::Blocked,
        (of, _) => of,
    };
    let mut to: Vec<String> = Vec::new();
    for row in rows {
        let Some(row) = row.as_object() else {
            return Err(format!(
                "`rows` is the rows that say who is told what, each a row; one of them is {row}"
            ));
        };
        let whom = word(row, &["whom"]);
        if !whom.is_empty() && tells_of(row, of) && !to.iter().any(|told| told == whom) {
            to.push(whom.to_string());
        }
    }
    // What it is of: a card, or a record of another table.
    let thing = word(said, &["card", "record"]);
    let who = word(said, &["asker", "agent"]);
    let (mut asks, mut unasked, mut untold) = (String::new(), String::new(), String::new());
    let (says, about) = match of {
        Of::Asked => {
            let question = word(said, &["question"]);
            asks = match question.is_empty() {
                false => question.to_string(),
                true => "Something was asked, and waits for an answer.".to_string(),
            };
            (unasked, untold) = asked_of_nobody(thing, who, project);
            let says = match (who.is_empty(), question.is_empty()) {
                (false, false) => format!("{thing}: {who} asks: {question}"),
                (true, false) => format!("{thing} asks: {question}"),
                (false, true) => format!("{thing}: {who} asked something, and waits for an answer."),
                (true, true) => format!("{thing}: something was asked, and waits for an answer."),
            };
            (says, thing)
        }
        Of::Left => {
            let why = word(said, &["why"]);
            let says = match (said.get("rounds").and_then(Value::as_i64), why.is_empty()) {
                (_, false) => format!("{thing} is left for a person: {why}"),
                (Some(1), _) => {
                    format!(
                        "{thing} was sent back 1 time, which is as often as it may be: it is left for a person."
                    )
                }
                (Some(rounds), _) if rounds > 1 => format!(
                    "{thing} was sent back {rounds} times, which is as often as it may be: it is left for a person."
                ),
                _ => format!("{thing} is left for a person."),
            };
            (says, thing)
        }
        Of::Blocked => {
            let says = match word(said, &["why"]) {
                "" => format!("{thing}: its agent could not do what it needed, and it is left for a person."),
                why => format!(
                    "{thing}: its agent could not do what it needed, and it is left for a person: {why}"
                ),
            };
            (says, thing)
        }
        Of::Review => {
            let says = match word(said, &["why"]) {
                "" => format!("{thing} waits for a person to review it."),
                why => why.to_string(),
            };
            (says, thing)
        }
        Of::Held => {
            let says = match word(said, &["says"]) {
                "" => format!("{thing} waits for somebody to press what its column offers."),
                says => says.to_string(),
            };
            (says, thing)
        }
        Of::Plan => {
            let accept = "Move it on to accept it, or say on it what should be otherwise.";
            let says = match who.is_empty() {
                false => {
                    format!("{thing}: {who} has said how it would be done, and the plan waits. {accept}")
                }
                true => format!("{thing} has a plan, which waits. {accept}"),
            };
            (says, thing)
        }
        Of::Failed => {
            let reason = word(said, &["reason", "error"]);
            let whose = if project.is_empty() { String::new() } else { format!(" for {project}") };
            let says = match reason.is_empty() {
                false => format!("Something in the fleet{whose} failed: {reason}"),
                true => format!("Something in the fleet{whose} failed."),
            };
            (says, from)
        }
        Of::Slow => (slow(thing, said), thing),
    };
    if matches!(of, Of::Asked | Of::Left | Of::Blocked | Of::Review | Of::Held | Of::Plan | Of::Slow)
        && thing.is_empty()
    {
        return Err(format!(
            "what was said names nothing that is `{}`: it says neither `card` nor `record`",
            of.word()
        ));
    }
    // A command's running past its time is one telling for the card's
    // coming (`asking`), whichever run says it: the command is begun
    // again where the daemon stopped while it ran.
    let asking = word(said, &["asking"]);
    let once = match (of, asking.is_empty()) {
        (Of::Slow, false) => format!("slow {thing} {asking} {}", word(said, &["command"])),
        _ => String::new(),
    };
    Ok(Telling { of, to, says, about: about.to_string(), once, asks, unasked, untold })
}

/// What is said of a command that has run longer than its row says it
/// may before somebody is told: which, how long so far, and that it goes
/// on.
fn slow(thing: &str, said: &Map<String, Value>) -> String {
    let command = word(said, &["command"]).lines().next().unwrap_or_default();
    let command = match command.char_indices().nth(COMMAND_SAID) {
        Some((cut, _)) => format!("{}…", &command[..cut]),
        None => command.to_string(),
    };
    let which = match command.is_empty() {
        true => "its command".to_string(),
        false => format!("`{command}`"),
    };
    let ran = said
        .get("running_ms")
        .and_then(Value::as_u64)
        .map(|ms| blueprint::duration::format(ms / 1000 * 1000));
    let past = match word(said, &["after"]) {
        "" => String::new(),
        after => format!(", past the {after} it may run before somebody is told"),
    };
    match ran {
        Some(ran) if ran != "0s" => {
            format!("{thing}: {which} has been running for {ran}{past}, and goes on.")
        }
        _ => format!("{thing}: {which} is still running{past}, and goes on."),
    }
}

/// Adds what is asked here to what a blueprint can do.
pub fn register(registry: &mut Registry) {
    add(
        registry,
        ActionSpec::new(
            "tells.whom",
            "Say whom something that waits for a person is told to, by the rows that say who is told what, and \
             the sentence that says it.",
        )
        .deciding()
        .required(
            "rows",
            T::List,
            "the rows that say who is told what, as they were read (`rows` of `rows.read`): each with `whom`, \
             and `of`, a list of which things, all of them where it holds none",
        )
        .required("of", T::String, &format!("which thing it is: {}", Of::words()))
        .arg(
            "said",
            T::Any,
            "what the event that said so carried: which `card` or `record`, who (`asker`, `agent`), the \
             `question`, how many `rounds`, `why`, `as` (a card left that says `blocked` is told of as that), \
             the `reason`, what a card held `says`; of a command run for a card, the `command`, how long it \
             may run (`after`), how long it has (`running_ms`) and which coming of the card it is (`asking`)",
        )
        .arg("from", T::String, "the instance that said it: what something that failed is about")
        .arg("project", T::String, "the project it happened in")
        .output("tells", "whether there is anybody to tell")
        .output("to", "whom, as each row says them, each once")
        .output("says", "the sentence")
        .output("about", "the id of what it is about: the card or the record, or for something that failed what failed")
        .output("why", &format!("{}: which thing it is, for why the line is there", Of::words()))
        .output("once", "which telling it is whichever run tells it, for `once` of `inbox.tell`; nothing otherwise")
        .output("asks", "of something asked: what is asked, in the words it was asked in; nothing otherwise")
        .output("unasked", "of something asked: what is said where it was asked, where nobody is asked it; nothing otherwise")
        .output("untold", "of something asked: what is said where the fleet reports, where nobody is asked it, naming what it was asked on; nothing otherwise")
        .idempotent(),
        |_, args| async move {
            let text = |key: &str| args.get(key).and_then(Value::as_str).unwrap_or_default();
            let rows = match args.get("rows") {
                Some(Value::Array(rows)) => rows.as_slice(),
                _ => &[],
            };
            let nothing = Map::new();
            let said = match args.get("said") {
                None | Some(Value::Null) => &nothing,
                Some(Value::Object(said)) => said,
                Some(other) => {
                    return Err(ActionError::new(format!(
                        "`said` is what an event carried, by name; it was given {other}"
                    )));
                }
            };
            let telling = whom(rows, text("of"), said, text("from"), text("project")).map_err(ActionError::new)?;
            Ok(json!({
                "tells": !telling.to.is_empty(),
                "to": telling.to,
                "says": telling.says,
                "about": telling.about,
                "why": telling.of.word(),
                "once": telling.once,
                "asks": telling.asks,
                "unasked": telling.unasked,
                "untold": telling.untold,
            }))
        },
    );
}

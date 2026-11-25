//! The tickets a document lists: read out of it, and filed in it once
//! they have come to something.
//!
//! Whoever plans work writes a document, and says the pieces it comes to
//! in a block of that document: a list written as JSON between a line that
//! says ` ```tickets ` and one that says ` ``` `, each a mapping with a
//! `title` and, where wanted, a `body` and `after`, the places in the list
//! of the tickets it waits on. A blueprint makes something of each
//! (`std/plan` adds a card), and then has the document say what they came
//! to where the block was, so that the same document read again lists
//! none.
//!
//! Taking the block out of the text, reading it, working out what each
//! ticket's `after` comes to once the tickets are things with ids, and
//! writing the document with the block replaced, are the two functions
//! here: [`read`] and [`filed`]. Neither reads or writes a file, and
//! neither knows what a ticket becomes.

use serde_json::{Value, json};

/// What opens the block, and what closes it.
const OPENS: &str = "```tickets";
const CLOSES: &str = "```";

/// Where the ids of what the tickets came to are written in the words
/// that take the block's place.
pub const IDS: &str = "{ids}";

/// What reading a document for its tickets came to. The kernel stands on
/// no language, so its words are written out as `blueprint::came!`
/// writes them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Came {
    /// It lists none: it has no block. One whose tickets were filed is
    /// such a document.
    None,
    /// Its tickets were read.
    Read,
    /// It has a block, and what is in it is not a list of tickets.
    Unread,
}

impl Came {
    /// Every word it can come to, in the order they are declared.
    pub const ALL: &'static [Came] = &[Came::None, Came::Read, Came::Unread];

    /// The word a blueprint reads.
    pub const fn word(self) -> &'static str {
        match self {
            Came::None => "none",
            Came::Read => "read",
            Came::Unread => "unread",
        }
    }

    /// The words, as a sentence lists them: `` `a`, `b` or `c` ``.
    pub fn words() -> String {
        let words: Vec<String> = Self::ALL.iter().map(|one| format!("`{}`", one.word())).collect();
        match words.split_last() {
            Some((last, rest)) if !rest.is_empty() => format!("{} or {last}", rest.join(", ")),
            _ => words.concat(),
        }
    }
}

/// A document read for its tickets.
#[derive(Clone, Debug, PartialEq)]
pub struct Read {
    pub came: Came,
    /// The tickets, in the order they are listed; none unless they were read.
    pub tickets: Vec<Value>,
    /// Why they could not be read, where they could not.
    pub why: String,
}

/// What stands between the first line that opens a block and the next
/// thing that closes one; nothing where no block is opened.
fn listed(content: &str) -> Option<&str> {
    let (_, after) = content.split_once(OPENS)?;
    Some(after.split(CLOSES).next().unwrap_or(after))
}

/// The tickets a document lists.
pub fn read(content: &str) -> Read {
    let unread = |why: String| Read { came: Came::Unread, tickets: Vec::new(), why };
    let Some(listed) = listed(content) else {
        return Read { came: Came::None, tickets: Vec::new(), why: String::new() };
    };
    match serde_json::from_str::<Value>(listed) {
        Ok(Value::Array(tickets)) => match tickets.iter().position(|ticket| !ticket.is_object()) {
            None => Read { came: Came::Read, tickets, why: String::new() },
            Some(place) => unread(format!(
                "ticket {} is {}, and a ticket is a mapping with a `title`",
                place + 1,
                tickets[place]
            )),
        },
        Ok(other) => unread(format!("they are a list, and what is written is {}", kind(&other))),
        Err(err) => unread(format!("this is not JSON: {err}")),
    }
}

fn kind(value: &Value) -> &'static str {
    match value {
        Value::Null => "nothing",
        Value::Bool(_) => "true or false",
        Value::Number(_) => "a number",
        Value::String(_) => "a text",
        Value::Array(_) => "a list",
        Value::Object(_) => "one mapping",
    }
}

/// What tickets came to, once each is a thing with an id.
#[derive(Clone, Debug, PartialEq)]
pub struct Filed {
    /// For each ticket in turn: the `id` of what it came to, and `after`,
    /// the ids of what the tickets it waits on came to.
    pub links: Vec<Value>,
    /// The ids, in the order of the tickets.
    pub ids: Vec<String>,
    /// The document, saying `in_place` wherever its block was.
    pub content: String,
}

/// The ids of what `made` came to, each by its `id`.
fn ids_of(made: &[Value]) -> Result<Vec<String>, String> {
    made.iter()
        .enumerate()
        .map(|(place, one)| match &one["id"] {
            Value::String(id) if !id.is_empty() => Ok(id.clone()),
            _ => Err(format!("what ticket {} came to has no `id`: {one}", place + 1)),
        })
        .collect()
}

/// `tickets` that came to `made`, one each and in the same order, filed
/// in the document they were read from.
///
/// A ticket's `after` names places in the list, counted from 1; a place
/// no ticket is at is left out, as it always was. `in_place` is what the
/// document says where its block was, with the ids in it where it says
/// `{ids}`.
pub fn filed(content: &str, tickets: &[Value], made: &[Value], in_place: &str) -> Result<Filed, String> {
    if tickets.len() != made.len() {
        return Err(format!(
            "there are {} tickets and {} things they came to: one each, in the same order",
            tickets.len(),
            made.len()
        ));
    }
    let ids = ids_of(made)?;
    let mut links = Vec::new();
    for (place, ticket) in tickets.iter().enumerate() {
        let after: Vec<String> = match &ticket["after"] {
            Value::Null => Vec::new(),
            Value::Array(places) => places
                .iter()
                .filter_map(|at| at.as_u64().and_then(|at| usize::try_from(at).ok()))
                .filter_map(|at| at.checked_sub(1).and_then(|at| ids.get(at)))
                .cloned()
                .collect(),
            other => {
                return Err(format!(
                    "`after` of ticket {} is the places in the list of the tickets it waits on ([1, 2]); it is \
                     {other}",
                    place + 1
                ));
            }
        };
        links.push(json!({ "id": ids[place], "after": after }));
    }
    let says = in_place.replace(IDS, &ids.join(", "));
    let content = match listed(content) {
        Some(listed) => content.replace(&format!("{OPENS}{listed}{CLOSES}"), &says),
        None => content.to_string(),
    };
    Ok(Filed { links, ids, content })
}

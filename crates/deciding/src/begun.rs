//! What is begun for an item where whoever has its column begins one
//! thing for each item and hears from it, and what the things done about
//! that one came to.
//!
//! The things done with an item where it is are a list (`segments`): what
//! is usual, with what the rows of a table lay about it. Where the usual
//! thing is begun and heard from (those who look at an item, whoever
//! says how it would be done), whoever has the column begins it itself,
//! and the list is gone through only for what is called before it and
//! after it. [`first`] says which: whether anything is called before the
//! first thing that is begun for this item, and what that one is begun
//! with, so that an item no row applies to costs no going through. And
//! where nothing is sent back from a column, [`ended`] says what the
//! word a list ended with comes to there. Each is a function of what it
//! is handed, with a table of cases.

use blueprint::{ActionSpec, ValueType as T};
use conductor::{ActionError, Registry};
use serde_json::{Map, Value, json};

use crate::add;
use crate::segments::{self, Asked};

pub mod left;
pub mod tidied;

/// What an action was handed, with a mapping it was handed whole (`from`)
/// saying each of these where nothing else does: the name the action
/// takes a thing by, and the word of the mapping that says it. So a text
/// hands on what it was itself handed (its row, what an event said) in
/// one line, and says by name only what it says otherwise.
pub(crate) fn with(args: &kernel::Fields, from: &str, stands: &[(&str, &str)]) -> kernel::Fields {
    let mut args = args.clone();
    let Some(said) = args.get(from).and_then(Value::as_object).cloned() else { return args };
    for (name, word) in stands {
        let there = args.get(*name).is_some_and(|said| !said.is_null());
        match said.get(*word) {
            Some(value) if !there && !value.is_null() => args.insert(name.to_string(), value.clone()),
            _ => None,
        };
    }
    args
}

/// What the first of a list is for an item.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct First {
    /// Something is called before anything is begun and heard from, or
    /// nothing is begun at all: the list is gone through.
    pub lists: bool,
    /// Where in the list the thing to begin is, where it is the first.
    pub row: String,
    pub order: i64,
    /// Only what is usual comes after it.
    pub usual: bool,
    /// Who it is put to and how many of them, as its row says and as
    /// whoever asks says of itself where the row says nothing.
    pub with: Map<String, Value>,
}

/// What the first of the list is for an item, from where the asking
/// begins: the thing that is begun and heard from, with what it is begun
/// with; or that the list is to be gone through, something being called
/// before it, nothing being begun at all, or its row saying of it what
/// cannot be said (which the going through says in a sentence).
pub fn first(asked: &Asked, own: &Map<String, Value>) -> First {
    let ahead = segments::ahead(asked);
    let through = First { lists: true, row: asked.row.clone(), order: asked.order, ..First::default() };
    let Some(run) = ahead.first().filter(|run| run.spawned) else { return through };
    match segments::council(run, own) {
        Ok(with) => First {
            lists: false,
            row: run.row.clone(),
            order: run.order,
            usual: segments::rest_is_usual(asked, run),
            with,
        },
        Err(_) => through,
    }
}

/// [`first`], of what an action was handed: `rows`, `usual` and `of` as
/// `segments.next` takes them, what the item holds (`labels`, `part_of`),
/// where it was said to be due for (`row`, `order`), and what whoever
/// asks says of itself (`own`). With no row of its own, whoever asks
/// begins what is usual as it always did, and is where it was said.
pub(crate) fn first_of(args: &kernel::Fields, item: &str, row: &str, order: i64) -> First {
    let own = segments::mapping(args, "own");
    let labels = args.get("labels").and_then(Value::as_array);
    let asked = Asked {
        rows: segments::rows(args),
        usual: segments::usual(args),
        item: item.to_string(),
        labels: labels.map(|all| all.iter().map(blueprint::expr::display).collect()).unwrap_or_default(),
        part_of: segments::text(args, "part_of"),
        row: row.to_string(),
        order,
        ..Asked::default()
    };
    if asked.rows.is_empty() {
        let usual = segments::Run { spawned: true, ..segments::Run::default() };
        let with = segments::council(&usual, &own).unwrap_or_default();
        return First { lists: false, row: row.to_string(), order, usual: true, with };
    }
    first(&asked, &own)
}

/// Every word [`ended`] can answer.
pub const ENDED_WORDS: &[&str] = &["on", "left", "nothing"];

/// What the things done with an item came to, where nothing is sent back
/// from its column.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Ended {
    /// The item, whether it was found, and the column it is in.
    pub item: String,
    pub found: bool,
    pub status: String,
    /// The column whose items these are, and where one goes on to.
    pub here: String,
    pub pass: String,
    /// The word they ended with, and what was said with it.
    pub came: String,
    pub says: String,
    /// What was said as the handing on was left to whoever asks, by
    /// whatever was begun for the item or by the things done before it:
    /// what is said of the item, where the things done since said nothing.
    pub said: String,
    /// They could not be gone through at all, and why.
    pub failed: bool,
    pub why: String,
    /// How its row says it came to be where it is, now; and as it said
    /// when whoever asks first looked, before anything was done.
    pub moved: Vec<String>,
    pub had: Vec<String>,
}

/// Which of an item's moves its row says was the last: none, for one
/// never moved; nothing that can be held against another, where the row
/// says something of its moves that cannot be read.
fn last_move(moved: &[String]) -> Option<u64> {
    match moved.is_empty() {
        true => Some(0),
        false => studio::models::Moved::read(moved).map(|moved| moved.count),
    }
}

/// What that comes to: the item goes on, with what was said; it is left
/// for a person, with the sentence that says why; or nothing, the item
/// being gone or moved since whoever asks first looked, or there being
/// nowhere for it to go on to. It is asked as the item is to be handed
/// on, of the item as it is then, by something that keeps nothing: asked
/// again where the daemon stopped as the item was handed on, it finds
/// the item moved and hands nothing on a second time. A word that sends it back leaves it: there is nobody here it is
/// sent back to. And so does a list that says something more is to be
/// begun and heard from: one thing is begun for an item here.
pub fn ended(asked: &Ended) -> Result<(&'static str, String), String> {
    if asked.item.is_empty() || asked.here.is_empty() {
        return Err(
            "`item` and `here` are the item and the column whose items these are, and one is not said".into(),
        );
    }
    if !asked.found || asked.status != asked.here {
        return Ok(("nothing", String::new()));
    }
    // Moved since whoever asks first looked, it is here by another coming:
    // somebody took it out and put it back, or what is asked was asked
    // before and the item handed on then, and has come round since.
    if last_move(&asked.moved) != last_move(&asked.had) {
        return Ok(("nothing", String::new()));
    }
    let (item, here) = (&asked.item, &asked.here);
    let said =
        [asked.says.trim(), asked.said.trim()].into_iter().find(|said| !said.is_empty()).unwrap_or_default();
    if asked.failed {
        let why = if asked.why.is_empty() { "it could not be called" } else { asked.why.as_str() };
        return Ok((
            "left",
            format!("{item}: what is done with a card in {here} could not be gone through: {why}"),
        ));
    }
    match asked.came.as_str() {
        "" | "on" if asked.pass.is_empty() => Ok(("nothing", String::new())),
        "" | "on" => Ok(("on", said.to_string())),
        "back" => {
            let says = format!(
                "{item}: what is done with a card in {here} sent it back, and nothing is sent back from there: it \
                 is left for a person. {said}"
            );
            Ok(("left", says.trim_end().to_string()))
        }
        "review" => Ok((
            "left",
            format!(
                "{item}: a row of `segments` names something more to begin for a card in {here} once the first \
                 has done, and one thing is begun for a card there: it is left for a person."
            ),
        )),
        "left" if said.is_empty() => Ok(("left", format!("{item}: it is left in {here} for a person."))),
        "left" => Ok(("left", said.to_string())),
        "nothing" => Ok(("nothing", String::new())),
        other => Err(format!(
            "what was done with {item} in {here} came to `{other}`, which is none of `on`, `back`, `left` or `nothing`"
        )),
    }
}

/// Adds what is asked here to what a blueprint can do.
pub fn register(registry: &mut Registry) {
    left::register(registry);
    tidied::register(registry);
    add(
        registry,
        ActionSpec::new(
            "work.came",
            "Say what the things done with an item where something is begun for each item came to: it goes on, \
             it is left for a person, or nothing.",
        )
        .deciding()
        .required("item", T::String, "the item")
        .required("found", T::Bool, "whether it was found")
        .arg("status", T::String, "the column it is in")
        .required("here", T::String, "the column whose items these are")
        .arg("pass", T::String, "where an item goes on to from there")
        .arg("came", T::String, "the word they ended with: `on` (if not said), `back`, `left` or `nothing`")
        .arg("says", T::String, "what was said with it")
        .arg("said", T::String, "what whatever was begun for the item said as it ended: said as the item goes on where nothing else is")
        .arg("failed", T::Bool, "they could not be gone through at all")
        .arg("why", T::String, "why not")
        .arg("moved", T::List, "how its row says it came to be where it is")
        .arg("had", T::List, "the same, as its row said when whoever asks first looked: an item moved since is no longer theirs to hand on")
        .output("came", "`on`, `left` or `nothing`")
        .output("says", "what is said as the item goes on, or the sentence where it is left")
        .idempotent(),
        move |_, args| async move {
            let text = |key: &str| segments::text(&args, key);
            let words = |key: &str| -> Vec<String> {
                let said = args.get(key).and_then(Value::as_array);
                said.map(|all| all.iter().map(blueprint::expr::display).collect()).unwrap_or_default()
            };
            let asked = Ended {
                item: text("item"),
                found: segments::flag(&args, "found"),
                status: text("status"),
                here: text("here"),
                pass: text("pass"),
                came: text("came"),
                says: text("says"),
                said: text("said"),
                failed: segments::flag(&args, "failed"),
                why: text("why"),
                moved: words("moved"),
                had: words("had"),
            };
            let (came, says) = ended(&asked).map_err(ActionError::new)?;
            Ok(json!({ "came": came, "says": says }))
        },
    );
}

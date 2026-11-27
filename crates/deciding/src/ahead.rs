//! Whether an item waiting where items are worked may be taken now, or
//! waits behind items worked from there before it that are not merged.
//!
//! An item's place is cut from the project's branch as it is when its
//! round begins. Where whoever works the column takes the next item as
//! soon as the last is handed on, the next is cut while the last waits to
//! be looked over, and has nothing of its work; items on a board often
//! build on the one before. So the row of whoever works the column may
//! say that the next is taken only once none it worked waits unmerged
//! (`next_after: merged`): none is in hand, and none in the column it
//! hands items on to. What it says otherwise, or nothing, is as it always
//! was (`handed`).
//!
//! Only an item that waits in the column is held back: one taken from it
//! already (sent back, answered, taken up again) goes on, or whoever works
//! it would wait on itself.
//!
//! [`ahead`] says which from the board's items as they are, and knows
//! nothing of what an item is beyond its id, its column and the column it
//! was taken from.

use blueprint::{ActionSpec, ValueType as T};
use conductor::{ActionError, Registry};
use serde_json::{Value, json};

use crate::add;

/// What [`ahead`] is asked.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Ahead {
    /// The item, and the column it is in.
    pub item: String,
    pub status: String,
    /// The column whose items are worked by whoever asks, the one an item
    /// is in while in hand, the one a worked item is handed on to, and
    /// the one it is finished in.
    pub here: String,
    pub doing: String,
    pub hands_to: String,
    pub done: String,
    /// What the row of whoever asks says of when the next item is taken:
    /// `handed`, `merged`, or nothing said, which is `handed`.
    pub next_after: String,
    /// The board's items, each with its `id`, `status`, and the column it
    /// was taken from (`more.taken`).
    pub items: Vec<Value>,
    /// What the item says it waits for now.
    pub waiting: String,
}

/// What [`ahead`] comes to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AheadCame {
    /// It may be taken.
    Go,
    /// It waits behind these, by id: why, for what is told of it
    /// (`kernel::cards::turn`), and whether it says something else now.
    Behind { ids: Vec<String>, why: String, news: bool },
}

impl AheadCame {
    pub fn word(&self) -> &'static str {
        match self {
            AheadCame::Go => "go",
            AheadCame::Behind { .. } => "behind",
        }
    }
}

/// Every word [`ahead`] can answer.
pub const AHEAD_WORDS: &[&str] = &["go", "behind"];

/// Whether an item may be taken now, by the items worked from the same
/// column that are in hand or handed on and not merged.
pub fn ahead(asked: &Ahead) -> Result<AheadCame, String> {
    if asked.item.is_empty() || asked.here.is_empty() || asked.doing.is_empty() {
        return Err(
            "`item`, `here` and `doing` are the item, the column whose items are worked and the one an item \
             is in while in hand, and one is not said"
                .into(),
        );
    }
    let merged = match asked.next_after.as_str() {
        "" | "handed" => false,
        "merged" => true,
        said => {
            return Err(format!(
                "`next_after` says when the next card is taken where cards are worked: `handed`, as soon as the \
                 last is handed on, or `merged`, once none worked from there waits unmerged; it says `{said}`"
            ));
        }
    };
    if !merged || asked.status != asked.here {
        return Ok(AheadCame::Go);
    }
    let said = |item: &Value, key: &str| item.get(key).map(blueprint::expr::display).unwrap_or_default();
    let unmerged = |status: &str| {
        status != asked.done
            && (status == asked.doing || (!asked.hands_to.is_empty() && status == asked.hands_to))
    };
    let ids: Vec<String> = asked
        .items
        .iter()
        .filter(|item| said(item, "id") != asked.item)
        .filter(|item| {
            item.pointer("/more/taken").map(blueprint::expr::display).unwrap_or_default() == asked.here
        })
        .filter(|item| unmerged(&said(item, "status")))
        .map(|item| said(item, "id"))
        .collect();
    if ids.is_empty() {
        return Ok(AheadCame::Go);
    }
    let is = if ids.len() == 1 { "is" } else { "are" };
    let why = format!("{}, worked from here, {is} not merged yet", ids.join(", "));
    let news = asked.waiting != kernel::cards::turn(&why, 1);
    Ok(AheadCame::Behind { ids, why, news })
}

/// Adds what is asked here to what a blueprint can do.
pub fn register(registry: &mut Registry) {
    let said = |args: &kernel::Fields, key: &str| {
        args.get(key).and_then(Value::as_str).unwrap_or_default().to_string()
    };
    add(
        registry,
        ActionSpec::new(
            "work.ahead",
            "Say whether an item waiting where items are worked may be taken now, or waits behind items worked \
             from there before it that are in hand or handed on and not merged, where the row says the next is \
             taken only once none is.",
        )
        .deciding()
        .required("item", T::String, "the item")
        .arg("status", T::String, "the column it is in")
        .required("here", T::String, "the column whose items are worked by whoever asks")
        .required("doing", T::String, "the column an item is in while it is in hand")
        .arg("hands_to", T::String, "the column a worked item is handed on to")
        .arg("done", T::String, "the column an item is finished in")
        .arg("next_after", T::String, "when the next item is taken: `handed`, as soon as the last is handed on; `merged`, once none worked from here waits unmerged; `handed`, if not said")
        .arg("items", T::List, "the board's items, each with its `id`, `status` and `more.taken`")
        .arg("waiting", T::String, "what the item says it waits for now")
        .output("came", "`go`, it may be taken; `behind`, it waits behind others")
        .output("behind", "for `behind`: the items it waits behind, by id")
        .output("why", "for `behind`: why, for what is told of it")
        .output("news", "for `behind`: whether it says something else now, and is to be told")
        .idempotent(),
        move |_, args| async move {
            let asked = Ahead {
                item: said(&args, "item"),
                status: said(&args, "status"),
                here: said(&args, "here"),
                doing: said(&args, "doing"),
                hands_to: said(&args, "hands_to"),
                done: said(&args, "done"),
                next_after: said(&args, "next_after"),
                items: args.get("items").and_then(Value::as_array).cloned().unwrap_or_default(),
                waiting: said(&args, "waiting"),
            };
            let came = ahead(&asked).map_err(ActionError::new)?;
            Ok(match &came {
                AheadCame::Behind { ids, why, news } => {
                    json!({ "came": came.word(), "behind": ids, "why": why, "news": news })
                }
                AheadCame::Go => json!({ "came": came.word(), "behind": [], "why": "", "news": false }),
            })
        },
    );
}

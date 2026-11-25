//! What becomes of a round when its item is moved away from it.
//!
//! An item is in the column where items are in hand while a round has
//! it, and somebody may move it out of there while its agent is at it:
//! back to where it waited, further back, on, or done. The round went on
//! to its end all the same, saw the item had moved and handed nothing
//! on, and an item put back where items are taken from was due again
//! and was worked a second time, as new, once a place was free. What
//! whoever works the column says of it (its row: `moved_away`) is what
//! becomes of it: `stop`, the usual, and the round is stopped at once,
//! its agent's turn called off, with what it left where it worked kept
//! as it is, and the item's row says that it was taken out of its
//! agent's hands, so that brought back it is taken up where it was left
//! and not given again as new; or `finish`, and the round goes on to its
//! end as it always did.
//!
//! [`away`] says which, from the move and the item's row, and knows
//! nothing of what an item is.

use blueprint::{ActionSpec, ValueType as T};
use conductor::{ActionError, Registry};
use serde_json::{Value, json};

use super::{Held, HeldCame, Moved, held, words};
use crate::add;

/// What [`away`] is asked: a move of an item out of the column it is in
/// while in hand, and the item's row as it is.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Away {
    /// Where it was moved from, and where to.
    pub from: String,
    pub to: String,
    /// The column whose items are worked by whoever asks, and the one an
    /// item is in while it is in hand.
    pub here: String,
    pub doing: String,
    /// The column its row says it was taken from: whose it is.
    pub taken: String,
    /// What its row keeps of its rounds: `[heard, n]` while a round has it.
    pub again: Vec<String>,
    /// The move was made by a blueprint's step (it says `through`): what
    /// works the item handing it on, and nobody taking it away.
    pub through: bool,
    /// What the row of whoever asks says of an item moved away from its
    /// round: `stop`, `finish`, or nothing said, which is `stop`.
    pub how: String,
}

/// What a move away comes to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AwayCame {
    /// The round is stopped, and the item's row is to keep this of its
    /// rounds: that it was taken out of its agent's hands, and where to.
    Stop(Vec<String>),
    /// Nothing: it was not moved away from a round of whoever asks, or
    /// whoever asks lets its round go on to its end.
    Nothing,
}

impl AwayCame {
    pub fn word(&self) -> &'static str {
        match self {
            AwayCame::Stop(_) => "stop",
            AwayCame::Nothing => "nothing",
        }
    }
}

/// Every word [`away`] can answer.
pub const AWAY_WORDS: &[&str] = &["stop", "nothing"];

/// Every word a row may say of an item moved away from its round.
pub const HOW_WORDS: &[&str] = &["stop", "finish"];

/// What becomes of an item's round when the item is moved out of the
/// column it is in while in hand.
///
/// It is stopped where the row of whoever asks says so (or says
/// nothing), the item was taken from that row's column, a round has it
/// (its row says the round was told of it, `heard`), and the move was
/// somebody's and not a blueprint's: a round hands its item on itself,
/// and a stage that sends an item back moves it in. Moved within the
/// column, or into it, nothing becomes of anything.
pub fn away(asked: &Away) -> Result<AwayCame, String> {
    if asked.here.is_empty() || asked.doing.is_empty() {
        return Err(
            "`here` and `doing` are the column whose items are worked and the one an item is in while in \
             hand, and one is not said"
                .into(),
        );
    }
    let how = match asked.how.as_str() {
        "" => "stop",
        said if HOW_WORDS.contains(&said) => said,
        said => {
            return Err(format!(
                "`moved_away` says what becomes of an item's round when it is moved away while its agent is \
                 at it, `stop` or `finish`; it says `{said}`"
            ));
        }
    };
    let held = asked.again.first().is_some_and(|word| word == "heard");
    let moved_away = asked.from == asked.doing && asked.to != asked.doing && !asked.to.is_empty();
    Ok(match moved_away && held && asked.taken == asked.here && !asked.through && how == "stop" {
        true => AwayCame::Stop(vec!["away".into(), asked.to.clone()]),
        false => AwayCame::Nothing,
    })
}

/// What was said as an item was last moved, where its row says.
pub(super) fn said_of(moved: &Option<Moved>) -> &str {
    moved.as_ref().map_or("", |moved| moved.said.as_str())
}

/// Whether an item whose round ended was moved away from it by
/// somebody, where whoever asks says where it is now.
pub(super) fn moved_away(args: &kernel::Fields) -> bool {
    let said = |key: &str| args.get(key).and_then(Value::as_str).unwrap_or_default().to_string();
    let asked = Held {
        status: said("status"),
        from: said("here"),
        doing: said("doing"),
        moved: words(args, "moved"),
        heard: args.get("heard").and_then(Value::as_u64),
    };
    !asked.status.is_empty() && moved_since(&asked)
}

/// Whether somebody has moved an item since its round was told of it,
/// whatever it was moved to: on, away, or back to where it is taken
/// from, which [`held`] answers as an item for the round to take.
/// Back there, it has moved since where the count of its moves is past
/// the one the round was told of, the round's own taking of it
/// included; one a round never took has not.
pub fn moved_since(asked: &Held) -> bool {
    match held(asked) {
        Ok(HeldCame::Gone) => true,
        Ok(HeldCame::Take) => {
            let count = Moved::read(&asked.moved).map_or(0, |moved| moved.count);
            asked.heard.is_some_and(|heard| count > heard)
        }
        _ => false,
    }
}

/// Adds what is asked here to what a blueprint can do.
pub fn register(registry: &mut Registry) {
    let said = |args: &kernel::Fields, key: &str| {
        args.get(key).and_then(Value::as_str).unwrap_or_default().to_string()
    };
    add(
        registry,
        ActionSpec::new(
            "work.away",
            "Say what becomes of an item's round when somebody moves the item out of the column it is in while \
             in hand: the round is stopped and the item kept as taken out of its agent's hands, or nothing.",
        )
        .deciding()
        .required("from", T::String, "the column it was moved from")
        .required("to", T::String, "the column it was moved to")
        .required("here", T::String, "the column whose items are worked by whoever asks")
        .required("doing", T::String, "the column an item is in while it is in hand")
        .arg("taken", T::String, "the column its row says it was taken from")
        .arg("again", T::List, "what its row keeps of its rounds")
        .arg("through", T::Bool, "the move was made by a blueprint's step")
        .arg("how", T::String, "what the row of whoever asks says of an item moved away from its round: `stop`, or `finish`; `stop`, if not said")
        .output("came", "`stop`, its round is stopped; `nothing`")
        .output("again", "for `stop`: what its row is to keep of its rounds")
        .idempotent(),
        move |_, args| async move {
            let again = args.get("again").and_then(Value::as_array).cloned().unwrap_or_default();
            let asked = Away {
                from: said(&args, "from"),
                to: said(&args, "to"),
                here: said(&args, "here"),
                doing: said(&args, "doing"),
                taken: said(&args, "taken"),
                again: again.iter().map(blueprint::expr::display).collect(),
                through: args.get("through").and_then(Value::as_bool).unwrap_or(false),
                how: said(&args, "how"),
            };
            let came = away(&asked).map_err(ActionError::new)?;
            let again = match &came {
                AwayCame::Stop(again) => again.clone(),
                AwayCame::Nothing => Vec::new(),
            };
            Ok(json!({ "came": came.word(), "again": again }))
        },
    );
}

//! Whether an item is held where it is until somebody presses what its
//! column offers.
//!
//! A row of a table beside the board may be something pressed on an
//! item in one column (`kernel::pressed`): one that holds keeps the item
//! there, and whatever looks after the column begins nothing for it
//! until each such row stands pressed at this coming of the item. That
//! is asked here, of what the item's row keeps of what was pressed, the
//! count of its moves (which coming this is) and the rows as the stage
//! was handed them; the blocks a stage asks of an item that is due
//! (`work.due`, `review.owed`, `work.owed`) ask it before anything else
//! they would begin, and answer `held` with what goes with it.

use kernel::pressed::{Pressed, held_says, rows, waits};
use serde_json::{Map, Value, json};
use studio::models::Moved;

/// What is asked.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Held {
    /// The item, and the column it is in.
    pub item: String,
    pub here: String,
    /// The rows of what can be pressed, as they were read.
    pub actions: Vec<Value>,
    /// What the item keeps of what was pressed on it.
    pub pressed: Vec<String>,
    /// How its row says it came to be where it is.
    pub moved: Vec<String>,
}

/// What a held item waits for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hold {
    /// Which coming of it this is: the count of its moves.
    pub at: u64,
    /// What is still to be pressed, by what each is called.
    pub waits: Vec<String>,
    /// The sentence that says so.
    pub says: String,
}

impl Hold {
    /// As a block hands it on to what keeps it on the item.
    pub fn value(&self) -> Value {
        json!({ "at": self.at, "waits": self.waits, "says": self.says })
    }
}

/// The coming of an item its row's moves say: none for one never moved,
/// or whose row says something that is not a move.
pub fn coming(moved: &[String]) -> u64 {
    Moved::read(moved).map_or(0, |moved| moved.count)
}

/// Whether the item is held, and by what: nothing where no row of its
/// column holds it, or every one that does stands pressed.
pub fn held(asked: &Held) -> Option<Hold> {
    let actions = rows(&asked.actions);
    let pressed = Pressed::read(&asked.pressed);
    let at = coming(&asked.moved);
    let left = waits(&actions, &asked.here, &pressed, at);
    if left.is_empty() {
        return None;
    }
    let says = held_says(&asked.item, &asked.here, &left);
    Some(Hold { at, waits: left.iter().map(|action| action.title.clone()).collect(), says })
}

/// The same, of what a block was handed: `actions`, or the `actions` of
/// what whoever asks says of itself (`own`, its row as the fleet read
/// it); `pressed` and `moved`, the item's.
pub(crate) fn of(args: &Map<String, Value>, item: &str, here: &str) -> Option<Hold> {
    let list = |key: &str| args.get(key).and_then(Value::as_array).cloned().unwrap_or_default();
    let words = |key: &str| list(key).iter().map(blueprint::expr::display).collect::<Vec<_>>();
    let actions = match args.get("actions").and_then(Value::as_array) {
        Some(actions) => actions.clone(),
        None => args
            .get("own")
            .and_then(|own| own.get("actions"))
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default(),
    };
    held(&Held {
        item: item.to_string(),
        here: here.to_string(),
        actions,
        pressed: words("pressed"),
        moved: words("moved"),
    })
}

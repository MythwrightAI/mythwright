//! What a card keeps beyond where it is, and what follows from it: the
//! functions of plain values `studio::cards` offers under the names they
//! had there, with the actions that ask them.
//!
//! A card is a row of its board, and what looks after the cards of a
//! board is one blueprint (`std/cards`): it hears what is said of any card
//! and writes what the card is to keep on the card's own row. What it
//! hears is said by events and what it writes is steps; what it decides
//! in between is here, each a function of what the row says and what was
//! said, which the blueprint asks as an action and then does what it is
//! told:
//!
//! - [`came`] (`board.came`): what a card's being moved means for it;
//! - [`said`] (`board.said`): whether what was said on it answers what was
//!   asked on it;
//! - [`turn`] (`board.waits`): the words for a card that waits its turn;
//! - [`after`] (`board.after`): whether the cards it waits on are done;
//! - [`held`] (`board.held`): which cards wait on other cards.
//!
//! Which column is which is the board's and its blueprint's: nothing here
//! names one. Each function is handed the columns it has to tell apart.
//!
//! Nothing is remembered between whiles but what the rows say. A card
//! that was asked whether it may begin and may not says what it waits for
//! in its `waiting`, in words this writes ([`waiting_for`]) and this
//! reads back ([`waits_on_others`]): that is what says it was asked, for
//! whenever a card it names is finished or taken away.

use serde_json::{Map, Value};

/// A value as a blueprint says it in text, as the language's `display`
/// does: the kernel stands on no language.
fn display(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

/// The words a block answers, written out as the language's `came!` writes
/// them: the kernel stands on no language.
macro_rules! worded {
    (
        $(#[$meta:meta])*
        $vis:vis enum $name:ident {
            $($(#[$of:meta])* $variant:ident => $word:literal),+ $(,)?
        }
    ) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        $vis enum $name {
            $($(#[$of])* $variant),+
        }

        impl $name {
            /// Every word it can come to, in the order they are declared.
            pub const ALL: &'static [$name] = &[$($name::$variant),+];

            /// The word a blueprint reads.
            pub const fn word(self) -> &'static str {
                match self {
                    $($name::$variant => $word),+
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
    };
}

/// What a card's row keeps for whatever looks after cards, read as a row
/// a person may have edited is read: a cell emptied and one never filled
/// both say nothing.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Kept {
    /// The column it is in.
    pub status: String,
    /// What was asked on it and is not answered.
    pub question: String,
    /// Who asked.
    pub asker: String,
    /// The column it was first taken from to be worked.
    pub taken: String,
    /// How many times it was sent back.
    pub rounds: u64,
    /// What it says it waits for.
    pub waiting: String,
    /// The cards it waits on, by id.
    pub after: Vec<String>,
}

impl Kept {
    /// What a card keeps, out of the fields its board declared beyond a
    /// card's own.
    pub fn of(status: &str, more: &Map<String, Value>) -> Kept {
        let words = |key: &str| more.get(key).and_then(Value::as_str).unwrap_or_default().to_string();
        let after = match more.get("after") {
            Some(Value::Array(ids)) => ids.iter().map(display).filter(|id| !id.is_empty()).collect(),
            // A board of somebody's own may keep one card there, and not a list.
            Some(Value::String(id)) if !id.is_empty() => vec![id.clone()],
            _ => Vec::new(),
        };
        Kept {
            status: status.to_string(),
            question: words("question"),
            asker: words("asker"),
            taken: words("taken"),
            // A count kept as a number with a point in it is still a count.
            rounds: more
                .get("rounds")
                .and_then(Value::as_f64)
                .filter(|count| *count > 0.0)
                .map_or(0, |count| count as u64),
            waiting: words("waiting"),
            after,
        }
    }
}

/// The columns a move is told apart by, as the board's blueprint says
/// them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Columns<'a> {
    /// Where cards are worked.
    pub worked: &'a str,
    /// Where work is looked over: a card put from there to where cards
    /// are worked was sent back.
    pub looked: &'a str,
    /// Where a card is finished.
    pub done: &'a str,
    /// The columns that stand before where work is looked over, as the
    /// board has them: a card put in one of them from there was sent
    /// back, wherever its stage says a card that did not pass goes.
    pub before: &'a [&'a str],
}

worded! {
    /// What a card's being moved came to.
    pub enum Came {
        /// It was put where cards are worked for the first time: where
        /// from is kept.
        Taken => "taken",
        /// It was sent back to be worked again, which is counted.
        Back => "back",
        /// It is finished.
        Finished => "finished",
        /// Anything else: nothing is kept of it.
        Elsewhere => "elsewhere",
    }
}

/// What a move means for what a card keeps.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Arrived {
    pub came: Came,
    /// A person put it somewhere while something asked on it was open:
    /// where they put it is their answer, and the question comes off.
    pub settles: bool,
    /// How many times it has now been sent back, where it was.
    pub rounds: Option<u64>,
    /// It is finished and still says it waits for something.
    pub rests: bool,
}

/// What a card's move from `from` to `to` means for it, `kept` being what
/// its row says and `by_hand` whether somebody made the move and no
/// blueprint did.
///
/// Put where cards are worked from where work is looked over, it was sent
/// back. Put there from anywhere else it was taken, the first time, and
/// its row keeps where from; after that, put there from anywhere but
/// where it was taken from, it was sent back too, by whatever looked at
/// it later. Put back from where it was taken from, it is only taken up
/// again. And put from where work is looked over in any column that
/// stands before that one, it was sent back as well: where a card that
/// did not pass goes is a stage's to say, and it may say where cards
/// wait to be worked. But a card sent back for something other than a
/// verdict on its work (`uncounted`, said with the move: to be brought up
/// to date) is not counted, and nothing is kept of it.
pub fn came(
    from: &str,
    to: &str,
    by_hand: bool,
    uncounted: bool,
    kept: &Kept,
    columns: Columns<'_>,
) -> Arrived {
    let came = if to == columns.done {
        Came::Finished
    } else if uncounted && !by_hand {
        Came::Elsewhere
    } else if from == columns.looked && columns.before.contains(&to) {
        Came::Back
    } else if to != columns.worked {
        Came::Elsewhere
    } else if from == columns.looked || (!kept.taken.is_empty() && from != kept.taken) {
        Came::Back
    } else if kept.taken.is_empty() {
        Came::Taken
    } else {
        Came::Elsewhere
    };
    Arrived {
        came,
        settles: by_hand && !kept.question.is_empty(),
        rounds: (came == Came::Back).then(|| kept.rounds + 1),
        rests: came == Came::Finished && !kept.waiting.is_empty(),
    }
}

worded! {
    /// What something said on a card came to.
    pub enum Said {
        /// It answers what was asked on the card.
        Answer => "answer",
        /// Nothing was asked, or whoever asked is who spoke.
        Nothing => "nothing",
    }
}

/// Whether what `by` said on a card answers what was asked on it: a
/// question is open, and it was not the agent that asked who spoke. A
/// card that does not say who asked takes anybody's words for the answer.
pub fn said(by: &str, kept: &Kept) -> Said {
    let asked_by = format!("agent:{}", kept.asker);
    match !kept.question.is_empty() && by != asked_by {
        true => Said::Answer,
        false => Said::Nothing,
    }
}

/// What a card says while it waits its turn where so many are worked at
/// once (`limit`), or where nothing is being started for now (`why`).
pub fn turn(why: &str, limit: i64) -> String {
    match (why.is_empty(), limit) {
        (false, _) => format!("no work is being started here for now: {why}"),
        (true, 1) => "one card is worked at a time here".to_string(),
        (true, limit) => format!("{limit} cards are worked at a time here"),
    }
}

/// How a card says it waits on other cards begins.
pub const AFTER: &str = "after ";

/// What a card says while the cards `left` are not done.
pub fn waiting_for(left: &[String]) -> String {
    format!("{AFTER}{}, not done yet", left.join(", "))
}

/// Whether what a card says it waits for is other cards: what
/// [`waiting_for`] wrote, whichever cards it named.
pub fn waits_on_others(waiting: &str) -> bool {
    waiting.starts_with(AFTER)
}

worded! {
    /// Whether a card may begin, for the cards it waits on.
    pub enum Awaited {
        /// Nothing it waits on is left.
        Clear => "clear",
        /// Some of them are not done.
        Waits => "waits",
    }
}

/// What asking a card whether the cards it waits on are done came to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct After {
    pub came: Awaited,
    /// What it is to say it waits for, where it waits.
    pub says: String,
    /// It said it waited on cards and none is left: that comes off it,
    /// and it is said to be free.
    pub lifts: bool,
    /// It waits, and its row is to say `says`, which it does not say now.
    pub news: bool,
    /// Whoever would begin it is to be told that nothing is left.
    pub answers: bool,
}

/// What a card is to do about the cards it waits on, `left` being those
/// of them that are not done, `kept` what its row says, and `keeps`
/// whether its board keeps what a card waits for at all.
///
/// `asked` is whether something that would begin the card asked, and is
/// owed its answer: then it is told when nothing is left, every time, and
/// a card that has to wait says so. Looked at again for any other reason
/// (a card it names was finished or taken away, what it waits on was said
/// again, the board started), a card that does not say it waits has
/// nobody waiting on it: nothing is written on it and nobody is told. So
/// a card looked at twice for two cards finished in one moment is said to
/// be free once.
pub fn after(asked: bool, left: &[String], kept: &Kept, keeps: bool) -> After {
    let (clear, said) = (left.is_empty(), waits_on_others(&kept.waiting));
    let says = match clear {
        true => String::new(),
        false => waiting_for(left),
    };
    After {
        came: if clear { Awaited::Clear } else { Awaited::Waits },
        lifts: clear && said,
        news: !clear && (asked || said) && kept.waiting != says,
        // A board that cannot say what a card waits for cannot say that
        // it was asked either, and such a card is told whenever it is
        // looked at and nothing is left.
        answers: clear && (asked || said || !keeps),
        says,
    }
}

/// A card as [`held`] reads it: its id and what it keeps.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    pub id: String,
    pub kept: Kept,
}

/// The cards of `rows` that wait on other cards, in the order given:
/// those not in `done` that name cards to wait on and say they wait on
/// them, which is what says somebody asked. With `by`, only those that
/// name that card.
///
/// Where the board keeps nothing of what a card waits for (`keeps` is
/// not so), no card can say it was asked: those that name `by` are all
/// given, and with no `by` none is.
pub fn held(rows: &[Row], done: &str, by: Option<&str>, keeps: bool) -> Vec<String> {
    rows.iter()
        .filter(|row| row.kept.status != done && !row.kept.after.is_empty())
        .filter(|row| by.is_none_or(|by| row.kept.after.iter().any(|id| id == by)))
        .filter(|row| match keeps {
            true => waits_on_others(&row.kept.waiting),
            false => by.is_some(),
        })
        .map(|row| row.id.clone())
        .collect()
}

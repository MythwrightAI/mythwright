//! What becomes of a piece of work where items are worked: what a text
//! that gives items to agents asks, and does as it is told.
//!
//! An item waits in a column until its turn comes, is worked in a round
//! while it is in the column where things are in hand, and is handed on.
//! What a turn comes to, and what a round that ended comes to, were
//! worked out in the text that asks, in the holes of its strings, from
//! the item's row and from whatever some handler had written there since
//! it heard of the item's last move. So an answer rested on which handler
//! had run first, and nothing tested it but playing the whole. Each is a
//! function here, of what the row says and nothing else, with a table of
//! cases that reaches every word it can answer.
//!
//! What the row says, and these read:
//!
//! - `status`, the column the item is in;
//! - `taken`, the column it was taken from to be worked, which is whose
//!   it is while it is in hand;
//! - `moved`, how it came to be where it is, written with the move
//!   itself ([`studio::models::Moved`]);
//! - `again`, what the text that works it keeps of its rounds: `[heard,
//!   n]`, a round has been told of the item's move `n`; `[asked]` and
//!   `[left, why]`, its round is over and it waits for a person;
//!   `[answer, words]`, what it asked was answered; `[unkept, why,
//!   tries]`, what was done could not be kept and it is given again;
//!   `[again, words, tries]`, something done with it where it is sent
//!   it back to whoever works it, with those words. A
//!   row written before the row said how the item came says `[back,
//!   words]` or `[said, words]` there, and is read as it was meant.
//!
//! Where items are not worked but something is started for each that
//! comes to a column, what an item said to be due is owed is [`coming`]:
//! by its row it is there now, and by the count of its moves this coming
//! of it is not one that was begun for already.
//!
//! Nothing here knows what an item is, or where it is kept: each is
//! handed the words of a row and answers with a word.

use blueprint::{ActionSpec, ValueType as T};
use conductor::{ActionError, Registry};
use serde_json::{Value, json};
use studio::models::Moved;

use crate::add;
use crate::turn::ASKED_NOTHING;

mod away;
pub use crate::turn::{WORKED_WORDS, Worked, WorkedCame, worked};
pub use away::{AWAY_WORDS, Away, AwayCame, HOW_WORDS, away, moved_since};
use away::{moved_away, said_of};

/// What a turn that came for an item is asked of.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Turn {
    /// Whether the item was found at all.
    pub found: bool,
    /// The column it is in.
    pub status: String,
    /// The column whose items are worked by whoever asks.
    pub here: String,
    /// The column an item is in while it is in hand.
    pub doing: String,
    /// The column it was taken from, as its row says.
    pub taken: String,
    /// What its row keeps of its rounds.
    pub again: Vec<String>,
    /// How it came to be where it is, as its row says.
    pub moved: Vec<String>,
    /// How often at most it is given again; none said is twice.
    pub most: i64,
    /// Where it goes on to when it is worked, and where when none is said.
    pub to: String,
    pub else_to: String,
    /// Where in the journal the turn came: which turn this is.
    pub at: u64,
}

/// A round that is to begin, and what it is told.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Round {
    /// It is taken from the column now: its row is to say whose it is.
    pub mine: bool,
    /// It was worked before and is given again.
    pub back: bool,
    /// It was in hand when whoever had it was stopped, and is taken up
    /// again where it was left: nobody sent it back.
    pub resumed: bool,
    /// What was said is the answer to what was asked on it.
    pub answer: bool,
    /// It is given again because what was done could not be kept.
    pub unkept: bool,
    /// What was said as it was put where it is, or the answer, or why
    /// what was done could not be kept.
    pub note: String,
    /// How often it has been given again for that.
    pub tries: i64,
    pub most: i64,
    pub to: String,
    /// What its row is to keep of its rounds from here.
    pub again: Vec<String>,
    /// The word this round is asked by, and no other round: asked for
    /// again after a restart, it is given the turn it had.
    pub asking: String,
    /// Which of the item's moves the round is told of, by the move's
    /// count: what the round holds the item's row against, to know that
    /// the item is still as it has it ([`held`]).
    pub heard: i64,
}

/// What a turn comes to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TurnCame {
    /// The item is gone, or is somewhere it is not worked from: the turn
    /// is given back.
    Gone,
    /// It is in hand and owed no round: its round is over and it waits
    /// for a person, or nothing has been said of it since the round that
    /// had it. The turn is given back.
    Over,
    Round(Round),
}

impl TurnCame {
    pub fn word(&self) -> &'static str {
        match self {
            TurnCame::Gone => "gone",
            TurnCame::Over => "over",
            TurnCame::Round(_) => "round",
        }
    }
}

/// Every word [`turn`] can answer.
pub const TURN_WORDS: &[&str] = &["gone", "over", "round"];

/// What a turn that came for an item comes to, by its row.
///
/// In its column it is taken up: the first time, or again where it was
/// worked from here before, told what was said as it was put there. In
/// hand, a round is owed it where it was answered, where what was done
/// could not be kept, and where it was sent back and no round has been
/// told of that move: its row says it was moved there from somewhere
/// else, by the move's own count, whoever has heard of the move. Anything
/// else in hand is owed nothing, and a turn that comes for it is one
/// nobody meant (an item can be said to be due twice).
pub fn turn(asked: &Turn) -> Result<TurnCame, String> {
    if asked.here.is_empty() {
        return Err("`here` is the column whose items are worked, and none is said".into());
    }
    if asked.doing.is_empty() {
        return Err("`doing` is the column an item is in while it is in hand, and none is said".into());
    }
    if asked.most < 0 {
        return Err(format!(
            "`most` is how often at most an item is given again, and is not less than none; it is {}",
            asked.most
        ));
    }
    if !asked.found {
        return Ok(TurnCame::Gone);
    }
    if asked.status.is_empty() {
        return Err("the item was found, and its `status` says no column".into());
    }
    let word = asked.again.first().map(String::as_str).unwrap_or_default();
    let second = asked.again.get(1).map(String::as_str).unwrap_or_default();
    let tries = asked.again.get(2).and_then(|tries| tries.parse::<i64>().ok()).unwrap_or(0);
    let moved = Moved::read(&asked.moved);
    let count = moved.as_ref().map_or(0, |moved| moved.count);
    let round = |mine: bool, back: bool, note: &str| Round {
        mine,
        back,
        resumed: false,
        answer: false,
        unkept: false,
        note: note.to_string(),
        tries: 0,
        most: if asked.most > 0 { asked.most } else { 2 },
        to: if asked.to.is_empty() { asked.else_to.clone() } else { asked.to.clone() },
        again: vec!["heard".to_string(), count.to_string()],
        asking: asked.at.to_string(),
        heard: count as i64,
    };
    let in_column = asked.status == asked.here;
    if !in_column && asked.status != asked.doing {
        return Ok(TurnCame::Gone);
    }
    // Taken from the column now, where its row does not say it was.
    let mine = in_column && asked.taken != asked.here;
    if in_column {
        // In its column it is taken up, the first time or again, and told
        // what was said as it was put there: by the move's own word,
        // whatever its row kept of an earlier round. A row no move has
        // written since the row came to say so is read as the handler
        // that heard it put there left it.
        return Ok(TurnCame::Round(match (&moved, word) {
            // Taken out of its agent's hands by being moved away while it
            // was worked (`away`): taken up where it was left, and not
            // given again as one sent back.
            (_, "away") if !mine => Round { resumed: true, ..round(false, false, said_of(&moved)) },
            (Some(moved), _) => round(mine, !mine, &moved.said),
            (None, "back") => round(mine, true, second),
            (None, "said") => round(mine, false, second),
            (None, _) => round(mine, !mine, ""),
        }));
    }
    Ok(match word {
        "asked" | "left" => TurnCame::Over,
        "answer" => TurnCame::Round(Round { answer: true, ..round(false, true, second) }),
        "unkept" => TurnCame::Round(Round { unkept: true, tries, ..round(false, true, second) }),
        "again" => TurnCame::Round(Round { tries, ..round(false, true, second) }),
        // Left in hand by whoever had it before, and taken up again: it
        // is not one sent back, and is told nothing was said.
        "resumed" => TurnCame::Round(Round { resumed: true, ..round(false, false, "") }),
        // Taken out of its agent's hands and put back in hand: by a
        // stage that sent it back there, it is sent back; by somebody,
        // it is taken up where it was left.
        "away" => match &moved {
            Some(moved) if moved.by == "blueprint" => TurnCame::Round(round(false, true, &moved.said)),
            _ => TurnCame::Round(Round { resumed: true, ..round(false, false, said_of(&moved)) }),
        },
        // A row from before the row said how the item came: what a
        // handler wrote of its being sent back.
        "back" => TurnCame::Round(round(false, true, second)),
        _ => {
            // Whether a round has been told of the move its row now says.
            let heard = word == "heard" && second.parse() == Ok(count);
            match &moved {
                Some(moved) if asked.taken == asked.here && moved.from != asked.here && !heard => {
                    TurnCame::Round(round(false, true, &moved.said))
                }
                _ => TurnCame::Over,
            }
        }
    })
}

/// What a round that ended is asked of, by whoever asked for the round.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Ended {
    /// The round said what was done could not be kept, and that the item
    /// is to be given again.
    pub again: bool,
    /// What it said of why.
    pub why: String,
    /// The round failed.
    pub failed: bool,
    /// The item could not be taken, and no round was begun.
    pub untaken: bool,
    /// How often it had been given again for that before this round.
    pub tries: i64,
    /// The round ended before it had got as far as asking anybody, and
    /// so before it gave its place back.
    pub unreached: bool,
    /// Its agent asked something in place of working: who, and what.
    pub asked: bool,
    pub asker: String,
    pub question: String,
    /// It is given again because something done with it sent it back,
    /// and not because what was done could not be kept.
    pub back: bool,
    /// The round left the item for a person, and why; and why it
    /// failed, where it did, which leaves the item too.
    pub left: bool,
    pub leaves: String,
    pub reason: String,
    /// Somebody moved the item away from the round while it was at it
    /// ([`held`] says it is no longer the round's): a round that was
    /// stopped for that has not failed, and leaves nothing.
    pub gone: bool,
}

/// What a round that ended comes to, for whoever asked for it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EndedCame {
    /// It is given again: what its row is to keep, and its turn is asked
    /// for once more.
    Again(Vec<String>),
    /// The round did not get as far as giving its place back, or was
    /// never begun: the place is given back here.
    Freed,
    /// Nothing more: the round did what there was to do.
    Done,
}

impl EndedCame {
    pub fn word(&self) -> &'static str {
        match self {
            EndedCame::Again(_) => "again",
            EndedCame::Freed => "freed",
            EndedCame::Done => "done",
        }
    }
}

/// What a round's agent asked in place of working, where it did and the
/// round says who: the asker, and the question. It is for whoever asked
/// for the round to say, and not the round: a round keeps nothing and is
/// begun again where the daemon stopped before it was over, and one that
/// said its own question said it a second time then. Said after somebody
/// had answered, the question was back on the item for good.
pub fn asks(asked: &Ended) -> Option<(String, String)> {
    if !asked.asked || asked.asker.is_empty() || asked.failed {
        return None;
    }
    let question = if asked.question.is_empty() { ASKED_NOTHING } else { &asked.question };
    Some((asked.asker.clone(), question.to_string()))
}

/// Why a round left its item for a person, where it did: it said so, or
/// it failed, which leaves the item where it is. Like what its agent
/// asked, it is for whoever asked for the round to say, once.
pub fn left(asked: &Ended) -> Option<String> {
    if asked.gone {
        return None;
    }
    match (asked.failed && !asked.untaken, asked.left) {
        (true, _) if asked.reason.is_empty() => Some("its round failed".to_string()),
        (true, _) => Some(asked.reason.clone()),
        (false, true) => Some(asked.leaves.clone()),
        (false, false) => None,
    }
}

/// Every word [`ended`] can answer.
pub const ENDED_WORDS: &[&str] = &["again", "freed", "done"];

pub fn ended(asked: &Ended) -> Result<EndedCame, String> {
    if asked.tries < 0 {
        return Err(format!(
            "`tries` is how often an item has been given again, and is not less than none; it is {}",
            asked.tries
        ));
    }
    Ok(if asked.failed || asked.untaken || asked.unreached {
        EndedCame::Freed
    } else if asked.again {
        let word = if asked.back { "again" } else { "unkept" };
        EndedCame::Again(vec![word.into(), asked.why.clone(), (asked.tries + 1).to_string()])
    } else {
        EndedCame::Done
    })
}

/// What an item that was said to be due is asked of, where items are
/// worked.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Due {
    /// Whether the item was found at all.
    pub found: bool,
    /// The column it is in.
    pub status: String,
    /// The column whose items are worked by whoever asks.
    pub here: String,
    /// The items it waits on, as its row says.
    pub after: Vec<String>,
    /// What it waited on is done: it is not asked again what it waits on.
    pub clear: bool,
    /// Something its column offers holds it there, and is not pressed
    /// (`crate::pressed`).
    pub held: bool,
}

/// What an item that is due comes to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DueCame {
    /// Its turn is asked for.
    Turn,
    /// It waits on others: whether they are done is asked, and it asks
    /// for no place meanwhile.
    Waits,
    /// It waits for something its column offers to be pressed, and asks
    /// for no place meanwhile.
    Held,
    /// It is gone, or has been moved on since it was said to be due.
    Nothing,
}

impl DueCame {
    pub fn word(&self) -> &'static str {
        match self {
            DueCame::Turn => "turn",
            DueCame::Waits => "waits",
            DueCame::Held => "held",
            DueCame::Nothing => "nothing",
        }
    }
}

/// Every word [`due`] can answer.
pub const DUE_WORDS: &[&str] = &["turn", "waits", "held", "nothing"];

/// What an item said to be due where it is worked comes to, by its row:
/// it may have been moved on since, or taken away.
pub fn due(asked: &Due) -> Result<DueCame, String> {
    if asked.here.is_empty() {
        return Err("`here` is the column whose items are worked, and none is said".into());
    }
    if !asked.found {
        return Ok(DueCame::Nothing);
    }
    if asked.status.is_empty() {
        return Err("the item was found, and its `status` says no column".into());
    }
    Ok(if asked.status != asked.here {
        DueCame::Nothing
    } else if asked.held {
        DueCame::Held
    } else if !asked.after.is_empty() && !asked.clear {
        DueCame::Waits
    } else {
        DueCame::Turn
    })
}

/// What an item said to be due is asked of, where something is started
/// for each item that comes to a column.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Coming {
    /// The item, and whether it was found.
    pub item: String,
    pub found: bool,
    /// The column it is in, and the column whose items whoever asks has.
    pub status: String,
    pub here: String,
    /// How its row says it came to be where it is.
    pub moved: Vec<String>,
    /// The items something was started for from here, as whoever asks
    /// keeps them: each a mapping that names the item (`card`) and which
    /// coming of it that was (`at`, the count of its moves then).
    pub kept: Vec<Value>,
    /// Something its column offers holds it there, and is not pressed
    /// (`crate::pressed`).
    pub held: bool,
}

/// What an item that came comes to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ComingCame {
    /// Something is started for it: which coming this is, to be kept.
    Begin { at: String },
    /// It is in the column, and this coming of it was taken up already.
    Had,
    /// It waits for something its column offers to be pressed: nothing
    /// is started for it meanwhile.
    Held,
    /// It is gone, or is not in the column: it was moved on since it was
    /// said to be due, or was said to be due for a move from before.
    Nothing,
}

impl ComingCame {
    pub fn word(&self) -> &'static str {
        match self {
            ComingCame::Begin { .. } => "begin",
            ComingCame::Had => "had",
            ComingCame::Held => "held",
            ComingCame::Nothing => "nothing",
        }
    }
}

/// Every word [`coming`] can answer.
pub const COMING_WORDS: &[&str] = &["begin", "had", "held", "nothing"];

/// What an item said to be due comes to where something is started for
/// each that comes, by its row and what whoever asks keeps.
///
/// An item is said to be due by whatever heard that it came, and that
/// may be long after: what hears a move is handed, when it is given
/// another column to listen for, every move there has been to that
/// column since it was last handed anything, and the items of those
/// moves have gone on. And one coming may be said twice (a look at the
/// column and the saying of the move may both find the item). So it goes
/// by the row: the item is in the column now, and its coming there,
/// which the count of its moves tells from the one before and the one
/// after, is not one something was started for already. An item never
/// moved was added where it is, and that is its coming. Where its row
/// says something of its moves that cannot be read, one coming is not
/// told from the next, and each saying begins as it always did.
pub fn coming(asked: &Coming) -> Result<ComingCame, String> {
    if asked.here.is_empty() {
        return Err("`here` is the column whose items something is started for, and none is said".into());
    }
    if asked.item.is_empty() {
        return Err("`item` is what was said to be due, and none is said".into());
    }
    if !asked.found {
        return Ok(ComingCame::Nothing);
    }
    if asked.status.is_empty() {
        return Err("the item was found, and its `status` says no column".into());
    }
    if asked.status != asked.here {
        return Ok(ComingCame::Nothing);
    }
    let at = match (Moved::read(&asked.moved), asked.moved.is_empty()) {
        (Some(moved), _) => moved.count.to_string(),
        (None, true) => "0".to_string(),
        (None, false) => return Ok(ComingCame::Begin { at: String::new() }),
    };
    let word = |entry: &Value, key: &str| entry.get(key).and_then(Value::as_str).map(str::to_string);
    let had = asked.kept.iter().any(|entry| {
        word(entry, "card").as_deref() == Some(asked.item.as_str()) && word(entry, "at").as_ref() == Some(&at)
    });
    Ok(match (had, asked.held) {
        (true, _) => ComingCame::Had,
        (false, true) => ComingCame::Held,
        (false, false) => ComingCame::Begin { at },
    })
}

/// How often at most, where a row may say: what it says, or the usual
/// where it says none.
pub fn most(said: i64, usual: i64) -> Result<i64, String> {
    if said < 0 || usual < 1 {
        return Err(format!(
            "how often at most is not less than none, and the usual is at least once; they are {said} and {usual}"
        ));
    }
    Ok(if said > 0 { said } else { usual })
}

/// What something started for an item is called: after what it is, and
/// the item, in words a name may be made of.
pub fn called(name: &str, item: &str) -> Result<String, String> {
    let word = blueprint::expr::slug(name);
    if word.is_empty() || item.is_empty() {
        return Err(format!(
            "what is started for an item is called after what it is and the item; they are `{name}` and `{item}`"
        ));
    }
    Ok(format!("{word}-{item}"))
}

/// What a round asks of the item it has, as it begins and before it
/// hands the item on.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Held {
    /// The column the item is in.
    pub status: String,
    /// The column it is taken from, and the one it is in while in hand.
    pub from: String,
    pub doing: String,
    /// How its row says it came to be where it is.
    pub moved: Vec<String>,
    /// Which of its moves the round was told of, by the move's count;
    /// nothing, where whoever asked for the round did not say.
    pub heard: Option<u64>,
}

/// Whether an item is still as a round has it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HeldCame {
    /// It is in the column it is taken from: the round takes it.
    Take,
    /// It is in hand, and the round's: as the round was told of it, or as
    /// the round itself took it.
    Mine,
    /// It is neither: handed on already, or moved by somebody since the
    /// round was told of it. The round works nothing and hands nothing on.
    Gone,
}

impl HeldCame {
    pub fn word(&self) -> &'static str {
        match self {
            HeldCame::Take => "take",
            HeldCame::Mine => "mine",
            HeldCame::Gone => "gone",
        }
    }
}

/// Every word [`held`] can answer.
pub const HELD_WORDS: &[&str] = &["take", "mine", "gone"];

/// Whether an item is still as a round has it, by its row.
///
/// A round is begun again where the daemon stopped in the middle of it,
/// and takes up an item it may already have handed on; and while a round
/// is at work somebody may move its item. Going by where the item is, a
/// round begun again that found its item back in hand (somebody having
/// sent it back in that moment) handed it on a second time, and the
/// sending back was undone with nobody told. So it goes by the item's
/// moves: in hand it is the round's where the last move is the one the
/// round was told of, or the round's own taking of it straight after.
/// Anything else has moved since, and is no longer the round's.
pub fn held(asked: &Held) -> Result<HeldCame, String> {
    if asked.from.is_empty() || asked.doing.is_empty() {
        return Err(
            "`from` and `doing` are the column an item is taken from and the one it is in while in hand, \
                    and one is not said"
                .into(),
        );
    }
    if asked.status.is_empty() {
        return Err("the item's `status` says no column".into());
    }
    if asked.status == asked.from {
        return Ok(HeldCame::Take);
    }
    if asked.status != asked.doing {
        return Ok(HeldCame::Gone);
    }
    let Some(heard) = asked.heard else { return Ok(HeldCame::Mine) };
    let moved = Moved::read(&asked.moved);
    let count = moved.as_ref().map_or(0, |moved| moved.count);
    let taken_by_a_round =
        moved.as_ref().is_some_and(|moved| moved.from == asked.from && moved.by == "blueprint");
    Ok(if count == heard || (count == heard + 1 && taken_by_a_round) {
        HeldCame::Mine
    } else {
        HeldCame::Gone
    })
}

/// A list that was handed, each of its entries as a word.
fn words(args: &kernel::Fields, key: &str) -> Vec<String> {
    let entry = |entry: &Value| entry.as_str().map(str::to_string).unwrap_or_else(|| entry.to_string());
    args.get(key).and_then(Value::as_array).map(|list| list.iter().map(entry).collect()).unwrap_or_default()
}

/// Adds what is asked here to what a blueprint can do.
pub fn register(registry: &mut Registry) {
    let said = |args: &kernel::Fields, key: &str| {
        args.get(key).and_then(Value::as_str).unwrap_or_default().to_string()
    };
    let flag = |args: &kernel::Fields, key: &str| args.get(key).and_then(Value::as_bool).unwrap_or(false);
    let number = |args: &kernel::Fields, key: &str| args.get(key).and_then(Value::as_i64).unwrap_or(0);
    add(
        registry,
        ActionSpec::new(
            "work.turn",
            "Say what a turn that came for an item comes to, by what its row says: a round of it begins, or the \
             turn is given back because its round is over or the item is gone.",
        )
        .deciding()
        .required("found", T::Bool, "whether the item was found")
        .arg("status", T::String, "the column it is in")
        .required("here", T::String, "the column whose items are worked by whoever asks")
        .required("doing", T::String, "the column an item is in while it is in hand")
        .arg("taken", T::String, "the column its row says it was taken from")
        .arg("again", T::List, "what its row keeps of its rounds")
        .arg("moved", T::List, "how its row says it came to be where it is")
        .arg("most", T::Int, "how often at most it is given again; twice, if 0 or not said")
        .arg("to", T::String, "where it goes on to when it is worked")
        .arg("else_to", T::String, "where it goes on to when `to` says nowhere")
        .arg("at", T::Int, "where in the journal the turn came: which turn this is")
        .output("came", "`round`, a round of it begins; `over`, it is in hand and owed none; `gone`, it is not there to be worked")
        .output("mine", "for a round: it is taken from the column now, and its row is to say so")
        .output("back", "for a round: it was worked before and is given again")
        .output("resumed", "for a round: it was left in hand by whoever had it, and is taken up again where it was left")
        .output("answer", "for a round: what was said is the answer to what was asked on it")
        .output("unkept", "for a round: it is given again because what was done could not be kept")
        .output("note", "for a round: what was said as it was put where it is, the answer, or why what was done was not kept")
        .output("tries", "for a round: how often it has been given again for what could not be kept")
        .output("most", "for a round: how often at most")
        .output("to", "for a round: where it goes on to")
        .output("again", "for a round: what its row is to keep of its rounds from here")
        .output("asking", "for a round: the word it is asked by and no other round is")
        .output("heard", "for a round: which of the item's moves it is told of, by the move's count")
        .output("round", "for a round: what it is handed of all this, each under the name a round takes it by")
        .idempotent(),
        move |_, args| async move {
            let asked = Turn {
                found: flag(&args, "found"),
                status: said(&args, "status"),
                here: said(&args, "here"),
                doing: said(&args, "doing"),
                taken: said(&args, "taken"),
                again: words(&args, "again"),
                moved: words(&args, "moved"),
                most: number(&args, "most"),
                to: said(&args, "to"),
                else_to: said(&args, "else_to"),
                at: args.get("at").and_then(Value::as_u64).unwrap_or(0),
            };
            let came = turn(&asked).map_err(ActionError::new)?;
            let round = match &came {
                TurnCame::Round(round) => round.clone(),
                _ => Round::default(),
            };
            Ok(json!({
                "came": came.word(), "mine": round.mine, "back": round.back, "answer": round.answer,
                "resumed": round.resumed,
                "unkept": round.unkept, "note": round.note, "tries": round.tries, "most": round.most,
                "to": round.to, "again": round.again, "asking": round.asking, "heard": round.heard,
                "round": {
                    "asking": round.asking, "to": round.to, "back": round.back, "answer": round.answer,
                    "resumed": round.resumed,
                    "note": round.note, "unkept": round.unkept, "tries": round.tries, "most": round.most,
                    "heard": round.heard,
                },
            }))
        },
    );
    add(
        registry,
        ActionSpec::new(
            "work.due",
            "Say what an item that was said to be due comes to where items are worked: its turn is asked for, it \
             waits on others, or nothing, it having been moved on or taken away.",
        )
        .deciding()
        .required("found", T::Bool, "whether the item was found")
        .arg("status", T::String, "the column it is in")
        .required("here", T::String, "the column whose items are worked by whoever asks")
        .arg("after", T::List, "the items it waits on, as its row says")
        .arg("clear", T::Bool, "what it waited on is done, and it is not asked again")
        .arg("item", T::String, "the item, for what is said of it where it is held")
        .arg("actions", T::List, "the rows of what can be pressed on an item, of every column (`kernel::pressed`)")
        .arg("pressed", T::List, "what the item keeps of what was pressed on it")
        .arg("moved", T::List, "how its row says it came to be where it is")
        .output("came", "`turn`, its turn is asked for; `waits`, it waits on others; `held`, it waits for something its column offers to be pressed; `nothing`")
        .output("hold", "for `held`: which coming of it this is (`at`), what is still to be pressed (`waits`) and the sentence (`says`)")
        .idempotent(),
        move |_, args| async move {
            let here = said(&args, "here");
            let hold = crate::pressed::of(&args, &said(&args, "item"), &here);
            let asked = Due {
                found: flag(&args, "found"),
                status: said(&args, "status"),
                here,
                after: words(&args, "after"),
                clear: flag(&args, "clear"),
                held: hold.is_some(),
            };
            let hold = hold.map(|hold| hold.value()).unwrap_or_default();
            Ok(json!({ "came": due(&asked).map_err(ActionError::new)?.word(), "hold": hold }))
        },
    );
    add(
        registry,
        ActionSpec::new(
            "work.owed",
            "Say what an item that was said to be due is owed where something is started for each item that \
             comes to a column, by what its row says and what is kept of those begun: it is begun; or nothing, \
             this coming of it having been begun for already, or the item not being in the column.",
        )
        .deciding()
        .required("item", T::String, "the item that was said to be due")
        .required("found", T::Bool, "whether the item was found")
        .arg("status", T::String, "the column it is in")
        .required("here", T::String, "the column whose items whoever asks has")
        .arg("moved", T::List, "how its row says it came to be where it is")
        .arg("kept", T::List, "the items begun for from here, each with which coming of it that was (`card`, `at`)")
        .output("came", "`begin`, something is started for it; `had`, this coming of it was begun for already; `held`, it waits for something its column offers to be pressed; `nothing`, it is not in the column")
        .arg("actions", T::List, "the rows of what can be pressed on an item, of every column (`kernel::pressed`)")
        .arg("pressed", T::List, "what the item keeps of what was pressed on it")
        .output("hold", "for `held`: which coming of it this is (`at`), what is still to be pressed (`waits`) and the sentence (`says`)")
        .output("at", "for `begin`: which coming this is, by the count of its moves, to be kept with the item")
        .arg("name", T::String, "what is started for it: a blueprint, by its name")
        .arg("said", T::Int, "how often at most it goes round, as the row says; the usual, if 0 or not said")
        .arg("most", T::Int, "how often at most where the row says none (once, if not said)")
        .arg("rows", T::List, "the rows of a table that lay more into the things done here, as `segments.next` takes them")
        .arg("usual", T::List, "what is usually done here, as `segments.next` takes it")
        .arg("of", T::Object, "which rows are this list's, as `segments.next` takes it")
        .arg("labels", T::List, "the labels the item holds")
        .arg("part_of", T::String, "what it is part of")
        .arg("own", T::Object, "what whoever asks says of itself: who it is put to (`role`, `picks`); and, where it is not said by name, how often at most it goes round (`rounds`)")
        .output("begins", "for `begin`: what is usual is started at once, nothing being called before it")
        .output("lists", "for `begin`: something is called first, or nothing is started: the list is gone through")
        .output("usual", "only what is usual comes after what is started")
        .output("with", "who it is put to, as the row that names what is started says and as whoever asks says where it does not")
        .output("called", "with `name`: what is started is called this, the name in words a key may be made of and the item")
        .output("most", "how often at most it goes round")
        .idempotent(),
        move |_, args| async move {
            let asked = Coming {
                item: said(&args, "item"),
                found: flag(&args, "found"),
                status: said(&args, "status"),
                here: said(&args, "here"),
                moved: words(&args, "moved"),
                kept: args.get("kept").and_then(Value::as_array).cloned().unwrap_or_default(),
                held: false,
            };
            let hold = crate::pressed::of(&args, &asked.item, &asked.here);
            let asked = Coming { held: hold.is_some(), ..asked };
            let came = coming(&asked).map_err(ActionError::new)?;
            let at = match &came {
                ComingCame::Begin { at } => at.clone(),
                _ => String::new(),
            };
            let args = crate::begun::with(&args, "own", &[("said", "rounds")]);
            let begin = matches!(came, ComingCame::Begin { .. });
            let first = crate::begun::first_of(&args, &asked.item, "", 0);
            let name = said(&args, "name");
            let called = match name.is_empty() {
                true => String::new(),
                false => called(&name, &asked.item).map_err(ActionError::new)?,
            };
            let usual = args.get("most").and_then(Value::as_i64).unwrap_or(1);
            Ok(json!({
                "came": came.word(), "at": at, "begins": begin && !first.lists, "lists": begin && first.lists,
                "usual": first.usual, "with": first.with, "called": called,
                "most": most(number(&args, "said"), usual).map_err(ActionError::new)?,
                "hold": hold.map(|hold| hold.value()).unwrap_or_default(),
            }))
        },
    );
    add(
        registry,
        ActionSpec::new(
            "work.started",
            "Say what goes with something started for an item: what it is called, and how often at most it goes \
             round where its row says none.",
        )
        .deciding()
        .required("name", T::String, "what is started: a blueprint, by its name")
        .required("item", T::String, "the item it is started for")
        .arg("said", T::Int, "how often at most, as the row says; the usual, if 0 or not said")
        .arg("usual", T::Int, "how often at most where the row says none (once, if not said)")
        .output("called", "what it is called: the name in words a key may be made of, and the item")
        .output("most", "how often at most")
        .idempotent(),
        move |_, args| async move {
            let usual = args.get("usual").and_then(Value::as_i64).unwrap_or(1);
            Ok(json!({
                "called": called(&said(&args, "name"), &said(&args, "item")).map_err(ActionError::new)?,
                "most": most(number(&args, "said"), usual).map_err(ActionError::new)?,
            }))
        },
    );
    add(
        registry,
        ActionSpec::new(
            "work.held",
            "Say whether an item is still as a round has it, by what its row says of its moves: the round takes \
             it, it is the round's in hand, or it has gone on or been moved by somebody since.",
        )
        .deciding()
        .required("status", T::String, "the column it is in")
        .required("from", T::String, "the column it is taken from")
        .required("doing", T::String, "the column an item is in while it is in hand")
        .arg("moved", T::List, "how its row says it came to be where it is")
        .arg("heard", T::Int, "which of its moves the round was told of, by the move's count; not said, if less than 0")
        .output("came", "`take`, the round takes it; `mine`, it is the round's in hand; `gone`, it is no longer the round's")
        .idempotent(),
        move |_, args| async move {
            let heard = args.get("heard").and_then(Value::as_i64).unwrap_or(-1);
            let asked = Held {
                status: said(&args, "status"),
                from: said(&args, "from"),
                doing: said(&args, "doing"),
                moved: words(&args, "moved"),
                heard: u64::try_from(heard).ok(),
            };
            Ok(json!({ "came": held(&asked).map_err(ActionError::new)?.word() }))
        },
    );
    add(
        registry,
        ActionSpec::new(
            "work.worked",
            "Say what giving an item to an agent came to: it was worked and goes on, its agent asked something in \
             place of working, or there was nobody to give it to and it is left where it is.",
        )
        .deciding()
        .required("item", T::String, "the item")
        .arg("available", T::Bool, "whether there was anybody to give it to (`available` of `agent.task`)")
        .arg("named", T::String, "who it was to be given to by name, where that one is not up (`named` of `agent.task`)")
        .arg("told", T::String, "who whoever asks named for it itself; empty where the item named them")
        .arg("external", T::Bool, "whether the one named is an agent the factory does not start, which takes up what is given to it itself (`external` of `agent.task`)")
        .arg("decision", T::String, "what the agent chose")
        .arg("by", T::String, "the agent")
        .arg("reply", T::String, "what it said")
        .arg("ends", T::List, "the words the agent may end with, each as `word=what it comes to` (`turn.came`); `done=on` and `ask=asks`, if not said")
        .arg("stays", T::String, "where the item stays when there was nobody to give it to, in words that follow `it stays`; `in doing`, if not said")
        .arg("seats", T::Int, "how many looked at it at once, each for itself (`seats` of `agent.task`); none, if 0 or not said. Where several did, `ends` says what they came to together comes to (`passed`, `failed`, `undecided`), as `turn.came` does")
        .arg("decided", T::String, "with `seats`: what those who looked came to together (`came` of `agent.task`)")
        .arg("words", T::String, "with `seats`: what they said (`words` of `agent.task`)")
        .arg("refused", T::List, "what the agent's program said it was refused as it went (`refused` of `agent.task`)")
        .arg("refused_means", T::String, "what a turn whose program was refused something comes to where its agent went on: `note`, it goes on and that is only said (the usual); `blocked`, it is left for a person")
        .arg("finish", T::List, "how the work is finished, where whoever gave it says (`finish` of `turn.came`): a turn that comes to `on` goes on only where its agent said every step is done, or waits only on something checked later; nearly done, it is given again once (`again`) and then left; with none done, left")
        .arg("given", T::Int, "how often it has been given again for being nearly done")
        .arg("cut", T::Bool, "its time ran out, its program was stopped, and what it chose and said is what it said when asked then what it had done")
        .output("came", "`left` where there was nobody to give it to; otherwise what the word the agent ended with comes to, or with `seats` what they came to together: `on`, `back`, `again`, `asks` or `left`")
        .output("nearly", "for `again`: it is given again because it was nearly done, `why` saying what is left")
        .output("time", "`cut`, its time ran out, or `kept`, it did not: a word to count it by")
        .output("says", "for `left`: the sentence for where the work is reported; for `back`: the words it goes back with")
        .output("why", "for `left`: why, in words kept with the item; for `again`: why")
        .output("asker", "for `asks`: who asked")
        .output("question", "for `asks`: what")
        .output("left_as", "for `left`, which way: `nobody`, there was nobody to give it to; `external`, an external agent takes it up; `unfinished`, not all of how it is finished was done; `cut`, its time ran out and it said nothing; or the word the agent ended with that comes to it, as `blocked`")
        .idempotent(),
        move |_, args| async move {
            let asked = Worked {
                item: said(&args, "item"),
                available: flag(&args, "available"),
                named: said(&args, "named"),
                told: !said(&args, "told").is_empty(),
                external: flag(&args, "external"),
                decision: said(&args, "decision"),
                by: said(&args, "by"),
                reply: said(&args, "reply"),
                ends: crate::turn::listed(&args, "ends"),
                stays: said(&args, "stays"),
                seats: crate::turn::seats(&args),
                decided: said(&args, "decided"),
                words: said(&args, "words"),
                refused: crate::turn::listed(&args, "refused"),
                refused_means: said(&args, "refused_means"),
                finish: crate::turn::listed(&args, "finish"),
                given: number(&args, "given"),
                cut: flag(&args, "cut"),
            };
            let came = worked(&asked).map_err(ActionError::new)?;
            Ok(json!({
                "came": came.came, "says": came.says, "why": came.why, "asker": came.asker,
                "question": came.question, "left_as": came.left_as, "nearly": came.nearly,
                "time": if asked.cut { "cut" } else { "kept" },
            }))
        },
    );
    add(
        registry,
        ActionSpec::new(
            "work.ended",
            "Say what a round that ended comes to for whoever asked for it: the item is given again, its place \
             is given back, or nothing more.",
        )
        .deciding()
        .arg("again", T::Bool, "the round said the item is to be given again, what was done not having been kept")
        .arg("why", T::String, "what it said of why")
        .arg("failed", T::Bool, "the round failed")
        .arg("untaken", T::Bool, "the item could not be taken, and no round was begun")
        .arg("tries", T::Int, "how often it had been given again for that before this round")
        .arg("unreached", T::Bool, "the round ended before it had got as far as asking anybody")
        .arg("asked", T::Bool, "its agent asked something in place of working")
        .arg("asker", T::String, "who asked, where the round says")
        .arg("question", T::String, "what it asked")
        .arg("back", T::Bool, "it is given again because something done with it sent it back, and not because what was done could not be kept")
        .arg("reason", T::String, "why the round failed, where it did")
        .arg("gave", T::Object, "what the round gave out, in place of saying each thing of it: `again`, `why`, `asked`, `asker`, `question`, `back`, `left`, `leaves`, and `available`, which a round that did not get as far as asking anybody leaves unsaid")
        .arg("status", T::String, "the column the item is in now; with `here`, `doing`, `moved` and `heard`, whether somebody moved it away from the round while it was at it (`work.held`), which leaves it for nobody")
        .arg("here", T::String, "the column it is taken from")
        .arg("doing", T::String, "the column an item is in while it is in hand")
        .arg("moved", T::List, "how its row says it came to be where it is now")
        .arg("heard", T::Int, "which of its moves the round was told of, by the move's count")
        .output("came", "`again`, it is given again; `freed`, its place is to be given back here; `done`, nothing more")
        .output("again", "for `again`: what its row is to keep of its rounds")
        .output("asks", "whether its agent asked something that is to be said of the item here")
        .output("asker", "for one that asks: who")
        .output("question", "for one that asks: what, or what one that said nothing is taken to have asked")
        .output("left", "whether the item is left for a person, the round having said so or failed, which is to be said of it here")
        .output("leaves", "for one that is left: why")
        .idempotent(),
        move |_, args| async move {
            // What the round gave out, handed whole, says each of these
            // where nothing else does.
            let gave = args.get("gave").filter(|_| args.contains_key("gave"));
            let of = |key: &str| gave.and_then(|gave| gave.get(key)).filter(|said| !said.is_null());
            let flag = |key: &str| of(key).and_then(Value::as_bool).unwrap_or_else(|| flag(&args, key));
            let said = |key: &str| {
                of(key).and_then(Value::as_str).map(str::to_string).unwrap_or_else(|| said(&args, key))
            };
            let asked = Ended {
                again: flag("again"),
                why: said("why"),
                failed: flag("failed"),
                untaken: flag("untaken"),
                tries: number(&args, "tries"),
                unreached: match gave {
                    Some(_) => of("available").is_none(),
                    None => flag("unreached"),
                },
                asked: flag("asked"),
                asker: said("asker"),
                question: said("question"),
                back: flag("back"),
                left: flag("left"),
                leaves: said("leaves"),
                reason: said("reason"),
                gone: moved_away(&args),
            };
            let came = ended(&asked).map_err(ActionError::new)?;
            let again = match &came {
                EndedCame::Again(again) => again.clone(),
                _ => Vec::new(),
            };
            let (asker, question) = asks(&asked).unwrap_or_default();
            let leaves = left(&asked);
            Ok(json!({
                "came": came.word(), "again": again, "asks": !asker.is_empty(), "asker": asker, "question": question,
                "left": leaves.is_some(), "leaves": leaves.unwrap_or_default(),
            }))
        },
    );
    away::register(registry);
}

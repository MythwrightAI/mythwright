//! What one card cost, by round.
//!
//! A card goes round: it is worked, the work is looked over, and it goes
//! on, is sent back to be worked again, or is left for a person. Each of
//! those goings round is a *round* here: the agents' turns that were
//! taken for it, what they say they took (tokens sent and written, what
//! they cost, how long each was at work), and what was decided at its
//! end. Nothing is kept for this. It is read from two things that are
//! kept already, each by an index and never a board whole: the card's
//! turns, found by the conversations an agent holds about it, and the
//! card's history.
//!
//! - **Which turns are the card's** is the conversation each is part of
//!   ([`does`]): `lane:<card>` where it is worked, `plan:<card>` where
//!   it is planned, and `review:<card>:<seat>` for a seat at its review,
//!   which is what the client reads (`heldIn` in `ui/src/heldin.ts`). One of any of
//!   those with a seat after the card is a seat: something laid into a
//!   round that has several look at once.
//! - **What was decided** is what the card's history says, in its order
//!   ([`decided`]): that somebody approved it (`card.approved`); that it
//!   was moved to a column before the one it was in, or to the board's
//!   last, after its work was handed on (`card.worked`), which is a card
//!   sent back or passed by whoever moved it; and that it was left for a
//!   person (`card.left`), wherever that was. A move before the work was
//!   handed on (somebody pulling a card out of work) decides nothing.
//! - **Which round a turn is of** is what is written of the asking it
//!   was taken for, never when it was asked ([`begun_by`]): a worker's
//!   turn says in its word (`asking`) the event that gave the card its
//!   turn (`go` in `std/working` hands the round that event's number),
//!   a plan's says `round-<number>`, and a seat at a review is asked by
//!   the review, whose name is `review-<card>-<number>` after the event
//!   that asked for it (the last name of `asked_by`). That number and
//!   the history's are of one journal, the one that keeps what is said
//!   of the card's project, so a turn is of the first round decided by a
//!   line said after the event its asking names, and of a round under
//!   way where none was. A turn from before, whose row says no such word,
//!   goes with the turn written just before it among the card's turns
//!   (`turns.nth`), or just after it where it is the first; and where no
//!   turn of the card says one, the turns are put in rounds by the order
//!   they were written, a round beginning at each turn that works or
//!   plans after one that looked, which is said of each such turn
//!   (`by_order`). Two things in one millisecond are told apart all the
//!   same: nothing here reads a clock.
//!
//! What is laid out is counted and never scored: nothing here says a
//! round, an agent or a card did well. A turn whose program said nothing
//! of what it took (a stand-in, or one with nothing to count by) is
//! counted as one that said nothing ([`Spent::unsaid`]), never as a
//! turn that took nothing. A card that keeps no history of its moves, or
//! whose rounds say no `card.worked`, has its turns in one round with
//! nothing decided. No action asks this: it is the daemon's to lay out
//! once for whoever reads a card (`GET /api/board/items/{id}/cost`), as
//! a comparison is laid out once (`comparisons::beside`).

use serde_json::{Value, json};

blueprint::came! {
    /// What a turn did for the card.
    pub enum Does {
        /// It worked the card.
        Works => "works",
        /// It said how the card is to be done.
        Plans => "plans",
        /// It looked the work over, one of those at its review.
        Looks => "looks",
        /// It did one of several pieces of work on the card at once, each
        /// in a place of its own (its way, after the card).
        Contends => "contends",
        /// It judged those pieces of work, alone or one of several.
        Judges => "judges",
    }
}

blueprint::came! {
    /// What was decided at the end of a round.
    pub enum Came {
        /// The work went on: approved, or passed by whoever moved it.
        Passed => "passed",
        /// It was sent back to be worked again.
        Back => "back",
        /// It was left for a person.
        Left => "left",
        /// Nothing yet: the round is under way.
        Open => "open",
    }
}

/// The conversations an agent holds about a card, each the word before
/// the card's id: where it is worked, where it is planned, where it is
/// looked over, where a rung of the project's chain looks at it while it
/// waits (`std/chain`), where one of several pieces of work on it is
/// done, and where those are judged.
pub const CONVERSATIONS: [(&str, Does); 6] = [
    ("lane", Does::Works),
    ("plan", Does::Plans),
    ("review", Does::Looks),
    ("chain", Does::Looks),
    ("contend", Does::Contends),
    ("judge", Does::Judges),
];

/// Each conversation a turn of `card` may be part of, as it is written
/// before any seat: a turn's is that, or that and `:<seat>`.
pub fn conversations(card: &str) -> Vec<String> {
    CONVERSATIONS.iter().map(|(word, _)| format!("{word}:{card}")).collect()
}

/// What a turn of the conversation `thread` did for `card`, or nothing
/// where the conversation is not one about it. What follows the card is
/// which piece of work it is where several are done at once, and judges'
/// seat; anywhere else it is a seat, somebody looking, whatever the
/// conversation is called.
pub fn does(thread: &str, card: &str) -> Option<Does> {
    let (word, rest) = thread.split_once(':')?;
    let (_, does) = CONVERSATIONS.iter().find(|(said, _)| *said == word)?;
    let after = rest.strip_prefix(card)?;
    match after.strip_prefix(':') {
        None if after.is_empty() => Some(*does),
        Some(seat) if !seat.is_empty() && !seat.contains(':') => match does {
            Does::Contends | Does::Judges => Some(*does),
            _ => Some(Does::Looks),
        },
        _ => None,
    }
}

/// Which of several pieces of work on `card` a turn of `thread` did,
/// where it did one: what follows the card where it contends.
pub fn way(thread: &str, card: &str) -> Option<String> {
    let rest = thread.strip_prefix("contend:")?.strip_prefix(card)?.strip_prefix(':')?;
    (does(thread, card) == Some(Does::Contends)).then(|| rest.to_string())
}

/// One turn of the card's, as plain values.
#[derive(Clone, Debug, PartialEq)]
pub struct Turn {
    /// The turn's own id.
    pub id: String,
    /// The agent that took it.
    pub agent: String,
    pub does: Does,
    /// How it stands, as the turn's row says (`done`, `failed`, …).
    pub status: String,
    /// What it chose, where it was given words to end on; nothing where
    /// whoever reads may not be told it.
    pub chose: Option<String>,
    /// Tokens sent to its program and written by it, as the program said.
    pub sent: u64,
    pub wrote: u64,
    pub cost_usd: f64,
    /// Whether its program said what it cost: nought where it did not is
    /// no cost, and is said to be not reported.
    pub cost_said: bool,
    /// When it was asked for, in milliseconds since 1970: shown, and
    /// never what places it.
    pub asked: i64,
    /// Which turn of all it is, in the order turns were written.
    pub nth: i64,
    /// The number of the event its asking names ([`begun_by`]), where
    /// its row says one.
    pub begun: Option<u64>,
    /// How long its program was at work, where it began and ended.
    pub took_ms: Option<i64>,
    /// That what it was asked and said is kept from whoever reads.
    pub kept_from: bool,
    /// Which of several pieces of work on the card it did, where it did
    /// one ([`way`]).
    pub way: Option<String>,
}

impl Turn {
    /// Whether its program said anything of what it took.
    pub fn says(&self) -> bool {
        self.sent > 0 || self.wrote > 0 || (self.cost_said && self.cost_usd > 0.0)
    }
}

/// A line of the card's history, as plain values: what was said, when,
/// by whom, what the line says in the board's own words, and what the
/// event carried.
#[derive(Clone, Debug, PartialEq)]
pub struct Said {
    /// Where its event is in the journal.
    pub seq: u64,
    pub topic: String,
    pub ts: i64,
    pub who: String,
    pub said: String,
    pub data: Value,
}

/// What was decided at the end of a round.
#[derive(Clone, Debug, PartialEq)]
pub struct Decided {
    pub came: Came,
    /// Who decided it, as the history names them.
    pub by: String,
    /// What was said with it: why it was sent back, what the approval
    /// said, why it was left.
    pub words: String,
    /// The line of the history that says it, in the board's words.
    pub said: String,
    /// When, in milliseconds since 1970: shown, and never what places a turn.
    pub at: i64,
    /// Where the line that says it is in the journal.
    pub seq: u64,
}

impl Decided {
    fn open() -> Decided {
        Decided {
            came: Came::Open,
            by: String::new(),
            words: String::new(),
            said: String::new(),
            at: 0,
            seq: 0,
        }
    }
}

/// What some turns come to, added up: counted, never scored.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Spent {
    pub turns: usize,
    pub sent: u64,
    pub wrote: u64,
    pub cost_usd: f64,
    /// How many of them said nothing of what they cost, whose nought is
    /// not in `cost_usd` as a cost: counted, never estimated.
    pub cost_unsaid: usize,
    /// How long their programs were at work, of those that began and ended.
    pub took_ms: i64,
    /// How many of them said nothing of what they took.
    pub unsaid: usize,
}

impl Spent {
    fn of<'a>(turns: impl IntoIterator<Item = &'a Turn>) -> Spent {
        let mut all = Spent::default();
        for turn in turns {
            all.turns += 1;
            all.sent += turn.sent;
            all.wrote += turn.wrote;
            match turn.cost_said {
                true => all.cost_usd += turn.cost_usd.max(0.0),
                // One in hand has not said yet.
                false if turn.status == "done" || turn.status == "failed" => all.cost_unsaid += 1,
                false => {}
            }
            all.took_ms += turn.took_ms.unwrap_or(0).max(0);
            all.unsaid += usize::from(!turn.says());
        }
        all
    }
}

/// One round: its turns in the order they were written, what they came
/// to, and what was decided at its end.
#[derive(Clone, Debug, PartialEq)]
pub struct Round {
    /// Which round it is, from 1.
    pub n: usize,
    pub turns: Vec<Turn>,
    pub spent: Spent,
    pub decided: Decided,
    /// How many of its turns were put in it by the order they were
    /// written, the card's turns saying nothing of what began them.
    pub by_order: usize,
}

fn text(data: &Value, key: &str) -> String {
    data.get(key).and_then(Value::as_str).map(str::trim).unwrap_or_default().to_string()
}

/// What was decided of the card, in the order its history says it:
/// `columns` are the board's, left to right; `said`, its history in the
/// order it was kept.
pub fn decided(columns: &[String], said: &[Said]) -> Vec<Decided> {
    let at = |column: &str| columns.iter().position(|one| one == column);
    let mut all = Vec::new();
    // Whether the work of the round under way was handed on.
    let mut handed = false;
    for line in said {
        let came = |came: Came, by: String, words: String| Decided {
            came,
            by,
            words,
            said: line.said.clone(),
            at: line.ts,
            seq: line.seq,
        };
        let decision = match line.topic.as_str() {
            "card.worked" => {
                handed = true;
                None
            }
            "card.approved" => Some(came(Came::Passed, text(&line.data, "by"), text(&line.data, "words"))),
            "card.left" => Some(came(Came::Left, line.who.clone(), text(&line.data, "why"))),
            "board.item.moved" if handed => {
                let (from, to) = (at(&text(&line.data, "from")), at(&text(&line.data, "to")));
                let last = columns.len().checked_sub(1);
                match (from, to) {
                    (Some(from), Some(to)) if to < from => Some(Came::Back),
                    (_, Some(to)) if Some(to) == last => Some(Came::Passed),
                    _ => None,
                }
                .map(|was| came(was, line.who.clone(), text(&line.data, "note")))
            }
            _ => None,
        };
        if let Some(decision) = decision {
            handed = false;
            all.push(decision);
        }
    }
    all
}

/// The number of the event an asking names, from what a turn's row
/// says of it: its word (`asking`, before any `:<seat>`), where that is
/// the number or ends `-<number>`; else the last name of what asked
/// (`asked_by`, its names after the last `/`), where that ends so. A
/// worker's word is the number itself, a plan's `round-<number>`, and a
/// seat's is the review's instance, which says nothing of it, while the
/// review's name does (`review-<card>-<number>`).
pub fn begun_by(asking: Option<&str>, asked_by: Option<&str>) -> Option<u64> {
    let number = |word: &str| {
        let tail = word.rsplit_once('-').map_or(word, |(_, tail)| tail);
        (!tail.is_empty() && tail.bytes().all(|byte| byte.is_ascii_digit()))
            .then(|| tail.parse().ok())
            .flatten()
    };
    let word = asking.map(|asking| asking.split(':').next().unwrap_or_default());
    word.and_then(number).or_else(|| asked_by.and_then(|by| by.rsplit('/').next()).and_then(number))
}

/// The card's rounds: each decision in its history with the turns of
/// the askings begun before it was said and after the one before, and a
/// round still under way for those after the last. `turns` are in any
/// order; each round has its own in the order they were written. Which
/// were put in a round by the order they were written, no turn of the
/// card saying what began it, is `by_order` of the round.
pub fn rounds(columns: &[String], said: &[Said], mut turns: Vec<Turn>) -> Vec<Round> {
    turns.sort_by_key(|turn| turn.nth);
    let decisions = decided(columns, said);
    // Which round each turn is of, by index; one past the decisions is the round under way.
    let worded = turns.iter().any(|turn| turn.begun.is_some());
    let places: Vec<usize> = if worded {
        let of = |begun: u64| {
            decisions.iter().position(|decision| begun < decision.seq).unwrap_or(decisions.len())
        };
        let known: Vec<Option<usize>> = turns.iter().map(|turn| turn.begun.map(of)).collect();
        (0..turns.len())
            .map(|at| {
                let before = known[..at].iter().rev().find_map(|place| *place);
                let after = known[at..].iter().find_map(|place| *place);
                known[at].or(before).or(after).unwrap_or(decisions.len())
            })
            .collect()
    } else {
        let mut round = 0;
        let mut looked = false;
        turns
            .iter()
            .map(|turn| {
                if turn.does != Does::Looks && looked {
                    round += 1;
                }
                looked = turn.does == Does::Looks;
                round.min(decisions.len())
            })
            .collect()
    };
    let mut all: Vec<Round> = decisions
        .into_iter()
        .enumerate()
        .map(|(index, decided)| Round {
            n: index + 1,
            turns: Vec::new(),
            spent: Spent::default(),
            decided,
            by_order: 0,
        })
        .collect();
    for (turn, place) in turns.into_iter().zip(places) {
        if place == all.len() {
            let n = all.len() + 1;
            all.push(Round {
                n,
                turns: Vec::new(),
                spent: Spent::default(),
                decided: Decided::open(),
                by_order: 0,
            });
        }
        all[place].by_order += usize::from(!worded);
        all[place].turns.push(turn);
    }
    for round in &mut all {
        round.spent = Spent::of(&round.turns);
    }
    all
}

fn rounds_by_order(rounds: &[Value]) -> usize {
    rounds.iter().map(|round| round["by_order"].as_u64().unwrap_or(0) as usize).sum()
}

fn spent_json(spent: &Spent) -> Value {
    json!({
        "turns": spent.turns, "sent": spent.sent, "wrote": spent.wrote, "cost_usd": spent.cost_usd,
        "cost_unsaid": spent.cost_unsaid, "took_ms": spent.took_ms, "unsaid": spent.unsaid,
    })
}

/// What a card cost, laid out once for whoever writes it down (the
/// command line and the client): its rounds, each with its turns, what
/// they came to and what was decided, and what all of them came to.
pub fn laid(card: &str, rounds: &[Round]) -> Value {
    let all = Spent::of(rounds.iter().flat_map(|round| &round.turns));
    let rounds: Vec<Value> = rounds
        .iter()
        .map(|round| {
            let turns: Vec<Value> = round
                .turns
                .iter()
                .map(|turn| {
                    let mut one = json!({
                        "id": turn.id, "agent": turn.agent, "does": turn.does.word(), "status": turn.status,
                        "chose": turn.chose, "sent": turn.sent, "wrote": turn.wrote,
                        "cost_usd": turn.cost_usd, "cost_said": turn.cost_said, "asked": turn.asked, "took_ms": turn.took_ms,
                        "says": turn.says(),
                    });
                    if turn.kept_from {
                        one["kept_from"] = json!(true);
                    }
                    if let Some(way) = &turn.way {
                        one["way"] = json!(way);
                    }
                    one
                })
                .collect();
            let decided = &round.decided;
            let mut came = json!({ "came": decided.came.word() });
            if decided.came != Came::Open {
                came["by"] = json!(decided.by);
                came["words"] = json!(decided.words);
                came["said"] = json!(decided.said);
                came["at"] = json!(decided.at);
            }
            json!({
                "n": round.n, "turns": turns, "spent": spent_json(&round.spent), "decided": came,
                "by_order": round.by_order,
            })
        })
        .collect();
    let by_order: usize = rounds_by_order(&rounds);
    json!({ "card": card, "rounds": rounds, "spent": spent_json(&all), "by_order": by_order })
}

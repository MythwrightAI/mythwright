//! Which of several agents with as little in hand as each other work
//! goes to: a word whoever gives the work says, and where each agent
//! comes by it. A function of plain values, which `agents` asks as it
//! chooses (`agents::Order`, where it always was).

use kernel::Result;

/// Which of several agents that could each be given a piece of work it
/// goes to: a word whoever gives the work says. An agent does one thing
/// at a time, so whichever has least in hand always comes first; this is
/// what tells those apart who have as little as each other.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Order {
    /// The one that was brought up first. What is done where nothing is said.
    #[default]
    First,
    /// Each in turn: the one that was given work longest ago, and before
    /// any of those one that was never given any.
    Turn,
    /// The one that has been given the least so far.
    Least,
    /// The one ranked highest by whoever brought it up (`Slot::rank`).
    Rank,
}

impl Order {
    /// The words there are, as they are written.
    pub const WORDS: &'static [&'static str] = &["first", "turn", "least", "rank"];

    /// The order a word names. Nothing said is the first.
    pub fn named(word: &str) -> Result<Order> {
        Ok(match word.trim() {
            "" | "first" => Order::First,
            "turn" => Order::Turn,
            "least" => Order::Least,
            "rank" => Order::Rank,
            other => {
                return Err(kernel::Error::invalid(format!(
                    "`{other}` is not a way to choose between agents; there are: `first` (the one brought up \
                     first), `turn` (each in turn), `least` (the one given the least so far) and `rank` (the one \
                     ranked highest)"
                )));
            }
        })
    }

    /// Where an agent comes among those with as little in hand: the lower
    /// first. `given` is how many turns it has been given and which was
    /// the last, counted from the first there ever was.
    pub fn place(self, rank: i64, given: (i64, i64)) -> i64 {
        match self {
            Order::First => 0,
            Order::Turn => given.1,
            Order::Least => given.0,
            Order::Rank => rank.saturating_neg(),
        }
    }
}

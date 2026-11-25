//! A window of a view read from rows in hand: what a table kept anywhere
//! but the database is read by, since it is read whole where it is kept,
//! and what the statements that read one kept there are held to.

use serde_json::{Map, Value};

use super::{Held, Order, Place, shows};

/// Where a window is read from.
#[derive(Clone, Debug, PartialEq)]
pub enum From {
    /// The first rows of the view.
    Start,
    /// The rows after a place.
    After(Place),
    /// The rows before a place.
    Before(Place),
    /// The rows from a place on, the row there among them.
    Of(Place),
    /// The rows from the one standing at this rank, the first being 0.
    At(u64),
}

/// A window of a view.
#[derive(Clone, Debug, PartialEq)]
pub struct Window {
    /// The rows, each by where it was among those handed in, in order.
    pub rows: Vec<usize>,
    /// Where each stands.
    pub places: Vec<Place>,
    /// Where to read on from, where there is more after.
    pub after: Option<Place>,
    /// Where to read back from, where there is more before.
    pub before: Option<Place>,
    /// Where the first row stands, where it was read by its rank.
    pub rank: Option<u64>,
}

/// A window of at most `limit` rows of `records` (each a number and what
/// its row holds) as the view held to `held` and read in `order` shows
/// them, read `from` where said.
pub fn window(
    order: &Order,
    held: &[Held],
    records: &[(i64, &Map<String, Value>)],
    from: &From,
    limit: usize,
) -> Window {
    let mut shown: Vec<(usize, Place)> = records
        .iter()
        .enumerate()
        .filter(|(_, (_, data))| shows(held, data))
        .map(|(at, (number, data))| (at, order.place(data, *number)))
        .collect();
    shown.sort_by(|(_, one), (_, two)| order.cmp(one, two));
    let (start, end, rank) = match from {
        From::Start => (0, limit.min(shown.len()), None),
        From::After(place) => {
            let start = shown.partition_point(|(_, one)| order.cmp(one, place).is_le());
            (start, (start + limit).min(shown.len()), None)
        }
        From::Of(place) => {
            let start = shown.partition_point(|(_, one)| order.cmp(one, place).is_lt());
            (start, (start + limit).min(shown.len()), None)
        }
        From::Before(place) => {
            let end = shown.partition_point(|(_, one)| order.cmp(one, place).is_lt());
            (end.saturating_sub(limit), end, None)
        }
        From::At(rank) => {
            let start = (*rank as usize).min(shown.len());
            (start, (start + limit).min(shown.len()), Some(*rank))
        }
    };
    let taken = &shown[start..end];
    Window {
        rows: taken.iter().map(|(at, _)| *at).collect(),
        places: taken.iter().map(|(_, place)| place.clone()).collect(),
        after: (end < shown.len()).then(|| taken.last().map(|(_, place)| place.clone())).flatten(),
        before: (start > 0).then(|| taken.first().map(|(_, place)| place.clone())).flatten(),
        rank,
    }
}

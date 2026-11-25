//! The rule for who work goes to, of those that could take it: a
//! function of the rule's text and plain values, which is `deciding`'s
//! (`deciding::dispatch`, offered here under the name it always had), and
//! who is offered to it, made here from those who could take the work.

pub use deciding::dispatch::{Asked, Offered, RULE, ask};

use crate::picking::Candidate;

/// One who could take the work, as the rule is shown them.
pub fn offered(one: &Candidate) -> Offered {
    Offered {
        name: one.slot.name.clone(),
        role: one.slot.role.clone(),
        rank: one.slot.rank,
        model: one.slot.model.clone().unwrap_or_default(),
        adapter: one.slot.adapter.clone(),
        in_hand: one.in_hand,
        given: one.given,
        last: one.last,
        apart: one.apart,
    }
}

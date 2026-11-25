//! Where people and agents talk.
//!
//! Channels, who is in each, what was said there and what is attached to
//! it ([`channels`], [`attachments`]); the one bot that says what the
//! factory's blueprints say ([`speaker`]); what a bot with a command
//! answers ([`answered`]); what the factory's bot does when it is spoken
//! to, by asking the daemon's routes as whoever spoke ([`bidden`]); and
//! what a blueprint can do in a channel. A
//! conversation is the factory's and no project's thing, and nothing
//! here knows a table, a board or a project's files.
//!
//! It came out of what people and agents work in (`studio`) when that
//! reached the most lines a crate may have: it was a part that asked
//! nothing of the rest. `studio` still opens it, and offers each thing
//! under the name it always gave (`studio::Channels`, `studio::channels`,
//! `studio::speaker`, `Studio::channels`). Its actions are added to a
//! registry by the studio, among its own and where they always were
//! ([`register`], [`answered::register`]), and its tables are still
//! listed with the studio's (`studio::copy`, `studio::apart`), which is
//! where a copy of a factory and the move of a project read them.

mod actions;
pub mod answered;
pub mod archived;
mod asking;
pub mod attachments;
pub mod bidden;
pub mod channels;
pub mod commanded;
pub mod commented;
pub mod conversed;
pub mod discussed;
pub mod docs;
pub mod door;
pub mod events;
pub mod followed;
pub mod following;
pub mod inbox;
pub mod inboxed;
mod joined;
pub mod kept;
pub mod managed;
mod posted;
mod reading;
pub mod renamed;
pub mod speaker;
pub mod taken;
pub mod telling;
pub mod trashed;

use conductor::Registry;

pub use archived::Archived;
pub use attachments::{Attachment, Attachments, Posted};
pub use channels::{Channel, Channels, Member, Message, Place, Placing};
pub use conversed::Conversing;
pub use door::Asked;
pub use inbox::Inbox;
pub use kept::Reader;
pub use speaker::{SaidBy, Speaker};

/// The names the tables kept here are migrated under, each with how
/// many steps this build knows of it: four lines of `studio::copy::KNOWS`,
/// which is what a copy of a factory is held to.
pub const KNOWS: [(&str, usize); 4] = [
    ("attachments", attachments::SCHEMA.len()),
    ("channels", channels::SCHEMA.len()),
    ("inbox", inbox::SCHEMA.len()),
    ("follows", following::SCHEMA.len()),
];

/// Adds what a blueprint can do in a channel (`channel.ensure`,
/// `channel.post`, `channel.join`, `channel.leave`) and what it tells
/// somebody (`inbox.tell`) to `registry`: asked
/// by the studio where its own actions are added, so that they are
/// listed where they always were.
pub fn register(registry: &mut Registry, channels: &Channels) {
    actions::register(registry, channels);
}

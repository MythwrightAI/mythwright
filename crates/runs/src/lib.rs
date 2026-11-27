//! What the runner keeps on disk, and how it is written.
//!
//! The conductor runs blueprints, and what makes it restartable is that
//! everything it does is written down first: an instance with the text it
//! runs and what it keeps, each run of its steps or of a handler, each
//! step, how far a loop or a list of things done around a step has come,
//! and where each handler has got to in the journal. Those rows are this
//! crate's: the tables and what they have become ([`Store::open`]), the
//! shapes they are read as ([`InstanceInfo`], [`RunInfo`], [`StepInfo`]),
//! and every write and question the runner has of them ([`Store`]).
//!
//! What is written down and what is said of it are one write
//! ([`journal::Journal::write`]): whoever writes is handed the events
//! made ready ([`to_say`]) and says them in the transaction, so that a
//! conductor stopped at any instant leaves both or neither.
//!
//! Every row says whose it is, or is reached from one that does
//! ([`apart`]): a project is to have a journal and a conductor of its
//! own, and its rows are to be moved there and asked for by project, not
//! worked out. And where a handler has got to among the events is a
//! place in one journal, kept with what that journal is called
//! ([`Place`]).
//!
//! Nothing here runs anything. There is no engine in it, no instance in
//! hand, no registry of actions and no handler: it stands on the journal,
//! and the conductor stands on it. Where a write has to ask something of
//! what is in hand, the conductor hands it the question as a plain thing
//! ([`Becomes::still`]), and where it has to know a word of the runner's
//! (which run carries a text over), it is told the word.
//!
//! Beside the rows is what a run holds back from them until its next
//! write ([`held`]), which came out of the runner as a rule over these
//! shapes alone; and the runner's machinery that holds no engine once
//! it is handed what begins a task of the conductor's ([`Hands`]): an
//! event got to the handlers it is for ([`delivery`]), turns taken by key
//! ([`turns`]) and work done whole once it is begun ([`whole`]). And
//! two things of plain values the runner asks: which stops are under way
//! ([`stops`]), and what the steps of a run left under way come to
//! ([`left`]).

pub mod apart;
pub mod beside;
pub mod copy;
pub mod delivery;
mod hands;
pub mod held;
mod holders;
mod instances;
pub mod left;
pub mod moved;
mod places;
mod read;
mod runs;
mod schema;
pub mod stops;
mod store;
mod together;
pub mod turns;
pub mod whole;

pub use hands::Hands;
pub use places::Place;
pub use schema::SMALL;
pub use store::{
    Below, Definition, InstanceFilter, InstanceInfo, InstanceStatus, LOOKED_UP, RunInfo, RunStatus, Started,
    StepInfo, StepStatus, Store, Taken, Version, blueprint_kept,
};
pub use together::{Becomes, Done, Ending, Say, to_say};

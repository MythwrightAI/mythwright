//! A sandbox test: a blueprint run in a factory of its own.
//!
//! Somebody who did not write a blueprint wants to see what it does
//! before it does it. A sandbox test is that: the text applied, events
//! said to it, cards moved, the clock moved on, and everything that
//! came of it read back as a real run is read. What it must never be is
//! a way to touch the factory in use, so a sandbox is made of nothing
//! of that factory, and this crate is where that is held:
//!
//! - **A journal in memory**, its projects' beside it in memory too,
//!   on a clock that only the sandbox moves ([`kernel::ManualClock`]).
//!   Nothing of it is written where a later program would find it.
//! - **A conductor of its own** on that journal, on a thread of the
//!   sandbox's own with a runtime nothing else runs on ([`Sandbox`]):
//!   what a sandbox starts is ended with the thread, and one that goes
//!   round for ever takes no time from the daemon's own work.
//! - **A registry built here**, never the daemon's ([`offered`]). An
//!   action in it is one of three kinds. The language's own and the
//!   blocks that only work something out are the real ones. What keeps
//!   rows in the factory's tables (a table declared, a card, a row, a
//!   channel and what is said in it) is the real code, made over the
//!   sandbox's journal ([`studio::Studio::open_within`]), so what it
//!   writes is the sandbox's. Everything else there is on the device (a
//!   program started, an agent asked, a repository, a project's file,
//!   another device, a copy) is a **stand-in** under the same name and
//!   description, which writes down what it was asked and answers what
//!   it was told to ([`stood`]). A stand-in runs nothing, opens nothing
//!   and reads nothing. What answers for such an action is chosen in
//!   one place as the sandbox is made ([`outside`]); today it is always
//!   the stand-in.
//! - **One folder**, which whoever makes the sandbox says: a project
//!   is said to be somewhere, and a step's lines are written to a file.
//!   Every table is kept in the journal: one kept as files is refused,
//!   as one kept by a store service is. It is removed as the sandbox
//!   ends.
//!
//! What a sandbox is handed to be made is plain values and one thing
//! more ([`Made`]): the text, a project's own texts as they were read,
//! what each action and event there is says of itself, and the
//! blueprints that ship, which are only ever read. It is handed no
//! journal, no studio, no agents, no hosting, no gateway and no key,
//! and this crate depends on none of the crates it would have to name
//! to hold one (but `hosting`, for two blocks that work something out,
//! and `studio`, whose tables it makes anew): a test reads this source
//! for a use of anything it must not hold.
//!
//! What a sandbox test shows is what the text does with what it is
//! given. It does not show what the things stood in would have done:
//! nothing was built, nobody was asked, nothing was sent. Every answer
//! says so.

mod at;
mod inside;
pub mod kept;
pub mod offered;
pub mod outside;
pub mod played;
mod rest;
mod shown;
pub mod stood;
mod thread;

use std::path::PathBuf;
use std::sync::Arc;

use blueprint::{ActionSpec, EventSpec};
use conductor::Library;
use kernel::Fields;
use serde::Deserialize;
use serde_json::Value;

pub use offered::Kind;
pub use stood::Answers;
pub use thread::{Sandbox, Seen};

/// What every project a sandbox has is called: the one made-up project it
/// starts with.
pub const PROJECT: &str = "sandbox";

/// How many of anything a sandbox lists: more than its limits let it make.
pub(crate) const MOST: usize = 100_000;

/// What is said with everything a sandbox answers.
pub const SAID: &str = "This was a sandbox test: the blueprint was run in a factory of its own, made for it, and \
                        nothing here touched the factory in use. What reaches outside the factory's tables (a \
                        program, an agent, a repository, a file, another device) was stood in and not exercised, so \
                        passing here is not passing for real.";

/// What a sandbox is made from.
pub struct Made {
    /// What it is found by.
    pub id: String,
    /// What the blueprint is called: what its text says, or the name it
    /// was asked for by.
    pub name: String,
    /// The text being run.
    pub source: String,
    /// A project's own texts it may call, each by its name, as they
    /// were read when the sandbox was made.
    pub own: Vec<(String, String)>,
    /// The blueprints that ship, to be read and nothing else.
    pub ships: Arc<dyn Library>,
    /// What each action there is on the device says of itself.
    pub actions: Vec<ActionSpec>,
    /// What each event there is on the device says of itself, and
    /// whether that is all of them.
    pub events: Vec<EventSpec>,
    pub knows_events: bool,
    /// The sandbox's own folder: made as the sandbox is, removed as it ends.
    pub folder: PathBuf,
    /// The moment its clock starts at, in milliseconds since 1970.
    pub now_ms: i64,
    /// Whether a stand-in waits to be answered or answers at once.
    pub answers: Answers,
    /// What a stand-in answers each time it is asked, by its action's
    /// name, where whoever tries said so beforehand.
    pub told: Fields,
    pub limits: Limits,
}

/// How much a sandbox may come to before it is stopped.
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    /// How many steps it may run.
    pub steps: u64,
    /// How many events it may say.
    pub events: u64,
    /// How many bytes it may keep: what is in its journals, and what
    /// was written to the logs of its steps.
    pub bytes: u64,
}

impl Default for Limits {
    fn default() -> Limits {
        Limits { steps: 4_000, events: 20_000, bytes: 8 << 20 }
    }
}

/// Something whoever runs a sandbox test does to its sandbox.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Act {
    /// Apply the text, with what it is told; or another the sandbox can
    /// read, by its name (`blueprint`), which is how what the text needs
    /// to be there before it is put there: a board, before a fleet.
    Apply {
        #[serde(default)]
        blueprint: Option<String>,
        #[serde(default)]
        params: Fields,
        #[serde(default)]
        key: Option<String>,
    },
    /// Say an event: its topic and what it carries.
    Say {
        topic: String,
        #[serde(default)]
        data: Value,
    },
    /// Do something to the tables as the made-up person: one of
    /// [`inside::DOES`], with what the action of that name takes.
    Do {
        action: String,
        #[serde(default)]
        with: Fields,
    },
    /// Move the clock on: `5m`, `2h`.
    Advance { by: String },
    /// Answer a stand-in that waits: what it gives, or why it fails.
    Answer {
        asked: u64,
        #[serde(default)]
        gives: Option<Value>,
        #[serde(default)]
        fails: Option<String>,
    },
    /// Say how stand-ins answer from now on; one that waits is answered
    /// its usual answer where they are to answer at once.
    Answers { answers: Answers },
}

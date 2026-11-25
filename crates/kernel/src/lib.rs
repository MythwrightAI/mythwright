//! Types every layer shares.
//!
//! Nothing here does I/O beyond reading the clock and the system's random
//! source, and asking the system where a path leads (`whole`). Keeping this
//! crate small and stable matters: everything depends on it, so a change
//! here rebuilds the world.

pub mod added;
pub mod addressed;
pub mod anchor;
pub mod approvals;
pub mod asked;
pub mod asking;
pub mod came;
pub mod cancel;
pub mod cards;
pub mod chain;
pub mod clock;
pub mod columns;
pub mod compared;
pub mod contents;
pub mod deepest;
pub mod discussed;
pub mod error;
pub mod event;
pub mod experiments;
pub mod first;
pub mod flags;
pub mod following;
pub mod gathered;
pub mod git;
pub mod given;
pub mod go;
pub mod id;
pub mod inside;
pub mod named;
pub mod outline;
pub mod parts;
pub mod paths;
pub mod place;
pub mod pool;
pub mod pressed;
pub mod principal;
pub mod ranked;
pub mod rows;
pub mod sides;
pub mod sought;
pub mod tickets;
pub mod topics;
pub mod trash;
pub mod walked;
pub mod whole;
pub mod why;
pub mod window;
pub mod worded;

use std::future::Future;
use std::pin::Pin;

pub use cancel::Cancel;
pub use chain::Chain;
pub use clock::{Clock, ManualClock, SystemClock};
pub use error::{Error, ErrorKind, Result};
pub use event::{Event, Source};
pub use principal::{Principal, PrincipalKind};

/// A boxed future. Traits that cross crate boundaries return this instead of
/// being generic or using a proc macro: one concrete type at the seam keeps
/// monomorphised code out of every caller.
/// What the owner a factory makes as it first starts is called until they
/// say what they are called (`people::opened`): a placeholder, and nothing
/// of the machine or of whoever runs it. The workspace made with them is
/// called `Unnamed workspace` meanwhile (`workspaces::called_after`).
pub const UNNAMED: &str = "Unnamed";

pub type BoxFut<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// JSON object, the shape of most payloads.
pub type Fields = serde_json::Map<String, serde_json::Value>;

/// Builds a [`Fields`] from a `json!` object literal.
pub fn fields(value: serde_json::Value) -> Fields {
    match value {
        serde_json::Value::Object(map) => map,
        _ => Fields::new(),
    }
}

//! The blueprint language.
//!
//! A blueprint is a text file that describes a piece of work as a graph of
//! steps, plus what to do when things happen around it. This crate turns the
//! text into a checked [`Blueprint`] and a drawable [`Graph`]. It does no I/O
//! and knows nothing about processes, agents or networks; the conductor
//! supplies those through a [`Catalog`] of actions.
//!
//! ```text
//! text --syntax--> tree --lower--> model --validate--> Blueprint
//!                                                         |
//!                                                         +--graph--> Graph
//! ```
//!
//! What the language's own words are is said once, in [`vocabulary`]: the
//! reader refuses what is not there and an editor offers what is.

pub mod address;
// What is read alone came out into a crate of its own (`reader`), and is
// offered here under the names it always had.
pub use reader::{at, diag, duration, expr, plain, syntax, template};
pub mod came;
pub mod catalog;
pub mod differs;
pub mod events;
pub mod format;
pub mod graph;
mod heard;
pub mod inside;
pub mod labels;
mod layout;
mod lower;
pub mod model;
pub mod names;
mod outset;
pub mod parts;
mod placed;
mod renamed;
pub mod schema;
pub mod secrets;
pub mod summary;
pub mod types;
pub mod unheard;
pub mod used;
pub mod validate;
pub mod vocabulary;

pub use address::Address;
pub use catalog::{
    ActionList, ActionSpec, ArgSpec, Catalog, Makes, Modelled, NoCatalog, intrinsic, intrinsics,
};
pub use diag::{Diagnostic, Severity};
pub use events::{EventList, EventSpec, FieldSpec, Said};
pub use format::{Settings, Unformatted, format, meaning, means_the_same};
pub use graph::{Graph, graph};
pub use heard::HEARD;
pub use inside::Within;
pub use labels::Labels;
pub use lower::MIN_POLL_MS;
pub use model::*;
pub use outset::{outset, version};
pub use syntax::{Pos, Span};
pub use template::{Cond, Template};
pub use types::ValueType;
pub use validate::{DECISIONS, RESERVED_TOPICS, grants};
pub use vocabulary::ROOTS;

/// The outcome of checking a blueprint's text.
#[derive(Clone, Debug)]
pub struct Checked {
    /// Present whenever the text was well formed enough to have a shape, even
    /// if it has errors, so an editor can still draw a half-written graph.
    pub blueprint: Option<Blueprint>,
    pub diagnostics: Vec<Diagnostic>,
}

impl Checked {
    pub fn has_errors(&self) -> bool {
        self.diagnostics.iter().any(Diagnostic::is_error)
    }

    pub fn errors(&self) -> impl Iterator<Item = &Diagnostic> {
        self.diagnostics.iter().filter(|diagnostic| diagnostic.is_error())
    }

    /// The blueprint, if it is fit to run.
    pub fn into_result(self) -> Result<Blueprint, Vec<Diagnostic>> {
        match self.blueprint {
            Some(blueprint) if !self.diagnostics.iter().any(Diagnostic::is_error) => Ok(blueprint),
            _ => Err(self.diagnostics),
        }
    }
}

/// Parses and checks `source` against `catalog`.
pub fn check(source: &str, catalog: &dyn Catalog) -> Checked {
    let tree = match syntax::parse(source) {
        Ok(tree) => tree,
        Err(err) => {
            return Checked {
                blueprint: None,
                diagnostics: vec![Diagnostic::error("syntax", err.message, err.span)],
            };
        }
    };
    let (blueprint, mut diagnostics) = lower::lower(&tree);
    if let Some(blueprint) = &blueprint {
        validate::validate(source, blueprint, catalog, &mut diagnostics);
        renamed::told(source, &mut diagnostics);
    }
    diagnostics.sort_by_key(|d| (d.span.start.line, d.span.start.col, !d.is_error()));
    Checked { blueprint, diagnostics }
}

/// A short, stable identifier for this exact text. Two instances with the same
/// revision are running the same blueprint.
pub fn revision(source: &str) -> String {
    // FNV-1a: tiny, dependency-free and stable across builds, which is all a
    // revision id needs. It is not a security boundary.
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in source.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}")
}

/// Renders diagnostics the way a compiler would, one per line.
pub fn render_diagnostics(file: &str, diagnostics: &[Diagnostic]) -> String {
    diagnostics.iter().map(|diagnostic| diagnostic.render(file)).collect::<Vec<_>>().join("\n")
}

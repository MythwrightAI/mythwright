//! Where a table is kept: a project's, or the factory's own.
//!
//! A table is always somebody's place. Most are a project's, known by the
//! project's id; a few are the factory's, for whatever is no one
//! project's thing (a forum's topics, which are a workspace's or a
//! project's and are moved between the two as a channel is). Those are
//! kept at a place of the factory's own, [`FACTORY`], which no project's
//! id can be: a project's id is a slug, of letters, digits and dashes.
//!
//! Its records are kept in the factory's own journal, as a project's are
//! where the project has no journal of its own, and what is said of them
//! carries `project: "@"`, which keeps it there too. Whose such an event
//! is, to whatever hears events by project, is nobody's: a handler of
//! any project hears it, as it hears a workspace's channel's messages,
//! and narrows by what the event names.

/// The factory's own place for tables: no project's id is this.
pub const FACTORY: &str = "@";

/// Whether `place` is the factory's own and no project's.
pub fn is_factory(place: &str) -> bool {
    place == FACTORY
}

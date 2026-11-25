//! The blueprint language as it is read.
//!
//! What is here asks nothing of the rest of the language: the text read
//! into a tree with where each thing is ([`syntax`]), what is said of
//! something wrong in it ([`diag`]), and what is written inside its
//! values: an expression in `${ ... }` ([`expr`]), text with such holes
//! in it ([`template`]), a duration ([`duration`]) and a time of day a
//! handler fires at ([`at`]), with whether what a text works out for
//! itself is plain ([`plain`]).
//!
//! It came out of `blueprint` when that crate was at its ceiling, and is
//! offered there under the names it always had (`blueprint::syntax`,
//! `blueprint::expr`, ...): whatever reads the language names it there.
//! Something new that is a function of the text alone, or of what is
//! written in one value, and asks nothing of a checked blueprint, goes
//! here.

pub mod at;
pub mod diag;
pub mod duration;
pub mod expr;
pub mod plain;
pub mod syntax;
pub mod template;

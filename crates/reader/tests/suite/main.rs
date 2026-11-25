//! One test binary for the crate. Add a module here rather than a new file
//! beside this directory: every extra test target is another link step.
//!
//! These are of what is read alone. What is read and then checked (a
//! blueprint lowered, validated, laid out) is held by `blueprint`'s suite,
//! through the names that crate offers this one's under.

mod at;
mod expr;
mod plain;
mod syntax;

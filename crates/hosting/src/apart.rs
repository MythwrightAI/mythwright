//! Whose each row kept here is.
//!
//! A service and a router are found again by name when a daemon starts,
//! and each says the project it was started for, or none
//! ([`journal::apart`]). They are a handful of rows; the list is here so
//! that whatever tells a factory's rows apart by project is told of
//! every table there is, and a test fails on one with no line.

use journal::apart::Told;

/// Every table kept here, each with how its rows are told apart by
/// project.
pub const TABLES: &[(&str, Told)] = &[
    // The project it was started for; nothing where it is nobody's.
    ("services", Told::Says("project")),
    ("routers", Told::Says("project")),
];

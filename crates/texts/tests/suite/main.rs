//! One test binary for the crate; add modules here.
//!
//! What is here needs no conductor: where texts are found is a function
//! of the folders and the layers it is handed. What a conductor keeps of
//! texts is played where one runs, in the runner's suite (`pinned.rs`,
//! `reached.rs`, `taking.rs`, `current.rs`, `authority.rs`, `linked.rs`),
//! which may use this crate and which this crate may not use.

mod folders;
mod listed;
mod named;

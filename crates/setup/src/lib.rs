//! What a project and an agent are set up with as somebody asks, by the
//! blueprints that ship: what a new project starts with ([`usual`], by
//! `std/project`, whose one action is [`given`], and the setups a factory
//! keeps of its own, [`setups`]), an agent added by hand ([`byhand`]),
//! and an example put into a project ([`examples`]). Each is applied, or written, as whoever asked, exactly
//! as they could apply or write it by hand, so nothing is made here that
//! there is not one other way to come by.
//!
//! It came out of `api` when that crate was at its ceiling, and serves
//! nothing: the routes there ask it, handed what they need of the daemon
//! (the runner, the studio, the agents, where projects keep their own
//! blueprints), and `api` offers it under the names it always had
//! (`api::examples`, `api::usual`, `api::byhand`).

pub mod byhand;
pub mod events;
pub mod examples;
pub mod given;
pub mod setups;
pub mod usual;

pub use given::register;

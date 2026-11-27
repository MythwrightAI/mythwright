//! Who is who.
//!
//! People's accounts and sessions, invitations to register, and the
//! credentials given to keys, agents and devices ([`identity`],
//! [`invited`]), invitations to enrol, for an agent the factory does
//! not start ([`enrolled`]), what a person is given beyond their role
//! ([`given`]), somebody a service outside vouches
//! for ([`outside`]), a service outside that proves each call by
//! signing it ([`callers`]), what somebody is named by after an `@`
//! ([`handle`]), the owner a factory makes as it first starts and the
//! links that sign somebody in once ([`opened`]), the themes somebody makes for the client, which
//! are kept with their account ([`themes`]), and how they arranged the
//! client's side panel, how a time is written for them and how what they
//! read looks, each kept the same way ([`arranged`], [`clocked`],
//! [`appearance`]): which it keeps
//! as text and acts on none of. It stands on the journal and on nothing above it, and
//! knows no project, table or channel.
//!
//! It came out of what people and agents work in (`studio`) when that
//! reached the most lines a crate may have: it was the part that asked
//! nothing of the rest. `studio` still opens it, and offers each thing
//! under the name it always gave (`studio::Identity`, `studio::identity`,
//! `Studio::identity`). Its tables are still listed with the studio's
//! (`studio::copy`, `studio::apart`), which is where a copy of a factory
//! and the move of a project read them.

pub mod appearance;
pub mod arranged;
pub mod callers;
pub mod clocked;
pub mod enrolled;
pub mod events;
pub mod given;
pub mod groups;
pub mod handle;
pub mod hashing;
pub mod identity;
pub mod invited;
pub mod keys;
pub mod opened;
pub mod outside;
pub mod themes;

pub use appearance::Appearance;
pub use arranged::Arranged;
pub use clocked::Clocked;
pub use groups::{Group, Groups};
pub use handle::{handle, held, named};
pub use hashing::Hashing;
pub use identity::{Account, Came, Credential, CredentialKind, Identity, SignedIn, Tier};
pub use keys::Withdrawing;
pub use themes::Themes;

/// The names the tables kept here are migrated under, each with how many
/// steps this build knows of it: six lines of `studio::copy::KNOWS`,
/// which is what a copy of a factory is held to.
pub const KNOWS: [(&str, usize); 6] = [
    ("identity", identity::SCHEMA.len()),
    ("themes", themes::SCHEMA.len()),
    ("arranged", arranged::SCHEMA.len()),
    ("clocked", clocked::SCHEMA.len()),
    ("groups", groups::SCHEMA.len()),
    ("appearance", appearance::SCHEMA.len()),
];

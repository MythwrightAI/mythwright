//! What a blueprint reads of where it runs that somebody said: the
//! variables of the profile (the copy of the factory) it runs in, kept in the factory's journal and
//! held in memory for every conductor of the factory ([`factory`]).
//!
//! It came out of the runner, which offers it under the names it always
//! gave (`conductor::profile`, `Engine::variables`,
//! `Engine::set_variables`) and says what was changed itself: what is
//! kept, and how a change is worked out, is here; what an expression is
//! handed, and what is said of a change, is the runner's.
//!
//! And a project's environments ([`project`]): named sets of variables
//! and secrets, one of which an instance of the project is applied
//! under, kept beside the runner's rows of the project, and held for
//! whatever runs under one ([`held`]), with what is said as one is made,
//! changed or taken away ([`events`]), what each last ran, in words
//! ([`last`]), and what each holds to as something comes to run under
//! it ([`rules`]); and what a project's feature flag says in one of
//! them, its variable written with what is said of it ([`flagged`]).

pub mod events;
pub mod factory;
pub mod flagged;
pub mod held;
pub mod kept;
pub mod last;
pub mod project;
pub mod rules;

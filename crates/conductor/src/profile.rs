//! The profile a blueprint runs in, and the environment it was applied
//! under, as an expression sees them.
//!
//! A **profile** is a copy of the factory: a daemon on a data folder of its
//! own, with a name such as `prod` or `dev1`, or none. It is true of where
//! a blueprint runs and not of what it was started with, so that a
//! blueprint can say "not here" without being told each time. An
//! expression is handed it as `profile`: what it is called is
//! `profile.name`, and what its owner has said is true of it is
//! `profile.vars`, the **variables**, each a name and a plain value (text,
//! a number, true or false), so that one blueprint reads the right address
//! in `prod` and in `dev1` without being edited. One that was not set
//! reads as nothing.
//!
//! An **environment** is a project's: one of the named sets of variables
//! and secrets a project defines (dev, staging, prod), which an instance is
//! applied under. An expression is handed it as `environment`
//! (`environment.name`, `environment.vars`). Until a project can define
//! one, every instance is applied under none, and `environment` is the
//! empty name and no variables. (Before 0.1.0 `environment` was the
//! profile's word: it was renamed before the first release.)
//!
//! Both maps are built here, [`view`] and [`unchosen`]. What is kept and
//! held of the profile's variables is `environments::factory`, which came
//! out of the runner; what is the runner's is what an expression is
//! handed and what is said of a change. The variables are set whole
//! ([`Engine::set_variables`]), which a blueprint asks with the
//! `profile.set` action (`blocks`) and is the owner's to use: what it
//! changes is what every blueprint in the factory reads.
//!
//! They are not for secrets. They are shown to whoever may read
//! blueprints and instances, kept in the database as they are, and may
//! appear in events and in the history of what was applied.

pub(crate) use environments::factory::Held;
pub use environments::factory::{Changes, NAMESPACE};
use kernel::{Fields, Result, Source};
use serde_json::{Value, json};

use crate::engine::Engine;

impl Engine {
    /// Says which profile this factory is, once, before anything runs.
    /// Whoever starts the factory knows; the conductor only passes it on.
    pub fn in_profile(&self, name: impl Into<String>) {
        self.inner.environment.called(name.into());
    }

    /// What this factory's profile is called: empty for one with none.
    pub fn profile(&self) -> &str {
        self.inner.environment.name()
    }

    /// The variables of this factory's profile, as they are now.
    pub fn variables(&self) -> Fields {
        self.inner.environment.vars()
    }

    /// Makes `vars` the variables of this factory's profile, in place of
    /// whatever was set, and says what that changed. They are kept before
    /// they are held, so a factory never reads what it would not read
    /// again after a restart. When something changed it is said once, as
    /// `system.profile`, by name and never by value, while the turn at
    /// changing them is still held; `by` is who is named there beside
    /// `source`.
    pub async fn set_variables(&self, vars: Fields, source: Source, by: &str) -> Result<Changes> {
        let mut changing = self.inner.environment.changing().await;
        let changes = changing.set(&self.inner.factory, vars).await?;
        if changes.is_empty() {
            return Ok(changes);
        }
        let data = json!({
            "set": changes.set, "changed": changes.changed, "removed": changes.removed,
            "by": source.actor, "by_name": by,
        });
        self.inner.factory.emit("system.profile", source, data).await?;
        Ok(changes)
    }
}

/// `profile`, as every expression sees it.
pub(crate) fn view(engine: &Engine) -> Value {
    engine.inner.environment.view()
}

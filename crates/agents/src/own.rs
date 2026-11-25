//! An agent never changes its own slot. What it may ask of the factory,
//! what its program may reach on the device and what it signs in with
//! are set for it by a person, or by an agent that holds what it gives,
//! and refused to the agent itself by
//! every door: its own route (`ensure`, `give`, `move_to`, an
//! invitation), and the row of a fleet's table that brings it up, which
//! the studio asks of what is running here (`own_rows_with`). Nobody
//! gives what they do not hold already holds it below what it holds;
//! this holds it at what it holds too, since a scope that lets it change
//! one thing (`shell`, a wall; `secrets:use`, a key named) would let it
//! hand itself what it was never shown.

use conductor::{Downgraded, InstanceFilter};
use kernel::{Error, Principal, PrincipalKind, Result};
use studio::Studio;

/// Refuses `by` changing the slot `name` where `by` is that agent.
pub(crate) fn not_its_own(name: &str, by: &Principal) -> Result<()> {
    if by.kind != PrincipalKind::Agent || by.id != name {
        return Ok(());
    }
    Err(Error::denied(format!(
        "`{name}` cannot change its own slot, even within what it holds: what an agent may ask of the factory, \
         what its program may reach on the device and what it signs in with are set by a person, or by an \
         agent that holds what it gives. Ask somebody who may change agents"
    )))
}

/// Has the studio ask, of a row an agent changes, whether it is the row
/// that agent is brought up by: one that something running holds it for
/// (`agent:<name>`, which `agent.ensure` writes) and is called after the
/// row, as `real` finds it. The engine is held only as long as it is
/// there: it holds the studio, and would be held by it for good.
pub(crate) fn install(studio: &Studio, engine: Downgraded) {
    studio.collections.own_rows_with(std::sync::Arc::new(move |agent, record| {
        let engine = engine.upgrade();
        Box::pin(async move {
            let Some(engine) = engine else { return false };
            let filter = InstanceFilter { live_only: true, limit: 100_000, ..InstanceFilter::default() };
            // Not asked is not let through.
            let Ok(live) = engine.instances(filter).await else { return true };
            let (hold, after) = (format!("agent:{agent}"), format!("-{record}"));
            // Held by its own text under what counts it (a role, by its
            // row: `crate::removed::Brought`), or by that itself.
            let counting = |one: &conductor::InstanceInfo| match (&one.parent, one.key == agent) {
                (Some(parent), true) => live.iter().find(|above| &above.id == parent).cloned(),
                _ => Some(one.clone()),
            };
            live.iter()
                .filter(|one| one.holds.contains(&hold))
                .filter_map(counting)
                .any(|one| one.key.ends_with(&after))
        })
    }));
}

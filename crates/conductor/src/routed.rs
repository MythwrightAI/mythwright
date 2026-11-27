//! What is asked of the whole factory, answered from every conductor.
//!
//! Each conductor keeps its own rows ([`crate::set`]). What names one
//! instance or one run is answered by whichever keeps it, where it is
//! asked (`Engine::instance`, `stop`, `run`, `wait_run`: its own rows
//! first, which costs a conductor nothing for what is its own, and then
//! [`Engine::elsewhere`]). What is here is what names none: a list over
//! all of them, put together as one conductor's rows would give it.

use kernel::Result;
use runs::{InstanceFilter, InstanceInfo, RunInfo};

use crate::engine::Engine;

/// Instances by what `filter` asks: a parent's, from the conductor that
/// keeps the parent; a project's, from the conductor that runs it; and
/// otherwise from all of them, the newest first.
pub(crate) async fn instances(engine: &Engine, filter: InstanceFilter) -> Result<Vec<InstanceInfo>> {
    if engine.inner.set.alone() {
        return engine.inner.store.instances(filter).await;
    }
    if let Some(parent) = &filter.parent {
        return engine.keeper(parent).await.inner.store.instances(filter).await;
    }
    gathered(of(engine, filter.project.as_deref()).await?, filter).await
}

/// Instances by what `filter` asks, from every conductor as it has them
/// written, waiting for none that is taking up what it keeps: for what
/// says how a daemon stands while it starts, which is answered before
/// its start is over. What names a parent is asked of its keeper, as in
/// [`instances`].
pub(crate) async fn instances_written(engine: &Engine, filter: InstanceFilter) -> Result<Vec<InstanceInfo>> {
    if engine.inner.set.alone() || filter.parent.is_some() {
        return instances(engine, filter).await;
    }
    let every = engine.every().into_iter();
    let theirs = match filter.project.clone() {
        // The factory's, which kept what was run for a project before it
        // had one of its own, and the project's own.
        Some(project) => every
            .filter(|one| one.inner.project.is_none() || one.inner.project == Some(project.clone()))
            .collect(),
        None => every.collect(),
    };
    gathered(theirs, filter).await
}

/// What each of `conductors` has written that `filter` asks for, put
/// together as one conductor's rows would give it.
async fn gathered(conductors: Vec<Engine>, filter: InstanceFilter) -> Result<Vec<InstanceInfo>> {
    let limit = if filter.limit == 0 { 500 } else { filter.limit };
    let mut all = Vec::new();
    for one in conductors {
        all.extend(one.inner.store.instances(filter.clone()).await?);
    }
    // Each conductor's are newest first already, and stay so among
    // those made in one moment.
    all.sort_by_key(|one| std::cmp::Reverse(one.created));
    all.truncate(limit);
    Ok(all)
}

/// The most recent runs: a project's, or of everything.
pub(crate) async fn recent_runs(
    engine: &Engine,
    limit: usize,
    project: Option<String>,
) -> Result<Vec<RunInfo>> {
    if engine.inner.set.alone() {
        return engine.inner.store.recent_runs(limit, project).await;
    }
    let mut all = Vec::new();
    for one in of(engine, project.as_deref()).await? {
        all.extend(one.inner.store.recent_runs(limit, project.clone()).await?);
    }
    all.sort_by_key(|one| std::cmp::Reverse(one.started));
    all.truncate(limit);
    Ok(all)
}

/// The conductors that may keep something of `project`: its own and
/// the factory's, which kept what was run for it before it had one of
/// its own; every one, where no project is named.
async fn of(engine: &Engine, project: Option<&str>) -> Result<Vec<Engine>> {
    let Some(project) = project else { return Ok(engine.every_up().await) };
    let (front, own) = (engine.up(engine.front()).await, engine.of(Some(project)).await?);
    Ok(match std::sync::Arc::ptr_eq(&front.inner, &own.inner) {
        true => vec![front],
        false => vec![front, own],
    })
}

/// Which blueprints are in use where, in every conductor.
pub(crate) async fn in_use(engine: &Engine) -> Result<Vec<(String, String)>> {
    let mut all = Vec::new();
    for one in engine.every_up().await {
        all.extend(one.inner.store.in_use().await?);
    }
    if !engine.inner.set.alone() {
        all.sort();
        all.dedup();
    }
    Ok(all)
}

impl Engine {
    /// Every run under way, of every conductor, without its steps.
    pub async fn under_way(&self) -> Result<Vec<RunInfo>> {
        let mut all = Vec::new();
        for one in self.every_up().await {
            all.extend(one.inner.store.unfinished_runs().await?);
        }
        Ok(all)
    }
}

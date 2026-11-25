//! An environment's life once Run has begun loading it, as events any
//! blueprint can hear, and measured: the run its
//! deploy began ended well (`project.environment.ready`, with where it is
//! reached and the cards it holds, as the blueprint it is loaded by gives
//! them out as `address` and `cards`) or did not (`.failed`), and what it
//! was loaded as was taken down once it was ready (`.down`: stopped, or
//! failed, by whatever ended it). How long a load took and how long it was
//! up are measured (`project.environment.loaded`,
//! `project.environment.up`), by the labels project, environment and
//! branch: counted, and never scored.
//!
//! The runner says what it says of its own runs as they end, and passes
//! most of it on without keeping it, so none of this is worked out from
//! that. Each is said by the environment's keeper in the write that keeps
//! that it was said (`environments::kept::ended`, `down`), once, of the
//! load it is kept as: a run of a load that another took the place of
//! says nothing. What says it is a wait on the run (`Engine::wait_run`,
//! which whatever conductor takes the run up answers), begun at Run and,
//! for a load whose end was never said, as the daemon starts; and for
//! what is taken down, `instance.finished`, which is kept, followed from
//! the start, with every environment looked at again as the daemon starts
//! and wherever the follower fell behind.

use std::collections::HashSet;

use conductor::RunStatus;
use conductor::environs::{Ended, Environment};
use kernel::Source;

use crate::state::App;

/// The waits on loads' runs under way, each once by its project, its
/// environment and its run: ended with the task that keeps them, as a node
/// stops.
#[derive(Default)]
struct Waits {
    under_way: tokio::task::JoinSet<String>,
    waited: HashSet<String>,
}

impl Waits {
    /// Waits, beside everything else, for the run `project`'s environment
    /// `name` was loaded by to end, and says how it came out.
    fn wait(&mut self, app: &App, project: &str, name: &str, run: &str) {
        let key = format!("{project}\n{name}\n{run}");
        if !self.waited.insert(key.clone()) {
            return;
        }
        let (app, project, name, run) = (app.clone(), project.to_string(), name.to_string(), run.to_string());
        self.under_way.spawn(async move {
            if let Err(err) = ended(&app, &project, &name, &run).await {
                degraded(&app, &project, &name, &err.message).await;
            }
            key
        });
    }

    /// Forgets the waits that are over.
    fn over(&mut self, key: Option<Result<String, tokio::task::JoinError>>) {
        if let Some(Ok(key)) = key {
            self.waited.remove(&key);
        }
    }
}

async fn ended(app: &App, project: &str, name: &str, run: &str) -> kernel::Result<()> {
    let read = app.engine.wait_run(run).await?;
    let ok = read.status == RunStatus::Ok;
    let at = read.finished.unwrap_or_else(|| app.journal.now_ms());
    let reason =
        read.error.clone().or_else(|| (!ok).then(|| format!("its run was {}", read.status.as_str())));
    let ended = Ended::of(ok, at, &read.outputs, reason);
    let source = Source::actor("system");
    let (environment, said) = app.engine.environment_ended((project, name), run, ended, source).await?;
    if let (true, Some(ran)) = (said, &environment.ran) {
        let took = (at - ran.at).max(0) as f64;
        let labels = [("project", project), ("environment", name), ("branch", ran.branch.as_str())];
        app.journal.measure("project.environment.loaded", &labels, took, ok);
    }
    Ok(())
}

/// Says that what `environment` of `project` was loaded as was taken down,
/// at `at`, where it is what was ended and it was ready.
async fn down(app: &App, project: &str, environment: &Environment, instance: &str, at: i64) {
    let source = Source::actor("system");
    let named = (project, environment.name.as_str());
    match app.engine.environment_down(named, instance, at, source).await {
        Ok((now, true)) => {
            let Some(ran) = &now.ran else { return };
            let ready = ran.ended.as_ref().map_or(at, |ended| ended.at);
            let labels = [("project", project), ("environment", named.1), ("branch", ran.branch.as_str())];
            app.journal.measure("project.environment.up", &labels, (at - ready).max(0) as f64, true);
        }
        Ok(_) => {}
        Err(err) => degraded(app, project, named.1, &err.message).await,
    }
}

async fn degraded(app: &App, project: &str, name: &str, why: &str) {
    let data = serde_json::json!({
        "what": format!("saying how `{project}`'s environment `{name}` was loaded"),
        "reason": why,
        "effect": "what is said of its load is said late, or not at all",
    });
    let _ = app.journal.emit("system.degraded", Source::actor("system"), data).await;
}

/// Looks at every environment of every project: a load whose end was never
/// said is waited for, and one that was ready and whose instance has ended
/// otherwise than well since is said to be down.
async fn settle(app: &App, waits: &mut Waits) {
    for project in app.studio.projects.list().await.unwrap_or_default() {
        for environment in app.engine.environments_of(&project.id).await.unwrap_or_default() {
            let Some(ran) = &environment.ran else { continue };
            match (&ran.run, &ran.ended) {
                (Some(run), None) => waits.wait(app, &project.id, &environment.name, run),
                (_, Some(ended)) if ended.ready && ran.down.is_none() => {
                    if let Some(at) = taken_down(app, &ran.instance).await {
                        down(app, &project.id, &environment, &ran.instance, at).await;
                    }
                }
                _ => {}
            }
        }
    }
}

/// When `instance` was taken down, where it has ended otherwise than well.
async fn taken_down(app: &App, instance: &str) -> Option<i64> {
    let info = app.engine.instance(instance).await.ok()?;
    let ended = !info.status.is_live() && info.status.as_str() != "done";
    ended.then_some(info.updated)
}

/// Keeps what is said of environments' loads from the start: what a stop
/// left unsaid, then each load as Run begins it (`project.environment.ran`,
/// said in the write that keeps it) and what is taken down as it is.
pub(crate) async fn keep(app: App) {
    let topics = "project.environment.ran,instance.finished";
    let mut heard = app.journal.follow_every(app.journal.now(), topics);
    let mut waits = Waits::default();
    settle(&app, &mut waits).await;
    loop {
        let event = tokio::select! {
            over = waits.under_way.join_next(), if !waits.under_way.is_empty() => {
                waits.over(over);
                continue;
            }
            heard = heard.next_or_behind() => match heard {
                Ok(heard) => heard,
                Err(_) => break,
            },
        };
        let Some(event) = event else {
            settle(&app, &mut waits).await;
            continue;
        };
        if event.topic == "project.environment.ran" {
            let (project, name) = (event.data["project"].as_str(), event.data["environment"].as_str());
            if let (Some(project), Some(name), Some(run)) = (project, name, event.data["run"].as_str()) {
                waits.wait(&app, project, name, run);
            }
            continue;
        }
        // Every instance that ends otherwise than well is asked of, so it is
        // asked of what is held in memory and of no database.
        if event.data["status"] == "done" {
            continue;
        }
        let Some(instance) = event.source.instance.as_deref() else { continue };
        if let Some((project, environment)) = app.engine.environments().up_as(instance) {
            down(&app, &project, &environment, instance, event.ts).await;
        }
    }
}

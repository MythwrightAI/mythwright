//! A project's environments as they are kept, in the journal of the
//! conductor that runs what is applied for the project: read, and each
//! change written with what is said of it, as one write
//! (`project.environment.made`, `.changed`, `.removed`), and then held
//! for what runs under one ([`crate::held::Held`]). What is said names
//! variables and secrets and never carries a value.
//!
//! The runner finds the journal (the conductor that keeps what is the
//! project's) and hands it here with what is held for every conductor;
//! each change holds the turn ([`crate::held::Held::changing`]) from its
//! reading to its writing.

use std::future::Future;

use journal::Journal;
use kernel::{Error, Principal, Result, Source};
use serde_json::json;

use crate::held::Held;
use crate::project::{Changed, Ended, Environment, NAMESPACE, Ran, key, named, read};

/// `project`'s environments, by name, as `journal` keeps them.
pub async fn of(journal: &Journal, project: &str) -> Result<Vec<Environment>> {
    let kept = journal.kv_list(NAMESPACE).await?;
    Ok(kept
        .into_iter()
        .filter(|(at, _)| named(at, project).is_some())
        .filter_map(|(_, kept)| read(kept))
        .collect())
}

/// `project`'s environment called `name`, where `journal` keeps one.
pub async fn one(journal: &Journal, project: &str, name: &str) -> Result<Option<Environment>> {
    Ok(journal.kv_get(NAMESPACE, &key(project, name)).await?.and_then(read))
}

/// What an apply that names none comes to run under, in `project` as
/// `journal` keeps its environments: none where it has none, and refused,
/// naming them, where it has any ([`crate::held::must_choose`]).
pub async fn none_named(journal: &Journal, project: &str) -> Result<String> {
    let names: Vec<String> = of(journal, project).await?.into_iter().map(|held| held.name).collect();
    match names.is_empty() {
        true => Ok(String::new()),
        false => Err(Error::invalid(crate::held::must_choose(project, &names))),
    }
}

/// Makes an environment of `project` called `name`, with nothing in it,
/// and says so, as one write. One of that name there already is refused.
pub async fn make(
    (journal, held): (&Journal, &Held),
    project: &str,
    name: &str,
    by: &Principal,
    source: Source,
) -> Result<Environment> {
    let _turn = held.changing().await;
    if one(journal, project, name).await?.is_some() {
        return Err(Error::conflict(format!("`{project}` has an environment called `{name}` already")));
    }
    let made = Environment::new(name, by, journal.now_ms())?;
    let (at, kept) = (key(project, name), made.kept());
    let said = json!({ "project": project, "environment": name, "by": made.by, "by_name": made.by_name });
    let write = journal.write(move |writing| {
        writing.keep(NAMESPACE, &at, &kept)?;
        writing.say("project.environment.made", source, said)
    });
    write.await?;
    held.put(project, &made);
    Ok(made)
}

/// Changes `project`'s environment called `name` by `change`, and says
/// what that changed, as one write. Where nothing changed nothing is
/// written or said.
pub async fn change(
    (journal, held): (&Journal, &Held),
    project: &str,
    name: &str,
    change: impl FnOnce(&mut Environment) -> Result<Changed>,
    by: &Principal,
    source: Source,
) -> Result<(Environment, Changed)> {
    let _turn = held.changing().await;
    let mut environment = one(journal, project, name).await?.ok_or_else(|| missing(project, name))?;
    let changed = change(&mut environment)?;
    if changed.is_empty() {
        return Ok((environment, changed));
    }
    let (at, kept) = (key(project, name), environment.kept());
    let said = changed_said(project, name, &changed, by);
    let write = journal.write(move |writing| {
        writing.keep(NAMESPACE, &at, &kept)?;
        writing.say("project.environment.changed", source, said)
    });
    write.await?;
    held.put(project, &environment);
    Ok((environment, changed))
}

/// What `project.environment.changed` says of `changed`, by name and
/// never by value.
pub(crate) fn changed_said(
    project: &str,
    name: &str,
    changed: &Changed,
    by: &Principal,
) -> serde_json::Value {
    json!({
        "project": project,
        "environment": name,
        "set": changed.set,
        "changed": changed.changed,
        "removed": changed.removed,
        "marked": changed.marked,
        "secrets": changed.secrets,
        "secrets_removed": changed.secrets_removed,
        "secrets_marked": changed.secrets_marked,
        "deploy": changed.deploy,
        "rules": changed.rules,
        "by": by.actor(),
        "by_name": by.name,
    })
}

/// Keeps what `project`'s environment called `name` was last loaded with,
/// and says so, as one write (`project.environment.ran`). Asked once what
/// it says of it has been started: the run is the record of how it goes.
pub async fn ran(
    (journal, held): (&Journal, &Held),
    project: &str,
    name: &str,
    ran: Ran,
    source: Source,
) -> Result<Environment> {
    let _turn = held.changing().await;
    let mut environment = one(journal, project, name).await?.ok_or_else(|| missing(project, name))?;
    let mut said = json!({
        "project": project,
        "environment": name,
        "branch": ran.branch,
        "commit": ran.commit,
        "blueprint": ran.blueprint,
        "instance": ran.instance,
        "by": ran.by,
        "by_name": ran.by_name,
    });
    if let Some(run) = &ran.run {
        said["run"] = json!(run);
    }
    environment.ran = Some(ran);
    let (at, kept) = (key(project, name), environment.kept());
    let write = journal.write(move |writing| {
        writing.keep(NAMESPACE, &at, &kept)?;
        writing.say("project.environment.ran", source, said)
    });
    write.await?;
    held.put(project, &environment);
    Ok(environment)
}

/// What an environment's life comes to of its run that was said
/// ([`ended`], [`down`]): the environment as it is kept now, and whether
/// this was said, which it is once.
pub type Said = (Environment, bool);

/// Says how the run `project`'s environment called `name` was last loaded
/// by came out (`project.environment.ready`, or `.failed`), and keeps that
/// it was said, as one write: only where `run` is still the run it was last
/// loaded by and nothing was said of it yet. A run of a load that another
/// has taken the place of says nothing: the environment is that one's now.
pub async fn ended(
    (journal, held): (&Journal, &Held),
    (project, name): (&str, &str),
    run: &str,
    ended: Ended,
    source: Source,
) -> Result<Said> {
    let _turn = held.changing().await;
    let mut environment = one(journal, project, name).await?.ok_or_else(|| missing(project, name))?;
    let Some(ran) =
        environment.ran.as_mut().filter(|ran| ran.run.as_deref() == Some(run) && ran.ended.is_none())
    else {
        return Ok((environment, false));
    };
    let ready = ended.ready;
    let mut said = life(project, name, ran);
    said["took_ms"] = json!((ended.at - ran.at).max(0));
    said["cards"] = json!(ended.cards);
    if let Some(address) = &ended.address {
        said["address"] = json!(address);
    }
    if let Some(reason) = &ended.reason {
        said["reason"] = json!(reason);
    }
    ran.ended = Some(ended);
    let (at, kept) = (key(project, name), environment.kept());
    let write = journal.write(move |writing| {
        writing.keep(NAMESPACE, &at, &kept)?;
        match ready {
            true => writing.say("project.environment.ready", source, said),
            false => writing.say("project.environment.failed", source, said),
        }
    });
    write.await?;
    held.put(project, &environment);
    Ok((environment, true))
}

/// Says that what `project`'s environment called `name` was last loaded as
/// (`instance`) was taken down after it was ready (`project.environment.down`),
/// at `at`, and keeps that, as one write: once, and only of the instance it
/// was last loaded as.
pub async fn down(
    (journal, held): (&Journal, &Held),
    (project, name): (&str, &str),
    instance: &str,
    at: i64,
    source: Source,
) -> Result<Said> {
    let _turn = held.changing().await;
    let mut environment = one(journal, project, name).await?.ok_or_else(|| missing(project, name))?;
    let up = |ran: &&mut Ran| {
        ran.instance == instance && ran.down.is_none() && ran.ended.as_ref().is_some_and(|ended| ended.ready)
    };
    let Some(ran) = environment.ran.as_mut().filter(up) else { return Ok((environment, false)) };
    let mut said = life(project, name, ran);
    let ready = ran.ended.as_ref().map_or(at, |ended| ended.at);
    said["up_ms"] = json!((at - ready).max(0));
    if let Some(address) = ran.ended.as_ref().and_then(|ended| ended.address.clone()) {
        said["address"] = json!(address);
    }
    ran.down = Some(at);
    let (key, kept) = (key(project, name), environment.kept());
    let write = journal.write(move |writing| {
        writing.keep(NAMESPACE, &key, &kept)?;
        writing.say("project.environment.down", source, said)
    });
    write.await?;
    held.put(project, &environment);
    Ok((environment, true))
}

/// What every event of an environment's life says of it and its load.
fn life(project: &str, name: &str, ran: &Ran) -> serde_json::Value {
    let mut said = json!({
        "project": project,
        "environment": name,
        "branch": ran.branch,
        "commit": ran.commit,
        "blueprint": ran.blueprint,
        "instance": ran.instance,
    });
    if let Some(run) = &ran.run {
        said["run"] = json!(run);
    }
    said
}

/// Takes `project`'s environment called `name` away, with its variables
/// and its list of secrets, and says so, as one write. Gives what it was:
/// whoever asked takes its secrets' values away after. Refused while
/// anything of what is `alive` runs under it, or an agent works under it
/// (the names `agents` comes to), which is looked at holding the turn an
/// instance made under it, and an agent said to work under it, hold as
/// each is written ([`Held::holding`]), so that nothing comes to be under
/// it between.
pub async fn remove(
    (journal, held): (&Journal, &Held),
    project: &str,
    name: &str,
    (alive, agents): (
        impl Future<Output = Result<Vec<runs::InstanceInfo>>>,
        impl Future<Output = Result<Vec<String>>>,
    ),
    (by, source): (&Principal, Source),
) -> Result<Environment> {
    let _turn = held.changing().await;
    let was = one(journal, project, name).await?.ok_or_else(|| missing(project, name))?;
    let alive = alive.await?.into_iter().filter(|alive| alive.environment == name);
    let mut running: Vec<String> = alive.map(|alive| format!("`{}`", alive.key)).collect();
    running.extend(agents.await?.into_iter().map(|agent| format!("the agent `{agent}`")));
    if let Some(refused) = in_use(project, name, &running) {
        return Err(refused);
    }
    let at = key(project, name);
    let said = json!({
        "project": project,
        "environment": name,
        "vars": was.vars.len(),
        "secrets": was.secrets.len(),
        "by": by.actor(),
        "by_name": by.name,
    });
    let write = journal.write(move |writing| {
        writing.forget(NAMESPACE, &at)?;
        writing.say("project.environment.removed", source, said)
    });
    write.await?;
    held.forget(project, Some(name));
    Ok(was)
}

/// Why an environment is not taken away while something runs or works
/// under it (`running`, each as it is said: "`deploy`", "the agent
/// `scout`"): it is theirs for as long as they are, and would be nothing.
pub fn in_use(project: &str, name: &str, running: &[String]) -> Option<Error> {
    let first = running.first()?;
    let more = match running.len() {
        1 => format!("{first} runs"),
        many => format!("{first} and {} more run", many - 1),
    };
    Some(Error::conflict(format!(
        "{more} under the environment `{name}`, which is theirs for as long as they do: stop them, or say \
         another environment of each agent, first (`{cli} instances --project {project}`, `{cli} agent list`), \
         and then take it away",
        cli = brand::CLI
    )))
}

/// What is said of an environment a project has not got.
pub fn missing(project: &str, name: &str) -> Error {
    Error::not_found(format!(
        "`{project}` has no environment called `{name}`. `{} project environment {project}` lists those it has",
        brand::CLI
    ))
}

//! The keys an agent's program is handed: secrets the factory keeps, each
//! under a variable of the program's environment.
//!
//! An agent's scopes say what it may ask of this factory. What it may do
//! anywhere else (send a branch to a remote, open a pull request, ask a
//! service something) is said there, by a key of that service's, and what
//! that key allows is the service's to hold to. Behind its wall an agent's
//! program has none of the keys of whoever runs the daemon, so one that is
//! to act elsewhere is handed one made for it: kept by the factory as any
//! secret is (`hosting::secrets`), named on the agent's slot, and read
//! into its program's environment as each turn starts.
//!
//! An agent that works under one of its project's environments is handed
//! what of it is marked for agents (`environments::project`): each marked
//! variable under its own name, and each marked secret as the variable it
//! is marked with. A key its slot names is looked for in that environment
//! first: one the environment lists is handed only where it is marked for
//! agents, and its turn does not start otherwise, never handed the
//! factory's of the same name in its place.
//!
//! It is in no event, row or argument, and what the program prints is
//! written down without it. What the program does with it is the
//! program's: a wall does not hold the network, so a key handed to an
//! agent is one it could send anywhere. Give it one that can do no more
//! than its work needs.

use std::collections::BTreeMap;

use kernel::{Error, Result};

use crate::{Agents, Slot};

/// Whether `name` can be a variable an agent is handed a key under: a
/// variable's name, and not one of the application's own.
fn is_variable(name: &str) -> bool {
    let shaped = name.starts_with(|first: char| first.is_ascii_alphabetic() || first == '_')
        && name.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '_');
    let own = ["PATH", "HOME"].contains(&name)
        || name.starts_with(&format!("{}_", brand::ENV_PREFIX))
        || name.starts_with("FACTORY_");
    shaped && !own
}

/// What a slot is told to be handed, as it is written (`REMOTE_TOKEN=remote-bot`,
/// the variable and then the secret's name): each variable with the secret
/// it is handed.
pub(crate) fn said(pairs: &[String]) -> Result<BTreeMap<String, String>> {
    let mut handed = BTreeMap::new();
    for pair in pairs.iter().map(|pair| pair.trim()).filter(|pair| !pair.is_empty()) {
        let wrong = |why: &str| {
            Error::invalid(format!(
                "`{pair}` does not say a key an agent is handed: {why}. It is written `VARIABLE=name`, the variable its program reads and the name the factory keeps the secret under, as in `REMOTE_TOKEN=remote-bot`"
            ))
        };
        let (variable, name) = pair.split_once('=').ok_or_else(|| wrong("there is no `=` in it"))?;
        let (variable, name) = (variable.trim(), name.trim());
        if !is_variable(variable) {
            return Err(wrong(&format!("`{variable}` cannot be a variable of its program's environment")));
        }
        if name.is_empty() || name != blueprint::expr::slug(name) {
            return Err(wrong(&format!("`{name}` cannot be what a secret is kept under")));
        }
        handed.insert(variable.to_string(), name.to_string());
    }
    Ok(handed)
}

/// The keys one turn is handed, read as it starts.
#[derive(Default)]
pub(crate) struct Keys {
    /// The variable, the name the secret is kept under, and its value.
    held: Vec<(String, String, String)>,
    /// The variables of its environment marked for agents, each under its
    /// own name with its value as text: not secrets, and not taken out of
    /// what the program says.
    plain: Vec<(String, String)>,
}

impl Keys {
    /// What the program's environment gains.
    pub(crate) fn env(&self) -> impl Iterator<Item = (String, String)> + '_ {
        let plain = self.plain.iter().cloned();
        plain.chain(self.held.iter().map(|(variable, _, value)| (variable.clone(), value.clone())))
    }

    /// The variables, for the agent to be told what it holds: the keys,
    /// and what of its environment is marked for it.
    pub(crate) fn variables(&self) -> Vec<String> {
        let plain = self.plain.iter().map(|(variable, _)| variable.clone());
        plain.chain(self.held.iter().map(|(variable, _, _)| variable.clone())).collect()
    }

    /// The same as [`Keys::unsaid`] for something that has the program's
    /// environment and not this: each variable a key is in, and what the
    /// key is said as in its place.
    pub(crate) fn unsay(&self) -> Vec<(String, String)> {
        self.held.iter().map(|(variable, name, _)| (variable.clone(), hosting::unsaid::mark(name))).collect()
    }

    /// What the program said, with each key it was handed taken out: one
    /// that repeats its key, in an answer or in what it fails with, would
    /// otherwise have it written down with the turn.
    pub(crate) fn unsaid(&self, said: &str) -> String {
        // One rule for what is looked for and what is written in its
        // place, for a key an agent is handed as for one a step is.
        let handed = self.held.iter().map(|(_, name, value)| (value.clone(), hosting::unsaid::mark(name)));
        hosting::unsaid::Unsaid::of(handed).from(said)
    }
}

impl Agents {
    /// The keys `slot` is handed, read now from where the factory keeps
    /// them, and what of the environment it works under is marked for
    /// agents. One that is not there, or that its environment lists and
    /// does not hand to agents, is why its turn does not start.
    pub(crate) fn keys_for(&self, slot: &Slot) -> std::result::Result<Keys, String> {
        let under = self.under(slot)?;
        if slot.secrets.is_empty() && under.is_none() {
            return Ok(Keys::default());
        }
        let kept = self.inner.hosting.secrets().map_err(|err| err.message)?;
        let unread = |name: &str, variable: &str, err: Error| {
            format!(
                "`{}` is handed the secret `{name}` as `{variable}`, and it cannot be read: {}",
                slot.name, err.message
            )
        };
        let mut keys = Keys::default();
        let project = slot.project.as_deref().unwrap_or_default();
        if let Some(held) = &under {
            keys.plain = held
                .values(true)
                .into_iter()
                .map(|(name, value)| (name, blueprint::expr::display(&value)))
                .collect();
            for (name, secret) in &held.secrets {
                let Some(variable) = &secret.agents else { continue };
                let value =
                    kept.get_of(project, &held.name, name).map_err(|err| unread(name, variable, err))?;
                keys.held.push((
                    variable.clone(),
                    environments::project::sought(project, &held.name, name),
                    value,
                ));
            }
        }
        for (variable, name) in &slot.secrets {
            let listed = under.as_ref().and_then(|held| held.secrets.get(name).map(|secret| (held, secret)));
            let (sought, value) = match listed {
                None => (name.clone(), kept.get(name).map_err(|err| unread(name, variable, err))?),
                Some((held, secret)) if secret.agents.is_some() => {
                    let value =
                        kept.get_of(project, &held.name, name).map_err(|err| unread(name, variable, err))?;
                    (environments::project::sought(project, &held.name, name), value)
                }
                Some((held, _)) => {
                    return Err(format!(
                        "`{}` is handed `{name}` as `{variable}`, and the environment `{}` it works under lists a \
                         secret `{name}` that it does not hand to agents: nothing is handed in its place, the \
                         factory's least of all. Whoever may mark it says so with `{cli} secret agents {name} VAR \
                         --project {project} --environment {}`",
                        slot.name,
                        held.name,
                        held.name,
                        cli = brand::CLI
                    ));
                }
            };
            keys.held.retain(|(handed, _, _)| handed != variable);
            keys.held.push((variable.clone(), sought, value));
        }
        Ok(keys)
    }

    /// The environment `slot` works under, as the runner holds it; none
    /// for one under none. One that was taken away since is why its turn
    /// does not start: nothing is handed in its place.
    fn under(&self, slot: &Slot) -> std::result::Result<Option<environments::project::Environment>, String> {
        let (Some(project), false) = (slot.project.as_deref(), slot.environment.is_empty()) else {
            return Ok(None);
        };
        let held = self.inner.environs.get().and_then(|held| held.of(project, &slot.environment));
        held.map(Some).ok_or_else(|| {
            format!(
                "`{}` works under the environment `{}` of `{project}`, which is not there any more: its program \
                 is handed nothing of it, and nothing in its place. Say another, or none, with `{} agent add {} \
                 --environment <name>`",
                slot.name,
                slot.environment,
                brand::CLI,
                slot.name
            )
        })
    }

    /// What a new agent of `project` added by hand saying no environment
    /// works under: none where the project has none, and refused, naming
    /// them, where it has any (`environments::held::must_choose`).
    pub(crate) fn none_said(&self, project: &str) -> Result<String> {
        let names = self.inner.environs.get().map(|held| held.names(project)).unwrap_or_default();
        match names.is_empty() {
            true => Ok(String::new()),
            false => Err(Error::invalid(environments::held::must_choose(project, &names))),
        }
    }

    /// What an agent of `project` that is said to work under `said` works
    /// under: none for the empty text, else an environment the project has.
    pub(crate) fn works_under(&self, project: Option<&str>, said: String) -> Result<String> {
        if said.is_empty() {
            return Ok(said);
        }
        let Some(project) = project else {
            return Err(Error::invalid(format!(
                "`{said}` is an environment of a project's, and this agent works in no project: say which project"
            )));
        };
        match self.inner.environs.get().and_then(|held| held.of(project, &said)) {
            Some(_) => Ok(said),
            None => Err(environments::kept::missing(project, &said)),
        }
    }
}

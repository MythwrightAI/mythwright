//! A project's environments: named sets of variables and secrets a
//! project defines for itself, such as `dev`, `staging` and
//! `prod`, one of which an instance of the project is applied under.
//!
//! Each environment is kept whole under the project and its name
//! (`runs::beside::PROJECT_ENVIRONMENTS`, a line of the runner's list of
//! what it keeps beside its rows, `Whose::Project`), in the journal of the
//! conductor that runs what is applied for the project, so that it goes
//! where the project goes: into a copy of the factory and with a project
//! moved to a journal of its own. What is here is what an environment is
//! and what a change of it comes to, as functions of plain values; the
//! runner keeps it, takes the turns and says what changed, as one write
//! (`conductor::environs`).
//!
//! **A variable is handed to an agent only where it is marked so**
//! ([`Variable::agents`], off unless somebody turns it on): an
//! environment filled for a deploy must not put `prod`'s database in a
//! working agent's way because nobody thought of agents. What an agent is
//! shown of an environment is its marked variables and nothing else
//! ([`Environment::shown`]). A variable is handed under its own name, so
//! a name a program's environment is the application's to set (`PATH`,
//! `HOME`, `FACTORY_…`) is never marked.
//!
//! A variable is not for secrets: it is shown to whoever may read the
//! project, kept as it is, and may appear in what a step prints. A
//! **secret** of an environment is listed here by name, with who set it,
//! when, and the variable an agent is handed it as ([`Secret`]); its value
//! is a file of the factory's secrets, sealed under the one name the
//! project, the environment and the secret's name make together
//! ([`sought`]), and nothing here or anywhere else gives it out. The list
//! is what is believed: a file nothing lists is never read, so it is
//! written before its secret is listed and taken away after it is not.
//!
//! **An environment may say how it is loaded** ([`Environment::deploy`]):
//! the name of a blueprint, found as any is for the project, which a
//! person runs for a branch of the project's repository, under the
//! environment, handed the branch. What a deploy does is that blueprint's
//! and nobody else's. What the environment last ran is kept with it
//! ([`Ran`]): the branch, the commit and the run, which is the record of
//! how it went and is read for that, never kept here a second time.

use std::collections::BTreeMap;

use kernel::{Error, Fields, Principal, Result};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// Where each environment is kept.
pub const NAMESPACE: &str = runs::beside::PROJECT_ENVIRONMENTS;

/// The longest name an environment may have.
const LONGEST: usize = 40;

/// One of a project's environments.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Environment {
    /// What it is called: lowercase letters, digits and `-`.
    pub name: String,
    /// Its variables, by name.
    #[serde(default)]
    pub vars: BTreeMap<String, Variable>,
    /// Its secrets, by name: who set each and when, and what an agent is
    /// handed it as. The value is never here: it is a file of the
    /// factory's secrets, sealed under the environment's name for it
    /// ([`sought`]), and is set and read only there.
    #[serde(default)]
    pub secrets: BTreeMap<String, Secret>,
    /// When it was made, in milliseconds, and by whom, as an actor and by name.
    pub made: i64,
    pub by: String,
    pub by_name: String,
    /// The blueprint it is loaded by, for a branch: none where it says none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deploy: Option<String>,
    /// What it was last loaded with, where it ever was.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ran: Option<Ran>,
    /// What it holds to: who approves what goes under it, and which
    /// branches may be run under it ([`Rules`]).
    #[serde(default, skip_serializing_if = "Rules::is_empty")]
    pub rules: Rules,
    /// Of each feature flag's variable (`FLAG_<NAME>`), the change of it
    /// asked last that waits for a go-ahead, by its id: only that one is
    /// written once it is approved, and only while the flag says what it
    /// said when it was asked (`crate::flagged`). Every change of the
    /// flag written since takes it away.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub asked: BTreeMap<String, String>,
}

pub use crate::rules::{MOST_APPROVING, Rules};

/// What an environment was last loaded with: the branch and the commit it
/// was then, who asked and when, and what that started. How it went is
/// the run's to say, and is read from it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Ran {
    /// The branch as it was chosen: `dev`, or as a remote holds it, `origin/dev`.
    pub branch: String,
    /// The commit the branch was at when it was run, whole.
    pub commit: String,
    /// The blueprint that was run, as the environment said it then.
    pub blueprint: String,
    /// When, in milliseconds, and by whom, as an actor and by name.
    pub at: i64,
    pub by: String,
    pub by_name: String,
    /// The instance that was applied, and the run of its steps it began.
    pub instance: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run: Option<String>,
    /// How that run came out, once it was said (`project.environment.ready`
    /// or `project.environment.failed`): kept so that it is said once.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ended: Option<Ended>,
    /// When what it was loaded as was taken down after it was ready, once
    /// that was said (`project.environment.down`), in milliseconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub down: Option<i64>,
}

/// How the run an environment was loaded by came out, as it was said.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Ended {
    /// Whether its steps went well: the environment is ready.
    pub ready: bool,
    /// When it ended, in milliseconds.
    pub at: i64,
    /// Where it is reached, where the blueprint it is loaded by gave one
    /// out as `address`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
    /// The cards it holds, where the blueprint gave them out as `cards`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cards: Vec<String>,
    /// Why it failed, where it did.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

impl Ended {
    /// How a run of the blueprint an environment is loaded by came out:
    /// `ok` where its steps went well, with what it gave out (`outputs`)
    /// and why it did not where it did not.
    pub fn of(ok: bool, at: i64, outputs: &Value, reason: Option<String>) -> Ended {
        let address = outputs.get("address").and_then(Value::as_str).filter(|said| !said.trim().is_empty());
        let cards = match outputs.get("cards") {
            Some(Value::Array(cards)) => cards.iter().filter_map(Value::as_str).map(str::to_string).collect(),
            Some(Value::String(cards)) => {
                cards.split(',').map(str::trim).filter(|card| !card.is_empty()).map(str::to_string).collect()
            }
            _ => Vec::new(),
        };
        Ended {
            ready: ok,
            at,
            address: address.map(|said| said.trim().to_string()),
            cards,
            reason: reason.filter(|_| !ok),
        }
    }
}

/// A variable of an environment: a plain value, and whether an agent
/// that works under the environment is handed it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Variable {
    pub value: Value,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub agents: bool,
}

/// A secret of an environment, as it is listed: never its value.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Secret {
    /// When it was last set, in milliseconds, and by whom.
    pub set: i64,
    pub by: String,
    pub by_name: String,
    /// The variable an agent that works under the environment is handed
    /// it as; none where it is not handed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agents: Option<String>,
}

/// What a change of an environment changed, by name.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Changed {
    /// Variables that were not set before.
    pub set: Vec<String>,
    /// Variables that have another value than they had.
    pub changed: Vec<String>,
    /// Variables that are no longer set.
    pub removed: Vec<String>,
    /// Variables whose mark for agents is another than it was.
    pub marked: Vec<String>,
    /// Secrets that were set or set again.
    pub secrets: Vec<String>,
    /// Secrets that are no longer kept.
    pub secrets_removed: Vec<String>,
    /// Secrets whose variable for agents is another than it was.
    pub secrets_marked: Vec<String>,
    /// Whether the blueprint it is loaded by is another than it was.
    pub deploy: bool,
    /// Whether what it holds to is another than it was.
    pub rules: bool,
}

impl Changed {
    pub fn is_empty(&self) -> bool {
        self.set.is_empty()
            && self.changed.is_empty()
            && self.removed.is_empty()
            && self.marked.is_empty()
            && self.secrets.is_empty()
            && self.secrets_removed.is_empty()
            && self.secrets_marked.is_empty()
            && !self.deploy
            && !self.rules
    }
}

/// What an environment is kept under: its project and its name, as
/// everything kept by project is (`runs::beside::project_of`).
pub fn key(project: &str, name: &str) -> String {
    format!("{project}\n{name}")
}

/// The name of an environment that a key of [`NAMESPACE`] is of, where
/// the key is `project`'s.
pub fn named<'a>(key: &'a str, project: &str) -> Option<&'a str> {
    key.strip_prefix(project)?.strip_prefix('\n')
}

/// Refuses what cannot be an environment's name.
pub fn check_name(name: &str) -> Result<()> {
    let shaped = !name.is_empty()
        && name.len() <= LONGEST
        && name.starts_with(|first: char| first.is_ascii_lowercase() || first.is_ascii_digit())
        && name.chars().all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-');
    if shaped {
        Ok(())
    } else {
        Err(Error::invalid(format!(
            "`{name}` cannot be the name of an environment. A name is lowercase letters, digits and `-`, \
             begins with a letter or a digit and is at most {LONGEST} long, such as `dev`, `staging` or `prod`"
        )))
    }
}

impl Environment {
    /// A new environment called `name`, made by `by` at `at`, with no
    /// variables.
    pub fn new(name: &str, by: &Principal, at: i64) -> Result<Environment> {
        check_name(name)?;
        Ok(Environment {
            name: name.to_string(),
            vars: BTreeMap::new(),
            secrets: BTreeMap::new(),
            made: at,
            by: by.actor(),
            by_name: by.name.clone(),
            deploy: None,
            ran: None,
            rules: Rules::default(),
            asked: BTreeMap::new(),
        })
    }

    /// Says who approves what goes under it, as actors (`user:<account>`),
    /// in the order said, each once; nobody, for an empty list. Whether
    /// each is somebody here is whoever asks's to look: what is refused
    /// here is what cannot be a person's actor.
    pub fn say_approve(&mut self, approve: &[String]) -> Result<Changed> {
        let mut said: Vec<String> = Vec::new();
        for one in approve {
            let one = one.trim();
            let person = one
                .strip_prefix("user:")
                .is_some_and(|id| !id.is_empty() && !id.contains(char::is_whitespace));
            if !person {
                return Err(Error::invalid(format!(
                    "`{one}` cannot approve what goes under `{}`: who approves is a person, said as `user:<account>`",
                    self.name
                )));
            }
            if !said.iter().any(|was| was == one) {
                said.push(one.to_string());
            }
        }
        if said.len() > MOST_APPROVING {
            return Err(Error::invalid(format!(
                "an environment names {MOST_APPROVING} people at most as approving what goes under it"
            )));
        }
        if self.rules.approve == said {
            return Ok(Changed::default());
        }
        self.rules.approve = said;
        Ok(Changed { rules: true, ..Changed::default() })
    }

    /// Says which branches may be run under it ([`Rules::lets`]), in the
    /// order said, each once; every one, for an empty list.
    pub fn say_branches(&mut self, branches: &[String]) -> Result<Changed> {
        let mut said: Vec<String> = Vec::new();
        for one in branches.iter().map(|one| one.trim()) {
            if let Some(why) = crate::rules::branch_refused(one) {
                return Err(Error::invalid(why));
            }
            if !said.iter().any(|was| was == one) {
                said.push(one.to_string());
            }
        }
        if said.len() > crate::rules::MOST_BRANCHES {
            return Err(Error::invalid(format!(
                "an environment names {} branches at most as those run under it",
                crate::rules::MOST_BRANCHES
            )));
        }
        if self.rules.branches == said {
            return Ok(Changed::default());
        }
        self.rules.branches = said;
        Ok(Changed { rules: true, ..Changed::default() })
    }

    /// Says the blueprint it is loaded by, or none. A name that cannot be
    /// a blueprint's is refused; whether there is one of that name is
    /// whoever asks's to look.
    pub fn say_deploy(&mut self, blueprint: Option<&str>) -> Result<Changed> {
        if let Some(name) = blueprint
            && let Some(why) = deploy_refused(name)
        {
            return Err(Error::invalid(why));
        }
        if self.deploy.as_deref() == blueprint {
            return Ok(Changed::default());
        }
        self.deploy = blueprint.map(str::to_string);
        Ok(Changed { deploy: true, ..Changed::default() })
    }

    /// Sets each of `vars`, leaving the rest as they are. A variable that
    /// was not set before is not handed to agents unless `agents` says so;
    /// one that was keeps its mark unless `agents` says otherwise.
    pub fn set(&mut self, vars: &Fields, agents: Option<bool>) -> Result<Changed> {
        let mut changed = Changed::default();
        for (name, value) in vars {
            check_var(name, value)?;
            if agents == Some(true) {
                may_hand(name)?;
                self.not_handed_as(name, None)?;
            }
        }
        for (name, value) in vars {
            match self.vars.get_mut(name) {
                None => {
                    changed.set.push(name.clone());
                    let agents = agents.unwrap_or(false);
                    self.vars.insert(name.clone(), Variable { value: value.clone(), agents });
                }
                Some(was) => {
                    if was.value != *value {
                        changed.changed.push(name.clone());
                        was.value = value.clone();
                    }
                    if let Some(agents) = agents.filter(|agents| *agents != was.agents) {
                        changed.marked.push(name.clone());
                        was.agents = agents;
                    }
                }
            }
        }
        Ok(changed)
    }

    /// Takes each of `names` away; a name that is not set is refused, with
    /// nothing taken away.
    pub fn unset(&mut self, names: &[String]) -> Result<Changed> {
        self.known(names)?;
        let mut changed = Changed::default();
        for name in names {
            if self.vars.remove(name).is_some() {
                changed.removed.push(name.clone());
            }
        }
        Ok(changed)
    }

    /// Marks each of `names` as handed to agents, or not. A name that is
    /// not set is refused, with nothing marked.
    pub fn mark(&mut self, names: &[String], agents: bool) -> Result<Changed> {
        self.known(names)?;
        if agents {
            for name in names {
                may_hand(name)?;
                self.not_handed_as(name, None)?;
            }
        }
        let mut changed = Changed::default();
        for name in names {
            if let Some(var) = self.vars.get_mut(name).filter(|var| var.agents != agents) {
                var.agents = agents;
                changed.marked.push(name.clone());
            }
        }
        Ok(changed)
    }

    /// Lists the secret `name` as set by `by` at `at`, in place of
    /// whatever was said of it, keeping what an agent is handed it as.
    /// Its value is kept apart, before this is kept.
    pub fn keep_secret(&mut self, name: &str, by: &Principal, at: i64) -> Result<Changed> {
        if let Some(why) = secret_refused(name) {
            return Err(Error::invalid(why));
        }
        let agents = self.secrets.get(name).and_then(|secret| secret.agents.clone());
        let kept = Secret { set: at, by: by.actor(), by_name: by.name.clone(), agents };
        self.secrets.insert(name.to_string(), kept);
        Ok(Changed { secrets: vec![name.to_string()], ..Changed::default() })
    }

    /// Takes the secret `name` off the list; its value is taken away
    /// after this is kept. One that is not listed is refused.
    pub fn drop_secret(&mut self, name: &str) -> Result<Changed> {
        self.secret_known(name)?;
        self.secrets.remove(name);
        Ok(Changed { secrets_removed: vec![name.to_string()], ..Changed::default() })
    }

    /// Says what an agent is handed the secret `name` as: a variable of
    /// its program's environment, or nothing for not at all.
    pub fn mark_secret(&mut self, name: &str, agents: Option<&str>) -> Result<Changed> {
        self.secret_known(name)?;
        if let Some(variable) = agents {
            if !is_name(variable) {
                return Err(Error::invalid(format!(
                    "`{variable}` cannot be the variable an agent is handed `{name}` as: a variable is \
                     letters, digits and `_`, and does not start with a digit, such as `STRIPE_KEY`"
                )));
            }
            may_hand(variable)?;
            self.not_handed_as(variable, Some(name))?;
        }
        let secret = self.secrets.get_mut(name).expect("known");
        if secret.agents.as_deref() == agents {
            return Ok(Changed::default());
        }
        secret.agents = agents.map(str::to_string);
        Ok(Changed { secrets_marked: vec![name.to_string()], ..Changed::default() })
    }

    /// Refuses `variable` where something else of the environment is
    /// handed to agents under it already: a variable marked for agents, or
    /// a secret other than `secret`.
    fn not_handed_as(&self, variable: &str, secret: Option<&str>) -> Result<()> {
        let by_a_variable = secret.is_some() && self.vars.get(variable).is_some_and(|var| var.agents);
        let by_a_secret = self
            .secrets
            .iter()
            .find(|(name, kept)| Some(name.as_str()) != secret && kept.agents.as_deref() == Some(variable));
        let what = match (by_a_variable, by_a_secret) {
            (true, _) => format!("the variable `{variable}`"),
            (false, Some((name, _))) => format!("the secret `{name}`"),
            (false, None) => return Ok(()),
        };
        Err(Error::conflict(format!(
            "an agent working under `{}` is handed {what} as `{variable}` already: it cannot be handed two \
             things under one name. Take that mark off first, or hand this as another name",
            self.name
        )))
    }

    fn secret_known(&self, name: &str) -> Result<()> {
        if self.secrets.contains_key(name) {
            return Ok(());
        }
        let has = if self.secrets.is_empty() {
            "none".to_string()
        } else {
            self.secrets.keys().map(|name| format!("`{name}`")).collect::<Vec<_>>().join(", ")
        };
        Err(Error::not_found(format!(
            "the environment `{}` has no secret `{name}`; it has {has}. Nothing was changed",
            self.name
        )))
    }

    fn known(&self, names: &[String]) -> Result<()> {
        match names.iter().find(|name| !self.vars.contains_key(*name)) {
            None => Ok(()),
            Some(name) => Err(Error::not_found(format!(
                "the environment `{}` has no variable `{name}`; it has {}. Nothing was changed",
                self.name,
                self.listed()
            ))),
        }
    }

    fn listed(&self) -> String {
        if self.vars.is_empty() {
            "none".to_string()
        } else {
            self.vars.keys().map(|name| format!("`{name}`")).collect::<Vec<_>>().join(", ")
        }
    }

    /// As it is kept.
    pub fn kept(&self) -> Value {
        serde_json::to_value(self).unwrap_or(Value::Null)
    }

    /// What it is shown as to somebody: all of it, or to an agent what is
    /// marked for agents alone, with nothing said of the rest. Its
    /// secrets are listed by name, never with a value, and only to whoever
    /// may see that they are there (`secret_names`).
    pub fn shown(&self, to_an_agent: bool, secret_names: bool) -> Value {
        let vars: serde_json::Map<String, Value> = self
            .vars
            .iter()
            .filter(|(_, var)| !to_an_agent || var.agents)
            .map(|(name, var)| (name.clone(), json!({ "value": var.value, "agents": var.agents })))
            .collect();
        let secrets: serde_json::Map<String, Value> = self
            .secrets
            .iter()
            .filter(|(_, secret)| !to_an_agent || secret.agents.is_some())
            .map(|(name, secret)| {
                (
                    name.clone(),
                    json!({ "set": secret.set, "by_name": secret.by_name, "agents": secret.agents }),
                )
            })
            .collect();
        let mut shown = json!({
            "name": self.name,
            "vars": vars,
            "made": self.made,
            "by": self.by,
            "by_name": self.by_name,
        });
        // How it is loaded is said to whoever may read the project, and to
        // an agent nothing of it, as nothing of the rest is.
        if !to_an_agent {
            shown["deploy"] = json!(self.deploy);
            shown["ran"] = json!(self.ran);
            shown["rules"] = json!(self.rules);
        }
        // Said not to be there where it is not to be said at all, so that
        // nobody takes an empty list for none.
        if secret_names {
            shown["secrets"] = Value::Object(secrets);
        }
        shown
    }

    /// The variables, as an expression reads them (`environment.vars`):
    /// each name and its value, of those an agent is handed alone where
    /// it is for one.
    pub fn values(&self, to_an_agent: bool) -> Fields {
        self.vars
            .iter()
            .filter(|(_, var)| !to_an_agent || var.agents)
            .map(|(name, var)| (name.clone(), var.value.clone()))
            .collect()
    }
}

/// What `project`'s environment `environment` calls its secret `name`
/// where the factory's secrets are kept and sealed: one name for the
/// three together, so that a file of one environment put in another's
/// place does not open.
pub fn sought(project: &str, environment: &str, name: &str) -> String {
    format!("{project}/{environment}/{name}")
}

/// Why `name` may not be a secret's name, if it may not. A name is a
/// file's name where it is kept, and is written in blueprints: lowercase
/// letters, digits, `-`, `_` and `.`, beginning with a letter or a digit.
/// So it never holds a `/`, which is how an environment's secret is
/// sought ([`sought`]).
pub fn secret_refused(name: &str) -> Option<String> {
    let mut letters = name.chars();
    let right = letters.next().is_some_and(|first| first.is_ascii_lowercase() || first.is_ascii_digit())
        && letters.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '-' | '_' | '.'))
        && name.len() <= 100
        && !name.ends_with('.');
    (!right).then(|| {
        format!(
            "`{name}` cannot be a secret's name: use lowercase letters, digits, `-`, `_` and `.`, beginning \
             with a letter or a digit, such as `stripe` or `mail.password`"
        )
    })
}

/// Why `name` cannot be the blueprint an environment is loaded by, if it
/// cannot: a blueprint is named as it is everywhere (`deploy`,
/// `site/deploy`), with no space and nothing a path would take for a way up.
pub fn deploy_refused(name: &str) -> Option<String> {
    let shaped = !name.is_empty()
        && name.len() <= 200
        && !name.starts_with('/')
        && name.split('/').all(|part| !part.is_empty() && part != "." && part != "..")
        && name.chars().all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | '/'));
    (!shaped).then(|| {
        format!(
            "`{name}` cannot be the name of a blueprint: a name is letters, digits, `-`, `_` and `.`, in parts \
             joined by `/`, such as `deploy` or `site/deploy`"
        )
    })
}

/// What a kept environment is read back as; nothing for what cannot be
/// read as one (a database edited by hand).
pub fn read(kept: Value) -> Option<Environment> {
    serde_json::from_value(kept).ok()
}

/// Whether `name` can be written after `environment.vars.` in an
/// expression.
fn is_name(name: &str) -> bool {
    let mut letters = name.bytes();
    letters.next().is_some_and(|first| first.is_ascii_alphabetic() || first == b'_')
        && letters.all(|letter| letter.is_ascii_alphanumeric() || letter == b'_')
}

/// Refuses a variable that could not be read back as it was meant.
fn check_var(name: &str, value: &Value) -> Result<()> {
    if !is_name(name) {
        return Err(Error::invalid(format!(
            "`{name}` cannot be the name of a variable. A name is letters, digits and `_`, and does not start \
             with a digit, so that a blueprint can read it as `environment.vars.<name>`"
        )));
    }
    let kind = match value {
        Value::String(_) | Value::Number(_) | Value::Bool(_) => return Ok(()),
        Value::Array(_) => "a list",
        Value::Object(_) => "a map",
        Value::Null => "nothing (null)",
    };
    Err(Error::invalid(format!(
        "the variable `{name}` is {kind}, and a variable is plain: text, a number, or true or false. Write \
         anything larger as text, for the blueprint that reads it to take apart"
    )))
}

/// Refuses to hand an agent a variable under a name its program's
/// environment is the application's to set.
fn may_hand(name: &str) -> Result<()> {
    let upper = name.to_ascii_uppercase();
    let own = ["PATH", "HOME", "USERPROFILE"].contains(&upper.as_str())
        || upper.starts_with(&format!("{}_", brand::ENV_PREFIX))
        || upper.starts_with("FACTORY_");
    if own {
        return Err(Error::invalid(format!(
            "`{name}` cannot be handed to an agent: an agent's program is handed a variable under its own \
             name, and `{name}` is one the application sets itself. Call it something else to hand it on"
        )));
    }
    Ok(())
}

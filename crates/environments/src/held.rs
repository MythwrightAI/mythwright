//! A project's environments as whatever runs under one reads them: held
//! in memory for every conductor of the factory, and what an environment
//! comes to for an instance applied under it.
//!
//! The runner works an expression out for every step, so what an
//! instance reads of its environment (`environment.name`,
//! `environment.vars`) is held here and never read from the journal as
//! it is worked out. What is kept is the journal's
//! (`runs::beside::PROJECT_ENVIRONMENTS`): each conductor puts what its
//! own journal keeps here as it starts ([`Held::load`]), and each change
//! is written and then held ([`Held::put`], [`Held::forget`]), holding
//! the turn ([`Held::changing`]), so that the two agree.
//!
//! What an environment comes to is a function of what is held:
//! - [`Held::view`]: what an expression reads, the empty one for an
//!   instance under none, and for an instance an agent applied only what
//!   is marked for agents;
//! - [`Held::sought`]: which secret a name an instance's text writes is,
//!   the environment's where it lists it and the factory's where it does
//!   not, and refused where it names a project's itself, or where an
//!   agent's instance would be handed what is not marked for agents;
//! - [`must_choose`] and [`not_another`]: what is said of an apply that
//!   names none, or another than the instance runs under;
//! - [`Held::holding`]: what comes to run under one held to its rules
//!   (`crate::rules`), a go-ahead asked of whatever keeps them
//!   ([`Approvals`], installed by [`Held::answers`]).

use std::collections::BTreeMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{OnceLock, RwLock};

use kernel::Fields;
use serde_json::{Value, json};

use crate::project::{Environment, key, named, secret_refused, sought};
use crate::rules::{Comes, Entering, unapproved};

/// The variable every program a step starts is told the environment it
/// runs under in, as an expression's `environment.name`: unset for
/// none. It said the profile before 0.1.0.
pub const ENV_VAR: &str = "FACTORY_ENVIRONMENT";

/// The environments of every project there is, by project and name.
#[derive(Default)]
pub struct Held {
    by: RwLock<BTreeMap<String, Environment>>,
    /// One change of a project's environments at a time, so that what is
    /// kept and what is held agree; and an instance made under one holds
    /// it as it is written, so that nothing takes the environment away
    /// between the look that nothing runs under it and the write.
    changing: tokio::sync::Mutex<()>,
    /// How many wait for the turn, and a word each time one more does:
    /// what a test that makes an order of two of them waits on.
    waiting: std::sync::atomic::AtomicUsize,
    one_more: tokio::sync::Notify,
    /// Whatever keeps go-aheads, asked whether a run holds one; nothing
    /// installed is a go-ahead nowhere, which is the side to be wrong on.
    approvals: OnceLock<Box<dyn Approvals>>,
}

/// Whether a run of `project` holds a go-ahead to start something under
/// its environment `environment`, given by one of `by` (as actors): a
/// go ask of the run's naming the environment, approved. It is read and
/// not spent, since it names the run and nothing else can use it.
/// Answered by whatever keeps them (`told`), which stands above the
/// runner; it holds nothing of the runner, which holds it.
pub trait Approvals: Send + Sync {
    fn approved<'a>(
        &'a self,
        project: &'a str,
        run: &'a str,
        environment: &'a str,
        by: &'a [String],
    ) -> Pin<Box<dyn Future<Output = bool> + Send + 'a>>;
}

/// What a name an instance's text gives a secret comes to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Sought {
    /// The factory's secret of that name: the environment lists none.
    Factory(String),
    /// The environment's, by the one name it is kept and sealed under.
    Environment(String),
}

impl Held {
    /// The turn at changing what is kept of a project's environments,
    /// taken in the order it is asked for.
    pub async fn changing(&self) -> tokio::sync::MutexGuard<'_, ()> {
        use std::sync::atomic::Ordering::SeqCst;
        self.waiting.fetch_add(1, SeqCst);
        self.one_more.notify_waiters();
        let turn = self.changing.lock().await;
        self.waiting.fetch_sub(1, SeqCst);
        turn
    }

    /// Waits until `many` wait for the turn.
    pub async fn waiting_for(&self, many: usize) {
        loop {
            let told = self.one_more.notified();
            if self.waiting.load(std::sync::atomic::Ordering::SeqCst) >= many {
                return;
            }
            told.await;
        }
    }

    /// Installs what answers whether a run holds a go-ahead; once.
    pub fn answers(&self, approvals: impl Approvals + 'static) {
        let _ = self.approvals.set(Box::new(approvals));
    }

    /// The turn, for an instance of `project` made under `environment`,
    /// with the environment there: held until its row is written, so that
    /// it is not taken away in between. Nothing for one under none.
    ///
    /// What comes to run under it (`entering`: named by whoever applies,
    /// or by a step's `environment:`, and not what is started by what
    /// runs under it already) is held to its rules in the same turn:
    /// refused for a branch it does not let run, and, by somebody its
    /// `approve` does not name, unless the run that asks holds a go-ahead
    /// one of them gave ([`Approvals`]).
    pub async fn holding(
        &self,
        project: Option<&str>,
        environment: &str,
        entering: Option<Entering<'_>>,
    ) -> kernel::Result<Option<tokio::sync::MutexGuard<'_, ()>>> {
        let Some(project) = project.filter(|_| !environment.is_empty()) else { return Ok(None) };
        let turn = self.changing().await;
        let held = self.of(project, environment).ok_or_else(|| crate::kept::missing(project, environment))?;
        let Some(entering) = entering else { return Ok(Some(turn)) };
        let by = match held.rules.comes(project, environment, &entering) {
            Comes::Goes => return Ok(Some(turn)),
            Comes::Refused(why) => return Err(kernel::Error::denied(why)),
            Comes::Waits(by) => by,
        };
        let approved = match (entering.run, self.approvals.get()) {
            (Some(run), Some(approvals)) => approvals.approved(project, run, environment, &by).await,
            _ => false,
        };
        match approved {
            true => Ok(Some(turn)),
            false => Err(kernel::Error::denied(unapproved(project, environment, entering.run.is_some()))),
        }
    }

    /// Holds what a journal keeps: each entry as it is kept, by its key.
    /// What cannot be read as an environment is left out.
    pub fn load(&self, kept: Vec<(String, Value)>) {
        let mut by = self.by.write().expect("environments held");
        for (at, kept) in kept {
            if let Some(environment) = crate::project::read(kept) {
                by.insert(at, environment);
            }
        }
    }

    /// Holds `environment` of `project` as it was just kept.
    pub fn put(&self, project: &str, environment: &Environment) {
        let mut by = self.by.write().expect("environments held");
        by.insert(key(project, &environment.name), environment.clone());
    }

    /// Lets go of `project`'s environment `name`, which was just taken
    /// away; or of every one of `project`'s, where `name` is none.
    pub fn forget(&self, project: &str, name: Option<&str>) {
        let mut by = self.by.write().expect("environments held");
        match name {
            Some(name) => {
                by.remove(&key(project, name));
            }
            None => by.retain(|at, _| named(at, project).is_none()),
        }
    }

    /// The names of `project`'s environments, in order.
    pub fn names(&self, project: &str) -> Vec<String> {
        let by = self.by.read().expect("environments held");
        by.keys().filter_map(|at| named(at, project)).map(str::to_string).collect()
    }

    /// The project and the environment that were last loaded as
    /// `instance` and are ready and not yet said to be taken down, as they
    /// are held: what is asked of every instance that ends, so asked of
    /// nothing but memory.
    pub fn up_as(&self, instance: &str) -> Option<(String, Environment)> {
        let by = self.by.read().expect("environments held");
        by.iter().find_map(|(at, environment)| {
            let ran = environment.ran.as_ref()?;
            let ready = ran.ended.as_ref().is_some_and(|ended| ended.ready);
            let up = ran.instance == instance && ready && ran.down.is_none();
            let project = at.strip_suffix(&environment.name)?.strip_suffix('\n')?;
            up.then(|| (project.to_string(), environment.clone()))
        })
    }

    /// `project`'s environment `name`, as it is held.
    pub fn of(&self, project: &str, name: &str) -> Option<Environment> {
        self.by.read().expect("environments held").get(&key(project, name)).cloned()
    }

    /// What an instance of `project` applied under `environment` reads as
    /// `environment`: its name and its variables, the marked ones alone
    /// for an instance an agent applied (`agent`). Under none, or under
    /// one taken away since, it reads its name and no variables.
    pub fn view(&self, project: Option<&str>, environment: &str, agent: bool) -> Value {
        let vars = match (project, environment) {
            (Some(project), name) if !name.is_empty() => {
                self.of(project, name).map(|environment| environment.values(agent)).unwrap_or_default()
            }
            _ => Fields::new(),
        };
        json!({ "name": environment, "vars": vars })
    }

    /// Which secret the name `name`, written in the text of an instance of
    /// `project` applied under `environment`, is: the environment's where
    /// it lists one of that name, else the factory's. A name that is no
    /// secret's (one with a `/` in it, which is how a project's own is
    /// kept) is refused, and so is one of the environment's that is not
    /// marked for agents, for an instance an agent applied: it is never
    /// handed the factory's of that name in its place.
    pub fn sought(
        &self,
        project: Option<&str>,
        environment: &str,
        name: &str,
        agent: bool,
    ) -> Result<Sought, String> {
        if name.contains('/') {
            return Err(format!(
                "`{name}` is not a secret's name a text may say: a secret is named as it is called, and an \
                 environment's own is found by its name where the instance runs under it"
            ));
        }
        let (project, held) = match project {
            Some(project) if !environment.is_empty() => match self.of(project, environment) {
                Some(held) => (project, held),
                // Taken away from under it: nothing is handed in its place,
                // the factory's of the name least of all.
                None => {
                    return Err(format!(
                        "this runs under the environment `{environment}` of `{project}`, which is not there any \
                         more: no secret is handed to it, and the factory's `{name}` is not handed in its place"
                    ));
                }
            },
            _ => return Ok(Sought::Factory(name.to_string())),
        };
        let Some(secret) = held.secrets.get(name) else { return Ok(Sought::Factory(name.to_string())) };
        if agent && secret.agents.is_none() {
            return Err(format!(
                "the environment `{environment}` of `{project}` does not hand its secret `{name}` to what an \
                 agent applied: whoever may mark it says so with `{cli} secret agents {name} VAR --project \
                 {project} --environment {environment}`",
                cli = brand::CLI
            ));
        }
        if let Some(why) = secret_refused(name) {
            return Err(why);
        }
        Ok(Sought::Environment(sought(project, environment, name)))
    }
}

impl Held {
    /// [`Held::sought`], as the one name a secret is handed on by.
    pub fn secret(
        &self,
        project: Option<&str>,
        environment: &str,
        name: &str,
        agent: bool,
    ) -> Result<String, String> {
        Ok(match self.sought(project, environment, name, agent)? {
            Sought::Factory(name) | Sought::Environment(name) => name,
        })
    }
}

/// What an apply comes to run under, of what it asks and what it is
/// asked under.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Chosen {
    /// This, whatever there is: what it is started under, or none.
    Is(String),
    /// The one it names of the project's, which the project has to have.
    Named(String, String),
    /// One of the project's, which somebody has to name where it has any.
    OneOf(String),
}

/// What an apply of `key` comes to run under: what it is started under
/// (`under`, its parent's or that of the instance that applies it for
/// itself), refused where it asks for another; else the environment it
/// names (`asked`) of its project, refused for what is applied for no
/// project; else none, or one of its project's for somebody who asks
/// (`person`), which is refused where the project has any. Asked for as
/// the empty name, it is none on purpose: what the daemon applies to look
/// after a project's tables, which are the project's whatever it deploys
/// to, and which no request from outside says.
///
/// Started under none, it may name one of its project's: a step that
/// says `environment:` starts its child under it (a pipeline under none
/// that deploys to `prod`), held there to the environment's rules. Under
/// one, it may name no other: a `dev` run never reaches `prod` by a step.
pub fn chosen(
    under: Option<String>,
    asked: Option<&str>,
    key: &str,
    project: Option<&str>,
    person: bool,
) -> kernel::Result<Chosen> {
    let names = asked.is_some_and(|asked| !asked.is_empty());
    if let Some(under) = under.filter(|under| !(under.is_empty() && names)) {
        return match asked.filter(|asked| *asked != under) {
            Some(asked) => Err(kernel::Error::conflict(not_another(key, &under, asked))),
            None => Ok(Chosen::Is(under)),
        };
    }
    match (project, asked) {
        (_, Some("")) => Ok(Chosen::Is(String::new())),
        (None, Some(name)) => Err(kernel::Error::invalid(format!(
            "`{name}` is an environment of a project's, and `{key}` is applied for no project: say which project \
             it is for"
        ))),
        (Some(project), Some(name)) => Ok(Chosen::Named(project.to_string(), name.to_string())),
        (Some(project), None) if person => Ok(Chosen::OneOf(project.to_string())),
        _ => Ok(Chosen::Is(String::new())),
    }
}

/// What is said of an apply that would make an instance of `project`,
/// which has the environments `names`, and names none of them.
pub fn must_choose(project: &str, names: &[String]) -> String {
    let first = names.first().map(String::as_str).unwrap_or("dev");
    format!(
        "{project} runs everything under one of its environments, and this names none: say one of {} \
         (`--environment {first}`)",
        names.join(", ")
    )
}

/// What is said of an apply over an instance that runs under `under`,
/// asking for `asked`.
pub fn not_another(key: &str, under: &str, asked: &str) -> String {
    let under = if under.is_empty() { "no environment".to_string() } else { under.to_string() };
    format!(
        "`{key}` runs under {under}, and this asks for {asked}: what an instance runs under is its own for its \
         life. Stop it first, or apply it under another key"
    )
}

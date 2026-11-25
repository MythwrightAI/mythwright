//! A project's environments: the runner's side. What one is, and how
//! it is kept, is `environments` (`project`, `kept`); what is here finds
//! the conductor that keeps what is the project's (whose own journal is
//! the project's, where it has one), and says what an instance runs
//! under for its life (`InstanceInfo::environment`) and reads of it, from
//! what is held for every conductor (`environments::held`).

pub use environments::flagged::{Expected, Flagged, Flagging};
use environments::held::Chosen;
pub use environments::kept::in_use;
use environments::kept::{self, missing};
pub use environments::last;
pub use environments::project::{
    Changed, Ended, Environment, NAMESPACE, Ran, Secret, Variable, deploy_refused, key, secret_refused,
    sought,
};
use environments::rules::Entering;
use kernel::{Principal, PrincipalKind, Result, Source};
use serde_json::Value;

use crate::engine::{Engine, Live};
use crate::{Apply, InstanceInfo};

impl Engine {
    /// `project`'s environments, by name.
    pub async fn environments_of(&self, project: &str) -> Result<Vec<Environment>> {
        kept::of(&self.keeper_of(project).await.inner.own, project).await
    }

    /// `project`'s environment called `name`, where it has one.
    pub async fn environment_of(&self, project: &str, name: &str) -> Result<Option<Environment>> {
        kept::one(&self.keeper_of(project).await.inner.own, project, name).await
    }

    /// Makes an environment of `project` called `name` (`kept::make`).
    pub async fn make_environment(
        &self,
        project: &str,
        name: &str,
        by: &Principal,
        source: Source,
    ) -> Result<Environment> {
        let keeper = self.keeper_of(project).await;
        kept::make((&keeper.inner.own, &self.inner.environs), project, name, by, source).await
    }

    /// Changes `project`'s environment called `name` by `change`
    /// (`kept::change`).
    pub async fn change_environment(
        &self,
        project: &str,
        name: &str,
        change: impl FnOnce(&mut Environment) -> Result<Changed>,
        by: &Principal,
        source: Source,
    ) -> Result<(Environment, Changed)> {
        let keeper = self.keeper_of(project).await;
        kept::change((&keeper.inner.own, &self.inner.environs), project, name, change, by, source).await
    }

    /// Says a project's feature flag in one of its environments: its
    /// variable written, with what is said of it, as one write
    /// (`environments::flagged`).
    pub async fn set_flag(&self, flagging: Flagging<'_>, by: &Principal, source: Source) -> Result<Flagged> {
        let keeper = self.keeper_of(flagging.project).await;
        environments::flagged::set((&keeper.inner.own, &self.inner.environs), flagging, by, source).await
    }

    /// Keeps a change of a project's feature flag that waits for a
    /// go-ahead as the one asked last of it in its environment, and says
    /// it is asked, as one write (`environments::flagged::ask`).
    pub async fn ask_flag(
        &self,
        flagging: Flagging<'_>,
        asked: &str,
        by: &Principal,
        source: Source,
    ) -> Result<Flagged> {
        let keeper = self.keeper_of(flagging.project).await;
        environments::flagged::ask((&keeper.inner.own, &self.inner.environs), flagging, asked, by, source)
            .await
    }

    /// Keeps what `project`'s environment called `name` was last loaded
    /// with, and says so (`kept::ran`).
    pub async fn environment_ran(
        &self,
        project: &str,
        name: &str,
        ran: Ran,
        source: Source,
    ) -> Result<Environment> {
        let keeper = self.keeper_of(project).await;
        kept::ran((&keeper.inner.own, &self.inner.environs), project, name, ran, source).await
    }

    /// Says how the run `project`'s environment called `name` was last
    /// loaded by came out, once (`kept::ended`).
    pub async fn environment_ended(
        &self,
        (project, name): (&str, &str),
        run: &str,
        ended: Ended,
        source: Source,
    ) -> Result<kept::Said> {
        let keeper = self.keeper_of(project).await;
        kept::ended((&keeper.inner.own, &self.inner.environs), (project, name), run, ended, source).await
    }

    /// Says that what `project`'s environment called `name` was last loaded
    /// as was taken down, once (`kept::down`).
    pub async fn environment_down(
        &self,
        (project, name): (&str, &str),
        instance: &str,
        at: i64,
        source: Source,
    ) -> Result<kept::Said> {
        let keeper = self.keeper_of(project).await;
        kept::down((&keeper.inner.own, &self.inner.environs), (project, name), instance, at, source).await
    }

    /// Takes `project`'s environment called `name` away, refused while
    /// anything runs under it or an agent works under it, as `agents`
    /// comes to (`kept::remove`).
    pub async fn remove_environment(
        &self,
        (project, name): (&str, &str),
        agents: impl std::future::Future<Output = Result<Vec<String>>>,
        by: &Principal,
        source: Source,
    ) -> Result<Environment> {
        let (keeper, held) = (self.keeper_of(project).await, &self.inner.environs);
        let alive = keeper.inner.store.alive_of(Some(project));
        kept::remove((&keeper.inner.own, held), project, name, (alive, agents), (by, source)).await
    }

    /// What is held of every project's environments, with the turn at
    /// changing them (`environments::held`).
    pub fn environments(&self) -> &std::sync::Arc<environments::held::Held> {
        &self.inner.environs
    }
}

impl Engine {
    /// An apply over an instance of `project` that names its environment,
    /// from what is not under it already (outside, or a step of a run under
    /// none): held to its rules as one made under it is.
    pub(crate) async fn held_naming(
        &self,
        request: &Apply,
        within: Option<&str>,
        existing: &InstanceInfo,
        caller: Option<&Source>,
    ) -> Result<()> {
        let asking = request.parent.as_deref().or(within).and_then(|at| self.live(at));
        let under = asking.map(|asking| asking.meta().environment).unwrap_or_default();
        let Some(named) = request.environment.as_deref().filter(|named| !named.is_empty() && *named != under)
        else {
            return Ok(());
        };
        let by = request.principal.actor();
        let entering = Entering::of(&by, &request.params, caller.and_then(|step| step.run.as_deref()));
        self.inner.environs.holding(existing.project.as_deref(), named, Some(entering)).await.map(drop)
    }

    /// The turn for `info`, made under its environment (`Held::holding`),
    /// held to its rules where it `enters` it by naming it.
    pub(crate) async fn hold_made(
        &self,
        info: &InstanceInfo,
        enters: bool,
        caller: Option<&Source>,
    ) -> Result<Option<tokio::sync::MutexGuard<'_, ()>>> {
        let by = info.principal.actor();
        let entering =
            enters.then(|| Entering::of(&by, &info.params, caller.and_then(|step| step.run.as_deref())));
        self.inner.environs.holding(info.project.as_deref(), &info.environment, entering).await
    }

    /// What an instance of `project` applied under `environment` by
    /// `principal` reads as `environment` (`environments::held`): the
    /// marked variables alone where an agent applied it.
    pub(crate) fn environment_view(
        &self,
        project: Option<&str>,
        environment: &str,
        principal: &Principal,
    ) -> Value {
        self.inner.environs.view(project, environment, principal.kind == PrincipalKind::Agent)
    }

    /// The same, of an instance in hand.
    pub(crate) fn seen_by(&self, live: &Live) -> Value {
        let meta = live.meta();
        self.environment_view(meta.project.as_deref(), &meta.environment, &meta.principal)
    }

    /// Which secret `name`, written in the text of an instance in hand, is:
    /// the one name it is kept under (`environments::held::Held::secret`).
    pub(crate) fn secret_for(&self, live: &Live, name: &str) -> std::result::Result<String, String> {
        let meta = live.meta();
        let agent = meta.principal.kind == PrincipalKind::Agent;
        self.inner.environs.secret(meta.project.as_deref(), &meta.environment, name, agent)
    }

    /// The environment what `request` asks for runs under
    /// (`environments::held::chosen`): its parent's, the one it names, or
    /// none; refused where somebody names none in a project that has some.
    /// Whether it comes to it by naming it, and is held to its rules.
    pub(crate) async fn environment_for(
        &self,
        request: &Apply,
        parent: Option<&str>,
        person: bool,
    ) -> Result<(String, bool)> {
        let under = match parent.map(|parent| (parent, self.live(parent))) {
            None => None,
            Some((_, Some(live))) => Some(live.meta().environment),
            Some((parent, None)) => Some(
                self.inner.store.instance(parent).await?.map(|info| info.environment).unwrap_or_default(),
            ),
        };
        let key = request.key.as_deref().unwrap_or(&request.blueprint);
        let project = request.project.as_deref();
        match environments::held::chosen(under, request.environment.as_deref(), key, project, person)? {
            Chosen::Is(environment) => Ok((environment, false)),
            Chosen::Named(project, name) => {
                let held =
                    self.environment_of(&project, &name).await?.ok_or_else(|| missing(&project, &name))?;
                self.inner.environs.put(&project, &held);
                Ok((name, true))
            }
            Chosen::OneOf(project) => {
                Ok((kept::none_named(&self.keeper_of(&project).await.inner.own, &project).await?, false))
            }
        }
    }
}

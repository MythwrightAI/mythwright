//! What runs for a project and its agents, taken away with it once it is
//! deleted for good.
//!
//! Forgetting a project stops nothing: a project is let go of and taken up
//! again with its fleet where it was, and while it is in the Trash it is
//! given back whole. Deleted for good it is not coming back, and what was
//! left running for it went on bringing its agents up and writing its
//! tables, which a project made again under its name then found as its
//! own. So deleting one for good stops what was applied for it that is
//! still running (each at the top of its tree, which takes down what is
//! under it, a fleet's roles and agents among it), stops each agent that
//! works in it, which calls off what it was asked and waits for what was
//! at work to end, and then takes each away as a person taking one away
//! would ([`Agents::remove`]: its slot, its credential, its place in each
//! channel, its box, its folder), each said as `agent.slot.removed`. What
//! it said and did stays, as it does for any agent taken away.
//!
//! One that something still running for another project holds is left,
//! and said to be: it is that project's to change.
//!
//! Nobody gets by deleting a project what they do not hold: whoever
//! deletes it for good is asked first what stopping what runs for it and
//! taking each of its agents away by hand would ask of them
//! ([`Agents::may_forget_project`]), and refused before anything goes.

use std::time::Duration;

use conductor::{Engine, InstanceFilter};
use kernel::{Error, Principal, PrincipalKind, Result};

use crate::Agents;

/// How long what an agent had at work is waited for to end once it was
/// stopped, before it is left and said to be.
const WAITED: Duration = Duration::from_secs(60);

/// What deleting a project for good did to what ran for it and its agents.
#[derive(Clone, Debug, Default, serde::Serialize)]
pub struct Forgotten {
    /// What was applied for it and was running, by id: stopped.
    pub stopped: Vec<String>,
    /// Its agents that were taken away, by name.
    pub removed: Vec<String>,
    /// Its agents that were left, each with why.
    pub left: Vec<(String, String)>,
}

impl Agents {
    /// Stops what runs for `project` and takes its agents away, as the
    /// project is deleted for good by `by`, who was held to what that
    /// takes. Asked by whatever deletes a project for good, and nothing
    /// else.
    pub async fn forget_project(&self, project: &str, engine: &Engine, by: &Principal) -> Result<Forgotten> {
        let mut forgotten = Forgotten::default();
        for root in engine.instances(roots(project)).await? {
            engine.stop(&root.id).await?;
            forgotten.stopped.push(root.id);
        }
        for name in self.its(project).await? {
            self.deactivate(&name).await?;
            for turn in self.in_hand_now().await?.into_iter().filter(|turn| turn.slot == name) {
                let _ = tokio::time::timeout(WAITED, self.wait(&turn.turn)).await;
            }
            match self.taken_away(&name, by, engine, false).await {
                Ok(_) => forgotten.removed.push(name),
                Err(why) => forgotten.left.push((name, why.message)),
            }
        }
        Ok(forgotten)
    }

    /// Whether `by` may have `project` deleted for good, as things stand:
    /// it stops what runs for it and takes its agents away, so whoever
    /// deletes it is asked what doing each by hand asks. Stopping what
    /// was applied takes `blueprints:run` (the route that stops one), and
    /// taking an agent away a person who holds `agents:write` and `admin`
    /// ([`Agents::remove`]). A project with neither asks nothing more.
    /// Asked before anything is deleted, by whatever deletes a project
    /// for good for somebody.
    pub async fn may_forget_project(&self, project: &str, engine: &Engine, by: &Principal) -> Result<()> {
        let running = engine.instances(roots(project)).await?;
        if !running.is_empty() && !by.can("blueprints:run") {
            let what: Vec<String> =
                running.iter().map(|one| format!("{} as `{}`", one.blueprint, one.key)).collect();
            return Err(Error::denied(format!(
                "deleting `{project}` for good stops what runs for it ({}), and stopping that is for whoever \
                 may run blueprints (`blueprints:run`), which {} does not hold: somebody who does may delete it, \
                 or stop those first",
                what.join(", "),
                by.name
            )));
        }
        let its = self.its(project).await?;
        let person = by.kind == PrincipalKind::User && by.can("agents:write") && by.can("admin");
        if !its.is_empty() && !person {
            return Err(Error::denied(format!(
                "deleting `{project}` for good takes its agents away ({}), and taking an agent away is for a \
                 person who manages this factory (`agents:write` and `admin`), which {} is not: somebody who \
                 manages it may delete it, or take those agents away first",
                its.join(", "),
                by.name
            )));
        }
        Ok(())
    }

    /// The agents that work in `project`, by name.
    async fn its(&self, project: &str) -> Result<Vec<String>> {
        let slots = self.slots().await?.into_iter();
        Ok(slots.filter(|slot| slot.project.as_deref() == Some(project)).map(|slot| slot.name).collect())
    }
}

/// What asks for what was applied for `project` and still runs, each at
/// the top of its tree.
fn roots(project: &str) -> InstanceFilter {
    InstanceFilter {
        live_only: true,
        roots_only: true,
        project: Some(project.to_string()),
        limit: 100_000,
        ..InstanceFilter::default()
    }
}

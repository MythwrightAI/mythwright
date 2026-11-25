//! An agent moved to another project, or to none, and nothing else of it
//! changed.
//!
//! The project an agent works in is what its program is shown behind its
//! wall (that project's folder, or a home of its own where it is of none),
//! which board its cards are of and where it is listed; so moving it
//! changes those from its next turn, and leaves everything else of it as
//! it was: one that was stopped stays stopped, it holds what it held, and
//! its credential is the one it had, since what an agent may ask of the
//! factory says no project. The one thing that goes with the project is
//! the environment it works under, which is one of that project's
//! (`slot.environment`): moved, it works under none, as one added by hand
//! to a project before it had environments does, and is told which by
//! being said again with `--environment`.
//!
//! **Who may**: whoever may change agents (`agents:write`) and holds what
//! the agent holds, which is what saying anything of an agent asks
//! ([`Agents::ensure`]): nobody points an agent that may do more than
//! they may at another project's files.
//!
//! **What is in the way**, each said in a sentence that says what to do:
//!
//! - *Something that is running holds it* (a fleet, by its row): such an
//!   agent's project is its fleet's, and it is changed where it is
//!   counted. Which agents those are is what the instances say they hold.
//! - *It has work in hand*: waiting, at work, or answered and not yet
//!   said to have ended. That work was asked in the project it is of and
//!   is kept there; the move is refused until it is over, rather than
//!   waiting for it, so that whoever asked is told at once and nothing
//!   changes under them later.
//!
//! One this factory does not start moves as any does: it has no wall and
//! no turns, so what changes is where it is listed and whose cards it is
//! offered.

use conductor::Engine;
use kernel::{Error, Principal, Result, Source};
use serde_json::json;

use crate::{Agents, Brought, Slot};

impl Agents {
    /// Moves the agent `name` to `project`, or to no project, and changes
    /// nothing else of it. `engine` is asked what is running that holds it.
    pub async fn move_to(
        &self,
        name: &str,
        project: Option<String>,
        by: &Principal,
        engine: &Engine,
    ) -> Result<Slot> {
        by.require("agents:write")?;
        crate::own::not_its_own(name, by)?;
        let brought = self.brought_by(&blueprint::expr::slug(name), engine).await?;
        // Held as it is while one is brought up or given work: no turn is
        // queued for it in the project it is leaving between the look at
        // what it has in hand and the write.
        let _held = self.inner.choosing.lock().await;
        let mut slot = self.slot(name).await?;
        if !self.may_move(&slot, project.as_ref(), by, &brought).await? {
            return Ok(slot);
        }
        slot.project = project;
        // An environment is of the project it was left in.
        slot.environment = String::new();
        slot.updated = self.inner.journal.now_ms();
        self.write_slot(slot.clone()).await?;
        let data =
            json!({ "slot": slot.name, "adapter": slot.adapter, "project": slot.project, "wall": slot.wall });
        let _ = self.inner.journal.emit("agent.slot.updated", Source::actor(by.actor()), data).await;
        Ok(slot)
    }

    /// Whether `slot` may be moved to `project` by `by`, as things stand
    /// and with what brings it up: refused in a sentence where it may
    /// not, and `false` where it is there already.
    pub async fn may_move(
        &self,
        slot: &Slot,
        project: Option<&String>,
        by: &Principal,
        brought: &Brought,
    ) -> Result<bool> {
        by.require("agents:write")?;
        let name = &slot.name;
        crate::own::not_its_own(name, by)?;
        if let Some(beyond) = slot.scopes.iter().find(|scope| !by.can(scope)) {
            return Err(Error::denied(format!(
                "{} cannot move `{name}` to another project: it holds `{beyond}`, which {} does not",
                by.name, by.name
            )));
        }
        if let Some(project) = project {
            self.inner.studio.projects.get(project).await?;
        }
        if slot.project.as_ref() == project {
            return Ok(false);
        }
        let cli = brand::CLI;
        if let Brought::By(holder) = brought {
            let table = match &holder.project {
                Some(project) => format!("the `agents` table of the project `{project}`"),
                None => "its project's `agents` table".to_string(),
            };
            return Err(Error::conflict(format!(
                "`{name}` is brought up by `{key}` ({id}), which is running and counts it, so its project is \
                 that one's: an agent of a fleet is changed in its row, in {table}. One that something else \
                 brought up is moved once that is stopped (`{cli} stop {id}`)",
                key = holder.key,
                id = holder.id,
            )));
        }
        let in_hand = self.in_hand(std::iter::once(name.as_str())).await?;
        let in_hand = in_hand.get(name).copied().unwrap_or(0);
        if in_hand > 0 {
            return Err(Error::conflict(format!(
                "`{name}` has {in_hand} piece{s} of work in hand, asked in the project it works in now, and is \
                 moved once that is over: move it again when it has finished (`{cli} agent turns {name}` says \
                 what it has), or stop it, which calls that work off (`{cli} agent stop {name}`), and move it \
                 then",
                s = if in_hand == 1 { "" } else { "s" },
            )));
        }
        Ok(true)
    }
}

//! How an agent that is on the built-in assistant is given a program that
//! can do work: what somebody is told who has just been told that the
//! assistant cannot.
//!
//! It is said by the one command that does it, because the command is not
//! the same for every agent. One added by hand is said again with its
//! adapter (`agent add <name> --adapter <package>`). One a fleet brought up
//! runs on what its row of the project's `agents` says, in the cell
//! `adapter`, and saying it of the agent itself would be said over by the
//! fleet the next time it looks at its rows: so that agent is given its
//! row, by the row's own id. A fleet that was up before any package was
//! installed goes on with the assistant after one is, and "give it a real
//! agent" alone left whoever read it at a package installed and a card
//! that still could not be worked.
//!
//! Whose an agent is, is asked of what is running: an instance that brings
//! an agent up holds it (`agent:<name>`, which `agent.ensure` writes), as
//! taking one away asks ([`crate::removed`]). Which row it is of is not
//! kept anywhere, and no role or table is known here. What brings a row's
//! agents up is called after the row, so the record is asked for by what
//! follows each dash of that instance's key, and taken only where that is
//! a record of the agent's own project whose table has the cell. Where
//! nothing is found the cell is still named, and how its rows are listed.

use conductor::Engine;

use crate::{Agents, Brought, Slot};

/// The cell of a row that says what the row's agents run on: what
/// `agent.ensure` calls it, and so what a fleet's table does.
pub const CELL: &str = "adapter";

/// Whose an agent is, as far as saying what it runs on goes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Whose {
    /// Nothing running holds it: it was added by hand, and is said again.
    Hand,
    /// A row of a table: the record's id.
    Row(String),
    /// Something running holds it, and no row was found for it: the
    /// project it is of, where it is of one.
    Held(Option<String>),
    /// Nobody could be asked what is running.
    Unasked,
}

/// What to do so that `slot` runs on a program that can do work, as a
/// clause that goes on from "the assistant cannot do this".
pub fn how(slot: &str, whose: &Whose) -> String {
    let cli = brand::CLI;
    let first = format!("install an agent package (`{cli} help extensions`)");
    let rows = |project: &Option<String>| match project {
        Some(project) => format!("`{cli} record list {project} agents`"),
        None => format!("`{cli} record list <project> agents`"),
    };
    match whose {
        Whose::Hand => {
            format!("{first} and say that `{slot}` runs on it: `{cli} agent add {slot} --adapter <package>`")
        }
        Whose::Row(record) => format!(
            "`{slot}` is one of a fleet's and runs on what its row says: {first} and say it in the row's \
             `{CELL}` cell, `{cli} record set {record} {CELL}=<package>`"
        ),
        Whose::Held(project) => format!(
            "`{slot}` is brought up by something running and runs on what that says: {first}, and for an \
             agent of a fleet say it in the `{CELL}` cell of its row ({} lists the rows, then `{cli} record \
             set <row> {CELL}=<package>`)",
            rows(project)
        ),
        Whose::Unasked => format!(
            "{first} and say that `{slot}` runs on it: `{cli} agent add {slot} --adapter <package>`, or for \
             an agent of a fleet the `{CELL}` cell of its row in the project's `agents` (`{cli} record set \
             <row> {CELL}=<package>`)"
        ),
    }
}

impl Agents {
    /// Whose `slot` is, asked of what is running now.
    pub(crate) async fn whose(&self, slot: &Slot, engine: &Engine) -> Whose {
        let holder = match self.brought_by(&slot.name, engine).await {
            Err(_) => return Whose::Unasked,
            Ok(Brought::By(holder)) => holder,
            // Its own text, applied for it by hand, is said again the same way.
            Ok(Brought::Nobody | Brought::Own(_)) => return Whose::Hand,
        };
        for (dash, _) in holder.key.match_indices('-') {
            let named = &holder.key[dash + 1..];
            let Ok((table, record)) = self.inner.studio.collections.record(named).await else { continue };
            if Some(&table.project) == slot.project.as_ref() && table.field(CELL).is_some() {
                return Whose::Row(record.id);
            }
        }
        Whose::Held(slot.project.clone())
    }
}

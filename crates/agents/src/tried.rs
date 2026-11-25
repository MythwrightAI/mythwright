//! Trying an agent's program where it will work, before it is given any.
//!
//! An agent behind a wall is shown what its package said its program
//! needs, and what its slot added. Whether that is enough for a
//! particular program, installed a particular way on this device, is
//! found out by starting it there; and a piece of work is a dear way of
//! finding out. Most packages say how their program is asked which models
//! it offers, which sends it no prompt. Asked that as a turn would start
//! it, behind the same wall and with the same environment, a program that
//! answers has started, found what it is made of, and is signed in.

use kernel::{Error, Principal, Result};
use serde::Serialize;

use crate::Agents;
use crate::adapter::{Start, started};

/// What came of trying an agent's program.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Tried {
    pub slot: String,
    pub program: String,
    /// Whether it was started behind a wall, as its turns are.
    pub walled: bool,
    /// Whether it started and answered.
    pub ok: bool,
    /// How many models it said it offers.
    pub models: usize,
    /// Why it did not answer, with what it printed as it gave up.
    pub error: Option<String>,
}

impl Agents {
    /// Starts `name`'s program as a turn of its would, and asks it which
    /// models it offers. No prompt is sent, and nothing is asked of a
    /// model. What is wrong with asking at all is an error; a program that
    /// does not answer is what was found out.
    pub async fn try_program(&self, name: &str, by: &Principal) -> Result<Tried> {
        self.try_program_in(name, None, by).await
    }

    /// The same, started as a turn asked in `channel` would start it,
    /// where one is said: a channel kept to its members that `by`
    /// writes in and the agent is in, so in the home the agent has for
    /// that channel, shown what is shown there. That is where a program
    /// is found to start and to be signed in with what its package says
    /// is its sign-in (`crate::kept`).
    pub async fn try_program_in(&self, name: &str, channel: Option<&str>, by: &Principal) -> Result<Tried> {
        by.require("agents:write")?;
        let slot = self.slot(name).await?;
        let cli = brand::CLI;
        if slot.external() {
            return Err(Error::invalid(format!(
                "`{name}` is an external agent: this factory starts no program for it, so there is nothing to \
                 try. Whether it has joined is said by `{cli} agent list`"
            )));
        }
        let kept = match channel {
            Some(channel) => {
                let channels = &self.inner.studio.channels;
                let reader = channels.may_write(channel, by).await?;
                let id = reader.channel().unwrap_or(channel).to_string();
                if !channels.is_kept(&id) {
                    return Err(Error::invalid(format!(
                        "that channel is open, where `{name}` is started as it is anywhere: a channel is said to \
                         try it in the home it has for one kept to its members (`{cli} agent try {name}` tries it \
                         as it is started everywhere else)"
                    )));
                }
                self.may_ask(&slot, Some(&id))?;
                Some(id)
            }
            None => None,
        };
        let Some(package) = self.extension(&slot.adapter).filter(|package| package.agent) else {
            return Err(Error::invalid(format!(
                "`{name}` is not run by a package's program (it is on `{}`), so there is nothing to ask without giving it work",
                slot.adapter
            )));
        };
        if !package.answers_unprompted() {
            return Err(Error::invalid(format!(
                "the package `{}` does not say how its program is asked anything but a prompt (`models.ask`), so `{name}` can only be tried by giving it work",
                package.name
            )));
        }
        if !slot.signins.is_empty() {
            return Err(Error::invalid(format!(
                "`{name}` draws its sign-in from what the factory holds, and one is handed to a piece of work: give it one to try it (`{cli} post <channel> \"@{name} ...\"`)"
            )));
        }
        self.credential(&slot).await?;
        let began = self.inner.journal.now_ms();
        let workdir = self.workdir(&slot, None, kept.as_deref()).map_err(Error::invalid)?;
        let env = self.environment(&slot);
        let holds = self.kept_is_held(&slot).await;
        let behind = self.behind(&slot, &workdir, None, &env, (holds, kept.as_deref(), &[]));
        let start = Start {
            slot: &slot.name,
            workdir: &workdir,
            env: &env,
            without: &[],
            hosting: Some(&self.inner.hosting),
            behind: behind.as_ref(),
            kept: None,
            unsay: &[],
        };
        let answered = match started(&package.program, &start) {
            Ok(command) => package.asked(command, Some(&self.inner.hosting), true).await,
            Err(why) => Err(why),
        };
        let tried = Tried {
            slot: slot.name.clone(),
            program: package.program.clone(),
            walled: behind.is_some(),
            ok: answered.is_ok(),
            models: answered.as_ref().map_or(0, Vec::len),
            error: answered.err().map(|why| {
                match self.package_note(&slot.adapter).filter(|_| behind.is_some()) {
                    Some(note) => format!("{why}. {note}"),
                    None => why,
                }
            }),
        };
        let labels = [("slot", slot.name.as_str()), ("adapter", slot.adapter.as_str())];
        let took = (self.inner.journal.now_ms() - began) as f64;
        self.inner.journal.measure("agent.tried", &labels, took, tried.ok);
        Ok(tried)
    }
}

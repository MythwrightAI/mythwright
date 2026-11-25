//! An agent this factory does not start.
//!
//! Work is done on a factory from outside it too: by a program somebody
//! runs for themselves, on this device or another, that changes files
//! and moves cards with the factory's own command line. Signed in as
//! the person who runs it, everything it does says that person did it,
//! and no card it works shows anybody given it. An **external** agent
//! is such a program given a name of its own: a slot like any agent's,
//! listed with the others, given cards, in channels, acting as
//! `agent:<name>` with a credential that holds what the slot says.
//!
//! What it is not is started. The factory runs no program for it, holds
//! no wall round one, keeps it no home and no token, and gives it no
//! work: a turn is a program started, so it has none, ever. Every place
//! that gives work leaves it out or refuses it by name
//! ([`not_given`]): [`Agents::could_take`] and so `available` and what
//! a role is asked; a step that names it (`agent.task`, `agent.consult`);
//! [`Agents::enqueue_chosen`], which is where every turn is queued; a
//! message that names it, which starts nothing, is answered by nobody
//! and is put in its box for it to find ([`Agents::tell_named`]); `agent.ensure`, since it is never a row of a fleet's;
//! `agent try`, since there is nothing to try. In a channel kept to its
//! members, and so in a conversation, it is as a person is: it reads
//! what it is in with its own credential (`crate::kept`), and what is
//! said to it there is put in its box.
//!
//! That it is external is what its slot says runs it (`adapter`:
//! [`EXTERNAL`]), which no package and no program set up on a device
//! may be called. An agent is one or the other from when it is made.
//!
//! **Its credential** is this factory's, made for the slot, an agent's
//! like any (`people`): it names the slot and holds the slot's scopes.
//! The factory keeps its hash and nothing else, where a started agent's
//! is also in a file its program reads: nothing here is started to read
//! one. So it is come by with an invitation (`people::enrolled`), made
//! by whoever may add agents and holds what the agent is to hold
//! ([`Agents::invite`]), and handed over once by whoever holds it
//! ([`Agents::join`]), with no session: the invitation is what lets it
//! in. What it holds is changed where any agent's is, and since its
//! token cannot be written again, the credential it has is given the
//! new list where it is ([`studio::Identity::rescope`]): the bearer
//! holds that from its next request. Stopping it, or taking it away,
//! withdraws the credential and whatever invitation waits; an
//! invitation made again lets it in again, and the credential made as
//! that one is used replaces the one before.

use std::time::Duration;

use kernel::principal::AGENT_SCOPES;
use kernel::{Error, Principal, PrincipalKind, Result, Source};
use serde::Serialize;
use serde_json::json;
use studio::CredentialKind;

use crate::{Agents, Slot, SlotSpec};

/// What the slot of an agent this factory does not start says runs it
/// (`extensions::EXTERNAL`, which a package is read against).
pub use extensions::EXTERNAL;

impl Slot {
    /// Whether this is an agent the factory does not start.
    pub fn external(&self) -> bool {
        self.adapter == EXTERNAL
    }
}

/// What an external agent holds unless whoever invites it says: what an
/// agent usually holds, and its own inbox, which is where what names it
/// waits for it, since nothing is started to tell it.
pub fn usual() -> Vec<String> {
    AGENT_SCOPES.iter().copied().chain(["inbox"]).map(str::to_string).collect()
}

/// Why nothing is given to the external agent `name`, and what to do.
pub fn not_given(name: &str) -> String {
    format!(
        "`{name}` is an external agent: this factory starts no program for it and gives it no work. It takes \
         work up itself, with its own credential: give it a card (`{cli} board give <card> {name}`), or ask \
         an agent this factory starts",
        cli = brand::CLI
    )
}

/// Who to bring in from outside, and what it is to hold.
#[derive(Clone, Debug, Default)]
pub struct Invite {
    pub name: String,
    /// The project it is listed with; unsaid, as it was, and none for a new one.
    pub project: Option<String>,
    /// What it may ask of the factory; unsaid, as it was, and what an
    /// agent usually holds for a new one.
    pub scopes: Option<Vec<String>>,
    /// How long the invitation works; unsaid, a day.
    pub lasts: Option<Duration>,
}

/// An invitation made: shown once, and kept nowhere here.
#[derive(Clone, Debug, Serialize)]
pub struct Invitation {
    pub agent: Slot,
    pub invitation: String,
    /// When it stops working, in milliseconds since 1970.
    pub expires: i64,
}

/// What an invitation handed over is answered: the credential, once.
#[derive(Clone, Debug, Serialize)]
pub struct Joined {
    pub agent: String,
    pub token: String,
    pub scopes: Vec<String>,
    pub project: Option<String>,
}

/// How an external agent stands with the factory.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Standing {
    /// Whether it has a credential that is in use.
    pub joined: bool,
    /// When the invitation that waits for it stops working, where one waits.
    pub invited_until: Option<i64>,
}

impl Agents {
    /// Brings an agent in that this factory does not start, or invites
    /// one that is there again: its slot, and an invitation whoever is
    /// to be that agent hands over for its credential ([`Agents::join`]).
    ///
    /// Whoever asks may add agents and gives nothing they do not hold,
    /// as for any agent. One that was stopped is in again. An invitation
    /// that waited is withdrawn for this one; a credential the agent has
    /// goes on working until this invitation is used, and is replaced
    /// then.
    pub async fn invite(&self, asked: Invite, by: &Principal) -> Result<Invitation> {
        by.require("agents:write")?;
        let name = blueprint::expr::slug(&asked.name);
        if name.is_empty() {
            return Err(Error::invalid("an agent needs a name with at least one letter or digit"));
        }
        crate::own::not_its_own(&name, by)?;
        let _held = self.inner.choosing.lock().await;
        let existing = self.find(&name).await?;
        match &existing {
            Some(slot) if !slot.external() => return Err(Error::conflict(started(slot))),
            Some(_) => {}
            None => self.not_the_speakers(&name)?,
        }
        let scopes =
            asked.scopes.or_else(|| existing.as_ref().map(|slot| slot.scopes.clone())).unwrap_or_else(usual);
        may_give(&scopes, by)?;
        if let Some(project) = &asked.project {
            self.inner.studio.projects.get(project).await?;
        }
        let now = self.inner.journal.now_ms();
        let slot = Slot {
            name: name.clone(),
            adapter: EXTERNAL.to_string(),
            model: None,
            project: asked.project.or_else(|| existing.as_ref().and_then(|slot| slot.project.clone())),
            instructions: String::new(),
            scopes,
            // Said of a program, and there is none: the least of each.
            permission_mode: "read".to_string(),
            signins: Vec::new(),
            role: String::new(),
            rank: 0,
            wall: false,
            reads: Vec::new(),
            changes: Vec::new(),
            git: "off".to_string(),
            secrets: Default::default(),
            outlives: false,
            web: false,
            active: true,
            leaving: false,
            created: existing.as_ref().map_or(now, |slot| slot.created),
            updated: now,
            since: match &existing {
                Some(slot) => slot.since,
                None => self.last_of_name(&name).await?,
            },
            // Nothing is handed to a program the factory does not start.
            environment: String::new(),
        };
        // The invitation before the slot: one that cannot be made (it
        // was to work for too long) leaves nothing behind.
        let identity = &self.inner.studio.identity;
        let (invitation, expires) =
            identity.invite_to_enrol(CredentialKind::Agent, &name, &by.actor(), asked.lasts).await?;
        self.write_slot(slot.clone()).await?;
        identity.rescope(CredentialKind::Agent, &name, &slot.scopes).await?;
        let topic = if existing.is_some() { "agent.slot.updated" } else { "agent.slot.created" };
        let said = json!({ "slot": name, "adapter": EXTERNAL, "project": slot.project, "wall": false });
        let _ = self.inner.journal.emit(topic, Source::actor(by.actor()), said).await;
        let invited = self.of_slot(&name, json!({ "slot": name, "expires": expires })).await;
        let _ = self.inner.journal.emit("agent.slot.invited", Source::actor(by.actor()), invited).await;
        Ok(Invitation { agent: slot, invitation, expires })
    }

    /// Lets in whoever hands over `invitation`: the credential of the
    /// agent it was made for, shown only now. It is asked with no
    /// session, since the invitation is what lets its bearer in, and it
    /// answers one thing for an invitation that is not one, that was
    /// used or withdrawn, that stopped working, and whose agent is
    /// stopped or gone: nobody learns which from asking.
    ///
    /// One at a time: an invitation handed over twice at once lets in
    /// once. The credential is made before the invitation is used up,
    /// so a stop between the two leaves an invitation that still works,
    /// and using it replaces a credential nobody was handed.
    pub async fn join(&self, invitation: &str) -> Result<Joined> {
        let started = std::time::Instant::now();
        let joined = self.let_in(invitation).await;
        let took = started.elapsed().as_secs_f64() * 1000.0;
        self.inner.journal.measure("agent.join", &[], took, joined.is_ok());
        joined
    }

    async fn let_in(&self, invitation: &str) -> Result<Joined> {
        let refused = || {
            Error::denied(format!(
                "that invitation lets no agent in: it is not one this factory made, it was used or withdrawn, \
                 or it stopped working. One is good once and for a while. Ask whoever made it for another \
                 (`{} agent invite <name>`)",
                brand::CLI
            ))
        };
        let _held = self.inner.choosing.lock().await;
        let identity = &self.inner.studio.identity;
        let Some(name) = identity.enrolling(CredentialKind::Agent, invitation).await? else {
            return Err(refused());
        };
        let slot = self.find(&name).await?.filter(|slot| slot.external() && slot.active);
        let Some(slot) = slot else { return Err(refused()) };
        let token =
            identity.issue(CredentialKind::Agent, &slot.name, &slot.name, Some(slot.scopes.clone())).await?;
        identity.enrolled(invitation).await?;
        let said = self.of_slot(&slot.name, json!({ "slot": slot.name })).await;
        let _ =
            self.inner.journal.emit("agent.slot.joined", Source::actor(slot.principal().actor()), said).await;
        Ok(Joined { agent: slot.name, token, scopes: slot.scopes, project: slot.project })
    }

    /// An external agent gives its own credential up: it is nobody's
    /// from now, and the agent is let in again by an invitation. Asked
    /// by the agent itself, with the credential that goes.
    pub async fn unjoin(&self, who: &Principal) -> Result<()> {
        let external = match who.kind {
            PrincipalKind::Agent => self.find(&who.id).await?.filter(Slot::external),
            _ => None,
        };
        let Some(slot) = external else {
            return Err(Error::invalid(
                "only an external agent leaves, and it asks with its own credential: a person signs out, a \
                 key is withdrawn, and an agent this factory starts is stopped",
            ));
        };
        self.inner.studio.identity.revoke_subject(CredentialKind::Agent, &slot.name).await?;
        let said = self.of_slot(&slot.name, json!({ "slot": slot.name })).await;
        let _ = self.inner.journal.emit("agent.slot.unjoined", Source::actor(who.actor()), said).await;
        Ok(())
    }

    /// How an external agent stands: whether it has joined, and whether
    /// an invitation waits for it.
    pub async fn standing(&self, slot: &Slot) -> Result<Standing> {
        let identity = &self.inner.studio.identity;
        Ok(Standing {
            joined: identity.has_credential(CredentialKind::Agent, &slot.name).await?,
            invited_until: identity.invited_until(CredentialKind::Agent, &slot.name).await?,
        })
    }

    /// What is said again of an external agent by the way any agent is
    /// said (`ensure`, `give`): what it may ask, the project it is
    /// listed with, and a channel it is brought into. Anything said of
    /// a program is said of nothing, and is refused by its name.
    /// Whoever calls holds `choosing`.
    pub(crate) async fn said_of_external(
        &self,
        name: String,
        spec: SlotSpec,
        existing: Option<Slot>,
        by: &Principal,
        into: Option<studio::Reader>,
        standing: bool,
    ) -> Result<Slot> {
        let cli = brand::CLI;
        let Some(was) = existing else {
            return Err(Error::invalid(format!(
                "an external agent is invited, not added: `{cli} agent invite {name}` makes it, with the \
                 invitation it joins by (`{cli} help invite`)"
            )));
        };
        if !was.external() {
            return Err(Error::conflict(started(&was)));
        }
        let said = |list: &Option<Vec<String>>| list.as_ref().is_some_and(|list| !list.is_empty());
        let words = |word: &Option<String>| word.as_ref().is_some_and(|word| !word.is_empty());
        let of_a_program = [
            ("adapter", spec.adapter.as_deref().is_some_and(|adapter| adapter != EXTERNAL)),
            ("model", words(&spec.model)),
            ("instructions", words(&spec.instructions)),
            (
                "permission_mode",
                spec.permission_mode.as_ref().is_some_and(|mode| *mode != was.permission_mode),
            ),
            ("signins", said(&spec.signins)),
            ("role", words(&spec.role)),
            ("rank", spec.rank.is_some_and(|rank| rank != 0)),
            ("wall", spec.wall == Some(true)),
            ("reads", said(&spec.reads)),
            ("changes", said(&spec.changes)),
            ("git", spec.git.as_ref().is_some_and(|git| *git != was.git)),
            ("secrets", said(&spec.secrets)),
            ("outlives", spec.outlives == Some(true)),
            ("web", spec.web == Some(true)),
        ];
        if let Some((what, _)) = of_a_program.iter().find(|(_, said)| *said) {
            return Err(Error::invalid(format!(
                "`{name}` is an external agent: this factory starts no program for it, so `{what}` says nothing \
                 of it. What is said of one is what it may ask of the factory (`scopes`), the project it is \
                 listed with, and a channel it is in"
            )));
        }
        let scopes = spec.scopes.unwrap_or_else(|| was.scopes.clone());
        may_give(&scopes, by)?;
        if let Some(project) = &spec.project {
            self.inner.studio.projects.get(project).await?;
        }
        let slot = Slot {
            scopes,
            project: spec.project.or(was.project.clone()),
            // Brought in again, one that was stopped is in: it has no
            // credential until it is invited again. Told only what it
            // holds, it is as it was.
            active: !standing || was.active,
            updated: self.inner.journal.now_ms(),
            ..was
        };
        if let Some(into) = &into {
            self.may_be_in(into, &slot)?;
        }
        self.write_slot(slot.clone()).await?;
        self.inner.studio.identity.rescope(CredentialKind::Agent, &name, &slot.scopes).await?;
        if let (Some(reader), Some(channel)) = (&into, &spec.channel) {
            self.inner.studio.channels.bring(reader, channel, &name).await?;
        }
        let said = json!({ "slot": name, "adapter": EXTERNAL, "project": slot.project, "wall": false });
        let _ = self.inner.journal.emit("agent.slot.updated", Source::actor(by.actor()), said).await;
        Ok(slot)
    }
}

impl Agents {
    /// Puts a line in the box of each of these external agents that a
    /// message names (`studio::Inbox`), where the agent may read the
    /// channel it was said in: asked of the channel as any reading of it
    /// is, with what the agent holds, so one kept to its members that it
    /// is not in tells it nothing. Not the agent that said the message,
    /// and not one that is in the conversation it was said in, whose
    /// line of that conversation the message raised as it was written.
    /// It is the same done twice: a message heard again after a stop
    /// tells nobody a second time.
    pub(crate) async fn tell_named(
        &self,
        outside: &[Slot],
        mentioned: &[String],
        within: &studio::Channel,
        message: &str,
        author: &str,
    ) {
        let (studio, channel) = (&self.inner.studio, within.id.as_str());
        for slot in outside.iter().filter(|slot| mentioned.contains(&slot.name)) {
            let actor = format!("agent:{}", slot.name);
            let told = within.conversation && within.members.iter().any(|member| member.actor == actor);
            // What it holds is what its slot says: its credential holds
            // that from when it has one.
            if actor == author || told || studio.channels.may_read(channel, &slot.principal()).await.is_err()
            {
                continue;
            }
            let _ = studio
                .channels
                .inbox
                .tell(studio::inbox::Tell::named(&actor, message, channel, author))
                .await;
        }
    }
}

/// Why an agent this factory starts is not invited from outside.
fn started(slot: &Slot) -> String {
    format!(
        "`{name}` is an agent this factory starts (on `{adapter}`), and an agent is started here or external \
         from when it is made. Invite the one from outside under another name, or take `{name}` away first \
         (`{cli} agent remove {name}`)",
        name = slot.name,
        adapter = slot.adapter,
        cli = brand::CLI
    )
}

/// Whether `by` holds everything an agent is to hold: nobody gives what
/// they do not.
pub(crate) fn may_give(scopes: &[String], by: &Principal) -> Result<()> {
    match scopes.iter().find(|scope| !by.can(scope)) {
        Some(beyond) => {
            Err(Error::denied(format!("{} cannot give an agent `{beyond}`, not holding it", by.name)))
        }
        None => Ok(()),
    }
}

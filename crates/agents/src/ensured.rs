//! Bringing an agent in: its slot made, or brought in line with what is
//! said of it.
//!
//! One place makes a slot and changes one ([`Agents::ensure`], and
//! [`Agents::give`] for what an agent holds and nothing else of it), and
//! everything that may be said of an agent is held there to what whoever
//! says it holds: what it may ask of the factory, how freely its program
//! runs, whether it has a wall and what it is shown behind one, what it
//! signs in with and the keys it is handed, and the channel it is
//! brought into.

use kernel::principal::AGENT_SCOPES;
use kernel::{Error, Principal, Result, Source};
use rusqlite::params;
use serde_json::json;

use crate::external::{EXTERNAL, may_give};
use crate::{Agents, PERMISSIONS, SLOT, Slot, SlotSpec, keys, repository};

/// What a slot that is said comes to before anything is written.
enum Ready<'a> {
    Slot(Box<Said<'a>>),
    /// One this factory does not start, said of another way.
    Outside(Box<(String, SlotSpec, Option<Slot>, Option<studio::Reader>)>, tokio::sync::MutexGuard<'a, ()>),
}

struct Said<'a> {
    slot: Slot,
    existing: Option<Slot>,
    into: Option<studio::Reader>,
    channel: Option<String>,
    /// The turn at choosing, held from the reading to the writing.
    turn: tokio::sync::MutexGuard<'a, ()>,
}

impl Agents {
    /// Creates a slot, or brings an existing one in line with `spec`, and
    /// issues it a fresh credential.
    pub async fn ensure(&self, spec: SlotSpec, by: &Principal) -> Result<Slot> {
        self.brought(spec, by, false).await
    }

    /// Changes what an agent that is there holds, and nothing else of it:
    /// one that was stopped stays stopped, which bringing it in again
    /// would not leave it. Whoever asks is held to what `ensure` holds
    /// them to: nobody gives what they do not hold. Its credential is
    /// replaced, and its program reads that each time it asks the factory
    /// anything, so what it holds is the new list from then on, in a turn
    /// it has in hand as in its next.
    pub async fn give(&self, name: &str, scopes: Vec<String>, by: &Principal) -> Result<Slot> {
        let spec = SlotSpec { name: name.to_string(), scopes: Some(scopes), ..SlotSpec::default() };
        self.brought(spec, by, true).await
    }

    /// Whether `spec` could be brought about by `by` as things stand,
    /// refused in the sentence `ensure` would refuse it in, with nothing
    /// written. `standing`: only of one that is there, as [`Agents::give`].
    pub async fn may_ensure(&self, spec: SlotSpec, by: &Principal, standing: bool) -> Result<()> {
        self.said(spec, by, standing).await.map(drop)
    }

    /// The agents a migration noted as owed a text of their own, which a
    /// start looks at once (`api::adopted`).
    pub async fn owed_texts(&self) -> Result<Vec<String>> {
        let asked = |connection: &mut rusqlite::Connection| {
            let mut statement =
                connection.prepare("SELECT name FROM slots WHERE owes != '' ORDER BY name")?;
            let rows = statement.query_map([], |row| row.get(0))?;
            rows.collect::<rusqlite::Result<Vec<String>>>()
        };
        self.inner.journal.db().call(asked).await
    }

    /// Takes the note off an agent that has been looked at.
    pub async fn owes_no_text(&self, name: &str) -> Result<()> {
        let name = name.to_string();
        let written = move |c: &mut rusqlite::Connection| {
            c.execute("UPDATE slots SET owes = '' WHERE name = ?1", [name])
        };
        self.inner.journal.db().call(written).await?;
        Ok(())
    }

    /// `standing`: only one that is there, left in or stopped as it is.
    async fn brought(&self, spec: SlotSpec, by: &Principal, standing: bool) -> Result<Slot> {
        let Said { slot, existing, into, channel, turn } = match self.said(spec, by, standing).await? {
            Ready::Slot(said) => *said,
            Ready::Outside(outside, _held) => {
                let (name, spec, existing, into) = *outside;
                return self.said_of_external(name, spec, existing, by, into, standing).await;
            }
        };
        let name = slot.name.clone();
        // Under an environment, the turn at changing environments is held
        // as its row is written: it is not taken away in between
        // (`environments::kept::remove`), which looks at the agents that
        // work under it holding the same turn.
        let environs = match self.inner.environs.get() {
            Some(held) => held.holding(slot.project.as_deref(), &slot.environment, None).await?,
            None => None,
        };
        self.write_slot(slot.clone()).await?;
        drop(environs);
        drop(turn);

        // The slot keeps its credential while it still says the right thing:
        // applying a fleet again must not pull the token out from under an
        // agent in the middle of its work. It is replaced when what the agent
        // may do has changed, or when the file is gone or no longer good.
        self.credential(&slot).await?;
        if let Some(channel) = &channel {
            let reader = into.as_ref().expect("leave was asked for where a channel was said");
            self.inner.studio.channels.bring(reader, channel, &name).await?;
        }
        self.out_of_kept(&slot).await;
        let topic = if existing.is_some() { "agent.slot.updated" } else { "agent.slot.created" };
        let mut data =
            json!({ "slot": name, "adapter": slot.adapter, "project": slot.project, "wall": slot.wall });
        if !slot.role.is_empty() {
            data["role"] = json!(slot.role);
        }
        let _ = self.inner.journal.emit(topic, Source::actor(by.actor()), data).await;
        Ok(slot)
    }

    /// What `spec` comes to, held to what `by` holds, with nothing
    /// written: the slot as it would be, with the turn at choosing held.
    async fn said(&self, spec: SlotSpec, by: &Principal, standing: bool) -> Result<Ready<'_>> {
        by.require("agents:write")?;
        let channel = spec.channel.clone();

        // Telling an agent to draw a sign-in hands its program a secret.
        if spec.signins.as_ref().is_some_and(|draws| !draws.is_empty()) {
            by.require("secrets:use")?;
        }
        // An agent is not a way into a channel its maker may not speak in:
        // asked of the channel before anything is written, so that one
        // kept to its members is not there for a maker who is not one.
        let into = match &spec.channel {
            Some(channel) => Some(self.inner.studio.channels.may_write(channel, by).await?),
            None => None,
        };
        let name = blueprint::expr::slug(&spec.name);
        if name.is_empty() {
            return Err(Error::invalid("an agent needs a name with at least one letter or digit"));
        }
        crate::own::not_its_own(&name, by)?;
        // Held while it is brought up, as it is while one is stopped: one
        // that was leaving and is counted in again is not seen out by its
        // worker between being read here and being written.
        let turn = self.inner.choosing.lock().await;
        let existing = self.find(&name).await?;
        if standing && existing.is_none() {
            return Err(Error::not_found(format!("there is no agent called `{name}`")));
        }
        if existing.is_none() {
            self.not_the_speakers(&name)?;
        }
        // One this factory does not start is said of by another way, and
        // nothing below is said of it (`crate::external`).
        if existing.as_ref().is_some_and(Slot::external) || spec.adapter.as_deref() == Some(EXTERNAL) {
            return Ok(Ready::Outside(Box::new((name, spec, existing, into)), turn));
        }
        let (wall, reads, changes) = self.wall_of(&spec, existing.as_ref(), by)?;
        let outlives = self.outlives_of(&spec, existing.as_ref(), by)?;
        let web = self.web_of(&spec, existing.as_ref(), by)?;
        let git =
            repository::Git::said(spec.git.as_deref(), existing.as_ref().map(|slot| slot.git.as_str()))?;
        let adapter = spec
            .adapter
            .or_else(|| existing.as_ref().map(|slot| slot.adapter.clone()))
            .unwrap_or_else(|| self.default_adapter());
        if !self.inner.adapters.lock().expect("agents lock").contains_key(&adapter) {
            return Err(Error::invalid(format!(
                "there is no adapter called `{adapter}`; this device has: {}. Others come as extension packages (`{} help extensions`)",
                self.adapters().join(", "),
                brand::CLI
            )));
        }
        if let Some(mode) = &spec.permission_mode
            && !PERMISSIONS.iter().any(|(name, _)| name == mode)
        {
            let known: Vec<String> =
                PERMISSIONS.iter().map(|(name, means)| format!("`{name}` ({means})")).collect();
            return Err(Error::invalid(format!(
                "`{mode}` is not a permission mode; there are {}",
                known.join(", ")
            )));
        }
        // An agent never gets more than the one creating it has.
        let wanted = spec
            .scopes
            .or_else(|| existing.as_ref().map(|slot| slot.scopes.clone()))
            .unwrap_or_else(|| AGENT_SCOPES.iter().map(|scope| scope.to_string()).collect());
        may_give(&wanted, by)?;
        if let Some(project) = spec.project.as_ref().filter(|project| !project.is_empty()) {
            self.inner.studio.projects.get(project).await?;
        }
        // A role is a word, said in blueprints and on the command line.
        let word = |role: &str| {
            role.len() <= 40 && role.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        };
        if let Some(role) = spec.role.as_deref().filter(|role| !word(role)) {
            return Err(Error::invalid(format!(
                "`{role}` cannot be a role: a role is a word of lowercase letters, digits and `-`, such as `worker`"
            )));
        }
        // An agent is never run more freely than it was meant to be. Where
        // its program cannot be held to the mode asked for (or to the usual
        // one, when none was), that is said, and it is not run.
        let mode = spec
            .permission_mode
            .clone()
            .or_else(|| existing.as_ref().map(|slot| slot.permission_mode.clone()))
            .unwrap_or_else(|| "edit".to_string());
        // Held to nothing, its program runs commands as whoever runs the
        // daemon. Making such an agent, or saying anything of one that is
        // (what it is told, what runs it), is running commands by another
        // hand: it takes being allowed to run them.
        if mode == "full" {
            by.require("shell")?;
        }
        let held = self.inner.extensions.lock().expect("agents lock").get(&adapter).map(|package| {
            (
                if package.title.is_empty() { adapter.clone() } else { package.title.clone() },
                package.permissions(),
            )
        });
        if let Some((title, held)) = held
            && !held.contains(&mode)
        {
            let can: Vec<String> = PERMISSIONS
                .iter()
                .filter(|(name, _)| held.iter().any(|mode| mode == name))
                .map(|(name, means)| format!("`{name}` ({means})"))
                .collect();
            return Err(Error::invalid(format!(
                "an agent on {title} cannot be held to `{mode}`; its program can only be given {}. Choose that for this agent if it is what you mean",
                can.join(" or ")
            )));
        }
        // What it draws its sign-in from is checked when it is said, and
        // again when what runs it changes: its package has to say how its
        // program is handed one.
        let moved = existing.as_ref().is_some_and(|slot| slot.adapter != adapter);
        let signins = match (spec.signins, &existing) {
            (Some(draws), _) => self.check_draws_of(&adapter, &draws)?,
            (None, Some(slot)) if moved => self.check_draws_of(&adapter, &slot.signins)?,
            (None, Some(slot)) => slot.signins.clone(),
            (None, None) => Vec::new(),
        };
        // The keys it is handed of what the factory keeps, as said or as it had.
        let secrets = match &spec.secrets {
            Some(pairs) => keys::said(pairs)?,
            None => existing.as_ref().map(|slot| slot.secrets.clone()).unwrap_or_default(),
        };
        // One that draws a sign-in, or is handed a key, is handed a secret
        // each turn, whoever said so first: changing what it is told or
        // what runs it takes what telling it so does.
        if !signins.is_empty() || !secrets.is_empty() {
            by.require("secrets:use")?;
        }
        // Said empty, it is none: what a text applied for no project says of the agent it brings up.
        let project = match spec.project.clone() {
            Some(said) => Some(said).filter(|said| !said.is_empty()),
            None => existing.as_ref().and_then(|slot| slot.project.clone()),
        };
        let environment = match (spec.environment, &existing, project.as_deref()) {
            (Some(said), ..) => self.works_under(project.as_deref(), said)?,
            (None, Some(slot), _) => slot.environment.clone(),
            // A new one added by hand to a project that has environments
            // says which it works under, as anything applied there does.
            (None, None, Some(project)) => self.none_said(project)?,
            (None, None, None) => String::new(),
        };
        let now = self.inner.journal.now_ms();
        let keep = |new: Option<String>, old: fn(&Slot) -> Option<String>| {
            new.or_else(|| existing.as_ref().and_then(old))
        };
        let slot = Slot {
            name: name.clone(),
            adapter,
            model: keep(spec.model, |slot| slot.model.clone()).filter(|model| !model.is_empty()),
            project: project.clone(),
            instructions: keep(spec.instructions, |slot| Some(slot.instructions.clone())).unwrap_or_default(),
            scopes: wanted,
            permission_mode: mode,
            signins,
            role: keep(spec.role, |slot| Some(slot.role.clone())).unwrap_or_default(),
            rank: spec.rank.or_else(|| existing.as_ref().map(|slot| slot.rank)).unwrap_or(0),
            wall,
            reads,
            changes,
            git: git.word().to_string(),
            secrets,
            outlives,
            web,
            // Brought in again, one that was stopped is in and one that
            // was leaving stays; told only what it holds, it is as it was.
            active: !standing || existing.as_ref().is_none_or(|slot| slot.active),
            leaving: standing && existing.as_ref().is_some_and(|slot| slot.leaving),
            created: existing.as_ref().map_or(now, |slot| slot.created),
            updated: now,
            // A new agent is no heir of one its name had before.
            since: match &existing {
                Some(slot) => slot.since,
                None => self.last_of_name(&name).await?,
            },
            environment,
        };
        if let Some(into) = &into {
            self.may_be_in(into, &slot)?;
        }
        Ok(Ready::Slot(Box::new(Said { slot, existing, into, channel, turn })))
    }

    /// What a slot on `adapter` may draw its sign-in from: nothing, or
    /// what it says, checked against what its package can be handed.
    fn check_draws_of(&self, adapter: &str, draws: &[String]) -> Result<Vec<String>> {
        if draws.is_empty() { Ok(Vec::new()) } else { self.check_draws(adapter, draws) }
    }

    /// Refuses a new agent the name this factory itself speaks under.
    /// What names something in a channel is said to one thing. One that
    /// had the name before there was such a name goes on as it is: the
    /// two are then told apart by what they are, an agent and a bot.
    pub(crate) fn not_the_speakers(&self, name: &str) -> Result<()> {
        if name != self.inner.studio.channels.speaker.name() {
            return Ok(());
        }
        Err(Error::conflict(format!(
            "an agent cannot be called `{name}`: that is what this factory itself speaks as in a channel, and \
             what names it there is said to that. Choose another name for the agent, or call the factory's \
             speaker something else first (`{} channel speaker <name>`)",
            brand::CLI
        )))
    }

    /// Writes a slot's row, whole: a new one, or over the one of its name.
    pub(crate) async fn write_slot(&self, row: Slot) -> Result<()> {
        let (name, outside, outlives) = (row.name.clone(), row.external(), row.outlives);
        self.inner
            .journal
            .db()
            .call(move |connection| {
                connection.execute(
                    &format!("INSERT OR REPLACE INTO slots ({SLOT}) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23)"),
                    params![
                        row.name,
                        row.adapter,
                        row.model,
                        row.project,
                        row.instructions,
                        serde_json::to_string(&row.scopes).unwrap_or_default(),
                        row.permission_mode,
                        row.active,
                        row.created,
                        row.updated,
                        serde_json::to_string(&row.signins).unwrap_or_default(),
                        row.role,
                        row.leaving,
                        row.rank,
                        row.wall,
                        serde_json::to_string(&row.reads).unwrap_or_default(),
                        serde_json::to_string(&row.changes).unwrap_or_default(),
                        row.git,
                        serde_json::to_string(&row.secrets).unwrap_or_default(),
                        row.outlives,
                        row.web,
                        row.since,
                        row.environment
                    ],
                )
            })
            .await?;
        // Once its row says so: what answers whether an agent is of a
        // kept channel as a person is.
        self.inner.hand.is_outside(&name, outside);
        // And what a stop does to its program from now: a turn in hand too.
        self.inner.outliving.is(&name, outlives);
        Ok(())
    }
}

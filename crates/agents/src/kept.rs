//! An agent in a channel kept to its members.
//!
//! A channel may be kept to its members (`talk::kept`), and an agent that
//! is in one is a way out of it unless it is held: anybody who may ask
//! it elsewhere could have it read the channel out, what it was asked
//! there and what it answered are its turns', and its key reads whatever
//! it is in. What holds it is here.
//!
//! **Its key opens the channel only while a turn asked there is in
//! hand.** A turn is of a kept channel where its conversation is that
//! channel's: it was asked there, it says its answer there, or it goes
//! on with a conversation held there ([`Agents::kept_of`]). Which that
//! is, is written on the turn as it is queued (`turns.kept`) and is
//! never another afterwards. While such a turn is in hand this crate
//! remembers it ([`Hand`]): from just before the write that says it
//! began, so that whoever hears that it began finds the channel open to
//! its agent (given back where that write did not begin it), until just
//! before the write that says it ended. That memory is what `talk` asks
//! each time the agent reads the channel or is handed an event of it
//! (`Channels::asked_with`). It is filled from the turns' rows as agents are
//! opened, so a daemon that comes back opens the channel to an agent
//! again only for the turn it finds in hand. An agent does one thing at
//! a time, so while its program is at work on something asked elsewhere
//! no turn of a kept channel is in hand, and its key opens none.
//!
//! **An agent the factory does not start is in one as a person is.**
//! It is somebody's program outside the factory, as a person with a key
//! is: nothing is started for it, so nothing of what holds a started
//! agent's program applies to it (no wall, no home, no sign-in, no turn),
//! and what it reads there it reads as whoever runs it does. Its own
//! credential opens a kept channel it is in whenever it asks, holding
//! the channels' scope, and no other; which agents are such is
//! remembered here too ([`Hand::outside`]), from the slots as agents
//! are opened and as one is invited or taken away, and is what `talk`
//! asks beside whether a turn is in hand (`Channels::outside_with`).
//!
//! **An agent with no wall is in no kept channel.** Its program sees
//! this device as whoever runs the daemon does, and what it read there
//! is in the one home every turn of its has. It is refused where it
//! would be brought into one, an agent that is in one and is then told
//! to work with no wall leaves it, which is said there, and so does
//! every agent of a factory that is started holding no walls.
//!
//! **An agent may be one kept channel's alone.** A channel may say that
//! its agents are asked nowhere else (`talk::kept`), and an agent that
//! is in such a one is given no work anywhere but there: not by a
//! message said to it in another channel, where the factory says why
//! it was not asked, not by a step that names it, and not as whichever
//! agent of a role is free ([`Agents::alone_in`], asked where work is
//! queued and where those who could take it are listed). An agent is
//! brought into no second kept channel while one holds it so, nor into
//! one that would while it is in another.
//!
//! **Its program has a home of its own for each kept channel.** Behind
//! its wall an agent's program is shown a home of its own where the home
//! of whoever runs the daemon is (`crate::walled`), and that is where a
//! program keeps what it was asked and what it made of it. A turn of a
//! kept channel is shown the home that is that channel's
//! ([`Agents::home_for`]), and a turn of anything else the one it always
//! had, so neither finds what the other left; an agent that works in no
//! project works in a folder of that channel's too. Its sign-in is
//! carried: a key or a token the factory holds is a value it is handed
//! each turn, and one that signs in as whoever runs the daemon is shown,
//! in every home, what its package says is its sign-in
//! (`agent.wall.signin`), where it is, so that a sign-in its program
//! renews in one home is the one every other reads.
//!
//! **An agent whose sign-in cannot be told from what it is asked is in
//! no kept channel.** A program that keeps its sign-in and its
//! conversations in one place keeps both there whatever home it is
//! shown, and that place is not the agent's alone: every agent signed
//! in the same way is shown it. Such is one that signs in as whoever
//! runs the daemon while its package does not say which of what it
//! keeps is its sign-in, one the daemon's own environment names a
//! folder for, and one that may be handed a folder the factory holds
//! ([`Agents::unparted`], asked as things stand each time). It is
//! refused where it would be brought in, in a sentence that says the
//! ways out, and one that comes to be so while it is in a kept channel
//! (what it signs in with was changed, its package was replaced by one
//! that says less, a folder was added to what it may draw, the daemon
//! was started another way) leaves it, which is said there, as one told
//! to work with no wall does ([`Agents::settle_kept`]).
//!
//! **What it kept of a channel goes with its place in it.** When an
//! agent leaves a kept channel ([`Agents::leave_channel`], or by being
//! told to work with no wall), when the channel is taken away
//! ([`Agents::forget_kept`]) and when the agent is (`crate::removed`),
//! the folder that was its own for that channel goes, and so does what
//! its program remembered of the conversations held there
//! (`agent.session`); with a turn of the channel still in hand, when
//! that turn has ended. Of a channel that is taken away, the turns that
//! stay as work a step asked for keep when they were and what they
//! cost, and none of their words. A copy of a factory takes none of
//! these folders: no agent's folder is in one.
//!
//! **What is said of such a turn is said to the channel's members.**
//! That it was queued, began and ended, and what its program prints as
//! it works, take being a member, as what is said in the channel does.
//! Somebody who does not read the channel is shown of the turn when it
//! was, how long it took and what it cost, and none of its words
//! ([`Agents::turns_for`]).

use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::PathBuf;
use std::sync::RwLock;

use kernel::{Error, Principal, Result};
use rusqlite::params;

use crate::{Agents, Slot, TurnInfo};

/// The folder of an agent's own in which what is its own for each kept
/// channel is, a folder each, called by the channel's id.
pub(crate) const PLACES: &str = "kept";

/// The turns in hand that are of a kept channel: for each, whose it is
/// and which channel.
#[derive(Default)]
pub(crate) struct Hand {
    asked: RwLock<HashMap<String, (String, String)>>,
    /// The agents this factory does not start, each as an actor: of a
    /// kept channel it is in with no turn, as a person is.
    outside: RwLock<HashSet<String>>,
}

impl Hand {
    /// Whether the agent that is `actor` has a turn of `channel` in hand.
    pub(crate) fn has(&self, channel: &str, actor: &str) -> bool {
        let asked = self.asked.read().expect("agents lock");
        asked.values().any(|(whose, of)| whose == actor && of == channel)
    }

    /// Whether the agent that is `actor` is one this factory does not
    /// start.
    pub(crate) fn outside(&self, actor: &str) -> bool {
        self.outside.read().expect("agents lock").contains(actor)
    }

    /// The agent called `slot` is one this factory does not start, or is
    /// not, or is gone: said once its row says so.
    pub(crate) fn is_outside(&self, slot: &str, outside: bool) {
        let (actor, mut known) = (format!("agent:{slot}"), self.outside.write().expect("agents lock"));
        match outside {
            true => known.insert(actor),
            false => known.remove(&actor),
        };
    }

    /// The turn `id`, of the agent called `slot` and of `channel`, is in
    /// hand.
    pub(crate) fn takes(&self, id: &str, slot: &str, channel: &str) {
        let of = (format!("agent:{slot}"), channel.to_string());
        self.asked.write().expect("agents lock").insert(id.to_string(), of);
    }

    /// The turn `id` is in hand no longer: said before its row says so.
    pub(crate) fn gives_back(&self, id: &str) {
        self.asked.write().expect("agents lock").remove(id);
    }

    /// The turn `id` taken in hand as it is about to be written as
    /// begun, where it is of a kept channel: whoever hears that it began
    /// is told so in that write, and finds the channel open to its
    /// agent. It is given back unless the write came to say so.
    pub(crate) fn taking<'a>(&'a self, id: &'a str, slot: &str, kept: Option<&str>) -> Taking<'a> {
        if let Some(channel) = kept {
            self.takes(id, slot, channel);
        }
        Taking { hand: self, id, begun: false }
    }
}

/// A turn in hand whose row does not say so yet. Let go of before the
/// row does (the write was not made, or began nothing, or whoever asked
/// for it went away), it is in hand no longer: the memory may say less
/// than the rows for a moment, and never goes on saying more.
pub(crate) struct Taking<'a> {
    hand: &'a Hand,
    id: &'a str,
    begun: bool,
}

impl Taking<'_> {
    /// Its row says it began.
    pub(crate) fn begun(mut self) {
        self.begun = true;
    }
}

impl Drop for Taking<'_> {
    fn drop(&mut self) {
        if !self.begun {
            self.hand.gives_back(self.id);
        }
    }
}

impl TurnInfo {
    /// What hearing something said of it takes: nothing, or being a
    /// member of the kept channel it is of.
    pub(crate) fn takes(&self) -> Option<String> {
        self.kept.as_deref().map(studio::Channels::kept_takes)
    }

    /// The same turn with its words left out: what somebody who does
    /// not read the kept channel it is of is shown of it. When it was,
    /// how long it took, how it ended and what it cost stay.
    fn without_its_words(self) -> TurnInfo {
        TurnInfo {
            thread: String::new(),
            prompt: String::new(),
            channel: None,
            reply_to: None,
            options: Vec::new(),
            reply: String::new(),
            error: None,
            decision: None,
            refused: Vec::new(),
            labels: Default::default(),
            cwd: None,
            hidden: Vec::new(),
            kept_from: true,
            asked: None,
            ..self
        }
    }
}

/// Why a program that signs in as whoever runs the daemon has to be
/// shown what it keeps in their home as one place, where it has to: its
/// sign-in cannot then be told from what it is asked. Such an agent is in
/// no channel kept to its members ([`Agents::unparted`]), and has no home
/// of its own ([`Agents::behind`]): its program is shown what its package
/// says it keeps (`agent.wall.keeps`), and is told what it is and how it
/// stands ahead of what it is asked. Any other is shown, in a home of its
/// own, only what its package says is its sign-in (`agent.wall.signin`).
pub(crate) enum Theirs {
    /// The daemon was started with this variable naming a folder for
    /// the program, where it keeps everything.
    Named(String),
    /// Its package says what it keeps there and not which of it is its
    /// sign-in.
    Unsaid,
}

/// Why `package`'s program, signing in as whoever runs the daemon, has to
/// be shown what it keeps in their home whole, where it has to. Asked as
/// things stand: the daemon started another way may change it.
pub(crate) fn theirs(package: &crate::Extension) -> Option<Theirs> {
    let named = package.signin.iter().flat_map(|by| by.variables()).find(|variable| {
        std::env::var_os(variable).is_some_and(|folder| std::path::Path::new(&folder).is_dir())
    });
    if let Some(variable) = named {
        return Some(Theirs::Named(variable));
    }
    let needs = &package.wall;
    (!needs.keeps.is_empty() && needs.signin.is_empty()).then_some(Theirs::Unsaid)
}

impl Agents {
    /// Why `slot`'s program works with no wall, where it does: its own
    /// row says so, or this factory holds none whatever a row says.
    fn unwalled(&self, slot: &str, wall: bool) -> Option<String> {
        if !wall {
            return Some(format!(
                "`{slot}` works with no wall, where its program sees this device as whoever runs the daemon does"
            ));
        }
        (self.inner.hosting.walls() == hosting::Walls::Open).then(|| {
            format!(
                "this factory was started holding no walls, so the program of `{slot}` sees this device as \
                 whoever runs the daemon does"
            )
        })
    }

    /// Why the sign-in of `slot`'s program cannot be told from what it
    /// is asked, where it cannot, and what would let it be: its program
    /// then keeps both in one place whatever home it is shown, and every
    /// agent signed in the same way is shown that place. Not one that
    /// no package runs (the built-in assistant, a program set up on
    /// this device), which is shown nothing but what is its own; not
    /// one handed a key or a token, which is a value; and not one whose
    /// package says which of what it keeps is its sign-in. Asked as
    /// things stand: a sign-in added, a package taken or the daemon
    /// started another way may change it.
    pub(crate) fn unparted(&self, slot: &Slot) -> Option<(String, String)> {
        let package = self.extension(&slot.adapter).filter(|package| package.agent)?;
        let (cli, name, adapter) = (brand::CLI, &slot.name, &slot.adapter);
        let a_value = format!(
            "the agent draws a key or a token the factory holds, which is a value it is handed and keeps \
             nowhere (`{cli} signin add <name> --for {adapter} --kind key`, then `{cli} agent add {name} \
             --signin <name>`; `{cli} help signin`)"
        );
        if !slot.signins.is_empty() {
            return self.may_draw_a_folder(slot).then(|| {
                let why = format!(
                    "`{name}` may be handed a sign-in that is a folder the factory holds, where its program \
                     keeps what it is asked beside its sign-in, for every agent that is handed the folder"
                );
                let way = format!(
                    "Have it draw a key or a token by its name, and no folder (`{cli} agent add {name} --signin \
                     <name>`; `{cli} help signin`)"
                );
                (why, way)
            });
        }
        // Signing in as whoever runs the daemon.
        match theirs(&package)? {
            Theirs::Named(variable) => {
                let why = format!(
                    "the daemon was started with `{variable}` naming a folder for the program of `{name}`, \
                     where it keeps what it is asked beside its sign-in, for every agent signed in that way"
                );
                Some((why, format!("Either the daemon is started without `{variable}`, or {a_value}")))
            }
            Theirs::Unsaid => {
                let why = format!(
                    "`{name}` signs in as whoever runs the daemon, and its package `{adapter}` does not say which \
                 of what its program keeps there is its sign-in, so what it is asked is kept beside its \
                 sign-in, for every agent signed in that way"
                );
                // The copy installed here may be from before the package said so.
                let came = self.inner.config.offered.as_ref().map(|dir| crate::extension::load_all(dir).0);
                let came = came.and_then(|mut came| came.remove(adapter));
                let newer = match came.is_some_and(|came| !came.wall.signin.is_empty()) {
                    true => format!(
                        ". The package `{adapter}` that came with this installation says it, and the one installed \
                     here is an earlier copy: `{cli} extension update {adapter}` takes the one that came"
                    ),
                    false => String::new(),
                };
                let way = format!(
                    "Either its package says which of what it keeps is its sign-in (`agent.wall.signin`; `{cli} \
                 help wall`), or {a_value}{newer}"
                );
                Some((why, way))
            }
        }
    }

    /// Why `slot` may be in no channel kept to its members as things
    /// stand, where it may not: what is so of it, the rule, and the way
    /// out.
    pub(crate) fn kept_out(&self, slot: &Slot) -> Option<(String, &'static str, String)> {
        // One this factory does not start is somebody's program outside
        // it, as a person with a key is: nothing here is started that a
        // wall, a home or a sign-in could hold, and it is in one as a
        // person is.
        if slot.external() {
            return None;
        }
        if let Some(why) = self.unwalled(&slot.name, slot.wall) {
            let way = format!(
                "Give it a wall (`{} agent add {} --wall`), or bring it into a channel that is open",
                brand::CLI,
                slot.name
            );
            return Some((why, "an agent with no wall is in no channel kept to its members", way));
        }
        let (why, way) = self.unparted(slot)?;
        let rule = "an agent whose sign-in cannot be told from what it is asked is in no channel kept to its \
                    members";
        Some((why, rule, way))
    }

    /// The kept channel a turn asked in `channel`, of the conversation
    /// `thread`, is of, by its id: the one the conversation is held in,
    /// where it is one of a channel's (`channel:<id>`, or `thread:<the
    /// message it hangs from>`, as [`crate::heard`] names them), else
    /// the one it says its answer in. Nothing for a turn of no kept
    /// channel, which in a factory with none is asked of nothing but
    /// that.
    pub(crate) async fn kept_of(&self, channel: Option<&str>, thread: &str) -> Option<String> {
        let channels = &self.inner.studio.channels;
        if !channels.keeps_any() {
            return None;
        }
        // A kept channel is called its own id, so what names one is that.
        let kept = |named: &str| {
            let id = named.trim_start_matches('#');
            channels.is_kept(id).then(|| id.to_string())
        };
        let held_in = match (thread.strip_prefix("channel:"), thread.strip_prefix("thread:")) {
            (Some(named), _) => kept(named),
            (None, Some(message)) => {
                let own = studio::Channels::daemon(
                    "which channel a conversation named by the message it hangs from is held in, so that a \
                     turn that goes on with one of a kept channel is kept as that channel's",
                );
                let said = channels.message(&own, message).await.ok();
                said.and_then(|said| kept(&said.channel))
            }
            (None, None) => None,
        };
        held_in.or_else(|| channel.and_then(kept))
    }

    /// Whether whoever applied a step may have an agent go on with the
    /// conversation `thread`: one that is a channel's is read by having
    /// an agent answer in it, so it takes reading that channel, and one
    /// of a channel kept from them is not there.
    pub(crate) async fn may_go_on(&self, thread: &str, by: &Principal) -> Result<()> {
        let channels = &self.inner.studio.channels;
        if let Some(channel) = thread.strip_prefix("channel:") {
            channels.may_read(channel, by).await?;
        } else if let Some(message) = thread.strip_prefix("thread:") {
            channels.may_read_message(message, by).await?;
        }
        Ok(())
    }

    /// The folder that is `slot`'s own for the kept channel `kept`: its
    /// home there is in it, and it is where an agent that works in no
    /// project works on what it is asked there.
    pub(crate) fn place_for(&self, slot: &str, kept: &str) -> PathBuf {
        self.inner.config.home.join(slot).join(PLACES).join(kept)
    }

    /// The home of its own `slot`'s program is shown for a turn: the one
    /// that is the kept channel's the turn is of, or the one it has for
    /// everything else.
    pub(crate) fn home_for(&self, slot: &str, kept: Option<&str>) -> PathBuf {
        match kept {
            Some(kept) => self.place_for(slot, kept).join("home"),
            None => self.inner.config.home.join(slot).join("home"),
        }
    }

    /// The kept channel `slot` is asked nowhere but in, by its id, where
    /// it is in one that says so of its agents. Never handed to
    /// anybody: what is said of it names no channel.
    pub(crate) fn alone_in(&self, slot: &Slot) -> Option<String> {
        let channels = &self.inner.studio.channels;
        match channels.keeps_any() {
            true => channels.alone_in(&slot.principal().actor()),
            false => None,
        }
    }

    /// Whether `slot` may be given a turn of the kept channel `kept`,
    /// or of none.
    pub(crate) fn may_ask(&self, slot: &Slot, kept: Option<&str>) -> Result<()> {
        if let Some(alone) = self.alone_in(slot)
            && kept != Some(alone.as_str())
        {
            return Err(Error::conflict(format!(
                "`{}` is in a channel kept to its members whose agents are asked nowhere else, and this is \
                 asked elsewhere: ask another agent, or ask it there",
                slot.name
            )));
        }
        let Some(kept) = kept else { return Ok(()) };
        if let Some((why, rule, way)) = self.kept_out(slot) {
            return Err(Error::conflict(format!("{why}, and {rule}: it is asked nothing in one. {way}")));
        }
        let actor = slot.principal().actor();
        if !self.inner.studio.channels.kept_with(&actor).iter().any(|within| within == kept) {
            return Err(Error::conflict(format!(
                "`{}` is not in that channel, which is kept to its members: an agent is asked there once \
                 somebody who is in it has brought it in",
                slot.name
            )));
        }
        Ok(())
    }

    /// Whether `slot`, as it is about to be written, may be brought
    /// into the channel `into` is leave for: into one kept to its
    /// members only with a wall and a sign-in that can be told from
    /// what it is asked, and only where that leaves it held to one
    /// channel at most.
    pub(crate) fn may_be_in(&self, into: &studio::Reader, slot: &Slot) -> Result<()> {
        let channels = &self.inner.studio.channels;
        let kept = into.channel().is_some_and(|channel| channels.is_kept(channel));
        if let Some((why, rule, way)) = self.kept_out(slot).filter(|_| kept) {
            return Err(Error::conflict(format!("{why}, and {rule}. {way}")));
        }
        channels.may_bring(into, &slot.name)
    }

    /// Forgets what the agent called `slot` kept of the kept channel
    /// `kept`: the folder that was its own for it, and what its program
    /// remembered of each conversation held there. Not while a turn of
    /// the channel is in hand, whose program is at work in that folder:
    /// then when the turn has ended ([`Agents::left_behind`]).
    pub(crate) async fn forget_place(&self, slot: &str, kept: &str) {
        if self.inner.hand.has(kept, &format!("agent:{slot}")) {
            return;
        }
        self.forget_said(slot, kept).await;
        // The folder last: gone, nothing of the channel is the agent's.
        let _ = std::fs::remove_dir_all(self.place_for(slot, kept));
    }

    /// Forgets what the program of the agent called `slot` remembered
    /// of each conversation held in the kept channel `kept`: those its
    /// turns of that channel went on with, and the channel's own.
    pub(crate) async fn forget_said(&self, slot: &str, kept: &str) {
        let (lookup, of) = (slot.to_string(), kept.to_string());
        let asked = move |connection: &mut rusqlite::Connection| {
            let mut statement =
                connection.prepare("SELECT DISTINCT thread FROM turns WHERE slot = ?1 AND kept = ?2")?;
            let rows = statement.query_map([&lookup, &of], |row| row.get::<_, String>(0))?;
            rows.collect::<rusqlite::Result<Vec<String>>>()
        };
        let held = self.of_each(self.may_hold(slot), asked).await.unwrap_or_default();
        let mut threads: BTreeSet<String> = held.into_iter().flatten().collect();
        threads.insert(format!("channel:{kept}"));
        let name = slot.to_string();
        // A conversation is remembered by the agent, a `/`, its name,
        // and where it was held in another folder an `@` and that.
        let forgotten = self.inner.journal.db().call(move |connection| {
            for thread in threads {
                let key = format!("{name}/{thread}");
                connection.execute(
                    "DELETE FROM kv WHERE namespace = ?1 AND (key = ?2 OR (key >= ?3 AND key < ?4))",
                    params![crate::removed::SESSIONS, key, format!("{key}@"), format!("{key}A")],
                )?;
            }
            Ok(())
        });
        let _ = forgotten.await;
    }

    /// Adds to `slots` each agent that has a folder of its own for the
    /// kept channel `channel`, asked anything there or not.
    pub(crate) fn with_a_place(&self, channel: &str, slots: &mut BTreeSet<String>) {
        for folder in std::fs::read_dir(&self.inner.config.home).into_iter().flatten().flatten() {
            if folder.path().join(PLACES).join(channel).exists() {
                slots.insert(folder.file_name().to_string_lossy().into_owned());
            }
        }
    }

    /// What `turn`, which has ended, was of is a kept channel its agent
    /// is in no longer: what it kept of it goes now, which could not
    /// while its program was at work there.
    pub(crate) async fn left_behind(&self, turn: &TurnInfo) {
        let Some(kept) = &turn.kept else { return };
        let actor = format!("agent:{}", turn.slot);
        if !self.inner.studio.channels.kept_with(&actor).iter().any(|within| within == kept) {
            self.forget_place(&turn.slot, kept).await;
        }
    }

    /// Takes the agent called `name` out of `channel`, as `by`, who may
    /// change agents and write in that channel: of one kept to its
    /// members, one of them. What it kept of a kept one goes with it.
    pub async fn leave_channel(&self, channel: &str, name: &str, by: &Principal) -> Result<()> {
        by.require("agents:write")?;
        let channels = &self.inner.studio.channels;
        let reader = channels.may_write(channel, by).await?;
        let slot = self.slot(name).await?;
        let id = reader.channel().unwrap_or(channel).to_string();
        channels.leave(&reader, &id, &slot.principal().actor()).await?;
        self.forget_place(&slot.name, &id).await;
        Ok(())
    }

    /// Forgets what every agent kept of the channel `channel`, which
    /// has been taken away: each one's folder for it and what its
    /// program remembered of what was said there; and takes the words
    /// out of the turns of it that stay, work a step asked for, which
    /// nobody reads the channel of any more.
    pub(crate) async fn forget_kept(&self, channel: &str) -> Result<()> {
        let of = channel.to_string();
        let asked = move |connection: &mut rusqlite::Connection| {
            let mut statement = connection.prepare("SELECT DISTINCT slot FROM turns WHERE kept = ?1")?;
            let slots = statement.query_map([&of], |row| row.get::<_, String>(0))?;
            let slots = slots.collect::<rusqlite::Result<Vec<String>>>()?;
            connection.execute(
                "UPDATE turns SET prompt = '', reply = '', error = NULL, decision = NULL, options = '[]'
                 WHERE kept = ?1 AND status NOT IN ('running', 'answered')",
                [&of],
            )?;
            Ok(slots)
        };
        let asked_there = self.of_each(self.every_turns(), asked).await?;
        let mut slots: BTreeSet<String> = asked_there.into_iter().flatten().collect();
        // And whoever was in it and was never asked anything there.
        self.with_a_place(channel, &mut slots);
        for slot in slots {
            self.forget_place(&slot, channel).await;
        }
        Ok(())
    }

    /// Takes `slot` out of every kept channel it is in, where it may be
    /// in none as things stand, and says so in each, once, with what
    /// would let it back: what it kept of each goes with it.
    pub(crate) async fn out_of_kept(&self, slot: &Slot) {
        let (channels, actor) = (&self.inner.studio.channels, slot.principal().actor());
        let within = channels.kept_with(&actor);
        if within.is_empty() {
            return;
        }
        let Some((why, rule, way)) = self.kept_out(slot) else { return };
        let own = studio::Channels::daemon(
            "an agent that may be in no channel kept to its members leaves each it is in, which is said in \
             that channel",
        );
        for channel in within {
            if channels.leave(&own, &channel, &actor).await.is_err() {
                continue;
            }
            let note = format!("{why}, and {rule}: it has left this one. {way}.");
            let _ = channels.post_speaker(&own, &channel, &note, studio::Place::default()).await;
            self.forget_place(&slot.name, &channel).await;
        }
    }

    /// Takes out of every kept channel each agent that may be in none
    /// as things stand: asked when something an agent's standing rests
    /// on has changed for more than one of them (a package taken or
    /// taken away, a sign-in added, the daemon started). Asked of
    /// nothing in a factory that keeps no channel.
    /// A channel given back from the Trash, which it went into at
    /// `went`: an agent that was taken away while it was there is in it
    /// no longer (one made since under its name is no heir to it), and
    /// every agent in a kept one is asked again, as things now stand,
    /// whether it may be.
    pub async fn channel_back(&self, channel: &str, went: i64) -> Result<()> {
        let channels = &self.inner.studio.channels;
        // An agent taken away while the channel was in the Trash leaves it.
        let own = studio::Channels::daemon("who is in a channel given back from the Trash");
        let found = channels.get(&own, channel).await?;
        for member in found.members.iter().filter(|member| member.actor.starts_with("agent:")) {
            let name = member.actor.trim_start_matches("agent:");
            if !self.slot(name).await.is_ok_and(|slot| slot.created <= went) {
                channels.leave(&own, &found.id, &member.actor).await?;
            }
        }
        self.settle_kept().await;
        Ok(())
    }

    pub(crate) async fn settle_kept(&self) {
        if !self.inner.studio.channels.keeps_any() {
            return;
        }
        for slot in self.slots().await.unwrap_or_default() {
            self.out_of_kept(&slot).await;
        }
    }

    /// A slot's turns, newest first, as `by` may be shown them: one of a
    /// kept channel `by` does not read has its words left out. And with
    /// each that a message asked for, what that message said
    /// (`TurnInfo::asked`), read by `by`'s own leave for the channel
    /// the turn answers in, in one reading a channel: a turn's row
    /// keeps the whole of what its agent was sent and names the message,
    /// and the words are the message's to say.
    pub async fn turns_for(&self, slot: &str, limit: usize, by: &Principal) -> Result<Vec<TurnInfo>> {
        let channels = &self.inner.studio.channels;
        // The leave `by` has for each channel a turn names, asked once
        // for each: none, where they do not read it.
        let mut leave: HashMap<String, Option<studio::Reader>> = HashMap::new();
        let mut shown = Vec::new();
        for turn in self.turns(slot, limit).await? {
            for channel in turn.kept.iter().chain(&turn.channel) {
                if !leave.contains_key(channel) {
                    leave.insert(channel.clone(), channels.may_read(channel, by).await.ok());
                }
            }
            let whole = turn.kept.as_ref().is_none_or(|kept| leave[kept].is_some());
            shown.push(if whole { turn } else { turn.without_its_words() });
        }
        // One left without its words names no message any more.
        let mut said: HashMap<String, String> = HashMap::new();
        for (channel, reader) in &leave {
            let Some(reader) = reader else { continue };
            let asked_by: Vec<String> = shown
                .iter()
                .filter(|turn| turn.channel.as_deref() == Some(channel.as_str()))
                .filter_map(|turn| turn.reply_to.clone())
                .collect();
            said.extend(channels.said_in(reader, &asked_by).await?);
        }
        for turn in &mut shown {
            let words = turn.reply_to.as_ref().and_then(|message| said.get(message));
            turn.asked = words.filter(|words| !words.trim().is_empty()).cloned();
        }
        Ok(shown)
    }

    /// The turns of each of `conversations`, and of every seat in one
    /// (`<conversation>:<seat>`), from every file that keeps turns, in
    /// the order they were asked, as `by` may be shown them: one of a
    /// kept channel `by` does not read has its words left out, as in
    /// [`Agents::turns_for`]; each is handed with the conversation it is
    /// part of, which is one that was asked for and says nothing that was
    /// said in it. Each is asked of the index of turns by conversation,
    /// so what is read is those turns and nothing more, however many
    /// there are of anything else.
    pub async fn turns_in(
        &self,
        conversations: &[String],
        by: &Principal,
    ) -> Result<Vec<(String, TurnInfo)>> {
        let mut found: Vec<TurnInfo> = Vec::new();
        for conversation in conversations {
            let (whole, seats, after) =
                (conversation.clone(), format!("{conversation}:"), format!("{conversation};"));
            let asked = move |connection: &mut rusqlite::Connection| {
                let mut statement = connection.prepare(&format!(
                    "SELECT {} FROM turns WHERE thread = ?1 OR (thread > ?2 AND thread < ?3)",
                    crate::turns::TURN
                ))?;
                let rows = statement.query_map([&whole, &seats, &after], crate::turns::read_turn)?;
                rows.collect::<rusqlite::Result<Vec<_>>>()
            };
            found.extend(self.of_each(self.every_turns(), asked).await?.into_iter().flatten());
        }
        found.sort_by_key(|turn| turn.nth);
        found.dedup_by(|one, other| one.id == other.id);
        let channels = &self.inner.studio.channels;
        let mut leave: HashMap<String, bool> = HashMap::new();
        let mut shown = Vec::with_capacity(found.len());
        for turn in found {
            let whole = match turn.kept.clone() {
                None => true,
                Some(kept) => match leave.get(&kept) {
                    Some(reads) => *reads,
                    None => {
                        let reads = channels.may_read(&kept, by).await.is_ok();
                        leave.insert(kept, reads);
                        reads
                    }
                },
            };
            let thread = turn.thread.clone();
            shown.push((thread, if whole { turn } else { turn.without_its_words() }));
        }
        Ok(shown)
    }
}

//! Turns: the queue, the worker that runs them, and the dispatcher that
//! turns channel messages into them.

use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use blueprint::Labels;
use journal::optional;
use kernel::{Cancel, Error, Result, Source, whole};
use serde::Serialize;
use serde_json::json;
use tokio::sync::{mpsc, oneshot};

use hosting::kept;

use crate::adapter::Turn;
use crate::running::Kept;
use crate::together::{Answer, Unbegun};
use crate::{Agents, Slot};

/// A piece of work for an agent.
#[derive(Clone, Debug, Default)]
pub struct TurnRequest {
    pub slot: String,
    /// `message`, `task` or `consult`.
    pub kind: String,
    /// Which conversation this continues. Turns with the same thread share
    /// the agent's memory of what came before.
    pub thread: String,
    pub prompt: String,
    /// Post the reply to this channel when the turn ends.
    pub channel: Option<String>,
    pub reply_to: Option<String>,
    /// For a consult: the answers to choose between.
    pub options: Vec<String>,
    /// What the turn is also measured with: the labels of the step it is
    /// taken for, so that the same work by two agents can be told apart.
    pub labels: Labels,
    /// The directory the agent's program is run in, relative to its
    /// project: a lane's worktree. None for the project itself.
    pub cwd: Option<String>,
    /// Folders of its project its program is not shown, from the
    /// project's top: the places of other pieces of work on the same item.
    pub hidden: Vec<String>,
    /// What becomes of it when a daemon is started and finds it in hand,
    /// cut off: begun again (as when nothing is said), or left as it was
    /// for a person.
    pub again: Option<bool>,
    /// After how many such starts at most it is begun again, whatever was
    /// said: [`MOST_RESTARTS`] when nothing is.
    pub most: Option<u32>,
    /// When the time it is given runs out, in milliseconds since 1970,
    /// where it is given a time.
    pub until: Option<i64>,
    /// What this asking is, in a word of the asker's own, and who asks
    /// (`crate::asked`): with both, asked the same
    /// again, the turn that was asked for before is given, ended or not.
    pub asking: Option<String>,
    pub asked_by: Option<String>,
}

/// How many times a turn is begun again after a daemon was started and
/// found it in hand, unless whoever asked for it says another number. A
/// turn that brings the daemon down must not go round for ever.
pub const MOST_RESTARTS: u32 = 3;

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct TurnInfo {
    pub id: String,
    pub slot: String,
    pub kind: String,
    pub thread: String,
    pub prompt: String,
    pub channel: Option<String>,
    pub reply_to: Option<String>,
    pub options: Vec<String>,
    /// `queued`, `running`, `done` or `failed`; and for a moment
    /// `answered`, while what it answered is said where it was asked,
    /// before it is said to have ended.
    pub status: String,
    pub reply: String,
    pub error: Option<String>,
    /// For a consult: which option was chosen.
    pub decision: Option<String>,
    /// What its program said it cost, in dollars: nought where it said
    /// nothing of it, which `cost_said` tells apart.
    pub cost_usd: f64,
    /// Whether its program said what it cost; a turn from before that
    /// says more than nought did.
    pub cost_said: bool,
    /// What its program said it was refused, where it said (`Reply::refused`).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub refused: Vec<String>,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub created: i64,
    pub started: Option<i64>,
    pub finished: Option<i64>,
    /// What it is measured with besides its agent and kind.
    pub labels: Labels,
    /// The directory of its project it is taken in, where that is not the
    /// project's own.
    pub cwd: Option<String>,
    /// Folders of its project its program was not shown.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub hidden: Vec<String>,
    /// Whether its program goes on without the daemon.
    pub outlives: bool,
    /// How many times a daemon was started and found it in hand.
    pub restarts: u32,
    /// Cut off so, whether it is begun again, and after how many such
    /// starts at most.
    pub again: bool,
    pub most: u32,
    /// When the time it was given runs out, where it was given one.
    pub until: Option<i64>,
    /// Whose it is, as its row says: the project its agent was of when
    /// it was queued, the empty text for nobody's, and nothing for one
    /// kept before a turn said. Which says where it is kept
    /// (`crate::filed`), and is not what is handed out of it.
    #[serde(skip)]
    pub(crate) project: Option<String>,
    /// Which turn of all it is (`crate::filed`).
    #[serde(skip)]
    pub(crate) nth: i64,
    /// The channel kept to its members that it is of (`crate::kept`).
    #[serde(skip)]
    pub(crate) kept: Option<String>,
    /// Which asking it was taken for, in its asker's word, and who asked
    /// (`crate::asked`): what places it among a card's rounds
    /// (`deciding::spent::begun_by`). Nothing for one asked in no word.
    /// Handed to nobody: who asked names whoever applied it.
    #[serde(skip)]
    pub asking: Option<String>,
    #[serde(skip)]
    pub asked_by: Option<String>,
    /// That its words were left out of this, for somebody who does not
    /// read the kept channel it is of.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub kept_from: bool,
    /// The words that asked for it, apart from everything its prompt
    /// puts round them: what was said in the message it answers. It is
    /// no part of the row: it is read from the message the row names
    /// where a turn is handed to somebody who reads that message's
    /// channel (`Agents::turns_for`), and is nothing anywhere else, for
    /// a turn no message asked for, and where the message is gone.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub asked: Option<String>,
}

impl TurnInfo {
    /// Which turn of all it is, in the order turns were written.
    pub fn nth(&self) -> i64 {
        self.nth
    }

    pub fn is_finished(&self) -> bool {
        self.status == "done" || self.status == "failed"
    }
}

pub(crate) const TURN: &str = "id, slot, kind, thread, prompt, channel, reply_to, options, status, reply, error, decision, \
                    cost_usd, input_tokens, output_tokens, created, started, finished, labels, cwd, outlives, \
                    restarts, again, most, until, project, coalesce(nth, rowid), kept, asking, asked_by, refused, hidden, \
                    coalesce(cost_said, cost_usd > 0)";

pub(crate) fn read_turn(row: &rusqlite::Row<'_>) -> rusqlite::Result<TurnInfo> {
    Ok(TurnInfo {
        id: row.get(0)?,
        slot: row.get(1)?,
        kind: row.get(2)?,
        thread: row.get(3)?,
        prompt: row.get(4)?,
        channel: row.get(5)?,
        reply_to: row.get(6)?,
        options: serde_json::from_str(&row.get::<_, String>(7)?).unwrap_or_default(),
        status: row.get(8)?,
        reply: row.get(9)?,
        error: row.get(10)?,
        decision: row.get(11)?,
        cost_usd: row.get(12)?,
        input_tokens: row.get::<_, i64>(13)? as u64,
        output_tokens: row.get::<_, i64>(14)? as u64,
        created: row.get(15)?,
        started: row.get(16)?,
        finished: row.get(17)?,
        labels: serde_json::from_str(&row.get::<_, String>(18)?).unwrap_or_default(),
        cwd: row.get(19)?,
        outlives: row.get::<_, i64>(20)? != 0,
        restarts: row.get::<_, i64>(21)? as u32,
        again: row.get::<_, i64>(22)? != 0,
        most: row.get::<_, i64>(23)? as u32,
        until: row.get(24)?,
        project: row.get(25)?,
        nth: row.get(26)?,
        kept: row.get(27)?,
        asking: row.get(28)?,
        asked_by: row.get(29)?,
        refused: row
            .get::<_, Option<String>>(30)?
            .and_then(|said| serde_json::from_str(&said).ok())
            .unwrap_or_default(),
        hidden: row
            .get::<_, Option<String>>(31)?
            .and_then(|said| serde_json::from_str(&said).ok())
            .unwrap_or_default(),
        cost_said: row.get::<_, i64>(32)? != 0,
        kept_from: false,
        asked: None,
    })
}

/// The option an agent chose, from the last `DECISION: <option>` line of its
/// reply.
fn decision(reply: &str, options: &[String]) -> Option<String> {
    reply.lines().rev().find_map(|line| {
        let line = line.trim().trim_matches(['*', '`', '_']);
        let (label, choice) = line.split_once(':')?;
        if !label.trim().eq_ignore_ascii_case("decision") {
            return None;
        }
        let choice = choice.trim().trim_matches(['*', '`', '.', '"', '\'']).to_lowercase();
        options.iter().find(|option| option.to_lowercase() == choice).cloned()
    })
}

/// What an agent said, without the line it chose on: for work, where the
/// choice is the blueprint's to read and the rest is what is shown.
fn without_decision(reply: &str, options: &[String]) -> String {
    let lines: Vec<&str> = reply.lines().collect();
    let chosen = lines.iter().rposition(|line| decision(line, options).is_some());
    match chosen {
        Some(at) => {
            let kept: Vec<&str> =
                lines.iter().enumerate().filter(|(index, _)| *index != at).map(|(_, line)| *line).collect();
            kept.join("\n").trim().to_string()
        }
        None => reply.to_string(),
    }
}

/// Whether a program that went on without the daemon has yet to end.
fn still_at_work(files: &kept::Files, hosting: &hosting::Hosting) -> bool {
    kept::Process::found(files, hosting.watch()).is_some_and(|process| !process.has_ended())
}

impl Agents {
    /// Where a slot's program is run: its project's directory, or `cwd`
    /// within it when a turn is to be taken somewhere of its own there.
    /// The sentence says why not, for whoever asked for the turn.
    ///
    /// One that works in no project works in the folder that is its
    /// own, and for a turn of the kept channel `kept` in the one that is
    /// its own for that channel (`crate::kept`).
    pub(crate) fn workdir(
        &self,
        slot: &Slot,
        cwd: Option<&str>,
        kept: Option<&str>,
    ) -> std::result::Result<PathBuf, String> {
        let base = match (&slot.project, kept) {
            (Some(project), _) => self
                .inner
                .studio
                .projects
                .dir_of(project)
                .ok_or_else(|| format!("its project `{project}` is gone"))?,
            (None, None) => self.inner.config.home.join(&slot.name),
            (None, Some(kept)) => {
                let place = self.place_for(&slot.name, kept);
                std::fs::create_dir_all(&place).map_err(|err| format!("{}: {err}", place.display()))?;
                place
            }
        };
        let Some(cwd) = cwd else { return Ok(base) };
        let Some(project) = &slot.project else {
            return Err(format!(
                "`cwd` is a directory of the agent's project, and `{}` works in none",
                slot.name
            ));
        };
        let within = Path::new(cwd).components().all(|part| matches!(part, Component::Normal(_)));
        if !within {
            return Err(format!(
                "`cwd` is a directory within the project `{project}`, written from its top, such as                  `.lanes/{project}-1`; `{cwd}` would leave it"
            ));
        }
        let dir = base.join(cwd);
        // What is there may lead out of the project all the same.
        let inside = whole::whole(&dir).ok().zip(whole::whole(&base).ok());
        match inside {
            Some((dir, base)) if dir.is_dir() && whole::within(&dir, &base) => Ok(dir),
            Some((dir, _)) if dir.is_dir() => {
                Err(format!("`cwd` is a directory within the project `{project}`; `{cwd}` leads out of it"))
            }
            _ => Err(format!("the project `{project}` has no directory `{cwd}` to work in")),
        }
    }

    /// Queues a turn and returns its id. The slot's worker is woken, or
    /// started if it has none.
    pub async fn enqueue(&self, request: TurnRequest) -> Result<String> {
        let _held = self.inner.choosing.lock().await;
        self.enqueue_chosen(request).await
    }

    /// The same, for whoever holds `choosing` already: having chosen who
    /// the work goes to, and not let go since.
    pub(crate) async fn enqueue_chosen(&self, request: TurnRequest) -> Result<String> {
        let slot = self.slot(&request.slot).await?;
        // No turn is ever queued for one this factory does not start:
        // a turn is a program started.
        if slot.external() {
            return Err(Error::conflict(crate::external::not_given(&slot.name)));
        }
        if !slot.active {
            return Err(Error::conflict(format!("the agent `{}` has been stopped", slot.name)));
        }
        if slot.leaving {
            return Err(Error::conflict(format!(
                "the agent `{}` is leaving: it finishes what it has in hand and takes no more",
                slot.name
            )));
        }
        // No turn is queued that would answer in an archived channel:
        // nothing is said there. Asked of memory, and of nothing where
        // no channel is archived.
        if let Some(channel) =
            request.channel.as_deref().filter(|named| self.inner.studio.channels.is_archived(named))
        {
            return Err(Error::conflict(format!(
                "#{} is archived, and nobody is asked anything in an archived channel: whoever may write in it \
                 gives it back first (`{} channel unarchive {}`)",
                channel.trim_start_matches('#'),
                brand::CLI,
                channel.trim_start_matches('#')
            )));
        }
        // A turn of a channel kept to its members is its members' agent's
        // to take, behind a wall: asked of nothing where none is kept.
        let kept = self.kept_of(request.channel.as_deref(), &request.thread).await;
        if let Err(refused) = self.may_ask(&slot, kept.as_deref()) {
            // One that may be in no kept channel as things stand has
            // left it before it is asked anything there.
            if kept.is_some() {
                self.out_of_kept(&slot).await;
            }
            return Err(refused);
        }
        let id = self.inner.journal.new_id("turn");
        let nth = self.queue(&id, request, &slot, kept).await?;
        self.was_given(&slot.name, nth);
        self.wake(&slot.name);
        Ok(id)
    }

    fn wake(&self, slot: &str) {
        let mut workers = self.inner.workers.lock().expect("agents lock");
        if let Some(worker) = workers.get(slot)
            && worker.send(()).is_ok()
        {
            return;
        }
        let (wake, woken) = mpsc::unbounded_channel();
        let _ = wake.send(());
        workers.insert(slot.to_string(), wake);
        drop(workers);
        let (agents, slot) = (self.clone(), slot.to_string());
        self.spawn(async move { agents.work(slot, woken).await });
    }

    pub async fn turn(&self, id: &str) -> Result<TurnInfo> {
        let lookup = id.to_string();
        let asked = move |connection: &mut rusqlite::Connection| {
            optional(connection.query_row(
                &format!("SELECT {TURN} FROM turns WHERE id = ?1"),
                [&lookup],
                read_turn,
            ))
        };
        let found = self.of_each(self.may_keep(id), asked).await?.into_iter().flatten().next();
        found.ok_or_else(|| Error::not_found(format!("there is no turn `{id}`")))
    }

    /// A slot's turns, newest first: from each file that holds any of
    /// them, put together by which turn of all each is.
    pub async fn turns(&self, slot: &str, limit: usize) -> Result<Vec<TurnInfo>> {
        let lookup = slot.to_string();
        let asked = move |connection: &mut rusqlite::Connection| {
            let mut statement = connection.prepare(&format!(
                "SELECT {TURN} FROM turns WHERE slot = ?1 ORDER BY rowid DESC LIMIT {limit}"
            ))?;
            let rows = statement.query_map([&lookup], read_turn)?;
            rows.collect::<rusqlite::Result<Vec<_>>>()
        };
        let mut turns: Vec<TurnInfo> =
            self.of_each(self.may_hold(slot), asked).await?.into_iter().flatten().collect();
        // What an agent of the name that was taken away was asked is
        // not this one's work (`crate::removed`).
        let since = self.since(slot).await;
        turns.retain(|turn| turn.nth > since);
        turns.sort_by_key(|turn| std::cmp::Reverse(turn.nth));
        turns.truncate(limit);
        Ok(turns)
    }

    /// Waits for a turn to end and returns it.
    pub async fn wait(&self, id: &str) -> Result<TurnInfo> {
        loop {
            let (done, finished) = oneshot::channel();
            self.inner.waiters.lock().expect("agents lock").entry(id.to_string()).or_default().push(done);
            self.inner.journal.measure(crate::filed::TURN_WAITED, &[], 1.0, true);
            let turn = self.turn(id).await?;
            if turn.is_finished() {
                return Ok(turn);
            }
            let _ = finished.await;
        }
    }

    /// Stops a turn that is running; one still queued is failed where it is.
    pub async fn cancel(&self, id: &str) {
        if let Some(cancel) = self.inner.running.lock().expect("agents lock").get(id) {
            cancel.cancel();
            return;
        }
        self.call_off_unbegun(Unbegun::Turn(id.to_string()), "cancelled before it started").await;
        self.notify(id);
    }

    /// Ends everything a slot has in hand, saying why: what it is doing
    /// is stopped where it is, and what waits behind that is not begun.
    pub(crate) async fn stop_turns_of(&self, slot: &str, why: &str) {
        // What waits is called off first, in one write, so that nothing
        // of it is begun as what is at work is stopped.
        for id in self.call_off_unbegun(Unbegun::Of(slot.to_string()), why).await {
            self.notify(&id);
        }
        let lookup = slot.to_string();
        let asked = move |connection: &mut rusqlite::Connection| {
            let mut statement =
                connection.prepare("SELECT id FROM turns WHERE slot = ?1 AND status = 'running'")?;
            let rows = statement.query_map([&lookup], |row| row.get(0))?;
            rows.collect::<rusqlite::Result<Vec<String>>>()
        };
        let at_work = self.of_each(self.may_hold(slot), asked).await.unwrap_or_default();
        let at_work: Vec<String> = at_work.into_iter().flatten().collect();
        for id in at_work {
            self.cancel(&id).await;
        }
    }

    pub(crate) fn notify(&self, id: &str) {
        if let Some(waiting) = self.inner.waiters.lock().expect("agents lock").remove(id) {
            for waiter in waiting {
                let _ = waiter.send(());
            }
        }
    }

    /// The next turn for a slot: one that was in hand when the conductor
    /// last stopped (running, or answered and not yet said to have
    /// ended), else the oldest queued, of every file that holds a turn
    /// of its.
    async fn next(&self, slot: &str) -> Option<TurnInfo> {
        let (lookup, draining) = (slot.to_string(), self.draining());
        // While the daemon drains, one somebody said to it in a channel
        // waits for the next daemon (`crate::drained`).
        let asked = move |connection: &mut rusqlite::Connection| {
            optional(connection.query_row(
                &format!(
                    "SELECT {TURN} FROM turns WHERE slot = ?1 AND status IN ('answered', 'running', 'queued')
                     AND NOT (?2 AND status = 'queued' AND kind = 'message')
                     ORDER BY status = 'queued', rowid LIMIT 1"
                ),
                rusqlite::params![lookup, draining],
                read_turn,
            ))
        };
        let held = self.may_hold(slot);
        let first = self.of_each(held.clone(), asked).await.ok()?;
        let (journal, next) = held
            .into_iter()
            .zip(first)
            .filter_map(|(journal, turn)| Some((journal, turn?)))
            .min_by_key(|(_, turn)| (turn.status == "queued", turn.nth))?;
        // Where it was found is where it is written as begun and ended.
        self.found_in(&next.id, &journal);
        Some(next)
    }

    /// One slot's worker: runs its turns, one at a time, in order.
    async fn work(self, slot: String, mut woken: mpsc::UnboundedReceiver<()>) {
        loop {
            match self.next(&slot).await {
                Some(turn) => self.run(turn).await,
                None => {
                    // With nothing more to do, one that is leaving is gone.
                    self.gone_if_done(&slot).await;
                    if woken.recv().await.is_none() {
                        return;
                    }
                }
            }
        }
    }

    async fn run(&self, mut turn: TurnInfo) {
        // Its answer was kept by an earlier daemon, which stopped before
        // the turn was said to have ended: its agent is not asked again.
        if turn.status == "answered" {
            return self.end_answered(turn).await;
        }
        let journal = &self.inner.journal;
        let now = journal.now_ms();
        // One that is in hand already was begun by an earlier daemon, which
        // stopped in the middle of it: counted, each time.
        let cut_off = turn.status == "running";
        if cut_off {
            turn.restarts += 1;
        }
        let files = self.inner.hosting.kept(&format!("turn.{}", turn.id));
        // Its program went on without that daemon, and is there to be
        // taken up: still at work, where its slot outlives the daemon, or
        // finished with its answer (`crate::running`).
        let outlives = self.inner.outliving.of(&turn.slot);
        let watch = self.inner.hosting.watch();
        let goes_on = cut_off && crate::running::there_to_take_up(&files, watch, outlives).await;
        // What becomes of it otherwise is what whoever asked for it said,
        // and after so many such stops it is left whatever they said: a
        // turn that brings the daemon down does not go round for ever.
        let mut left = match cut_off {
            true if turn.restarts > turn.most => Some(format!(
                "the daemon was stopped while {} had this in hand, {} times now, and it is begun again after {} \
                 such stops at most. It is left as it was for a person: ask again to have it done",
                turn.slot, turn.restarts, turn.most
            )),
            // Its program went on by itself past the time it was given.
            true if goes_on
                && turn.until.is_some_and(|until| now > until)
                && still_at_work(&files, &self.inner.hosting) =>
            {
                Some(format!(
                    "the daemon was stopped while {} had this in hand, and by the time it came back the time \
                     this was given had passed with its program still at work by itself. It was stopped: ask \
                     again to have it done",
                    turn.slot
                ))
            }
            true if !goes_on && !turn.again => Some(format!(
                "the daemon was stopped while {} had this in hand, and it was asked for to be left as it was then \
                 and not begun again. Ask again to have it done",
                turn.slot
            )),
            _ => None,
        };
        // One taken up where it is began when it began.
        let started = if goes_on { turn.started.unwrap_or(now) } else { now };
        // With the conversation it is part of: for a card, that says which
        // card the agent now has, to whatever draws the fleet at work.
        let mut about = json!({
            "slot": turn.slot, "turn": turn.id, "kind": turn.kind, "channel": turn.channel, "thread": turn.thread,
        });
        if cut_off {
            about["again"] = json!(turn.restarts);
            about["taken_up"] = json!(goes_on);
        }
        // Where it is called off from as soon as it is said to have begun:
        // somebody who heard that may call it off before its program is.
        let cancel = Cancel::new();
        if left.is_none() {
            self.inner.running.lock().expect("agents lock").insert(turn.id.clone(), cancel.clone());
        }
        // Let go of before it came to be begun here (`Agents::let_go_under`):
        // what was to ask for it again is over, and nobody waits for it.
        // Looked at once it is where it is called off from, so that a
        // letting go after this finds it there.
        let let_go = left.is_none() && self.was_let_go(&turn.id);
        if let_go {
            self.inner.running.lock().expect("agents lock").remove(&turn.id);
            left = Some(
                "what asked for this before the daemon was last stopped came to an end without asking for it \
                 again, and nobody waits for it. It was called off"
                    .to_string(),
            );
        }
        self.begin(&turn, started, about).await;

        // Whether somebody called it off: an agent that was stopped at
        // its work did not fail at it, and whoever reads how it ended is
        // told which.
        let mut called_off = let_go;
        let outcome = match &left {
            Some(why) => {
                // Left, its program is not left at work on it.
                if let Some(mut process) = kept::Process::found(&files, self.inner.hosting.watch()) {
                    process.stop().await;
                    process.forget();
                }
                Err(why.clone())
            }
            None => {
                // Every turn's program is started by its wrapper, where
                // that can be: whether it goes on without the daemon is
                // asked of its slot as the daemon stops (`crate::running`).
                let outliving = self.inner.outliving.clone();
                let slot = turn.slot.clone();
                let kept = kept::can().then(|| Kept {
                    files,
                    taken_up: goes_on,
                    unsay: Vec::new(),
                    outlives: Arc::new(move || outliving.of(&slot)),
                });
                // Taken up where it was, it is somebody's again once they
                // ask for it, and theirs to hold to a time. Until then it
                // is held to the time it was first given: a program that
                // nobody is waiting for is not left at work for ever.
                let nobodys = async {
                    let until = turn.until.filter(|until| goes_on && *until > now);
                    let Some(until) = until else { return std::future::pending().await };
                    let left = (until - journal.now_ms()).max(0) as u64;
                    journal.clock().sleep(std::time::Duration::from_millis(left)).await;
                    // Nobody may ask for it from here on, if nobody has.
                    if !self.claimed(&turn.id) {
                        std::future::pending::<()>().await;
                    }
                };
                let attempt = self.attempt(&turn, cancel.clone(), kept);
                tokio::pin!(attempt);
                let outcome = tokio::select! {
                    outcome = &mut attempt => {
                        called_off = cancel.is_cancelled();
                        outcome
                    }
                    () = nobodys => {
                        cancel.cancel();
                        let _ = attempt.await;
                        Err(format!(
                            "{} went on with this by itself after the daemon was stopped, nobody asked for it \
                             again, and the time it was given ran out. Its program was stopped",
                            turn.slot
                        ))
                    }
                };
                self.inner.running.lock().expect("agents lock").remove(&turn.id);
                outcome
            }
        };

        let finished = journal.now_ms();
        let (mut reply, error) = match outcome {
            Ok(reply) => (reply, None),
            Err(message) => (crate::Reply::default(), Some(message)),
        };
        // A consult is asked to choose and must; work may be given a
        // choice to end on (done, or a question instead), and what it
        // said is then kept without the line the choice was made on.
        let chosen = if turn.options.is_empty() { None } else { decision(&reply.text, &turn.options) };
        if (turn.kind == "task" || turn.kind == "message") && chosen.is_some() {
            reply.text = without_decision(&reply.text, &turn.options);
        }
        let answer = Answer { reply, error, chosen, finished };
        self.conclude(&turn, answer, started, called_off, false).await;
    }

    async fn attempt(
        &self,
        turn: &TurnInfo,
        cancel: Cancel,
        mut outliving: Option<Kept>,
    ) -> std::result::Result<crate::Reply, String> {
        let slot = self.slot(&turn.slot).await.map_err(|err| err.message)?;
        if !slot.active {
            return Err("the agent has been stopped".to_string());
        }
        let adapter = self.inner.adapters.lock().expect("agents lock").get(&slot.adapter).cloned();
        let adapter =
            adapter.ok_or_else(|| format!("this device has no adapter called `{}`", slot.adapter))?;
        // It acts on the factory as itself, with a credential of this
        // factory's: one is made now if it has none that works.
        self.credential(&slot).await.map_err(|err| err.message)?;
        let journal = self.inner.journal.clone();
        // A conversation is carried on where it was held: an agent's
        // program keeps what was said by the directory it was said in, so
        // the same thread in another directory begins again.
        let session_key = match &turn.cwd {
            Some(cwd) => format!("{}/{}@{cwd}", slot.name, turn.thread),
            None => format!("{}/{}", slot.name, turn.thread),
        };
        let workdir = self.workdir(&slot, turn.cwd.as_deref(), turn.kept.as_deref())?;
        // Work asked in a thread of the channel it answers in is shown the
        // thread as it is when it is begun: what is asked is the same asked
        // again, whatever has been said there since.
        let mut prompt = match (&turn.channel, turn.thread.strip_prefix("thread:")) {
            (Some(channel), Some(root)) if turn.kind == "task" => {
                self.thread_shown(channel, root, turn.prompt.clone()).await
            }
            _ => turn.prompt.clone(),
        };
        // A consult is asked to choose. Work that may end more than one
        // way is asked which way it ended, in other words: something that
        // can choose but cannot do work (the built-in assistant) tells the
        // two apart by them, and does not answer work with a choice.
        if turn.kind == "consult" && !turn.options.is_empty() {
            prompt.push_str(&format!(
                "\n\nWhen you have done what you can, end your reply with a line of exactly this form, choosing one:\n\
                 DECISION: <{}>",
                turn.options.join(" | ")
            ));
        } else if !turn.options.is_empty() {
            prompt.push_str(&format!(
                "\n\nEnd your reply with a line of exactly this form, saying which of these it came to:\n\
                 DECISION: <{}>",
                turn.options.join(" | ")
            ));
        }
        // A limit is a sign-in's and not the work's: the turn begins again
        // on another, from what it was asked. It does so no more often
        // than there are sign-ins it could be handed, so that a package
        // which takes every failure for a limit cannot keep it going.
        let most = self.could_draw(&slot);
        // What the channel it is asked in keeps to hand, as the agent may read it: titles and places, read once.
        let mut docs = self.docs_told(turn.channel.as_deref(), &slot.principal()).await;
        docs.push_str(&self.discussing_told(turn.channel.as_deref(), &slot.name));
        // And what is attached to the card it is about, by name and size, with what reads each.
        docs.push_str(&self.attached_told(&turn.thread, &slot.principal()).await);
        let mut again = 0;
        loop {
            // And under the sign-in it was held under: what a program keeps
            // of a conversation is the account's it was signed in with.
            // And in a home of the same kind: what a program remembers of
            // a conversation is in the home it was held in.
            let kept = journal.kv_get(crate::removed::SESSIONS, &session_key).await.ok().flatten();
            let (session, under, held_in) = remembered(kept.as_ref(), turn.kept.is_some());
            let drawn = match slot.signins.is_empty() {
                true => None,
                false => Some(self.draw(&slot, &turn.id, under.as_deref(), &cancel).await?),
            };
            let held = drawn.as_ref().map(|drawn| drawn.name.clone());
            let mut env = self.environment(&slot);
            let home = self.home_for(&slot.name, turn.kept.as_deref());
            let home_of_its_own = home.clone();
            let handed = match drawn.as_ref().map(|drawn| self.hand(drawn, &slot, home)) {
                Some(Ok(handed)) => Some(Arc::new(handed)),
                Some(Err(why)) => {
                    if let Some(drawn) = drawn {
                        self.give_back(drawn).await;
                    }
                    return Err(why);
                }
                None => None,
            };
            let holds = self.kept_is_held(&slot).await;
            let behind = self.behind(
                &slot,
                &workdir,
                handed.as_deref(),
                &env,
                (holds, turn.kept.as_deref(), &turn.hidden),
            );
            let (walled, git) = (behind.is_some(), behind.as_ref().map(|behind| behind.git));
            let without = handed.as_ref().map(|handed| handed.loses.clone()).unwrap_or_default();
            env.extend(handed.iter().flat_map(|handed| handed.gains.clone()));
            // And the keys its slot says it is handed, read now.
            let keys = match self.keys_for(&slot) {
                Ok(keys) => Arc::new(keys),
                Err(why) => {
                    if let Some(drawn) = drawn {
                        self.give_back(drawn).await;
                    }
                    return Err(why);
                }
            };
            env.extend(keys.env());
            let holds_keys = keys.variables();
            // What it is and how it stands, as its standing instructions
            // where its home has the file its program reads them from.
            let brief = self.preamble(&slot, turn.cwd.as_deref(), git, &holds_keys, &docs);
            let instructions = behind.as_ref().and_then(|behind| behind.instructions.clone());
            let briefing = crate::told::Briefing {
                home: &home_of_its_own,
                named: self.extension(&slot.adapter).and_then(|package| package.wall.instructions.clone()),
                at: instructions.as_deref(),
            };
            let system = self.briefed(&slot, brief, briefing, |said| keys.unsaid(said));
            // Where its program keeps what it remembers of a conversation.
            let own_home = match &behind {
                Some(behind) => behind.own_home,
                None => handed.as_ref().is_some_and(|handed| handed.moves_its_folder()),
            };
            // One that goes on without the daemon writes to files, and
            // what it was handed is taken out on the way there. Handed
            // something that is in no variable, it cannot be: it is the
            // daemon's to hold for this turn, as one that does not outlive it.
            let mut unsay = keys.unsay();
            match handed.as_deref().map(crate::signins::Handed::unsay) {
                Some(Some(more)) => unsay.extend(more),
                Some(None) => outliving = outliving.filter(|kept| kept.taken_up),
                None => {}
            }
            // What its program says is written down without the value it
            // was handed, should it repeat it.
            let unsaid = move |said: &str| match &handed {
                Some(handed) => keys.unsaid(&handed.unsaid(said)),
                None => keys.unsaid(said),
            };
            let shown = unsaid.clone();
            let (progress_journal, progress_slot, progress_turn) =
                (journal.clone(), slot.name.clone(), turn.id.clone());
            // Whose the agent is goes with what it prints, as with
            // everything said of it: nothing here names an instance.
            let (progress_project, progress_takes) = (slot.project.clone(), turn.takes());
            let request = Turn {
                slot: slot.name.clone(),
                principal: slot.principal(),
                system,
                prompt: prompt.clone(),
                session: session.filter(|_| under == held && held_in == Some(own_home)),
                model: slot.model.clone(),
                workdir: workdir.clone(),
                env,
                without,
                hosting: Some(self.inner.hosting.clone()),
                behind,
                permission_mode: slot.permission_mode.clone(),
                cli: Some(self.client()),
                web: slot.web,
                cancel: cancel.clone(),
                // Only the first try takes up what an earlier daemon
                // started: one begun again on another sign-in is begun.
                kept: outliving.take().map(|kept| Kept { unsay: unsay.clone(), ..kept }).inspect(|was| {
                    outliving = Some(Kept { taken_up: false, ..was.clone() });
                }),
                progress: Arc::new(move |text: &str| {
                    let mut data =
                        json!({ "slot": progress_slot, "turn": progress_turn, "text": shown(text) });
                    if let Some(project) = &progress_project {
                        data["project"] = json!(project);
                    }
                    // Of a turn of a kept channel, to that channel's members.
                    let source =
                        Source::actor(format!("agent:{progress_slot}")).taking(progress_takes.as_deref());
                    progress_journal.emit_ephemeral("agent.output", source, data);
                }),
            };
            // On pipes, it ends with the daemon: a drain says so of it.
            if request.kept.is_none() {
                self.inner.piped.lock().expect("agents lock").insert(turn.id.clone());
            }
            let outcome = adapter.run(request).await;
            self.inner.piped.lock().expect("agents lock").remove(&turn.id);
            let failure = match outcome {
                Ok(mut reply) => {
                    reply.text = unsaid(&reply.text);
                    if let Some(drawn) = drawn {
                        self.give_back(drawn).await;
                    }
                    if let Some(session) = &reply.session {
                        let home = if own_home { OWN } else { THEIRS };
                        let kept = match &held {
                            Some(signin) => json!({ "session": session, "signin": signin, "home": home }),
                            None => json!({ "session": session, "home": home }),
                        };
                        let _ = journal.kv_set(crate::removed::SESSIONS, &session_key, &kept).await;
                    }
                    if turn.kind == "consult"
                        && !turn.options.is_empty()
                        && decision(&reply.text, &turn.options).is_none()
                    {
                        return Err(format!("it did not choose one of: {}", turn.options.join(", ")));
                    }
                    return Ok(reply);
                }
                Err(failure) => crate::Failure {
                    message: self.hinted(&slot.adapter, unsaid(&failure.message), walled, failure.code),
                    ..failure
                },
            };
            let Some(drawn) = drawn else { return Err(failure.message) };
            let limit = self.limited(&slot, &failure).filter(|_| !cancel.is_cancelled());
            if let Some(limit) = limit {
                self.set_aside(&drawn, limit, &failure.message).await;
            }
            self.give_back(drawn).await;
            if limit.is_none() {
                return Err(failure.message);
            }
            if again >= most {
                return Err(format!(
                    "{}. It was begun again {again} times, once for each sign-in `{}` could be handed, and \
                     met a limit each time",
                    failure.message, slot.name
                ));
            }
            again += 1;
        }
    }

    /// Starts listening for channel messages, and picks up any turns left
    /// unfinished by the last conductor.
    pub fn start(&self) {
        let agents = self.clone();
        self.spawn(async move {
            let journal = agents.inner.journal.clone();
            // How this daemon was started may be what an agent's place
            // in a kept channel rested on: its walls, its environment.
            agents.settle_kept().await;
            let asked = |connection: &mut rusqlite::Connection| {
                let mut statement = connection.prepare(
                    "SELECT DISTINCT slot FROM turns WHERE status IN ('queued', 'running', 'answered')",
                )?;
                let rows = statement.query_map([], |row| row.get(0))?;
                rows.collect::<rusqlite::Result<Vec<String>>>()
            };
            let slots = agents.of_each(agents.every_turns(), asked).await.unwrap_or_default();
            let slots: std::collections::BTreeSet<String> = slots.into_iter().flatten().collect();
            for slot in slots {
                agents.wake(&slot);
            }
            // One that was leaving and has nothing left in hand is gone
            // now: a slot with no work has no worker to see it out.
            let leaving: Vec<String> = journal
                .db()
                .call(|connection| {
                    let mut statement =
                        connection.prepare("SELECT name FROM slots WHERE active = 1 AND leaving = 1")?;
                    let rows = statement.query_map([], |row| row.get(0))?;
                    rows.collect()
                })
                .await
                .unwrap_or_default();
            for slot in leaving {
                agents.gone_if_done(&slot).await;
            }
            // Where to listen from is written down before listening starts,
            // so that messages posted while the conductor is down are still
            // answered however early the last one stopped.
            // A place in each journal: what is said in a project's
            // channel is that project's, and kept where its events are.
            // A number alone, which a program from before kept, is a
            // place in the factory's own.
            let cursor = match journal.kv_get("agents", "cursor").await {
                Ok(Some(cursor)) => journal::Places::from_kept(&cursor).unwrap_or_default(),
                _ => {
                    let now = journal.now();
                    let _ = journal.kv_set("agents", "cursor", &now.kept()).await;
                    now
                }
            };
            // And what is said in a topic, which asks an agent it names (`crate::topical`).
            let mut follow = journal.follow_every(cursor, "channel.message,record.commented");
            while let Ok(event) = follow.next().await {
                if !event.is_durable() {
                    continue;
                }
                match event.topic.as_str() {
                    "channel.message" => agents.dispatch(&event).await,
                    _ => agents.topical(&event).await,
                }
                let _ = journal.kv_set("agents", "cursor", &follow.places().kept()).await;
            }
        });
    }
}

/// The home a conversation was held in, as what its program remembered of
/// it says: one of the agent's own, or the one that is shown it from the
/// home of whoever runs the daemon.
const OWN: &str = "own";
const THEIRS: &str = "theirs";

/// What its program remembered of a conversation, as it was kept: its
/// handle on it, the sign-in it was held under, and whether it was held in
/// a home of the agent's own. What a program from before kept says
/// nothing of the home: one held under a sign-in the factory handed, or
/// of a kept channel (`of_kept`), was in a home of the agent's own, and
/// any other in the one shown it from the home of whoever runs the
/// daemon, which no turn has once it has a home of its own. So such a
/// conversation is begun again there once, and is the same done twice:
/// nothing is forgotten until a new one is kept in its place.
fn remembered(
    kept: Option<&serde_json::Value>,
    of_kept: bool,
) -> (Option<String>, Option<String>, Option<bool>) {
    match kept {
        None => (None, None, None),
        Some(serde_json::Value::String(session)) => (Some(session.clone()), None, Some(of_kept)),
        Some(kept) => {
            let signin = kept["signin"].as_str().map(str::to_string);
            let own = match kept["home"].as_str() {
                Some(home) => home == OWN,
                None => signin.is_some() || of_kept,
            };
            (kept["session"].as_str().map(str::to_string), signin, Some(own))
        }
    }
}

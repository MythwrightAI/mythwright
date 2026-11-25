//! What the runner writes down and what it says of it, as one write.
//!
//! A run is written as ended and said to have finished; an instance is
//! written as active, failed, done or stopped and said to be; a step's
//! record is written and the step said to have started or finished. Each
//! pair is one transaction of the journal's ([`journal::Journal::write`]):
//! a conductor stopped at any instant leaves both or neither, and nothing
//! has to be looked for when one starts. Whatever is new that writes
//! something down and says so goes the same way: the event is made first
//! ([`to_say`]) and handed to what writes.
//!
//! Two things that only make sense together are one write as well: how a
//! run ended and what became of its instance ([`Ending`]).
//!
//! One thing cannot be one write, because something that takes time comes
//! between its two halves: an instance that failed says so, what it
//! started is stopped and what it held given back, and then it says that
//! it is over. So that it has failed is written with a note that the rest
//! is owed (`owes`, a column of its row), which is taken off in the write
//! that says it is over. A conductor that starts asks for the rows that
//! owe ([`Store::owing`], an index of those rows alone, empty but in that
//! moment), finishes each, and says the rest: exactly, and once.
//!
//! A journal that a program from before this wrote may hold an instance
//! written as active and never said to be. The migration that adds the
//! column notes each instance that is active then as owed one look
//! (`Engine::say_active_unsaid`); the first conductor to start on that
//! journal looks, says what was not said in the write that takes the
//! note off, and nothing is looked for at any start after it.

use journal::Writing;
use kernel::{Result, Source};
use rusqlite::params;
use serde_json::Value;

use crate::store::{
    Definition, INSTANCE_COLUMNS, InstanceInfo, InstanceStatus, RunStatus, StepInfo, Store, collect,
};

/// Something to be said with what is written.
pub struct Say {
    topic: &'static str,
    source: Source,
    data: Value,
    /// Kept in the journal, or only passed on ([`Say::passing`]).
    kept: bool,
    /// Where it is said is where a step began, and is written on the
    /// step's row ([`Say::begins_step`]).
    begins: bool,
}

impl Say {
    /// Passes it on to whoever is watching, and keeps nothing: for what
    /// is said of something that is not written down either.
    pub fn pass_on(self, journal: &journal::Journal) {
        journal.emit_ephemeral(self.topic, self.source, self.data);
    }

    /// Says it by itself, kept as any event is where it is kept, and
    /// passed on in its order where it is only passed on: where nothing
    /// is written down beside it that it could be said with.
    pub async fn tell(self, journal: &journal::Journal) {
        // The journal failing is not something a run can recover from, and it
        // must not turn a successful step into a failed one.
        let _ = match self.kept {
            true => journal.emit(self.topic, self.source, self.data).await,
            false => journal.pass_on(self.topic, self.source, self.data).await,
        };
    }

    /// The same, passed on where it is said and not kept, where `kept`
    /// is false: numbered, in its order, and nothing that reads the
    /// journal finds it ([`journal::Writing::pass_on`]).
    pub fn kept(self, kept: bool) -> Say {
        Say { kept, ..self }
    }

    /// The same, where it says that the step it is of began: its number,
    /// and when, are written on the step's row in the write it is said
    /// in ([`Store::step_began`]), so that where a step began is known
    /// whether or not the journal keeps what said so.
    pub fn begins_step(self) -> Say {
        Say { begins: true, ..self }
    }

    /// What it is said on.
    pub fn topic(&self) -> &'static str {
        self.topic
    }

    /// Whether it is kept in the journal, or only passed on.
    pub fn is_kept(&self) -> bool {
        self.kept
    }
}

/// An event made ready, for whatever writes down what it says.
pub fn to_say(topic: &'static str, source: Source, data: Value) -> Say {
    Say { topic, source, data, kept: true, begins: false }
}

/// Says each of `says` in the write under way, in order.
pub(crate) fn said(writing: &mut Writing<'_>, says: Vec<Say>) -> rusqlite::Result<()> {
    for one in says {
        let (run, step) = (one.source.run.clone(), one.source.step.clone());
        let seq = match one.kept {
            true => writing.say(one.topic, one.source, one.data)?,
            false => writing.pass_on(one.topic, one.source, one.data)?,
        };
        if let (true, Some(run), Some(step), true) = (one.begins, run, step, seq != 0) {
            writing.transaction().execute(
                "UPDATE steps SET began = ?3, began_at = ?4 WHERE run = ?1 AND step = ?2",
                params![run, step, seq as i64, writing.ts()],
            )?;
        }
    }
    Ok(())
}

/// Which record says that the step which set something has acted: what
/// an instance keeps is written in one transaction with it
/// ([`Store::set_instance_state_with`]).
#[derive(Clone, Debug)]
pub enum Done {
    /// The step's own, with what it gave.
    Step { run: String, step: String, gave: Value },
    /// That of one element of the list the step is done for.
    Element { run: String, step: String, position: usize, gave: Value },
    /// How many entries are done of what is done as a step starts
    /// (`start`) or at its ending; with no step, as the run ends.
    Entry { run: String, step: Option<String>, start: bool, done: usize },
    /// How many entries are done of what is done as an instance stops,
    /// which is done in no run: kept in the instance's own row.
    Stopping { instance: String, done: usize },
}

/// How a run ended, what is said of that, and what becomes of its
/// instance with it.
pub struct Ending {
    pub run: String,
    pub status: RunStatus,
    pub outputs: Value,
    pub error: Option<String>,
    pub now: i64,
    /// What its steps did, where that was not written as they went.
    pub steps: Vec<StepInfo>,
    /// That it failed, where it did, and that it finished.
    pub says: Vec<Say>,
    pub becomes: Option<Becomes>,
}

/// What an instance becomes as the run of its steps ends.
pub struct Becomes {
    pub instance: String,
    /// What it is written as; nothing, where what it is was written
    /// already (an update that failed, gone back from).
    pub status: Option<InstanceStatus>,
    pub note: Option<String>,
    /// The run was an update's, which ends with it: gone back from, to
    /// the definition here and saying why, or taken for good.
    pub update: Option<(Option<Definition>, Option<String>)>,
    /// It has failed, and what it started is still to be stopped.
    pub owes: bool,
    /// What is said of it, if it is still the run's to settle.
    pub says: Vec<Say>,
    /// Whether the run still speaks for its instance: asked in the write
    /// itself, where nothing can come between the answer and what is
    /// written. Whoever worked out what the instance becomes looked
    /// before they began and then waited (for what a failed update had
    /// started to be handed back, for what it has under it to be
    /// counted); an apply or a stop that came meanwhile has the instance
    /// in hand, and this run says nothing more of it.
    pub still: Box<dyn Fn() -> bool + Send>,
    /// Done in the write, where the instance was settled by it: what is
    /// in hand of the instance is brought to what was written, before
    /// anybody else can read either.
    pub then: Option<Box<dyn FnOnce() + Send>>,
}

impl Store {
    /// Writes down how a run ended and, where it is the run of an
    /// instance's steps, what the instance becomes, and says all of it:
    /// one write. True where the instance was settled by it: stopped as
    /// its steps ended, it is stopped, and nothing more is said of it
    /// here.
    pub async fn end_run(&self, ending: Ending) -> Result<bool> {
        let wrote = self.journal.write(move |writing| {
            let transaction = writing.transaction();
            // What its loops had got through is only for going on with.
            transaction.execute("DELETE FROM step_elements WHERE run = ?1", params![ending.run])?;
            for step in &ending.steps {
                crate::runs::put(transaction, step)?;
            }
            transaction.execute(
                "UPDATE runs SET status = ?2, outputs = ?3, error = ?4, finished = ?5, carries = NULL
                 WHERE id = ?1",
                params![
                    ending.run,
                    ending.status.as_str(),
                    ending.outputs.to_string(),
                    ending.error,
                    ending.now
                ],
            )?;
            said(writing, ending.says)?;
            let Some(becomes) = ending.becomes else { return Ok(false) };
            // Applied over or stopped since what it becomes was worked
            // out: the run has ended, and the instance is not its to say.
            if !(becomes.still)() {
                return Ok(false);
            }
            // An update gone back from is one only where the instance
            // went back: stopped meanwhile, it stays stopped.
            let mut settled = true;
            if let Some((revert, why)) = becomes.update {
                let back = revert.is_some();
                let written =
                    crate::store::end_update(transaction, &becomes.instance, revert, why, ending.now)?;
                settled = !back || written == 1;
            }
            if let (true, Some(status)) = (settled, becomes.status) {
                // One that is active and owes a look at what is under it
                // (`below`) owes it still: it is looked at from here.
                let written = transaction.execute(
                    "UPDATE instances SET status = ?2, error = ?3, updated = ?4,
                            owes = CASE WHEN ?5 IS NOT NULL THEN ?5
                                        WHEN ?2 = 'active' AND owes = 'below' THEN owes END
                     WHERE id = ?1 AND status IN ('starting', 'active')",
                    params![
                        becomes.instance,
                        status.as_str(),
                        becomes.note,
                        ending.now,
                        becomes.owes.then_some("down")
                    ],
                )?;
                settled = written == 1;
            }
            if settled {
                said(writing, becomes.says)?;
                if let Some(then) = becomes.then {
                    then();
                }
            }
            Ok(settled)
        });
        Ok(wrote.await?.0)
    }

    /// Writes an instance as failed that was running, with why, and says
    /// so: true where it was written. With `owes`, what it started is
    /// still to be stopped and it has yet to say that it is over
    /// ([`Store::taken_down`]).
    pub async fn fail_instance(
        &self,
        id: &str,
        error: String,
        owes: bool,
        now: i64,
        says: Vec<Say>,
    ) -> Result<bool> {
        let id = id.to_string();
        let wrote = self.journal.write(move |writing| {
            let written = writing.transaction().execute(
                "UPDATE instances SET status = 'failed', error = ?2, updated = ?3, owes = ?4
                 WHERE id = ?1 AND status IN ('starting', 'active')",
                params![id, error, now, owes.then_some("down")],
            )?;
            if written == 1 {
                said(writing, says)?;
            }
            Ok(written == 1)
        });
        Ok(wrote.await?.0)
    }

    /// Writes an instance that was taking a text as gone back to the one
    /// it had, with why, and says so: true where it was written. Stopped
    /// meanwhile, it is stopped, and nothing is said of it here.
    pub async fn go_back(
        &self,
        id: &str,
        to: crate::store::Definition,
        why: String,
        now: i64,
        says: Vec<Say>,
    ) -> Result<bool> {
        let id = id.to_string();
        let wrote = self.journal.write(move |writing| {
            let written = crate::store::end_update(writing.transaction(), &id, Some(to), Some(why), now)?;
            if written == 1 {
                said(writing, says)?;
            }
            Ok(written == 1)
        });
        Ok(wrote.await?.0)
    }

    /// Says the last of an instance that failed, once what it started
    /// has been stopped, and takes off the note that this was owed: true
    /// where it was owed, and so said. The same for the other thing a
    /// row can owe, a look for having been said to be active.
    pub async fn taken_down(&self, id: &str, says: Vec<Say>) -> Result<bool> {
        let id = id.to_string();
        let wrote = self.journal.write(move |writing| {
            let owed = writing.transaction().execute(
                "UPDATE instances SET owes = NULL WHERE id = ?1 AND owes IS NOT NULL",
                params![id],
            )?;
            if owed == 1 {
                said(writing, says)?;
            }
            Ok(owed == 1)
        });
        Ok(wrote.await?.0)
    }

    /// The instances that owe `what`. `down`: those that failed and that
    /// a conductor was stopped in the middle of taking down. `stop`: those
    /// whose stop had begun, with something done as they stop. `below`:
    /// those somebody applied over, whose steps had not settled, or had
    /// and what is under them had not yet been looked at. `said`:
    /// those a program from before wrote as active, which may never have
    /// been said to be (the migration that made the column says which).
    pub async fn owing(&self, what: &'static str) -> Result<Vec<InstanceInfo>> {
        self.db
            .call(move |connection| {
                collect(
                    connection,
                    &format!(
                        "SELECT {INSTANCE_COLUMNS} FROM instances INDEXED BY instances_owing
                         WHERE owes IS NOT NULL AND owes = ?1 ORDER BY rowid"
                    ),
                    [what],
                    crate::store::read_instance,
                )
            })
            .await
    }

    /// Takes off the note that what is under an instance is still to be
    /// looked at: it has been.
    pub async fn looked_below(&self, id: &str) -> Result<()> {
        let id = id.to_string();
        self.db
            .call(move |connection| {
                connection.execute("UPDATE instances SET owes = NULL WHERE id = ?1 AND owes = 'below'", [id])
            })
            .await
            .map(|_| ())
    }

    /// Whether a text is being carried over to an instance just now: the
    /// run that does what the text says in place of its steps (`flow`, as
    /// the runner calls such a run) is under way.
    pub async fn carrying(&self, id: &str, flow: &'static str) -> Result<bool> {
        let id = id.to_string();
        self.db
            .call(move |connection| {
                connection.query_row(
                    "SELECT EXISTS (SELECT 1 FROM runs WHERE instance = ?1 AND flow = ?2 AND status = 'running')",
                    params![id, flow],
                    |row| row.get(0),
                )
            })
            .await
    }

    /// Takes the note off every instance still owed a look for having
    /// been said to be active: each was looked at, or is active no longer.
    pub async fn looked(&self) -> Result<()> {
        self.db
            .call(|connection| connection.execute("UPDATE instances SET owes = NULL WHERE owes = 'said'", []))
            .await
            .map(|_| ())
    }

    /// How far what an instance does as it stops had come, and why it
    /// was being stopped; nothing, where no stop of it had begun.
    pub async fn kept_stop(&self, id: &str) -> Result<Value> {
        let id = id.to_string();
        let kept: Option<Option<String>> = self
            .db
            .call(move |connection| {
                journal::optional(connection.query_row(
                    "SELECT hooks FROM instances WHERE id = ?1 AND owes = 'stop'",
                    [id],
                    |row| row.get(0),
                ))
            })
            .await?;
        Ok(kept.flatten().and_then(|kept| serde_json::from_str(&kept).ok()).unwrap_or(Value::Null))
    }

    /// Keeps how far it has come, beside why, with the note that the
    /// stop is owed: from the first of it, which is kept before anything
    /// is done, until the instance is written as stopped. A conductor
    /// that starts in between finishes it.
    pub async fn save_stop(&self, id: &str, kept: Value) -> Result<()> {
        let (id, kept) = (id.to_string(), kept.to_string());
        self.db
            .call(move |connection| {
                connection.execute(
                    "UPDATE instances SET owes = 'stop', hooks = json_patch(coalesce(hooks, '{}'), ?2)
                     WHERE id = ?1 AND status IN ('starting', 'active')",
                    params![id, kept],
                )
            })
            .await
            .map(|_| ())
    }

    /// Keeps how far what is done around a step has come (as a run
    /// ends, with no step), and says with it that an entry could not be
    /// done: one write. Said apart, a conductor stopped between the two
    /// did the entry again, and it was said a second time that it could
    /// not be done.
    pub async fn save_hooks_saying(
        &self,
        run: &str,
        step: Option<&str>,
        kept: Value,
        says: Vec<Say>,
    ) -> Result<()> {
        let (run, step, kept) = (run.to_string(), step.map(str::to_string), kept.to_string());
        let wrote = self.journal.write(move |writing| {
            match step {
                Some(step) => writing.transaction().execute(
                    "UPDATE steps SET hooks = ?3 WHERE run = ?1 AND step = ?2",
                    params![run, step, kept],
                )?,
                None => writing
                    .transaction()
                    .execute("UPDATE runs SET hooks = ?2 WHERE id = ?1", params![run, kept])?,
            };
            said(writing, says)
        });
        wrote.await.map(|_| ())
    }

    /// The same of what an instance does as it stops ([`Store::save_stop`]).
    pub async fn save_stop_saying(&self, id: &str, kept: Value, says: Vec<Say>) -> Result<()> {
        let (id, kept) = (id.to_string(), kept.to_string());
        let wrote = self.journal.write(move |writing| {
            writing.transaction().execute(
                "UPDATE instances SET owes = 'stop', hooks = json_patch(coalesce(hooks, '{}'), ?2)
                 WHERE id = ?1 AND status IN ('starting', 'active')",
                params![id, kept],
            )?;
            said(writing, says)
        });
        wrote.await.map(|_| ())
    }

    /// Writes an instance as done and says so, where it was active only
    /// for its children and the last of them has ended: decided in the
    /// write, so that it is so only where it is still active, owes
    /// nothing, has no child alive and no run under way, and `still`
    /// holds (nobody has applied or stopped it since it was looked at).
    /// Whether it was written.
    pub async fn rest_instance(
        &self,
        id: &str,
        now: i64,
        still: Box<dyn Fn() -> bool + Send>,
        says: Vec<Say>,
    ) -> Result<bool> {
        let id = id.to_string();
        let wrote = self.journal.write(move |writing| {
            if !still() {
                return Ok(false);
            }
            let written = writing.transaction().execute(
                "UPDATE instances SET status = 'done', updated = ?2
                 WHERE id = ?1 AND status = 'active' AND owes IS NULL
                   AND NOT EXISTS (SELECT 1 FROM instances AS child
                                   WHERE child.parent = ?1 AND child.status IN ('starting', 'active'))
                   AND NOT EXISTS (SELECT 1 FROM runs WHERE runs.instance = ?1 AND runs.status = 'running')",
                params![id, now],
            )?;
            if written == 1 {
                said(writing, says)?;
            }
            Ok(written == 1)
        });
        wrote.await.map(|(rested, _)| rested)
    }

    /// Writes an instance as stopped and says so, and that it is over.
    pub async fn stop_instance(&self, id: &str, now: i64, says: Vec<Say>) -> Result<()> {
        let id = id.to_string();
        let wrote = self.journal.write(move |writing| {
            // Only one that was still running: one that ended while it
            // was being stopped (its steps failed, or came to their end)
            // has said how it ended, and is not said to have been stopped.
            let written = writing.transaction().execute(
                "UPDATE instances SET status = 'stopped', error = NULL, updated = ?2, owes = NULL, hooks = NULL
                 WHERE id = ?1 AND status IN ('starting', 'active')",
                params![id, now],
            )?;
            if written == 1 {
                said(writing, says)?;
            }
            Ok(())
        });
        wrote.await.map(|_| ())
    }

    /// Writes an instance as stopped with `note` as why, each of `runs`
    /// that was still under way as cancelled for the same reason, and
    /// says all of it: one write. A run says that it finished only where
    /// it was this that ended it, and the instance that it was stopped
    /// only where it was still running.
    pub async fn stop_with_its_runs(
        &self,
        id: &str,
        note: String,
        now: i64,
        runs: Vec<(String, Say)>,
        says: Vec<Say>,
    ) -> Result<()> {
        let id = id.to_string();
        let wrote = self.journal.write(move |writing| {
            for (run, finished) in runs {
                let transaction = writing.transaction();
                let ended = transaction.execute(
                    "UPDATE runs SET status = 'cancelled', error = ?2, finished = ?3
                     WHERE id = ?1 AND status = 'running'",
                    params![run, note, now],
                )?;
                if ended == 1 {
                    transaction.execute("DELETE FROM step_elements WHERE run = ?1", params![run])?;
                    said(writing, vec![finished])?;
                }
            }
            let written = writing.transaction().execute(
                "UPDATE instances SET status = 'stopped', error = ?2, updated = ?3, owes = NULL, hooks = NULL
                 WHERE id = ?1 AND status IN ('starting', 'active')",
                params![id, note, now],
            )?;
            if written == 1 {
                said(writing, says)?;
            }
            Ok(())
        });
        wrote.await.map(|_| ())
    }

    /// Writes a step's record, and before it whatever `before` hands over
    /// (the steps of its run that were passed over and are written with
    /// the run's next write, with what is said of them), and says what
    /// the record says of the step:
    /// that it started, or how it ended.
    pub async fn put_step_saying(
        &self,
        step: StepInfo,
        before: impl FnOnce() -> (Vec<StepInfo>, Vec<Say>) + Send + 'static,
        says: Vec<Say>,
    ) -> Result<()> {
        let wrote = self.journal.write(move |writing| {
            let (passed, of_them) = before();
            for one in &passed {
                crate::runs::put(writing.transaction(), one)?;
            }
            said(writing, of_them)?;
            crate::runs::put(writing.transaction(), &step)?;
            said(writing, says)
        });
        wrote.await.map(|_| ())
    }

    /// Writes down that a run began, with what says so (`run.started`
    /// passed on and not kept): where a run is taken up after a restart,
    /// one that began and wrote nothing else is told from one that never
    /// began by this ([`Store::run_begun`]).
    pub async fn begin_run(&self, run: &str, says: Vec<Say>) -> Result<()> {
        let run = run.to_string();
        let wrote = self.journal.write(move |writing| {
            writing.transaction().execute("UPDATE runs SET begun = 1 WHERE id = ?1", [&run])?;
            said(writing, says)
        });
        wrote.await.map(|_| ())
    }

    /// Writes whatever `held` hands over when the writing is done: the
    /// steps of a run that were passed over and are not yet written, and
    /// what is said of them. All of it, or none; nothing where there is
    /// none.
    pub async fn put_held_saying(
        &self,
        held: impl FnOnce() -> (Vec<StepInfo>, Vec<Say>) + Send + 'static,
    ) -> Result<()> {
        let wrote = self.journal.write(move |writing| {
            let (passed, of_them) = held();
            for one in &passed {
                crate::runs::put(writing.transaction(), one)?;
            }
            said(writing, of_them)
        });
        wrote.await.map(|_| ())
    }
}

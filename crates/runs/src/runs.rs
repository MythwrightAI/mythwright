//! The runs of instances and the steps of runs, as they are kept.
//!
//! A run is written down before it starts and a step before it runs, and
//! each again as it ends, so that what was under way when the conductor
//! stopped is there to be picked up. A handler's run is written together
//! with the handler's place in the journal, in one transaction: an event
//! starts its run once.

use journal::{Prepared, optional};
use kernel::Result;
use rusqlite::params;
use serde_json::Value;

use crate::store::{RUN_COLUMNS, RunInfo, STEP_COLUMNS, StepInfo, Store, collect, read_run, read_step};

/// Writes a step's row, in place of the one there was: but for what is
/// kept beside it of what is done around the step and of its having acted
/// (`hooks`), which no write of the row carries. That stays for as long as
/// the step is in the try it was kept in (taken up after a restart, saving
/// where it is, ending) and goes in the write that begins another try. A
/// row written whole, with that put back in a second write, had lost it
/// for a conductor stopped between the two.
///
/// Nor does a write carry which of its run's steps the step was to begin
/// (`nth`, which [`IN_ORDER`] reads by): that is said here, by the write
/// that first keeps the row, and is the row's for as long as it is
/// there. It is one above the highest any row of the run says, and not
/// how many rows the run has: the steps of a task that is done again
/// are taken away and written anew, and a count would give one of them
/// a number that a step beside the task has. The run's rows are looked
/// through for it only where the step has none yet, in this statement
/// and no other.
pub(crate) fn put(connection: &rusqlite::Connection, step: &StepInfo) -> rusqlite::Result<usize> {
    connection.execute_kept(
        &format!(
            "INSERT INTO steps ({STEP_COLUMNS}, nth)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10,
                 CASE WHEN NOT EXISTS (SELECT 1 FROM steps WHERE run = ?1 AND step = ?2)
                      THEN (SELECT coalesce(max(nth), 0) + 1 FROM steps WHERE run = ?1) END)
             ON CONFLICT (run, step) DO UPDATE SET
                 hooks = CASE WHEN excluded.status = 'running' AND excluded.attempt != steps.attempt
                              THEN NULL ELSE steps.hooks END,
                 status = excluded.status, attempt = excluded.attempt, tolerated = excluded.tolerated,
                 output = excluded.output, error = excluded.error, saved = excluded.saved,
                 started = excluded.started, finished = excluded.finished"
        ),
        params![
            step.run,
            step.step,
            step.status.as_str(),
            step.attempt,
            step.tolerated,
            step.output.to_string(),
            step.error,
            step.saved.to_string(),
            step.started,
            step.finished
        ],
    )
}

/// The order a run's steps are read in: the order they began.
///
/// When a step began is kept in milliseconds, and several steps of a
/// run begin in one, so it is not what they are read by: read by that
/// and then by name, a run that did `greet` and then `child` read
/// `child`, `greet`. Each row says which of its run's steps it was to
/// begin ([`put`]), and that alone orders those that say. It leads and
/// when a step began does not, so that a clock put back, and a step
/// whose record is written again later (taken up after a stop, tried
/// again), move nothing: a step is read where it first began.
///
/// A row a program from before wrote says nothing. Those are read
/// first, as they always were (by when they began and then by name):
/// whatever of their run says its place was first written after every
/// one of them was.
const IN_ORDER: &str = "nth IS NOT NULL, nth, started, step";

/// Calls off what an instance was to be carried over to, in whatever is
/// being written: it has taken a text, that one or another, or another
/// was asked for. Kept as called off, and no longer as what it was, so
/// that a conductor that starts on a run still under way says the right
/// thing of it as it ends it. Found among the runs under way, which are
/// few.
pub(crate) fn carried(connection: &rusqlite::Connection, instance: &str) -> rusqlite::Result<usize> {
    connection.execute_kept(
        "UPDATE runs SET carries = 'false'
         WHERE id IN (SELECT run FROM under_way WHERE instance = ?1) AND carries IS NOT NULL",
        [instance],
    )
}

/// Writes a run that is beginning, in whatever is being written, with
/// what it is to carry its instance over to where it is such a run. It
/// says whose it is ([`crate::apart`]): its instance's project, read
/// here from what the database keeps of the instance. And where the
/// journal it says things in had got to (`place`, its last number):
/// whatever the run says is after that.
fn make(
    connection: &rusqlite::Connection,
    run: &RunInfo,
    carries: Option<&Value>,
    place: u64,
) -> rusqlite::Result<usize> {
    connection.execute_kept(
        &format!(
            "INSERT INTO runs ({RUN_COLUMNS}, carries, place, project)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, {})",
            crate::apart::of_instance(2)
        ),
        params![
            run.id,
            run.instance,
            run.flow,
            run.revision,
            run.status.as_str(),
            run.event.to_string(),
            run.outputs.to_string(),
            run.error,
            run.started,
            run.finished,
            carries.map(Value::to_string),
            i64::try_from(place).unwrap_or(i64::MAX)
        ],
    )
}

/// Keeps which step asked for the run `run` of `instance`, where that is
/// the child `asking` asks for: in the run (`asked_by`, where it says
/// none), and by what the asking was named, where it was named
/// ([`crate::beside::ASKED`]: the first run an asking began is its run).
fn asked(
    connection: &rusqlite::Connection,
    run: &str,
    instance: &str,
    asking: &kernel::asking::Asking,
) -> rusqlite::Result<()> {
    let is: bool = connection.query_row_kept(
        "SELECT EXISTS (SELECT 1 FROM instances WHERE id = ?1 AND parent = ?2 AND key = ?3)",
        params![instance, asking.parent, asking.key],
        |row| row.get(0),
    )?;
    if !is {
        return Ok(());
    }
    connection.execute_kept(
        "UPDATE runs SET asked_by = ?2 WHERE id = ?1 AND asked_by IS NULL",
        params![run, asking.by],
    )?;
    if let Some(name) = &asking.asked {
        connection.execute_kept(
            "INSERT INTO kv (namespace, key, value) VALUES (?1, ?2, ?3) ON CONFLICT (namespace, key) DO NOTHING",
            params![crate::beside::ASKED, format!("{}\n{name}", asking.parent), Value::from(run).to_string()],
        )?;
    }
    Ok(())
}

impl Store {
    /// A run of an instance's steps that a step asked for (a `call` of
    /// its child: [`kernel::asking`]) keeps which step asked, in the
    /// write that makes it.
    pub async fn insert_run(&self, run: RunInfo) -> Result<()> {
        let place = self.journal.last_seq();
        let asking = kernel::asking::current().filter(|_| run.flow == "main");
        self.db
            .call(move |connection| {
                let transaction = journal::whole(connection)?;
                make(&transaction, &run, None, place)?;
                if let Some(asking) = &asking {
                    asked(&transaction, &run.id, &run.instance, asking)?;
                }
                transaction.commit()
            })
            .await
    }

    /// The same of a run that was begun with nobody asking (the start of a
    /// conductor came to a child a step had asked for, owed its beginning,
    /// before the step was taken up): kept as the step finds it.
    pub async fn asked_for(&self, run: &str, instance: &str, asking: kernel::asking::Asking) -> Result<()> {
        let (run, instance) = (run.to_string(), instance.to_string());
        self.db.call(move |connection| asked(connection, &run, &instance, &asking)).await
    }

    /// The run a step of `parent`'s asked for, where the event that set
    /// the step's run off named its asking `asked`; nothing before it is
    /// begun. Read by its name, however many runs there are.
    pub async fn begun_for(&self, parent: &str, asked: &str) -> Result<Option<String>> {
        let key = format!("{parent}\n{asked}");
        self.db
            .call(move |connection| {
                optional(connection.query_row_kept(
                    "SELECT value FROM kv WHERE namespace = ?1 AND key = ?2",
                    params![crate::beside::ASKED, key],
                    |row| row.get::<_, String>(0),
                ))
            })
            .await
            .map(|kept| kept.and_then(|kept| serde_json::from_str(&kept).ok()))
    }

    /// Writes the run that will carry an instance over to another text,
    /// and with it what it is to be carried over to: one write, so that a
    /// conductor stopped at any moment after it finds both. A text the
    /// instance was waiting to be carried over to before this is called
    /// off by it, and is kept no longer.
    /// `by` is the step that asked, where one did (`asked_by`).
    pub async fn insert_carry(&self, run: RunInfo, carries: Value, by: Option<String>) -> Result<()> {
        let place = self.journal.last_seq();
        self.db
            .call(move |connection| {
                let transaction = journal::whole(connection)?;
                carried(&transaction, &run.instance)?;
                make(&transaction, &run, Some(&carries), place)?;
                transaction
                    .execute_kept("UPDATE runs SET asked_by = ?2 WHERE id = ?1", params![run.id, by])?;
                transaction.commit()
            })
            .await
    }

    /// The child under `parent` by the name `key` that the step `by`
    /// asked to be carried over to a text, where the run that was to
    /// carry it over has taken it, or ended however it ended. One that
    /// still waits to carry it over is not given: asked for again, it is
    /// the run there is that answers.
    pub async fn carried_for(&self, parent: &str, key: &str, by: &str) -> Result<Option<String>> {
        let (parent, key, by) = (parent.to_string(), key.to_string(), by.to_string());
        self.db
            .call(move |connection| {
                optional(connection.query_row_kept(
                    "SELECT instances.id FROM instances JOIN runs ON runs.instance = instances.id
                     WHERE instances.parent = ?1 AND instances.key = ?2 AND runs.asked_by = ?3
                       AND (runs.carries IS NULL OR runs.carries = 'false')
                     ORDER BY runs.rowid DESC LIMIT 1",
                    params![parent, key, by],
                    |row| row.get(0),
                ))
            })
            .await
    }

    /// What the run that is to carry an instance over is to carry it over
    /// to, where that is still owed; `false` once the instance took a
    /// text or another was asked for, which called it off; and nothing
    /// for a run that has ended, or that never had anything kept.
    pub async fn carries(&self, run: &str) -> Result<Option<Value>> {
        let run = run.to_string();
        let kept: Option<Option<String>> = self
            .db
            .call(move |connection| {
                optional(connection.query_row_kept(
                    "SELECT carries FROM runs WHERE id = ?1 AND status = 'running'",
                    [run],
                    |row| row.get(0),
                ))
            })
            .await?;
        Ok(kept.flatten().and_then(|kept| serde_json::from_str(&kept).ok()))
    }

    /// Creates a handler's run and moves its place in the journal called
    /// `journal` past the event that caused it, together. Either both
    /// happen or neither, so an event never starts two runs and is never
    /// lost between "seen" and "started". With no place (`seq` is
    /// nought) the run was started by no event of a journal's.
    pub async fn insert_triggered_run(
        &self,
        run: RunInfo,
        handler: &str,
        journal: &str,
        seq: u64,
    ) -> Result<()> {
        let (handler, journal) = (handler.to_string(), journal.to_string());
        let place = self.journal.last_seq();
        self.db
            .call(move |connection| {
                let transaction = journal::whole(connection)?;
                make(&transaction, &run, None, place)?;
                if seq > 0 {
                    crate::places::put(&transaction, &run.instance, &handler, &journal, seq)?;
                }
                transaction.commit()
            })
            .await
    }

    /// Creates the run a `poll:` handler begins for what it read changing,
    /// and keeps what it read as what it last read, together (under
    /// [`crate::beside::POLL`], by `memory`): what was read is taken for
    /// seen only where a run was begun for it. Kept before the run was
    /// written, a reading the run never came of (the handler told to take
    /// no more as it read, the write refused, the daemon gone between the
    /// two) was the next reading's to be held to, and the change it was
    /// set nothing off.
    pub async fn insert_polled_run(&self, run: RunInfo, memory: &str, read: &Value) -> Result<()> {
        let (memory, read) = (memory.to_string(), read.to_string());
        let place = self.journal.last_seq();
        self.db
            .call(move |connection| {
                let transaction = journal::whole(connection)?;
                make(&transaction, &run, None, place)?;
                transaction.execute_kept(
                    "INSERT INTO kv (namespace, key, value) VALUES (?1, ?2, ?3)
                     ON CONFLICT (namespace, key) DO UPDATE SET value = excluded.value",
                    params![crate::beside::POLL, memory, read],
                )?;
                transaction.commit()
            })
            .await
    }

    /// Where the journal a run says things in had got to as the run was
    /// written: what the run said is after it. Nought where nothing says,
    /// as for a run a program from before wrote, or one that is not there.
    pub async fn run_place(&self, run: &str) -> Result<u64> {
        let run = run.to_string();
        self.db
            .call(move |connection| {
                let place: Option<Option<i64>> = optional(connection.query_row_kept(
                    "SELECT place FROM runs WHERE id = ?1",
                    [&run],
                    |row| row.get(0),
                ))?;
                Ok(place.flatten().and_then(|place| u64::try_from(place).ok()).unwrap_or(0))
            })
            .await
    }

    /// Whether a run's row says that it began, where its beginning was
    /// only passed on ([`crate::Store::begin_run`]), with where its journal
    /// had got to as it was written ([`Store::run_place`]), from which a
    /// beginning that was kept is looked for: one question of the
    /// database for the two.
    pub async fn run_begun(&self, run: &str) -> Result<(bool, u64)> {
        let run = run.to_string();
        self.db
            .call(move |connection| {
                let row: Option<(Option<i64>, Option<i64>)> = optional(connection.query_row_kept(
                    "SELECT begun, place FROM runs WHERE id = ?1",
                    [&run],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                ))?;
                let (begun, place) = row.unwrap_or((None, None));
                Ok((begun.is_some(), place.and_then(|place| u64::try_from(place).ok()).unwrap_or(0)))
            })
            .await
    }

    /// Where a step of a run began, as its row says it: the number its
    /// beginning was said with, and when. Nothing for a row a program
    /// from before wrote, whose beginning is in the journal.
    pub async fn step_began(&self, run: &str, step: &str) -> Result<Option<(u64, i64)>> {
        let (run, step) = (run.to_string(), step.to_string());
        self.db
            .call(move |connection| {
                let began: Option<(Option<i64>, Option<i64>)> = optional(connection.query_row_kept(
                    "SELECT began, began_at FROM steps WHERE run = ?1 AND step = ?2",
                    params![run, step],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                ))?;
                Ok(match began {
                    Some((Some(seq), Some(at))) => Some((seq as u64, at)),
                    _ => None,
                })
            })
            .await
    }

    /// A run with its steps, in the order they began ([`IN_ORDER`]).
    pub async fn run(&self, id: &str) -> Result<Option<RunInfo>> {
        let id = id.to_string();
        self.db
            .call(move |connection| {
                let run = optional(connection.query_row_kept(
                    &format!("SELECT {RUN_COLUMNS} FROM runs WHERE id = ?1"),
                    [&id],
                    read_run,
                ))?;
                let Some(mut run) = run else { return Ok(None) };
                run.steps = collect(
                    connection,
                    &format!("SELECT {STEP_COLUMNS} FROM steps WHERE run = ?1 ORDER BY {IN_ORDER}"),
                    [&id],
                    read_step,
                )?;
                Ok(Some(run))
            })
            .await
    }

    /// The last run of an instance's own steps, without its steps.
    pub async fn last_main(&self, instance: &str) -> Result<Option<RunInfo>> {
        let instance = instance.to_string();
        let found = self
            .db
            .call(move |connection| {
                collect(
                    connection,
                    &format!(
                        "SELECT {RUN_COLUMNS} FROM runs WHERE instance = ?1 AND flow = 'main' \
                         ORDER BY rowid DESC LIMIT 1"
                    ),
                    [&instance],
                    read_run,
                )
            })
            .await?;
        Ok(found.into_iter().next())
    }

    /// Runs of an instance, newest first, without their steps.
    pub async fn runs(&self, instance: &str, limit: usize) -> Result<Vec<RunInfo>> {
        let instance = instance.to_string();
        self.db
            .call(move |connection| {
                collect(
                    connection,
                    &format!(
                        "SELECT {RUN_COLUMNS} FROM runs WHERE instance = ?1 ORDER BY rowid DESC LIMIT {limit}"
                    ),
                    [&instance],
                    read_run,
                )
            })
            .await
    }

    /// The most recent runs across all instances, or across those applied
    /// for one project.
    pub async fn recent_runs(&self, limit: usize, project: Option<String>) -> Result<Vec<RunInfo>> {
        self.db
            .call(move |connection| match project {
                None => collect(
                    connection,
                    &format!("SELECT {RUN_COLUMNS} FROM runs ORDER BY rowid DESC LIMIT {limit}"),
                    [],
                    read_run,
                ),
                Some(project) => collect(
                    connection,
                    &format!(
                        "SELECT {RUN_COLUMNS} FROM runs
                         WHERE instance IN (SELECT id FROM instances WHERE project = ?1)
                         ORDER BY rowid DESC LIMIT {limit}"
                    ),
                    [&project],
                    read_run,
                ),
            })
            .await
    }

    /// Runs that were in progress when the conductor last stopped.
    pub async fn unfinished_runs(&self) -> Result<Vec<RunInfo>> {
        let columns = crate::apart::of_table("runs", RUN_COLUMNS);
        self.db
            .call(move |connection| {
                collect(
                    connection,
                    &format!(
                        "SELECT {columns} FROM under_way CROSS JOIN runs ON runs.id = under_way.run
                         ORDER BY runs.rowid"
                    ),
                    [],
                    read_run,
                )
            })
            .await
    }

    /// The latest main run of an instance.
    pub async fn last_main_run(&self, instance: &str) -> Result<Option<RunInfo>> {
        let instance = instance.to_string();
        self.db
            .call(move |connection| {
                optional(connection.query_row_kept(
                    &format!(
                        "SELECT {RUN_COLUMNS} FROM runs WHERE instance = ?1 AND flow = 'main'
                         ORDER BY rowid DESC LIMIT 1"
                    ),
                    [&instance],
                    read_run,
                ))
            })
            .await
    }

    /// Takes away the steps of a run whose names begin with `prefix`: what
    /// the steps of a task did, when the task is about to run them again.
    pub async fn forget_steps(&self, run: &str, prefix: &str) -> Result<()> {
        let (run, prefix) = (run.to_string(), prefix.to_string());
        self.db
            .call(move |connection| {
                // Not `LIKE`: a step's name may have `_` in it, which to
                // `LIKE` is any character.
                connection.execute_kept(
                    "DELETE FROM steps WHERE run = ?1 AND substr(step, 1, length(?2)) = ?2",
                    params![run, prefix],
                )
            })
            .await
            .map(|_| ())
    }

    /// Writes down what element `position` of a `for_each` step gave, so
    /// that a loop the conductor stops in the middle of is not begun again
    /// from its first element. Where each element is a task, the steps
    /// inside it that were recorded for this element are forgotten in the
    /// same stroke: they are named alike for every element, and those of
    /// one that has finished must never be taken for the next one's.
    pub async fn put_element(
        &self,
        run: &str,
        step: &str,
        position: usize,
        output: &Value,
        inside: Option<String>,
    ) -> Result<()> {
        let (run, step, output) = (run.to_string(), step.to_string(), output.to_string());
        self.db
            .call(move |connection| {
                let transaction = journal::whole(connection)?;
                transaction.execute_kept(
                    "INSERT OR REPLACE INTO step_elements (run, step, position, output) VALUES (?1, ?2, ?3, ?4)",
                    params![run, step, position as i64, output],
                )?;
                if let Some(prefix) = inside {
                    transaction.execute_kept(
                        "DELETE FROM steps WHERE run = ?1 AND substr(step, 1, length(?2)) = ?2",
                        params![run, prefix],
                    )?;
                }
                transaction.commit()
            })
            .await
    }

    /// Writes down that element `position` of a `for_each` step has begun,
    /// before anything of it is done: what tells an element the conductor
    /// stopped in the middle of from one it had not come to.
    /// `after` is the last thing the journal had kept by then.
    pub async fn begin_element(&self, run: &str, step: &str, position: usize, after: u64) -> Result<()> {
        let (run, step) = (run.to_string(), step.to_string());
        self.db
            .call(move |connection| {
                connection.execute_kept(
                    "INSERT OR IGNORE INTO step_elements (run, step, position, output, began) \
                     VALUES (?1, ?2, ?3, NULL, ?4)",
                    params![run, step, position as i64, after as i64],
                )
            })
            .await
            .map(|_| ())
    }

    /// The last thing the journal had kept when element `position` of a
    /// `for_each` step began, where that is written down.
    pub async fn element_began(&self, run: &str, step: &str, position: usize) -> Result<Option<u64>> {
        let (run, step) = (run.to_string(), step.to_string());
        let began: Option<Option<i64>> = self
            .db
            .call(move |connection| {
                optional(connection.query_row_kept(
                    "SELECT began FROM step_elements WHERE run = ?1 AND step = ?2 AND position = ?3",
                    params![run, step, position as i64],
                    |row| row.get(0),
                ))
            })
            .await?;
        Ok(began.flatten().map(|began| began as u64))
    }

    /// What the elements of a `for_each` step that have finished gave, in
    /// order from the first, as far as they go; and whether the one after
    /// them had begun.
    pub async fn elements(&self, run: &str, step: &str) -> Result<(Vec<Value>, bool)> {
        let (run, step) = (run.to_string(), step.to_string());
        self.db
            .call(move |connection| {
                let mut statement = connection.prepare_cached(
                    "SELECT position, output FROM step_elements WHERE run = ?1 AND step = ?2 ORDER BY position",
                )?;
                let rows = statement.query_map(params![run, step], |row| {
                    Ok((row.get::<_, i64>(0)?, row.get::<_, Option<String>>(1)?))
                })?;
                let mut gave = Vec::new();
                for row in rows {
                    let (position, output) = row?;
                    if position != gave.len() as i64 {
                        break;
                    }
                    match output {
                        Some(output) => gave.push(serde_json::from_str(&output).unwrap_or(Value::Null)),
                        None => return Ok((gave, true)),
                    }
                }
                Ok((gave, false))
            })
            .await
    }

    /// Forgets what the elements of a `for_each` step gave: the loop is
    /// beginning, or has ended.
    pub async fn forget_elements(&self, run: &str, step: &str) -> Result<()> {
        let (run, step) = (run.to_string(), step.to_string());
        self.db
            .call(move |connection| {
                connection
                    .execute("DELETE FROM step_elements WHERE run = ?1 AND step = ?2", params![run, step])
            })
            .await
            .map(|_| ())
    }

    /// Keeps how far what is done around a step has come, in the step's
    /// own record; with no step, what is done as the run ends, in the run's.
    pub async fn save_hooks(&self, run: &str, step: Option<&str>, kept: Value) -> Result<()> {
        let (run, step, kept) = (run.to_string(), step.map(str::to_string), kept.to_string());
        self.db
            .call(move |connection| match step {
                Some(step) => connection.execute_kept(
                    "UPDATE steps SET hooks = ?3 WHERE run = ?1 AND step = ?2",
                    params![run, step, kept],
                ),
                None => {
                    connection.execute_kept("UPDATE runs SET hooks = ?2 WHERE id = ?1", params![run, kept])
                }
            })
            .await
            .map(|_| ())
    }

    /// Keeps `kept` at `path` of what is kept beside a step's record and
    /// changes nothing else there, in one statement: written while the
    /// step's own action may write there too (a `set` keeps that its step
    /// has acted), where a whole read and written back would take that
    /// away.
    pub async fn keep_at(&self, run: &str, step: &str, path: String, kept: Value) -> Result<()> {
        let (run, step, kept) = (run.to_string(), step.to_string(), kept.to_string());
        self.db
            .call(move |connection| {
                connection.execute_kept(
                    "UPDATE steps SET hooks = json_set(coalesce(hooks, '{}'), ?3, json(?4))
                     WHERE run = ?1 AND step = ?2",
                    params![run, step, path, kept],
                )
            })
            .await
            .map(|_| ())
    }

    /// What was kept of that, for a step or a run that is being taken up.
    pub async fn kept_hooks(&self, run: &str, step: Option<&str>) -> Result<Value> {
        let (run, step) = (run.to_string(), step.map(str::to_string));
        let kept: Option<Option<String>> =
            self.db
                .call(move |connection| {
                    optional(match step {
                        Some(step) => connection.query_row_kept(
                            "SELECT hooks FROM steps WHERE run = ?1 AND step = ?2",
                            params![run, step],
                            |row| row.get(0),
                        ),
                        None => connection
                            .query_row("SELECT hooks FROM runs WHERE id = ?1", [run], |row| row.get(0)),
                    })
                })
                .await?;
        Ok(kept.flatten().and_then(|kept| serde_json::from_str(&kept).ok()).unwrap_or(Value::Null))
    }

    pub async fn save_checkpoint(&self, run: &str, step: &str, saved: Value) -> Result<()> {
        let (run, step) = (run.to_string(), step.to_string());
        self.db
            .call(move |connection| {
                connection.execute_kept(
                    "UPDATE steps SET saved = ?3 WHERE run = ?1 AND step = ?2",
                    params![run, step, saved.to_string()],
                )
            })
            .await
            .map(|_| ())
    }
}

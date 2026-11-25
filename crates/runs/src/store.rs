//! Durable state of instances, runs and steps: the tables, the shapes
//! read from them, and what is asked of an instance's own row.
//!
//! Listings are in insertion order (`rowid`), not by timestamp: two rows made
//! in the same millisecond still have a definite order.
//!
//! Every transition is written before anything acts on it. After a crash the
//! rows are the truth: an instance that says `active` is brought back, a run
//! that says `running` is picked up where it stopped, and a step that says
//! `running` is resumed or re-run by its action's rules.

use crate::apart::of_instance;
pub(crate) use crate::read::{
    INSTANCE_COLUMNS, INSTANCE_WRITTEN, RUN_COLUMNS, STEP_COLUMNS, collect, read_instance, read_run,
    read_step,
};
use crate::schema::SCHEMA;
use crate::together::{Done, Say, said};
use blueprint::Labels;
use journal::{Db, Journal, optional};
use kernel::{Cancel, Error, Fields, Principal, Result};
use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;
use serde_json::Value;

macro_rules! status_enum {
    ($name:ident { $($variant:ident => $text:literal),+ $(,)? }) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
        #[serde(rename_all = "lowercase")]
        pub enum $name { $($variant),+ }

        impl $name {
            pub fn as_str(self) -> &'static str {
                match self { $(Self::$variant => $text),+ }
            }

            pub(crate) fn parse(text: &str) -> Self {
                match text {
                    $($text => Self::$variant,)+
                    // An unknown status is a row written by a newer build;
                    // treating it as failed is the conservative reading.
                    _ => Self::Failed,
                }
            }
        }
    };
}

status_enum!(InstanceStatus {
    Starting => "starting",
    Active => "active",
    Done => "done",
    Failed => "failed",
    Stopped => "stopped",
});

status_enum!(RunStatus {
    Running => "running",
    Ok => "ok",
    Failed => "failed",
    Cancelled => "cancelled",
});

status_enum!(StepStatus {
    Running => "running",
    Ok => "ok",
    Failed => "failed",
    Skipped => "skipped",
    Cancelled => "cancelled",
});

impl InstanceStatus {
    /// Whether the instance is still doing or holding something.
    pub fn is_live(self) -> bool {
        matches!(self, InstanceStatus::Starting | InstanceStatus::Active)
    }
}

impl StepStatus {
    pub fn is_settled(self) -> bool {
        self != StepStatus::Running
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct InstanceInfo {
    pub id: String,
    pub blueprint: String,
    pub revision: String,
    #[serde(skip)]
    pub source: String,
    pub params: Fields,
    pub state: Fields,
    pub status: InstanceStatus,
    pub parent: Option<String>,
    /// Its name within its parent; re-applying a blueprint with the same key
    /// updates this instance rather than starting another.
    pub key: String,
    /// The instance this one is attached to, when it was started by `uses`.
    pub target: Option<String>,
    pub project: Option<String>,
    /// What everything measured under it also carries: what the step that
    /// started it said in `labels:`, added to what that step was under.
    pub labels: Labels,
    #[serde(skip)]
    pub principal: Principal,
    /// Things this instance owns that outlive its runs, e.g. services.
    pub holds: Vec<String>,
    pub error: Option<String>,
    pub created: i64,
    pub updated: i64,
    /// The definition to fall back to if the update in progress fails.
    #[serde(skip)]
    pub previous: Option<Definition>,
    /// How its text reads a handler or a wait that says no `scope`
    /// (`conductor::whose`): `project` in one taken by a program that has
    /// the word; nothing in one from before, which hears the whole
    /// factory, as it did, until somebody applies it again.
    pub heard: Option<String>,
    /// The environment of its project's it was applied under, the empty
    /// text for none: its own for its life, written as it is made and
    /// never another afterwards, and its children's.
    #[serde(default)]
    pub environment: String,
}

/// A blueprint's text, the params it was applied with, and who applied it.
#[derive(Clone, Debug, PartialEq, Serialize, serde::Deserialize)]
pub struct Definition {
    pub revision: String,
    pub source: String,
    pub params: Fields,
    /// Whoever applied it, with what they held then: the authority the
    /// text runs with. They are kept together so that going back to a
    /// definition is going back to its authority too. `None` only in one
    /// written down before this was kept, which runs with what its
    /// instance has.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub principal: Option<Principal>,
    /// The state to go back to with it, where going back to a definition
    /// is going back to its state too (`conductor::carry`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<Fields>,
    /// In one kept to go back to: what the instance held and which
    /// instances there were when the update began, so that going back
    /// hands back whatever the update started since ([`Started`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub before: Option<Started>,
    /// How it reads a handler or a wait that says no `scope`
    /// (`conductor::whose`). `None` in one written down before the word
    /// `project` was there: it hears the whole factory, as it was meant.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub heard: Option<String>,
}

/// Where an instance stood as an update of it began: what tells what the
/// update started from what was there.
#[derive(Clone, Debug, PartialEq, Serialize, serde::Deserialize)]
pub struct Started {
    /// What it held.
    pub holds: Vec<String>,
    /// The newest instance there was, as the table counts them. A child of
    /// its that comes after was made by the update.
    pub newest: i64,
    /// What is under it was owed a look, for an apply somebody asked for
    /// before: gone back to, it is owed still. Written with the text the
    /// update took ([`Store::update_instance_definition`]).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub below: bool,
}

/// One of the definitions an instance has been given.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Version {
    /// Counted from 1, in the order they were applied.
    pub version: u32,
    pub revision: String,
    pub source: String,
    pub params: Fields,
    /// Who applied it, as an actor and by name. Empty for one from before
    /// this was kept.
    pub by: String,
    pub by_name: String,
    pub applied: i64,
    /// `applied`, or `reverted` for an update that failed and was undone:
    /// the instance went back to the version before it.
    pub outcome: String,
    /// On whose authority it ran. For a version from before each apply
    /// took the authority of whoever asked, that is whoever applied the
    /// instance first, and need not be `by`.
    #[serde(skip)]
    pub principal: Option<Principal>,
}

fn principal_text(principal: &Principal) -> String {
    serde_json::to_string(principal).unwrap_or_default()
}

/// Writes down a definition an instance was given, after the ones before
/// it, and says which version it is: the one that keeps the text the
/// instance runs from now (`instances.text`), which its own row then
/// leaves out.
pub(crate) fn record_version(
    connection: &Connection,
    instance: &str,
    definition: &Definition,
    now: i64,
) -> rusqlite::Result<i64> {
    let by = definition.principal.as_ref();
    connection.query_row(
        "INSERT INTO instance_versions
             (instance, version, revision, source, params, actor, actor_name, applied, principal)
         SELECT ?1, coalesce(max(version), 0) + 1, ?2, ?3, ?4, ?5, ?6, ?7, ?8
         FROM instance_versions WHERE instance = ?1
         RETURNING version",
        params![
            instance,
            definition.revision,
            definition.source,
            Value::Object(definition.params.clone()).to_string(),
            by.map(Principal::actor).unwrap_or_default(),
            by.map(|by| by.name.clone()).unwrap_or_default(),
            now,
            by.map(principal_text),
        ],
        |row| row.get(0),
    )
}

#[derive(Clone, Debug, Serialize)]
pub struct RunInfo {
    pub id: String,
    pub instance: String,
    /// `main`, `on.<handler>`, or `on_update`.
    pub flow: String,
    pub revision: String,
    pub status: RunStatus,
    /// The event that fired the handler; `null` for the main flow.
    pub event: Value,
    pub outputs: Value,
    pub error: Option<String>,
    pub started: i64,
    pub finished: Option<i64>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub steps: Vec<StepInfo>,
}

#[derive(Clone, Debug, Serialize)]
pub struct StepInfo {
    pub run: String,
    pub step: String,
    pub status: StepStatus,
    pub attempt: u32,
    /// Failed, but the run was allowed to carry on.
    pub tolerated: bool,
    pub output: Value,
    pub error: Option<String>,
    #[serde(skip)]
    pub saved: Value,
    pub started: i64,
    pub finished: Option<i64>,
}

/// Which instances to list.
#[derive(Clone, Debug, Default)]
pub struct InstanceFilter {
    pub live_only: bool,
    pub parent: Option<String>,
    /// Only instances with no parent.
    pub roots_only: bool,
    pub project: Option<String>,
    pub blueprint: Option<String>,
    /// Only those under this environment of their project's.
    pub environment: Option<String>,
    pub limit: usize,
}

/// What each reading of one instance's or run's row is measured as that
/// is asked for want of it being in hand (its parent, whose it is, or the
/// blueprint it runs), by what was read (`kind`: `parent`, `project`,
/// `run`, and `blueprint`, read by the keeper of a record's history in
/// each journal in turn: `studio::through`). Whoever asks looks
/// in hand first, so whether one is read goes by whether the instance had
/// been let go of in that moment, as it ends: what one piece of work cost
/// the factory is read without them (`costed.rs` in the API suite).
pub const LOOKED_UP: &str = "instance.looked_up";

/// The blueprint an instance runs, by the name it was made with, read
/// from its row in `db`: nothing where `db` keeps no row of it, or keeps
/// no instances at all. It makes nothing and opens no store, so that
/// what reads it with no runner in hand (a record's history,
/// `studio::through`) may ask it of any journal's database.
pub async fn blueprint_kept(db: &Db, id: &str) -> Option<String> {
    let id = id.to_string();
    let read = db.call(move |connection| {
        optional(
            connection.query_row("SELECT blueprint FROM instances WHERE id = ?1", [&id], |row| row.get(0)),
        )
    });
    read.await.ok().flatten()
}

#[derive(Clone)]
pub struct Store {
    pub(crate) db: Db,
    /// For what is written and said as one write (`together`).
    pub(crate) journal: Journal,
}

impl Store {
    pub async fn open(journal: &Journal) -> Result<Store> {
        let db = journal.db().clone();
        // Under the runner's name, as they always were: the tables were
        // the conductor's own before they were a crate's, and a journal
        // from before knows them by that.
        db.migrate("conductor", SCHEMA).await?;
        // What a journal from before still has that nothing reads, where
        // taking it away is nothing.
        db.call(|connection| crate::schema::shed(connection)).await?;
        Ok(Store { db, journal: journal.clone() })
    }

    /// Writes a new instance down and says that it started (`says`), as
    /// one write, and says whether it did. One asked for under an
    /// instance of another project is refused, and nothing is written.
    ///
    /// `under` is what is cancelled when the instance it is started under
    /// stops. It is looked at here, on the database's own thread, where
    /// work is done in the order it was asked for: whoever stops an
    /// instance cancels first and counts its children afterwards, so a
    /// child is either written down before the count and stopped with the
    /// rest, or comes after the cancelling and is not written down at all.
    /// Looked at anywhere else, one could be written down between the two
    /// and go on with nothing above it.
    pub async fn insert_instance(
        &self,
        info: InstanceInfo,
        under: Option<Cancel>,
        says: Vec<Say>,
    ) -> Result<bool> {
        let wrote = self
            .journal
            .write(move |writing| {
                let connection = writing.transaction();
                if under.as_ref().is_some_and(Cancel::is_cancelled) {
                    return Ok(Ok(false));
                }
                // A tree of instances never spans two projects: what is
                // started under something is of its project. Asked here,
                // of the parent's own row, since everything that tells
                // rows apart by project rests on it (`crate::apart`).
                if let Some(parent) = &info.parent {
                    let of: Option<String> = optional(connection.query_row(
                        &format!("SELECT {} WHERE EXISTS (SELECT 1 FROM instances WHERE id = ?1)", of_instance(1)),
                        [parent],
                        |row| row.get(0),
                    ))?;
                    if let Some(of) = of.filter(|of| *of != crate::apart::said(info.project.as_deref())) {
                        return Ok(Err(spans(&info, &of)));
                    }
                }
                // Its first version is what it starts with.
                let first = Definition {
                    revision: info.revision.clone(),
                    source: info.source.clone(),
                    params: info.params.clone(),
                    principal: Some(info.principal.clone()),
                    state: None,
                    before: None,
                    heard: info.heard.clone(),
                };
                let text = record_version(connection, &info.id, &first, info.created)?;
                connection.execute(
                    &format!(
                        "INSERT INTO instances ({INSTANCE_WRITTEN})
                         VALUES (?1, ?2, ?3, '', ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, NULL, ?16, ?17, ?18, ?19)"
                    ),
                    params![
                        info.id,
                        info.blueprint,
                        info.revision,
                        Value::Object(info.params).to_string(),
                        Value::Object(info.state).to_string(),
                        info.status.as_str(),
                        info.parent,
                        info.key,
                        info.target,
                        info.project,
                        principal_text(&info.principal),
                        serde_json::to_string(&info.holds).unwrap_or_default(),
                        info.error,
                        info.created,
                        info.updated,
                        serde_json::to_string(&info.labels).unwrap_or_default(),
                        info.heard,
                        text,
                        info.environment,
                    ],
                )?;
                said(writing, says)?;
                Ok(Ok(true))
            })
            .await?;
        wrote.0.map_err(Error::invalid)
    }

    pub async fn instance(&self, id: &str) -> Result<Option<InstanceInfo>> {
        let id = id.to_string();
        self.db
            .call(move |connection| {
                optional(connection.query_row(
                    &format!("SELECT {INSTANCE_COLUMNS} FROM instances WHERE id = ?1"),
                    [&id],
                    read_instance,
                ))
            })
            .await
    }

    /// The last instance started under `parent` by that name, alive or
    /// not.
    pub async fn last_instance_by_key(&self, parent: &str, key: &str) -> Result<Option<InstanceInfo>> {
        let (parent, key) = (parent.to_string(), key.to_string());
        self.db
            .call(move |connection| {
                optional(connection.query_row(
                    &format!(
                        "SELECT {INSTANCE_COLUMNS} FROM instances WHERE key = ?1 AND parent = ?2
                         ORDER BY rowid DESC LIMIT 1"
                    ),
                    params![key, parent],
                    read_instance,
                ))
            })
            .await
    }

    /// The last instance started under `parent` by that name that is
    /// over, whatever was started by that name since.
    pub async fn last_over_by_key(&self, parent: &str, key: &str) -> Result<Option<InstanceInfo>> {
        let (parent, key) = (parent.to_string(), key.to_string());
        self.db
            .call(move |connection| {
                optional(connection.query_row(
                    &format!(
                        "SELECT {INSTANCE_COLUMNS} FROM instances
                         WHERE key = ?1 AND parent = ?2 AND status NOT IN ('starting', 'active')
                         ORDER BY rowid DESC LIMIT 1"
                    ),
                    params![key, parent],
                    read_instance,
                ))
            })
            .await
    }

    /// The live instance called `key`: among a parent's children, or, for
    /// one with no parent, among those applied for the same project. Two
    /// projects may each have a `fleet`; they are not the same fleet.
    /// One with no parent is looked for among what is alive of its own
    /// project, and nothing of another's is read.
    pub async fn live_instance_by_key(
        &self,
        parent: Option<&str>,
        key: &str,
        project: Option<&str>,
    ) -> Result<Option<InstanceInfo>> {
        let (parent, key, project) =
            (parent.map(str::to_string), key.to_string(), project.map(str::to_string));
        self.db
            .call(move |connection| {
                optional(match parent {
                    Some(parent) => connection.query_row(
                        &format!(
                            "SELECT {INSTANCE_COLUMNS} FROM instances
                             WHERE key = ?1 AND parent = ?2 AND status IN ('starting', 'active')
                             ORDER BY rowid DESC LIMIT 1"
                        ),
                        params![key, parent],
                        read_instance,
                    ),
                    None => connection.query_row(
                        &format!(
                            "SELECT {} FROM alive INDEXED BY alive_project
                             CROSS JOIN instances ON instances.id = alive.instance
                             WHERE alive.project = ?2 AND key = ?1 AND parent IS NULL
                             ORDER BY instances.rowid DESC LIMIT 1",
                            crate::apart::of_table("instances", INSTANCE_COLUMNS)
                        ),
                        params![key, crate::apart::said(project.as_deref())],
                        read_instance,
                    ),
                })
            })
            .await
    }

    /// What an instance is a child of, whether or not it is still
    /// running; nothing for one that has no parent, or that is not known.
    pub async fn parent_of(&self, id: &str) -> Result<Option<String>> {
        let id = id.to_string();
        let began = std::time::Instant::now();
        let read = self.db.call(move |connection| {
            optional(connection.query_row("SELECT parent FROM instances WHERE id = ?1", [&id], |row| {
                row.get::<_, Option<String>>(0)
            }))
        });
        let read = read.await;
        self.looked_up("parent", began, read.is_ok());
        read.map(Option::flatten)
    }

    /// Measures a reading of one row asked for want of it in hand
    /// ([`LOOKED_UP`]).
    pub(crate) fn looked_up(&self, kind: &str, began: std::time::Instant, ok: bool) {
        let took = began.elapsed().as_secs_f64() * 1000.0;
        self.journal.measure(LOOKED_UP, &[("kind", kind)], took, ok);
    }

    /// The newest instance there is, as the table counts them; 0 for none.
    pub async fn newest_instance(&self) -> Result<i64> {
        self.db
            .call(|connection| {
                connection.query_row("SELECT coalesce(max(rowid), 0) FROM instances", [], |row| row.get(0))
            })
            .await
    }

    /// The child of `parent` called `key` that was made after `newest` was
    /// the newest instance there was, running or not; the latest, if there
    /// is more than one.
    pub async fn child_since(&self, parent: &str, key: &str, newest: i64) -> Result<Option<InstanceInfo>> {
        let (parent, key) = (parent.to_string(), key.to_string());
        self.db
            .call(move |connection| {
                optional(connection.query_row(
                    &format!(
                        "SELECT {INSTANCE_COLUMNS} FROM instances
                         WHERE parent = ?1 AND key = ?2 AND rowid > ?3
                         ORDER BY rowid DESC LIMIT 1"
                    ),
                    params![parent, key, newest],
                    read_instance,
                ))
            })
            .await
    }

    /// The live children of `parent` that were made after `newest` was the
    /// newest instance there was.
    pub async fn children_since(&self, parent: &str, newest: i64) -> Result<Vec<String>> {
        let parent = parent.to_string();
        self.db
            .call(move |connection| {
                let mut statement = connection.prepare(
                    "SELECT id FROM instances
                     WHERE parent = ?1 AND rowid > ?2 AND status IN ('starting', 'active')
                     ORDER BY rowid",
                )?;
                let rows = statement.query_map(params![parent, newest], |row| row.get::<_, String>(0))?;
                rows.collect()
            })
            .await
    }

    pub async fn instances(&self, filter: InstanceFilter) -> Result<Vec<InstanceInfo>> {
        self.db
            .call(move |connection| {
                // What is alive is read from what is kept of it, by
                // project where one is asked for (`crate::apart`), unless
                // it is a parent's children that are asked for, which are
                // found by the parent.
                let alive = filter.live_only && filter.parent.is_none();
                let mut clauses = vec!["1 = 1".to_string()];
                let mut values: Vec<rusqlite::types::Value> = Vec::new();
                let mut bind = |clause: &str, value: String| {
                    values.push(rusqlite::types::Value::Text(value));
                    clauses.push(format!("{clause} ?{}", values.len()));
                };
                if let Some(parent) = filter.parent {
                    bind("instances.parent =", parent);
                }
                if let Some(project) = filter.project {
                    bind(if alive { "alive.project =" } else { "instances.project =" }, project);
                }
                if let Some(blueprint) = filter.blueprint {
                    bind("instances.blueprint =", blueprint);
                }
                if let Some(environment) = filter.environment {
                    bind("instances.environment =", environment);
                }
                if filter.live_only && !alive {
                    clauses.push("instances.status IN ('starting', 'active')".into());
                }
                if filter.roots_only {
                    clauses.push("instances.parent IS NULL".into());
                }
                let limit = if filter.limit == 0 { 500 } else { filter.limit };
                let from = match alive {
                    true => "alive CROSS JOIN instances ON instances.id = alive.instance",
                    false => "instances",
                };
                collect(
                    connection,
                    &format!(
                        "SELECT {} FROM {from} WHERE {} ORDER BY instances.rowid DESC LIMIT {limit}",
                        crate::apart::of_table("instances", INSTANCE_COLUMNS),
                        clauses.join(" AND ")
                    ),
                    rusqlite::params_from_iter(values),
                    read_instance,
                )
            })
            .await
    }

    /// Says `note` of a live instance beside its status, where it says
    /// nothing else; with none, takes back what it says that begins
    /// `begins`. Its status is not touched: this is not what decides it.
    pub async fn set_instance_note(
        &self,
        id: &str,
        note: Option<String>,
        begins: String,
        now: i64,
    ) -> Result<()> {
        let id = id.to_string();
        self.db
            .call(move |connection| {
                connection.execute(
                    "UPDATE instances SET error = ?2, updated = ?4
                     WHERE id = ?1 AND status IN ('starting', 'active')
                       AND ((?2 IS NOT NULL AND error IS NULL) OR substr(error, 1, length(?3)) = ?3)",
                    params![id, note, begins, now],
                )
            })
            .await
            .map(|_| ())
    }

    pub async fn set_instance_state(&self, id: &str, state: Fields, now: i64) -> Result<()> {
        self.set_instance_state_with(id, state, None, now, Vec::new()).await
    }

    /// Writes what an instance keeps and, in the one transaction, the
    /// record that the step which set it has acted ([`Done`]): after any
    /// stop both are there or neither is.
    pub async fn set_instance_state_with(
        &self,
        id: &str,
        state: Fields,
        done: Option<Done>,
        now: i64,
        says: Vec<Say>,
    ) -> Result<()> {
        let id = id.to_string();
        self.journal
            .write(move |writing| {
                let transaction = writing.transaction();
                transaction.execute(
                    "UPDATE instances SET state = ?2, updated = ?3 WHERE id = ?1",
                    params![id, Value::Object(state).to_string(), now],
                )?;
                match done {
                    None => {}
                    Some(Done::Step { run, step, gave }) => {
                        let acted = serde_json::json!({ "ok": true, "output": gave }).to_string();
                        transaction.execute(
                            "UPDATE steps SET hooks = json_set(coalesce(hooks, '{}'), '$.acted', json(?3))
                             WHERE run = ?1 AND step = ?2",
                            params![run, step, acted],
                        )?;
                    }
                    Some(Done::Element { run, step, position, gave }) => {
                        transaction.execute(
                            "INSERT OR REPLACE INTO step_elements (run, step, position, output)
                             VALUES (?1, ?2, ?3, ?4)",
                            params![run, step, position as i64, gave.to_string()],
                        )?;
                    }
                    Some(Done::Stopping { instance, done }) => {
                        transaction.execute(
                            "UPDATE instances SET hooks = json_set(coalesce(hooks, '{}'), '$.end', ?2)
                             WHERE id = ?1",
                            params![instance, done as i64],
                        )?;
                    }
                    Some(Done::Entry { run, step, start, done }) => {
                        let path = if start { "$.start" } else { "$.end" };
                        let set = format!("hooks = json_set(coalesce(hooks, '{{}}'), '{path}', ?2)");
                        match step {
                            Some(step) => transaction.execute(
                                &format!("UPDATE steps SET {set} WHERE run = ?1 AND step = ?3"),
                                params![run, done as i64, step],
                            )?,
                            None => transaction.execute(
                                &format!("UPDATE runs SET {set} WHERE id = ?1"),
                                params![run, done as i64],
                            )?,
                        };
                    }
                }
                said(writing, says)
            })
            .await
            .map(|_| ())
    }

    pub async fn set_instance_holds(&self, id: &str, holds: Vec<String>, now: i64) -> Result<()> {
        let id = id.to_string();
        self.db
            .call(move |connection| {
                connection.execute(
                    "UPDATE instances SET holds = ?2, updated = ?3 WHERE id = ?1",
                    params![id, serde_json::to_string(&holds).unwrap_or_default(), now],
                )
            })
            .await
            .map(|_| ())
    }

    /// Points an instance at a new definition, writes it down as the
    /// instance's next version, and gives the instance the authority of
    /// whoever applied it: all at once, so that after a restart the text is
    /// never found with somebody else's authority. `previous` is what to
    /// fall back to if the run that follows fails; `None` leaves it as it is.
    /// Nothing where it was not written; and otherwise whether what is
    /// under it is owed a look once its steps have settled ([`Below`]).
    pub async fn update_instance_definition(
        &self,
        id: &str,
        taken: Taken,
        now: i64,
        says: Vec<Say>,
    ) -> Result<Option<Below>> {
        let id = id.to_string();
        let Taken { definition, state, previous, below, places } = taken;
        self.journal
            .write(move |writing| {
                let transaction = writing.transaction();
                let owed = "SELECT owes IS 'below' FROM instances WHERE id = ?1";
                let kept: bool =
                    transaction.query_row(owed, [&id], |row| row.get(0)).optional()?.unwrap_or(false);
                // Only one that is still running takes a text: one that
                // ended as it was being applied over has said how it
                // ended, and something is given only where it was
                // written. A look owed from before is owed still, and in
                // what it would go back to.
                let written = transaction.execute(
                    "UPDATE instances SET revision = ?2, source = ?3, text = NULL, params = ?4, state = ?5,
                            status = 'starting', error = NULL, updated = ?6,
                            previous = coalesce(json_set(?7, '$.before.below', json(?11)), previous),
                            principal = coalesce(?8, principal),
                            owes = CASE WHEN ?9 OR owes = 'below' THEN 'below' ELSE owes END,
                            heard = ?10
                     WHERE id = ?1 AND status IN ('starting', 'active')",
                    params![
                        id,
                        definition.revision,
                        // Kept in the version written below, which the row then names.
                        "",
                        Value::Object(definition.params.clone()).to_string(),
                        Value::Object(state).to_string(),
                        now,
                        previous.map(|previous| serde_json::to_string(&previous).unwrap_or_default()),
                        definition.principal.as_ref().map(principal_text),
                        below,
                        definition.heard,
                        if kept { "true" } else { "false" },
                    ],
                )?;
                if written == 1 {
                    crate::runs::carried(transaction, &id)?;
                    let text = record_version(transaction, &id, &definition, now)?;
                    transaction.execute("UPDATE instances SET text = ?2 WHERE id = ?1", params![id, text])?;
                    for (handler, journal, from) in places {
                        crate::places::make(transaction, &id, &handler, &journal, from)?;
                    }
                    said(writing, says)?;
                }
                let owes = match (kept, below) {
                    (true, _) => Below::Kept,
                    (false, true) => Below::Asked,
                    (false, false) => Below::Not,
                };
                Ok((written == 1).then_some(owes))
            })
            .await
            .map(|wrote| wrote.0)
    }

    /// The definitions an instance has been given, the latest first.
    pub async fn versions(&self, instance: &str, limit: usize) -> Result<Vec<Version>> {
        let instance = instance.to_string();
        self.db
            .call(move |connection| {
                connection
                    .prepare(
                        "SELECT version, revision, source, params, actor, actor_name, applied, outcome, principal
                         FROM instance_versions WHERE instance = ?1 ORDER BY version DESC LIMIT ?2",
                    )?
                    .query_map(params![instance, limit as i64], |row| {
                        let params: String = row.get(3)?;
                        Ok(Version {
                            version: row.get(0)?,
                            revision: row.get(1)?,
                            source: row.get(2)?,
                            params: serde_json::from_str(&params).unwrap_or_default(),
                            by: row.get(4)?,
                            by_name: row.get(5)?,
                            applied: row.get(6)?,
                            outcome: row.get(7)?,
                            principal: row
                                .get::<_, Option<String>>(8)?
                                .and_then(|text| serde_json::from_str(&text).ok()),
                        })
                    })?
                    .collect()
            })
            .await
    }
}

/// Whether what is under an instance that took a text is owed a look
/// once its steps have settled.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Below {
    Not,
    /// Because somebody asked for this text.
    Asked,
    /// Because somebody asked for one before it, and the look is not over:
    /// no text taken since takes it away, and it is owed still where the
    /// instance goes back.
    Kept,
}

impl Below {
    /// Whether a look is owed at all.
    pub fn owed(self) -> bool {
        self != Below::Not
    }
}

/// A text an instance takes, with what goes with it: the state it then
/// has; what to go back to if the run that follows fails (nothing leaves
/// that as it is); and whether what is under it is owed a look once its
/// steps have settled (`below`), because somebody asked.
pub struct Taken {
    pub definition: Definition,
    pub state: Fields,
    pub previous: Option<Definition>,
    pub below: bool,
    /// Where each handler the text brings begins (`trigger::places`):
    /// the handler, what the journal it is a place in is called, and the
    /// place.
    pub places: Vec<(String, String, u64)>,
}

/// Ends an update, in whatever is being written. With `revert`, the
/// instance goes back to that definition and to the authority it was
/// applied with, and stays active, carrying `error` to say why.
pub(crate) fn end_update(
    connection: &Connection,
    id: &str,
    revert: Option<Definition>,
    error: Option<String>,
    now: i64,
) -> rusqlite::Result<usize> {
    let Some(definition) = revert else {
        return connection.execute("UPDATE instances SET previous = NULL WHERE id = ?1", params![id]);
    };
    let kept = definition.before.as_ref().is_some_and(|before| before.below);
    // The version that was just tried did not hold.
    connection.execute(
        "UPDATE instance_versions SET outcome = 'reverted'
         WHERE instance = ?1
           AND version = (SELECT max(version) FROM instance_versions WHERE instance = ?1)",
        params![id],
    )?;
    // Only one that is still running goes back to running: stopped
    // meanwhile, it stays stopped.
    connection.execute(
        "UPDATE instances SET revision = ?2, source = ?3, text = NULL, params = ?4, status = 'active',
                error = ?5, updated = ?6, previous = NULL,
                principal = coalesce(?7, principal), state = coalesce(?8, state),
                owes = CASE WHEN owes = 'below' AND NOT ?10 THEN NULL ELSE owes END,
                heard = ?9
         WHERE id = ?1 AND status IN ('starting', 'active')",
        params![
            id,
            definition.revision,
            definition.source,
            Value::Object(definition.params).to_string(),
            error,
            now,
            definition.principal.as_ref().map(principal_text),
            definition.state.map(|state| Value::Object(state).to_string()),
            definition.heard,
            kept,
        ],
    )
}

/// Why an instance is not written under one of another project
/// (`parents`, the empty text for one that is nobody's).
fn spans(info: &InstanceInfo, parents: &str) -> String {
    let of = |project: Option<&str>| match project.filter(|project| !project.is_empty()) {
        Some(project) => format!("the project `{project}`"),
        None => "no project".to_string(),
    };
    format!(
        "`{}` was not started: it was asked for as something of {}, under `{}`, which is of {}. What an \
         instance starts is of its own project, so that a tree of instances is never of two; apply it for \
         that project, or by itself",
        info.blueprint,
        of(info.project.as_deref()),
        info.parent.as_deref().unwrap_or_default(),
        of(Some(parents)),
    )
}

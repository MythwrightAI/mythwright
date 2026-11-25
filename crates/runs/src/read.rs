//! How a row of each of the store's tables is read: the columns asked
//! for, in the order they are read in, and what each row comes to. Kept
//! beside the store, under the names the store always gave them.

use kernel::{Fields, Principal};
use rusqlite::{Connection, Row};
use serde_json::Value;

use crate::store::{InstanceInfo, InstanceStatus, RunInfo, RunStatus, StepInfo, StepStatus};

pub(crate) fn json(text: String) -> Value {
    serde_json::from_str(&text).unwrap_or(Value::Null)
}

pub(crate) fn object(text: String) -> Fields {
    match json(text) {
        Value::Object(map) => map,
        _ => Fields::new(),
    }
}

/// The columns an instance is read by, in the order [`read_instance`]
/// reads them. Its text is in the row where nothing says which of its
/// versions keeps it (a row a program from before wrote, and one that
/// went back to what it ran before an update), and is read from that
/// version otherwise: one row found by its key.
pub(crate) const INSTANCE_COLUMNS: &str = "id, blueprint, revision, CASE WHEN instances.text IS NULL THEN instances.source \
                                ELSE (SELECT kept.source FROM instance_versions kept \
                                      WHERE kept.instance = instances.id AND kept.version = instances.text) END, \
                                params, state, status, parent, key, target, \
                                project, principal, holds, error, created, updated, previous, labels, heard, environment";

/// The columns an instance is written with, in the order it is handed
/// them: what it is read by, with its text left out of the row and the
/// version that keeps it said in its place ([`crate::store::record_version`]).
pub(crate) const INSTANCE_WRITTEN: &str = "id, blueprint, revision, source, params, state, status, parent, key, target, \
                                project, principal, holds, error, created, updated, previous, labels, heard, text, environment";

pub(crate) fn read_instance(row: &Row<'_>) -> rusqlite::Result<InstanceInfo> {
    let principal: String = row.get(11)?;
    Ok(InstanceInfo {
        id: row.get(0)?,
        blueprint: row.get(1)?,
        revision: row.get(2)?,
        source: row.get(3)?,
        params: object(row.get(4)?),
        state: object(row.get(5)?),
        status: InstanceStatus::parse(&row.get::<_, String>(6)?),
        parent: row.get(7)?,
        key: row.get(8)?,
        target: row.get(9)?,
        project: row.get(10)?,
        principal: serde_json::from_str(&principal).unwrap_or_else(|_| Principal {
            // A row whose principal cannot be read keeps running, but with no
            // authority: every action that needs a capability will refuse.
            kind: kernel::PrincipalKind::System,
            id: "unknown".into(),
            name: "unknown".into(),
            scopes: Vec::new(),
            key: None,
        }),
        holds: serde_json::from_str(&row.get::<_, String>(12)?).unwrap_or_default(),
        error: row.get(13)?,
        created: row.get(14)?,
        updated: row.get(15)?,
        previous: row.get::<_, Option<String>>(16)?.and_then(|text| serde_json::from_str(&text).ok()),
        labels: serde_json::from_str(&row.get::<_, String>(17)?).unwrap_or_default(),
        heard: row.get(18)?,
        environment: row.get(19)?,
    })
}

pub(crate) const RUN_COLUMNS: &str =
    "id, instance, flow, revision, status, event, outputs, error, started, finished";

pub(crate) fn read_run(row: &Row<'_>) -> rusqlite::Result<RunInfo> {
    Ok(RunInfo {
        id: row.get(0)?,
        instance: row.get(1)?,
        flow: row.get(2)?,
        revision: row.get(3)?,
        status: RunStatus::parse(&row.get::<_, String>(4)?),
        event: json(row.get(5)?),
        outputs: json(row.get(6)?),
        error: row.get(7)?,
        started: row.get(8)?,
        finished: row.get(9)?,
        steps: Vec::new(),
    })
}

pub(crate) const STEP_COLUMNS: &str =
    "run, step, status, attempt, tolerated, output, error, saved, started, finished";

pub(crate) fn read_step(row: &Row<'_>) -> rusqlite::Result<StepInfo> {
    Ok(StepInfo {
        run: row.get(0)?,
        step: row.get(1)?,
        status: StepStatus::parse(&row.get::<_, String>(2)?),
        attempt: row.get::<_, i64>(3)? as u32,
        tolerated: row.get::<_, i64>(4)? != 0,
        output: json(row.get(5)?),
        error: row.get(6)?,
        saved: json(row.get(7)?),
        started: row.get(8)?,
        finished: row.get(9)?,
    })
}

pub(crate) fn collect<T>(
    connection: &Connection,
    sql: &str,
    params: impl rusqlite::Params,
    read: fn(&Row<'_>) -> rusqlite::Result<T>,
) -> rusqlite::Result<Vec<T>> {
    let mut statement = connection.prepare(sql)?;
    let rows = statement.query_map(params, read)?;
    rows.collect()
}

//! What else is asked of the instances as they are kept, each for one
//! thing the runner does: the text an instance takes where it is, what it
//! is called under its parent, the labels it carries, which blueprints
//! are in use where, and an instance beside the last run of its steps as
//! of one moment. Each was written beside what asks it, in the runner,
//! and is here because it reads or writes the rows.

use std::collections::HashMap;

use blueprint::Labels;
use journal::optional;
use kernel::Result;
use rusqlite::params;

use crate::store::{
    Definition, INSTANCE_COLUMNS, InstanceInfo, RUN_COLUMNS, RunInfo, Store, read_instance, read_run,
    record_version,
};
use crate::together::{Say, said};

impl Store {
    /// An instance as it is written, and the last run of its steps, as
    /// of one moment: one answer of the database, which takes what is
    /// asked of it in order. Nothing, where there is no such instance.
    ///
    /// For whoever decides by the two together. Asked apart, a run that
    /// ended between the askings is read as over beside an instance read
    /// before it had become what the run made it.
    pub async fn standing(&self, id: &str) -> Result<Option<(InstanceInfo, Option<RunInfo>)>> {
        let id = id.to_string();
        self.db
            .call(move |connection| {
                let instance = optional(connection.query_row(
                    &format!("SELECT {INSTANCE_COLUMNS} FROM instances WHERE id = ?1"),
                    [&id],
                    read_instance,
                ))?;
                let Some(instance) = instance else { return Ok(None) };
                let last = optional(connection.query_row(
                    &format!(
                        "SELECT {RUN_COLUMNS} FROM runs WHERE instance = ?1 AND flow = 'main'
                         ORDER BY rowid DESC LIMIT 1"
                    ),
                    [&id],
                    read_run,
                ))?;
                Ok(Some((instance, last)))
            })
            .await
    }

    /// Gives an instance another text where it is, and the authority of
    /// whoever applied it, written down as its next version: its params,
    /// its state and its status are left as they are, and so is what it
    /// says of itself. And says that it took it (`says`), in that write,
    /// with the place each handler the text brings begins from
    /// (`places`: the handler, the journal and the place in it), so
    /// that what is said once the text is said to be taken is after
    /// where they begin, whatever is stopped in between.
    pub async fn take_text(
        &self,
        id: &str,
        definition: Definition,
        places: Vec<(String, String, u64)>,
        now: i64,
        says: Vec<Say>,
    ) -> Result<()> {
        let id = id.to_string();
        self.journal
            .write(move |writing| {
                let transaction = writing.transaction();
                crate::runs::carried(transaction, &id)?;
                let text = record_version(transaction, &id, &definition, now)?;
                for (handler, journal, from) in places {
                    crate::places::make(transaction, &id, &handler, &journal, from)?;
                }
                transaction.execute(
                    "UPDATE instances SET revision = ?2, source = '', text = ?3, updated = ?4,
                            principal = coalesce(?5, principal), heard = ?6
                     WHERE id = ?1",
                    rusqlite::params![
                        id,
                        definition.revision,
                        text,
                        now,
                        definition.principal.as_ref().and_then(|by| serde_json::to_string(by).ok()),
                        definition.heard,
                    ],
                )?;
                said(writing, says)
            })
            .await
            .map(|_| ())
    }

    /// The live children of `parent` whose key is some name with `#` and a
    /// position after it, by that name: `(its id, the blueprint it is of)`.
    pub async fn misnamed_children(&self, parent: &str) -> Result<HashMap<String, Vec<(String, String)>>> {
        let parent = parent.to_string();
        self.db
            .call(move |connection| {
                let mut statement = connection.prepare(
                    "SELECT id, key, blueprint FROM instances
                     WHERE parent = ?1 AND status IN ('starting', 'active') AND key LIKE '%#%'
                     ORDER BY rowid",
                )?;
                let rows = statement.query_map([&parent], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?))
                })?;
                let mut found: HashMap<String, Vec<(String, String)>> = HashMap::new();
                for row in rows {
                    let (id, key, blueprint) = row?;
                    if let Some((name, position)) = key.rsplit_once('#')
                        && !position.is_empty()
                        && position.bytes().all(|byte| byte.is_ascii_digit())
                    {
                        found.entry(name.to_string()).or_default().push((id, blueprint));
                    }
                }
                Ok(found)
            })
            .await
    }

    /// Calls an instance something else under its parent.
    pub async fn rename_instance(&self, id: &str, key: &str, now: i64) -> Result<()> {
        let (id, key) = (id.to_string(), key.to_string());
        self.db
            .call(move |connection| {
                connection.execute(
                    "UPDATE instances SET key = ?2, updated = ?3 WHERE id = ?1",
                    params![id, key, now],
                )
            })
            .await
            .map(|_| ())
    }

    /// Writes down the labels an instance carries from here on.
    pub async fn keep_labels(&self, id: &str, labels: &Labels, now: i64) -> Result<()> {
        let (id, labels) = (id.to_string(), serde_json::to_string(labels).unwrap_or_default());
        self.db
            .call(move |connection| {
                connection.execute(
                    "UPDATE instances SET labels = ?2, updated = ?3 WHERE id = ?1",
                    rusqlite::params![id, labels, now],
                )
            })
            .await
            .map(|_| ())
    }

    /// Each project, with the name of each blueprint that has an instance
    /// alive there, at any depth. Asked of the instances as they are, so
    /// it is as true with ten thousand of them as with ten.
    pub async fn in_use(&self) -> Result<Vec<(String, String)>> {
        self.db
            .call(|connection| {
                let mut statement = connection.prepare(
                    "SELECT DISTINCT alive.project, instances.blueprint
                     FROM alive CROSS JOIN instances ON instances.id = alive.instance
                     WHERE alive.project != ''
                     ORDER BY alive.project, instances.blueprint",
                )?;
                let rows = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
                rows.collect()
            })
            .await
    }
}

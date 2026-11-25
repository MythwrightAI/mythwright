//! What goes with every change of a topic, in the write that keeps it
//! (`studio::factory::Written`): a revision where what it says changed,
//! and what its status came to where that changed. Asked inside the
//! write, of the row as it was there and as it is written, whatever door
//! the change came by (the topic's page, a table's row, `record set`, a
//! CSV read in, a blueprint's step, an agent).

use journal::Writing;
use kernel::Source;
use kernel::topics::{Line, MODEL, PARENT, Place, Tree};
use rusqlite::params;
use serde_json::{Map, Value, json};
use studio::Collection;
use studio::collections::Record;

/// What the keeper writes beside a topic's row.
pub(crate) struct Revised;

/// What every event of a topic says of it.
pub(crate) fn about(id: &str, data: &Map<String, Value>) -> Value {
    let mut said =
        json!({ "record": id, "title": data.get("title").and_then(Value::as_str).unwrap_or_default() });
    match Place::of(data) {
        Some(Place::Project(project)) => said["project"] = json!(project),
        Some(Place::Workspace(workspace)) => said["workspace"] = json!(workspace),
        None => {}
    }
    said
}

fn text<'a>(data: &'a Map<String, Value>, field: &str) -> &'a str {
    data.get(field).and_then(Value::as_str).unwrap_or_default()
}

impl studio::factory::Written for Revised {
    fn keeps(&self, model: &str) -> bool {
        model == MODEL
    }

    fn written(
        &self,
        writing: &mut Writing<'_>,
        collection: &Collection,
        before: Option<&Map<String, Value>>,
        record: &Record,
        source: &Source,
    ) -> rusqlite::Result<()> {
        let data = &record.data;
        let by = source.actor.clone();
        let source = source.clone().taking(collection.reads.as_deref());
        if kernel::topics::revised(before, data) {
            let nth: i64 = writing.transaction().query_row(
                "SELECT coalesce(max(nth), 0) + 1 FROM topic_revisions WHERE topic = ?1",
                [&record.id],
                |row| row.get(0),
            )?;
            writing.transaction().execute(
                "INSERT INTO topic_revisions (topic, nth, title, body, summary, by, through, at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    record.id,
                    nth,
                    text(data, "title"),
                    text(data, "body"),
                    text(data, "summary"),
                    by,
                    source.instance,
                    record.updated
                ],
            )?;
            let mut said = about(&record.id, data);
            said["nth"] = json!(nth);
            if let Some(by) = &by {
                said["by"] = json!(by);
            }
            writing.say_here("topic.revised", source.clone(), said)?;
        }
        if let Some(topic) = kernel::topics::status_said(before, data) {
            let mut said = about(&record.id, data);
            if let Some(by) = &by {
                said["by"] = json!(by);
            }
            if topic == "topic.answered" && !text(data, "answer").is_empty() {
                said["answer"] = json!(text(data, "answer"));
            }
            writing.say_here(topic, source, said)?;
        }
        Ok(())
    }

    fn arranging(&self) -> Option<&'static str> {
        Some(crate::tree::ARRANGING)
    }

    /// Every page under it, at any depth, each before the pages under it:
    /// a page deleted takes them into the Trash, and they come back with it.
    fn goes_with(
        &self,
        connection: &rusqlite::Connection,
        collection: &Collection,
        record: &Record,
    ) -> rusqlite::Result<Vec<String>> {
        let lines = lines(connection, collection)?;
        let tree = Tree::read(&lines);
        Ok(tree.subtree(&record.id).into_iter().skip(1).map(str::to_string).collect())
    }

    /// The first back under the page it was under, where that is there and
    /// of the same workspace, with every page given back of that page's
    /// place; and otherwise a wiki of its own, which is said.
    fn given_back(
        &self,
        connection: &rusqlite::Connection,
        collection: &Collection,
        records: &mut [Record],
    ) -> rusqlite::Result<Option<String>> {
        let Some(top) = records.first() else { return Ok(None) };
        let Some(parent) = top.data.get(PARENT).and_then(Value::as_str).filter(|said| !said.is_empty())
        else {
            return Ok(None);
        };
        let title = text(&top.data, "title").to_string();
        let held: Option<String> = journal::optional(connection.query_row(
            "SELECT data FROM records WHERE id = ?1 AND project = ?2 AND collection = ?3",
            params![parent, collection.project, collection.name],
            |row| row.get(0),
        ))?;
        let above = held.and_then(|held| serde_json::from_str::<Map<String, Value>>(&held).ok());
        let (to, said) = match above {
            None => (
                None,
                format!("The page `{title}` was under is gone, so `{title}` is a wiki of its own now."),
            ),
            Some(above) => {
                let (was, now) = (Place::of(&top.data), Place::of(&above));
                let one_space = match (&was, &now) {
                    (Some(was), Some(now)) if was == now => true,
                    (Some(was), Some(now)) => space(connection, was)
                        .zip(space(connection, now))
                        .is_some_and(|(was, now)| was == now),
                    _ => false,
                };
                if one_space {
                    (now, String::new())
                } else {
                    let said = format!(
                        "The page `{title}` was under, `{}`, is in another workspace now, so `{title}` is a wiki of its \
                         own.",
                        text(&above, "title")
                    );
                    (None, said)
                }
            }
        };
        match to {
            Some(to) => {
                let (workspace, project) = to.cells();
                for record in
                    records.iter_mut().filter(|record| Place::of(&record.data).as_ref() != Some(&to))
                {
                    record.data.insert("workspace".into(), workspace.clone());
                    record.data.insert("project".into(), project.clone());
                }
                Ok(None)
            }
            None => {
                records[0].data.insert(PARENT.into(), Value::Null);
                Ok(Some(said))
            }
        }
    }

    /// Each page's revisions.
    fn forgotten(&self, writing: &mut Writing<'_>, ids: &[String]) -> rusqlite::Result<()> {
        for id in ids {
            writing.transaction().execute("DELETE FROM topic_revisions WHERE topic = ?1", [id])?;
        }
        Ok(())
    }
}

/// Every topic of the table as the tree is read from, inside a piece of
/// work on the database that keeps it.
fn lines(connection: &rusqlite::Connection, collection: &Collection) -> rusqlite::Result<Vec<Line>> {
    let mut asked = connection.prepare(
        "SELECT id, json_extract(data, '$.parent'), position FROM records WHERE project = ?1 AND collection = ?2",
    )?;
    let read = asked.query_map(params![collection.project, collection.name], |row| {
        Ok(Line { id: row.get(0)?, parent: row.get(1)?, position: row.get(2)?, space: None })
    })?;
    read.collect()
}

/// The workspace a place is of: its own, or its project's, as the
/// factory's own table of where projects are says.
fn space(connection: &rusqlite::Connection, place: &Place) -> Option<String> {
    match place {
        Place::Workspace(workspace) => Some(workspace.clone()),
        Place::Project(project) => connection
            .query_row("SELECT workspace FROM workspace_projects WHERE project = ?1", [project], |row| {
                row.get(0)
            })
            .ok(),
    }
}

//! A forum's topics beside the channels: the keeper
//! of the factory's own table of them.
//!
//! A topic is a row of `topics` at the factory's own place for tables
//! (`kernel::place::FACTORY`), declared by `std/topics`, which is applied
//! as the first topic is made by whoever manages the factory. Its body is
//! a document kept in the row, what is said on it is its replies, each of
//! which may answer another, and it is a workspace's or a project's, as a
//! channel is.
//!
//! **Every change of a topic is one write with what goes with it**, by
//! whatever door: the keeper installs into the studio what more the write
//! of a topic's row does ([`revised`]), a revision where what it says
//! changed and what its status came to, so that nothing of it is kept true
//! only by its being written here.
//!
//! **Its own doors** are where it is, who follows it and what answered it
//! (`place`, `follow`, `answered`): where it is and who follows it are
//! written by nothing else, and every other door refuses them, naming
//! these (`studio::models::own_refused`).
//!
//! **Where each is** is brought about as a channel's is ([`Topics::settle`]):
//! one that says a project that is gone, a workspace that is gone, or
//! nothing, is made the own of the usual workspace, or of the workspace
//! a removed one's things went to, each said as the system.
//!
//! Topics are open to whoever may read records for now: who may
//! view one (its access, groups) is still to be built.

pub mod context;
pub mod events;
mod linked;
mod revised;
mod tree;

use std::sync::Arc;

use conductor::{Apply, Engine, RunStatus};
use journal::Journal;
use kernel::topics::{CATEGORIES, OPEN, PARENT, Place, TOPICS, Tree};
use kernel::{Error, Principal, PrincipalKind, Result, Source};
use serde_json::{Map, Value, json};
use studio::collections::Say;
use studio::models::Topic;
use studio::records::Checking;
use studio::{Collection, Record, Studio};

/// The blueprint that declares the topics' tables.
pub const BLUEPRINT: &str = "std/topics";
const FACTORY: &str = kernel::place::FACTORY;

/// What this crate says, among what the factory describes.
pub fn register(registry: &mut conductor::Registry) {
    registry.emits(events::events());
}

/// A topic as it is handed out: its row, and where it is. Its `status` is
/// null where it has none.
pub fn shown(record: &Record) -> Value {
    let mut data = Value::Object(record.data.clone());
    data["status"] = json!(kernel::topics::status_of(&record.data));
    json!({
        "id": record.id,
        "number": record.number,
        "position": record.position,
        "data": data,
        "created": record.created,
        "created_by": record.created_by,
        "updated": record.updated,
    })
}

/// A reply, as the topic's page and `topic show` read it.
fn reply_of(line: &studio::history::Line) -> Value {
    json!({
        "seq": line.seq,
        "parent": line.data.get("parent"),
        "by": line.actor,
        "who": line.who,
        "body": line.data.get("body"),
        "at": line.ts,
    })
}

/// The keeper of the factory's topics.
#[derive(Clone)]
pub struct Topics {
    journal: Journal,
    studio: Studio,
    /// What applies `std/topics` where it is not yet: nothing in a sandbox
    /// or a test that makes its tables itself.
    engine: Option<Engine>,
}

impl Topics {
    /// Opens the keeper's tables and has every write of a topic write
    /// what goes with it, from now on: once, as a node starts.
    pub async fn open(journal: &Journal, studio: &Studio) -> Result<()> {
        journal.db().migrate("topics", kernel::topics::STEPS).await?;
        studio.collections.written_with(Arc::new(revised::Revised));
        Ok(())
    }

    pub fn new(journal: Journal, studio: Studio, engine: Option<Engine>) -> Topics {
        Topics { journal, studio, engine }
    }

    /// The table of topics, applying `std/topics` as `by` where the
    /// factory has none yet: that takes `admin`, and somebody who does
    /// not hold it is told who does.
    async fn table(&self, by: &Principal) -> Result<Collection> {
        if let Ok(table) = self.studio.collections.get(FACTORY, TOPICS).await {
            return Ok(table);
        }
        let Some(engine) = &self.engine else {
            return Err(Error::not_found("this factory has no topics"));
        };
        if !by.can("admin") {
            return Err(Error::denied(
                "this factory has no topics yet: whoever manages it makes the first one, which sets them up",
            ));
        }
        let mut asked = Apply::new(BLUEPRINT, by.clone());
        asked.environment = Some(String::new());
        let applied = engine.apply(asked).await?;
        if let Some(run) = &applied.run {
            let finished = engine.wait_run(run).await?;
            if finished.status != RunStatus::Ok {
                return Err(Error::internal(
                    finished.error.unwrap_or_else(|| format!("applying `{BLUEPRINT}` did not finish well")),
                ));
            }
        }
        self.studio.collections.get(FACTORY, TOPICS).await
    }

    /// The place a topic is to be: the one said, held to being there, or
    /// the usual workspace where nothing is.
    async fn place_said(&self, workspace: Option<&str>, project: Option<&str>) -> Result<Place> {
        match (workspace.filter(|said| !said.is_empty()), project.filter(|said| !said.is_empty())) {
            (Some(_), Some(_)) => Err(Error::invalid(
                "a topic is a workspace's or a project's, and not both: say `workspace` or `project`",
            )),
            (Some(workspace), None) => Ok(Place::Workspace(self.studio.workspaces.get(workspace).await?.id)),
            (None, Some(project)) => Ok(Place::Project(self.studio.projects.get(project).await?.id)),
            (None, None) => match self.studio.workspaces.usual().await? {
                Some(usual) => Ok(Place::Workspace(usual)),
                None => Err(Error::invalid(
                    "this factory has no workspace for a topic to be in yet: its owner registers first",
                )),
            },
        }
    }

    /// Makes a topic, called `title`, with `body`, where it is said to be:
    /// a wiki of its own, after every page there is. It has the `status`
    /// said (`open` or `standing`), and none where none is.
    pub async fn make(
        &self,
        title: &str,
        body: &str,
        (workspace, project): (Option<&str>, Option<&str>),
        status: Option<&str>,
        by: &Principal,
    ) -> Result<Value> {
        by.require("records:write")?;
        let title = kernel::topics::titled(title).map_err(Error::invalid)?;
        let status = kernel::topics::made_status(status).map_err(Error::invalid)?;
        let place = self.place_said(workspace, project).await?;
        let table = self.table(by).await?;
        table.may_change(by)?;
        make_in(self, &table, (title, body, status), place, None, by).await
    }

    /// The topic `named` names: its id, or its title whatever its capitals.
    pub async fn find(&self, named: &str, by: &Principal) -> Result<Record> {
        by.require("records:read")?;
        let table = self.studio.collections.get(FACTORY, TOPICS).await.map_err(|_| none(named))?;
        table.may_read(by)?;
        if named.starts_with(&format!("{FACTORY}-{TOPICS}-")) {
            return self.studio.records.get(named, by).await;
        }
        let key = kernel::topics::title_key(named);
        let called: Vec<Record> = self
            .studio
            .collections
            .records(&table)
            .await?
            .into_iter()
            .filter(|record| kernel::topics::title_key(&Topic::read(&record.data).title) == key)
            .collect();
        match called.as_slice() {
            [one] => Ok(one.clone()),
            [] => Err(none(named)),
            several => Err(Error::conflict(format!(
                "{} topics are called `{named}`: say which by its id ({})",
                several.len(),
                several.iter().map(|record| record.id.as_str()).collect::<Vec<_>>().join(", ")
            ))),
        }
    }

    /// Every topic, as it is handed out, with how many replies each has.
    pub async fn list(&self, by: &Principal) -> Result<Vec<Value>> {
        by.require("records:read")?;
        let Ok(table) = self.studio.collections.get(FACTORY, TOPICS).await else {
            return Ok(Vec::new());
        };
        table.may_read(by)?;
        let replies = self.studio.records.commented(FACTORY, TOPICS, by).await?;
        let records = self.studio.collections.records(&table).await?;
        Ok(records
            .iter()
            .map(|record| {
                let mut topic = shown(record);
                topic["replies"] = json!(replies.get(&record.id).copied().unwrap_or(0));
                topic
            })
            .collect())
    }

    /// A topic whole: its row, its replies, oldest first, each with what
    /// it answers, where it is in its wiki (`path`, the pages above it from
    /// its wiki down, each its id and title) and the pages directly under
    /// it (`pages`, in the order they stand, each with how it stands).
    pub async fn read(&self, named: &str, by: &Principal) -> Result<Value> {
        let record = self.find(named, by).await?;
        let (path, under) = self.whereabouts(&record.id).await?;
        let lines = self.studio.collections.history(&record.id).await?;
        let mut replies: Vec<Value> =
            lines.iter().filter(|line| line.topic == studio::records::COMMENTED).map(reply_of).collect();
        // What was said a moment ago, which the keeper of histories has not
        // written yet: so that whoever replied reads what they said.
        let query = journal::EventQuery {
            after: lines.last().map_or(0, |line| line.seq),
            topics: studio::records::COMMENTED.to_string(),
            limit: 500,
            newest_first: true,
            ..Default::default()
        };
        for event in self.journal.read(query).await?.into_iter().rev() {
            if event.data.get("id").and_then(Value::as_str) == Some(record.id.as_str()) {
                let actor = event.source.actor.clone().unwrap_or_default();
                let who = actor.split_once(':').map_or(actor.as_str(), |(_, name)| name).to_string();
                replies.push(json!({
                    "seq": event.seq, "parent": event.data.get("parent"), "by": event.source.actor,
                    "who": who, "body": event.data.get("body"), "at": event.ts,
                }));
            }
        }
        Ok(json!({ "topic": shown(&record), "replies": replies, "path": path, "pages": under }))
    }

    /// The pages above `id` from its wiki down, and those directly under it,
    /// each its id and title (and of those under it, how it stands): what a
    /// page's page draws, and what an agent asked there is told of it.
    pub async fn whereabouts(&self, id: &str) -> Result<(Vec<Value>, Vec<Value>)> {
        let table = self.studio.collections.get(FACTORY, TOPICS).await?;
        let (pages, lines) = self.pages(&table, false).await?;
        let tree = Tree::read(&lines);
        let page = |id: &str| pages.iter().find(|page| page.id == id);
        let said = |id: &str, more: bool| {
            let mut said = json!({ "id": id, "title": page(id).map(tree::Page::title) });
            if more {
                said["status"] = json!(page(id).and_then(|page| kernel::topics::status_of(&page.cells)));
            }
            said
        };
        let path = tree.path(id);
        let above = path[..path.len().saturating_sub(1)].iter().map(|id| said(id, false)).collect();
        let under = tree.beside(Some(id)).iter().map(|line| said(&line.id, true)).collect();
        Ok((above, under))
    }

    /// Says something in a topic, under the reply it answers or the topic
    /// itself: the number it was said at.
    pub async fn reply(&self, named: &str, body: &str, parent: Option<u64>, by: &Principal) -> Result<u64> {
        let record = self.find(named, by).await?;
        Ok(self.studio.records.reply(&record.id, body, parent, by, None).await?.1)
    }

    /// Writes a topic's own cells and says what is said of that, in one
    /// write, under the topic's turn.
    async fn own(
        &self,
        named: &str,
        by: &Principal,
        change: impl FnOnce(&mut Map<String, Value>) -> Result<Option<(&'static str, Value)>>,
    ) -> Result<Value> {
        by.require("records:write")?;
        let found = self.find(named, by).await?;
        let _turn = self.studio.collections.changing(&found.id).await;
        let (table, mut record) = self.studio.collections.record(&found.id).await?;
        table.may_change(by)?;
        let Some((topic, more)) = change(&mut record.data)? else {
            return Ok(shown(&record));
        };
        let source = Source::actor(by.actor()).taking(table.reads.as_deref());
        let said = |record: &Record| {
            let mut data = revised::about(&record.id, &record.data);
            data["by"] = json!(by.actor());
            for (key, value) in more.as_object().into_iter().flatten() {
                data[key] = value.clone();
            }
            vec![Say::new(topic, (source.clone(), data))]
        };
        let record = self.studio.collections.put_saying(&table, record, said).await?;
        Ok(shown(&record))
    }

    /// Whoever asks follows a topic, or follows it no longer.
    pub async fn follow(&self, named: &str, follows: bool, by: &Principal) -> Result<Value> {
        if by.kind == PrincipalKind::Agent {
            return Err(Error::invalid(
                "an agent follows no topic: it is asked in one by being named in a reply",
            ));
        }
        let actor = by.actor();
        self.own(named, by, move |data| {
            let mut followers = Topic::read(data).followers;
            if followers.contains(&actor) == follows {
                return Ok(None);
            }
            followers.retain(|one| *one != actor);
            if follows {
                followers.push(actor);
            }
            data.insert("followers".into(), json!(followers));
            Ok(Some(("topic.followed", json!({ "follows": follows }))))
        })
        .await
    }

    /// Marks the reply said at `reply` as what answered a topic, which is
    /// then answered, in one write; or, with nothing, opens it again. A
    /// topic with no status is no question, and is refused either way:
    /// marking an answer is not how one is given a status.
    pub async fn answered(&self, named: &str, reply: Option<u64>, by: &Principal) -> Result<Value> {
        by.require("records:write")?;
        let found = self.find(named, by).await?;
        // How it stands is read under its turn, so that a status taken
        // away meanwhile is not given back by marking an answer.
        let _turn = self.studio.collections.changing(&found.id).await;
        let (_, now) = self.studio.collections.record(&found.id).await?;
        let title = Topic::read(&now.data).title;
        if kernel::topics::status_of(&now.data).is_none() {
            return Err(Error::invalid(kernel::topics::not_a_question(brand::CLI, &title, &now.id)));
        }
        let mut fields = Map::new();
        match reply {
            Some(reply) => {
                let lines = self.studio.collections.history(&found.id).await?;
                if !lines.iter().any(|line| line.seq == reply && line.topic == studio::records::COMMENTED) {
                    return Err(Error::not_found(format!(
                        "nothing was said in `{title}` at {reply}: what answered a topic is one of its replies"
                    )));
                }
                fields.insert("status".into(), json!("answered"));
                fields.insert("answer".into(), json!(reply.to_string()));
            }
            None => {
                fields.insert("status".into(), json!(OPEN));
                fields.insert("answer".into(), Value::Null);
            }
        }
        let record = match self.studio.records.checked(&found.id, fields, by).await? {
            Checking::Same(record) => record,
            Checking::Change(checked) => self.studio.records.write(*checked, by, None).await?,
        };
        Ok(shown(&record))
    }

    /// Each revision of a topic, oldest first, each with who made it by
    /// name (`who`), as a record's history names them.
    pub async fn revisions(&self, named: &str, by: &Principal) -> Result<Vec<Value>> {
        let record = self.find(named, by).await?;
        let id = record.id.clone();
        let mut revisions: Vec<Value> = self
            .journal
            .db()
            .call(move |connection| {
                connection
                    .prepare(
                        "SELECT nth, title, body, summary, by, through, at FROM topic_revisions WHERE topic = ?1 ORDER BY nth",
                    )?
                    .query_map([&id], |row| {
                        Ok(json!({
                            "nth": row.get::<_, i64>(0)?, "title": row.get::<_, String>(1)?,
                            "body": row.get::<_, String>(2)?, "summary": row.get::<_, String>(3)?,
                            "by": row.get::<_, Option<String>>(4)?, "through": row.get::<_, Option<String>>(5)?,
                            "at": row.get::<_, i64>(6)?,
                        }))
                    })?
                    .collect()
            })
            .await?;
        for revision in &mut revisions {
            let source = Source {
                actor: revision["by"].as_str().map(str::to_string),
                instance: revision["through"].as_str().map(str::to_string),
                ..Source::default()
            };
            revision["who"] = json!(studio::history::who(&self.studio.identity, &source).await);
        }
        Ok(revisions)
    }
}

/// Makes a topic of `table`, at `place`, under the page `under` where one
/// is said: after every row of the table, so after every page beside it.
async fn make_in(
    topics: &Topics,
    table: &Collection,
    (title, body, status): (String, &str, Option<&str>),
    place: Place,
    under: Option<&str>,
    by: &Principal,
) -> Result<Value> {
    let (workspace, project) = place.cells();
    let mut data = Map::new();
    data.insert("title".into(), json!(title));
    data.insert("body".into(), json!(body));
    data.insert("status".into(), json!(status));
    data.insert("workspace".into(), workspace);
    data.insert("project".into(), project);
    data.insert(PARENT.into(), json!(under));
    let collections = &topics.studio.collections;
    let last = collections.last_position(table).await?;
    let said = |record: &Record| {
        let about = json!({ "id": record.id, "project": FACTORY, "collection": TOPICS });
        vec![Say::new("record.created", (Source::actor(by.actor()).taking(table.reads.as_deref()), about))]
    };
    let record = collections.add_saying(table, data, last + 1.0, by, said).await?;
    Ok(shown(&record))
}

/// Where a topic is, as `topic.placed` says where it was.
fn said_place(place: &Place) -> String {
    match place {
        Place::Workspace(id) => format!("workspace:{id}"),
        Place::Project(id) => format!("project:{id}"),
    }
}

/// What is said of a topic that is not there.
fn none(named: &str) -> Error {
    Error::not_found(format!("there is no topic `{named}`: `{} topic list` says which there are", brand::CLI))
}

/// The keeper's own tables, by the name they are migrated under, with
/// how many steps of them this build knows; and each table, which goes
/// into a copy of the factory as it is.
pub const KNOWS: &[(&str, usize)] = &[("topics", kernel::topics::STEPS.len())];
pub const TABLES_COPIED: &[&str] = &["topic_revisions"];

/// The keeper's own tables, each with how its rows are told apart by
/// project: a revision of a topic is the factory's, whatever project the
/// topic is of, since a topic is a row of the factory's own table.
pub const APART: &[(&str, journal::apart::Told)] = &[("topic_revisions", journal::apart::Told::Factory)];

/// The categories' table is the topics' too: what `std/topics` declares.
pub const TABLES: &[&str] = &[TOPICS, CATEGORIES];

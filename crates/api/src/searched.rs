//! What the search index reads of tables (`search::Rows`), answered from
//! the studio's tables, the factory's topics among them: the index stands
//! below both, so it asks for them back, as it asks the studio for files
//! and cards. And what a search finds by what it is called
//! (`search::Named`), listed as its asker may read it.

use std::collections::HashMap;
use std::sync::Arc;

use kernel::topics::{PARENT, Place, TOPICS};
use kernel::{Principal, Result};
use serde_json::{Map, Value, json};
use studio::board::BOARD;
use studio::search::{Asked, Library, Named, Reply, Row};
use studio::{Collection, Collections, Record, Store, Studio};

use crate::state::App;

const FACTORY: &str = kernel::place::FACTORY;

/// Says what reads the tables for the index of `studio`.
pub fn install(studio: &Studio) {
    // The tables alone, and not the studio: the index is the studio's, and
    // what it holds must not hold the studio in turn.
    studio.search.rows_with(Arc::new(Tables { collections: studio.collections.clone() }));
}

struct Tables {
    collections: Collections,
}

/// Whether a table's rows are found as rows: a board's are found as cards,
/// a library's as the documents they are, and mockups are pages.
fn held(table: &Collection) -> bool {
    table.name != BOARD && !matches!(table.store, Store::Documents | Store::Mockups)
}

fn is_topics(table: &Collection) -> bool {
    table.project == FACTORY && table.name == TOPICS
}

fn said<'a>(data: &'a Map<String, Value>, field: &str) -> &'a str {
    data.get(field).and_then(Value::as_str).unwrap_or_default()
}

/// What a row is called: its `title`, or its first field of text, as the
/// client calls it; its id where that says nothing.
fn title_of(table: &Collection, record: &Record) -> String {
    let field = table
        .fields
        .iter()
        .find(|field| field.name == "title")
        .or_else(|| table.fields.iter().find(|field| field.kind == "text"));
    let title = field.map(|field| said(&record.data, &field.name)).unwrap_or_default();
    if title.trim().is_empty() { record.id.clone() } else { title.to_string() }
}

/// What a row's fields of words say, but what it is called.
fn text_of(table: &Collection, record: &Record, title: &str) -> String {
    let words = table.fields.iter().filter(|field| matches!(field.kind.as_str(), "text" | "document"));
    words
        .map(|field| said(&record.data, &field.name))
        .filter(|text| !text.is_empty() && *text != title)
        .collect::<Vec<_>>()
        .join("\n")
}

/// The titles of the pages above each page, from its wiki down.
struct Wiki {
    pages: HashMap<String, (String, Option<String>)>,
}

impl Wiki {
    fn read(records: &[Record]) -> Wiki {
        let pages = records
            .iter()
            .map(|record| {
                let parent = Some(said(&record.data, PARENT).to_string()).filter(|parent| !parent.is_empty());
                (record.id.clone(), (said(&record.data, "title").to_string(), parent))
            })
            .collect();
        Wiki { pages }
    }

    /// The pages above `id`, nearest last, no deeper than a wiki goes.
    fn above(&self, id: &str) -> Vec<&str> {
        let mut above = Vec::new();
        let mut at = self.pages.get(id).and_then(|(_, parent)| parent.as_deref());
        while let Some(page) = at.filter(|page| above.len() <= kernel::topics::DEPTH && !above.contains(page))
        {
            above.push(page);
            at = self.pages.get(page).and_then(|(_, parent)| parent.as_deref());
        }
        above.reverse();
        above
    }

    fn place(&self, id: &str) -> String {
        let titles: Vec<&str> = self
            .above(id)
            .iter()
            .filter_map(|page| self.pages.get(*page))
            .map(|(title, _)| title.as_str())
            .collect();
        if titles.is_empty() { "Topics".to_string() } else { titles.join(" › ") }
    }
}

impl Tables {
    fn row(&self, table: &Collection, record: &Record, wiki: Option<&Wiki>) -> Row {
        let title = title_of(table, record);
        let (project, place) = match wiki {
            Some(wiki) => {
                let project = Place::of(&record.data).and_then(|place| place.project().map(str::to_string));
                (project, wiki.place(&record.id))
            }
            None => {
                let called = table.shown.title_chosen.clone().or_else(|| table.shown.title.clone());
                let project = Some(table.project.clone()).filter(|project| project != FACTORY);
                (project, called.unwrap_or_else(|| table.name.clone()))
            }
        };
        Row {
            id: record.id.clone(),
            at: table.project.clone(),
            table: table.name.clone(),
            project,
            topic: wiki.is_some(),
            text: text_of(table, record, &title),
            title,
            place,
            replies: Vec::new(),
            updated: record.updated,
        }
    }

    async fn replies(&self, id: &str) -> Result<Vec<Reply>> {
        let lines = self.collections.history(id).await?;
        let said = lines.into_iter().filter(|line| line.topic == studio::records::COMMENTED);
        Ok(said
            .map(|line| Reply { seq: line.seq, body: said_of(&line.data, "body") })
            .filter(|reply| !reply.body.is_empty())
            .collect())
    }
}

fn said_of(data: &Value, field: &str) -> String {
    data.get(field).and_then(Value::as_str).unwrap_or_default().to_string()
}

impl studio::search::Rows for Tables {
    fn libraries(&self) -> Asked<'_, Vec<Library>> {
        Box::pin(async move {
            let tables = self.collections.list(None).await?;
            Ok(tables
                .iter()
                .filter(|table| table.store == Store::Documents)
                .map(|table| Library {
                    project: table.project.clone(),
                    name: table.name.clone(),
                    folder: studio::documents::folder(table).to_string(),
                })
                .collect())
        })
    }

    fn rows(&self) -> Asked<'_, Vec<Row>> {
        Box::pin(async move {
            let mut rows = Vec::new();
            for table in self.collections.list(None).await?.iter().filter(|table| held(table)) {
                let records = self.collections.records(table).await?;
                let wiki = is_topics(table).then(|| Wiki::read(&records));
                for record in &records {
                    let mut row = self.row(table, record, wiki.as_ref());
                    if row.topic {
                        row.replies = self.replies(&record.id).await?;
                    }
                    rows.push(row);
                }
            }
            Ok(rows)
        })
    }

    fn row<'a>(&'a self, id: &'a str) -> Asked<'a, Vec<Row>> {
        Box::pin(async move {
            let (table, record) = self.collections.record(id).await?;
            if !held(&table) {
                return Ok(Vec::new());
            }
            if !is_topics(&table) {
                return Ok(vec![self.row(&table, &record, None)]);
            }
            // A page, and every page under it: what each is under is said of each.
            let records = self.collections.records(&table).await?;
            let wiki = Wiki::read(&records);
            let under = records.iter().filter(|other| other.id != id && wiki.above(&other.id).contains(&id));
            let mut rows = vec![self.row(&table, &record, Some(&wiki))];
            rows.extend(under.map(|page| self.row(&table, page, Some(&wiki))));
            Ok(rows)
        })
    }
}

/// Everything found by what it is called that `who` may read: the
/// projects (`projects:read`), the channels they may read, the people
/// (`channels:read`, as `/people` is read), the agents (`agents:read`),
/// and the blueprints applied and the runs (`blueprints:read`).
/// Narrowed to one kind and one project where asked.
pub async fn named(
    app: &App,
    who: &Principal,
    kind: Option<&str>,
    project: Option<&str>,
    text: &str,
) -> Result<Vec<Named>> {
    let asks = |one: &str| kind.is_none_or(|kind| kind == one);
    let within = |of: Option<&str>| project.is_none_or(|project| of == Some(project));
    let mut named = Vec::new();
    if asks("project") && who.can("projects:read") {
        for one in app.studio.projects.list().await? {
            if within(Some(&one.id)) {
                let link = json!({ "id": one.id });
                named.push(Named {
                    reference: format!("project:{}", one.id),
                    kind: "project",
                    project: Some(one.id.clone()),
                    title: one.name,
                    also: vec![one.id],
                    place: None,
                    link,
                });
            }
        }
    }
    if asks("channel") && who.can("channels:read") {
        for one in app.studio.channels.readable(who).await? {
            if one.conversation || !within(one.project.as_deref()) {
                continue;
            }
            // One kept to its members is called by its title; its name is its id.
            let title = one.title.unwrap_or_else(|| format!("#{}", one.name));
            let link = json!({ "channel": one.id });
            named.push(Named {
                reference: format!("channel:{}", one.id),
                kind: "channel",
                project: one.project,
                title,
                also: vec![],
                place: None,
                link,
            });
        }
    }
    if asks("person") && who.can("channels:read") && project.is_none() {
        for one in app.studio.identity.accounts().await? {
            let actor = one.principal().actor();
            let link = json!({ "actor": actor, "name": one.name });
            named.push(Named {
                reference: actor,
                kind: "person",
                project: None,
                title: one.name.clone(),
                also: vec![handle(&one.name)],
                place: None,
                link,
            });
        }
    }
    if asks("agent") && who.can("agents:read") && project.is_none() {
        for slot in app.agents.slots().await? {
            let actor = format!("agent:{}", slot.name);
            let link = json!({ "actor": actor, "name": slot.name });
            named.push(Named {
                reference: actor,
                kind: "agent",
                project: None,
                title: slot.name.clone(),
                also: vec![format!("@{}", slot.name)],
                place: None,
                link,
            });
        }
    }
    if asks("run") && who.can("blueprints:read") {
        named.extend(runs(app, project, text).await?);
    }
    Ok(named)
}

/// What somebody is mentioned as: their name in small letters, joined by dashes.
fn handle(name: &str) -> String {
    let small = name.to_lowercase();
    let words = small.split(|c: char| !c.is_ascii_alphanumeric() && c != '_').filter(|word| !word.is_empty());
    format!("@{}", words.collect::<Vec<_>>().join("-"))
}

/// Blueprints applied (each by its blueprint and the name it was applied
/// under), and runs: by their id, among the most recent and the one it
/// whole names.
async fn runs(app: &App, project: Option<&str>, text: &str) -> Result<Vec<Named>> {
    let mut named = Vec::new();
    let mut called: HashMap<String, String> = HashMap::new();
    let filter = conductor::InstanceFilter {
        roots_only: true,
        project: project.map(str::to_string),
        limit: 500,
        ..Default::default()
    };
    for one in app.engine.instances(filter).await? {
        let title = if one.key.is_empty() || one.key == one.blueprint {
            one.blueprint.clone()
        } else {
            format!("{} · {}", one.blueprint, one.key)
        };
        let link = json!({ "instance": one.id });
        called.insert(one.id.clone(), title.clone());
        named.push(Named {
            reference: format!("instance:{}", one.id),
            kind: "run",
            project: one.project,
            title,
            also: vec![one.id],
            place: Some("Applied".to_string()),
            link,
        });
    }
    let mut runs = app.engine.recent_runs(200, project.map(str::to_string)).await?;
    // One named whole, by its id, which may be older than those.
    let whole = text.trim();
    let an_id = whole
        .strip_prefix("run_")
        .is_some_and(|rest| rest.len() == 16 && rest.chars().all(char::is_alphanumeric));
    if an_id && !runs.iter().any(|run| run.id == whole) {
        runs.extend(app.engine.run(whole).await.ok());
    }
    for run in runs {
        let link = json!({ "run": run.id });
        let of = called.get(&run.instance).unwrap_or(&run.instance);
        let place = Some(format!("Run of {of}"));
        named.push(Named {
            reference: format!("run:{}", run.id),
            kind: "run",
            project: None,
            title: run.id.clone(),
            also: vec![],
            place,
            link,
        });
    }
    Ok(named)
}

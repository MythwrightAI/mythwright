//! Search across everything the factory knows: files, blueprints,
//! messages, cards, every table's rows, the topics and what was said on
//! them, and the documents of every library, in an index; and projects,
//! channels, people, agents and runs by what they are called ([`named`]).
//!
//! The index lives in its own database file with its own thread. That is the
//! whole answer to "indexing must not block anyone": whatever the indexer is
//! doing, a write to the journal or a request from the client never waits
//! behind it.
//!
//! It is kept current from events (a file written, a message posted, a card
//! moved), not by rescanning. A full pass runs at start-up to pick up what
//! changed while the daemon was down; it skips anything whose stamp has not
//! changed, reports progress as it goes, and yields between small batches.
//!
//! It came out of `studio` whole when that crate was at its ceiling, and
//! is offered there under the names it always had (`studio::search`,
//! `studio::Search`). What it reads of the projects' files and of the
//! board, which are the studio's and stand above it, it asks back of
//! whoever opens it ([`Indexed`], which `studio` answers) as plain values;
//! the channels are `talk`'s, below it. What it reads of tables, the
//! topics among them, it asks of whoever keeps them ([`Rows`], which the
//! node answers from the studio's tables). What it says is described with
//! the studio's events, where it always was, and its tables are migrated
//! under the name they always had (`search`), in a database of its own.

use std::collections::{BTreeMap, HashMap};
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use journal::{Db, Journal, Places};
use kernel::walked::Entry;
use kernel::{Event, Principal, Result, Source};
use rusqlite::params;
use serde::Serialize;
use serde_json::{Value, json};

use talk::{Channels, Message};

pub mod named;
pub mod rows;

pub use named::{Named, found};
pub use rows::{Library, Reply, Row, Rows};

const SCHEMA: &[&str] = &["
    CREATE VIRTUAL TABLE search USING fts5(
        title, body,
        kind UNINDEXED, project UNINDEXED, ref UNINDEXED, link UNINDEXED,
        tokenize = 'unicode61 remove_diacritics 2'
    );

    CREATE TABLE docs (
        ref TEXT PRIMARY KEY,
        stamp TEXT NOT NULL,
        fts INTEGER NOT NULL
    ) WITHOUT ROWID;
"];

/// Documents written per transaction. Small, so the index thread is never
/// busy for long and a query never waits behind a big batch.
const BATCH: usize = 25;
/// Files larger than this are not indexed.
const MAX_INDEXED_BYTES: u64 = 512 * 1024;
const TEXT_EXTENSIONS: &[&str] = &[
    "md", "txt", "rs", "toml", "json", "yaml", "yml", "ts", "tsx", "js", "jsx", "mjs", "css", "html", "py",
    "go", "sh", "sql", "c", "h", "cpp", "hpp", "java", "cs", "rb", "php", "swift", "kt", "tf", "proto",
    "graphql", "xml", "ini", "cfg",
];
const TEXT_NAMES: &[&str] = &["README", "LICENSE", "Makefile", "Dockerfile", "Justfile", "Procfile"];
/// The events the index is kept current from.
const TOPICS: &str = "project.created, project.removed, project.rescanned, project.file.changed, channel.message, channel.removed, channel.placed, channel.renamed, channel.created, board.item.*, record.created, record.updated, record.removed, record.commented, collection.ensured, topic.moved, topic.placed";
/// Why the index reads every channel: it is the daemon's own, handed to
/// nobody as it is, and what a search finds in it is held to what its
/// asker reads when it is answered ([`Search::readable`]).
const INDEXING: &str = "the search index is made from everything said, and answered by what its asker reads";
/// Marks the start and end of a matched word in a snippet. Private-use
/// characters, so they cannot collide with anything in the text; the client
/// turns them into highlights.
pub const MARK_START: char = '\u{e000}';
pub const MARK_END: char = '\u{e001}';

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Hit {
    /// `file:<project>/<path>`, `msg:<id>`, `item:<id>`, `row:<id>` (and
    /// `row:<id>/r<seq>` for something said on a topic), or what a thing
    /// found by its name is known by.
    pub reference: String,
    /// `file`, `blueprint`, `message`, `card`, `document`, `record`,
    /// `topic`, or of what is found by its name ([`Named`]): `project`,
    /// `channel`, `person`, `agent` or `run`.
    pub kind: String,
    pub project: Option<String>,
    pub title: String,
    /// A passage with the matched words between [`MARK_START`] and [`MARK_END`].
    pub snippet: String,
    /// Where it is, in words, where that says more than its project: a
    /// row's table, a page's wiki, a document's library and folders.
    pub place: Option<String>,
    /// What the client needs to open it: `{path}` (of a document with the
    /// `line` of the heading nearest what matched), `{channel, id}`, `{id}`,
    /// or of a row `{id, at, table}` (and `reply` of something said on a
    /// topic, by the number it was said at).
    pub link: Value,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Progress {
    /// `idle`, `scanning` or `indexing`.
    pub phase: &'static str,
    pub done: usize,
    pub total: usize,
    /// Documents in the index.
    pub docs: usize,
}

/// What is asked of whoever keeps something the index reads.
pub type Asked<'a, T> = Pin<Box<dyn Future<Output = Result<T>> + Send + 'a>>;

/// A card as the index takes it in.
#[derive(Clone, Debug)]
pub struct Card {
    /// `<project>-<number>`.
    pub id: String,
    pub project: String,
    pub title: String,
    /// The column it is in.
    pub status: String,
    pub body: String,
    /// When it was last changed: what tells the index it is to be read again.
    pub updated: i64,
}

/// What the index reads of the projects and of the board, asked back of
/// whoever opens it, which keeps both (`studio`).
pub trait Indexed: Send + Sync {
    /// Every project there is, by its id.
    fn projects(&self) -> Asked<'_, Vec<String>>;
    /// The files of a project, its folder's and its own blueprints where
    /// the factory keeps them, each by its path with when it was last
    /// changed and how large it is.
    fn files<'a>(&'a self, project: &'a str) -> Asked<'a, BTreeMap<String, (i64, u64)>>;
    /// A file of a project as text.
    fn read<'a>(&'a self, project: &'a str, path: &'a str) -> Asked<'a, String>;
    /// What a file of a project is as it stands.
    fn stat<'a>(&'a self, project: &'a str, path: &'a str) -> Asked<'a, Entry>;
    /// Every card of every board.
    fn cards(&self) -> Asked<'_, Vec<Card>>;
    /// One card, by its id.
    fn card<'a>(&'a self, id: &'a str) -> Asked<'a, Card>;
}

#[derive(Clone)]
pub struct Search {
    db: Db,
    journal: Journal,
    channels: Channels,
    kept: Arc<dyn Indexed>,
    /// What reads the tables, where the node has said what does.
    rows: Arc<Mutex<Option<Arc<dyn Rows>>>>,
    /// The libraries as they were last read: which files are documents.
    libraries: Arc<Mutex<Vec<Library>>>,
    progress: Arc<Mutex<Progress>>,
}

/// Something to put in the index.
struct Doc {
    reference: String,
    /// Changes when the content does; a doc whose stamp is unchanged is skipped.
    stamp: String,
    kind: &'static str,
    project: Option<String>,
    title: String,
    body: String,
    link: Value,
}

/// A document that may need indexing, before its content has been read.
enum Pending {
    File { project: String, path: String, stamp: String },
    Ready(Doc),
}

impl Pending {
    fn reference(&self) -> String {
        match self {
            Pending::File { project, path, .. } => file_ref(project, path),
            Pending::Ready(doc) => doc.reference.clone(),
        }
    }

    fn stamp(&self) -> &str {
        match self {
            Pending::File { stamp, .. } => stamp,
            Pending::Ready(doc) => &doc.stamp,
        }
    }
}

fn file_ref(project: &str, path: &str) -> String {
    format!("file:{project}/{path}")
}

fn is_text(path: &str) -> bool {
    let name = path.rsplit('/').next().unwrap_or(path);
    if brand::is_blueprint_path(name) {
        return true;
    }
    match name.rsplit_once('.') {
        Some((_, extension)) => TEXT_EXTENSIONS.contains(&extension.to_ascii_lowercase().as_str()),
        None => TEXT_NAMES.contains(&name),
    }
}

fn message_doc(message: &Message, channel_name: &str, project: Option<String>) -> Doc {
    Doc {
        reference: format!("msg:{}", message.id),
        stamp: message.created.to_string(),
        kind: "message",
        project,
        title: format!("#{channel_name} · {}", message.author_name),
        body: message.body.clone(),
        link: json!({ "channel": message.channel, "id": message.id }),
    }
}

fn item_doc(item: &Card) -> Doc {
    Doc {
        reference: format!("item:{}", item.id),
        stamp: item.updated.to_string(),
        kind: "card",
        project: Some(item.project.clone()),
        title: format!("{} · {}", item.title, item.status),
        body: item.body.clone(),
        link: json!({ "id": item.id }),
    }
}

/// Turns what a person typed into a query the index accepts: each word must
/// appear, and the last may be a prefix, so results narrow as they type.
fn match_expression(text: &str) -> Option<String> {
    let words: Vec<&str> =
        text.split(|c: char| !c.is_alphanumeric() && c != '_').filter(|word| !word.is_empty()).collect();
    let last = words.len().checked_sub(1)?;
    Some(
        words
            .iter()
            .enumerate()
            .map(|(index, word)| if index == last { format!("\"{word}\"*") } else { format!("\"{word}\"") })
            .collect::<Vec<_>>()
            .join(" "),
    )
}

impl Search {
    pub async fn open(
        db: Db,
        journal: Journal,
        channels: Channels,
        kept: Arc<dyn Indexed>,
    ) -> Result<Search> {
        db.migrate("search", SCHEMA).await?;
        let docs: i64 = db.call(|c| c.query_row("SELECT count(*) FROM docs", [], |row| row.get(0))).await?;
        let progress = Progress { phase: "idle", docs: docs as usize, ..Progress::default() };
        Ok(Search {
            db,
            journal,
            channels,
            kept,
            rows: Arc::default(),
            libraries: Arc::default(),
            progress: Arc::new(Mutex::new(progress)),
        })
    }

    /// Says what reads the tables for the index, before it is started:
    /// until then it holds no rows and no file as a document.
    pub fn rows_with(&self, rows: Arc<dyn Rows>) {
        *self.rows.lock().expect("search lock") = Some(rows);
    }

    fn rows(&self) -> Option<Arc<dyn Rows>> {
        self.rows.lock().expect("search lock").clone()
    }

    /// Reads which folders are libraries again.
    async fn read_libraries(&self) {
        let Some(rows) = self.rows() else { return };
        if let Ok(libraries) = rows.libraries().await {
            *self.libraries.lock().expect("search lock") = libraries;
        }
    }

    /// The library a file of a project is a document of, if any.
    fn library_of(&self, project: &str, path: &str) -> Option<Library> {
        self.libraries.lock().expect("search lock").iter().find(|one| one.holds(project, path)).cloned()
    }

    /// What tells the index a file is to be read again: when it was last
    /// changed and how large it is, and whether it is a document, which
    /// changes as a folder comes to be a library or stops being one.
    fn file_stamp(&self, project: &str, path: &str, modified: i64, size: u64) -> String {
        match self.library_of(project, path) {
            Some(_) => format!("doc:{modified}:{size}"),
            None => format!("{modified}:{size}"),
        }
    }

    pub fn progress(&self) -> Progress {
        self.progress.lock().expect("search lock").clone()
    }

    /// How the index stands, with a fresh count of what is in it: documents
    /// are added as things happen, not only by a full pass.
    pub async fn status(&self) -> Progress {
        let docs: Option<i64> =
            self.db.call(|c| c.query_row("SELECT count(*) FROM docs", [], |row| row.get(0))).await.ok();
        let mut progress = self.progress();
        if let Some(docs) = docs {
            progress.docs = docs as usize;
        }
        progress
    }

    fn set_progress(&self, change: impl FnOnce(&mut Progress)) {
        let progress = {
            let mut progress = self.progress.lock().expect("search lock");
            change(&mut progress);
            progress.clone()
        };
        // Progress is for whoever is watching; it is not kept.
        let data = serde_json::to_value(&progress).unwrap_or(Value::Null);
        self.journal.emit_ephemeral("search.progress", Source::actor("system"), data);
    }

    /// What of `hits` whoever asks may be shown, as far as channels go:
    /// something said in a channel kept to its members is found by them
    /// and by nobody else. What reading channels takes at all, and what a
    /// card or a file takes, is whoever answers a search's to ask beside
    /// this.
    pub async fn readable(&self, hits: Vec<Hit>, by: &Principal) -> Result<Vec<Hit>> {
        let said = |hit: &Hit| hit.kind == "message";
        if !hits.iter().any(said) {
            return Ok(hits);
        }
        let kept = self.channels.kept_from(Some(by)).await?;
        let from =
            |hit: &Hit| hit.link.get("channel").and_then(Value::as_str).is_some_and(|of| kept.contains(of));
        Ok(hits.into_iter().filter(|hit| !(said(hit) && from(hit))).collect())
    }

    /// Searches. `kind` and `project` narrow the results when given. What
    /// is found is everything the index holds: whoever hands it to
    /// somebody holds it to what they read first ([`Self::readable`]).
    pub async fn query(
        &self,
        text: &str,
        kind: Option<&str>,
        project: Option<&str>,
        limit: usize,
    ) -> Result<Vec<Hit>> {
        let Some(expression) = match_expression(text) else { return Ok(Vec::new()) };
        let (kind, project, limit) =
            (kind.map(str::to_string), project.map(str::to_string), limit.clamp(1, 200));
        self.db
            .call(move |connection| {
                // A match in a title counts for more than one in the body.
                // A document's whole text too, to say which heading what
                // matched is under.
                let mut statement = connection.prepare(&format!(
                    "SELECT ref, kind, project, title, snippet(search, 1, '{MARK_START}', '{MARK_END}', ' … ', 14), link,
                            CASE WHEN kind = 'document' THEN body END
                     FROM search
                     WHERE search MATCH ?1 AND (?2 IS NULL OR kind = ?2) AND (?3 IS NULL OR project = ?3)
                     ORDER BY bm25(search, 6.0, 1.0)
                     LIMIT {limit}"
                ))?;
                let rows = statement.query_map(params![expression, kind, project], |row| {
                    let mut link: Value = serde_json::from_str(&row.get::<_, String>(5)?).unwrap_or(Value::Null);
                    let snippet: String = row.get(4)?;
                    if let Some(text) = row.get::<_, Option<String>>(6)?
                        && let Some((line, heading)) =
                            kernel::sought::heading_at(&text, &snippet, (MARK_START, MARK_END))
                    {
                        link["line"] = json!(line);
                        link["heading"] = json!(heading);
                    }
                    let place = link.get("where").and_then(Value::as_str).map(str::to_string);
                    Ok(Hit {
                        reference: row.get(0)?,
                        kind: row.get(1)?,
                        project: row.get(2)?,
                        title: row.get(3)?,
                        snippet,
                        place,
                        link,
                    })
                })?;
                rows.collect()
            })
            .await
    }

    async fn stamps(&self) -> Result<HashMap<String, String>> {
        self.db
            .call(|connection| {
                let mut statement = connection.prepare("SELECT ref, stamp FROM docs")?;
                let rows = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
                rows.collect()
            })
            .await
    }

    async fn store(&self, docs: Vec<Doc>) -> Result<()> {
        if docs.is_empty() {
            return Ok(());
        }
        self.db
            .call(move |connection| {
                let transaction = journal::whole(connection)?;
                for doc in docs {
                    transaction.execute(
                        "DELETE FROM search WHERE rowid = (SELECT fts FROM docs WHERE ref = ?1)",
                        [&doc.reference],
                    )?;
                    transaction.execute(
                        "INSERT INTO search (title, body, kind, project, ref, link) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                        params![doc.title, doc.body, doc.kind, doc.project, doc.reference, doc.link.to_string()],
                    )?;
                    transaction.execute(
                        "INSERT OR REPLACE INTO docs (ref, stamp, fts) VALUES (?1, ?2, ?3)",
                        params![doc.reference, doc.stamp, transaction.last_insert_rowid()],
                    )?;
                }
                transaction.commit()
            })
            .await
    }

    /// Removes documents by reference, or every one starting with a prefix
    /// ending in `*`.
    async fn forget(&self, references: Vec<String>) -> Result<()> {
        if references.is_empty() {
            return Ok(());
        }
        self.db
            .call(move |connection| {
                let transaction = journal::whole(connection)?;
                for reference in references {
                    let (clause, value) = match reference.strip_suffix('*') {
                        Some(prefix) => ("ref >= ?1 AND ref < ?1 || char(1114111)", prefix.to_string()),
                        None => ("ref = ?1", reference),
                    };
                    transaction.execute(
                        &format!("DELETE FROM search WHERE rowid IN (SELECT fts FROM docs WHERE {clause})"),
                        [&value],
                    )?;
                    transaction.execute(&format!("DELETE FROM docs WHERE {clause}"), [&value])?;
                }
                transaction.commit()
            })
            .await
    }

    /// Removes every message of a channel that has been taken away. A
    /// message is kept with where it leads, which begins with its channel.
    async fn forget_said_in(&self, channel: &str) -> Result<()> {
        let leads = json!({ "channel": channel, "id": "" }).to_string();
        let leads = leads.trim_end_matches("\"\"}").to_string();
        let said: Vec<String> = self
            .db
            .call(move |connection| {
                let mut statement = connection
                    .prepare("SELECT ref FROM search WHERE kind = 'message' AND instr(link, ?1) = 1")?;
                let rows = statement.query_map([&leads], |row| row.get(0))?;
                rows.collect()
            })
            .await?;
        self.forget(said).await
    }

    async fn load(&self, pending: Pending) -> Option<Doc> {
        match pending {
            Pending::Ready(doc) => Some(doc),
            Pending::File { project, path, stamp } => {
                // Unreadable, binary or too large: simply not indexed.
                let body = self.kept.read(&project, &path).await.ok()?;
                if let Some(library) = self.library_of(&project, &path) {
                    return Some(rows::document_doc(&library, &path, stamp, body));
                }
                let blueprint = brand::is_blueprint_path(&path);
                Some(Doc {
                    reference: file_ref(&project, &path),
                    stamp,
                    kind: if blueprint { "blueprint" } else { "file" },
                    title: path.clone(),
                    body,
                    link: json!({ "path": path }),
                    project: Some(project),
                })
            }
        }
    }

    async fn project_files(&self, project: &str) -> Vec<Pending> {
        // Its folder's, and its own blueprints where the factory keeps them.
        let Ok(stamps) = self.kept.files(project).await else { return Vec::new() };
        let to_index = |(path, (modified, size)): (String, (i64, u64))| {
            let project = project.to_string();
            (size <= MAX_INDEXED_BYTES && is_text(&path)).then(|| Pending::File {
                stamp: self.file_stamp(&project, &path, modified, size),
                project,
                path,
            })
        };
        stamps.into_iter().filter_map(to_index).collect()
    }

    /// Brings `pending` into the index, a batch at a time, skipping what has
    /// not changed. Reports progress as it goes.
    async fn index(&self, pending: Vec<Pending>, known: &HashMap<String, String>) -> Result<usize> {
        let mut written = 0;
        let mut batch: Vec<Doc> = Vec::with_capacity(BATCH);
        for (done, item) in pending.into_iter().enumerate() {
            let unchanged = known.get(&item.reference()).is_some_and(|stamp| stamp == item.stamp());
            if !unchanged && let Some(doc) = self.load(item).await {
                batch.push(doc);
            }
            if batch.len() == BATCH {
                written += batch.len();
                self.store(std::mem::take(&mut batch)).await?;
                self.set_progress(|progress| progress.done = done + 1);
                // Let anything else that is waiting have a turn.
                tokio::task::yield_now().await;
            }
        }
        written += batch.len();
        self.store(batch).await?;
        Ok(written)
    }

    /// Indexes everything, skipping what is already current, and drops what
    /// no longer exists. Returns how many documents were written.
    pub async fn reindex(&self) -> Result<usize> {
        let started = self.journal.now_ms();
        self.set_progress(|progress| {
            *progress = Progress { phase: "scanning", docs: progress.docs, ..Progress::default() }
        });

        self.read_libraries().await;
        let mut pending: Vec<Pending> = Vec::new();
        for project in self.kept.projects().await? {
            pending.extend(self.project_files(&project).await);
        }
        let own = Channels::daemon(INDEXING);
        let channels: HashMap<String, (String, Option<String>)> = self
            .channels
            .list(&own)
            .await?
            .into_iter()
            .map(|channel| (channel.id, (channel.name, channel.project)))
            .collect();
        let mut after = 0;
        loop {
            let page = self.channels.all_messages(&own, after, 500).await?;
            let Some((last, _)) = page.last() else { break };
            after = *last;
            for (_, message) in &page {
                let (name, project) = channels.get(&message.channel).cloned().unwrap_or_default();
                pending.push(Pending::Ready(message_doc(message, &name, project)));
            }
        }
        pending.extend(self.kept.cards().await?.iter().map(|item| Pending::Ready(item_doc(item))));
        if let Some(rows) = self.rows() {
            for row in rows.rows().await? {
                pending.extend(row.replies.iter().map(|reply| Pending::Ready(rows::reply_doc(&row, reply))));
                pending.push(Pending::Ready(rows::row_doc(&row)));
            }
        }

        let known = self.stamps().await?;
        let current: std::collections::HashSet<String> = pending.iter().map(Pending::reference).collect();
        let stale: Vec<String> =
            known.keys().filter(|reference| !current.contains(*reference)).cloned().collect();
        let total = pending.len();
        self.set_progress(|progress| {
            *progress = Progress { phase: "indexing", done: 0, total, docs: progress.docs }
        });

        let written = self.index(pending, &known).await?;
        self.forget(stale).await?;
        let docs: i64 =
            self.db.call(|c| c.query_row("SELECT count(*) FROM docs", [], |row| row.get(0))).await?;
        self.set_progress(|progress| {
            *progress = Progress { phase: "idle", done: total, total, docs: docs as usize }
        });
        let data = json!({ "docs": docs, "written": written, "took_ms": self.journal.now_ms() - started });
        let _ = self.journal.emit("search.indexed", Source::actor("system"), data).await;
        Ok(written)
    }

    /// What was said in a channel is found under the project the channel
    /// is of. Said to be of another, or of none, each thing said there is
    /// written again as that: or a search within the project it was of
    /// would go on finding it, and one within the project it now is of
    /// would not.
    async fn place_said_in(&self, channel: &str) -> Result<()> {
        let own = Channels::daemon(INDEXING);
        let Ok(channel) = self.channels.get(&own, channel).await else { return Ok(()) };
        let mut after = 0;
        loop {
            let page = self.channels.all_messages(&own, after, 500).await?;
            let Some((last, _)) = page.last() else { break };
            after = *last;
            let said = page.iter().filter(|(_, message)| message.channel == channel.id);
            let docs = said.map(|(_, message)| message_doc(message, &channel.name, channel.project.clone()));
            self.store(docs.collect()).await?;
        }
        Ok(())
    }

    /// Applies one event to the index.
    async fn apply(&self, event: &Event) -> Result<()> {
        let text = |key: &str| event.data[key].as_str().unwrap_or_default().to_string();
        match event.topic.as_str() {
            "project.file.changed" => {
                let (project, path) = (text("project"), text("path"));
                if text("change") == "removed" {
                    return self.forget(vec![file_ref(&project, &path)]).await;
                }
                if !is_text(&path) {
                    return Ok(());
                }
                // The same stamp a full pass would give it, so that pass
                // finds it already current.
                let Ok(entry) = self.kept.stat(&project, &path).await else { return Ok(()) };
                if entry.size > MAX_INDEXED_BYTES {
                    return self.forget(vec![file_ref(&project, &path)]).await;
                }
                let stamp = self.file_stamp(&project, &path, entry.modified, entry.size);
                self.index(vec![Pending::File { project, path, stamp }], &HashMap::new()).await.map(|_| ())
            }
            "project.created" | "project.rescanned" => {
                let pending = self.project_files(&text("project")).await;
                let known = self.stamps().await?;
                self.index(pending, &known).await.map(|_| ())
            }
            "project.removed" => self.forget(vec![format!("file:{}/*", text("project"))]).await,
            "channel.message" => {
                let own = Channels::daemon(INDEXING);
                let Ok(message) = self.channels.message(&own, &text("id")).await else { return Ok(()) };
                let project = event.data["project"].as_str().map(str::to_string);
                self.store(vec![message_doc(&message, &text("channel_name"), project)]).await
            }
            "channel.removed" => self.forget_said_in(&text("channel")).await,
            // Each thing said there is written again under the project the
            // channel is of now, or under what it is called now.
            "channel.placed" | "channel.renamed" => self.place_said_in(&text("channel")).await,
            // Given back from the Trash, with everything said there.
            "channel.created" if event.data["restored"] == true => self.place_said_in(&text("channel")).await,
            "board.item.removed" => self.forget(vec![format!("item:{}", text("id"))]).await,
            topic if topic.starts_with("board.item.") => match self.kept.card(&text("id")).await {
                Ok(item) => self.store(vec![item_doc(&item)]).await,
                Err(_) => Ok(()),
            },
            "record.removed" => {
                let row = rows::row_ref(&text("id"));
                self.forget(vec![format!("{row}/*"), row]).await
            }
            "record.created" | "record.updated" => self.read_row(&text("id")).await.map(|_| ()),
            "topic.moved" | "topic.placed" => self.read_row(&text("record")).await.map(|_| ()),
            "record.commented" => {
                // Said on a topic: found as what was said there, from what
                // the event says, as the topic's history may not have it yet.
                let Some(row) = self.read_row(&text("id")).await?.into_iter().next() else { return Ok(()) };
                if !row.topic {
                    return Ok(());
                }
                let reply = Reply { seq: event.seq, body: text("body") };
                self.store(vec![rows::reply_doc(&row, &reply)]).await
            }
            // A folder may have come to be a library, or stopped being one:
            // its files are documents now, or files again.
            "collection.ensured" => {
                self.read_libraries().await;
                let pending = self.project_files(&text("project")).await;
                let known = self.stamps().await?;
                self.index(pending, &known).await.map(|_| ())
            }
            _ => Ok(()),
        }
    }

    /// Writes a row again as it is now, with the pages under it, and gives
    /// them back: nothing where there is nobody to read it or it is none
    /// the index holds.
    async fn read_row(&self, id: &str) -> Result<Vec<Row>> {
        let Some(rows) = self.rows() else { return Ok(Vec::new()) };
        let Ok(read) = rows.row(id).await else { return Ok(Vec::new()) };
        self.store(read.iter().map(rows::row_doc).collect()).await?;
        Ok(read)
    }

    /// Applies every event recorded since the index was last brought up to
    /// date. After this returns, a search sees everything that had happened
    /// when it was called.
    pub async fn catch_up(&self) -> Result<()> {
        // Where it has read to is a place in each journal: a number
        // alone, as a program from before kept, is one in the factory's.
        let kept = self.journal.kv_get("search", "cursor").await?;
        let mut cursor = kept.and_then(|kept| Places::from_kept(&kept)).unwrap_or_default();
        loop {
            let (events, read_to) = self.journal.read_after(&cursor, TOPICS, 200).await?;
            if events.is_empty() {
                break;
            }
            cursor = read_to;
            for event in &events {
                self.apply(event).await?;
            }
            self.journal.kv_set("search", "cursor", &cursor.kept()).await?;
        }
        let docs: i64 =
            self.db.call(|c| c.query_row("SELECT count(*) FROM docs", [], |row| row.get(0))).await?;
        self.set_progress(|progress| progress.docs = docs as usize);
        Ok(())
    }

    /// Runs the indexer for as long as the returned task lives: one full
    /// pass, then every change as it happens.
    pub fn start(&self) -> tokio::task::JoinHandle<()> {
        let search = self.clone();
        tokio::spawn(async move {
            // Everything recorded so far is covered by the full pass.
            let start = search.journal.now();
            let _ = search.reindex().await;
            let _ = search.journal.kv_set("search", "cursor", &start.kept()).await;
            let mut follow = search.journal.follow_every(start, TOPICS);
            while let Ok(event) = follow.next().await {
                if event.is_durable() {
                    let _ = search.apply(&event).await;
                    let _ = search.journal.kv_set("search", "cursor", &follow.places().kept()).await;
                }
            }
        })
    }
}

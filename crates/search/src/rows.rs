//! What the index holds of tables, asked of whoever keeps them ([`Rows`],
//! which the node answers from the studio's tables): every table's
//! rows by what their text fields say, the factory's topics among them
//! with what was said on each, and the documents of every library, which
//! are files of a project's folder and found as documents.
//!
//! Whoever may read what is found is not decided here: a hit says which
//! table it is of (`link.at`, `link.table`), and whoever hands it to
//! somebody asks that table, as every other way of reading a row does.

use serde_json::{Value, json};

use crate::{Asked, Doc};

/// Something said on a topic.
#[derive(Clone, Debug)]
pub struct Reply {
    /// The number it was said at, which its topic's talk is opened at.
    pub seq: u64,
    pub body: String,
}

/// A row as the index takes it in.
#[derive(Clone, Debug)]
pub struct Row {
    pub id: String,
    /// Where its table is kept: a project's id, or the factory's own place.
    pub at: String,
    pub table: String,
    /// The project it is of, where it is one's.
    pub project: Option<String>,
    /// Whether it is a topic, a page of a wiki: found as one, and opened as its page.
    pub topic: bool,
    pub title: String,
    /// What its text fields say, one after another.
    pub text: String,
    /// Where it is, in words: the pages above it from its wiki down, or its table.
    pub place: String,
    /// Of a topic, what was said on it, where the whole of it was asked for.
    pub replies: Vec<Reply>,
    /// When it was last changed: what tells the index it is to be read again.
    pub updated: i64,
}

/// A library: the folder of a project whose Markdown files are its documents.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Library {
    pub project: String,
    pub name: String,
    /// From the top of the project; empty for the whole of it.
    pub folder: String,
}

impl Library {
    /// Whether a file of its project, by its path from the top, is one of its documents.
    pub fn holds(&self, project: &str, path: &str) -> bool {
        project == self.project
            && pages::document::is_document(path)
            && (self.folder.is_empty()
                || path.strip_prefix(&self.folder).is_some_and(|rest| rest.starts_with('/')))
    }
}

/// What the index reads of tables, asked back of whoever keeps them.
pub trait Rows: Send + Sync {
    /// Every library there is.
    fn libraries(&self) -> Asked<'_, Vec<Library>>;
    /// Every row of every table the index holds (not a board's cards,
    /// which are found as cards, nor a library's documents, which are
    /// found as files are), each topic with what was said on it.
    fn rows(&self) -> Asked<'_, Vec<Row>>;
    /// A row by its id, as it is now, with every page under it where it is
    /// a page of a wiki (what each is under is said of each): without what
    /// was said on any. Nothing where it is not one the index holds.
    fn row<'a>(&'a self, id: &'a str) -> Asked<'a, Vec<Row>>;
}

pub(crate) fn row_ref(id: &str) -> String {
    format!("row:{id}")
}

fn link(row: &Row) -> Value {
    json!({ "id": row.id, "at": row.at, "table": row.table, "where": row.place })
}

fn kind(row: &Row) -> &'static str {
    if row.topic { "topic" } else { "record" }
}

pub(crate) fn row_doc(row: &Row) -> Doc {
    Doc {
        reference: row_ref(&row.id),
        // Where it is goes with when it changed: a page whose wiki was
        // called something else is written again.
        stamp: format!("{}:{}", row.updated, row.place),
        kind: kind(row),
        project: row.project.clone(),
        title: row.title.clone(),
        body: row.text.clone(),
        link: link(row),
    }
}

pub(crate) fn reply_doc(row: &Row, reply: &Reply) -> Doc {
    let mut link = link(row);
    link["reply"] = json!(reply.seq);
    Doc {
        reference: format!("{}/r{}", row_ref(&row.id), reply.seq),
        stamp: reply.seq.to_string(),
        kind: kind(row),
        project: row.project.clone(),
        title: row.title.clone(),
        body: reply.body.clone(),
        link,
    }
}

/// A document of a library: called by the heading it begins with, or by
/// its file's name where it begins with none.
pub(crate) fn document_doc(library: &Library, path: &str, stamp: String, text: String) -> Doc {
    let parts = pages::document::parts(&text);
    let name = pages::document::stem(path);
    let within = path.strip_prefix(&library.folder).unwrap_or(path).trim_start_matches('/');
    let folders = within.rsplit_once('/').map(|(folders, _)| folders.replace('/', " › "));
    let place = match folders {
        Some(folders) => format!("{} › {folders}", library.name),
        None => library.name.clone(),
    };
    Doc {
        reference: crate::file_ref(&library.project, path),
        stamp,
        kind: "document",
        project: Some(library.project.clone()),
        title: parts.heading.unwrap_or_else(|| name.to_string()),
        body: text,
        link: json!({ "path": path, "library": library.name, "where": place }),
    }
}

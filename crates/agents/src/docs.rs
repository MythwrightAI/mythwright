//! A channel's documents as somebody may see them, and what an agent
//! asked there is told of them; and what is attached to the card an
//! agent's turn is about, which it is told the same way.
//!
//! The list is the channel's (`talk::docs`), kept as what each entry is
//! said as and nothing more. What an entry names is looked at here, as
//! whoever is shown it may read it: a library's record as its library
//! lets them read records, and a project's file as reading that project's
//! files lets them, links followed. An entry is never left out: one that
//! names nothing now is said to be gone, and one whoever looks may not
//! read is said to be kept from them, with nothing of it shown but what
//! the entry itself says. Whoever adds one is held to reading it, there
//! and then ([`Agents::may_add_doc`]).
//!
//! An agent asked something in a channel that keeps documents is told,
//! in a few lines of what it is told ahead of every prompt, each one it
//! may read by what it is called and where it is, with the command that
//! reads it: titles and places only, never a word of what is in them. One
//! it may not read is not told at all.

use kernel::{Error, Principal, Result};
use serde::Serialize;
use studio::docs::{Doc, Kept};
use studio::documents;

use crate::Agents;

/// An entry of a channel's documents, as somebody may see it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Seen {
    /// What it is said as, which is how it is taken away.
    pub doc: String,
    /// `record` or `file`.
    pub kind: &'static str,
    /// The project whose it is: of a file always, of a record where it was found.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
    /// Its file, from the top of its project, where it may be read.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// What it is called, where it may be read: a document by its title,
    /// a file by its name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// `there`; `gone`, where it names nothing now; or `kept`, where
    /// whoever looks may not read it.
    pub state: &'static str,
    /// Who added it, as an actor, and when.
    pub by: String,
    pub added: i64,
}

/// Where a document is, read as somebody may read it.
enum Found {
    There { project: String, path: String, title: String },
    Gone,
    Kept(Error),
}

impl Agents {
    /// What `doc` names, as `by` may read it.
    async fn look_at(&self, doc: &Doc, by: &Principal) -> Found {
        let studio = &self.inner.studio;
        match doc {
            Doc::Record(id) => {
                let Ok((collection, record)) = studio.collections.record(id).await else {
                    return Found::Gone;
                };
                let Some(path) = documents::path_of(&collection, id)
                    .filter(|_| collection.store == studio::Store::Documents)
                else {
                    return Found::Gone;
                };
                if let Err(why) = collection.may_read(by) {
                    return Found::Kept(why);
                }
                let title =
                    record.data.get("title").and_then(|title| title.as_str()).unwrap_or(id).to_string();
                Found::There {
                    project: collection.project.clone(),
                    path: documents::file_of(&collection, &path),
                    title,
                }
            }
            Doc::File { project, path } => {
                if let Err(why) = by.require("projects:read") {
                    return Found::Kept(why);
                }
                let Some(root) = studio.projects.dir_of(project) else { return Found::Gone };
                if let Err(why) = studio.projects.may_read_path(project, path, by).await {
                    return Found::Kept(why);
                }
                if !root.join(path).is_file() {
                    return Found::Gone;
                }
                let title = path.rsplit('/').next().unwrap_or(path).to_string();
                Found::There { project: project.clone(), path: path.clone(), title }
            }
        }
    }

    /// The entries of a channel's documents as `by` may see them, in the
    /// order they were kept.
    pub async fn docs_seen(&self, kept: Vec<Kept>, by: &Principal) -> Vec<Seen> {
        let mut seen = Vec::with_capacity(kept.len());
        for entry in kept {
            let named = Doc::parse(&entry.doc).ok();
            let kind = match named {
                Some(Doc::File { .. }) => "file",
                _ => "record",
            };
            let mut one = Seen {
                doc: entry.doc,
                kind,
                project: None,
                path: None,
                title: None,
                state: "gone",
                by: entry.by,
                added: entry.added,
            };
            if let Some(Doc::File { project, path }) = &named {
                (one.project, one.path) = (Some(project.clone()), Some(path.clone()));
            }
            match named {
                None => {}
                Some(doc) => match self.look_at(&doc, by).await {
                    Found::There { project, path, title } => {
                        (one.project, one.path, one.title, one.state) =
                            (Some(project), Some(path), Some(title), "there");
                    }
                    Found::Gone => one.state = "gone",
                    Found::Kept(_) => one.state = "kept",
                },
            }
            seen.push(one);
        }
        seen
    }

    /// Refuses `doc` to `by` unless it is there and they may read it:
    /// nobody adds to a channel what they could not read themselves.
    pub async fn may_add_doc(&self, doc: &Doc, by: &Principal) -> Result<()> {
        match self.look_at(doc, by).await {
            Found::There { .. } => Ok(()),
            Found::Kept(why) => Err(why),
            Found::Gone => Err(Error::not_found(match doc {
                Doc::Record(id) => format!("there is no document `{id}` in any library"),
                Doc::File { project, path } => format!("there is no file `{path}` in `{project}`"),
            })),
        }
    }

    /// What an agent asked something in `channel` is told of it: what it
    /// says it is for, where its managers said (`channel describe`), and
    /// the documents it keeps that the agent may read, each by what it is
    /// called and where it is, with the command that reads it. Nothing
    /// where the channel says neither, or the agent may read none of them.
    pub(crate) async fn docs_told(&self, channel: Option<&str>, agent: &Principal) -> String {
        let Some(channel) = channel else { return String::new() };
        let channels = &self.inner.studio.channels;
        // As the agent reads the channel, which it is asked in.
        let Ok(reader) = channels.may_read(channel, agent).await else { return String::new() };
        let described = match channels.get(&reader, channel).await {
            Ok(found) if !found.topic.trim().is_empty() => {
                format!("\n\nThe channel you are asked in says what it is for: {}", found.topic.trim())
            }
            _ => String::new(),
        };
        let Ok(kept) = channels.docs(&reader, channel).await else { return described };
        let cli = brand::CLI;
        let mut lines = Vec::new();
        for seen in self.docs_seen(kept, agent).await.into_iter().filter(|seen| seen.state == "there") {
            let (title, project, path) = (
                seen.title.unwrap_or_default(),
                seen.project.unwrap_or_default(),
                seen.path.unwrap_or_default(),
            );
            lines.push(match seen.kind {
                "file" => format!(
                    "- {title}: the file `{path}` of `{project}` (`{cli} file read {project} {path}`)"
                ),
                _ => format!(
                    "- {title}: the document `{}` of a library of `{project}` (`{cli} record show {}`)",
                    seen.doc, seen.doc
                ),
            });
        }
        match lines.is_empty() {
            true => described,
            false => format!(
                "{described}\n\nThe channel you are asked in keeps these documents to hand. Read the ones that bear \
                 on what you are asked:\n{}",
                lines.join("\n")
            ),
        }
    }
}

impl Agents {
    /// What is attached to the card a turn's conversation is about
    /// (`lane:<card>`, `review:<card>:<seat>` and the rest of
    /// `deciding::spent::CONVERSATIONS`), as the agent may read the card:
    /// each by its name and size with the command that reads it, and
    /// never what is in it. Nothing where the conversation is about no
    /// card, or the card has nothing attached.
    pub(crate) async fn attached_told(&self, thread: &str, agent: &Principal) -> String {
        let Some((word, rest)) = thread.split_once(':') else { return String::new() };
        if !deciding::spent::CONVERSATIONS.iter().any(|(said, _)| *said == word) {
            return String::new();
        }
        let card = rest.split(':').next().unwrap_or_default();
        // Most cards have nothing attached: that is asked by one look, and
        // the card and who may read it only where something is.
        if !self.inner.studio.collections.any_attached(card).await.unwrap_or(false) {
            return String::new();
        }
        let Ok(attached) = self.inner.studio.records.attachments(card, agent).await else {
            return String::new();
        };
        let cli = brand::CLI;
        let lines: Vec<String> = attached
            .iter()
            .map(|one| {
                let size = match one.size {
                    size if size >= 1024 * 1024 => format!("{:.1} MB", size as f64 / (1024.0 * 1024.0)),
                    size if size >= 1024 => format!("{} KB", size.div_ceil(1024)),
                    size => format!("{size} bytes"),
                };
                format!(
                    "- {} ({}, {size}), attached by {}: `{cli} attachment {} --out <file>`",
                    one.name, one.media, one.who, one.id
                )
            })
            .collect();
        match lines.is_empty() {
            true => String::new(),
            false => format!(
                "\n\n{card} has these attached. Read the ones that bear on what you are asked:\n{}",
                lines.join("\n")
            ),
        }
    }
}

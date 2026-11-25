//! What a search finds by what it is called, read as it is asked for: a
//! project, a channel, somebody, an agent, a blueprint applied and a run.
//! Each is few beside what the index holds, or (a run) said too often to
//! keep an index of, and is always found by what it is called now. Who
//! lists them for a search lists only what its asker may read.

use serde_json::Value;

use crate::{Hit, MARK_END, MARK_START};

/// Something to be found by what it is called.
#[derive(Clone, Debug)]
pub struct Named {
    pub reference: String,
    /// `project`, `channel`, `person`, `agent` or `run`.
    pub kind: &'static str,
    pub project: Option<String>,
    pub title: String,
    /// What else it is known by: a handle, a name, an id.
    pub also: Vec<String>,
    /// Where it is, in words, where it is somewhere.
    pub place: Option<String>,
    /// What the client needs to open it.
    pub link: Value,
}

/// Those of `named` that answer what was typed (each word begins a word of
/// what one is called), the best first, at most `limit`.
pub fn found(text: &str, named: Vec<Named>, limit: usize) -> Vec<Hit> {
    let typed = kernel::sought::words(text);
    let mut answering: Vec<(u8, Named)> = named
        .into_iter()
        .filter_map(|one| {
            let names: Vec<&str> =
                std::iter::once(one.title.as_str()).chain(one.also.iter().map(String::as_str)).collect();
            kernel::sought::answers(&typed, &names).map(|rank| (rank, one))
        })
        .collect();
    answering.sort_by(|(a, one), (b, other)| {
        a.cmp(b).then(one.title.len().cmp(&other.title.len())).then(one.title.cmp(&other.title))
    });
    answering
        .into_iter()
        .take(limit)
        .map(|(_, one)| {
            let mark = |name: &str| kernel::sought::marked(name, &typed, (MARK_START, MARK_END));
            let snippet = std::iter::once(&one.title)
                .chain(&one.also)
                .map(|name| mark(name))
                .collect::<Vec<_>>()
                .join(" · ");
            Hit {
                reference: one.reference,
                kind: one.kind.to_string(),
                project: one.project,
                title: one.title,
                snippet,
                place: one.place,
                link: one.link,
            }
        })
        .collect()
}

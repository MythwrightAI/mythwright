//! Somebody named in what is said on a record, told in their box.
//!
//! What is decided of it is `talk`'s ([`talk::commented`]): who a word
//! names, whether they read the record, and the line. This is what that
//! asks of the records. The line is told by the keeper of histories as it
//! writes the comment's line of the record's history, and before it keeps
//! where it has read to ([`crate::history`]), so a comment is told of once
//! wherever the daemon stops. What reading a record takes, and what was
//! said on it as somebody may read it now, are answered here, of the
//! tables the keeper holds in memory as they are declared ([`Declared`]).
//!
//! Whoever follows a page of a wiki is told of a change of it the same way
//! ([`talk::followed`]): by the keeper of histories, as it writes the
//! change's line and before it keeps where it has read to, of the page's
//! followers as the page holds them then ([`change`]).

use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, RwLock};

use kernel::{Event, Principal};
use serde_json::{Map, Value};
use talk::commented::{Comment, OnRecord};
use talk::followed::Change;

use crate::board::BOARD;
use crate::collections::{Collection, Collections};

/// Every table there is, as the keeper of histories last read them.
pub type Declared = Arc<RwLock<Vec<Collection>>>;

impl Collection {
    /// What reading a record of this takes, every scope of it.
    pub fn reading(&self) -> Vec<String> {
        let read = if self.name == BOARD { "board:read" } else { "records:read" };
        std::iter::once(read.to_string()).chain(self.reads.clone()).collect()
    }
}

/// The table that owns `record`, of those `declared` holds.
fn owner(declared: &Declared, record: &str) -> Option<Collection> {
    declared.read().expect("declared lock").iter().find(|table| table.owns(record)).cloned()
}

/// What a comment on `record` of `collection` is, from its event, where
/// it names anybody or anybody follows the record (`followers`): what
/// `talk` is handed to tell them. `from` is who said it, which the line
/// says it came from, and `by` whose doing it was, a key's maker for a
/// key: neither is told.
pub(crate) fn comment(
    collection: &Collection,
    record: &str,
    event: &Event,
    (from, by): (String, String),
    (followers, above): Followed,
) -> Option<Comment> {
    let body = event.data.get("body").and_then(Value::as_str)?;
    let followed = !followers.is_empty() || above.iter().any(|(_, followers)| !followers.is_empty());
    if talk::channels::mentions(body).is_empty() && !followed {
        return None;
    }
    let from = if from.is_empty() { by.clone() } else { from };
    Some(Comment {
        record: record.to_string(),
        seq: event.seq,
        from,
        by,
        body: body.to_string(),
        takes: collection.reading(),
        at: event.ts,
        followers,
        above,
    })
}

/// Who follows a page as a change of it was made, and the pages above it
/// with who follows each.
pub(crate) type Followed = (Vec<String>, Vec<(String, Vec<String>)>);

/// What a change of `record` of `collection` is to whoever follows it, from
/// its event, where it is a page of a wiki somebody followed then and
/// follows still, or a page above it does with its pages, and it is a
/// change they are told of (`what`, `kernel::following::page_change`):
/// what `talk` is handed to tell them.
pub(crate) fn change(
    collection: &Collection,
    record: &str,
    event: &Event,
    (from, by): (String, String),
    (followers, above): Followed,
    what: kernel::following::PageChange,
) -> Option<Change> {
    let followers = if what.own { followers } else { Vec::new() };
    let above = if what.above { above } else { Vec::new() };
    if followers.is_empty() && above.iter().all(|(_, followers)| followers.is_empty()) {
        return None;
    }
    let from = if from.is_empty() { by.clone() } else { from };
    Some(Change {
        record: record.to_string(),
        seq: event.seq,
        from,
        by,
        takes: collection.reading(),
        at: event.ts,
        followers,
        above,
        topic: event.topic.clone(),
        noun: what.noun.to_string(),
        verb: what.verb.to_string(),
        own: what.own,
    })
}

/// Who follows a record of `collection` that holds `data`: nobody, but of a
/// page of a wiki.
pub(crate) fn followers(collection: &Collection, data: &Map<String, Value>) -> Vec<String> {
    match collection.model.as_deref() == Some(kernel::topics::MODEL) {
        true => crate::models::Topic::read(data).followers,
        false => Vec::new(),
    }
}

/// What answers for a line of something said on a record. It holds the
/// records weakly: it is held by the inbox, which the journal holds, and
/// the records hold the journal, so held strongly a node was never let go
/// of. Asked once the studio is gone, it answers that nothing was said.
pub(crate) struct OnRecords {
    pub(crate) collections: std::sync::Weak<Collections>,
    pub(crate) declared: Declared,
}

impl talk::commented::OnRecords for OnRecords {
    fn takes(&self, record: &str) -> Option<Vec<String>> {
        owner(&self.declared, record).map(|table| table.reading())
    }

    fn table_takes(&self, project: &str, name: &str) -> Option<Vec<String>> {
        let declared = self.declared.read().expect("declared lock");
        declared.iter().find(|table| table.project == project && table.name == name).map(Collection::reading)
    }

    fn said(
        &self,
        by: Principal,
        record: String,
        seq: u64,
    ) -> Pin<Box<dyn Future<Output = Option<OnRecord>> + Send>> {
        let (collections, table) = (self.collections.upgrade(), owner(&self.declared, &record));
        Box::pin(async move {
            let (collections, table) = (collections?, table?);
            table.may_read(&by).ok()?;
            let line = collections.history(&record).await.ok()?.into_iter().find(|line| line.seq == seq)?;
            // A change that is no comment (of a page somebody follows) says no words.
            let body = line.data.get("body").and_then(Value::as_str).unwrap_or_default().to_string();
            // What it is called, where it is still there and says.
            let kept = collections.record(&record).await.ok().map(|(_, kept)| kept);
            let title = kept.as_ref().and_then(|kept| {
                ["title", "name"]
                    .iter()
                    .find_map(|name| kept.data.get(*name).and_then(Value::as_str).map(str::to_string))
            });
            // A page of a wiki is said by where it is now: the titles above it.
            let path = match &kept {
                Some(kept)
                    if table.project == kernel::place::FACTORY && table.name == kernel::topics::TOPICS =>
                {
                    above(&collections, kept).await
                }
                _ => Vec::new(),
            };
            Some(OnRecord {
                record,
                project: table.project,
                table: table.name,
                title,
                path,
                who: line.who,
                body,
                did: kernel::topics::change_said(&line.topic).map(str::to_string),
                topic: line.topic,
                seq,
                created: line.ts,
            })
        })
    }
}

/// The titles of the pages above a topic, its wiki first, read by id up
/// its `parent`s as they are now: a parent that names nothing, and a ring,
/// end it (`kernel::topics::tree` reads the tree so too).
async fn above(collections: &Collections, page: &crate::Record) -> Vec<String> {
    let parent = |data: &serde_json::Map<String, Value>| {
        data.get(kernel::topics::PARENT)
            .and_then(Value::as_str)
            .filter(|said| !said.is_empty())
            .map(str::to_string)
    };
    let (mut titles, mut seen) = (Vec::new(), vec![page.id.clone()]);
    let mut at = parent(&page.data);
    while let Some(id) = at.take() {
        if seen.contains(&id) {
            return Vec::new();
        }
        let Ok((_, kept)) = collections.record(&id).await else { break };
        titles.push(kept.data.get("title").and_then(Value::as_str).unwrap_or(&id).to_string());
        at = parent(&kept.data);
        seen.push(id);
    }
    titles.reverse();
    titles
}

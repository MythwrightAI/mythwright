//! What is said on a record that names somebody puts a line in their box.
//!
//! **Told.** Something said on a record (`record.commented`, any record,
//! a card among them) that names somebody with `@` puts a line in their
//! box ([`crate::inbox`], kind `commented`): of a person, by the word
//! their account's name comes to (`people::handle`, asked of what was
//! installed: [`crate::Channels::names_with`], as a channel's message
//! names somebody), and of an agent the factory does not start, by its
//! name, as a person is told. An agent the factory starts is given no
//! line and no turn: what is said on a record is no conversation with it.
//! Nobody is told who may not read the record as things stand, asked in
//! the write that tells, of what they hold then and of what reading the
//! record takes, which whoever tells hands over ([`Comment::takes`]): the
//! factory's owner is told where named, and never for reading everything.
//! Whoever said it is not told. Somebody who follows the record (a page
//! of a wiki) is told by their line of the page ([`crate::followed`]),
//! which the same write raises, and not named besides: one line for one
//! thing said.
//!
//! **Once.** It is told by whoever keeps the record's history, as the
//! line of it is written and before where they have read to is kept, so a
//! comment heard again (a daemon started again in between) is told again
//! by the same, which a line keeps as being about that very comment (its
//! number in the journal), and nothing is told twice.
//!
//! **Shown.** The line keeps no words: what it is about is the comment
//! by its number, and what opens it is the record. Whoever reads the box
//! is shown the words from the record's history as they may read the
//! record now, by what keeps records ([`OnRecords`], installed by it,
//! since nothing here knows a table); one they may no longer read is
//! passed over, as a channel's line is, not listed and not counted.

use kernel::principal::covers;
use kernel::{Principal, Result};
use serde::Serialize;
use std::future::Future;
use std::pin::Pin;

use crate::channels::{Channels, mentions};
use crate::inbox::Tell;

/// Something said on a record, as whoever tells of it hands it over.
#[derive(Clone, Debug)]
pub struct Comment {
    /// The record it was said on, which is what opens the line.
    pub record: String,
    /// Its number in the journal, which is what the line is about.
    pub seq: u64,
    /// Who the journal says said it (`user:`, `key:<id>`, `agent:`), which
    /// the line says it came from.
    pub from: String,
    /// Whose doing that was, as an actor: for a key, its maker. Not told.
    pub by: String,
    pub body: String,
    /// What reading the record takes, every scope of it.
    pub takes: Vec<String>,
    /// When it was said.
    pub at: i64,
    /// Who follows the record, a page of a wiki, each an actor: told of
    /// it by their line of the page ([`crate::followed`]), and by no other
    /// where they are told at once.
    pub followers: Vec<String>,
    /// The pages above it, the nearest first, each with who follows it:
    /// told where they follow it with its pages.
    pub above: Vec<(String, Vec<String>)>,
}

impl Comment {
    /// What it is to whoever follows the record: a change of it.
    pub fn change(&self) -> crate::followed::Change {
        crate::followed::Change {
            record: self.record.clone(),
            seq: self.seq,
            from: self.from.clone(),
            by: self.by.clone(),
            takes: self.takes.clone(),
            at: self.at,
            followers: self.followers.clone(),
            above: self.above.clone(),
            topic: "record.commented".to_string(),
            noun: "reply".to_string(),
            verb: "said".to_string(),
            own: true,
        }
    }
}

/// What a line of something said on a record is shown with.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct OnRecord {
    /// The record, its project and its table.
    pub record: String,
    pub project: String,
    pub table: String,
    /// What the record is called, where it says (its `title` or `name`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Where it is, where it is a page of a wiki: the titles of
    /// the pages above it as they are now, its wiki first.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub path: Vec<String>,
    /// Who said it, by name as the history has them, and the words.
    pub who: String,
    pub body: String,
    /// What the line of the record's history is of (`record.commented`;
    /// for a page somebody follows, whatever changed it, `topic.revised`).
    pub topic: String,
    /// What the change did, in a few words, where it is no comment: what a
    /// line of a page somebody follows says in place of words
    /// (`kernel::topics::change_said`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub did: Option<String>,
    /// The comment's place in the record's history.
    pub seq: u64,
    pub created: i64,
}

/// What keeps records answers for a line of something said on one: said
/// by it, since nothing here knows a table ([`crate::Inbox::records_with`]).
pub trait OnRecords: Send + Sync {
    /// What reading `record` takes now, every scope of it; nothing where
    /// no table there is owns it. Asked in a write, of what is in memory.
    fn takes(&self, record: &str) -> Option<Vec<String>>;

    /// What reading the records of the table `name` of `project` takes now,
    /// every scope of it; nothing where there is no such table. Asked in a
    /// write, of what is in memory, for whoever follows a table.
    fn table_takes(&self, _project: &str, _name: &str) -> Option<Vec<String>> {
        None
    }

    /// What was said on `record` at `seq`, as `by` may read it now:
    /// nothing where they may not, or it is not there.
    fn said(
        &self,
        by: Principal,
        record: String,
        seq: u64,
    ) -> Pin<Box<dyn Future<Output = Option<OnRecord>> + Send>>;
}

/// Whether whoever holds `held` reads what takes every scope of `takes`.
pub(crate) fn reads(held: &[String], takes: &[String]) -> bool {
    takes.iter().all(|scope| covers(held, scope))
}

impl Tell {
    /// That `owner` was named in what `from` said on `record`, the
    /// comment that is `seq` in the journal.
    pub fn commented(owner: &str, record: &str, seq: u64, from: &str) -> Tell {
        Tell {
            owner: owner.to_string(),
            kind: crate::inbox::Kind::Commented,
            about: seq.to_string(),
            place: record.to_string(),
            from: from.to_string(),
            why: "named".to_string(),
            says: None,
            said_by: None,
            parts: None,
        }
    }
}

impl Channels {
    /// Tells whoever `comment` names, each in their box where they read
    /// the record as things stand: the people its words name, and the
    /// agents the factory does not start; and whoever follows the record,
    /// by their line of it, which is all a follower it names is told. One
    /// write; nothing at all where it names nobody and nobody follows.
    /// How many were told.
    pub async fn tell_commented(&self, comment: Comment) -> Result<usize> {
        let names = mentions(&comment.body);
        if names.is_empty() && comment.followers.is_empty() && comment.above.is_empty() {
            return Ok(0);
        }
        let (inbox, naming) = (self.inbox.clone(), self.naming.get().cloned());
        let wrote = self.journal.write(move |writing| {
            let transaction = writing.transaction();
            let mut named: Vec<(String, Vec<String>)> = match &naming {
                Some(naming) if !names.is_empty() => {
                    naming(transaction, &names)?.into_iter().map(|one| (one.actor(), one.scopes)).collect()
                }
                _ => Vec::new(),
            };
            for name in &names {
                let actor = format!("agent:{name}");
                if inbox.memory.outside(&actor) && !named.iter().any(|(one, _)| *one == actor) {
                    let held = inbox.held(transaction, &actor)?;
                    named.push((actor, held));
                }
            }
            let (follows, mut told) = crate::followed::tell_in(writing, &inbox, &comment.change())?;
            for (actor, held) in named {
                let follows = follows.contains(&actor);
                if follows || actor == comment.by || actor == comment.from || !reads(&held, &comment.takes) {
                    continue;
                }
                let tell = Tell::commented(&actor, &comment.record, comment.seq, &comment.from);
                told += usize::from(inbox.tell_in(writing, &tell, comment.at)?.is_some());
            }
            Ok(told)
        });
        let told = wrote.await?.0;
        self.following.wakes().notify_one();
        Ok(told)
    }
}

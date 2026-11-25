//! Something said on a record that answers something said there before
//! (`parent`, the number it was said at): so a record's comments are
//! replies, nested as a forum's are (a topic's).
//!
//! What it answers is a comment of the same record, found among the
//! lines of its history, and where the keeper of histories has not yet
//! written that line, in the journal that keeps what is said of it. It is
//! put under that one where it is no deeper than replies go, and beside
//! it at the deepest there is otherwise (`kernel::topics::placed_under`):
//! where it goes is said with it, and kept with its line as `anchor` is.

use journal::EventQuery;
use kernel::{Error, Principal, Result};
use serde_json::Value;

use crate::collections::Record;
use crate::records::{COMMENTED, Records, Saying};

/// What something said on a record says of what it answers.
pub const PARENT: &str = "parent";

impl Records {
    /// Says an agent's answer to what it was asked on a record, under what
    /// asked it (`parent`), as the agent (`agent`): the factory says it for
    /// the agent it started, on the leave of whoever asked, who said that
    /// there. So the agent is not asked whether it may change the record,
    /// and is given nothing by it: whatever it says by its own commands is
    /// asked of it as ever ([`Records::reply`]). What is said, and the
    /// number it was said at.
    pub async fn answer(
        &self,
        id: &str,
        body: &str,
        parent: u64,
        agent: &Principal,
    ) -> Result<(Record, u64)> {
        let saying = Saying {
            body,
            public: false,
            anchor: None,
            once: None,
            asker: None,
            parent: Some(parent),
            answered: true,
        };
        self.say_on(id, saying, agent, None).await
    }

    /// The comment a reply to `parent` on `record` is put under, or why
    /// there is none to answer.
    pub(crate) async fn placed_under(&self, record: &Record, parent: u64) -> Result<u64> {
        let parent_of = |data: &Value| data.get(PARENT).and_then(Value::as_u64);
        let mut replies: Vec<(u64, Option<u64>)> = self
            .collections
            .history(&record.id)
            .await?
            .into_iter()
            .filter(|line| line.topic == COMMENTED)
            .map(|line| (line.seq, parent_of(&line.data)))
            .collect();
        // What it answers, and each above that up to the top of its chain, said a moment ago and not yet a
        // line of its history is read from the journal: where the keeper of histories is behind by several
        // replies, a chain read from the history alone is too short, and a reply would be put too deep.
        let mut wanted = Some(parent);
        while let Some(seq) = wanted {
            if let Some((_, above)) = replies.iter().find(|(one, _)| *one == seq) {
                wanted = *above;
                continue;
            }
            match self.said_at(record, seq).await? {
                Some(above) => {
                    replies.push((seq, above));
                    // Each answers something said before it, so the walk up ends.
                    wanted = above.filter(|above| *above < seq);
                }
                None if seq == parent => {
                    return Err(Error::not_found(format!(
                        "nothing was said on `{}` at {parent} to answer: a reply answers something said there",
                        record.id
                    )));
                }
                None => break,
            }
        }
        let chain = kernel::topics::chain(&replies, parent);
        Ok(kernel::topics::placed_under(&chain).unwrap_or(parent))
    }

    /// What something said on `record` at `seq` answers, read from the
    /// journal: `None` where nothing was said there at that number, and
    /// `Some(None)` where it answers the record itself.
    async fn said_at(&self, record: &Record, seq: u64) -> Result<Option<Option<u64>>> {
        let query = EventQuery {
            after: seq.saturating_sub(1),
            topics: COMMENTED.to_string(),
            limit: 1,
            ..EventQuery::default()
        };
        let found = self.journal.of(Some(&record.project)).read(query).await?;
        Ok(found
            .into_iter()
            .find(|event| {
                event.seq == seq && event.data.get("id").and_then(Value::as_str) == Some(record.id.as_str())
            })
            .map(|event| event.data.get(PARENT).and_then(Value::as_u64)))
    }
}

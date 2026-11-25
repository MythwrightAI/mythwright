//! What a blueprint's step left (a row added, a card added, something
//! said on a record), found again where the step is done again because
//! the daemon stopped in the middle of it ([`conductor::ActionCx::again`]):
//! so that `record.add`, `board.add` and `record.comment` are the same
//! done twice, as `channel.post` is.
//!
//! What is left is said in the write that keeps it (`record.created`,
//! `board.item.created`, `record.commented`), and what an action says
//! names its run and its step, as everything said of a step's doing does
//! ([`conductor::ActionCx::source`]); it says too which of the things done
//! under the step's name it is ([`DOING`], `ActionCx::doing`), as a
//! message keeps it. So a step done again looks among what was said on
//! that topic in the journal that keeps what is said of the project,
//! from a little before its try began (`ActionCx::began_at`, a place
//! found by the time: `Journal::place_before`), by its run, and takes
//! what says its step and its doing. Nothing is kept for it beside what
//! is said, and nothing is read on the way of a step that is not done
//! again.
//!
//! A table kept outside the journal's database (as files of the project,
//! by a store service, as documents or mockups) has its record written
//! and then said, so a stop between the two leaves a record nothing said:
//! a step done again that finds nothing there is refused as it always
//! was, and not given a second record.

use conductor::ActionCx;
use journal::{EventQuery, Journal};
use kernel::{Error, Result};
use serde_json::Value;

use crate::Studio;
use crate::board::Through;
use crate::collections::Store;

/// What something a step left says of which of the things done under
/// its step's name left it.
pub const DOING: &str = "doing";

/// How many of a run's events on a topic are read at a time.
const PAGE: usize = 500;

/// Where no row says when the step began (a program from before wrote
/// it): how many of what was said on the topic lately are looked through.
const LATELY: usize = 500;

/// Says on what is said of something a step leaves which of the things
/// done under its step's name left it: nothing for what nobody's step did.
pub(crate) fn mark(through: Option<&Through>, data: &mut Value) {
    if let Some(doing) = through.and_then(|through| through.doing.as_deref()) {
        data[DOING] = Value::String(doing.to_string());
    }
}

/// Which doing of which step is looking, and since when.
struct Looking {
    run: String,
    step: String,
    doing: String,
    since: Option<i64>,
}

impl Looking {
    async fn of(cx: &ActionCx) -> Looking {
        Looking {
            run: cx.run().to_string(),
            step: cx.step().to_string(),
            doing: cx.doing().to_string(),
            since: cx.began_at().await,
        }
    }

    fn left(&self, event: &kernel::Event) -> bool {
        event.source.run.as_deref() == Some(self.run.as_str())
            && event.source.step.as_deref() == Some(self.step.as_str())
            && event.data.get(DOING).and_then(Value::as_str) == Some(self.doing.as_str())
    }

    /// What was said on `topic` in `journal` by this doing of the step.
    async fn find(&self, journal: &Journal, topic: &str) -> Result<Option<Value>> {
        let Some(since) = self.since else {
            let lately =
                EventQuery { topics: topic.into(), newest_first: true, limit: LATELY, ..Default::default() };
            let said = journal.read(lately).await?;
            return Ok(said.into_iter().find(|event| self.left(event)).map(|event| event.data));
        };
        let mut after = journal.place_before(since).await?;
        loop {
            let page = EventQuery {
                after,
                topics: topic.into(),
                run: Some(self.run.clone()),
                limit: PAGE,
                ..Default::default()
            };
            let said = journal.read(page).await?;
            let last = said.last().map(|event| event.seq);
            if let Some(found) = said.iter().find(|event| self.left(event)) {
                return Ok(Some(found.data.clone()));
            }
            match last {
                Some(last) if said.len() == PAGE => after = last,
                _ => return Ok(None),
            }
        }
    }
}

impl Studio {
    /// The id of what this doing of `cx`'s step left of `project`'s that
    /// it said on `topic`, where the step is being done again and left
    /// one; nothing where it is not, or left none. Where it left none and
    /// `table` is kept outside the journal's database, a record may be
    /// there that nothing said, and the step is refused, as a step that
    /// cannot be done twice is.
    pub(crate) async fn left(
        &self,
        cx: &ActionCx,
        (project, table): (&str, Option<&str>),
        topic: &str,
    ) -> Result<Option<String>> {
        if !cx.again() {
            return Ok(None);
        }
        let looking = Looking::of(cx).await;
        let found = looking.find(&self.journal.said_of(Some(project)), topic).await?;
        if let Some(found) = found {
            return Ok(found.get("id").and_then(Value::as_str).map(str::to_string));
        }
        let Some(table) = table else { return Ok(None) };
        match self.collections.find(project, table).await? {
            Some(kept) if kept.store != Store::Local => Err(Error::invalid(format!(
                "was interrupted by a restart, and `{table}` keeps its records outside the factory's database, \
                 where one may have been written that nothing said: it is not added again automatically"
            ))),
            _ => Ok(None),
        }
    }
}

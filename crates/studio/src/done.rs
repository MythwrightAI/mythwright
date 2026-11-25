//! What the factory's bot did to records lately: the lines of records'
//! histories a blueprint's step wrote, which are the bot's doing
//! (`kernel::worded`), for whoever reads about the bot. Nothing is kept
//! for it: it is read from the histories as they are, and each line is
//! held to what its reader may read as that record's history is.

use kernel::{Principal, Result};
use serde::Serialize;
use serde_json::Value;

use crate::Collections;
use crate::history::{Line, as_called};

/// A line of a record's history that says what the bot did, with which
/// record it is of and where that is.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Done {
    pub record: String,
    pub project: String,
    /// The table the record is of.
    pub collection: String,
    #[serde(flatten)]
    pub line: Line,
}

/// How many lines of a journal are read at a time, looking for those a
/// reader may read.
const PAGE: usize = 200;

/// The lines of a history a blueprint's step wrote, newest first.
const THROUGH: &str = "SELECT record, seq, ts, topic, actor, who, said, data FROM record_history \
                       WHERE instr(data, '\"through\"') > 0 AND coalesce(json_extract(data, '$.through'), '') <> '' \
                       ORDER BY ts DESC, seq DESC LIMIT ?1 OFFSET ?2";

impl Collections {
    /// What the bot did to records lately: the lines of every record's
    /// history a blueprint's step wrote, newest first and `most` at most,
    /// each worded as the bot is called now, and only of a record `by`
    /// may read the history of (`Collection::may_read`): a line of a
    /// table that is not there any more, or kept from them, is passed
    /// over. No index goes by who did a thing, so each journal's
    /// histories are looked through, a page at a time; it is asked only
    /// where somebody reads about the bot.
    pub async fn done_through(&self, by: &Principal, most: usize) -> Result<Vec<Done>> {
        let tables = self.list(None).await?;
        let bot =
            self.bot.get().map(|speaker| speaker.name()).unwrap_or_else(|| talk::speaker::DEFAULT.into());
        let mut done: Vec<Done> = Vec::new();
        for journal in self.factory().every_records() {
            let mut skip = 0;
            let mut found = 0;
            loop {
                let page: Vec<(String, Line)> = journal
                    .db()
                    .call(move |connection| {
                        connection
                            .prepare(THROUGH)?
                            .query_map((PAGE as i64, skip as i64), |row| {
                                let line = Line {
                                    seq: row.get::<_, i64>(1)? as u64,
                                    ts: row.get(2)?,
                                    topic: row.get(3)?,
                                    actor: row.get(4)?,
                                    who: row.get(5)?,
                                    said: row.get(6)?,
                                    data: serde_json::from_str(&row.get::<_, String>(7)?)
                                        .unwrap_or(Value::Null),
                                };
                                Ok((row.get(0)?, line))
                            })?
                            .collect()
                    })
                    .await?;
                let ended = page.len() < PAGE;
                skip += page.len();
                for (record, mut line) in page {
                    let Some(table) = tables.iter().find(|table| table.owns(&record)) else { continue };
                    if table.may_read(by).is_err() {
                        continue;
                    }
                    (line.who, line.said) = as_called(&line.who, &line.said, &line.data, &bot);
                    let (project, collection) = (table.project.clone(), table.name.clone());
                    done.push(Done { record, project, collection, line });
                    found += 1;
                }
                if ended || found >= most {
                    break;
                }
            }
        }
        done.sort_by_key(|one| std::cmp::Reverse(one.line.ts));
        done.truncate(most);
        Ok(done)
    }
}

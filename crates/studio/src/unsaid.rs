//! What was written of a card and never said.
//!
//! A card's move is written to its row and then said, and the daemon may
//! stop between the two. Nothing hears of such a move: the stage of the
//! column the card is now in was never told it came, and what counts and
//! tidies was never told it left. What was said is in the journal, and
//! what is so is the row; so when the daemon has started the two are held
//! against each other, and a card whose row is in another column than it
//! was last said to be in has that move said now, late. From there it is
//! any move: whatever would have heard it then hears it once.
//!
//! A card that was added with nobody told is the same hole by another
//! door: its row is there, and nothing was ever said of it. It is said to
//! have been added, late, and is from there a card added like any other.
//!
//! It is exact, and the same done twice. A move is written and said under
//! its record's turn, and each card is looked at under that turn: so a
//! move being made at this moment is seen whole or not at all, and a move
//! said here is found said by whoever looks next.
//!
//! Only a row written about when the daemon last stopped can be one of
//! these: a move is said in the moment after it is written. So a board is
//! looked at once for a start of the daemon (asked again, it answers at
//! once that there is nothing), and of its cards only those written since
//! a minute before the last thing the journal kept before this start are
//! looked at closely, and where the board is kept here those are all
//! that is read of it; on a board of thousands of cards said long ago
//! that is none of them. Where such a card was last said to be is read
//! from what the journal kept of late, and for a card not spoken of there
//! from its own history, which is kept for good.
//!
//! Who made the move is not known: the row says only what kind of actor
//! did. It is said by the system, which holds nothing, so that nothing is
//! done on the authority of whoever it was (`actor.holds` of a late move
//! says no). What was said as it was moved is on the row, written with
//! the move (`models::Moved`), and is said with it.

use std::collections::HashMap;

use journal::EventQuery;
use kernel::{Result, Source};
use serde_json::{Value, json};

use crate::board::Board;
use crate::models::{Card, Moved};

const MOVED: &str = "board.item.moved";
const CREATED: &str = "board.item.created";

/// How much of what the journal last kept of cards is read: more than is
/// ever said between a row being written and the daemon stopping. What a
/// card was said to be before that is in its history.
const OF_LATE: usize = 2000;

/// How long before the last thing the journal kept a row may have been
/// written and its saying still be the one that was lost, in
/// milliseconds: far longer than the moment between the two.
const ABOUT_THEN_MS: i64 = 60_000;

impl Board {
    /// Says, of each card of `project`'s board that was moved or added
    /// with nobody told, that it was: the cards it was said of, each with
    /// where it was last said to be (nowhere, of one never said to be
    /// made) and where it is. Once for a start of the daemon: asked
    /// again, it answers at once that there is nothing.
    pub async fn settle(&self, project: &str) -> Result<Vec<Value>> {
        // Held for the whole of the look: whoever asks meanwhile waits,
        // and is told there is nothing more.
        let mut settled = self.settled.lock().await;
        if settled.contains(project) {
            return Ok(Vec::new());
        }
        let late = self.unsaid(project).await?;
        settled.insert(project.to_string());
        Ok(late)
    }

    /// The look itself, however often it is asked for.
    pub async fn unsaid(&self, project: &str) -> Result<Vec<Value>> {
        let board = self.of(project).await?;
        let since = self.stopped_about().await? - ABOUT_THEN_MS;
        // Asked of where the cards are kept, so that a board of thousands
        // said long ago is not read to find that none of them was written.
        let written = self.collections.written_since(&board, since).await?;
        let mut late = Vec::new();
        if written.is_empty() {
            return Ok(late);
        }
        let of_late = self.said_of_late(project).await?;
        for card in written {
            let said = match of_late.get(&card.id) {
                Some(column) => Some(column.clone()),
                None => self.said_in_history(&card.id).await?,
            };
            // Said to be where it is, or said nothing of and made long ago
            // (a board that keeps no history): nothing is said on a guess.
            let is_in = Card::read(card.data.clone()).status;
            if said.as_deref() == Some(is_in.as_str()) || (said.is_none() && card.created < since) {
                continue;
            }
            // Under its turn, as a move is made: whoever is moving it now
            // has written and said, or has done neither.
            let _turn = self.collections.changing(&card.id).await;
            let Ok((board, record)) = self.card(&card.id).await else { continue };
            let made = record.created;
            let holds = record.data.clone();
            let now = Card::read(record.data);
            let said = match self.said_of_late(project).await?.remove(&card.id) {
                Some(column) => Some(column),
                None => self.said_in_history(&card.id).await?,
            };
            let source = Source::actor("system");
            match said {
                Some(from) if from != now.status => {
                    let mut data = json!({
                        "id": card.id, "project": project, "title": now.title,
                        "from": from, "to": now.status, "by": "system", "late": true,
                    });
                    // What was said as it was moved is on the row, where
                    // the row's word of its last move is of this one.
                    let said =
                        Moved::read(&now.moved).filter(|moved| moved.from == from).map(|moved| moved.said);
                    if let Some(said) = said.filter(|said| !said.is_empty()) {
                        data["note"] = json!(said);
                    }
                    let (source, data) = crate::fieldsaid::onto(&board, &holds, (source, data));
                    self.journal.emit("board.item.moved", source, data).await?;
                    late.push(json!({ "id": card.id, "from": from, "to": now.status }));
                }
                None if made >= since => {
                    let data = json!({
                        "id": card.id, "project": project, "title": now.title, "status": now.status, "late": true,
                    });
                    let (source, data) = crate::fieldsaid::onto(&board, &holds, (source, data));
                    self.journal.emit("board.item.created", source, data).await?;
                    late.push(json!({ "id": card.id, "from": "", "to": now.status }));
                }
                _ => {}
            }
        }
        Ok(late)
    }

    /// When the last thing was kept that the journals held as this
    /// daemon opened them, in milliseconds; the beginning of time where
    /// they held nothing. The newest any of them still has of what it
    /// held then: an event may have been taken out since.
    async fn stopped_about(&self) -> Result<i64> {
        let mut about = 0;
        for journal in self.journal.every() {
            let last = journal.held_when_opened();
            let query = EventQuery { before: last + 1, newest_first: true, limit: 1, ..Default::default() };
            about = about.max(journal.read(query).await?.first().map_or(0, |event| event.ts));
        }
        Ok(about)
    }

    /// The column each card of `project` was last said to be in, by what
    /// the journal kept of late.
    async fn said_of_late(&self, project: &str) -> Result<HashMap<String, String>> {
        let query = EventQuery {
            topics: format!("{MOVED},{CREATED}"),
            newest_first: true,
            limit: OF_LATE,
            ..Default::default()
        };
        let mut said = HashMap::new();
        // Whichever journal keeps what is said of the project: the
        // newest first, by when each was said.
        for event in self.journal.read_every(query).await? {
            if event.data["project"] != project {
                continue;
            }
            let column = if event.topic == MOVED { &event.data["to"] } else { &event.data["status"] };
            if let (Some(id), Some(column)) = (event.data["id"].as_str(), column.as_str()) {
                said.entry(id.to_string()).or_insert_with(|| column.to_string());
            }
        }
        Ok(said)
    }

    /// The same of one card, by its own history.
    async fn said_in_history(&self, id: &str) -> Result<Option<String>> {
        let lines = self.collections.history(id).await?;
        Ok(lines.iter().rev().find_map(|line| match line.topic.as_str() {
            MOVED => line.data["to"].as_str().map(str::to_string),
            CREATED => line.data["status"].as_str().map(str::to_string),
            _ => None,
        }))
    }
}

//! What one card cost, by round, for whoever may read the card.
//!
//! Laid out once here (`agents::spent`, a function of plain values with
//! its cases) for the command line and the client to write down, as a
//! comparison is. What it is read from is kept already and is asked by
//! keys: the card, its board's columns, its history, and its turns by
//! the conversations an agent holds about it. Nothing is kept for it.

use agents::spent::{self, Said, Turn};
use kernel::{Principal, Result};
use serde_json::Value;

use crate::state::App;

/// What the card `id` cost, by round, as `who` may be told it: whoever
/// may read the board, and the card's table where it is kept private.
/// A turn of a channel kept to its members that `who` does not read is
/// laid out with its figures, and without what it chose.
pub(crate) async fn of(app: &App, id: &str, who: &Principal) -> Result<Value> {
    who.require("board:read")?;
    let card = app.studio.board.get(id).await?;
    let (collection, _) = app.studio.collections.record(id).await?;
    collection.may_read(who)?;
    let columns = app.studio.board.columns(&card.project).await?;
    let said: Vec<Said> = app
        .studio
        .collections
        .history(id)
        .await?
        .into_iter()
        .map(|line| Said {
            seq: line.seq,
            topic: line.topic,
            ts: line.ts,
            who: line.who,
            said: line.said,
            data: line.data,
        })
        .collect();
    let turns: Vec<Turn> = app
        .agents
        .turns_in(&spent::conversations(id), who)
        .await?
        .into_iter()
        .filter_map(|(thread, turn)| {
            let (does, way) = (spent::does(&thread, id)?, spent::way(&thread, id));
            let took = turn.started.zip(turn.finished).map(|(began, ended)| ended - began);
            let (nth, begun) =
                (turn.nth(), spent::begun_by(turn.asking.as_deref(), turn.asked_by.as_deref()));
            Some(Turn {
                id: turn.id,
                agent: turn.slot,
                does,
                status: turn.status,
                chose: turn.decision,
                sent: turn.input_tokens,
                wrote: turn.output_tokens,
                cost_usd: turn.cost_usd,
                cost_said: turn.cost_said,
                asked: turn.created,
                nth,
                begun,
                took_ms: took,
                kept_from: turn.kept_from,
                way,
            })
        })
        .collect();
    Ok(spent::laid(id, &spent::rounds(&columns, &said, turns)))
}

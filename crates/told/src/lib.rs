//! The parts of a line a blueprint told (`kernel::parts`), as whoever's
//! line it is reads them now, and a card such a part lists passed or
//! failed from there. It came out of `api` when that crate was at its
//! ceiling, and serves nothing: the routes there ask it
//! (`/inbox/{id}/parts`, `/inbox/{id}/parts/{part}/cards/{card}`).
//!
//! A line keeps what it was told with; what a part comes to is asked of
//! the board as it is when it is read: the cards a part of cards lists,
//! each with what was said on it since it came where it is listed, and
//! whether whoever reads may pass or fail it. A part of a column kept
//! which cards were there as it was told, and lists those wherever each
//! is now, one that is gone said to be gone; one that kept none (a line
//! from before, a part of a label) lists what is there now. Nothing is read for them
//! that they may not read otherwise: the board, by `board:read`. Rows of a
//! table, runs and measurements are read the same way ([`read`]), each
//! handed with the tab of the line's page it is in where it says one.
//!
//! Passing and failing are the card's, not the line's: a card failed is
//! moved through the board's one door with why as what was said as it
//! was moved, which is what its agent is told when it is next given the
//! card; one passed is approved by whoever passed it (`card.approved`, a
//! line of its history and no comment: words are said on a card only
//! where somebody writes some), and then moved where its part says, or
//! has what the part says of its column pressed on it through the
//! board's one door, or nothing more.
//! What each comes to, and what is refused (a card listed no longer, one
//! failed without why where why is asked, an agent's own work), is
//! `kernel::parts::press`, with its cases.

pub mod actions;
pub mod asks;
pub mod events;
pub mod goes;
pub mod read;

use journal::Journal;
use kernel::parts::{Cards, Done, Part, Pressing, Then, Verdict};
use kernel::{Error, Principal, Result, Source};
use serde_json::{Value, json};
use studio::Studio;

pub use asks::{Ask, Asks, Came};

/// What this crate says, among what the factory describes, and what a
/// blueprint may ask of it (`approval.ask`), over the factory's journal and
/// its studio.
pub fn register(registry: &mut conductor::Registry, journal: &Journal, studio: &Studio) {
    registry.emits(events::events());
    actions::register(registry, journal, studio);
}

/// The parts of one line of whoever asks, each as it comes to now.
pub async fn parts(
    (studio, journal, engine): (&Studio, &Journal, &conductor::Engine),
    who: &Principal,
    id: &str,
) -> Result<Value> {
    let line = studio.channels.inbox.one(who, id).await?;
    let mut shown = Vec::new();
    for (tab, part) in kernel::parts::tabbed(line.parts.as_ref().unwrap_or(&Value::Null)) {
        let mut said = match part {
            Part::Links(links) => json!({ "kind": "links", "links": links }),
            Part::Message { channel, message } => {
                let mut said = json!({ "kind": "message", "channel": channel, "message": message });
                // Where it is, as whoever reads the line may read it: the thread it is in or hangs a thread, or nothing of it.
                let read = studio.channels.may_read_message(&message, who).await;
                match match read {
                    Ok(reader) => studio.channels.message(&reader, &message).await,
                    Err(refused) => Err(refused),
                } {
                    // A message a thread hangs from opens that thread too.
                    Ok(found) => {
                        let hangs = (found.replies > 0).then(|| found.id.clone());
                        said["thread"] = json!(found.thread.or(hangs));
                    }
                    Err(refused) => said["refused"] = json!(refused.message),
                }
                said
            }
            // What somebody asked to be changed, as it stands now for whoever reads it.
            Part::Change { project, ask } => match Asks::new(journal, studio).shown(who, &ask).await {
                Ok(shown) => json!({ "kind": "change", "project": project, "change": shown }),
                Err(refused) => {
                    json!({ "kind": "change", "project": project, "ask": ask, "refused": refused.message })
                }
            },
            // A run that waits to be let go on, as it stands now for whoever reads it.
            Part::Go { project, ask } => {
                match Asks::new(journal, studio).reading(engine).shown(who, &ask).await {
                    Ok(shown) => json!({ "kind": "go", "project": project, "go": shown }),
                    Err(refused) => {
                        json!({ "kind": "go", "project": project, "ask": ask, "refused": refused.message })
                    }
                }
            }
            Part::Cards(cards) => {
                let mut said = json!({
                    "kind": "cards", "project": cards.project, "column": cards.column, "label": cards.label,
                    "fail_to": cards.fail_to, "pass_to": cards.pass_to, "pass_press": cards.pass_press,
                    "reason": cards.reason, "kept": cards.kept.is_some(),
                });
                match listed(studio, who, &cards).await {
                    Ok(listed) => said["cards"] = json!(listed),
                    // Whoever may not read the board is shown none of it, and told why.
                    Err(refused) => said["refused"] = json!(refused.message),
                }
                said
            }
            // Rows of a table, as whoever reads the line may read the table.
            Part::Rows(rows) => {
                let mut said = json!({
                    "kind": "rows", "project": rows.project, "table": rows.table, "ids": rows.ids,
                    "saying": rows.saying,
                });
                read_into(&mut said, read::rows(studio, who, &rows).await);
                said
            }
            Part::Runs(runs) => {
                let mut said = json!({ "kind": "runs", "project": runs.project, "most": runs.most });
                match &runs.of {
                    kernel::parts::RunsOf::Instance(id) => said["instance"] = json!(id),
                    kernel::parts::RunsOf::Key(key) => said["key"] = json!(key),
                    kernel::parts::RunsOf::Environment(name) => said["environment"] = json!(name),
                }
                read_into(&mut said, read::runs(engine, who, &runs).await);
                said
            }
            Part::Measures(measures) => {
                let mut said = json!({ "kind": "measures", "last": measures.last });
                read_into(&mut said, read::measures(journal, who, &measures).await);
                said
            }
        };
        // Which tab of the line's page it is in, where it says one.
        if !tab.is_empty() {
            said["tab"] = json!(tab);
        }
        shown.push(said);
    }
    Ok(json!({ "line": line.id, "parts": shown }))
}

/// What a part comes to put beside what it says of itself, or why whoever
/// reads it is shown none of it.
fn read_into(said: &mut Value, read: Result<Value>) {
    match read {
        Ok(Value::Object(read)) => {
            said.as_object_mut().into_iter().for_each(|said| said.extend(read.clone()))
        }
        Ok(_) => {}
        Err(refused) => said["refused"] = json!(refused.message),
    }
}

/// The cards a part lists as the board is now, each as it is shown with
/// the part: what was said on it since it came there, and whether `who`
/// may pass or fail it, with why not.
async fn listed(studio: &Studio, who: &Principal, cards: &Cards) -> Result<Vec<Value>> {
    who.require("board:read")?;
    let board = &studio.board;
    let mut shown = Vec::new();
    let items = match (&cards.kept, cards.column.is_empty()) {
        (Some(kept), _) => {
            let mut items = Vec::new();
            for id in kept {
                match board.get(id).await {
                    Ok(item) if item.project == cards.project => items.push(item),
                    Ok(_) => shown.push(gone(id)),
                    Err(err) if err.kind == kernel::ErrorKind::NotFound => shown.push(gone(id)),
                    Err(err) => return Err(err),
                }
            }
            items
        }
        (None, false) => board.column(&cards.project, &cards.column).await?,
        (None, true) => board.list(&cards.project).await?,
    };
    let kept = cards.kept.is_some();
    for item in items.iter().filter(|item| kept || cards.lists(&item.status, &item.labels)) {
        let lines = studio::said::lines(&studio.collections.history(&item.id).await?);
        let thread = kernel::parts::thread(&lines, &cards.column);
        let pressing = pressing_of(who, item, &lines, None, "");
        let may = match who.require("board:write") {
            Ok(()) => kernel::parts::may(cards, &pressing),
            Err(refused) => Err(refused.message),
        };
        let thread: Vec<Value> = thread
            .iter()
            .map(|line| json!({ "who": line.who, "actor": line.actor, "words": line.words, "ts": line.ts, "topic": line.topic }))
            .collect();
        let mut card = json!({
            "id": item.id, "title": item.title, "status": item.status, "assignee": item.assignee,
            "labels": item.labels, "thread": thread, "may": may.is_ok(),
        });
        if let Err(why) = may {
            card["why_not"] = json!(why);
        }
        shown.push(card);
    }
    // In the order the line was told of them, a card that is gone among them.
    if let Some(kept) = &cards.kept {
        shown.sort_by_key(|card| kept.iter().position(|id| card["id"] == id.as_str()));
    }
    Ok(shown)
}

/// A card a part kept that is on its board no longer.
fn gone(id: &str) -> Value {
    json!({ "id": id, "gone": true, "title": "", "status": "", "assignee": null, "labels": [], "thread": [],
            "may": false, "why_not": format!("{id} is no longer on the board: it was deleted, or is in the Trash") })
}

fn pressing_of(
    who: &Principal,
    item: &studio::Item,
    lines: &[kernel::parts::Line],
    verdict: Option<Verdict>,
    reason: &str,
) -> Pressing {
    Pressing {
        id: item.id.clone(),
        verdict,
        reason: reason.to_string(),
        by: who.actor(),
        status: item.status.clone(),
        labels: item.labels.clone(),
        assignee: item.assignee.clone(),
        worked: kernel::parts::worked(lines),
    }
}

/// Passes or fails a card the line's part lists: `verdict` (`pass` or
/// `fail`) and `reason`, why, which failing asks for unless the part says
/// it does not, as `body` says them. `actor` is who is said to have done
/// it (`state::Said` in `api`: the key, where it was a key). The card as it
/// then is, and what was done.
pub async fn press(
    (studio, journal): (&Studio, &Journal),
    (who, actor): (&Principal, &str),
    (id, part, card): (&str, usize, &str),
    body: &Value,
) -> Result<Value> {
    let line = studio.channels.inbox.one(who, id).await?;
    let parts = kernel::parts::of(line.parts.as_ref().unwrap_or(&Value::Null));
    let Some(Part::Cards(cards)) = part.checked_sub(1).and_then(|at| parts.get(at)) else {
        return Err(Error::not_found(format!(
            "the line `{id}` has no part {part} that lists cards: its parts are counted from 1"
        )));
    };
    // Whether whoever asks may move a card at all is said first, in the
    // words it always is; only somebody who may is told of the card.
    who.require("board:write")?;
    let item = studio.board.get(card).await?;
    if item.project != cards.project {
        return Err(Error::not_found(format!("`{card}` is not a card of `{}`", cards.project)));
    }
    let verdict = Verdict::read(body.get("verdict").and_then(Value::as_str).unwrap_or_default())
        .map_err(Error::invalid)?;
    let reason = body.get("reason").and_then(Value::as_str).unwrap_or_default();
    let lines = studio::said::lines(&studio.collections.history(&item.id).await?);
    let done = kernel::parts::press(cards, &pressing_of(who, &item, &lines, Some(verdict), reason))
        .map_err(Error::invalid)?;
    let did = match &done {
        Done::Moved { to, said } => {
            let said = Some(said.as_str()).filter(|said| !said.is_empty());
            studio.board.move_said(card, to, (None, said, 0), who, actor).await?;
            json!({ "moved": to, "said": said })
        }
        Done::Passed { then } => {
            // Approved by whoever passed it, as a review that passes says it: a line of its history.
            let data = json!({ "card": card, "project": cards.project, "by": who.name, "words": "" });
            journal.emit("card.approved", Source::actor(actor), data).await?;
            match then {
                Then::Nothing => json!({ "approved": true }),
                Then::Move(to) => {
                    studio.board.move_said(card, to, (None, None, 0), who, actor).await?;
                    json!({ "approved": true, "moved": to })
                }
                Then::Press(action) => {
                    studio.board.press(card, action, false, who, (None, Some(actor))).await?;
                    json!({ "approved": true, "pressed": action })
                }
            }
        }
    };
    let verdict = if verdict == Verdict::Pass { "pass" } else { "fail" };
    Ok(json!({ "card": studio.board.get(card).await?, "verdict": verdict, "did": did }))
}

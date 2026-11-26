//! What a card keeps beyond where it is, and what follows from it, asked
//! as actions by what looks after the cards of a board (`std/cards`). What
//! is decided is a function of plain values and the kernel's
//! (`kernel::cards`), offered here under the names it always had; what is
//! here reads the rows and registers the actions that ask it.

use blueprint::{ActionSpec, ValueType as T};
use conductor::{ActionError, Registry};
use kernel::ErrorKind;
use serde_json::{Value, json};

pub(crate) use kernel::cards::AFTER;
pub use kernel::cards::{
    After, Arrived, Awaited, Came, Columns, Kept, Row, Said, after, came, held, said, turn, waiting_for,
    waits_on_others,
};

use crate::actions::{add, project_of, text};
use crate::{Item, Studio};

/// A project's cards as [`held`] reads them, and whether its board keeps
/// what a card waits for. Where it keeps it, [`held`] takes only the cards
/// that say they wait on others, and only those are read
/// ([`crate::board::Board::waiting`]): the board read whole for each card
/// that was finished cost the more the more cards there were beside it.
async fn rows(studio: &Studio, project: &str) -> Result<(Vec<Row>, bool), ActionError> {
    let keeps = keeps_waiting(studio, project).await;
    let cards = match keeps {
        true => studio.board.waiting(project).await?,
        false => studio.board.list(project).await?,
    };
    let rows = cards.iter().map(|card| Row { id: card.id.clone(), kept: Kept::of(&card.status, &card.more) });
    Ok((rows.collect(), keeps))
}

/// Whether a project's board declares the field a card says what it
/// waits for in.
async fn keeps_waiting(studio: &Studio, project: &str) -> bool {
    match studio.collections.get(project, crate::board::BOARD).await {
        Ok(board) => board.fields.iter().any(|field| field.name == "waiting"),
        Err(_) => false,
    }
}

fn kept_of(card: &Item) -> Kept {
    Kept::of(&card.status, &card.more)
}

pub(crate) fn register(registry: &mut Registry, studio: &Studio) {
    add(
        registry,
        studio,
        ActionSpec::new(
            "board.came",
            "Say what a card's being moved means for what the card keeps: that it was taken to be worked, sent \
             back (to where cards are worked, or from where work is looked over to any column the board has \
             before that one), finished, or nothing; whether a question on it is settled by the move; and, of \
             one that is finished, which cards wait on it.",
        )
        .deciding()
        .required("id", T::String, "the card")
        .required("from", T::String, "the column it was moved from")
        .required("to", T::String, "the column it was moved to")
        .required("by_hand", T::Bool, "whether somebody moved it, and no blueprint did")
        .arg("uncounted", T::Bool, "the blueprint that moved it said it sends it back for something other than a verdict on its work: it is not counted")
        .required("worked", T::String, "the column cards are worked in")
        .required("looked", T::String, "the column work is looked over in: a card put from there to where cards are worked was sent back")
        .required("done", T::String, "the column a card is finished in")
        .capability("board:read")
        .output("came", &format!("{}: first put where cards are worked; sent back to be worked again; finished; or anything else", Came::words()))
        .output("settles", "whether what was asked on the card is settled by the move: somebody moved it while a question was open")
        .output("rounds", "how many times it has now been sent back, where `came` is `back`")
        .output("rests", "whether it is finished and still says it waits for something")
        .output("holds", "where it is finished, the cards that wait on it, by id")
        .idempotent(),
        |studio, _cx, args| async move {
            let word = |key: &str| text(&args, key).unwrap_or_default();
            let card = studio.board.get(&word("id")).await?;
            let (worked, looked, done) = (word("worked"), word("looked"), word("done"));
            // The columns before where work is looked over, on this board;
            // none where the board has no such column.
            let board = studio.board.columns(&card.project).await.unwrap_or_default();
            let before: Vec<&str> = match board.iter().position(|column| *column == looked) {
                Some(at) => board[..at].iter().map(String::as_str).collect(),
                None => Vec::new(),
            };
            let columns = Columns { worked: &worked, looked: &looked, done: &done, before: &before };
            let flag = |key: &str| args.get(key).and_then(Value::as_bool).unwrap_or(false);
            let arrived = came(&word("from"), &word("to"), flag("by_hand"), flag("uncounted"), &kept_of(&card), columns);
            let holds = match arrived.came {
                Came::Finished => {
                    let (rows, keeps) = rows(&studio, &card.project).await?;
                    held(&rows, &done, Some(&card.id), keeps)
                }
                _ => Vec::new(),
            };
            Ok(json!({
                "came": arrived.came.word(),
                "settles": arrived.settles,
                "rounds": arrived.rounds,
                "rests": arrived.rests,
                "holds": holds,
            }))
        },
    );

    add(
        registry,
        studio,
        ActionSpec::new(
            "board.said",
            "Say whether what was said on a card answers what was asked on it: a question is open, and it was \
             not whoever asked who spoke.",
        )
        .deciding()
        .required("id", T::String, "the card")
        .required("by", T::String, "who spoke, as `user:<id>` or `agent:<slot>`")
        .capability("board:read")
        .output("came", &format!("{}: it answers what was asked; or nothing was asked, or the asker spoke", Said::words()))
        .output("asker", "who asked, where the card says")
        .idempotent(),
        |studio, _cx, args| async move {
            let card = studio.board.get(&text(&args, "id").unwrap_or_default()).await?;
            let kept = kept_of(&card);
            let came = said(&text(&args, "by").unwrap_or_default(), &kept);
            Ok(json!({ "came": came.word(), "asker": kept.asker }))
        },
    );

    add(
        registry,
        studio,
        ActionSpec::new(
            "board.waits",
            "Say what a card says while it waits its turn where a limited number are worked at once, and whether \
             what waits is a card of this board at all.",
        )
        .deciding()
        .required("id", T::String, "what waits: a card, if it is one")
        .arg("project", T::String, "which project's board (default: this instance's)")
        .arg("why", T::String, "why nothing is being started for now, where that is why it waits")
        .arg("limit", T::Int, "how many are worked at once")
        .capability("board:read")
        .output("mine", "whether it is a card of this board")
        .output("says", "what the card is to say it waits for")
        .idempotent(),
        |studio, cx, args| async move {
            let project = project_of(&cx, &args)?;
            let mine = match studio.board.get(&text(&args, "id").unwrap_or_default()).await {
                Ok(card) => card.project == project,
                // What waits may be something else than a card, or a card that is gone.
                Err(gone) if gone.kind == ErrorKind::NotFound => false,
                Err(other) => return Err(other.into()),
            };
            let limit = args.get("limit").and_then(Value::as_i64).unwrap_or(0);
            Ok(json!({ "mine": mine, "says": turn(&text(&args, "why").unwrap_or_default(), limit) }))
        },
    );

    add(
        registry,
        studio,
        ActionSpec::new(
            "board.after",
            "Say whether the cards a card waits on are done, and what follows: what it is to say it waits for, \
             whether that comes off it, and whether whoever would begin it is to be told.",
        )
        .deciding()
        .required("id", T::String, "the card")
        .required("done", T::String, "the column a card is finished in")
        .arg("asked", T::Bool, "whether something that would begin the card asked, and is owed its answer (default: no, the card is only looked at again)")
        .capability("board:read")
        .output("came", &format!("{}: nothing it waits on is left; or some are not done", Awaited::words()))
        .output("left", "the cards it waits on that are not done, by id: one that is done, or that names no card of the project any more, is not waited for")
        .output("says", "what it is to say it waits for, where it waits")
        .output("lifts", "whether it said it waited on cards and none is left")
        .output("news", "whether its row is to say `says` and does not say it now")
        .output("answers", "whether whoever would begin it is to be told that nothing is left")
        .idempotent(),
        |studio, _cx, args| async move {
            let asked = args.get("asked").and_then(Value::as_bool).unwrap_or(false);
            let done = text(&args, "done").unwrap_or_default();
            let card = match studio.board.get(&text(&args, "id").unwrap_or_default()).await {
                Ok(card) => card,
                // Looked at again, a card that is gone waits for nothing
                // and nobody waits for it; asked for, it is not there.
                Err(gone) if gone.kind == ErrorKind::NotFound && !asked => {
                    return Ok(json!({
                        "came": Awaited::Clear.word(), "left": [], "says": "",
                        "lifts": false, "news": false, "answers": false,
                    }));
                }
                Err(other) => return Err(other.into()),
            };
            let kept = kept_of(&card);
            let mut left = Vec::new();
            for id in &kept.after {
                match studio.board.get(id).await {
                    Ok(other) if other.project == card.project && other.status != done => left.push(id.clone()),
                    // Done, another project's, or naming no card any more.
                    Ok(_) => {}
                    Err(gone) if gone.kind == ErrorKind::NotFound => {}
                    Err(other) => return Err(other.into()),
                }
            }
            let keeps = keeps_waiting(&studio, &card.project).await;
            let after = after(asked, &left, &kept, keeps);
            Ok(json!({
                "came": after.came.word(),
                "left": left,
                "says": after.says,
                "lifts": after.lifts,
                "news": after.news,
                "answers": after.answers,
            }))
        },
    );

    add(
        registry,
        studio,
        ActionSpec::new(
            "board.held",
            "The cards of a board that wait on other cards: those not finished that say so, which is what says \
             they were asked whether they may begin.",
        )
        .deciding()
        .required("done", T::String, "the column a card is finished in")
        .arg("project", T::String, "which project's board (default: this instance's)")
        .arg("by", T::String, "only those that wait on this card")
        .capability("board:read")
        .output("ids", "the cards, by id, in the order of the board")
        .output("count", "how many there are")
        .idempotent(),
        |studio, cx, args| async move {
            let project = project_of(&cx, &args)?;
            let (rows, keeps) = rows(&studio, &project).await?;
            let by = text(&args, "by");
            let ids = held(&rows, &text(&args, "done").unwrap_or_default(), by.as_deref(), keeps);
            Ok(json!({ "count": ids.len(), "ids": ids }))
        },
    );
}

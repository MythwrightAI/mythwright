//! The one door something a card's column offers is pressed by (a
//! route, a command, the bot, a blueprint's step), as whoever presses,
//! and a card said to be held for it. What a column offers is the rows of
//! a table that is an action's (`models::ACTION`) and sits beside the
//! board; what pressing comes to is the kernel's (`kernel::pressed`). A
//! press, and that a card was said to be held, are kept on its row
//! (`pressed`) in the write that says so, once for each coming of it.

use kernel::pressed::{Action, Asked, Pressed, Pressing, Refused, did, held_says, holding, pressing, rows};
use kernel::{Error, Principal, Result};
use serde_json::{Value, json};

use crate::board::{Board, Item, Through, item, told};
use crate::collections::{Collection, Record, Say};
use crate::models::{ACTION, Card, Moved, PRESSED};

/// What a press came to.
#[derive(Clone, Debug, serde::Serialize)]
pub struct Done {
    /// The card as it now is.
    pub card: Item,
    /// Whether anything was written: pressing what stands pressed, or
    /// taking back what is not, writes nothing.
    pub done: bool,
    /// The card was held, and this was the last of what held it.
    pub released: bool,
    /// What still holds it, by what each is called.
    pub waits: Vec<String>,
}

/// The coming of a card to where it is: the count of its moves.
fn coming(card: &Card) -> u64 {
    Moved::read(&card.moved).map_or(0, |moved| moved.count)
}

impl Board {
    /// What can be pressed on the cards of `board`, of every column: the
    /// rows of each table of its project that is an action's and sits
    /// beside it, in the order the tables and their rows are listed.
    pub async fn actions_of(&self, board: &Collection) -> Result<Vec<Action>> {
        let mut found = Vec::new();
        for table in self.collections.list(Some(&board.project)).await? {
            let beside = table.shown.beside.as_deref() == Some(board.name.as_str());
            if beside && table.model.as_deref() == Some(ACTION.name) {
                let records = self.collections.records(&table).await?;
                let read: Vec<Value> =
                    records.iter().map(|record| json!({ "id": record.id, "data": record.data })).collect();
                found.extend(rows(&read));
            }
        }
        Ok(found)
    }

    /// Presses `action` on a card, or takes a press of it back (`back`),
    /// as `by` and through `through` where a blueprint's step does it;
    /// `said`, the key it was done with, where somebody used one. Who may
    /// is what the action's row says, of whoever presses: never of
    /// whoever applied what asks.
    pub async fn press(
        &self,
        id: &str,
        action: &str,
        back: bool,
        by: &Principal,
        (through, said): (Option<&Through>, Option<&str>),
    ) -> Result<Done> {
        by.require("board:write")?;
        let _turn = self.collections.changing(id).await;
        let (board, mut record) = self.card(id).await?;
        // A press changes a card, and of a board that steers another
        // table it is asked what any change of one is, as a move is.
        self.collections.steered(&board, by).await?;
        let actions = self.actions_of(&board).await?;
        let card = Card::read(record.data.clone());
        let actor = said.map(str::to_string).unwrap_or_else(|| by.actor());
        let kind = actor.split(':').next().unwrap_or_default();
        let (at, column) = (coming(&card), card.status.as_str());
        let asked = Asked {
            item: id,
            column,
            coming: at,
            word: action,
            back,
            by: (kind, &actor, &by.name),
            at: self.journal.now_ms(),
        };
        let came = pressing(&actions, &Pressed::read(&card.pressed), asked, |scope| by.can(scope)).map_err(
            |refused| match refused {
                Refused::Unoffered(why) => Error::not_found(why),
                Refused::Unfit(why) => Error::invalid(why),
                Refused::Denied(why) => Error::denied(why),
            },
        )?;
        if board.field(PRESSED).is_none_or(|field| field.kind != "list") {
            return Err(Error::invalid(format!(
                "the board of {} keeps nothing of what is pressed on a card: apply `std/board` to it again",
                board.project
            )));
        }
        let Pressing { action, now, released, waits: left } = came;
        let Some(now) = now else {
            return Ok(Done { card: item(record), done: false, released, waits: left });
        };
        record.data.insert(PRESSED.into(), json!(now.kept()));
        let (name, title, words) = (action.name.clone(), action.title.clone(), did(action, back));
        let still = left.clone();
        let pressed = move |record: &Record| {
            let card = item(record.clone());
            let data = json!({
                "id": card.id, "project": card.project, "status": card.status, "action": name,
                "title": title, "coming": at, "back": back, "did": words, "released": released, "name": by.name,
                "waits": still,
            });
            vec![Say::new("board.item.pressed", crate::board::told_as(by, through, said, data))]
        };
        let record = self.collections.put_saying(&board, record, pressed).await?;
        Ok(Done { card: item(record), done: true, released, waits: left })
    }

    /// Says that a card is held where it is until what its column offers
    /// is pressed, once for this coming of it: nothing is written or said
    /// where it was said already, where it has gone on, or where nothing
    /// holds it after all (it was pressed since whoever asks looked).
    /// Whether anything was said.
    pub async fn hold(&self, id: &str, by: &Principal, through: Option<&Through>) -> Result<bool> {
        by.require("board:write")?;
        let _turn = self.collections.changing(id).await;
        let (board, mut record) = self.card(id).await?;
        let actions = self.actions_of(&board).await?;
        let card = Card::read(record.data.clone());
        let at = coming(&card);
        let kept = board.field(PRESSED).is_some_and(|field| field.kind == "list");
        let held = holding(&actions, &card.status, &Pressed::read(&card.pressed), at).filter(|_| kept);
        let Some((now, left)) = held else {
            return Ok(false);
        };
        let says = held_says(id, &card.status, &left);
        let titles: Vec<String> = left.iter().map(|one| one.title.clone()).collect();
        let who: Vec<String> = left.iter().map(|one| one.who_may()).collect();
        record.data.insert(PRESSED.into(), json!(now.kept()));
        let held = move |record: &Record| {
            let card = item(record.clone());
            let data = json!({
                "id": card.id, "card": card.id, "project": card.project, "status": card.status,
                "coming": at, "waits": titles, "who": who, "says": says,
            });
            vec![Say::new("board.item.held", told(by, through, data))]
        };
        self.collections.put_saying(&board, record, held).await?;
        Ok(true)
    }
}

/// Adds what a blueprint does with what is pressed on a card.
pub(crate) fn register(registry: &mut conductor::Registry, studio: &crate::Studio) {
    use blueprint::{ActionSpec, ValueType as T};

    use crate::actions::{add, text, through};
    add(
        registry,
        studio,
        ActionSpec::new(
            "board.press",
            "Press something a card's column offers on it, as a person would press its button, or take a press \
             back. Who may is what its row says, of whoever applied this.",
        )
        .example(kernel::asked::BOARD_PRESS)
        .required("id", T::String, "the card")
        .required("action", T::String, "what is pressed: its name, or what it is called, as its row says")
        .arg("back", T::Bool, "the press is taken back, while the card is where it was pressed")
        .capability("board:write")
        .output("done", "whether anything was written: pressing what stands pressed writes nothing")
        .output("released", "the card was held, and this was the last of what held it")
        .output("waits", "what still holds it, by what each is called")
        .idempotent(),
        |studio, cx, args| async move {
            let (id, action) = (text(&args, "id").unwrap_or_default(), text(&args, "action").unwrap_or_default());
            let back = args.get("back").and_then(Value::as_bool).unwrap_or(false);
            let through = through(&cx);
            let pressing = studio.board.press(&id, &action, back, cx.principal(), (Some(&through), None)).await?;
            Ok(json!({ "done": pressing.done, "released": pressing.released, "waits": pressing.waits }))
        },
    );
    add(
        registry,
        studio,
        ActionSpec::new(
            "board.hold",
            "Say that a card waits where it is for something its column offers to be pressed, once for each \
             time it comes there; nothing, where nothing holds it.",
        )
        .required("id", T::String, "the card")
        .capability("board:write")
        .output("said", "whether it was said now: not where it was said already for this time it came")
        .idempotent(),
        |studio, cx, args| async move {
            let id = text(&args, "id").unwrap_or_default();
            let through = through(&cx);
            Ok(json!({ "said": studio.board.hold(&id, cx.principal(), Some(&through)).await? }))
        },
    );
}

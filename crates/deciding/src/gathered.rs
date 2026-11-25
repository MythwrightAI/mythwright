//! Tickets that are things there already, asked as actions:
//! `tickets.named`, `tickets.fit` and `tickets.placed`.
//!
//! What each works out is a function of plain values and is the kernel's
//! ([`kernel::gathered`]), offered here under the names it always had;
//! this is how a blueprint asks it.
//!
//! It was the studio's (`studio::gathered`), and came here with nothing of a
//! table in it: a fleet's blueprints ask it as they ask the rest here.

use blueprint::{ActionSpec, ValueType as T};
use conductor::{ActionError, Registry};
use kernel::BoxFut;
use serde_json::{Map, Value, json};

pub use kernel::gathered::*;

/// What an action was handed for the cells to read, with what each is
/// usually called where nothing was said.
fn cells(args: &Map<String, Value>) -> Cells {
    let mut cells = Cells::default();
    let said = |key: &str, cell: &mut String| {
        if let Some(name) = args.get(key).and_then(Value::as_str).filter(|name| !name.is_empty()) {
            *cell = name.to_string();
        }
    };
    said("titled", &mut cells.title);
    said("says", &mut cells.body);
    said("waits", &mut cells.waits);
    said("within", &mut cells.within);
    said("state", &mut cells.state);
    said("labels", &mut cells.labels);
    said("over", &mut cells.over);
    said("begins", &mut cells.begins);
    cells
}

/// The arguments every one of these takes for how a row is read.
fn reading(spec: ActionSpec) -> ActionSpec {
    spec.arg("waits", T::String, "the cell a row keeps the ids of what it waits on in (`after`, if not said)")
        .arg("within", T::String, "the cell a row keeps what it is part of in (`part_of`, if not said)")
        .arg("state", T::String, "the cell that says where a row is (`status`, if not said)")
        .arg("over", T::String, "what that cell says of a row that is done; no row is done, if not said")
}

pub(crate) fn register(registry: &mut Registry) {
    let list = |args: &Map<String, Value>, key: &str| {
        args.get(key).and_then(Value::as_array).cloned().unwrap_or_default()
    };
    let names = move |args: &Map<String, Value>, key: &str| words(args.get(key));
    let text = |args: &Map<String, Value>, key: &str| {
        args.get(key).and_then(Value::as_str).unwrap_or_default().to_string()
    };

    let spec = reading(
        ActionSpec::new(
            "tickets.named",
            "Say, for whoever plans, the rows there already that a plan is to put in order: those named, and with a \
             label every row that carries it and is not done.",
        )
        .required("rows", T::List, "the table's rows, as `rows.read` gives them")
        .arg("ids", T::List, "the rows to put in order, by id")
        .arg("label", T::String, "a label: every row that carries it and is not done is one of them too")
        .arg("fresh", T::Bool, "whether the label is still to be looked for; once what it came to is written down, it is not")
        .arg("titled", T::String, "the cell that says what a row is called (`title`, if not said)")
        .arg("says", T::String, "the cell that says what is wanted (`body`, if not said)")
        .arg("labels", T::String, "the cell a row keeps its labels in (`labels`, if not said)"),
    )
    .output("ids", "those that are there: the ones named, in that order, and then the ones the label brought")
    .output("count", "how many those are")
    .output("listed", "their ids as a sentence lists them")
    .output("missing", "those named that are no row of the table")
    .output("shown", "each of them in words: its id, where it is, what it is called and what is wanted, what it waits on and what it is part of")
    .output("fix", "whether the label brought any that were not named, so that what is named is to be written down")
    .idempotent();
    registry.add_fn(spec, move |_cx, args| -> BoxFut<'static, conductor::ActionResult> {
        Box::pin(async move {
            let cells = cells(&args);
            let rows = things(&list(&args, "rows"), &cells);
            let fresh = args.get("fresh").and_then(Value::as_bool).unwrap_or(true);
            let label = if fresh { text(&args, "label") } else { String::new() };
            let named = named(&rows, &names(&args, "ids"), &label, &cells);
            Ok(json!({
                "count": named.ids.len(),
                "listed": listed(&named.ids),
                "ids": named.ids,
                "missing": named.missing,
                "shown": named.shown,
                "fix": named.fix,
            }))
        })
    });

    let spec = reading(
        ActionSpec::new(
            "tickets.fit",
            "Say whether a plan's tickets fit what is there, before anything is made of them: a ticket may be a row \
             there already (`card`, its id) of those the plan was asked to put in order, and waits on places in the \
             list or on rows by id.",
        )
        .arg("read", T::String, "what reading the plan for its tickets came to, as `tickets.read` says it")
        .arg("unread", T::String, "why they could not be read, where they could not")
        .arg("tickets", T::List, "the tickets, as they were read")
        .arg("may", T::List, "the rows the plan was asked to put in order, by id: a ticket may be one of these and no other row")
        .required("rows", T::List, "the table's rows, as `rows.read` gives them")
        .required("of", T::String, "what the tickets are to be part of"),
    )
    .output(
        "came",
        &format!(
            "{}: they can be taken as written; something is wrong and nothing is to be written; or the plan lists \
             none and gathers nothing, and is one piece of work",
            Fit::words()
        ),
    )
    .output("why", "what is wrong, a sentence for each thing")
    .output("new", "the tickets that are new, in order, each with its `title` and `body`: a row is to be made for each")
    .idempotent();
    registry.add_fn(spec, move |_cx, args| -> BoxFut<'static, conductor::ActionResult> {
        Box::pin(async move {
            let cells = cells(&args);
            let of = text(&args, "of");
            if of.is_empty() {
                return Err(ActionError::new(
                    "`of` is what the tickets are to be part of, and nothing is said",
                ));
            }
            let rows = things(&list(&args, "rows"), &cells);
            let fitted = fit(
                &text(&args, "read"),
                &text(&args, "unread"),
                &list(&args, "tickets"),
                &names(&args, "may"),
                &rows,
                &of,
                &cells,
            );
            Ok(json!({ "came": fitted.came.word(), "why": fitted.why, "new": fitted.new }))
        })
    });

    let spec = reading(
        ActionSpec::new(
            "tickets.placed",
            "Say what every thing of a plan that fits waits on and is part of, once its new tickets have come to \
             rows: those, the rows it named, and the rows it was asked to put in order and did not name, which are \
             taken as they stand.",
        )
        .arg("tickets", T::List, "the tickets, as they were read")
        .arg("made", T::List, "what each new ticket came to, in their order, each with its `id`")
        .arg("may", T::List, "the rows the plan was asked to put in order, by id")
        .required("rows", T::List, "the table's rows as they were when the plan was held against them")
        .required("of", T::String, "what they are part of")
        .arg("begins", T::String, "what the state cell says of a row that has not been begun: only such rows are made ready; every row, if not said"),
    )
    .output("links", "for each in turn: its `id`, `after` (the ids of what it waits on) and what it is part of, under the name of the cell that keeps that")
    .output("ids", "their ids, in that order")
    .output("listed", "the ids as a sentence lists them")
    .output("count", "how many they are")
    .output("ready", "those to be made ready: the new ones, and of those there already the ones that have not been begun")
    .output("told", "what changed for the rows there already, in sentences; empty where nothing did")
    .idempotent();
    registry.add_fn(spec, move |_cx, args| -> BoxFut<'static, conductor::ActionResult> {
        Box::pin(async move {
            let cells = cells(&args);
            let of = text(&args, "of");
            if of.is_empty() {
                return Err(ActionError::new("`of` is what the tickets are part of, and nothing is said"));
            }
            let rows = things(&list(&args, "rows"), &cells);
            let placed = placed(
                &list(&args, "tickets"),
                &list(&args, "made"),
                &names(&args, "may"),
                &rows,
                &of,
                &cells,
            )
            .map_err(ActionError::new)?;
            Ok(json!({
                "count": placed.ids.len(),
                "listed": placed.ids.join(", "),
                "links": placed.links,
                "ids": placed.ids,
                "ready": placed.ready,
                "told": placed.told,
            }))
        })
    });
}

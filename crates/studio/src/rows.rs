//! A table read as what its rows come to, asked as `rows.read`. What it
//! comes to is a function of the table's cells, its rows and what is asked,
//! and is the kernel's (`kernel::rows`), offered here under the names it
//! always had; what is here is the action, which reads the table and its
//! rows, and leaves out a column somebody added: that is how the table is
//! looked at, and steers nothing (it says no `takes`, so what it holds is
//! held to nothing of what whoever reads the rows does with them). It is
//! not handed, and a cell that `fill` names is never taken from one called
//! the same: a blueprint reads what it declares.

use blueprint::expr::slug;
use blueprint::{ActionSpec, ValueType as T};
use conductor::{ActionError, Registry};
pub use kernel::rows::{Asked, Cell};
use serde_json::{Map, Value};

use crate::Studio;
use crate::actions::{add, project_of, text};
use crate::fields::Field;

/// The rows of a table as they are taken, and what is asked of them
/// together ([`kernel::rows::read`]), but for the columns somebody added.
pub(crate) fn read(
    table: &str,
    fields: &[Field],
    records: &[(String, Map<String, Value>)],
    asked: &Asked<'_>,
) -> Result<Value, ActionError> {
    let cells: Vec<Cell<'_>> = fields
        .iter()
        .filter(|field| !field.added)
        .map(|field| Cell { name: &field.name, kind: &field.kind })
        .collect();
    kernel::rows::read(table, &cells, records, asked).map_err(ActionError::new)
}

/// The same asked with everything written out as a blueprint writes it:
/// the table's fields each with its `name` and `kind`, and what is asked
/// by the names `rows.read` takes it under. For whoever holds this to a
/// table of cases.
pub fn told(
    table: &str,
    fields: &Value,
    records: &[(String, Map<String, Value>)],
    asked: &Map<String, Value>,
) -> Result<Value, String> {
    let fields: Vec<Field> = serde_json::from_value(fields.clone()).map_err(|err| err.to_string())?;
    read(table, &fields, records, &asked_as(asked)).map_err(|err| err.message)
}

/// What is asked, out of what an action was handed.
fn asked_as(args: &Map<String, Value>) -> Asked<'_> {
    let cell = |key: &str| args.get(key).and_then(Value::as_str).filter(|cell| !cell.is_empty());
    let table = |key: &str| args.get(key).and_then(Value::as_object);
    Asked {
        fill: table("fill"),
        once: cell("once"),
        had: args.get("had"),
        adding: cell("adding"),
        counting: table("counting"),
        saying: table("saying"),
        giving: table("giving"),
        gathering: table("gathering"),
    }
}

pub(crate) fn register(registry: &mut Registry, studio: &Studio) {
    add(
        registry,
        studio,
        ActionSpec::new(
            "rows.read",
            "Read a table's rows as they are taken: each with what it leaves unsaid filled in, and what can be \
             said of them together.",
        )
.example(kernel::asked::ROWS_READ)
        .deciding()
        .required("collection", T::String, "the table's name")
        .arg("project", T::String, "which project (default: this instance's)")
        .arg("fill", T::Object, "what a cell that says nothing is taken to say, by cell: a value or a form. A cell with nothing here is filled by its kind: nothing for a number, false for a flag, none for a list, empty text otherwise")
        .within("fill", kernel::inside::GIVEN)
        .arg("once", T::String, "a cell in which each row is to say what no earlier row says: each row is given `again`, and those that say it again are listed")
        .arg("had", T::List, "the rows there were before, by id, to say which were taken away")
        .arg("adding", T::String, "a cell whose numbers are added up over the rows, leaving out those that say `once` again")
        .arg("counting", T::Object, "for each row, figures over rows of other lists, by the name each is given: the row is given what the figure comes to under the name, and how many rows it was over as `<name>_rows`")
        .within("counting", kernel::inside::COUNTING)
        .arg("giving", T::Object, "what each row is given, by cell, once it is filled, marked (`again`) and counted: a value or a form, as `fill` takes them")
        .within("giving", kernel::inside::GIVEN)
        .arg("gathering", T::Object, "lists of what one cell says, by the name each is given, in the order of the rows")
        .within("gathering", kernel::inside::GATHERING)
        .arg("saying", T::Object, "what a row is to say, by cell, to be one of `those`; a row is one of them when it says all of it")
        .within("saying", kernel::inside::SAYING)
        .capability("records:read")
        .output("rows", "each row: its `id`, every cell the table declares or `fill` names, and what `once` and `counting` add")
        .output("ids", "the rows there are now, by id")
        .output("count", "how many rows there are")
        .output("those", "the rows that say what `saying` asks, by id; none where it asks nothing")
        .output("others", "the rows that do not, by id; none where `saying` asks nothing")
        .output("chosen", "the rows that say what `saying` asks, as `rows` gives them; none where it asks nothing")
        .output("gathered", "each list `gathering` asks for, by its name")
        .output("gone", "those of `had` that are no longer there")
        .output("went", "how many those are")
        .output("again", "the rows that say in the `once` cell what an earlier row says, each with that row as `first`")
        .output("sum", "what the `adding` cell comes to")
        .idempotent(),
        |studio, cx, args| async move {
            let project = project_of(&cx, &args)?;
            let name = slug(&text(&args, "collection").unwrap_or_default());
            let collection = studio.collections.get(&project, &name).await?;
            let records = studio.records.list(&project, &name, cx.principal()).await?;
            let records: Vec<(String, Map<String, Value>)> =
                records.into_iter().map(|record| (record.id, record.data)).collect();
            let asked = asked_as(&args);
            read(&name, &collection.fields, &records, &asked)
        },
    );
}

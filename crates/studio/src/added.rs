//! A table's columns as the people who look at it change them: one
//! added, called something else, made to hold another kind of thing,
//! what a choice offers changed and put in order, and one taken away.
//!
//! What a table's columns are is said where it is declared, and that is
//! not all they are: whoever looks at the table may add a column of
//! their own (`added` on its field), call any column something else
//! (`label_chosen`) and put what a choice offers in another order
//! (`of_chosen`). Each is the table's, kept in its row with its fields
//! for everybody who opens it, as a chosen icon is ([`crate::icons`]):
//! never one person's and never one view's. It stands beside what is
//! declared. A table declared again keeps every column somebody added,
//! with what its records hold there, and what somebody chose of the
//! columns it declares ([`carry`], asked in the write that keeps the
//! declaration); a declaration that comes to say a field of a name
//! somebody's column has is the one that stands from then on.
//!
//! What is declared is not changed here. A declared field is read by
//! what declared it (a blueprint, or this program for a model's), so
//! what kind of thing it holds, what it offers and whether it is there
//! are said where the table is declared; somebody may call it something
//! else, and put its options in order, and no more. And a column
//! somebody adds is never one that gives or guards anything: it says no
//! `takes`, is never needed, and is never called what a declared field
//! or a record's own keys are (`kernel::added`, which is the plain
//! values of all this; `kernel::columns` is what a *view* says of the
//! columns there are).
//!
//! A column's name is what a record, a file, a view and a blueprint know
//! it by, and is never changed: renaming one is choosing its label.
//!
//! Every change is one write of the journal that keeps the table, with
//! what is said of it (`collection.fields`): the row is read, changed
//! and written there, so that two changes side by side are both kept,
//! and stopped anywhere both the change and the saying are kept or
//! neither is. What a change has to ask of the records (whether each
//! still fits, whether one still chooses an option that is to go) is
//! asked before the write, as where a table is declared, and the write
//! is refused where the column is no longer as it was when that was
//! asked.

use journal::optional;
use kernel::added as plain;
use kernel::{Error, Principal, Result, Source};
use rusqlite::params;
use serde_json::json;

use crate::board::{BOARD, COLUMN};
use crate::collections::{COLLECTION, Collection, Collections, Store, View, missing, read_collection};
use crate::fields::Field;

/// A column somebody adds.
#[derive(Clone, Debug, Default)]
pub struct New {
    /// What it is called. Its name is made from this, once.
    pub label: String,
    /// One of `kernel::added::KINDS`.
    pub kind: String,
    /// For a choice, what can be chosen; for a record or records, the table they are of.
    pub of: Vec<String>,
    /// The column it is a copy of, by name: the same kind and options,
    /// whatever `kind` and `of` say, and none of what its rows hold.
    pub like: Option<String>,
    /// Where it stands in a view, said in the same write.
    pub place: Option<Place>,
}

/// Where an added column stands in one view of its table.
#[derive(Clone, Debug, Default)]
pub struct Place {
    pub view: String,
    /// How that view is laid out, for a table that has no view yet: the
    /// one it is shown by until somebody saves one is then kept as this.
    pub kind: String,
    /// The view's columns as they stand for whoever asked, by name.
    pub order: Vec<String>,
    /// The column it stands before; after the last, where none is said.
    pub before: Option<String>,
}

/// A change to a column there is.
#[derive(Clone, Debug)]
pub enum Change {
    /// What it is called, or with nothing its name again.
    Label(Option<String>),
    /// What kind of thing it holds, with what that kind needs said (`of`).
    Kind { kind: String, of: Vec<String> },
    /// What a choice offers, in order: the same things in another order,
    /// or other things.
    Options(Vec<String>),
    /// One thing a choice offers, called something else; rows that chose
    /// it say the new word.
    Renamed { from: String, to: String },
}

/// What a column is headed by: what somebody called it, else its name as it reads.
pub fn heading(field: &Field) -> String {
    field.label_chosen.clone().unwrap_or_else(|| plain::reads(&field.name))
}

/// The fields a table is declared with, beside what somebody added to
/// the table there already is: what is declared is nobody's addition,
/// and each added column whose name the declaration does not take
/// stands after the declared ones.
pub(crate) fn beside(declared: Vec<Field>, existing: Option<&Collection>) -> Vec<Field> {
    let mut fields: Vec<Field> = declared.into_iter().filter(|field| !field.added).collect();
    let kept: Vec<&Field> =
        existing.iter().flat_map(|table| table.fields.iter().filter(|field| field.added)).collect();
    let stand = {
        let names: Vec<&str> = fields.iter().map(|field| field.name.as_str()).collect();
        let added: Vec<&str> = kept.iter().map(|field| field.name.as_str()).collect();
        plain::standing(&names, &added).into_iter().map(str::to_string).collect::<Vec<_>>()
    };
    fields.extend(kept.into_iter().filter(|field| stand.contains(&field.name)).cloned());
    // What a declared choice offers is held to what its records choose
    // before it is written: with what somebody added to it, which a
    // declaration does not take away. It is worked out again in the
    // write ([`carry`]), from the row as it is then.
    for field in fields.iter_mut().filter(|field| !field.added && field.of_chosen.is_empty()) {
        let was = existing.and_then(|table| table.field(&field.name));
        given_back(field, was);
        field.of_chosen.clear();
    }
    fields
}

/// What a declared choice offers that its declaration says: all it
/// offers, less what somebody added to it.
fn declared_of(field: &Field) -> Vec<String> {
    let theirs = field.of_added.as_deref().unwrap_or_default();
    field.of.iter().filter(|option| !theirs.contains(option)).cloned().collect()
}

/// What somebody added to a declared choice once it is arranged as
/// `chosen` over `was`, the field as its table keeps it: not known
/// where it was not known (put in order before it was kept).
pub(crate) fn added_to(was: &Field, chosen: &[String]) -> Option<Vec<String>> {
    match (was.of_chosen.is_empty(), &was.of_added) {
        (true, _) => Some(plain::added(chosen, &was.of, &[])),
        (false, Some(theirs)) => Some(plain::added(chosen, &declared_of(was), theirs)),
        (false, None) => None,
    }
}

/// A declared field given back what somebody chose of what it offers,
/// from `was`, the field as its table keeps it. Where the declaration
/// says an order itself (a board's columns arranged, which the board
/// declares), that stands and is held to what it offers. Otherwise what
/// was chosen is kept in its order, with what somebody added to it
/// standing beside the declaration's own (`of`, less what [`beside`]
/// has given it back already), and what they added is kept with it, so
/// that the next declaration tells the two apart.
fn given_back(field: &mut Field, was: Option<&Field>) {
    if field.kind != "choice" {
        field.of_chosen.clear();
        field.of_added = None;
        return;
    }
    if !field.of_chosen.is_empty() {
        field.of = plain::ordered(&field.of, &field.of_chosen);
        field.of_chosen.retain(|option| field.of.contains(option));
        if let Some(theirs) = &mut field.of_added {
            theirs.retain(|option| field.of.contains(option));
        }
        return;
    }
    let declared = declared_of(field);
    match was.filter(|was| was.kind == "choice" && !was.of_chosen.is_empty()) {
        Some(was) => {
            field.of = plain::offered(&declared, was.of_added.as_deref(), &was.of_chosen);
            field.of_chosen =
                was.of_chosen.iter().filter(|option| field.of.contains(option)).cloned().collect();
            let theirs = was.of_added.as_deref().unwrap_or_default();
            field.of_added = Some(plain::added(&field.of_chosen, &declared, theirs));
        }
        None => (field.of, field.of_added) = (declared, None),
    }
}

/// Gives a table that is being written, as it was just declared, what
/// people added to it and chose of its columns, as its row says in this
/// very write: each chosen icon, label and order of options by the
/// field's name, and each added column the declaration has no field
/// called. What the declaration itself says somebody chose of an order
/// (a board's columns arranged, which is declared by the board) stands.
///
/// It is read where the row is written and not when whoever declares the
/// table first read it: a column added while a blueprint was being
/// applied would be written over by what was read before it.
pub(crate) fn carry(connection: &rusqlite::Connection, table: &mut Collection) -> rusqlite::Result<()> {
    let kept: Option<(String, Option<String>)> = optional(connection.query_row(
        "SELECT fields, beside FROM collections WHERE project = ?1 AND name = ?2",
        [&table.project, &table.name],
        |row| Ok((row.get(0)?, row.get(1)?)),
    ))?;
    // What somebody called the table stands, as what they called a column does.
    let (kept, shown) = kept.map_or((None, None), |(fields, shown)| (Some(fields), shown));
    table.shown.title_chosen = crate::shown::Shown::read(shown).title_chosen;
    let kept: Vec<Field> = kept.and_then(|kept| serde_json::from_str(&kept).ok()).unwrap_or_default();
    table.fields.retain(|field| !field.added);
    for field in &mut table.fields {
        let was = kept.iter().find(|was| was.name == field.name);
        field.icon_chosen = was.and_then(|was| was.icon_chosen.clone());
        field.label_chosen = was.and_then(|was| was.label_chosen.clone());
        given_back(field, was);
    }
    let declared: Vec<String> = table.fields.iter().map(|field| field.name.clone()).collect();
    table.fields.extend(kept.into_iter().filter(|was| was.added && !declared.contains(&was.name)));
    Ok(())
}

/// What a change came to, for the write that keeps it.
struct Done {
    /// The field it was of, by name.
    field: String,
    /// What was done: `added`, `called`, `retyped`, `options` or `removed`.
    did: &'static str,
    /// The views are written with the fields: one was changed, or went.
    views: bool,
    /// A view that is said to have changed with it.
    view: Option<String>,
    /// What the rows hold under the field's name goes, where the rows are this database's.
    strip: bool,
    /// One thing a choice offers is called something else in the rows that chose it.
    renamed: Option<(String, String)>,
}

impl Done {
    fn of(field: &str, did: &'static str) -> Done {
        Done { field: field.to_string(), did, views: false, view: None, strip: false, renamed: None }
    }
}

fn no_field(table: &Collection, field: &str) -> Error {
    Error::not_found(format!("`{}` in `{}` has no field called `{field}`", table.name, table.project))
}

/// What is said where the column is no longer as it was when the change was asked.
fn moved_on(table: &Collection) -> Error {
    Error::conflict(format!(
        "the columns of `{}` were changed by somebody else as this was asked: look at them as they are now, and ask again",
        table.name
    ))
}

/// The field called `field`, where somebody added it: what is declared
/// is changed where the table is declared.
fn theirs<'a>(table: &'a Collection, field: &str, what: &str) -> Result<&'a Field> {
    let one = table.field(field).ok_or_else(|| no_field(table, field))?;
    if one.added {
        return Ok(one);
    }
    let whose = match &table.model {
        Some(model)
            if crate::models::of(&table.name, Some(model))
                .ok()
                .flatten()
                .is_some_and(|model| model.has(field)) =>
        {
            format!("a `{model}` has it, and this program reads it")
        }
        _ => "what declared the table reads it".to_string(),
    };
    Err(Error::conflict(format!(
        "`{field}` of `{}` is declared with the table: {whose}. {what} is said where the table is declared, and here \
         only of a column somebody added",
        table.name
    )))
}

/// Refuses a label that another column of the table is already headed by.
fn alike(table: &Collection, label: &str, but: Option<&str>) -> Result<()> {
    let other = table.fields.iter().filter(|field| Some(field.name.as_str()) != but);
    match other.into_iter().find(|field| heading(field).eq_ignore_ascii_case(label)) {
        None => Ok(()),
        Some(field) => Err(Error::conflict(format!(
            "`{}` has a column called {} already (`{}`); call this one something else",
            table.name,
            heading(field),
            field.name
        ))),
    }
}

/// What a choice offers, as it is kept: each a lowercase word joined by
/// dashes, once, in the order given.
fn options(field: &str, written: &[String]) -> Result<Vec<String>> {
    let mut of: Vec<String> = Vec::new();
    for option in
        written.iter().map(|option| blueprint::expr::slug(option)).filter(|option| !option.is_empty())
    {
        if !of.contains(&option) {
            of.push(option);
        }
    }
    if of.is_empty() {
        return Err(Error::invalid(format!(
            "`{field}` is a choice and needs what can be chosen: at least one thing, each a word or a few"
        )));
    }
    if of.len() > plain::MOST {
        return Err(Error::invalid(format!(
            "a choice offers {} things at most, and `{field}` was given {}",
            plain::MOST,
            of.len()
        )));
    }
    Ok(of)
}

/// Takes what a view says of the column `name` out of it: the column is gone.
fn forget(view: &mut View, name: &str) {
    for names in view.hides.iter_mut().chain(view.shown.order.iter_mut()) {
        names.retain(|one| one != name);
    }
    if view.shown.freezes.as_deref() == Some(name) {
        view.shown.freezes = None;
    }
    if let Some(figures) = &mut view.shown.figures {
        figures.remove(name);
    }
    if let Some(widths) = &mut view.shown.widths {
        widths.remove(name);
    }
    view.shown.widths.take_if(|widths| widths.is_empty());
    if let Some(only) = &mut view.only {
        only.remove(name);
    }
    view.only.take_if(|only| only.is_empty());
    view.group.take_if(|group| group == name);
    view.done.take_if(|done| done == name);
}

/// Takes away the views the table's fields no longer allow (a board by a
/// choice that is one no longer), as a declaration does; refused where
/// that would leave the table none. Whether any went.
fn tidy(table: &mut Collection, field: &str) -> Result<bool> {
    let fields = table.fields.clone();
    let before = table.views.len();
    let stands: Vec<View> = table.views.iter().filter(|view| view.check(&fields).is_ok()).cloned().collect();
    if stands.is_empty() && before > 0 {
        let view = table.views.first().map(|view| view.name.clone()).unwrap_or_default();
        return Err(Error::conflict(format!(
            "the view `{view}` of `{}` is laid out by `{field}` and is the only view the table has; save another \
             view before changing or taking away the column it goes by",
            table.name
        )));
    }
    table.views = stands;
    Ok(table.views.len() != before)
}

impl Collections {
    /// The table, for somebody who may change how it is looked at: what
    /// saving a view of it takes. One that is kept from whoever asks is
    /// not there for them, in the words it would not be there in.
    pub(crate) async fn looked_at(&self, project: &str, name: &str, by: &Principal) -> Result<Collection> {
        if !by.can("board:write") {
            by.require("records:write")?;
        }
        let table = self.get(project, name).await?;
        if table.open_to(by).is_err() {
            return Err(missing(project, name));
        }
        table.may_change(by)?;
        Ok(table)
    }

    /// Refuses unless `by` may change what columns the table has, and
    /// the table is one whose columns can be.
    fn shapes(table: &Collection, by: &Principal) -> Result<()> {
        by.require(plain::SCOPE)?;
        if matches!(table.store, Store::Documents | Store::Mockups) {
            return Err(Error::conflict(format!(
                "the records of `{}` are files of the project as people wrote them, and a column added here would be \
                 written into each of them: what it keeps of each is said where it is declared",
                table.name
            )));
        }
        Ok(())
    }

    /// The kind a column somebody adds or retypes is to be, with what
    /// that kind needs said, or why it cannot be that.
    async fn kind_of(
        &self,
        table: &Collection,
        field: &str,
        kind: &str,
        of: &[String],
        by: &Principal,
    ) -> Result<(String, Vec<String>)> {
        if !plain::KINDS.contains(&kind) {
            return Err(Error::invalid(format!(
                "a column somebody adds holds one of: {}; `{}` is not one of them",
                plain::KINDS.join(", "),
                kind.chars().take(plain::LONGEST).collect::<String>()
            )));
        }
        let of = match kind {
            "choice" => options(field, of)?,
            "record" | "records" => {
                let named: Vec<String> =
                    of.iter().map(|one| blueprint::expr::slug(one)).filter(|one| !one.is_empty()).collect();
                let [named] = named.as_slice() else {
                    return Err(Error::invalid(format!(
                        "`{field}` names records and needs the one table whose records it can name, as in `of: [sprints]`"
                    )));
                };
                // Naming a table takes being able to read it, and one kept
                // from whoever asks is not there for them.
                let there =
                    self.find(&table.project, named).await?.filter(|other| other.may_read(by).is_ok());
                if there.is_none() {
                    return Err(Error::invalid(format!(
                        "`{field}` is to name records of `{named}`, and the project `{}` has no collection called `{named}`",
                        table.project
                    )));
                }
                vec![named.clone()]
            }
            _ => Vec::new(),
        };
        Ok((kind.to_string(), of))
    }

    /// Changes a table's row as it is when the change is made, in one
    /// write with what is said of it. `work` is handed the table as the
    /// row then says it is, and says what it did or why it will not.
    async fn reshape(
        &self,
        table: &Collection,
        by: &Principal,
        work: impl FnOnce(&mut Collection) -> Result<Done> + Send + 'static,
    ) -> Result<(Collection, String)> {
        let now = self.factory().now_ms();
        let (project, name, actor) = (table.project.clone(), table.name.clone(), by.actor());
        // Its rows may be written here, which what is worked out of them goes by.
        let counting = self.counted.writing(&table.project, &table.name);
        let written = self.rows(&table.project).write(move |writing| {
            let transaction = writing.transaction();
            let read = transaction.query_row(
                &format!("SELECT {COLLECTION} FROM collections WHERE project = ?1 AND name = ?2"),
                [&project, &name],
                read_collection,
            );
            let Some(mut table) = optional(read)? else { return Ok(Err(missing(&project, &name))) };
            let done = match work(&mut table) {
                Ok(done) => done,
                Err(refused) => return Ok(Err(refused)),
            };
            table.updated = now;
            // Each part of the row by itself, from the row as it was read
            // in this write: what somebody else wrote of the rest stays.
            transaction.execute(
                "UPDATE collections SET fields = ?3, updated = ?4 WHERE project = ?1 AND name = ?2",
                params![project, name, serde_json::to_string(&table.fields).unwrap_or_default(), now],
            )?;
            if done.views {
                transaction.execute(
                    "UPDATE collections SET views = ?3 WHERE project = ?1 AND name = ?2",
                    params![project, name, serde_json::to_string(&table.views).unwrap_or_default()],
                )?;
            }
            // A column taken away, or made another kind, takes its index with it (`indexes`).
            if done.did == "removed" || done.did == "retyped" {
                transaction.execute_batch(&kernel::window::statements::dropped(
                    &project,
                    &name,
                    &done.field,
                ))?;
            }
            if table.store == Store::Local {
                let cell = format!("$.\"{}\"", done.field);
                if done.strip {
                    transaction.execute(
                        "UPDATE records SET data = json_remove(data, ?3)
                         WHERE project = ?1 AND collection = ?2 AND json_type(data, ?3) IS NOT NULL",
                        params![project, name, cell],
                    )?;
                }
                if let Some((from, to)) = &done.renamed {
                    transaction.execute(
                        "UPDATE records SET data = json_set(data, ?3, ?5)
                         WHERE project = ?1 AND collection = ?2 AND json_extract(data, ?3) = ?4",
                        params![project, name, cell, from, to],
                    )?;
                }
            }
            // Of a table kept private it is heard by whoever may read the table.
            let source = Source::actor(actor.clone()).taking(table.reads.as_deref());
            let said = json!({ "project": project, "name": name, "field": done.field, "did": done.did });
            writing.say("collection.fields", source, said)?;
            if let Some(view) = &done.view {
                let said = json!({ "project": project, "name": name, "view": view });
                writing.say("collection.views", Source::actor(actor.clone()), said)?;
            }
            // A board's columns are what its cards' column can be: put in
            // order here, they are the board's columns set.
            if name == BOARD && done.field == COLUMN && done.did == "options" {
                let columns = table.field(COLUMN).map(|field| field.of.clone()).unwrap_or_default();
                let said = json!({ "project": project, "columns": columns });
                writing.say("board.columns", Source::actor(actor.clone()), said)?;
            }
            Ok(Ok((table, done.field)))
        });
        let written = written.await;
        counting.moved(Some(0));
        let (table, field) = written?.0?;
        self.indexes.forget(&table.project, &table.name, Some(&field));
        Ok((table, field))
    }

    /// Adds a column to a table, for everybody who opens it. Gives the
    /// table as it then is, and what the column is called by everything
    /// that reads it.
    pub async fn add_column(
        &self,
        project: &str,
        name: &str,
        new: New,
        by: &Principal,
    ) -> Result<(Collection, String)> {
        let table = self.looked_at(project, name, by).await?;
        Self::shapes(&table, by)?;
        let headed: Vec<String> = table.fields.iter().map(heading).collect();
        let headed: Vec<&str> = headed.iter().map(String::as_str).collect();
        let (label, kind, of) = match &new.like {
            Some(like) => {
                let copied = table.field(like).ok_or_else(|| no_field(&table, like))?;
                let label = match new.label.trim() {
                    "" => plain::copy(&heading(copied), &headed),
                    said => plain::label(said)?,
                };
                (label, copied.kind.clone(), copied.of.clone())
            }
            None => (plain::label(&new.label)?, new.kind.clone(), new.of.clone()),
        };
        alike(&table, &label, None)?;
        let names: Vec<&str> = table.fields.iter().map(|field| field.name.as_str()).collect();
        let called = plain::free(&plain::name(&label), &names);
        let (kind, of) = self.kind_of(&table, &called, &kind, &of, by).await?;
        let field = Field {
            name: called.clone(),
            kind,
            of,
            required: false,
            quiet: false,
            takes: Default::default(),
            takes_of: Default::default(),
            takes_each: false,
            unsaid: Vec::new(),
            hidden: false,
            suggests: Vec::new(),
            zoned: false,
            said: false,
            writer_of: None,
            icon: None,
            icon_chosen: None,
            added: true,
            label_chosen: Some(label.clone()).filter(|label| *label != plain::reads(&called)),
            of_chosen: Vec::new(),
            of_added: None,
            rewording: false,
            calls: None,
            indexed: false,
        };
        // A column begins with nothing in it. Rows kept in this database
        // are emptied of what a column of the name once held, in the
        // write; rows kept as files, or by a store service, are theirs.
        if table.store != Store::Local {
            let mut with = table.clone();
            with.fields.push(field.clone());
            let holds = |record: &crate::collections::Record| {
                record.data.get(&called).is_some_and(|value| !value.is_null())
            };
            let left = self.records(&with).await?.iter().filter(|record| holds(record)).count();
            if left > 0 {
                return Err(Error::conflict(format!(
                    "{left} record(s) of `{name}` already say something under `{called}`, where they are kept (a column \
                     of that name was there before): call the column something else, or take that out of them first"
                )));
            }
        }
        let place = new.place;
        self.reshape(&table, by, move |table| {
            let names: Vec<&str> = table.fields.iter().map(|field| field.name.as_str()).collect();
            if plain::free(&plain::name(&label), &names) != field.name {
                return Err(moved_on(table));
            }
            alike(table, &label, None)?;
            let mut done = Done { strip: true, ..Done::of(&field.name, "added") };
            table.fields.push(field);
            if let Some(place) = place {
                let has = |one: &String| table.fields.iter().any(|field| &field.name == one);
                let order: Vec<String> = place.order.iter().filter(|one| has(one)).cloned().collect();
                let left_out: Vec<String> = table
                    .fields
                    .iter()
                    .filter(|field| field.hidden)
                    .map(|field| field.name.clone())
                    .collect();
                let placed = plain::placed(&order, &done.field, place.before.as_deref());
                // A table with no view is shown by one of its own name until somebody saves one:
                // putting a column somewhere in it is saving it.
                let first = View {
                    name: place.view.trim().to_string(),
                    kind: if place.kind.is_empty() { "table".into() } else { place.kind.clone() },
                    ..View::default()
                };
                if table.views.is_empty() && !first.name.is_empty() && first.check(&table.fields).is_ok() {
                    table.views.push(first);
                }
                if let Some(view) = table.views.iter_mut().find(|view| view.name == place.view.trim()) {
                    view.shown.order = Some(placed);
                    // It is somebody's from here on, and says what it leaves out as one of theirs does.
                    if !view.saved {
                        view.hides.get_or_insert(left_out);
                        view.saved = true;
                    }
                    (done.views, done.view) = (true, Some(view.name.clone()));
                }
            }
            Ok(done)
        })
        .await
    }

    /// Changes a column there is: what it is called, of any column; and
    /// of one somebody added, what kind of thing it holds and what it
    /// offers. The order of what a choice offers is anybody's to change
    /// who may save a view, of a declared choice too.
    pub async fn change_column(
        &self,
        project: &str,
        name: &str,
        field: &str,
        change: Change,
        by: &Principal,
    ) -> Result<Collection> {
        let table = self.looked_at(project, name, by).await?;
        let was = table.field(field).ok_or_else(|| no_field(&table, field))?.clone();
        let of = field.to_string();
        let changed = match change {
            Change::Label(label) => {
                let label = label.as_deref().map(plain::label).transpose()?;
                self.reshape(&table, by, move |table| {
                    if let Some(label) = &label {
                        alike(table, label, Some(&of))?;
                    }
                    let Some(one) = table.fields.iter_mut().find(|one| one.name == of) else {
                        return Err(no_field(table, &of));
                    };
                    // Called what its name reads as, it is called nothing else.
                    one.label_chosen = label.filter(|label| *label != plain::reads(&one.name));
                    Ok(Done::of(&of, "called"))
                })
                .await
            }
            Change::Kind { kind, of: said } if !(was.kind == "choice" && kind == "choice") => {
                Self::shapes(&table, by)?;
                theirs(&table, field, "What kind of thing it holds")?;
                let (kind, said) = self.kind_of(&table, field, &kind, &said, by).await?;
                let now = Field { kind, of: said, of_chosen: Vec::new(), of_added: None, ..was.clone() };
                let left = self.misfit(&table, &now).await?;
                if left > 0 {
                    return Err(Error::conflict(format!(
                        "`{field}` holds a `{}` and {left} record(s) of `{name}` say something there that a `{}` \
                         does not take: change or empty those first, and then make it one",
                        was.kind, now.kind
                    )));
                }
                self.reshape(&table, by, move |table| {
                    let Some(one) = table.fields.iter_mut().find(|one| one.name == of) else {
                        return Err(no_field(table, &of));
                    };
                    if *one != was {
                        return Err(moved_on(table));
                    }
                    *one = now;
                    let views = tidy(table, &of)?;
                    Ok(Done { views, ..Done::of(&of, "retyped") })
                })
                .await
            }
            Change::Kind { of: said, .. } | Change::Options(said) => {
                if was.kind != "choice" {
                    return Err(Error::invalid(format!(
                        "`{field}` of `{name}` is a `{}`, and only a choice offers things to choose",
                        was.kind
                    )));
                }
                let now = options(field, &said)?;
                // The same things in another order is how the table is
                // looked at; other things is what its column is.
                let reordered = plain::same(&was.of, &now);
                if !reordered {
                    Self::shapes(&table, by)?;
                    theirs(&table, field, "What it offers")?;
                    for gone in was.of.iter().filter(|option| !now.contains(option)) {
                        let left = self.chose(&table, field, gone).await?;
                        if left > 0 {
                            return Err(Error::conflict(format!(
                                "`{gone}` is still the `{field}` of {left} record(s) of `{name}`; change them before taking it away"
                            )));
                        }
                    }
                }
                self.reshape(&table, by, move |table| {
                    let Some(one) = table.fields.iter_mut().find(|one| one.name == of) else {
                        return Err(no_field(table, &of));
                    };
                    if one.kind != "choice"
                        || (reordered && !plain::same(&one.of, &now))
                        || (!reordered && *one != was)
                    {
                        return Err(moved_on(table));
                    }
                    let gone: Vec<String> =
                        one.of.iter().filter(|option| !now.contains(option)).cloned().collect();
                    // Of a declared choice the order is kept apart, so that
                    // its declaration said again does not put its own back.
                    // What somebody added to it is kept with the order, so that a
                    // declaration said again tells it from its own.
                    one.of_added = if one.added { None } else { added_to(one, &now) };
                    one.of_chosen = if one.added { Vec::new() } else { now.clone() };
                    one.of = now;
                    let mut views = false;
                    for only in table.views.iter_mut().filter_map(|view| view.only.as_mut()) {
                        if let Some(held) =
                            only.get_mut(&of).filter(|held| held.iter().any(|one| gone.contains(one)))
                        {
                            held.retain(|one| !gone.contains(one));
                            views = true;
                        }
                        only.retain(|_, held| !held.is_empty());
                    }
                    Ok(Done { views, ..Done::of(&of, "options") })
                })
                .await
            }
            Change::Renamed { from, to } => {
                Self::shapes(&table, by)?;
                theirs(&table, field, "What it offers")?;
                let to = blueprint::expr::slug(&to);
                if was.kind != "choice" || !was.of.contains(&from) {
                    return Err(Error::invalid(format!(
                        "`{from}` is not something `{field}` of `{name}` offers"
                    )));
                }
                if to.is_empty() || was.of.contains(&to) {
                    return Err(Error::conflict(format!(
                        "`{field}` of `{name}` offers `{to}` already, or that is no word: call `{from}` something it does not offer"
                    )));
                }
                // Rows that chose it say the new word in the same write
                // where they are this database's. Kept as files, or by a
                // store service, each would be a write of its own.
                if table.store != Store::Local {
                    let chose = |record: &&crate::collections::Record| {
                        record.data.get(field).and_then(|value| value.as_str()) == Some(&from)
                    };
                    let left = self.records(&table).await?.iter().filter(chose).count();
                    if left > 0 {
                        return Err(Error::conflict(format!(
                            "{left} record(s) of `{name}` chose `{from}`, and they are kept where each would have to be \
                             written again by itself: add `{to}`, change them, and then take `{from}` away"
                        )));
                    }
                }
                self.reshape(&table, by, move |table| {
                    let Some(one) = table.fields.iter_mut().find(|one| one.name == of) else {
                        return Err(no_field(table, &of));
                    };
                    if *one != was {
                        return Err(moved_on(table));
                    }
                    for option in one.of.iter_mut().filter(|option| **option == from) {
                        *option = to.clone();
                    }
                    let mut views = false;
                    for held in table.views.iter_mut().filter_map(|view| view.only.as_mut()?.get_mut(&of)) {
                        for option in held.iter_mut().filter(|option| **option == from) {
                            (*option, views) = (to.clone(), true);
                        }
                    }
                    Ok(Done { views, renamed: Some((from, to)), ..Done::of(&of, "options") })
                })
                .await
            }
        };
        changed.map(|(table, _)| table)
    }

    /// Takes away a column somebody added, with what the table's rows
    /// hold there. A declared one is taken away where the table is declared.
    pub async fn remove_column(
        &self,
        project: &str,
        name: &str,
        field: &str,
        by: &Principal,
    ) -> Result<Collection> {
        let table = self.looked_at(project, name, by).await?;
        Self::shapes(&table, by)?;
        theirs(&table, field, "Whether it is there")?;
        let of = field.to_string();
        let removed = self.reshape(&table, by, move |table| {
            theirs(table, &of, "Whether it is there")?;
            table.fields.retain(|one| one.name != of);
            for view in &mut table.views {
                forget(view, &of);
            }
            tidy(table, &of)?;
            Ok(Done { views: true, strip: true, ..Done::of(&of, "removed") })
        });
        removed.await.map(|(table, _)| table)
    }
}

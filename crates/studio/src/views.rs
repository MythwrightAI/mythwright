//! A view: one way of looking at a collection.
//!
//! How its records are laid out, what that goes by, and which of its
//! fields it leaves out. A blueprint declares some with the collection
//! (`collection.ensure`), and whoever looks at it saves the rest beside
//! them; either is kept with the collection.

use std::collections::BTreeMap;

use kernel::{Error, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::collections::{LAYOUTS, STAMPS};
use crate::fields::Field;

/// The kinds of field a view's records can be gathered by, and held to:
/// one that names something, alone or among several. A record that
/// holds several things (a `list`, `records`) is in the band of each.
const NAMES: &[&str] = &["record", "records", "choice", "actor", "text", "list"];
/// Those kinds, as a sentence ends with them.
const NAMING: &str = "a `record`, `records`, `choice`, `actor`, `text` or `list`";

/// A way of looking at a collection.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct View {
    pub name: String,
    /// One of `LAYOUTS`.
    pub kind: String,
    /// For a board: the choice field whose values are its columns. For a
    /// calendar: the date its records are put on the days of, a field or
    /// one of `STAMPS`; when it was made, where none is said. For an
    /// order: the field in which a record says which others come before
    /// it, by their ids.
    #[serde(default)]
    pub by: Option<String>,
    /// The field its records are gathered by, each thing it names a band
    /// of its own (`group`): for a board, an order and a table, where said.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    /// For an order: the choice field whose last value says a record is
    /// done, and holds up nothing that comes after it (`done`). The first
    /// choice the collection has, where none is said.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub done: Option<String>,
    /// What it is held to (`only`): for each field named, the things a
    /// record has to hold one of to be shown, or the values it has to
    /// say one of. A view that says nothing shows every record.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub only: Option<BTreeMap<String, Vec<String>>>,
    /// Somebody looking at the collection saved it, and it is theirs to
    /// change and take away. One that is not was declared by the blueprint,
    /// and is the blueprint's.
    #[serde(default)]
    pub saved: bool,
    /// The fields it leaves out, by name, where it says which (`hides`).
    /// One that says nothing leaves out what the collection's fields say
    /// is left out to begin with (`hidden`), if it is the blueprint's, and
    /// nothing if somebody saved it: a view saved before a view could say
    /// this showed every field, and goes on showing them. A name that is
    /// no field of the collection leaves nothing out.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hides: Option<Vec<String>>,
    /// What it says of its columns: their order, which it is frozen up to, the figure at the foot of each.
    #[serde(flatten)]
    pub shown: kernel::columns::Columns,
}

impl View {
    /// Says that this is one of the ways a collection is laid out.
    pub(crate) fn laid_out(&self) -> Result<()> {
        let (name, kind) = (&self.name, &self.kind);
        if !LAYOUTS.contains(&kind.as_str()) {
            return Err(Error::invalid(format!(
                "the view `{name}` is a `{kind}`, which is not a way a collection is laid out; there are: {}",
                LAYOUTS.join(", ")
            )));
        }
        Ok(())
    }

    /// Says that what a blueprint's view leaves out are fields of the
    /// collection. Asked of a view as it is declared, where a name that is
    /// no field is a slip of whoever wrote it; never of one somebody
    /// saved, which may outlast a field it left out.
    pub(crate) fn leaves_out_fields(&self, fields: &[Field]) -> Result<()> {
        let unknown =
            self.hides.iter().flatten().find(|name| !fields.iter().any(|field| &field.name == *name));
        match unknown {
            None => Ok(()),
            Some(unknown) => Err(Error::invalid(format!(
                "the view `{}` hides `{unknown}`, which is not a field of the collection; it has: {}",
                self.name,
                fields.iter().map(|field| field.name.as_str()).collect::<Vec<_>>().join(", ")
            ))),
        }
    }

    /// Says what is wrong with what it says of its columns (`kernel::columns`), asked as it is declared or
    /// saved and never of one that is kept, which may outlast a column.
    pub(crate) fn shows(&self, fields: &[Field]) -> Result<()> {
        let kinds: Vec<_> = fields.iter().map(|field| (field.name.as_str(), field.kind.as_str())).collect();
        self.shown.check(&self.name, &self.kind, &kinds)
    }

    /// Says what is wrong with this view of a collection with these
    /// fields, if anything is.
    pub(crate) fn check(&self, fields: &[Field]) -> Result<()> {
        self.laid_out()?;
        let (name, kind) = (&self.name, &self.kind);
        let by = self.by.as_deref().and_then(|by| fields.iter().find(|field| field.name == by));
        if kind == "board" && !by.is_some_and(|field| field.kind == "choice") {
            return Err(Error::invalid(format!(
                "the view `{name}` is a board and needs `by`: the choice field whose values are its columns"
            )));
        }
        if kind == "calendar"
            && let Some(named) = self.by.as_deref()
            && !STAMPS.contains(&named)
            && !by.is_some_and(|field| field.kind == "date")
        {
            return Err(Error::invalid(format!(
                "the view `{name}` is a calendar by `{named}`, which is not a date of its records; it goes by a date field, or by `created` or `updated`"
            )));
        }
        if kind == "order" && !by.is_some_and(|field| field.kind == "list" || field.kind == "records") {
            return Err(Error::invalid(format!(
                "the view `{name}` is an order and needs `by`: the field in which a record says which others come before it, a `list` of their ids or a `records` of its own collection"
            )));
        }
        let field = |named: &str| fields.iter().find(|field| field.name == named);
        if let Some(named) = self.done.as_deref()
            && !(kind == "order" && field(named).is_some_and(|field| field.kind == "choice"))
        {
            return Err(Error::invalid(format!(
                "the view `{name}` says `done: {named}`, which is for an order and names the choice field whose last value says a record is done"
            )));
        }
        if let Some(named) = self.group.as_deref()
            && !((kind == "board" || kind == "order" || kind == "table")
                && field(named).is_some_and(|field| NAMES.contains(&field.kind.as_str())))
        {
            return Err(Error::invalid(format!(
                "the view `{name}` says `group: {named}`, which is for a board, an order or a table and names the field its records are gathered by: {NAMING}"
            )));
        }
        for (named, any) in self.only.iter().flatten() {
            if !field(named).is_some_and(|field| NAMES.contains(&field.kind.as_str())) {
                return Err(Error::invalid(format!(
                    "the view `{name}` says `only` by `{named}`, which is not a field its records can be held to: {NAMING}"
                )));
            }
            if any.is_empty() || any.iter().any(|one| one.trim().is_empty()) {
                return Err(Error::invalid(format!(
                    "the view `{name}` says `only` by `{named}` and nothing a record is to hold there; say at least one thing, as in `only: {{ {named}: [urgent] }}`, or leave `{named}` out"
                )));
            }
        }
        Ok(())
    }

    /// What a view is held to, as a blueprint writes `only` and as a
    /// page sends it: a map from each field to the things a record is to
    /// hold one of there, a list or one alone. Nothing, where nothing is
    /// said or the map is empty.
    pub fn held_to(name: &str, said: Option<&Value>) -> Result<Option<BTreeMap<String, Vec<String>>>> {
        let wrong = || {
            Error::invalid(format!(
                "the view `{name}` says `only`, which is a map from a field to what a record is to hold there, as in `only: {{ labels: [urgent, soon] }}`"
            ))
        };
        let map = match said {
            None | Some(Value::Null) => return Ok(None),
            Some(Value::Object(map)) => map,
            Some(_) => return Err(wrong()),
        };
        let mut only = BTreeMap::new();
        for (field, any) in map {
            let any: Vec<String> = match any {
                Value::Array(all) if all.iter().all(|one| !one.is_object() && !one.is_array()) => {
                    all.iter().map(blueprint::expr::display).collect()
                }
                Value::Array(_) | Value::Object(_) | Value::Null => return Err(wrong()),
                one => vec![blueprint::expr::display(one)],
            };
            only.insert(field.clone(), any);
        }
        Ok((!only.is_empty()).then_some(only))
    }

    /// This view as somebody's own: its name without the space round it,
    /// and what it leaves out said once each. A view of somebody's always
    /// says what it leaves out: one that said nothing would be read as one
    /// from before a view could, which shows every field.
    pub(crate) fn into_saved(self) -> View {
        let mut hides: Vec<String> = Vec::new();
        for name in self.hides.iter().flatten().map(|name| name.trim()).filter(|name| !name.is_empty()) {
            if !hides.iter().any(|kept| kept == name) {
                hides.push(name.to_string());
            }
        }
        View { name: self.name.trim().to_string(), saved: true, hides: Some(hides), ..self }
    }
}

//! What a table says of how it is shown, which is all it is: nothing
//! here lets anybody in or keeps anybody out, and nothing in this
//! program acts on it. It is kept with the table and said to whoever
//! reads what the table is, and whoever draws the table goes by it.
//!
//! A table may say it sits beside another of its project (`beside`):
//! it is then reached from that table's page and is no thing of its own
//! in a list of what the project has. With that it may say which of its
//! own fields says what of the other a row is about (`about`): the
//! column of a board a stage is of, where the other is laid out in
//! columns. A row that leaves that field empty is about what it is
//! called. A table that sits beside one that is not there sits beside
//! nothing, and is shown as any table is. What has a page of its own
//! kind under its project (a board, a library, mockups) sits beside
//! nothing: it would be reached both ways, or drawn as what it is not.
//!
//! And it may say its columns in groups (`groups`): headings in order,
//! each with the fields under it in order. A blueprint's fields are a
//! map, which keeps no order, so the order is said here and not by a
//! word on each field. A field in no group is drawn before the groups.
//!
//! And it may say its rows are not written by hand (`by_hand: false`):
//! they are added and changed by the commands and blueprints that keep
//! the table (each heavy run of a script, each deploy), and somebody who
//! looks at it reads them. The client then offers no new row, no cell or
//! field to change and no column to add, call something else or take
//! away, and goes on offering what is how somebody looks at a table and
//! taking a row away. The daemon refuses nothing for it: every door that
//! writes a record is as it was, since a command and a blueprint are what
//! fill such a table. Only a table of plain records says it: a board, a
//! library and mockups each have a page of their own kind.
//!
//! And it may say what it is called for people (`title`), beside its name,
//! which is what everything else knows it by (`rows.read`, its address,
//! where a script keeps its runs) and never changes: `Test runs` for
//! `test-runs`. Whoever may change the table may call it something else
//! for everybody (`title_chosen`, [`crate::Collections::title`]), which
//! stands over what is declared, as a column's label does, and is kept
//! when the table is declared again ([`crate::added::carry`]). A board, a
//! library and mockups keep their words.

use kernel::{Error, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::collections::{Spec, Store};
use crate::fields::Field;

/// A heading, and the fields that are drawn under it, in order.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Group {
    pub name: String,
    pub fields: Vec<String>,
}

/// What a table says of how it is shown. In its row it is kept whole in
/// the column called `beside`, which was all it said when that was made.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Shown {
    /// The table it sits beside, by its name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub beside: Option<String>,
    /// The field of this one that says what of the other a row is about.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub about: Option<String>,
    /// Its columns in groups, in the order they are drawn in.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub groups: Vec<Group>,
    /// `Some(false)` where its rows are not written by hand; nothing
    /// otherwise, which is what a table that says `by_hand: true` keeps
    /// too, since that is what saying nothing means.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub by_hand: Option<bool>,
    /// What it is called for people, as it is declared.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// What somebody called it, which stands over what is declared.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title_chosen: Option<String>,
    /// What it is for, in a sentence, said under its head where it is
    /// shown (`description`, as a blueprint writes it).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// The most letters what a table is for is said in.
pub const LONGEST_DESCRIPTION: usize = 200;

/// The most letters a table's title has.
pub const LONGEST_TITLE: usize = 80;

/// What a table is called for people, trimmed, or why `written` is not
/// that; nothing where it is empty.
pub fn titled(written: &str) -> Result<Option<String>> {
    let title = written.trim();
    if title.chars().any(char::is_control) {
        return Err(Error::invalid("what a table is called is one line of words"));
    }
    let letters = title.chars().count();
    if letters > LONGEST_TITLE {
        return Err(Error::invalid(format!(
            "what a table is called is {LONGEST_TITLE} letters at most, and this is {letters}"
        )));
    }
    Ok((!title.is_empty()).then(|| title.to_string()))
}

impl Shown {
    /// As it is kept in its table's row; nothing, where it says nothing.
    pub(crate) fn kept(&self) -> Option<String> {
        (*self != Shown::default()).then(|| serde_json::to_string(self).unwrap_or_default())
    }

    /// What a table's row keeps of it; nothing said, for a row from
    /// before a table could say so.
    pub(crate) fn read(kept: Option<String>) -> Shown {
        kept.and_then(|kept| serde_json::from_str(&kept).ok()).unwrap_or_default()
    }

    /// Whether the table called `name`, with these fields and kept so,
    /// can say this.
    pub(crate) fn fits(&self, name: &str, fields: &[Field], store: Store) -> Result<()> {
        let has = |field: &str| fields.iter().any(|one| one.name == field);
        let it_has = || fields.iter().map(|field| field.name.as_str()).collect::<Vec<_>>().join(", ");
        let own_page = name == crate::board::BOARD || matches!(store, Store::Documents | Store::Mockups);
        match (&self.beside, &self.about) {
            (Some(beside), _) if beside == name => {
                return Err(Error::invalid(format!(
                    "`{name}` says it sits beside `{name}`, which is itself; `beside` names another table of the \
                     same project, as in `beside: board`"
                )));
            }
            (Some(beside), _) if own_page => {
                return Err(Error::invalid(format!(
                    "`{name}` says it sits beside `{beside}`, and a board, a library and mockups each have a \
                     page of their own under their project: only a table of plain records sits beside another"
                )));
            }
            _ if self.by_hand == Some(false) && own_page => {
                return Err(Error::invalid(format!(
                    "`{name}` says its rows are not written by hand (`by_hand: false`), and a board, a library \
                     and mockups each have a page of their own under their project: only a table of plain records \
                     says so"
                )));
            }
            (_, Some(about)) if !has(about) => {
                return Err(Error::invalid(format!(
                    "`{name}` says what a row of it is about is in its field `{about}` (`about`), and it has no \
                     field called that; it has: {}",
                    it_has()
                )));
            }
            _ => {}
        }
        let mut seen: Vec<(&str, &str)> = Vec::new();
        for (group, field) in
            self.groups.iter().flat_map(|group| group.fields.iter().map(move |field| (group, field)))
        {
            if !has(field) {
                return Err(Error::invalid(format!(
                    "the group `{}` of `{name}` names `{field}`, and `{name}` has no field called that; it has: {}",
                    group.name,
                    it_has()
                )));
            }
            if let Some((before, _)) = seen.iter().find(|(_, named)| named == field) {
                return Err(Error::invalid(format!(
                    "`{field}` of `{name}` is named in the group `{before}` and again in `{}`: a field is drawn \
                     in one place, under one heading",
                    group.name
                )));
            }
            seen.push((&group.name, field));
        }
        Ok(())
    }
}

impl Spec {
    /// The same, said to sit beside another table of its project
    /// (`beside`, as a blueprint writes it, which is that table's name),
    /// each row about what the field `about` says of that table. A
    /// fleet's `stages` say `beside: board` and `about: column`.
    pub fn sitting(mut self, beside: Option<&str>, about: Option<&str>) -> Result<Spec> {
        fn said(written: Option<&str>) -> Option<&str> {
            written.map(str::trim).filter(|word| !word.is_empty())
        }
        let (beside, about) = (said(beside), said(about));
        // A name as a table's is written, and no other: one that says a
        // project before it would otherwise be read as a table of this
        // project that is not there, and nobody told.
        if let Some(written) = beside.filter(|written| blueprint::expr::slug(written) != *written) {
            return Err(Error::invalid(format!(
                "`beside` names another table of the same project, as in `beside: board`; `{written}` is not a \
                 table's name, and a table sits beside one of its own project and of no other"
            )));
        }
        if let (None, Some(about)) = (beside, about) {
            return Err(Error::invalid(format!(
                "`about: {about}` says which field says what a row is about of the table this one sits beside, and \
                 no `beside` says which table that is"
            )));
        }
        (self.shown.beside, self.shown.about) = (beside.map(str::to_string), about.map(str::to_string));
        Ok(self)
    }

    /// The same, saying whether its rows are written by hand: `by_hand`,
    /// as a blueprint writes it. `false` is a table that commands and
    /// blueprints fill and people read; `true`, and nothing written, is
    /// any other table.
    pub fn by_hand(mut self, written: &Value) -> Result<Spec> {
        self.shown.by_hand = match written {
            Value::Null | Value::Bool(true) => None,
            Value::Bool(false) => Some(false),
            _ => {
                return Err(Error::invalid(format!(
                    "`by_hand` is `false` for a table whose rows are written only by the commands and blueprints \
                     that keep it, and `true` (the usual) for one people write in too; `{}` is neither",
                    blueprint::expr::display(written)
                )));
            }
        };
        Ok(self)
    }

    /// The same, called something for people beside its name: `title`,
    /// as a blueprint writes it.
    pub fn titled(mut self, written: Option<&str>) -> Result<Spec> {
        self.shown.title = written.map(titled).transpose()?.flatten();
        Ok(self)
    }

    /// The same, saying what it is for in a sentence: `description`, as
    /// a blueprint writes it.
    pub fn described(mut self, written: Option<&str>) -> Result<Spec> {
        let said = written.map(str::trim).filter(|said| !said.is_empty());
        if let Some(said) = said {
            let letters = said.chars().count();
            if said.chars().any(char::is_control) || letters > LONGEST_DESCRIPTION {
                return Err(Error::invalid(format!(
                    "what a table is for is said in one line of {LONGEST_DESCRIPTION} letters at most, and this is \
                     {letters} letters{}",
                    if said.chars().any(char::is_control) { " over more than one line" } else { "" }
                )));
            }
        }
        self.shown.description = said.map(str::to_string);
        Ok(self)
    }

    /// The same, with its columns in groups: `groups`, as a blueprint
    /// writes it, a list of headings each with the fields under it.
    pub fn grouped(mut self, written: &Value) -> Result<Spec> {
        let wrong = || {
            Error::invalid(
                "`groups` is a list of headings in the order they are drawn, each with the fields under it in \
                 order, as in `groups: [{ \"Who they are\": [role, count] }, { \"What they are told\": [told] }]`",
            )
        };
        let listed = match written {
            Value::Null => return Ok(self),
            Value::Array(listed) => listed,
            _ => return Err(wrong()),
        };
        for one in listed {
            let said = one.as_object().filter(|said| said.len() == 1).and_then(|said| said.iter().next());
            let (name, fields) =
                said.and_then(|(name, fields)| Some((name.trim(), fields.as_array()?))).ok_or_else(wrong)?;
            if name.is_empty() {
                return Err(wrong());
            }
            let fields = fields.iter().map(blueprint::expr::display).collect();
            self.shown.groups.push(Group { name: name.to_string(), fields });
        }
        Ok(self)
    }
}

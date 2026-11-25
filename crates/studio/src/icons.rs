//! The icon a table's column is drawn with: a name, and whose it is to say.
//!
//! A field may say its icon where its table is declared (`icon`), and
//! somebody who looks at the table may choose another. Either is the
//! field's, kept with the table for everybody who opens it: never one
//! person's, and never one view's. What somebody chose stands over what
//! the declaration says, and is theirs as a view they saved is: it is
//! kept when the table is declared again, by the field's name
//! ([`crate::added::carry`], with whatever else somebody chose of a
//! column), and goes with a field that is gone. Taken away, what is declared is drawn
//! again, or the mark of what kind of thing the field holds.
//!
//! An icon is a name and nothing more here. Which drawing a name is, is
//! whoever draws it to know: this program keeps no set of them, so a set
//! that changes is no change here, and a name nothing draws fails
//! nothing.

use journal::optional;
use kernel::{Error, Principal, Result, Source};
use rusqlite::params;
use serde_json::{Value, json};

use crate::collections::{COLLECTION, Collection, Collections, missing, read_collection};

/// The most letters an icon's name has.
pub const LONGEST: usize = 48;

/// An icon's name as it is kept, or why what was written is not one:
/// lowercase letters, digits and dashes, so that it is said one way and
/// can be written into an address or a text as it is.
pub fn name(written: &str) -> Result<String> {
    let name = written.trim();
    let letter = |byte: u8| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-';
    if !name.is_empty() && name.len() <= LONGEST && name.bytes().all(letter) {
        return Ok(name.to_string());
    }
    // What is said back is no longer than a name may be, whatever was sent.
    let shown: String = written.chars().take(LONGEST).collect();
    Err(Error::invalid(format!(
        "an icon is said by its name, which is lowercase letters, digits and dashes, {LONGEST} at most, as in \
         `calendar-days`; `{shown}` is not such a name"
    )))
}

/// The icon a field says as its table is declared (`icon`), where it says one.
pub(crate) fn declared(field: &str, said: Option<&Value>) -> Result<Option<String>> {
    match said {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(written)) => name(written)
            .map(Some)
            .map_err(|err| Error::invalid(format!("the field `{field}` says `icon`, and {}", err.message))),
        Some(other) => Err(Error::invalid(format!(
            "the field `{field}` says `icon`, which is the name of the icon its column is drawn with, as in \
             `icon: calendar-days`; it says {other}"
        ))),
    }
}

impl Collections {
    /// Chooses the icon a field's column is drawn with, for whoever looks
    /// at the table next; or, with none, takes the choice away, and what
    /// the table's declaration says is drawn again.
    pub async fn choose_icon(
        &self,
        project: &str,
        name: &str,
        field: &str,
        icon: Option<&str>,
        by: &Principal,
    ) -> Result<Collection> {
        // How a column is drawn is the table's, as a view of it is: chosen
        // by whoever may save one.
        if !by.can("board:write") {
            by.require("records:write")?;
        }
        // A table that is kept from whoever asks is not there for them,
        // in the words it would not be there in.
        let table = self.get(project, name).await?;
        if table.open_to(by).is_err() {
            return Err(missing(project, name));
        }
        table.may_change(by)?;
        let icon = icon.map(self::name).transpose()?;
        let no_field = || Error::not_found(format!("`{name}` in `{project}` has no field called `{field}`"));
        if table.field(field).is_none() {
            return Err(no_field());
        }
        // Read and written as one piece of work, and only the fields: two
        // icons chosen side by side are both kept, and what else the row
        // says is as whoever wrote it last left it.
        let now = self.factory().now_ms();
        let (of_project, called, of_field, chosen) =
            (project.to_string(), name.to_string(), field.to_string(), icon.clone());
        let written: Option<Collection> = self
            .rows(project)
            .db()
            .call(move |connection| {
                let read = connection.query_row(
                    &format!("SELECT {COLLECTION} FROM collections WHERE project = ?1 AND name = ?2"),
                    [&of_project, &called],
                    read_collection,
                );
                let Some(mut table) = optional(read)? else { return Ok(None) };
                let Some(one) = table.fields.iter_mut().find(|one| one.name == of_field) else {
                    return Ok(None);
                };
                one.icon_chosen = chosen;
                table.updated = now;
                connection.execute(
                    "UPDATE collections SET fields = ?3, updated = ?4 WHERE project = ?1 AND name = ?2",
                    params![
                        of_project,
                        called,
                        serde_json::to_string(&table.fields).unwrap_or_default(),
                        now
                    ],
                )?;
                Ok(Some(table))
            })
            .await?;
        let table = written.ok_or_else(no_field)?;
        let mut data = json!({ "project": project, "name": name, "field": field });
        if let Some(icon) = icon {
            data["icon"] = json!(icon);
        }
        // Of a table kept private it is heard by whoever may read the table.
        let source = Source::actor(by.actor()).taking(table.reads.as_deref());
        let _ = self.factory().emit("collection.icon", source, data).await;
        Ok(table)
    }
}

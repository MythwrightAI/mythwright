//! Numbers written before a field was a choice, come to its words.
//!
//! A table that ships is declared again over the rows a factory from
//! before holds, and has to take what they hold. A card's priority was a
//! number, the higher first and nothing as nought, and is now a word
//! (`urgent`, `high`, `normal`, `low`, nothing standing for `normal`).
//! A choice that says what nothing stands for (`unsaid`) takes a number
//! as the word it is counted to from there (`kernel::ranked::word_of`,
//! asked where any value is fitted to a field), so the rows that hold one
//! fit it as it is declared. And once it is declared, each number a row
//! still holds of it is written as its word, each record in the write
//! that says so of it (`record.reworded`, a line of its history). A row
//! holds a number of a choice no longer once it is written, so it is done
//! once, as the field comes to be such a choice or comes to have other
//! words (`held::rewordable`): declared again as it is, nothing is asked
//! of the rows at all. That a field is being reworded is kept on it in
//! the table's own row (`Field::rewording`), written with the declaration
//! that begins it and taken off once every row is a word, so a copy and a
//! move apart carry it as they carry the row: stopped half way, it is
//! finished by the next declaration or, sooner, as the daemon starts, each
//! row said once as it always was. A choice that says nothing of what
//! nothing stands for has nowhere to count a number from, and is refused
//! over rows that hold numbers, as it always was.

use kernel::{Principal, Result, Source};
use serde_json::{Map, Value, json};

use crate::collections::{Collection, Collections};
use crate::fields::Field;
use crate::whole::Say;

/// The word a value held of `field` comes to, where it is a number and the
/// field a choice that says what nothing stands for.
fn word(field: &Field, held: &Value) -> Option<String> {
    let number = held.as_f64().filter(|_| held.is_number() && field.kind == "choice")?;
    kernel::ranked::word_of(number, &field.of, field.unsaid.first().map(String::as_str))
}

impl Collections {
    /// Writes each number a record of `collection` holds of a choice as
    /// the word it comes to, record by record under its turn and as it is
    /// then, each with what is said of it in the same write: done through
    /// the blueprint whose step declares the table, where one does.
    /// `fields` are those whose numbers it is to take as words.
    pub(crate) async fn reword(
        &self,
        collection: &Collection,
        fields: &[String],
        by: &Principal,
        through: Option<&str>,
    ) -> Result<()> {
        if fields.is_empty() {
            return Ok(());
        }
        for (id, number) in self.numbered(collection, fields).await? {
            let _turn = self.changing(&id).await;
            let Some(mut record) = self.one(collection, &id, number).await? else { continue };
            let mut said = Vec::new();
            for field in &collection.fields {
                let held = record.data.get(&field.name).cloned().unwrap_or(Value::Null);
                let Some(word) = word(field, &held) else { continue };
                record.data.insert(field.name.clone(), Value::String(word.clone()));
                let mut data: Map<String, Value> = Map::new();
                for (key, value) in [
                    ("id", json!(record.id)),
                    ("project", json!(record.project)),
                    ("collection", json!(record.collection)),
                    ("field", json!(field.name)),
                    ("was", held),
                    ("now", json!(word)),
                ] {
                    data.insert(key.into(), value);
                }
                if let Some(through) = through {
                    data.insert("through".into(), json!(through));
                }
                said.push(Value::Object(data));
            }
            if said.is_empty() {
                continue;
            }
            let source = Source::actor(by.actor()).taking(collection.reads.as_deref());
            let say =
                said.into_iter().map(|data| Say::new("record.reworded", (source.clone(), data))).collect();
            self.put_saying(collection, record, |_| say).await?;
        }
        self.reworded(collection, fields).await
    }

    /// Takes off the mark that `fields` of `collection` are being
    /// reworded, from the table's row as it is now.
    async fn reworded(&self, collection: &Collection, fields: &[String]) -> Result<()> {
        let (project, name, fields) = (collection.project.clone(), collection.name.clone(), fields.to_vec());
        self.rows(&collection.project)
            .db()
            .call(move |connection| {
                let kept: Option<String> = journal::optional(connection.query_row(
                    "SELECT fields FROM collections WHERE project = ?1 AND name = ?2",
                    [&project, &name],
                    |row| row.get(0),
                ))?;
                let Some(mut kept) = kept.and_then(|kept| serde_json::from_str::<Vec<Field>>(&kept).ok())
                else {
                    return Ok(());
                };
                let marked = |field: &Field| field.rewording && fields.contains(&field.name);
                if !kept.iter().any(marked) {
                    return Ok(());
                }
                for field in kept.iter_mut().filter(|field| fields.contains(&field.name)) {
                    field.rewording = false;
                }
                connection.execute(
                    "UPDATE collections SET fields = ?3 WHERE project = ?1 AND name = ?2",
                    [&project, &name, &serde_json::to_string(&kept).unwrap_or_default()],
                )?;
                Ok(())
            })
            .await
    }

    /// Finishes, as the daemon starts, what a declaration began writing
    /// as words and a stop left half done, as the system. Where it cannot
    /// be finished now the mark stays, and the next declaration of the
    /// table finishes it.
    pub(crate) async fn reword_left(&self, collection: &Collection) {
        let left: Vec<String> = collection
            .fields
            .iter()
            .filter(|field| field.rewording)
            .map(|field| field.name.clone())
            .collect();
        let _ = self.reword(collection, &left, &Principal::system(), None).await;
    }
}

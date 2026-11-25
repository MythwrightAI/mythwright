//! What is written inside an argument.
//!
//! An argument of an action is often a mapping with words of its own: the
//! fields of a table, each by a name with its `type` and whether it is
//! `quiet`; a health check, which is `tcp`, `http` or `cmd`. The action
//! reads those words, and nothing in the language knows them. This is how
//! an action says what they are, so that whoever shows a blueprint can say
//! what one of them means and offer those not yet written: an editor, the
//! notes, the command line, a schema, and a form of a step's arguments.
//!
//! It describes and never decides. Nothing is checked by it, when a
//! blueprint is read, checked or run: a key it does not name is one it
//! says nothing of, since the action is what takes or refuses it, and a
//! text may be older than this description.
//!
//! It is said as rows, so that it can be a plain table in any crate: where
//! something is written, what kind of thing it is, and a sentence.
//!
//! ```text
//! ("*", "each", "a field of the table")
//! ("*.type", "string", "what it holds")
//! ("*.type.text", "word", "words, as they are written")
//! ("*.quiet", "bool", "a change to it alone is not news of its record")
//! ```
//!
//! Where is a path from the argument down, its parts joined by dots: a
//! key by its name, and `*` for a thing known by a name of its writer's
//! choosing. The kind is a type an argument can have ([`ValueType`]), or
//! one of two words: `each`, of a path that ends in `*`, whose sentence
//! says what each such thing is; and `word`, of one of the few words the
//! key before it may be, or that each thing of the `*` before it is. A
//! list of mappings is said as one of them is: the keys an item takes.

use serde::Serialize;

use crate::types::ValueType;

/// One line of such a table: where, what kind, and the sentence.
pub type Row<'a> = (&'a str, &'a str, &'a str);

/// The kind of a row that says what each thing known by a name is.
pub const EACH: &str = "each";
/// The kind of a row that says one of a few words.
pub const WORD: &str = "word";

/// What a mapping an action reads holds.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Within {
    /// Where it holds things by a name of their writer's choosing: what
    /// each of them is, as it reads after `is` (`a field of the table`).
    /// Nothing, where the keys below are written in the mapping itself.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub each: Option<String>,
    /// The keys that are taken: by each such thing, or by the mapping.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub keys: Vec<Key>,
    /// Where each such thing is a word and no mapping: the few it is one of.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub words: Vec<Word>,
}

/// One key of such a mapping.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Key {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: ValueType,
    pub help: String,
    /// Where it is one of a few words: those words.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub words: Vec<Word>,
    /// What it holds in turn, where it is a mapping that is described.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub within: Option<Within>,
}

/// A word that may be written, and what it means where that is worth saying.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Word {
    pub name: String,
    pub help: String,
}

impl Within {
    /// Reads a table of rows. A row that fits nowhere (its path goes
    /// through a key no row before it said, or its kind is no kind) is
    /// left out: a test of each table holds what comes back from
    /// [`Within::rows`] to the table.
    pub fn of(rows: &[Row]) -> Within {
        let mut within = Within::default();
        for (path, kind, says) in rows {
            let parts: Vec<&str> = path.split('.').collect();
            within.put(&parts, kind, says);
        }
        within
    }

    /// The mapping a path leads to. A `*` is the things of the mapping it
    /// is reached in, which are described where the mapping is.
    fn at(&mut self, path: &[&str]) -> Option<&mut Within> {
        let Some((first, rest)) = path.split_first() else { return Some(self) };
        if *first == "*" {
            return if self.each.is_some() { self.at(rest) } else { None };
        }
        let key = self.keys.iter_mut().find(|key| key.name == *first)?;
        // Only the key a row is written directly under comes to hold
        // something by it: a path that goes on through it and fits
        // nowhere leaves the key as it was.
        match rest.is_empty() {
            true => Some(key.within.get_or_insert_with(Within::default)),
            false => key.within.as_mut()?.at(rest),
        }
    }

    fn put(&mut self, path: &[&str], kind: &str, says: &str) -> Option<()> {
        let (last, before) = path.split_last()?;
        let word = || Word { name: last.to_string(), help: says.to_string() };
        match kind {
            EACH if *last == "*" => self.at(before)?.each = Some(says.to_string()),
            EACH => return None,
            // Of the things a `*` stands for, or of the key before it.
            WORD if before.last() == Some(&"*") => self.at(before)?.words.push(word()),
            WORD => {
                let (key, before) = before.split_last()?;
                let key = self.at(before)?.keys.iter_mut().find(|one| one.name == *key)?;
                key.words.push(word());
            }
            ty => {
                let ty = ValueType::parse(ty)?;
                let key = Key {
                    name: last.to_string(),
                    ty,
                    help: says.to_string(),
                    words: Vec::new(),
                    within: None,
                };
                self.at(before)?.keys.push(key);
            }
        }
        Some(())
    }

    /// The same as rows, in the order a table is written in: what each
    /// thing is, the words it may be, each key with its words, and then
    /// what each key holds in turn.
    pub fn rows(&self) -> Vec<(String, String, String)> {
        let mut rows = Vec::new();
        self.written("", &mut rows);
        rows
    }

    fn written(&self, under: &str, rows: &mut Vec<(String, String, String)>) {
        let mut row =
            |path: String, kind: &str, says: &str| rows.push((path, kind.to_string(), says.to_string()));
        let under = match &self.each {
            Some(each) => {
                row(format!("{under}*"), EACH, each);
                format!("{under}*.")
            }
            None => under.to_string(),
        };
        for word in &self.words {
            row(format!("{under}{}", word.name), WORD, &word.help);
        }
        for key in &self.keys {
            let path = format!("{under}{}", key.name);
            row(path.clone(), key.ty.name(), &key.help);
            for word in &key.words {
                row(format!("{path}.{}", word.name), WORD, &word.help);
            }
        }
        // What each key holds in turn, after the keys beside it: a table
        // reads a mapping whole before it goes into one of its keys.
        for key in &self.keys {
            if let Some(within) = &key.within {
                within.written(&format!("{under}{}.", key.name), rows);
            }
        }
    }
}

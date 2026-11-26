//! Models: the records this program itself reads, field by field.
//!
//! What a collection's records are made of is whatever its blueprint
//! declares, and nearly all of it is kept, checked and handed on without
//! anything here knowing what it means. A few records are different: the
//! board moves a card by its `status`, and reads and writes the rest of a
//! card by name. Code that reads a field by its name is right only while
//! the field is there and is what the code takes it to be, and a blueprint
//! is a text that can be written, or changed, long after this was built.
//!
//! A model is that understanding written once: the fields, and beside
//! them the struct the code reads them through. The compiler holds the
//! code to the struct, a test holds the struct to the description, and a
//! collection is held to the description when its blueprint is applied:
//! one that says it is a model's (`model: card`) may write the model's
//! fields out, and must then say of each what the model says. What it
//! leaves out that a record can do without is there all the same, so that
//! a field this program learns to keep is on every such collection there
//! already is, with nothing applied again.
//!
//! There is a model only where code here reads fields by name. Anything
//! else a blueprint declares is its own, and needs none.

use kernel::{Error, Result};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Map, Value};

use crate::fields::Field;

/// One field as a model has it. What a choice can be is not the model's
/// to say: a board's columns are its blueprint's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Has {
    pub name: &'static str,
    /// One of `fields::KINDS`.
    pub kind: &'static str,
    pub required: bool,
    /// Kept by the program for itself: it is no line of a record's
    /// history when it changes, and no column or cell that is shown.
    pub own: bool,
}

impl Has {
    /// The field as a collection declares it, with what can be chosen
    /// where it is a choice.
    pub fn field(&self, of: Vec<String>) -> Field {
        Field {
            name: self.name.into(),
            kind: self.kind.into(),
            of,
            required: self.required,
            quiet: self.own,
            takes: Default::default(),
            takes_of: std::collections::BTreeMap::new(),
            takes_each: false,
            stands: false,
            unsaid: Vec::new(),
            hidden: self.own,
            suggests: Vec::new(),
            zoned: false,
            said: false,
            writer_of: None,
            icon: None,
            icon_chosen: None,
            added: false,
            label_chosen: None,
            of_chosen: Vec::new(),
            of_added: None,
            rewording: false,
            calls: None,
            indexed: false,
        }
    }

    /// Whether a collection can be given the field without being told
    /// anything more: a choice needs what can be chosen, a record the
    /// collection it names, and those only a blueprint says.
    fn stands_alone(&self) -> bool {
        !matches!(self.kind, "choice" | "record" | "records")
    }
}

/// How a field is spoken of when two descriptions of it differ.
fn said(kind: &str, required: bool) -> String {
    format!("a `{kind}` that {}", if required { "is needed" } else { "may be left empty" })
}

#[derive(Clone, Copy, Debug)]
pub struct Model {
    pub name: &'static str,
    pub fields: &'static [Has],
    /// The collection that is one of these whether or not its blueprint
    /// says so: the code that reads the model reads that collection, as it
    /// did before a blueprint could say.
    pub home: Option<&'static str>,
    /// What writes the fields it keeps for itself (`own`), for whoever is
    /// refused one at a door that writes any record.
    pub doors: &'static str,
}

/// What a card is: the fields `Card` reads, in the order they are shown.
pub const CARD: Model = Model {
    name: "card",
    fields: &[
        Has { name: "title", kind: "text", required: true, own: false },
        Has { name: "body", kind: "text", required: false, own: false },
        Has { name: "status", kind: "choice", required: true, own: false },
        Has { name: "assignee", kind: "actor", required: false, own: false },
        Has { name: "labels", kind: "list", required: false, own: false },
        Has { name: "due", kind: "date", required: false, own: false },
        // How it came to be where it is ([`Moved`]).
        Has { name: MOVED, kind: "list", required: false, own: true },
        // What was pressed on it of what its column offers
        // (`kernel::pressed::Pressed`), written by the board alone.
        Has { name: PRESSED, kind: "list", required: false, own: true },
    ],
    home: Some("board"),
    doors: "the board, as a card is moved and pressed",
};

/// The field of a card that says how it came to be where it is.
pub const MOVED: &str = "moved";

/// The field of a card that keeps what was pressed on it.
pub const PRESSED: &str = "pressed";

/// What can be pressed on a card in a column (`kernel::pressed::Action`
/// reads a row of one): a table that says it is one of these, and sits
/// beside a board, offers its rows on that board's cards, and the board
/// asks them who may press and what holds a card where it is.
pub const ACTION: Model = Model {
    name: "action",
    fields: &[
        Has { name: "title", kind: "text", required: true, own: false },
        Has { name: "name", kind: "text", required: false, own: false },
        Has { name: "column", kind: "text", required: false, own: false },
        Has { name: "who", kind: "choice", required: false, own: false },
        Has { name: "scope", kind: "text", required: false, own: false },
        Has { name: "holds", kind: "flag", required: false, own: false },
        Has { name: "again", kind: "flag", required: false, own: false },
        Has { name: "says", kind: "text", required: false, own: false },
    ],
    home: None,
    doors: "nothing",
};

/// What a topic is (`kernel::topics`): a forum's topic, a row of the
/// factory's own table of them, and a page of a wiki. Where it is, the
/// page it is under and who follows it, the keeper of topics writes
/// through its own doors, and nothing else does.
pub const TOPIC: Model = Model {
    name: kernel::topics::MODEL,
    fields: &[
        Has { name: "title", kind: "text", required: true, own: false },
        Has { name: "body", kind: "document", required: false, own: false },
        Has { name: "summary", kind: "document", required: false, own: false },
        Has { name: "status", kind: "choice", required: false, own: false },
        Has { name: "workspace", kind: "text", required: false, own: true },
        Has { name: "project", kind: "text", required: false, own: true },
        Has { name: "followers", kind: "list", required: false, own: true },
        Has { name: "answer", kind: "text", required: false, own: false },
        // The page it is under, by its id; nothing for a wiki.
        Has { name: kernel::topics::PARENT, kind: "text", required: false, own: true },
    ],
    home: None,
    doors: "the topic's own commands (New page, Move to… and Follow in the Topics panel and on its page, \
            `topic make --under`, `topic move` and `topic follow`)",
};

/// Every model there is.
pub const MODELS: &[Model] = &[CARD, ACTION, TOPIC];

/// Says that `named` writes none of the fields `collection`'s model keeps
/// for itself (`own`): those are written through the model's own doors.
pub fn own_refused<'a>(
    collection: &crate::Collection,
    mut named: impl Iterator<Item = &'a String>,
) -> Result<()> {
    let Some(model) =
        collection.model.as_deref().and_then(|model| MODELS.iter().find(|one| one.name == model))
    else {
        return Ok(());
    };
    match named.find(|name| model.fields.iter().any(|has| has.own && has.name == name.as_str())) {
        None => Ok(()),
        Some(name) => Err(Error::invalid(format!(
            "`{name}` of a `{}` is written by {} and by nothing else",
            model.name, model.doors
        ))),
    }
}

/// The model of the collection called `collection`: the one its blueprint
/// names (`said`), or the one that collection always is.
pub fn of(collection: &str, said: Option<&str>) -> Result<Option<&'static Model>> {
    match said.map(str::trim).filter(|said| !said.is_empty()) {
        Some(said) => MODELS.iter().find(|model| model.name == said).map(Some).ok_or_else(|| {
            let known: Vec<&str> = MODELS.iter().map(|model| model.name).collect();
            Error::invalid(format!(
                "`{said}` is not a model a collection can be; there is: {}. A collection that is none of them says no `model`, and its fields are all its own",
                known.join(", ")
            ))
        }),
        None => Ok(MODELS.iter().find(|model| model.home == Some(collection))),
    }
}

impl Model {
    /// Whether `name` is one of the model's own fields.
    pub fn has(&self, name: &str) -> bool {
        self.fields.iter().any(|has| has.name == name)
    }

    /// The fields of `collection` as its blueprint declared them, held to
    /// the model: one of the model's that is declared must be what the
    /// model says it is, one that is not declared is added where a record
    /// can do without it, and the rest are the blueprint's own.
    pub fn hold(&self, collection: &str, mut declared: Vec<Field>) -> Result<Vec<Field>> {
        for has in self.fields {
            match declared.iter().find(|field| field.name == has.name) {
                Some(field) if field.kind != has.kind || field.required != has.required => {
                    return Err(Error::invalid(format!(
                        "`{collection}` is a `{}`, whose `{}` is {}; it is declared here as {}. Write it as the model has it, or leave it out",
                        self.name,
                        has.name,
                        said(has.kind, has.required),
                        said(&field.kind, field.required)
                    )));
                }
                Some(_) => {}
                None if has.required || !has.stands_alone() => {
                    return Err(Error::invalid(format!(
                        "`{collection}` is a `{}`, which has `{}`: {}. Declare it among the fields",
                        self.name,
                        has.name,
                        said(has.kind, has.required)
                    )));
                }
                None => declared.push(has.field(Vec::new())),
            }
        }
        Ok(declared)
    }

    /// Gives a collection declared before the model had them the fields
    /// it would be given now, so that what reads the model finds them.
    pub(crate) fn complete(&self, fields: &mut Vec<Field>) {
        for has in self.fields.iter().filter(|has| has.stands_alone()) {
            if !fields.iter().any(|field| field.name == has.name) {
                fields.push(has.field(Vec::new()));
            }
        }
    }

    /// Says that `fields`, as a blueprint writes them, write every field
    /// of the model out and say of each what the model says. A collection
    /// is not held to this when it is declared, where a field left out is
    /// simply given; the blueprints that ship are, so that each can be
    /// read by itself and says the whole of what it makes.
    pub fn written(&self, fields: &Value) -> Result<()> {
        for has in self.fields {
            let Some(about) = fields.get(has.name) else {
                return Err(Error::invalid(format!(
                    "the model `{}` has `{}` ({}), and it is not written among the fields",
                    self.name,
                    has.name,
                    said(has.kind, has.required)
                )));
            };
            let kind = about.get("type").and_then(Value::as_str).unwrap_or("text");
            let required = about.get("required").and_then(Value::as_bool).unwrap_or(false);
            if kind != has.kind || required != has.required {
                return Err(Error::invalid(format!(
                    "the model `{}` has `{}` as {}, and it is written as {}",
                    self.name,
                    has.name,
                    said(has.kind, has.required),
                    said(kind, required)
                )));
            }
        }
        Ok(())
    }
}

/// A card, as the board reads it out of the record that holds it. Its
/// fields are `CARD`'s, one for one; a test says so.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Card {
    #[serde(default, deserialize_with = "kept")]
    pub title: String,
    #[serde(default, deserialize_with = "kept")]
    pub body: String,
    /// The column it is in.
    #[serde(default, deserialize_with = "kept")]
    pub status: String,
    /// `user:<id>` or `agent:<slot>`.
    #[serde(default, deserialize_with = "kept")]
    pub assignee: Option<String>,
    #[serde(default, deserialize_with = "kept")]
    pub labels: Vec<String>,
    /// The day it is wanted by, `2026-11-30`.
    #[serde(default, deserialize_with = "kept")]
    pub due: Option<String>,
    /// How it came to be where it is, written with the move that put it
    /// there and by nothing else ([`Moved`]); empty for a card that was
    /// never moved.
    #[serde(default, deserialize_with = "kept")]
    pub moved: Vec<String>,
    /// What was pressed on it of what its column offers, written by
    /// nothing but the board (`kernel::pressed::Pressed`).
    #[serde(default, deserialize_with = "kept")]
    pub pressed: Vec<String>,
    /// What else its board keeps of it: the fields the board's blueprint
    /// declared beyond a card's own, each by its name, where it has one.
    /// Nothing here reads them; they are carried.
    #[serde(default, skip_serializing_if = "Map::is_empty")]
    pub more: Map<String, Value>,
}

/// How a card came to be where it is: what its row says of the move that
/// put it there, in the same write as the move.
///
/// What is done with a card when it comes somewhere is decided from its
/// row, and a row that said only where the card is left the rest to
/// whoever heard the move: why it was sent there was known to a handler,
/// and whatever looked at the row before that handler had its turn took
/// the card for one nobody had said anything of. So the row says it: the
/// column it was moved from, the kind of actor that moved it, what was
/// said as it was moved, and how many times it has been moved, which is
/// what tells one move from the next for whoever has to say which of
/// them it has taken up. It is kept as a list of four words, as a record
/// keeps any list.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Moved {
    /// The column it was moved from.
    pub from: String,
    /// The kind of actor that moved it: what its name begins with
    /// (`user`, `key`, `agent`, `bot`, `system`), or `blueprint` where a
    /// step did.
    pub by: String,
    /// What was said as it was moved; empty where nothing was.
    pub said: String,
    /// How many times it has been moved between columns, this one counted.
    pub count: u64,
}

impl Moved {
    /// What a card's `moved` says; nothing for a card never moved, or
    /// whose row says something that is not this.
    pub fn read(kept: &[String]) -> Option<Moved> {
        let [from, by, said, count] = kept else { return None };
        let count = count.parse().ok()?;
        Some(Moved { from: from.clone(), by: by.clone(), said: said.clone(), count })
    }

    /// As a card's `moved` keeps it.
    pub fn kept(&self) -> Vec<String> {
        vec![self.from.clone(), self.by.clone(), self.said.clone(), self.count.to_string()]
    }
}

/// Reads one field of a record, taking what does not fit as nothing.
/// Records kept as files are written by people as well, and a card with
/// one odd value is still a card: reading must not fail on it.
fn kept<'de, D, T>(from: D) -> std::result::Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Default + DeserializeOwned,
{
    Ok(serde_json::from_value(Value::deserialize(from)?).unwrap_or_default())
}

impl Card {
    /// The card a record's `data` holds.
    pub fn read(mut data: Map<String, Value>) -> Card {
        let mut own = Map::new();
        for has in CARD.fields {
            if let Some(value) = data.remove(has.name) {
                own.insert(has.name.to_string(), value);
            }
        }
        data.retain(|_, value| !value.is_null());
        own.insert("more".into(), Value::Object(data));
        serde_json::from_value(Value::Object(own)).unwrap_or_default()
    }

    /// The card as a record's `data` holds it.
    pub fn data(&self) -> Map<String, Value> {
        let mut data = match serde_json::to_value(self) {
            Ok(Value::Object(data)) => data,
            _ => Map::new(),
        };
        if let Some(Value::Object(more)) = data.remove("more") {
            data.extend(more);
        }
        data
    }
}

/// A topic, as the keeper of topics reads it out of its row. Its fields
/// are `TOPIC`'s, one for one; a test says so.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Topic {
    #[serde(default, deserialize_with = "kept")]
    pub title: String,
    /// Its Markdown.
    #[serde(default, deserialize_with = "kept")]
    pub body: String,
    /// What is known now, kept current: Markdown.
    #[serde(default, deserialize_with = "kept")]
    pub summary: String,
    /// Where it stands; nothing for a topic that is not tracked as a question.
    #[serde(default, deserialize_with = "kept")]
    pub status: Option<String>,
    /// The workspace it is the own of, where it is a workspace's.
    #[serde(default, deserialize_with = "kept")]
    pub workspace: Option<String>,
    /// The project it is of, where it is a project's.
    #[serde(default, deserialize_with = "kept")]
    pub project: Option<String>,
    /// Who follows it, each an actor.
    #[serde(default, deserialize_with = "kept")]
    pub followers: Vec<String>,
    /// The reply that answered it, by its number.
    #[serde(default, deserialize_with = "kept")]
    pub answer: Option<String>,
    /// The page it is under, by its id; nothing for a wiki.
    #[serde(default, deserialize_with = "kept")]
    pub parent: Option<String>,
}

impl Topic {
    /// The topic a record's `data` holds; what is beyond the model is
    /// left where it is, since the keeper reads nothing of it.
    pub fn read(data: &Map<String, Value>) -> Topic {
        let own: Map<String, Value> = TOPIC
            .fields
            .iter()
            .filter_map(|has| data.get(has.name).map(|value| (has.name.to_string(), value.clone())))
            .collect();
        serde_json::from_value(Value::Object(own)).unwrap_or_default()
    }
}

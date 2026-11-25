//! Records of any collection, made and changed by name.
//!
//! A blueprint that declares a collection says what its records are made
//! of; this is how one of them comes to be, is changed and is taken away,
//! by a blueprint's step, over the API or from the terminal, wherever the
//! collection is kept. Each of those is an event (`record.created`,
//! `record.updated`, `record.removed`) that says which record, of which
//! collection, of which project; so a collection may count them as the
//! history of its records, and anything may follow them.
//!
//! Something can also be said on a record, any record (`record.commented`):
//! it changes nothing, and is kept as a line of the record's history. What
//! follows from something being said is not decided here. That a card has
//! a question open, and that what someone says on it then is the answer,
//! is what the card's own blueprint makes of this event.
//!
//! What is said may be marked public by whoever says it, and is not unless
//! they do. The mark is kept, said with the event and shown in the
//! history, and that is all that is done with it here: it lets nobody
//! read anything they could not, and what is told to anybody else because
//! of it (a record that is also kept somewhere outside the factory) is a
//! blueprint's to do, hearing the event.
//!
//! A board's cards are records too. They are read here like any others
//! and spoken on like any others, and they are not written here: moving a
//! card is an event of its own that the rest of the factory hangs on, and
//! `board` is where that is done.

use journal::Journal;
use kernel::{Error, Principal, Result};
use serde_json::{Map, Value, json};

use crate::board::{BOARD, Through, told};
use crate::collections::{Collection, Collections, Record, Say};
use crate::history::rule_for;

/// What is said when something is said on a record.
pub const COMMENTED: &str = "record.commented";

/// What something said on a record says of itself where whoever said it
/// marked it public; left out, it is private.
pub const PUBLIC: &str = "public";

/// What something said on a record says of the words it is about, where
/// it is about a few of them and not the whole record.
pub const ANCHOR: &str = "anchor";

/// What a blueprint's step that says which saying it is keeps with it.
pub const ONCE: &str = "once";

/// How many of what was said lately in a project a step done again looks
/// through for its own.
const LATELY: usize = 500;

/// What is said on a record, beside who says it.
pub(crate) struct Saying<'a> {
    pub body: &'a str,
    pub public: bool,
    pub anchor: Option<&'a kernel::anchor::Anchor>,
    /// Which saying it is, in a step's words: kept with it.
    pub once: Option<&'a str>,
    /// Who asked a question it says, as what asked says them: an agent's
    /// name, or a record that asks (an A/B test, on its card).
    pub asker: Option<&'a str>,
    /// What it answers, where it is a reply (`crate::replied`).
    pub parent: Option<u64>,
    /// An agent's answer to what it was asked there, said by the factory
    /// for it ([`Records::answer`]): who it is said by is not asked
    /// whether they may change the record.
    pub answered: bool,
}

/// The most that is said on a record at once.
const MAX_SAID: usize = 16 * 1024;

/// What a change of a record's cells comes to once every check has been
/// asked of it ([`Records::checked`]).
pub enum Checking {
    /// It asks for what is so already: the record, as it is.
    Same(Record),
    /// It changes something, and may be written ([`Records::write`]).
    Change(Box<Checked>),
}

/// A change of a record's cells that every check has passed, with the
/// record as it is to be written and the names of the cells it changes.
pub struct Checked {
    collection: Collection,
    record: Record,
    changed: Vec<String>,
}

#[derive(Clone)]
pub struct Records {
    pub(crate) journal: Journal,
    pub(crate) collections: Collections,
}

/// What is told of a record, and by whom: as of a card, and, of a record
/// of a table that is kept private, with what hearing of it takes.
fn said_of(
    collection: &Collection,
    by: &Principal,
    through: Option<&Through>,
    data: Value,
) -> (kernel::Source, Value) {
    let (source, data) = told(by, through, data);
    (source.taking(collection.reads.as_deref()), data)
}

/// Says that a collection is not a board, whose cards are the board's to write.
fn not_a_board(collection: &Collection, doing: &str) -> Result<()> {
    if collection.name != BOARD {
        return Ok(());
    }
    Err(Error::invalid(format!(
        "a board's cards are not {doing} as records: moving one is an event of its own, which the rest of the factory hangs on. Use `board.add`, `board.update` and `board.move`, which `{cli} board` and the board's page do",
        cli = brand::CLI
    )))
}

impl Records {
    pub(crate) fn new(journal: Journal, collections: Collections) -> Records {
        Records { journal, collections }
    }

    /// What every event about a record says of it.
    fn about(record: &Record) -> Value {
        json!({ "id": record.id, "project": record.project, "collection": record.collection })
    }

    /// Adds a record to a project's collection, after the ones it has.
    /// `through` is the blueprint whose step this is, when it is one.
    pub async fn add(
        &self,
        project: &str,
        collection: &str,
        fields: Map<String, Value>,
        by: &Principal,
        through: Option<&Through>,
    ) -> Result<Record> {
        by.require("records:write")?;
        let collection = self.collections.get(project, &blueprint::expr::slug(collection)).await?;
        not_a_board(&collection, "added")?;
        collection.may_change(by)?;
        crate::models::own_refused(&collection, fields.keys())?;
        let last = self.collections.last_position(&collection).await?;
        let created = |record: &Record| {
            let mut about = Self::about(record);
            crate::left::mark(through, &mut about);
            vec![Say::new("record.created", said_of(&collection, by, through, about))]
        };
        self.collections.add_saying(&collection, fields, last + 1.0, by, created).await
    }

    /// Keeps a record under `number`, one of the caller's choosing that is
    /// another record's (a card's) own: added where the table has none of
    /// that number, and changed to `fields` where it has; so it is the
    /// same done twice, and found again by its id without the rest being
    /// read. Two at once for one number: the second changes what the first added.
    pub async fn keep(
        &self,
        (project, collection, number): (&str, &str, i64),
        fields: Map<String, Value>,
        by: &Principal,
        through: Option<&Through>,
    ) -> Result<Record> {
        by.require("records:write")?;
        let collection = self.collections.get(project, &blueprint::expr::slug(collection)).await?;
        not_a_board(&collection, "kept")?;
        collection.may_change(by)?;
        let id = crate::collections::record_id(project, &collection.name, number);
        if self.collections.one(&collection, &id, number).await?.is_none() {
            let last = self.collections.last_position(&collection).await?;
            let created = |record: &Record| {
                vec![Say::new("record.created", said_of(&collection, by, through, Self::about(record)))]
            };
            let added = self.collections.add_numbered(
                &collection,
                fields.clone(),
                (last + 1.0, Some(number)),
                by,
                created,
            );
            match added.await {
                Err(there) if there.kind == kernel::ErrorKind::Conflict => {}
                done => return done,
            }
        }
        self.update(&id, fields, by, through).await
    }

    /// Adds a record to a table that is declared, with the fields
    /// `declaring` says, where the project has none of that name; of one
    /// that is there, only the fields it has are written, and it keeps its
    /// own, and whether its rows are written by hand (`by_hand`) with it.
    /// Either way it is a plain table: one that says what writing or
    /// reading it takes, what its rows steer, or that is a model's, is
    /// written only by whoever writes it, and never by a blueprint that
    /// was told its name.
    pub async fn add_declaring(
        &self,
        (project, collection): (&str, &str),
        mut fields: Map<String, Value>,
        (declaring, by_hand): (&Value, &Value),
        owner: &str,
        by: &Principal,
        through: Option<&Through>,
    ) -> Result<Record> {
        let name = blueprint::expr::slug(collection);
        let table = match self.collections.find(project, &name).await? {
            Some(table) => table,
            None => {
                let spec = crate::collections::Spec::read(declaring, &Value::Null, None)?.by_hand(by_hand)?;
                self.collections.ensure(project, &name, spec, Some(owner), by).await?
            }
        };
        if table.takes.is_some() || table.reads.is_some() || table.steers.is_some() || table.model.is_some() {
            return Err(Error::invalid(format!(
                "`{name}` says what writing or reading it takes, what its rows steer, or what it is a model of, \
                 and a row is kept there only by whoever writes it: name a table that says none of that, or one \
                 the project has not got, which is then made"
            )));
        }
        fields.retain(|field, _| table.field(field).is_some());
        self.add(project, &name, fields, by, through).await
    }

    /// Changes the fields named, and leaves the others as they are. One
    /// given nothing is cleared. Saying what is already so changes nothing
    /// and is not an event.
    pub async fn update(
        &self,
        id: &str,
        fields: Map<String, Value>,
        by: &Principal,
        through: Option<&Through>,
    ) -> Result<Record> {
        by.require("records:write")?;
        let _turn = self.collections.changing(id).await;
        match self.checked(id, fields, by).await? {
            Checking::Same(record) => Ok(record),
            Checking::Change(checked) => self.write(*checked, by, through).await,
        }
    }

    /// Every check a change of `fields` of the record `id` is asked, of
    /// `by` and the record as it is now, with what it would change; nothing
    /// is written. Whoever asks holds the record's turn
    /// (`Collections::changing`) from before this until what it gives is
    /// written ([`Records::write`]), so that nothing changes between.
    pub async fn checked(&self, id: &str, fields: Map<String, Value>, by: &Principal) -> Result<Checking> {
        let (collection, mut record) = self.collections.record(id).await?;
        not_a_board(&collection, "changed")?;
        collection.may_change(by)?;
        self.collections.not_its_own(&collection, id, by).await?;
        for name in fields.keys() {
            crate::fields::known(&collection, name)?;
        }
        crate::models::own_refused(&collection, fields.keys())?;
        let cells = kernel::approvals::changed(&record.data, &fields);
        if cells.is_empty() {
            return Ok(Checking::Same(record));
        }
        // What each changed cell held before, for a guard that says what
        // was written there now and what was there already.
        let mut was = Map::new();
        for cell in &cells {
            was.insert(cell.name.clone(), cell.before.clone());
            record.data.insert(cell.name.clone(), cell.after.clone());
        }
        let changed: Vec<String> = cells.into_iter().map(|cell| cell.name).collect();
        crate::fields::stamp(&collection, &mut record.data, &changed, by, false)?;
        let given = changed
            .iter()
            .map(|name| (name.as_str(), record.data.get(name).unwrap_or(&Value::Null), was.get(name)));
        crate::fields::may_set(&collection, &record.data, given, by)?;
        self.collections.steered(&collection, by).await?;
        self.collections.may_name(&collection, &record.data, Some(&was), by).await?;
        Ok(Checking::Change(Box::new(Checked { collection, record, changed })))
    }

    /// Writes a change every check has passed ([`Records::checked`]), with
    /// what is said of it, as said by `by` or through `through`, under the
    /// turn the checks were asked in.
    pub async fn write(&self, checked: Checked, by: &Principal, through: Option<&Through>) -> Result<Record> {
        self.write_also(checked, by, through, None).await
    }

    /// The same, with what more is said of it (`more`, put into what
    /// `record.updated` carries: who approved it, of an approved change)
    /// and what else the write does ([`crate::whole::Also`]), as one write.
    pub async fn write_also(
        &self,
        checked: Checked,
        by: &Principal,
        through: Option<&Through>,
        also: Option<(Value, crate::whole::Also)>,
    ) -> Result<Record> {
        let Checked { collection, record, changed } = checked;
        let (more, also) = also.map_or((Value::Null, None), |(more, also)| (more, Some(also)));
        let updated = |record: &Record| {
            let mut data = Self::about(record);
            now_says(&collection, &changed, &record.data, &mut data);
            data["changed"] = json!(changed);
            for (key, value) in more.as_object().into_iter().flatten() {
                data[key] = value.clone();
            }
            let (source, data) = said_of(&collection, by, through, data);
            // Changed from outside with a key somebody made, it is the key
            // that changed it (`key:<id>`), and not the account the key is
            // of, which may hold more: a blueprint that does something for
            // whoever changed a record (a feature's plan accepted by its
            // being moved on) asks what they hold (`actor.holds`), and is
            // answered for the key. What a blueprint's step changes is said
            // as it always was.
            let source = match (through, &by.key) {
                (None, Some(key)) => kernel::Source { actor: Some(format!("key:{key}")), ..source },
                _ => source,
            };
            vec![Say::new("record.updated", (source, data))]
        };
        self.collections.put_also(&collection, record, updated, also).await
    }

    /// Says something on a record, a card included. Nothing of the record
    /// is changed: what was said is an event, and a line of the record's
    /// history in order with everything else that happened to it. It needs
    /// what changing that kind of record needs. A collection that would
    /// keep nothing of it refuses, and so does a saying the journal could
    /// not keep, so that nothing said is ever lost without anyone being
    /// told.
    ///
    /// `public` is whoever says it marking it so: it is said with the
    /// event, where it is, and left out where it is not, so that what is
    /// said with nothing more is private. It asks nothing more of whoever
    /// says it, and gives nobody anything to read. Who the event says said
    /// it is the key, where it was said with one (`Principal::key`).
    pub async fn comment(
        &self,
        id: &str,
        body: &str,
        public: bool,
        by: &Principal,
        through: Option<&Through>,
    ) -> Result<Record> {
        self.comment_on(id, body, public, None, by, through).await
    }

    /// Says something on a record that answers what was said there at
    /// `parent` (`crate::replied`), or the record itself where that is
    /// none: what is said, and the number it was said at.
    pub async fn reply(
        &self,
        id: &str,
        body: &str,
        parent: Option<u64>,
        by: &Principal,
        through: Option<&Through>,
    ) -> Result<(Record, u64)> {
        self.say_on(
            id,
            Saying { body, public: false, anchor: None, once: None, asker: None, parent, answered: false },
            by,
            through,
        )
        .await
    }

    /// Says something on a record about a few words of it (`anchor`: the
    /// words quoted, what is either side of them and the headings over
    /// them), or on the whole of it where there is none. What it is about
    /// is said with the event, and so kept with its line of the record's
    /// history: the record's file, which other programs write, holds
    /// nothing of it, and it is found again by looking for the words.
    pub async fn comment_on(
        &self,
        id: &str,
        body: &str,
        public: bool,
        anchor: Option<&kernel::anchor::Anchor>,
        by: &Principal,
        through: Option<&Through>,
    ) -> Result<Record> {
        Ok(self
            .say_on(
                id,
                Saying { body, public, anchor, once: None, asker: None, parent: None, answered: false },
                by,
                through,
            )
            .await?
            .0)
    }

    /// What every way of saying something on a record comes to, of what
    /// [`Saying`] says: what a blueprint's step that says which saying it is
    /// (`once`) says is kept with that, so that the step done again finds it
    /// ([`Records::said_once`]). What is said, and the number it was said at.
    pub(crate) async fn say_on(
        &self,
        id: &str,
        saying: Saying<'_>,
        by: &Principal,
        through: Option<&Through>,
    ) -> Result<(Record, u64)> {
        let Saying { body, public, anchor, once, asker, parent, answered } = saying;
        let (collection, record) = match self.collections.record(id).await {
            Ok(found) => found,
            // Whoever may change neither kind of record is not told
            // whether there is one.
            Err(_) if !answered && !by.can("board:write") && !by.can("records:write") => {
                by.require("records:write")?;
                unreachable!("it holds neither")
            }
            Err(none) => return Err(none),
        };
        if !answered {
            collection.may_change(by)?;
        }
        let body = words(body)?;
        if rule_for(&collection.history, COMMENTED).is_none() {
            return Err(Error::invalid(unkept(&collection)));
        }
        let mut data = Self::about(&record);
        data["body"] = json!(body);
        // Said with a key somebody made, it is the key that said it
        // (`key:<id>`), and not the account the key is of, which may hold
        // more than the key does: whatever then asks what the speaker
        // holds (`actor.holds`, before something marked public is told
        // anywhere) is answered for the key. What a blueprint's step says
        // is said as it always was, by whoever applied it and through it.
        let said = match (through, &by.key) {
            (None, Some(key)) => Some(format!("key:{key}")),
            _ => None,
        };
        // Who said it, as the journal names them, for whatever has to tell
        // one speaker from another: an answer from the one who asked.
        // A question a blueprint's step says for an agent (`decided`) is the
        // agent's, as its line of the history is: what tells whose words an
        // answer is from the asker's own goes by that.
        let speaker = through.and_then(|through| through.decided.clone());
        data["by"] = json!(said.clone().or(speaker).unwrap_or_else(|| by.actor()));
        if public {
            data[PUBLIC] = json!(true);
        }
        if let Some(anchor) = anchor {
            data[ANCHOR] = json!(anchor);
        }
        if let Some(once) = once {
            data[ONCE] = json!(once);
        }
        crate::left::mark(through, &mut data);
        if let Some(asker) = asker.filter(|asker| !asker.is_empty()) {
            data["asker"] = json!(asker);
        }
        if let Some(parent) = parent {
            data[crate::replied::PARENT] = json!(self.placed_under(&record, parent).await?);
        }
        let (source, data) = said_of(&collection, by, through, data);
        let (source, data) = crate::fieldsaid::onto(&collection, &record.data, (source, data));
        let source = match said {
            Some(key) => kernel::Source { actor: Some(key), ..source },
            None => source,
        };
        // What is said is kept as its saying and no other way: where
        // that fails nothing was kept, and whoever said it is told so.
        let said = self.journal.emit(COMMENTED, source, data).await.map_err(|err| {
            Error::unavailable(format!(
                "What was said on `{id}` was not kept, and nobody has heard it: {}. Say it again.",
                err.message.trim_end_matches('.')
            ))
        })?;
        Ok((record, said.seq))
    }

    /// Whether something said on `record` already says it is the saying
    /// `once` names: a step done again after a stop looks here before it
    /// says it a second time. Of what was said in its project lately, the
    /// newest first: a step under way when the daemon stopped said what it
    /// said a moment before.
    pub(crate) async fn said_once(&self, record: &str, once: &str) -> Result<bool> {
        let project = self.collections.record(record).await?.1.project;
        let lately = journal::EventQuery {
            topics: COMMENTED.into(),
            newest_first: true,
            limit: LATELY,
            ..Default::default()
        };
        let said = self.journal.of(Some(&project)).read(lately).await?;
        Ok(said.iter().any(|event| {
            event.data.get("id").and_then(Value::as_str) == Some(record)
                && event.data.get(ONCE).and_then(Value::as_str) == Some(once)
        }))
    }

    /// One record, by its id.
    pub async fn get(&self, id: &str, by: &Principal) -> Result<Record> {
        if !by.can("board:read") {
            by.require("records:read")?;
        }
        // A card is the board's to show, as it is the board's to change.
        let (collection, record) = self.collections.record(id).await?;
        collection.may_read(by)?;
        Ok(record)
    }

    /// The records of a project's collection, in the order they were made.
    pub async fn list(&self, project: &str, collection: &str, by: &Principal) -> Result<Vec<Record>> {
        if !by.can("board:read") {
            by.require("records:read")?;
        }
        let collection = self.collections.get(project, &blueprint::expr::slug(collection)).await?;
        collection.may_read(by)?;
        self.collections.records(&collection).await
    }

    /// How many things were said on each record of a collection, by the
    /// record's id, for whoever may read its records: what a list of
    /// them shows of each without asking for each one's history.
    pub async fn commented(
        &self,
        project: &str,
        collection: &str,
        by: &Principal,
    ) -> Result<std::collections::BTreeMap<String, u64>> {
        if !by.can("board:read") {
            by.require("records:read")?;
        }
        let collection = self.collections.get(project, &blueprint::expr::slug(collection)).await?;
        collection.may_read(by)?;
        self.collections.commented(&collection).await
    }

    /// Takes a record away, into the Trash where the factory keeps what
    /// is deleted (`crate::trash`). What named it goes on saying what it
    /// named, and its history is kept.
    pub async fn remove(
        &self,
        id: &str,
        by: &Principal,
        through: Option<&Through>,
    ) -> Result<Option<String>> {
        Ok(self.removing(id, by, through).await?.trash)
    }

    /// The same, saying how many records went with it: the pages under a
    /// page, which go into the Trash with it and come back with it.
    pub async fn removing(
        &self,
        id: &str,
        by: &Principal,
        through: Option<&Through>,
    ) -> Result<crate::trash::Gone> {
        by.require("records:write")?;
        let (collection, record) = self.collections.record(id).await?;
        not_a_board(&collection, "taken away")?;
        collection.may_change(by)?;
        self.collections.not_its_own(&collection, id, by).await?;
        let removed = Say::new("record.removed", said_of(&collection, by, through, Self::about(&record)));
        self.collections.trash_saying(&collection, &record, vec![removed], by).await
    }
}

impl Records {
    /// Says of a document what was done to its file through the daemon by
    /// someone who can be named: written for the first time, or changed.
    /// `was` is what the file held before, where it was there. A file that
    /// is no library's document says nothing, and neither does one written
    /// back as it was.
    ///
    /// What another program does to the file is not said here: nobody
    /// knows who did it, and it is in the library the next time it is read.
    pub async fn file_written(
        &self,
        project: &str,
        file: &str,
        was: Option<&str>,
        now: &str,
        by: &Principal,
    ) {
        let Ok(Some((collection, id))) = self.collections.document_at(project, file).await else {
            return;
        };
        let mut data = json!({ "id": id, "project": project, "collection": collection.name });
        let path = crate::documents::path_of(&collection, &id);
        // What the file now holds, read as the library reads a document.
        let holds = path.as_ref().map(|path| crate::documents::read(&collection, path, now, (0, 0), 0).data);
        let holds = holds.unwrap_or_default();
        let Some(was) = was else {
            let said = crate::fieldsaid::onto(&collection, &holds, said_of(&collection, by, None, data));
            let _ = self.journal.emit("record.created", said.0, said.1).await;
            return;
        };
        let Some(path) = path else { return };
        let changed = crate::documents::changed(&collection, &path, was, now);
        if changed.is_empty() {
            return;
        }
        now_says(&collection, &changed, &holds, &mut data);
        data["changed"] = json!(changed);
        let (source, data) =
            crate::fieldsaid::onto(&collection, &holds, said_of(&collection, by, None, data));
        let _ = self.journal.emit("record.updated", source, data).await;
    }

    /// Says that a document was given back from the Trash, where it is now.
    pub async fn file_restored(&self, project: &str, file: &str, by: &Principal) {
        let Ok(Some((collection, id))) = self.collections.document_at(project, file).await else {
            return;
        };
        let data = json!({ "id": id, "project": project, "collection": collection.name, "restored": true });
        let holds = self.collections.record(&id).await.map(|(_, record)| record.data).unwrap_or_default();
        let (source, data) =
            crate::fieldsaid::onto(&collection, &holds, said_of(&collection, by, None, data));
        let _ = self.journal.emit("record.created", source, data).await;
    }

    /// Says that a document was taken away, when its file was removed
    /// through the daemon by someone who can be named.
    pub async fn file_removed(&self, project: &str, file: &str, by: &Principal) {
        let Ok(Some((collection, id))) = self.collections.document_at(project, file).await else {
            return;
        };
        let data = json!({ "id": id, "project": project, "collection": collection.name });
        let (source, data) = said_of(&collection, by, None, data);
        let _ = self.journal.emit("record.removed", source, data).await;
    }
}

/// What each changed cell now says, for the two kinds of cell that say
/// one of a few things: a choice (`chose`) and a flag (`flags`), each by
/// the field's name, and each left out where no such cell changed.
/// Whatever does something for a record having been changed (a
/// feature's plan accepted by its being moved on, somebody told that a
/// document was marked public) goes by these, which are of the change
/// itself: the record as it is when the event is heard may have been
/// changed again, by somebody else, and what that one did would be
/// taken for this one's doing. Both ways a record's change is said ask
/// it: a record changed, and a document's file written through the
/// daemon, which said neither before.
fn now_says(collection: &Collection, changed: &[String], holds: &Map<String, Value>, said: &mut Value) {
    for (word, kind) in [("chose", "choice"), ("flags", "flag")] {
        let now: Map<String, Value> = collection
            .fields
            .iter()
            .filter(|field| field.kind == kind && changed.contains(&field.name))
            .map(|field| (field.name.clone(), holds.get(&field.name).cloned().unwrap_or(Value::Null)))
            .collect();
        if !now.is_empty() {
            said[word] = Value::Object(now);
        }
    }
}

/// What is said on a record, as it is kept: something, and not a book.
fn words(said: &str) -> Result<String> {
    let said = said.trim();
    if said.is_empty() {
        return Err(Error::invalid("there is nothing here to say on it: it needs some words"));
    }
    if said.len() > MAX_SAID {
        return Err(Error::invalid(format!(
            "that is too much to say on a record at once ({} bytes; the most is {MAX_SAID}); put the rest in a file and say where it is",
            said.len()
        )));
    }
    Ok(said.to_string())
}

/// Why nothing can be said on the records of a collection that keeps no
/// history of it, and what to do about it.
fn unkept(collection: &Collection) -> String {
    let (name, project) = (&collection.name, &collection.project);
    let how = if collection.name == BOARD {
        format!(
            "This board was made before cards could be spoken on: apply its blueprint again (`{} apply std/board --project {project}`, or the project's own)",
            brand::CLI
        )
    } else {
        format!(
            "The blueprint that declares `{name}` must count `{COMMENTED}` in its `history` (`{COMMENTED}: \"{{actor}} said: {{body}}\"`), and be applied again"
        )
    };
    format!(
        "what is said on a record of `{name}` would be kept nowhere: `{name}` in `{project}` keeps no history of it. {how}"
    )
}

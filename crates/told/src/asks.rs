//! A change somebody asks for that they may not make themselves, and a
//! person approving or declining it (`docs/approvals.md`).
//!
//! An ask is a row of `asked`, kept where its record is (the project's
//! own file, or the factory's for a project from before), so that the
//! change an approval allows and the approval being spent are one write:
//! the record, the ask written as approved, and what is said of both
//! (`studio::Records::write_also`). Approving is the change: no grant
//! stands apart from it, nothing is handed to the asker, and asked again
//! it finds the ask decided and changes nothing.
//!
//! The change is made as whoever asked (an agent's work is credited to
//! the agent) with every check asked of the approver, as the record is
//! then: nobody gives by approving what they could not do themselves,
//! and nothing of the approver's goes past this one write. What is said
//! of it names both (`approved_by`).
//!
//! Two things are told after their write, since they are kept in the
//! factory's file and not the project's: the lines of the inboxes of
//! whoever could make it, and what came of it, told back to the asker.
//! Each is owed on the row (`owes`) until it is done, and a start does
//! what is owed ([`Asks::settle`]); each is the same done twice (an
//! inbox's line by `said_by`, the bot's message by what said it).

use journal::Journal;
use kernel::approvals::{self, Cell, Need, Of, Status};
use kernel::{Error, ErrorKind, Principal, PrincipalKind, Result, Source};
use rusqlite::{OptionalExtension, params};
use serde_json::{Map, Value, json};
use studio::{Changes, Checking, Collection, Doing, Record, Studio};
use talk::SaidBy;
use talk::inbox::{Kind, Tell};

/// What a line of somebody's inbox says why it was told: an ask, and
/// which, so that each is a line of its own. A line is one for its owner,
/// what it is about and why, and the asks of one record would otherwise
/// be one line, which showed the newest alone.
fn why_of(ask: &Ask) -> String {
    format!("asked {}", ask.id)
}

/// What an ask keeps, as its columns are read.
pub(crate) const COLUMNS: &str =
    "id, project, record, collection, kind, cells, updated, asker, why, named, place, status, decided, at";

/// One change somebody asked for, as it is kept.
#[derive(Clone, Debug, serde::Serialize)]
pub struct Ask {
    pub id: String,
    pub project: String,
    pub record: String,
    pub collection: String,
    pub kind: String,
    /// Each cell it sets: `name`, `before` and `after`.
    pub cells: Value,
    /// When its record was last changed as it was asked.
    pub updated: i64,
    /// Who asked: `kind`, `id`, `name` and `key`, as their principal says.
    pub asker: Value,
    pub why: String,
    /// Whose inboxes it was told to, as actors.
    pub named: Vec<String>,
    /// Where it was asked from, for telling what came of it: a channel and
    /// a thread, where it was asked in one.
    pub place: Value,
    pub status: String,
    /// Who decided it, when, and what they said: nothing while it waits.
    pub decided: Option<Value>,
    pub at: i64,
}

impl Ask {
    pub(crate) fn read(row: &rusqlite::Row<'_>) -> rusqlite::Result<Ask> {
        let json = |at: usize| -> rusqlite::Result<Value> {
            let text: Option<String> = row.get(at)?;
            Ok(text.and_then(|text| serde_json::from_str(&text).ok()).unwrap_or(Value::Null))
        };
        let named = json(9)?;
        Ok(Ask {
            id: row.get(0)?,
            project: row.get(1)?,
            record: row.get(2)?,
            collection: row.get(3)?,
            kind: row.get(4)?,
            cells: json(5)?,
            updated: row.get(6)?,
            asker: json(7)?,
            why: row.get(8)?,
            named: named
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect(),
            place: json(10)?,
            status: row.get(11)?,
            decided: Some(json(12)?).filter(|decided| !decided.is_null()),
            at: row.get(13)?,
        })
    }

    pub fn status(&self) -> Status {
        Status::read(&self.status)
    }

    /// Whether it is a run waiting to be let go on (`crate::goes`), and
    /// not a change of a record's cells.
    pub fn is_go(&self) -> bool {
        self.kind == approvals::Kind::Go.as_str()
    }

    /// The cells as the change asks them, by their names.
    pub fn fields(&self) -> Map<String, Value> {
        let cells = self.cells.as_array().into_iter().flatten();
        cells.filter_map(|cell| Some((cell["name"].as_str()?.to_string(), cell["after"].clone()))).collect()
    }

    /// Who asked, as their principal: holding nothing, since the change is
    /// theirs by name and never by what they hold.
    pub fn asker(&self) -> Principal {
        let text = |key: &str| self.asker[key].as_str().unwrap_or_default().to_string();
        Principal {
            kind: if text("kind") == "agent" { PrincipalKind::Agent } else { PrincipalKind::User },
            id: text("id"),
            name: text("name"),
            scopes: Vec::new(),
            key: self.asker["key"].as_str().map(str::to_string),
        }
    }

    /// What is said of it, as every event about it says it.
    fn about(&self) -> Value {
        let names: Vec<&str> =
            self.cells.as_array().into_iter().flatten().filter_map(|cell| cell["name"].as_str()).collect();
        json!({
            "ask": self.id, "id": self.record, "project": self.project, "collection": self.collection,
            "cells": names, "asker": self.asker().actor(),
        })
    }
}

/// What asking came to.
#[derive(Debug)]
pub enum Came {
    /// It asked for what is so already.
    Nothing(Record),
    /// The asker holds what it takes, and it was made as their change.
    Made(Record),
    /// It is kept for somebody who could make it, who was told.
    Asked(Ask),
}

/// Asks, approvals and declines, over the studio and the factory's
/// journal: nothing is held here, so one is made where it is wanted.
#[derive(Clone)]
pub struct Asks {
    pub(crate) journal: Journal,
    pub(crate) studio: Studio,
    /// The runner, for a run that waits to be let go on (`crate::goes`):
    /// the environment whose rule says who approves is its to read.
    pub(crate) engine: Option<conductor::Engine>,
}

/// The actor something done with `by` is said to be done by: the key,
/// where it was a key somebody made.
pub(crate) fn actor(by: &Principal) -> String {
    match &by.key {
        Some(key) => format!("key:{key}"),
        None => by.actor(),
    }
}

/// What a change takes, as a record of `collection` would be once it is
/// made: whoever makes it is asked each.
/// Whether `collection` is a board, whose cards are changed by its own write.
fn is_board(collection: &Collection) -> bool {
    collection.name == studio::board::BOARD
}

pub fn needs(collection: &Collection, after: &Map<String, Value>, cells: &[Cell]) -> Vec<Need> {
    let write = if is_board(collection) { "board:write" } else { "records:write" };
    let mut needs = vec![Need::new(Of::Records, &[write])];
    let mut table: Vec<String> = collection.reads.iter().chain(&collection.takes).cloned().collect();
    table.dedup();
    needs.push(Need { of: Of::Table, scopes: table });
    for cell in cells {
        let Some(field) = collection.field(&cell.name) else { continue };
        let mut scopes: Vec<String> = field.takes.scopes().to_vec();
        let written = blueprint::expr::display(&cell.after);
        scopes.extend(field.takes_of.get(&written).map(|takes| takes.scopes().to_vec()).unwrap_or_default());
        needs.push(Need { of: Of::Cell(cell.name.clone()), scopes });
    }
    // What a row gives that gives what it lists, as it would say it: what
    // a list leaves unsaid stands for what the table says it does.
    for field in collection.fields.iter().filter(|field| field.takes_each) {
        let said: Vec<String> = match after.get(&field.name) {
            Some(Value::Array(each)) => each.iter().map(blueprint::expr::display).collect(),
            Some(Value::Null) | None => Vec::new(),
            Some(one) => vec![blueprint::expr::display(one)],
        };
        let said: Vec<String> =
            said.into_iter().map(|one| one.trim().to_string()).filter(|one| !one.is_empty()).collect();
        let gives = if said.is_empty() { field.unsaid.clone() } else { said };
        needs.push(Need { of: Of::Cell(field.name.clone()), scopes: gives });
    }
    // What a cell took, while the row as it would be still says it: asked
    // of whoever changes anything of the row.
    for field in collection.fields.iter().filter(|field| field.stands) {
        let scopes: Vec<String> =
            field.standing(after.get(&field.name)).into_iter().map(String::from).collect();
        if !scopes.is_empty() {
            needs.push(Need { of: Of::Standing(field.name.clone()), scopes });
        }
    }
    needs
}

impl Asks {
    pub fn new(journal: &Journal, studio: &Studio) -> Asks {
        Asks { journal: journal.clone(), studio: studio.clone(), engine: None }
    }

    /// The same, reading the runner where a run that waits is decided.
    pub fn reading(mut self, engine: &conductor::Engine) -> Asks {
        self.engine = Some(engine.clone());
        self
    }

    /// The journals that may keep asks: the factory's own, and each
    /// project's that keeps its records, each once.
    async fn keepers(&self) -> Result<Vec<Journal>> {
        let mut all = vec![self.journal.records_of(None)];
        for project in self.studio.projects.list().await? {
            let keeps = self.journal.records_of(Some(&project.id));
            if !all.iter().any(|one| one.name() == keeps.name()) {
                all.push(keeps);
            }
        }
        Ok(all)
    }

    /// The asks `filter` finds, a statement's `WHERE`, of every keeper.
    pub(crate) async fn found(&self, filter: &'static str, value: String) -> Result<Vec<Ask>> {
        let mut all = Vec::new();
        for keeper in self.keepers().await? {
            let value = value.clone();
            all.extend(
                keeper
                    .db()
                    .call(move |connection| {
                        let mut statement = connection
                            .prepare(&format!("SELECT {COLUMNS} FROM asked WHERE {filter} ORDER BY at"))?;
                        let rows = statement.query_map([&value], Ask::read)?;
                        rows.collect::<rusqlite::Result<Vec<Ask>>>()
                    })
                    .await?,
            );
        }
        Ok(all)
    }

    /// One ask by its id: refused as not there to nobody's advantage.
    pub async fn one(&self, id: &str) -> Result<Ask> {
        let found = self.found("id = ?1", id.to_string()).await?;
        found.into_iter().next().ok_or_else(|| Error::not_found(format!("there is no ask `{id}`")))
    }

    /// The ask `id` in the journal that keeps it, read in `connection`.
    pub(crate) fn read_one(connection: &rusqlite::Connection, id: &str) -> rusqlite::Result<Option<Ask>> {
        connection
            .query_row(&format!("SELECT {COLUMNS} FROM asked WHERE id = ?1"), [id], Ask::read)
            .optional()
    }

    /// Asks for `fields` of `record` to be set as `by`. What `by` may make
    /// is made at once; what it may not, for want of scopes alone, is
    /// kept and told to whoever could make it, the person `to` names
    /// where they could (an account's id, name or email); anything else
    /// is refused as the change would be. `place` is where it was asked
    /// from (`channel`, `thread`), for telling what came of it.
    pub async fn ask(
        &self,
        by: &Principal,
        record: &str,
        fields: Map<String, Value>,
        why: &str,
        to: Option<&str>,
        place: Value,
    ) -> Result<Came> {
        if !matches!(by.kind, PrincipalKind::User | PrincipalKind::Agent) {
            return Err(Error::invalid("a change is asked for by a person or an agent".to_string()));
        }
        let _turn = self.studio.collections.changing(record).await;
        let (collection, kept) = self.studio.collections.record(record).await?;
        collection.may_read(by)?;
        self.studio.collections.not_its_own(&collection, record, by).await?;
        let cells = approvals::changed(&kept.data, &fields);
        if cells.is_empty() {
            // Unknown cells are refused as a change of them would be.
            for name in fields.keys() {
                if collection.field(name).is_none() {
                    return self.made(record, fields, by).await;
                }
            }
            return Ok(Came::Nothing(kept));
        }
        let cells_json = cells_json(&cells);
        // Asked again with the same words (a turn begun again), it is the ask there is.
        let waiting =
            self.found("record = ?1 AND status = 'waiting' AND kind = 'cells'", record.to_string()).await?;
        if let Some(same) =
            waiting.into_iter().find(|one| one.asker().actor() == by.actor() && one.cells == cells_json)
        {
            return Ok(Came::Asked(same));
        }
        let refused = match self.made(record, fields.clone(), by).await {
            Ok(came) => return Ok(came),
            Err(refused) if refused.kind == ErrorKind::Denied => refused,
            Err(refused) => return Err(refused),
        };
        let mut after = kept.data.clone();
        for cell in &cells {
            after.insert(cell.name.clone(), cell.after.clone());
        }
        let lacks = approvals::lacking(&needs(&collection, &after, &cells), &by.scopes);
        if lacks.is_empty() {
            return Err(refused);
        }
        let mine = self
            .found(
                "asker LIKE ?1 AND status = 'waiting' AND kind = 'cells'",
                format!("%\"id\":{}%", json!(by.id)),
            )
            .await?;
        if mine.iter().filter(|one| one.asker().actor() == by.actor()).count() >= approvals::MOST_WAITING {
            return Err(Error::conflict(format!(
                "{} has {} changes waiting to be approved already, which is as many as may wait at once: withdraw one, or wait for one to be decided",
                by.name,
                approvals::MOST_WAITING
            )));
        }
        let could = self.could(&collection, record, &fields, by).await?;
        let named: Vec<String> =
            match to.and_then(|to| could.iter().find(|(account, _)| named_by(account, to))) {
                Some((account, _)) => vec![format!("user:{}", account.id)],
                None => could.iter().map(|(account, _)| format!("user:{}", account.id)).collect(),
            };
        if named.is_empty() {
            return Err(Error::denied(format!(
                "{} may not make this change ({}), and nobody here who could make it may read the record: it is not asked",
                by.name, refused.message
            )));
        }
        let ask = Ask {
            id: self.journal.new_id("ask"),
            project: kept.project.clone(),
            record: record.to_string(),
            collection: collection.name.clone(),
            kind: approvals::Kind::Cells.as_str().to_string(),
            cells: cells_json,
            updated: kept.updated,
            asker: json!({ "kind": if by.kind == PrincipalKind::Agent { "agent" } else { "user" }, "id": by.id, "name": by.name, "key": by.key }),
            why: why.trim().to_string(),
            named,
            place,
            status: Status::Waiting.as_str().to_string(),
            decided: None,
            at: self.journal.now_ms(),
        };
        let row = ask.clone();
        let source = Source::actor(actor(by)).taking(collection.reads.as_deref());
        let mut data = row.about();
        data["why"] = json!(row.why);
        self.journal
            .records_of(Some(&ask.project))
            .write(move |writing| {
                writing.transaction().execute(
                    &format!("INSERT INTO asked ({COLUMNS}, owes) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, NULL, ?13, 'told')"),
                    params![
                        row.id,
                        row.project,
                        row.record,
                        row.collection,
                        row.kind,
                        row.cells.to_string(),
                        row.updated,
                        row.asker.to_string(),
                        row.why,
                        json!(row.named).to_string(),
                        row.place.to_string(),
                        row.status,
                        row.at
                    ],
                )?;
                writing.say("record.asked", source, data)?;
                Ok(())
            })
            .await?;
        self.tell(&ask).await?;
        Ok(Came::Asked(ask))
    }

    /// The change made by `by` with what `by` holds: under the record's
    /// turn, which whoever asks holds.
    async fn made(&self, record: &str, fields: Map<String, Value>, by: &Principal) -> Result<Came> {
        let (collection, _) = self.studio.collections.record(record).await?;
        if is_board(&collection) {
            by.require("board:write")?;
            let doing = Doing { by, said_as: by, through: None, said: None, also: None, dry: false };
            self.studio.board.change(record, Changes::of(fields), doing).await?;
            return Ok(Came::Made(self.studio.collections.record(record).await?.1));
        }
        by.require("records:write")?;
        match self.studio.records.checked(record, fields, by).await? {
            Checking::Same(kept) => Ok(Came::Nothing(kept)),
            Checking::Change(checked) => Ok(Came::Made(self.studio.records.write(*checked, by, None).await?)),
        }
    }

    /// Who could make the change as things stand, and may read the
    /// record: every person here but `by`, with what each holds now.
    async fn could(
        &self,
        collection: &Collection,
        record: &str,
        fields: &Map<String, Value>,
        by: &Principal,
    ) -> Result<Vec<(studio::Account, Principal)>> {
        let mut could = Vec::new();
        for account in self.studio.identity.accounts().await? {
            if by.kind == PrincipalKind::User && account.id == by.id {
                continue;
            }
            let them = account.principal();
            match self.may_make(collection, record, fields, &them).await {
                Ok(()) => could.push((account, them)),
                // What is wrong with the change itself (a card's column, which is moved) is so for anybody.
                Err(wrong) if wrong.kind == ErrorKind::Invalid => return Err(wrong),
                Err(_) => {}
            }
        }
        Ok(could)
    }

    /// Whether `them` could make the change: read the record, write
    /// records, and pass every check of it, as it stands. Nothing is
    /// written.
    async fn may_make(
        &self,
        collection: &Collection,
        record: &str,
        fields: &Map<String, Value>,
        them: &Principal,
    ) -> Result<()> {
        if them.kind != PrincipalKind::User {
            return Err(Error::denied("a change is approved by a person".to_string()));
        }
        collection.may_read(them)?;
        if is_board(collection) {
            them.require("board:write")?;
            let doing = Doing { by: them, said_as: them, through: None, said: None, also: None, dry: true };
            return self.studio.board.change(record, Changes::of(fields.clone()), doing).await.map(|_| ());
        }
        them.require("records:write")?;
        self.studio.records.checked(record, fields.clone(), them).await.map(|_| ())
    }

    /// Tells each owner of `ask`'s lines, and then takes off that it owed
    /// them: told twice, a line is told once (`said_by`).
    async fn tell(&self, ask: &Ask) -> Result<()> {
        let says = approvals::told(&ask.asker().name, &ask.record, &ask.collection, &cells_of(&ask.cells));
        for owner in &ask.named {
            let tell = Tell {
                owner: owner.clone(),
                kind: Kind::Told,
                about: ask.record.clone(),
                place: ask.record.clone(),
                from: ask.asker().actor(),
                why: why_of(ask),
                says: Some(says.clone()),
                said_by: Some(ask.id.clone()),
                parts: Some(json!([{ "kind": "change", "project": ask.project, "ask": ask.id }])),
            };
            self.studio.channels.inbox.tell(tell).await?;
        }
        self.owed(ask, None).await
    }

    /// Takes off what `ask` owed, or says it owes `now`.
    pub(crate) async fn owed(&self, ask: &Ask, now: Option<&'static str>) -> Result<()> {
        let id = ask.id.clone();
        self.journal
            .records_of(Some(&ask.project))
            .db()
            .call(move |connection| {
                connection.execute("UPDATE asked SET owes = ?2 WHERE id = ?1", params![id, now])
            })
            .await?;
        Ok(())
    }

    /// Approves `id` as `by` (`approve`), which makes the change as whoever
    /// asked, or declines it with `why`. Asked again once it is decided,
    /// it is answered as it was decided and nothing changes. One whose
    /// record was changed since, or which has waited its time, is written
    /// as stale or expired, and nothing is made.
    pub async fn decide(&self, by: &Principal, id: &str, approve: bool, why: &str) -> Result<Ask> {
        let found = self.one(id).await?;
        if found.is_go() {
            return self.decide_go(by, found, approve, why).await;
        }
        let _turn = self.studio.collections.changing(&found.record).await;
        let keeper = self.journal.records_of(Some(&found.project));
        let read = id.to_string();
        let ask = keeper.db().call(move |connection| Self::read_one(connection, &read)).await?;
        let Some(ask) = ask.filter(|ask| ask.status() == Status::Waiting) else {
            return self.one(id).await;
        };
        if by.kind != PrincipalKind::User || by.actor() == ask.asker().actor() {
            return Err(Error::denied(format!(
                "a change is approved or declined by a person who could make it, and not by whoever asked for it: {} may not",
                by.name
            )));
        }
        let now = self.journal.now_ms();
        let kept = match self.studio.collections.record(&ask.record).await {
            Ok((collection, kept)) => Some((collection, kept)),
            Err(gone) if gone.kind == ErrorKind::NotFound => None,
            Err(other) => return Err(other),
        };
        // Whoever may not read the record is told of no ask about it.
        if let Some((collection, _)) = &kept {
            collection.may_read(by)?;
        }
        let standing = match &kept {
            Some((_, kept)) => approvals::stands(Status::Waiting, ask.at, (ask.updated, kept.updated), now),
            None => Status::Stale,
        };
        if standing != Status::Waiting {
            let decided = json!({ "by": "system", "name": "", "at": now, "why": "" });
            return self.close(&ask, standing, decided, Source::default()).await;
        }
        let (collection, record) = kept.expect("a record that is there");
        let fields = ask.fields();
        self.may_make(&collection, &ask.record, &fields, by).await?;
        self.studio.collections.not_its_own(&collection, &ask.record, &ask.asker()).await?;
        // A change is made as whoever asked, so only while they can act:
        // one taken away, stopped, or whose key was withdrawn is not made
        // anything as. It may still be declined.
        let asker = ask.asker();
        let now_as = asker.key.as_ref().map_or_else(|| asker.actor(), |key| format!("key:{key}"));
        if approve && self.studio.identity.holder(&now_as).await?.is_none() {
            return Err(Error::conflict(format!(
                "{} cannot act now (it is stopped, taken away, or its key was withdrawn), and a change is made as \
                 whoever asked for it: start it again to approve this, or decline it",
                asker.name
            )));
        }
        let decided = json!({ "by": actor(by), "name": by.name, "at": now, "why": why.trim() });
        let source = Source::actor(actor(by)).taking(collection.reads.as_deref());
        if !approve {
            return self.close(&ask, Status::Declined, decided, source).await;
        }
        // What it asks for is so already, and nothing is made.
        if approvals::changed(&record.data, &fields).is_empty() {
            return self.close(&ask, Status::Approved, decided, source).await;
        }
        let more = json!({ "approved_by": actor(by), "asked": ask.id });
        let (row, said) = (ask.clone(), decided.clone());
        let also: studio::whole::Also = Box::new(move |writing| {
            let changed = writing.transaction().execute(
                "UPDATE asked SET status = 'approved', decided = ?2, owes = 'decided' WHERE id = ?1 AND status = 'waiting'",
                params![row.id, said.to_string()],
            )?;
            if changed == 0 {
                return Err(rusqlite::Error::QueryReturnedNoRows);
            }
            let mut data = row.about();
            data["by"] = said["by"].clone();
            writing.say("record.ask.approved", source.clone(), data.clone())?;
            data["status"] = json!("approved");
            writing.say("record.ask.decided", source, data)?;
            Ok(())
        });
        if is_board(&collection) {
            // A card is changed by its board's own write, which its board's history and stages hear.
            let doing = Doing {
                by,
                said_as: &asker,
                through: None,
                said: None,
                also: Some((more, also)),
                dry: false,
            };
            self.studio.board.change(&ask.record, Changes::of(fields), doing).await?;
        } else if let Checking::Change(checked) = self.studio.records.checked(&ask.record, fields, by).await?
        {
            self.studio.records.write_also(*checked, &asker, None, Some((more, also))).await?;
        }
        let ask = self.one(id).await?;
        self.told_back(&ask).await?;
        Ok(ask)
    }

    /// Writes `ask` as `status`, which is no approval with a change, with
    /// what is said of it, and tells whoever asked.
    async fn close(&self, ask: &Ask, status: Status, decided: Value, source: Source) -> Result<Ask> {
        let (row, word) = (ask.clone(), status.as_str());
        self.journal
            .records_of(Some(&ask.project))
            .write(move |writing| {
                let changed = writing.transaction().execute(
                    "UPDATE asked SET status = ?2, decided = ?3, owes = 'decided' WHERE id = ?1 AND status = 'waiting'",
                    params![row.id, word, decided.to_string()],
                )?;
                if changed == 0 {
                    return Ok(());
                }
                let mut data = row.about();
                data["by"] = decided["by"].clone();
                if word == "declined" {
                    data["why"] = decided["why"].clone();
                    writing.say("record.ask.declined", source.clone(), data.clone())?;
                }
                data["status"] = json!(word);
                writing.say("record.ask.decided", source, data)?;
                Ok(())
            })
            .await?;
        let ask = self.one(&ask.id).await?;
        self.told_back(&ask).await?;
        Ok(ask)
    }

    /// Takes back an ask its asker no longer wants: theirs alone.
    pub async fn withdraw(&self, by: &Principal, id: &str) -> Result<Ask> {
        let ask = self.one(id).await?;
        if ask.is_go() {
            return Err(Error::invalid(format!(
                "`{id}` is a run waiting to be let go on, which is decided and not withdrawn: stop the run instead"
            )));
        }
        if by.actor() != ask.asker().actor() {
            return Err(Error::denied(format!(
                "only whoever asked withdraws an ask: {} did not ask `{id}`",
                by.name
            )));
        }
        let _turn = self.studio.collections.changing(&ask.record).await;
        let decided = json!({ "by": actor(by), "name": by.name, "at": self.journal.now_ms(), "why": "" });
        self.close(&ask, Status::Withdrawn, decided, Source::actor(actor(by))).await
    }

    /// Withdraws, as the factory and saying `why`, every ask `actor` has
    /// waiting: for an asker taken away, whom no change can be made as.
    /// Says how many.
    pub async fn withdraw_all_of(&self, actor: &str, why: &str) -> Result<usize> {
        let mut withdrawn = 0;
        for ask in self.found("status = ?1 AND kind = 'cells'", "waiting".into()).await? {
            if ask.asker().actor() != actor {
                continue;
            }
            let _turn = self.studio.collections.changing(&ask.record).await;
            let decided = json!({ "by": "system", "name": "", "at": self.journal.now_ms(), "why": why });
            self.close(&ask, Status::Withdrawn, decided, Source::actor("system")).await?;
            withdrawn += 1;
        }
        Ok(withdrawn)
    }

    /// Tells whoever asked what came of `ask`, where it was asked from: in
    /// its channel and thread by the factory's bot naming them, or in
    /// their inbox; and then takes off that it owed that.
    async fn told_back(&self, ask: &Ask) -> Result<()> {
        let decided = ask.decided.clone().unwrap_or(Value::Null);
        // Withdrawn by the factory, its asker is gone: there is nobody to tell.
        if ask.status() == Status::Withdrawn && decided["by"] == "system" {
            return self.owed(ask, None).await;
        }
        let by = decided["name"].as_str().unwrap_or_default();
        let why = decided["why"].as_str().unwrap_or_default();
        let sentence = approvals::decided(ask.status(), &ask.record, by, why);
        let asker = ask.asker();
        let said_by = SaidBy {
            blueprint: "approvals".into(),
            instance: ask.id.clone(),
            run: ask.id.clone(),
            step: ask.status.clone(),
            ..SaidBy::default()
        };
        let channels = &self.studio.channels;
        // Said where they asked only while they may write there still,
        // as what they hold now: one who left it, or no longer may, is
        // told in their box.
        let now = match &asker.key {
            Some(key) => format!("key:{key}"),
            None => asker.actor(),
        };
        let reader = match (ask.place["channel"].as_str(), self.studio.identity.holder(&now).await?) {
            (Some(channel), Some(holder)) => channels.may_write(channel, &holder).await.ok(),
            _ => None,
        };
        let posted = match (ask.place["channel"].as_str(), &reader) {
            (Some(channel), Some(reader)) if channels.said_already(reader, &said_by).await?.is_none() => {
                let body = format!("@{} {sentence}", asker.name);
                let place = studio::Place {
                    thread: ask.place["thread"].as_str(),
                    said_by: Some(&said_by),
                    ..studio::Place::default()
                };
                channels.post_speaker(reader, channel, &body, place).await.is_ok()
            }
            (Some(_), Some(_)) => true,
            _ => false,
        };
        if !posted {
            let tell = Tell {
                owner: asker.actor(),
                kind: Kind::Told,
                about: ask.record.clone(),
                place: ask.record.clone(),
                from: decided["by"].as_str().unwrap_or("system").to_string(),
                why: why_of(ask),
                says: Some(sentence),
                said_by: Some(format!("{}:{}", ask.id, ask.status)),
                parts: None,
            };
            channels.inbox.tell(tell).await?;
        }
        self.owed(ask, None).await
    }

    /// What a start owes: the lines of asks that were kept and not yet
    /// told, and what came of decided ones not yet told back.
    pub async fn settle(&self) -> Result<()> {
        for ask in self.found("owes = ?1", "told".into()).await? {
            match ask.is_go() {
                true => self.tell_go(&ask).await?,
                false => self.tell(&ask).await?,
            }
        }
        for ask in self.found("owes = ?1 AND kind = 'cells'", "decided".into()).await? {
            self.told_back(&ask).await?;
        }
        Ok(())
    }

    /// What `who` is shown of an ask, as the record and they stand now:
    /// each cell before, now and after with what it takes and whether
    /// they hold it, how the ask stands, and whether they may decide it.
    /// Kept from whoever may not read the record, but for that it is.
    pub async fn shown(&self, who: &Principal, id: &str) -> Result<Value> {
        let ask = self.one(id).await?;
        if ask.is_go() {
            return self.shown_go(who, &ask).await;
        }
        let (collection, kept) = self.studio.collections.record(&ask.record).await?;
        collection.may_read(who)?;
        let cells = cells_of(&ask.cells);
        let mut after = kept.data.clone();
        for cell in &cells {
            after.insert(cell.name.clone(), cell.after.clone());
        }
        let needs = needs(&collection, &after, &cells);
        let shown: Vec<Value> = cells
            .iter()
            .map(|cell| {
                let takes: Vec<Value> = needs
                    .iter()
                    .filter(|need| need.of == Of::Cell(cell.name.clone()))
                    .flat_map(|need| &need.scopes)
                    .map(|scope| json!({ "scope": scope, "held": kernel::principal::covers(&who.scopes, scope) }))
                    .collect();
                json!({
                    "name": cell.name, "before": cell.before, "after": cell.after,
                    "now": kept.data.get(&cell.name).cloned().unwrap_or(Value::Null), "takes": takes,
                })
            })
            .collect();
        // What changing the record at all takes, beside what each cell takes.
        let whole: Vec<Value> = needs
            .iter()
            .filter(|need| !matches!(need.of, Of::Cell(_)))
            .flat_map(|need| &need.scopes)
            .map(|scope| json!({ "scope": scope, "held": kernel::principal::covers(&who.scopes, scope) }))
            .collect();
        let standing =
            approvals::stands(ask.status(), ask.at, (ask.updated, kept.updated), self.journal.now_ms());
        let may = match standing {
            Status::Waiting if who.actor() == ask.asker().actor() => {
                Err("You asked for this change: somebody who could make it decides it.".to_string())
            }
            Status::Waiting => {
                let fields = ask.fields();
                self.may_make(&collection, &ask.record, &fields, who).await.map_err(|refused| refused.message)
            }
            other => Err(approvals::decided(other, &ask.record, "", "")),
        };
        Ok(json!({
            "ask": ask.id, "project": ask.project, "record": ask.record, "collection": ask.collection,
            "title": kept.data.get("title").cloned().unwrap_or(Value::Null),
            "asker": { "actor": ask.asker().actor(), "name": ask.asker().name },
            "why": ask.why, "cells": shown, "takes": whole, "status": standing.as_str(), "decided": ask.decided, "at": ask.at,
            "may": may.is_ok(), "why_not": may.err(),
        }))
    }

    /// The asks `who` made, and those waiting that they could decide, the
    /// newest first.
    pub async fn listed(&self, who: &Principal) -> Result<Vec<Value>> {
        let mut listed = Vec::new();
        for ask in self.found("1 = 1 OR ?1 = ''", String::new()).await?.into_iter().rev() {
            let mine = ask.asker().actor() == who.actor();
            if !mine && !(ask.status() == Status::Waiting && ask.named.contains(&who.actor())) {
                continue;
            }
            if let Ok(shown) = self.shown(who, &ask.id).await {
                listed.push(shown);
            }
        }
        Ok(listed)
    }
}

/// Whether `account` is who `to` names: by its id, its name or its email.
pub(crate) fn named_by(account: &studio::Account, to: &str) -> bool {
    let to = to.trim().trim_start_matches("user:").trim_start_matches('@');
    account.id == to || account.name.eq_ignore_ascii_case(to) || account.email.eq_ignore_ascii_case(to)
}

/// The cells as an ask keeps them.
fn cells_json(cells: &[Cell]) -> Value {
    json!(
        cells
            .iter()
            .map(|cell| json!({ "name": cell.name, "before": cell.before, "after": cell.after }))
            .collect::<Vec<_>>()
    )
}

/// The cells an ask keeps, read back.
fn cells_of(kept: &Value) -> Vec<Cell> {
    kept.as_array()
        .into_iter()
        .flatten()
        .map(|cell| Cell {
            name: cell["name"].as_str().unwrap_or_default().to_string(),
            before: cell["before"].clone(),
            after: cell["after"].clone(),
        })
        .collect()
}

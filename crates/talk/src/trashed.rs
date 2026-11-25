//! A channel taken away into the Trash, and given back from it whole.
//!
//! What `Channels::remove` takes away for good is taken out here and kept
//! with the channel's row of the Trash, in the one write that takes it
//! out and says so: the channel's row and who is in it, the documents it
//! keeps to hand, everything said there, what is attached there, and the lines of anybody's inbox that
//! open it. What the journal kept of what was said goes from the journal
//! as it does for a channel taken away, since the Trash has the words
//! now; and what is attached is moved into the channel's folder of the
//! Trash, so that a copy of the factory, which copies that folder, has
//! the channel as the Trash keeps it. Whatever follows a channel going
//! hears that it went (`channel.removed`, saying `trash`), and forgets
//! what it holds of it as it always did.
//!
//! Given back, every row is written again as it was kept, in one write
//! with what is said of it (`channel.created`, saying `restored`; a line
//! put back in a box is told as one is), and a kept channel is kept to
//! the same members. An open one whose name another has taken meanwhile
//! comes back under the first of `<name>-2`, `<name>-3` that is free;
//! one whose project is gone is the own of the workspace it was in, or of
//! the usual one. What an agent remembered of its conversations there
//! went as it went and is not given back.
//!
//! The rows are kept as their columns are, read and written by name, so
//! that a column added to any of the tables later is kept with them and
//! one a channel from before lacks is left to its default.
//!
//! Which tables, and the row of the Trash, are said here; the row of the
//! Trash is the studio's to write, so it is handed in (`keep`, `out`).

use std::path::{Path, PathBuf};

use journal::Writing;
use kernel::trash::{Kind, Trashed};
use kernel::{Principal, Result, Source};
use rusqlite::types::{Value as Sql, ValueRef};
use serde_json::{Map, Value, json};

use crate::channels::{Channel, Channels, read_channels};
use crate::kept::{ALSO, READ};
use kernel::principal::MEMBER;

/// What a channel is in the Trash: each table that holds something of
/// one, with the column that says which channel, in the order its rows
/// are taken out and written again.
const HELD: [(&str, &str); 8] = [
    ("channels", "id"),
    ("channel_members", "channel"),
    ("channel_docs", "channel"),
    ("discussing", "channel"),
    ("discussions", "channel"),
    ("messages", "channel"),
    ("attachments", "channel"),
    ("inbox", "place"),
];

/// Where what is attached to a channel in the Trash is kept, in its
/// folder there.
const ATTACHED: &str = "kept";

/// The rows of `table` whose `column` is `value`, each as its columns
/// are, in the order they were written; and takes them away.
fn take(
    transaction: &rusqlite::Connection,
    table: &str,
    column: &str,
    value: &str,
) -> rusqlite::Result<Value> {
    let mut statement =
        transaction.prepare(&format!("SELECT * FROM {table} WHERE \"{column}\" = ?1 ORDER BY rowid"))?;
    let names: Vec<String> = statement.column_names().map(str::to_string).collect();
    let rows = statement.query_map([value], |row| {
        let mut one = Map::new();
        for (at, name) in names.iter().enumerate() {
            let value = match row.get_ref(at)? {
                ValueRef::Null => Value::Null,
                ValueRef::Integer(number) => json!(number),
                ValueRef::Real(number) => json!(number),
                ValueRef::Text(text) | ValueRef::Blob(text) => json!(String::from_utf8_lossy(text)),
            };
            one.insert(name.clone(), value);
        }
        Ok(Value::Object(one))
    })?;
    let rows: Vec<Value> = rows.collect::<rusqlite::Result<_>>()?;
    drop(statement);
    transaction.execute(&format!("DELETE FROM {table} WHERE \"{column}\" = ?1"), [value])?;
    Ok(Value::Array(rows))
}

/// Writes `rows` of `table` again as they were kept.
fn put(transaction: &rusqlite::Connection, table: &str, rows: &Value) -> rusqlite::Result<()> {
    for row in rows.as_array().into_iter().flatten().filter_map(Value::as_object) {
        let names: Vec<String> = row.keys().map(|name| format!("\"{name}\"")).collect();
        let holes: Vec<String> = (1..=row.len()).map(|at| format!("?{at}")).collect();
        let values = row.values().map(|value| match value {
            Value::Null => Sql::Null,
            Value::Bool(said) => Sql::Integer(i64::from(*said)),
            Value::Number(number) => match number.as_i64() {
                Some(whole) => Sql::Integer(whole),
                None => Sql::Real(number.as_f64().unwrap_or_default()),
            },
            Value::String(text) => Sql::Text(text.clone()),
            other => Sql::Text(other.to_string()),
        });
        transaction.execute(
            &format!("INSERT INTO {table} ({}) VALUES ({})", names.join(", "), holes.join(", ")),
            rusqlite::params_from_iter(values),
        )?;
    }
    Ok(())
}

/// The ids of what is attached in what the Trash keeps of a channel.
pub fn attached(kept: &Value) -> Vec<String> {
    let rows = kept["attachments"].as_array().into_iter().flatten();
    rows.filter_map(|row| row["id"].as_str()).filter(|id| is_name(id)).map(str::to_string).collect()
}

/// Whether an id is a file's name and nothing more: an id read back out
/// of the Trash is never followed somewhere else.
fn is_name(id: &str) -> bool {
    !id.is_empty() && !id.contains(['/', '\\']) && id != "." && id != ".."
}

/// Moves `from` to `to` where it is there, by renaming it, or by copying
/// it where the two are on different disks.
fn moved(from: &Path, to: &Path) {
    if !from.exists() {
        return;
    }
    if let Some(parent) = to.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if std::fs::rename(from, to).is_err() && std::fs::copy(from, to).is_ok() {
        let _ = std::fs::remove_file(from);
    }
}

/// What a channel given back is called: its own name, or where another
/// open channel has that now, the first after it that none has.
fn free_name(transaction: &rusqlite::Connection, name: &str) -> rusqlite::Result<String> {
    let taken = |name: &str| -> rusqlite::Result<bool> {
        transaction
            .query_row("SELECT EXISTS (SELECT 1 FROM channels WHERE name = ?1)", [name], |row| row.get(0))
    };
    if !taken(name)? {
        return Ok(name.to_string());
    }
    let mut nth = 2;
    loop {
        let said = format!("{name}-{nth}");
        if !taken(&said)? {
            return Ok(said);
        }
        nth += 1;
    }
}

impl Channels {
    /// Takes `channel` into the Trash, as the row of it `entry` gives
    /// (`trash`), with everything said and attached in it; `workspace` is
    /// the one it was the own of, to be given back to, and `folder` the
    /// Trash's folder of the data folder. `keep` writes the row of the
    /// Trash in the write that takes the channel out. The owner's to do,
    /// as taking one away is.
    pub async fn trash(
        &self,
        channel: &str,
        by: &Principal,
        (entry, workspace, folder): (String, Option<String>, Option<PathBuf>),
        keep: impl FnOnce(&mut Writing<'_>, &Trashed) -> rusqlite::Result<()> + Send + 'static,
    ) -> Result<(Channel, Trashed)> {
        by.require("admin")?;
        let found = self.found(channel, by).await?;
        let _turn = self.memory.changing().await;
        let channel = self.row(&found.id).await?;
        let remembered = self.memory.forget(&channel.id);
        let were: Vec<String> = channel.members.iter().map(|member| member.actor.clone()).collect();
        let were: Vec<&str> = were.iter().map(String::as_str).collect();
        let mut row = Trashed::new(entry, Kind::Channel, &channel.id, "", &by.actor(), self.journal.now_ms());
        row.called = match &channel.title {
            Some(title) if channel.kept => title.clone(),
            _ => format!("#{}", channel.name),
        };
        row.reads = Some(channel.takes(&were).unwrap_or_else(|| READ.to_string()));
        let source = Source::actor(by.actor()).taking(channel.takes(&were).as_deref());
        let (inbox, id, mut entry) = (self.inbox.clone(), channel.id.clone(), row);
        let wrote = self.journal.write(move |writing| {
            // First, while the channel is there to say who read it.
            let lines = {
                let transaction = writing.transaction();
                let lines = take(transaction, "inbox", "place", &id)?;
                put(transaction, "inbox", &lines)?;
                lines
            };
            inbox.forget_place(writing, &id)?;
            let mut kept = json!({ "inbox": lines, "workspace": workspace });
            for (table, column) in HELD.iter().filter(|(table, _)| *table != "inbox") {
                kept[*table] = take(writing.transaction(), table, column, &id)?;
            }
            let counted = |table: &str| kept[table].as_array().map_or(0, Vec::len);
            let said = json!({
                "channel": id,
                "name": kept["channels"][0]["name"],
                "project": kept["channels"][0]["project"],
                "messages": counted("messages"),
                "attachments": counted("attachments"),
                "trash": entry.id,
            });
            entry.kept = kept;
            keep(writing, &entry)?;
            writing.say_here("channel.removed", source, said)?;
            Ok(entry)
        });
        let entry = match wrote.await {
            Ok((entry, _)) => entry,
            Err(error) => {
                if let Some(members) = remembered {
                    self.memory.keep(&channel.id, members, channel.alone);
                }
                return Err(error);
            }
        };
        self.memory.archived.write().expect("members lock").remove(&channel.id);
        self.discussed.forget(&channel.id);
        // What is attached goes with it, after the write: stopped before,
        // it is found where it always was as the channel is given back.
        if let Some(folder) = folder.map(|folder| folder.join(&entry.id).join(ATTACHED)) {
            for id in attached(&entry.kept) {
                moved(&self.attachments.path(&id), &folder.join(&id));
            }
        }
        self.journal.forget_in_every("channel.message", "channel", &channel.id).await?;
        Ok((channel, entry))
    }

    /// Gives back a channel the Trash keeps as `entry` (read whole), in
    /// one write with `out`, which takes the row of the Trash away. Of a
    /// project still where `project_there` says; otherwise the own of the
    /// workspace it was, or of the usual one. Gives back the channel as it
    /// now is.
    pub async fn put_back(
        &self,
        entry: &Trashed,
        project_there: bool,
        folder: Option<PathBuf>,
        by: &Principal,
        out: impl FnOnce(&mut Writing<'_>) -> rusqlite::Result<()> + Send + 'static,
    ) -> Result<Channel> {
        let _turn = self.memory.changing().await;
        // What is attached is put back first: a stop after it finds the
        // files where a channel's are, and the channel still in the Trash.
        let folder = folder.map(|folder| folder.join(&entry.id).join(ATTACHED));
        for id in attached(&entry.kept) {
            if let Some(folder) = &folder {
                moved(&folder.join(&id), &self.attachments.path(&id));
            }
        }
        let (mut kept, id) = (entry.kept.clone(), entry.thing.clone());
        let (placing, actor, inbox) = (self.placing.get().cloned(), by.actor(), self.inbox.clone());
        let wrote = self.journal.write(move |writing| {
            let transaction = writing.transaction();
            let row = &mut kept["channels"][0];
            if row["kept"].as_i64() != Some(1) {
                let name = row["name"].as_str().unwrap_or(&id).to_string();
                row["name"] = json!(free_name(transaction, &name)?);
            }
            if !project_there {
                row["project"] = Value::Null;
            }
            for (table, _) in HELD.iter().filter(|(table, _)| *table != "inbox") {
                put(transaction, table, &kept[*table])?;
            }
            let channel = read_channels(transaction, "WHERE id = ?1", &[&id])?
                .pop()
                .ok_or(rusqlite::Error::QueryReturnedNoRows)?;
            let within: Vec<&str> = channel.members.iter().map(|member| member.actor.as_str()).collect();
            let source = Source::actor(&actor).taking(channel.takes(&within).as_deref());
            let said = json!({ "channel": channel.id, "name": channel.name, "project": channel.project, "restored": true });
            writing.say_here("channel.created", source.clone(), said)?;
            if let (None, Some(placing)) = (&channel.project, placing) {
                placing(writing, &channel, kept["workspace"].as_str(), source)?;
            }
            put(writing.transaction(), "inbox", &kept["inbox"])?;
            for line in kept["inbox"].as_array().into_iter().flatten() {
                let (owner, line) = (line["owner"].as_str().unwrap_or_default(), line["id"].as_str().unwrap_or_default());
                inbox.told_back(writing, owner, line)?;
            }
            out(writing)?;
            Ok(channel)
        });
        let channel = wrote.await?.0;
        self.discussed.taken_up(&self.journal, &channel.id).await?;
        if channel.kept {
            let within = channel.members.iter().map(|member| member.actor.clone());
            self.memory.keep(&channel.id, within, channel.alone);
        }
        if channel.archived.is_some() {
            self.memory
                .archived
                .write()
                .expect("members lock")
                .insert(channel.id.clone(), channel.name.clone());
        }
        if let Some(folder) = folder.and_then(|folder| folder.parent().map(Path::to_path_buf)) {
            let _ = std::fs::remove_dir_all(folder);
        }
        Ok(channel)
    }

    /// What is left of a channel the Trash kept as `entry`, once it is
    /// deleted for good: what is attached, wherever a stop left it, and
    /// anything the journal still held of what was said there.
    pub async fn forget_trashed(&self, entry: &Trashed) -> Result<()> {
        for id in attached(&entry.kept) {
            let _ = std::fs::remove_file(self.attachments.path(&id));
        }
        self.journal.forget_in_every("channel.message", "channel", &entry.thing).await?;
        Ok(())
    }

    /// Whether `by` reads a channel in the Trash that took `reads` to
    /// hear of (its row's `reads`): an open one by whoever reads
    /// channels, and a kept one as the one expression says of whoever was
    /// in it as it went.
    pub fn would_read(&self, reads: Option<&str>, by: &Principal) -> bool {
        let actor = by.actor();
        match reads.and_then(|reads| reads.strip_prefix(MEMBER)) {
            Some(kept) => {
                let member = kept.split(ALSO).skip(1).any(|one| one == actor);
                self.memory.reads_as(true, member, &actor, &by.scopes)
            }
            None => by.can(READ),
        }
    }
}

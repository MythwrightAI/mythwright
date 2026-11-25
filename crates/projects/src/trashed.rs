//! The Trash's rows and its folder: the writes a row of the Trash is
//! kept and taken out by, how long what is deleted is kept, and what goes
//! into the Trash and comes back out of it as files (a file or a folder of
//! a project) and as a project's row. The Trash itself, what can go there
//! and who sees what, is said where the rest of it is (`studio::trash`,
//! which offers all of this under the names it always had, and the `trash`
//! crate, which reads, gives back and empties it). The `trash` table is a
//! step of the records' tables, which are the studio's: its rows are
//! written here into a table this crate does not make.

use std::path::{Path, PathBuf};

use journal::{Journal, Writing, optional};
use kernel::trash::{Keeping, Whence, beside};
pub use kernel::trash::{Kind, Trashed};
use kernel::{Error, Principal, Result, Source};
use rusqlite::params;
use serde_json::{Value, json};

use crate::{PROJECT, Projects, blocking, read_project};

/// Where how long what is deleted is kept is kept, of the factory's small
/// durable state: it goes into a copy.
pub const KEEPS: &str = "trash.kept";
const KEY: &str = "factory";
/// The folder of the data folder files are kept in while they are in the
/// Trash: it goes into a copy.
pub const FOLDER: &str = "trash";
/// What says, beside a file kept in the Trash, where it came from.
pub const WHENCE: &str = "whence.json";
/// What the file or folder itself is called there.
pub const KEPT: &str = "kept";
/// What says a file kept in the Trash is going for good.
pub const GOING: &str = "going";

const COLUMNS: &str = "id, kind, thing, project, of_table, called, kept, reads, by, at";

fn read_trashed(row: &rusqlite::Row<'_>) -> rusqlite::Result<Trashed> {
    let kind = Kind::parse(&row.get::<_, String>(1)?);
    let (thing, project): (String, String) = (row.get(2)?, row.get(3)?);
    Ok(Trashed {
        id: row.get(0)?,
        kind,
        project: if kind == Kind::Project { thing.clone() } else { project },
        thing,
        table: row.get(4)?,
        called: row.get(5)?,
        kept: serde_json::from_str(&row.get::<_, String>(6)?).unwrap_or_default(),
        with: row.get::<_, i64>(10)?.max(0) as usize,
        reads: row.get(7)?,
        by: row.get(8)?,
        who: String::new(),
        at: row.get(9)?,
        goes: None,
        may: false,
    })
}

/// The board's table, by its name (`studio::board::BOARD`): reading one
/// of its rows takes reading the board.
pub const BOARD: &str = "board";

/// What hearing of it takes: what reading the thing does.
fn source(entry: &Trashed, by: &str) -> Source {
    Source::actor(by).taking(entry.taking(BOARD))
}

/// Writes `entry` into the Trash in `writing`, and says so.
pub fn put_in(writing: &mut Writing<'_>, entry: &Trashed) -> rusqlite::Result<()> {
    let project = if matches!(entry.kind, Kind::Project | Kind::Channel) {
        String::new()
    } else {
        entry.project.clone()
    };
    writing.transaction().execute(
        &format!("INSERT INTO trash ({COLUMNS}) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)"),
        params![
            entry.id,
            entry.kind.as_str(),
            entry.thing,
            project,
            entry.table,
            entry.called,
            entry.kept.to_string(),
            entry.reads,
            entry.by,
            entry.at
        ],
    )?;
    writing.say("trash.put", source(entry, &entry.by), entry.about())?;
    Ok(())
}

/// Takes `entry` out of the Trash in `writing`, and says why: `topic` is
/// `trash.restored` or `trash.gone`.
pub fn take_out(
    writing: &mut Writing<'_>,
    entry: &Trashed,
    topic: &str,
    by: &str,
    more: Value,
) -> rusqlite::Result<()> {
    writing.transaction().execute("DELETE FROM trash WHERE id = ?1", [&entry.id])?;
    // A project deleted for good takes where it kept its own blueprints, and its outline, with it (`crate::placed`).
    if entry.kind == Kind::Project && topic == "trash.gone" {
        writing.forget(texts::library::folders::NAMESPACE, &entry.thing)?;
        writing.forget(kernel::outline::NAMESPACE, &entry.thing)?;
    }
    let mut data = entry.about();
    if let (Value::Object(data), Value::Object(more)) = (&mut data, more) {
        data.extend(more);
    }
    writing.say(topic, source(entry, by), data)?;
    Ok(())
}

/// How long what is deleted is kept, as the factory's owner last said.
pub async fn keeping(journal: &Journal) -> Keeping {
    let kept = journal.kv_get(KEEPS, KEY).await.ok().flatten();
    kept.and_then(|kept| serde_json::from_value(kept).ok()).unwrap_or_default()
}

/// A row of the Trash for `thing`, deleted now by `by`.
pub fn new_entry(journal: &Journal, kind: Kind, thing: &str, project: &str, by: &Principal) -> Trashed {
    Trashed::new(journal.new_id("trash"), kind, thing, project, &by.actor(), journal.now_ms())
}

/// The record a file of a project is, by where it is (a library's
/// document, or a mockup's folder), which the tables say (`studio`'s
/// `Collections`).
pub trait KnownAt: Sync {
    fn known_at(&self, project: &str, path: &str)
    -> impl std::future::Future<Output = Option<String>> + Send;
}

/// Moves a file or a folder, whole: by renaming it where it can be, and
/// otherwise by copying it and taking the first away.
fn move_whole(from: &Path, to: &Path) -> std::io::Result<()> {
    if std::fs::rename(from, to).is_ok() {
        return Ok(());
    }
    copy_whole(from, to)?;
    match std::fs::metadata(from)?.is_dir() {
        true => std::fs::remove_dir_all(from),
        false => std::fs::remove_file(from),
    }
}

fn copy_whole(from: &Path, to: &Path) -> std::io::Result<()> {
    if !std::fs::metadata(from)?.is_dir() {
        return std::fs::copy(from, to).map(|_| ());
    }
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        copy_whole(&entry.path(), &to.join(entry.file_name()))?;
    }
    Ok(())
}

impl Projects {
    /// The folder files are kept in while they are in the Trash: none
    /// where the factory has no data folder of its own (a sandbox), which
    /// deletes them at once.
    pub fn trash_folder(&self) -> Option<PathBuf> {
        self.data.lock().expect("projects lock").as_ref().map(|data| data.join(FOLDER))
    }

    /// Deletes a file or a folder of a project, for somebody: into the
    /// Trash where the factory keeps what is deleted, at once where it
    /// keeps nothing. Asks what taking one away asks.
    pub async fn delete(&self, id: &str, path: &str, by: &Principal) -> Result<Option<String>> {
        match keeping(&self.journal).await.at_once() || self.trash_folder().is_none() {
            true => self.remove(id, path, by).await.map(|_| None),
            false => self.trash_path(id, path, by).await.map(|trashed| trashed.map(|trashed| trashed.id)),
        }
    }

    /// Moves a file or a folder of a project into the Trash. What says
    /// where it came from is written first, then the thing is moved, and
    /// then its row is kept: stopped between any two, a start finds what
    /// was begun and finishes it ([`Trash::settle`]).
    pub async fn trash_path(&self, id: &str, path: &str, by: &Principal) -> Result<Option<Trashed>> {
        by.require("projects:write")?;
        self.may_read_path(id, path, by).await?;
        if let Some(inside) = self.kept_from(id, by).iter().find(|kept| crate::closed::holds(path, kept)) {
            return Err(Error::denied(format!(
                "`{path}` holds `{inside}`, which is kept private in `{id}` and is not {}'s to take away",
                by.name
            )));
        }
        let Some(folder) = self.trash_folder() else {
            self.remove(id, path, by).await?;
            return Ok(None);
        };
        let dir = self.roots(id)?;
        let mut entry = new_entry(&self.journal, Kind::File, path, id, by);
        entry.reads = self.takes_to_read(id, path);
        let whence = Whence {
            id: entry.id.clone(),
            project: id.to_string(),
            path: path.to_string(),
            by: entry.by.clone(),
            at: entry.at,
            reads: entry.reads.clone(),
        };
        let (relative, kept) = (path.to_string(), folder.join(&entry.id));
        let entry_path = blocking(move || {
            let target = dir.resolve(&relative)?;
            if dir.is_top(&target) {
                return Err(Error::invalid("that is the project itself"));
            }
            if std::fs::symlink_metadata(&target).is_err() {
                return Err(Error::not_found(format!("there is no `{relative}`")));
            }
            std::fs::create_dir_all(&kept)?;
            std::fs::write(kept.join(WHENCE), serde_json::to_vec(&whence).unwrap_or_default())?;
            move_whole(&target, &kept.join(KEPT))?;
            Ok(whence.entry())
        })
        .await?;
        entry.called = entry_path.called;
        entry.kept = entry_path.kept;
        let row = entry.clone();
        let written = self.journal.records_of(Some(id)).write(move |writing| put_in(writing, &row));
        written.await?;
        // Whatever was under it is found, and said to be gone, by a scan.
        self.scan(id).await?;
        Ok(Some(entry))
    }

    /// Gives a file or a folder of a project back from the Trash, where it
    /// was or beside what is there now (`beside_it`). A document or a
    /// mockup given back beside it is known by where it is now, and what
    /// was said on it goes with it: which record a path is, is asked of
    /// `collections`.
    pub async fn restore_file(
        &self,
        journal: &Journal,
        entry: &Trashed,
        beside_it: bool,
        by: &Principal,
        collections: &impl KnownAt,
    ) -> Result<String> {
        by.require("projects:write")?;
        self.may_read_path(&entry.project, &entry.thing, by).await?;
        let dir = self.roots(&entry.project)?;
        let folder = self.trash_folder().ok_or_else(|| Error::internal("the factory has no Trash here"))?;
        let (path, kept, id) = (entry.thing.clone(), folder.join(&entry.id), entry.id.clone());
        let back = blocking(move || {
            let there = |path: &str| dir.resolve(path).is_ok_and(|at| at.exists());
            let to = match (there(&path), beside_it) {
                (false, _) => path.clone(),
                (true, true) => beside(&path, there),
                (true, false) => {
                    let cli = brand::CLI;
                    return Err(Error::conflict(format!(
                        "something is at `{path}` again, and it is not written over. Give it back beside it, \
                         as `{}`: `{cli} trash restore {id} --beside`, or Restore beside in the Trash",
                        beside(&path, there)
                    )));
                }
            };
            let target = dir.resolve(&to)?;
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            // Given back already, by a start that stopped before its row went.
            if kept.join(KEPT).exists() {
                move_whole(&kept.join(KEPT), &target)?;
            }
            Ok(to)
        })
        .await?;
        // Given back beside what took its place, a document or a mockup is
        // known by where it is now: what was said on it goes with it, in
        // the write that gives it back.
        let moved = match back == entry.thing {
            true => None,
            false => match (
                collections.known_at(&entry.project, &entry.thing).await,
                collections.known_at(&entry.project, &back).await,
            ) {
                (Some(was), Some(now)) => Some((was, now)),
                _ => None,
            },
        };
        let (row, actor, said) = (entry.clone(), by.actor(), json!({ "to": back }));
        let written = journal.write(move |writing| {
            if let Some((was, now)) = &moved {
                writing
                    .transaction()
                    .execute("UPDATE record_history SET record = ?2 WHERE record = ?1", [was, now])?;
            }
            take_out(writing, &row, "trash.restored", &actor, said)
        });
        written.await?;
        if let Some(folder) = self.trash_folder() {
            let _ = std::fs::remove_dir_all(folder.join(&entry.id));
        }
        self.scan(&entry.project).await?;
        Ok(back)
    }

    pub async fn restore_project(&self, entry: &Trashed, by: &Principal) -> Result<()> {
        by.require("projects:write")?;
        let kept = &entry.kept["project"];
        let text = |name: &str| kept[name].as_str().unwrap_or_default().to_string();
        let project = crate::Project {
            id: text("id"),
            name: text("name"),
            path: text("path"),
            brownfield: kept["brownfield"].as_bool().unwrap_or(true),
            trusted: kept["trusted"].as_bool().unwrap_or(false),
            created: kept["created"].as_i64().unwrap_or(entry.at),
            folder: crate::GIVEN,
            repository: false,
        };
        if project.id.is_empty() || project.path.is_empty() {
            return Err(Error::internal(format!("what is kept of `{}` cannot be read", entry.thing)));
        }
        if self.dir_of(&project.id).is_some() {
            return Err(Error::conflict(format!(
                "there is a project called `{}` again, and it is not given back over it",
                project.id
            )));
        }
        let dir = PathBuf::from(&project.path);
        let (holdings, made) = (self.holdings(), dir.clone());
        blocking(move || {
            if let Some(holdings) = holdings {
                holdings.take(&made)?;
            }
            Ok(std::fs::create_dir_all(&made)?)
        })
        .await?;
        self.journal.own(&project.id).await?;
        // Its own blueprints are back before it is, and back in the Trash where it is not.
        self.carry_place(&project.id, &entry.id, false)?;
        let workspace = entry.kept["workspace"].as_str().map(str::to_string);
        let (row, placing, out) = (project.clone(), self.placed.making(), entry.clone());
        let actor = by.actor();
        let written = self.journal.write(move |writing| {
            writing.transaction().execute(
                &format!("INSERT INTO projects ({PROJECT}) VALUES (?1, ?2, ?3, ?4, ?5, ?6)"),
                params![row.id, row.name, row.path, row.brownfield, row.created, row.trusted],
            )?;
            if let Some(placing) = placing {
                placing(writing, &row.id, workspace.as_deref(), &Source::actor(&actor))?;
            }
            take_out(writing, &out, "trash.restored", &actor, json!({}))
        });
        if let Err(err) = written.await {
            self.carry_place(&project.id, &entry.id, true)?;
            return Err(match err.kind {
                kernel::ErrorKind::Conflict => {
                    Error::conflict(format!("{} is another project's folder now", dir.display()))
                }
                _ => err,
            });
        }
        self.dirs.lock().expect("projects lock").insert(project.id.clone(), dir);
        if project.trusted {
            self.trusted.lock().expect("projects lock").insert(project.id.clone());
        }
        self.snapshot(&project.id, false).await?;
        let data = json!({
            "project": project.id, "name": project.name, "path": project.path,
            "brownfield": project.brownfield, "restored": true,
        });
        let _ = self.journal.emit("project.created", Source::actor(by.actor()), data).await;
        Ok(())
    }
}

/// Puts a project that is being forgotten into the Trash, in the write
/// that forgets it (`Projects::forget`): its row, and the workspace it was
/// in, read before either goes.
pub(crate) fn put_project(
    writing: &mut Writing<'_>,
    project: &str,
    (id, at, by): &(String, i64, String),
) -> rusqlite::Result<()> {
    let transaction = writing.transaction();
    let Some(row) = optional(transaction.query_row(
        &format!("SELECT {PROJECT} FROM projects WHERE id = ?1"),
        [project],
        read_project,
    ))?
    else {
        return Ok(());
    };
    let workspace: Option<String> = optional(transaction.query_row(
        "SELECT workspace FROM workspace_projects WHERE project = ?1",
        [project],
        |row| row.get(0),
    ))?;
    let mut entry = Trashed::new(id.clone(), Kind::Project, project, project, by, *at);
    entry.called = row.name.clone();
    entry.kept = json!({ "project": row, "workspace": workspace });
    put_in(writing, &entry)
}

/// What is in the Trash of `journal`, the latest first, with nothing of
/// what puts each back (a channel's is all that was said in it); or the
/// one called `id`, whole.
pub async fn rows(journal: &Journal, id: Option<String>) -> Result<Vec<Trashed>> {
    let columns = if id.is_some() { COLUMNS.to_string() } else { COLUMNS.replace(" kept,", " 'null',") };
    // How many went in with it, which a listing says without reading what is kept of them.
    let columns = format!("{columns}, coalesce(json_array_length(kept, '$.with'), 0)");
    journal
        .db()
        .call(move |connection| {
            let mut statement = connection.prepare(&format!(
                "SELECT {columns} FROM trash WHERE ?1 IS NULL OR id = ?1 ORDER BY at DESC"
            ))?;
            let rows = statement.query_map([&id], read_trashed)?;
            rows.collect()
        })
        .await
}

/// Says how long what is deleted is kept: so many days, nought for
/// deleting at once, or for ever with nothing. The owner's to say.
pub async fn keep_for(journal: &Journal, days: Option<u32>, by: &Principal) -> Result<Keeping> {
    by.require("admin")?;
    let days = kernel::trash::days(days)?;
    let now = Keeping { days, by: Some(by.name.clone()), at: Some(journal.now_ms()) };
    journal.kv_set(KEEPS, KEY, &json!(now)).await?;
    Ok(now)
}

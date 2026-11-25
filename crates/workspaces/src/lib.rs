//! Workspaces: what gathers the projects, and the channels, that belong
//! together.
//!
//! A project is one folder, with its board, its fleet and its channels.
//! Some things are made of several: an application, what is served for it
//! and its scripts are three folders and one piece of work to whoever
//! works on them. A workspace is the name for that: a name and the
//! projects in it. A channel may belong to one too, for talk that is of
//! no one project of it.
//!
//! **A factory always has a workspace, every project is always in one,
//! and every channel is a project's or a workspace's own.** The first
//! workspace is made for whoever owns the factory, called after them. A
//! project goes where whoever adds it says or, where they say nothing,
//! into the usual one: the workspace made first that is still there. A
//! channel made for no project is the own of the workspace whoever makes
//! it says, or of the usual one. So a project is put in another
//! workspace and never taken out of all of them, a channel is said to
//! belong to another thing and never to nothing, the last workspace is
//! never taken away, one that holds something is taken away only with
//! where that goes, and a project that is forgotten leaves its channels
//! to the workspace it was in. Nothing is the factory's own: what a
//! channel that belonged to nothing was, a channel of the usual
//! workspace is. The rule is brought about in one place
//! ([`Workspaces::settle`]), for a factory whose owner has just
//! registered and for one an earlier program made, in which a workspace
//! was something a project or a channel might not have.
//!
//! It is grouping and nothing else. Being in a workspace changes nothing
//! of a project (its id, its cards' ids, its folder, its fleet) and lets
//! nobody in and keeps nobody out: whoever may read channels reads the
//! channels of every workspace, as before there were any.
//!
//! What belongs where is kept here and not on the project or the channel:
//! a row for each project, saying its workspace, and one for each
//! channel that is a workspace's. A channel's project is where it always
//! was, on the channel, and a channel is a project's or a workspace's and
//! never both: the one write that says either takes the other away
//! ([`Workspaces::place_channel`]). What a row names may be gone (a
//! channel removed): such a row is passed over when it is read and taken
//! away when whoever removed the thing says so.
//!
//! What is written here is said in the write that keeps it
//! ([`journal::Journal::write`]): a workspace made, called something else
//! or taken away, what a channel belongs to, and that a project was
//! placed. Where a project is listed is a row of the factory's own and
//! nothing of the project's, so what says it was placed is kept in the
//! factory's own journal with that row (`journal::HERE`), whichever
//! journal keeps everything else said of the project: one write for
//! every project, and a daemon stopped anywhere leaves a project placed
//! and said to be, or neither. It is heard by project as any event is.
//!
//! It came out of `studio` whole when that crate was at its ceiling, and
//! is offered there under the names it always had (`studio::workspaces`,
//! `studio::Workspaces`): it asked nothing of the rest of it but the
//! projects' table and two things said in the writes of a project's row
//! ([`place_new`], [`forget_project`]), which `studio` hands its projects.
//! What it says is described with the studio's events, where it always
//! was, and its table is migrated under the name it always had
//! (`workspaces`).

use std::sync::Arc;

use journal::{Journal, Writing, optional};
use kernel::principal::MEMBER;
use kernel::{Error, Principal, Result, Source};
use rusqlite::{Connection, params};
use serde::Serialize;
use serde_json::json;

use talk::{Channel, Channels};

pub const SCHEMA: &[&str] = &["
    CREATE TABLE workspaces (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        created INTEGER NOT NULL
    );

    -- A project is in one workspace or in none.
    CREATE TABLE workspace_projects (
        project TEXT PRIMARY KEY,
        workspace TEXT NOT NULL
    ) WITHOUT ROWID;
    CREATE INDEX workspace_projects_of ON workspace_projects (workspace);

    -- A channel that is a workspace's and no one project's.
    CREATE TABLE workspace_channels (
        channel TEXT PRIMARY KEY,
        workspace TEXT NOT NULL
    ) WITHOUT ROWID;
    CREATE INDEX workspace_channels_of ON workspace_channels (workspace);
"];

/// A workspace, with what is in it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Workspace {
    /// A slug of the name it was made with. It does not change when the
    /// workspace is called something else.
    pub id: String,
    pub name: String,
    pub created: i64,
    /// Whether it is the usual one: where a project goes when nobody
    /// says, which is the workspace made first that is still there.
    pub usual: bool,
    /// The projects in it, by what each is called.
    pub projects: Vec<String>,
    /// The channels that are the workspace's own, by name. A channel of
    /// one of its projects is that project's and is not listed here.
    pub channels: Vec<String>,
}

/// What a channel belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Belongs<'a> {
    Nothing,
    Project(&'a str),
    Workspace(&'a str),
}

/// What bringing the rule about came to ([`Workspaces::settle`]).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Settled {
    /// The workspace that was made, where the factory had none.
    pub made: Option<String>,
    /// The projects that were in none and were put in the usual one.
    pub placed: Vec<String>,
    /// The channels that were no project's and no workspace's, or said a
    /// project that is gone, and were made the usual workspace's own.
    pub channels: Vec<String>,
}

/// The workspace made first that is still there: by when each was made,
/// and of two made in one moment the one written first.
const USUAL: &str = "SELECT id FROM workspaces ORDER BY created, rowid LIMIT 1";

/// The workspace a project is in, passing over a row that names one
/// that is gone.
const OF_PROJECT: &str =
    "SELECT w.workspace FROM workspace_projects w JOIN workspaces s ON s.id = w.workspace
     WHERE w.project = ?1";

const PLACE: &str = "INSERT INTO workspace_projects (project, workspace) VALUES (?1, ?2)
     ON CONFLICT (project) DO UPDATE SET workspace = excluded.workspace";

const PLACE_CHANNEL: &str = "INSERT INTO workspace_channels (channel, workspace) VALUES (?1, ?2)
     ON CONFLICT (channel) DO UPDATE SET workspace = excluded.workspace";

const MAKE: &str = "INSERT INTO workspaces (id, name, created) VALUES (?1, ?2, ?3)";

/// What hearing of a channel takes: nothing for an open one, and being a
/// member for one that is kept to its members, as everything said of
/// such a channel is said.
fn takes(channel: &str, kept: bool) -> Option<String> {
    kept.then(|| format!("{MEMBER}{channel}"))
}

/// What the first workspace is called, after the factory's owner: `Ada's
/// workspace`, and `Unnamed workspace` for the owner a first start made
/// whom nobody has named yet ([`kernel::UNNAMED`]).
pub fn called_after(owner: &str) -> String {
    match owner.trim() {
        kernel::UNNAMED => "Unnamed workspace".to_string(),
        owner => format!("{owner}'s workspace"),
    }
}

/// What says a channel was made a workspace's own.
fn placed(channel: &str, name: &str, workspace: &str) -> serde_json::Value {
    json!({ "channel": channel, "name": name, "project": null, "workspace": workspace })
}

/// The workspace `asked` names by its id, where there is one, and
/// otherwise the usual one: nothing only where the factory has none.
fn named_or_usual(connection: &Connection, asked: Option<&str>) -> rusqlite::Result<Option<String>> {
    let named = match asked {
        Some(asked) => {
            optional(connection.query_row("SELECT id FROM workspaces WHERE id = ?1", [asked], |row| {
                row.get::<_, String>(0)
            }))?
        }
        None => None,
    };
    named.map_or_else(|| usual(connection), |named| Ok(Some(named)))
}

/// Makes a channel that is being made for no project the own of the
/// workspace whoever makes it asked for, where that is there, and
/// otherwise of the usual one: in the write that makes its row
/// (`Channels::makes_with`), so that no channel is ever written that
/// belongs to nothing, and says so there. Where the factory has no
/// workspace yet (nobody owns it) nothing is written:
/// [`Workspaces::settle`] places it when there is one.
fn place_made(
    writing: &mut Writing<'_>,
    channel: &Channel,
    asked: Option<&str>,
    by: Source,
) -> rusqlite::Result<()> {
    let transaction = writing.transaction();
    let Some(workspace) = named_or_usual(transaction, asked)? else { return Ok(()) };
    transaction.execute(PLACE_CHANNEL, params![channel.id, workspace])?;
    writing.say_here("channel.placed", by, placed(&channel.id, &channel.name, &workspace))?;
    Ok(())
}

/// A project is being forgotten, in the write that forgets it
/// (`studio::placed`): its channels are the own of the workspace
/// it was in from here on, each said, and no workspace lists it any
/// more. A channel is a project's or a workspace's, and a project that
/// is gone is neither. Where the factory has no workspace (nobody owns
/// it) its channels are left saying it, and are placed when the factory
/// is settled.
pub fn forget_project(writing: &mut Writing<'_>, project: &str, by: &Source) -> rusqlite::Result<()> {
    let transaction = writing.transaction();
    let was: Option<String> = optional(transaction.query_row(OF_PROJECT, [project], |row| row.get(0)))?;
    let into = match was {
        Some(was) => Some(was),
        None => usual(transaction)?,
    };
    if let Some(into) = into {
        let channels: Vec<(String, String, bool)> = transaction
            .prepare("SELECT id, name, kept FROM channels WHERE project = ?1 ORDER BY name")?
            .query_map([project], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
            .collect::<rusqlite::Result<_>>()?;
        for (channel, name, kept) in channels {
            transaction.execute("UPDATE channels SET project = NULL WHERE id = ?1", [&channel])?;
            transaction.execute(PLACE_CHANNEL, params![channel, into])?;
            let source = by.clone().taking(takes(&channel, kept).as_deref());
            writing.say_here("channel.placed", source, placed(&channel, &name, &into))?;
        }
    }
    transaction.execute("DELETE FROM workspace_projects WHERE project = ?1", [project])?;
    Ok(())
}

/// What somebody is told who says a channel belongs to nothing.
pub fn to_nothing(channel: &str) -> Error {
    let cli = brand::CLI;
    Error::invalid(format!(
        "a channel is a project's or a workspace's, and never nobody's: say which `{channel}` is to be, \
         `{cli} channel belongs {channel} --workspace <workspace>` or `--project <project>`"
    ))
}

fn usual(connection: &Connection) -> rusqlite::Result<Option<String>> {
    optional(connection.query_row(USUAL, [], |row| row.get(0)))
}

/// Puts a project that is being made in the workspace whoever adds it
/// asked for, where that is there, and otherwise in the usual one: in
/// the write that makes its row (`studio::placed`), so that no
/// project is ever written without its place, and says so there. Where
/// the factory has no workspace yet (nobody owns it) nothing is written:
/// [`Workspaces::settle`] places it when there is one.
pub fn place_new(
    writing: &mut Writing<'_>,
    project: &str,
    asked: Option<&str>,
    by: &Source,
) -> rusqlite::Result<()> {
    let transaction = writing.transaction();
    let Some(workspace) = named_or_usual(transaction, asked)? else { return Ok(()) };
    transaction.execute(PLACE, params![project, workspace])?;
    writing.say_here("project.placed", by.clone(), json!({ "project": project, "workspace": workspace }))?;
    Ok(())
}

fn nameless() -> Error {
    Error::invalid("a workspace needs a name with at least one letter or digit")
}

/// What a write of this module's came to, where it may find things
/// other than they were when it was asked for.
enum Wrote {
    Done,
    /// A workspace it names is not there any more.
    Gone(String),
    /// It would have taken the last workspace away.
    Last,
    /// It would have left what the workspace holds in none.
    Holds,
    /// It holds a channel that was not there when whoever asked was
    /// asked whether they may move its channels.
    Moved,
}

#[derive(Clone)]
pub struct Workspaces {
    journal: Journal,
    channels: Channels,
}

impl Workspaces {
    /// Whoever keeps the projects is told, as this is opened, to make a
    /// project with its place and forget one with where its channels go
    /// ([`place_new`], [`forget_project`]): `studio` does, as it opens
    /// both, since this stands below it and knows of the projects only
    /// their table.
    pub async fn open(journal: Journal, channels: Channels) -> Result<Workspaces> {
        journal.db().migrate("workspaces", SCHEMA).await?;
        // From here on a channel that is no project's is made with the
        // workspace it is the own of.
        channels.makes_with(Arc::new(place_made));
        Ok(Workspaces { journal, channels })
    }

    /// The project of that id, as whoever keeps the projects answers for
    /// one that is not there (`studio::Projects::get`): its row is in the
    /// factory's own database, beside this crate's tables.
    async fn project(&self, id: &str) -> Result<String> {
        let lookup = id.to_string();
        self.journal
            .db()
            .call(move |connection| {
                optional(
                    connection
                        .query_row("SELECT id FROM projects WHERE id = ?1", [&lookup], |row| row.get(0)),
                )
            })
            .await?
            .ok_or_else(|| Error::not_found(format!("there is no project `{id}`")))
    }

    /// Every workspace, by what it is called, one with nothing in it
    /// among them. The order does not move: by name as it reads, and by
    /// id where two read alike.
    pub async fn list(&self) -> Result<Vec<Workspace>> {
        self.journal
            .db()
            .call(|connection| {
                let mut statement = connection
                    .prepare("SELECT id, name, created FROM workspaces ORDER BY lower(name), id")?;
                let rows = statement.query_map([], |row| {
                    Ok(Workspace {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        created: row.get(2)?,
                        usual: false,
                        projects: Vec::new(),
                        channels: Vec::new(),
                    })
                })?;
                let mut all: Vec<Workspace> = rows.collect::<rusqlite::Result<_>>()?;
                let first = usual(connection)?;
                // Only what is there: a row may name a project that was
                // forgotten or a channel that was removed.
                let mut projects = connection.prepare(
                    "SELECT p.id FROM workspace_projects w JOIN projects p ON p.id = w.project
                     WHERE w.workspace = ?1 ORDER BY lower(p.name), p.id",
                )?;
                let mut channels = connection.prepare(
                    "SELECT c.id FROM workspace_channels w JOIN channels c ON c.id = w.channel
                     WHERE w.workspace = ?1 ORDER BY c.name",
                )?;
                for one in &mut all {
                    one.usual = first.as_deref() == Some(one.id.as_str());
                    one.projects =
                        projects.query_map([&one.id], |row| row.get(0))?.collect::<rusqlite::Result<_>>()?;
                    one.channels =
                        channels.query_map([&one.id], |row| row.get(0))?.collect::<rusqlite::Result<_>>()?;
                }
                Ok(all)
            })
            .await
    }

    /// A workspace by its id, or by what it is called.
    pub async fn get(&self, id_or_name: &str) -> Result<Workspace> {
        let slug = blueprint::expr::slug(id_or_name);
        let found = self
            .list()
            .await?
            .into_iter()
            .find(|one| one.id == id_or_name || one.id == slug || one.name == id_or_name);
        found.ok_or_else(|| Error::not_found(format!("there is no workspace `{id_or_name}`")))
    }

    /// The usual workspace: where a project goes when nobody says, which
    /// is the one made first that is still there. None only where the
    /// factory has no workspace, which is while nobody owns it.
    pub async fn usual(&self) -> Result<Option<String>> {
        self.journal.db().call(|connection| usual(connection)).await
    }

    /// Brings about that the factory has a workspace, that every project
    /// is in one and that every channel is a project's or a workspace's
    /// own, and is the same done twice: where there is no workspace one
    /// is made, called after `owner` (`Ada's workspace`), each project
    /// that is in none is put in the usual one, and each channel that
    /// belongs to nothing, or says a project that is gone, is made the
    /// usual one's own. Each thing it writes it says, as the system (of
    /// a channel kept to its members, to them), and where there is
    /// nothing to bring about it writes nothing and says nothing.
    ///
    /// It is asked as the factory's owner registers and as a daemon
    /// starts, which is what takes up a factory an earlier program made,
    /// and what closes whatever a stop left undone; and nowhere else,
    /// since nothing else leaves a project in none. With no workspace
    /// and nobody to call one after, nothing is made until there is
    /// somebody.
    pub async fn settle(&self, owner: Option<&str>) -> Result<Settled> {
        let name = owner.map(called_after);
        let now = self.journal.now_ms();
        let system = || Source::actor("system");
        let written = self.journal.write(move |writing| {
            let transaction = writing.transaction();
            let mut settled = Settled::default();
            let into = match (usual(transaction)?, name) {
                (Some(usual), _) => usual,
                (None, None) => return Ok(settled),
                (None, Some(name)) => {
                    // It has a letter in it whatever its owner is called.
                    let id = blueprint::expr::slug(&name);
                    transaction.execute(MAKE, params![id, name, now])?;
                    writing.say("workspace.created", system(), json!({ "workspace": id, "name": name }))?;
                    settled.made = Some(id.clone());
                    id
                }
            };
            // Each project that is in none: one with no row, and one
            // whose row names a workspace that is gone.
            let mut loose = transaction.prepare(
                "SELECT p.id FROM projects p
                 WHERE NOT EXISTS (SELECT 1 FROM workspace_projects w JOIN workspaces s ON s.id = w.workspace
                                   WHERE w.project = p.id)
                 ORDER BY p.id",
            )?;
            settled.placed = loose.query_map([], |row| row.get(0))?.collect::<rusqlite::Result<_>>()?;
            for project in &settled.placed {
                transaction.execute(PLACE, params![project, into])?;
                writing.say_here(
                    "project.placed",
                    system(),
                    json!({ "project": project, "workspace": into }),
                )?;
            }
            // Each channel that is nobody's: no project's and no
            // workspace's own, or a project's that is not there.
            let loose: Vec<(String, String, bool)> = transaction
                .prepare(
                    "SELECT c.id, c.name, c.kept FROM channels c
                     WHERE (c.project IS NULL OR NOT EXISTS (SELECT 1 FROM projects p WHERE p.id = c.project))
                       AND NOT EXISTS (SELECT 1 FROM workspace_channels w JOIN workspaces s ON s.id = w.workspace
                                       WHERE w.channel = c.id)
                     ORDER BY c.name",
                )?
                .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
                .collect::<rusqlite::Result<_>>()?;
            for (channel, name, kept) in loose {
                transaction.execute("UPDATE channels SET project = NULL WHERE id = ?1", [&channel])?;
                transaction.execute(PLACE_CHANNEL, params![channel, into])?;
                let source = system().taking(takes(&channel, kept).as_deref());
                writing.say_here("channel.placed", source, placed(&channel, &name, &into))?;
                settled.channels.push(channel);
            }
            Ok(settled)
        });
        Ok(written.await?.0)
    }

    /// Makes a workspace, with nothing in it.
    pub async fn create(&self, name: &str, by: &Principal) -> Result<Workspace> {
        by.require("projects:write")?;
        let name = name.trim().to_string();
        let id = blueprint::expr::slug(&name);
        if id.is_empty() {
            return Err(nameless());
        }
        let (row, source) = ((id.clone(), name, self.journal.now_ms()), Source::actor(by.actor()));
        let written = self.journal.write(move |writing| {
            writing.transaction().execute(MAKE, params![row.0, row.1, row.2])?;
            writing.say("workspace.created", source, json!({ "workspace": row.0, "name": row.1 }))?;
            Ok(())
        });
        written.await.map_err(|err| match err.kind {
            kernel::ErrorKind::Conflict => {
                Error::conflict(format!("there is already a workspace called `{id}`"))
            }
            _ => err,
        })?;
        self.get(&id).await
    }

    /// Calls a workspace something else. Its id is what it was, so
    /// nothing that names it moves.
    pub async fn rename(&self, workspace: &str, name: &str, by: &Principal) -> Result<Workspace> {
        by.require("projects:write")?;
        let was = self.get(workspace).await?;
        let name = name.trim().to_string();
        if blueprint::expr::slug(&name).is_empty() {
            return Err(nameless());
        }
        if name == was.name {
            return Ok(was);
        }
        let (id, before, source) = (was.id.clone(), was.name, Source::actor(by.actor()));
        let written = self.journal.write(move |writing| {
            writing
                .transaction()
                .execute("UPDATE workspaces SET name = ?2 WHERE id = ?1", params![id, name])?;
            writing.say(
                "workspace.renamed",
                source,
                json!({ "workspace": id, "name": name, "was": before }),
            )?;
            Ok(())
        });
        written.await?;
        self.get(&was.id).await
    }

    /// The workspace made with the owner a factory made as it first
    /// started, called after them once they say what they are called: each
    /// still called `Unnamed workspace`, and none that anybody called
    /// anything else meanwhile. Its id is what it was made with
    /// (`unnamed-workspace`), as every workspace's is: what was kept of it
    /// by its id (the panel's choice, the tabs of each workspace, an
    /// address) is kept. Said as the owner, who named themselves.
    pub async fn follow_owner(&self, now: &str, by: &Principal) -> Result<Vec<String>> {
        let (before, after) = (called_after(kernel::UNNAMED), called_after(now));
        if before == after {
            return Ok(Vec::new());
        }
        let actor = by.actor();
        let written = self.journal.write(move |writing| {
            let ids: Vec<String> = writing
                .transaction()
                .prepare("SELECT id FROM workspaces WHERE name = ?1 ORDER BY id")?
                .query_map([&before], |row| row.get(0))?
                .collect::<rusqlite::Result<_>>()?;
            for id in &ids {
                writing
                    .transaction()
                    .execute("UPDATE workspaces SET name = ?2 WHERE id = ?1", params![id, after])?;
                writing.say(
                    "workspace.renamed",
                    Source::actor(actor.clone()),
                    json!({ "workspace": id, "name": after, "was": before }),
                )?;
            }
            Ok(ids)
        });
        Ok(written.await?.0)
    }

    /// Takes a workspace away, and nothing that was in it: its projects
    /// and its own channels are in `to` afterwards, put there in the
    /// write that takes the workspace away and each said to have been,
    /// and are as they were in every other way. One with nothing in it
    /// needs no `to`. The last workspace there is stays: a factory
    /// always has one.
    ///
    /// Moving a channel by taking its workspace away is asked what
    /// saying what the channel belongs to is asked
    /// ([`Self::place_channel`]): changing channels, and reading the
    /// channel where it is kept to its members. So one that holds
    /// channels of its own is taken away by somebody who could have
    /// placed each of them, and nobody moves by removing what they could
    /// not move by placing. One that holds none asks nothing of channels.
    pub async fn remove(&self, workspace: &str, to: Option<&str>, by: &Principal) -> Result<Workspace> {
        by.require("projects:write")?;
        let cli = brand::CLI;
        let gone = self.get(workspace).await?;
        let last = || {
            Error::conflict(format!(
                "`{}` is the only workspace, and a factory always has one. It can be called something \
                 else instead: `{cli} workspace rename {} <name>`",
                gone.name, gone.id
            ))
        };
        let holds = || {
            Error::conflict(format!(
                "`{}` is not empty, and what is in it is always in a workspace: say where its projects \
                 and channels go, `{cli} workspace remove {} --to <workspace>`",
                gone.name, gone.id
            ))
        };
        let into = match to {
            Some(to) => Some(self.get(to).await?.id),
            None => None,
        };
        if into.as_deref() == Some(gone.id.as_str()) {
            return Err(Error::invalid(format!(
                "what is in `{}` cannot go to `{}`, which is the workspace being taken away: name another",
                gone.name, gone.id
            )));
        }
        // Whoever moves its channels may place each of them: asked of the
        // channel, as placing one is. One kept from them is named to
        // nobody, here either: only that there is one is said, which
        // being told the workspace is not empty already says.
        if into.is_some() {
            for channel in &gone.channels {
                match self.channels.may_write(channel, by).await {
                    Ok(_) => {}
                    Err(refused) if refused.kind == kernel::ErrorKind::NotFound => {
                        return Err(Error::denied(format!(
                            "`{}` holds a channel that is not yours to move: what a channel belongs to is \
                             said by somebody who reads it and may change channels. Have one of them say \
                             what it belongs to (`{cli} channel belongs`), or take the workspace away",
                            gone.name
                        )));
                    }
                    Err(refused) => return Err(refused),
                }
            }
        }
        let asked: Vec<String> = gone.channels.clone();
        // Said to whoever may read projects, so of the channels that were
        // its own only those anybody reads: one kept to its members is
        // named to nobody here, and that it moved is said to its members.
        let kept = self.channels.kept_from(None).await?;
        let open: Vec<&String> = gone.channels.iter().filter(|channel| !kept.contains(*channel)).collect();
        let mut said =
            json!({ "workspace": gone.id, "name": gone.name, "projects": gone.projects, "channels": open });
        if let Some(into) = &into {
            said["to"] = json!(into);
        }
        let (id, source) = (gone.id.clone(), Source::actor(by.actor()));
        let written = self.journal.write(move |writing| {
            let transaction = writing.transaction();
            // As things are in this write, which may not be as they were
            // when it was asked for.
            let others: i64 =
                transaction
                    .query_row("SELECT count(*) FROM workspaces WHERE id != ?1", [&id], |row| row.get(0))?;
            if others == 0 {
                return Ok(Wrote::Last);
            }
            let projects: Vec<String> = transaction
                .prepare(
                    "SELECT w.project FROM workspace_projects w JOIN projects p ON p.id = w.project
                     WHERE w.workspace = ?1 ORDER BY w.project",
                )?
                .query_map([&id], |row| row.get(0))?
                .collect::<rusqlite::Result<_>>()?;
            let channels: Vec<(String, String)> = transaction
                .prepare(
                    "SELECT c.id, c.name FROM workspace_channels w JOIN channels c ON c.id = w.channel
                     WHERE w.workspace = ?1 ORDER BY c.name",
                )?
                .query_map([&id], |row| Ok((row.get(0)?, row.get(1)?)))?
                .collect::<rusqlite::Result<_>>()?;
            // One made the workspace's own since whoever asks was asked
            // whether they may move its channels is not moved unasked.
            if into.is_some() && channels.iter().any(|(channel, _)| !asked.contains(channel)) {
                return Ok(Wrote::Moved);
            }
            match &into {
                Some(into) => {
                    let there: Option<String> = optional(transaction.query_row(
                        "SELECT id FROM workspaces WHERE id = ?1",
                        [into],
                        |row| row.get(0),
                    ))?;
                    if there.is_none() {
                        return Ok(Wrote::Gone(into.clone()));
                    }
                    transaction.execute(
                        "UPDATE workspace_projects SET workspace = ?2 WHERE workspace = ?1",
                        params![id, into],
                    )?;
                    transaction.execute(
                        "UPDATE workspace_channels SET workspace = ?2 WHERE workspace = ?1",
                        params![id, into],
                    )?;
                }
                None if !(projects.is_empty() && channels.is_empty()) => return Ok(Wrote::Holds),
                None => {}
            }
            // What a row names may be gone: such a row goes with the workspace.
            transaction.execute("DELETE FROM workspace_projects WHERE workspace = ?1", [&id])?;
            transaction.execute("DELETE FROM workspace_channels WHERE workspace = ?1", [&id])?;
            transaction.execute("DELETE FROM workspaces WHERE id = ?1", [&id])?;
            writing.say("workspace.removed", source.clone(), said)?;
            for project in projects {
                let moved = json!({ "project": project, "workspace": into, "was": id });
                writing.say_here("project.placed", source.clone(), moved)?;
            }
            for (channel, name) in channels {
                // Of a channel kept to its members it is said to them.
                let Some(into) = &into else { continue };
                let takes = takes(&channel, kept.contains(&channel));
                let moved = placed(&channel, &name, into);
                writing.say_here("channel.placed", source.clone().taking(takes.as_deref()), moved)?;
            }
            Ok(Wrote::Done)
        });
        match written.await?.0 {
            Wrote::Done => self.shown_to(gone, by).await,
            Wrote::Gone(into) => Err(Error::not_found(format!("there is no workspace `{into}`"))),
            Wrote::Last => Err(last()),
            Wrote::Holds => Err(holds()),
            Wrote::Moved => Err(Error::conflict(format!(
                "what `{}` holds changed as it was being taken away, and nothing was: ask again",
                workspace
            ))),
        }
    }

    /// A workspace as `by` is shown it: without the channels of its own
    /// that are kept to their members and that `by` is not one of, and
    /// without the conversations, which no list of channels has. It is
    /// still grouping and nothing else: every channel anybody reads is
    /// listed for everybody, whichever workspace it is of, and what is
    /// left out is left out by the channel, which is asked.
    pub async fn shown_to(&self, mut workspace: Workspace, by: &Principal) -> Result<Workspace> {
        if !workspace.channels.is_empty() {
            let kept = self.channels.unlisted_for(by).await?;
            workspace.channels.retain(|channel| !kept.contains(channel));
        }
        Ok(workspace)
    }

    /// Every workspace as `by` is shown it ([`Self::shown_to`]).
    pub async fn list_for(&self, by: &Principal) -> Result<Vec<Workspace>> {
        let mut all = self.list().await?;
        if all.iter().any(|one| !one.channels.is_empty()) {
            let kept = self.channels.unlisted_for(by).await?;
            for one in &mut all {
                one.channels.retain(|channel| !kept.contains(channel));
            }
        }
        Ok(all)
    }

    /// The workspace a project is in: none only for one that is not
    /// there, or while the factory has no workspace.
    pub async fn of_project(&self, project: &str) -> Result<Option<String>> {
        let lookup = project.to_string();
        self.journal
            .db()
            .call(move |connection| optional(connection.query_row(OF_PROJECT, [&lookup], |row| row.get(0))))
            .await
    }

    /// The workspace a channel is the own of, if it is one's.
    pub async fn of_channel(&self, channel: &str) -> Result<Option<String>> {
        let lookup = channel.to_string();
        self.journal
            .db()
            .call(move |connection| {
                optional(connection.query_row(
                    "SELECT w.workspace FROM workspace_channels w JOIN workspaces s ON s.id = w.workspace
                     WHERE w.channel = ?1",
                    [&lookup],
                    |row| row.get(0),
                ))
            })
            .await
    }

    /// Puts a project in a workspace, out of the one it was in. Nothing
    /// of the project changes by it. Done again it is the same, and says
    /// nothing a second time. With no workspace said it is refused: a
    /// project is always in one, and is put in another to leave this.
    pub async fn place_project(
        &self,
        project: &str,
        workspace: Option<&str>,
        by: &Principal,
    ) -> Result<Workspace> {
        by.require("projects:write")?;
        let project = self.project(project).await?;
        let Some(workspace) = workspace else {
            return Err(Error::invalid(format!(
                "a project is always in a workspace: `{project}` is put in another, and never taken out of \
                 the one it is in. `{cli} workspace put <workspace> {project}`",
                cli = brand::CLI
            )));
        };
        let into = self.get(workspace).await?;
        let (row, source) = ((project, into.id.clone()), Source::actor(by.actor()));
        let written = self.journal.write(move |writing| {
            let transaction = writing.transaction();
            let (project, into) = row;
            let was: Option<String> =
                optional(transaction.query_row(OF_PROJECT, [&project], |row| row.get(0)))?;
            if was.as_deref() == Some(into.as_str()) {
                return Ok(Wrote::Done);
            }
            // As things are in this write: taken away since it was asked
            // for, a workspace is nowhere to put a project.
            let there: Option<String> = optional(transaction.query_row(
                "SELECT id FROM workspaces WHERE id = ?1",
                [&into],
                |row| row.get(0),
            ))?;
            if there.is_none() {
                return Ok(Wrote::Gone(into));
            }
            transaction.execute(PLACE, params![project, into])?;
            writing.say_here(
                "project.placed",
                source,
                json!({ "project": project, "workspace": into, "was": was }),
            )?;
            Ok(Wrote::Done)
        });
        match written.await?.0 {
            Wrote::Gone(into) => Err(Error::not_found(format!("there is no workspace `{into}`"))),
            _ => self.get(&into.id).await,
        }
    }

    /// Says what a channel belongs to: a project or a workspace. It is
    /// one or the other and never both, so saying either takes the other
    /// away, in one write with what is said of it. Done again it is the
    /// same, and says nothing a second time. Belonging to nothing is
    /// refused: a channel is always one or the other.
    ///
    /// The channel's project is a column of a table another crate makes
    /// (`channels.project`, `talk::channels`), written here by a statement
    /// of this crate's own and nowhere else after the channel is made: it
    /// is one transaction with the row of `workspace_channels`, which is
    /// this crate's, and `talk` stands below and cannot write that. The
    /// head of `talk::channels` says the same from its side; the test
    /// that holds the two together is the studio suite's
    /// `workspaces::a_channel_belongs_to_a_project_or_to_a_workspace_and_never_to_both`,
    /// which reads the channel back through `Channels::get`.
    pub async fn place_channel(
        &self,
        channel: &str,
        belongs: Belongs<'_>,
        by: &Principal,
    ) -> Result<Channel> {
        // Asked of the channel: one kept from `by` is not there to place.
        let reader = self.channels.may_write(channel, by).await?;
        let channel = self.channels.get(&reader, channel).await?;
        let (project, workspace) = match belongs {
            Belongs::Nothing => return Err(to_nothing(&channel.name)),
            Belongs::Project(project) => (Some(self.project(project).await?), None),
            Belongs::Workspace(workspace) => (None, Some(self.get(workspace).await?.id)),
        };
        let was = self.of_channel(&channel.id).await?;
        if channel.project == project && was == workspace {
            return Ok(channel);
        }
        let row = (channel.id.clone(), project.clone(), workspace.clone());
        let said = json!({
            "channel": channel.id, "name": channel.name, "project": project, "workspace": workspace,
        });
        // Of a channel kept to its members it is said to them.
        let source = Source::actor(by.actor()).taking(channel.takes(&[]).as_deref());
        let written = self.journal.write(move |writing| {
            let transaction = writing.transaction();
            transaction.execute("UPDATE channels SET project = ?2 WHERE id = ?1", params![row.0, row.1])?;
            match &row.2 {
                Some(workspace) => transaction.execute(PLACE_CHANNEL, params![row.0, workspace])?,
                None => transaction.execute("DELETE FROM workspace_channels WHERE channel = ?1", [&row.0])?,
            };
            writing.say_here("channel.placed", source, said)?;
            Ok(())
        });
        written.await?;
        self.channels.get(&reader, &channel.id).await
    }

    /// A channel was removed: it is no workspace's any more.
    pub async fn forgot_channel(&self, channel: &str) -> Result<()> {
        let lookup = channel.to_string();
        self.journal
            .db()
            .call(move |connection| {
                connection.execute("DELETE FROM workspace_channels WHERE channel = ?1", [&lookup])
            })
            .await?;
        Ok(())
    }
}

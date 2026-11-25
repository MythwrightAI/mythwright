//! Groups of people and agents, and what each group's members may ask.
//!
//! A group is a name, its members (people by their accounts, `user:<id>`,
//! and agents by their names, `agent:<name>`) and what it holds: a list of
//! scopes, by area as anything held is, which each member holds beside
//! what it holds itself. It is the factory's and nobody's project's: two
//! tables of the factory's own (`groups`, `group_members`), with a list of
//! steps of their own, kept in a copy with the accounts and moved with no
//! project. A group holds no group: one level.
//!
//! **Who changes one** is whoever manages the factory (`admin`), and
//! nobody gives by one what they do not hold: a scope added to what a
//! group holds is held by whoever adds it, and somebody is put in a group
//! only by whoever holds everything the group holds. What was given stays
//! given if whoever gave it holds less later, as a scope given to a person
//! does ([`crate::given`]). **An agent never changes a group it is in**,
//! nor puts itself in one or takes itself out, as it never changes its own
//! slot (`agents::own`): whatever it holds, `admin` included.
//!
//! **What a member holds** is worked out where a session's, a key's
//! maker's and an agent's credential's scopes are ([`Identity::known`],
//! [`Identity::holder`], and [`held_through`] inside a write), so a change
//! of a group reaches its members at their next request, and an agent's
//! credential is not replaced for it. A key somebody made holds what it
//! was given and is not widened by a group; where a person holds less by
//! a group changing (they left it, it holds less, it was taken away), the
//! keys they made are narrowed in the same write, since a key holds no
//! more than whoever made it, and whatever was told something because of
//! what they held asks again ([`Identity::withdrawn`]).
//!
//! **Held in memory** (a map of each group with its members), filled from
//! the rows as identity is opened: what the keeper of a topic's access
//! asks synchronously, and what a session is worked out from. Changes are
//! made one at a time, and the memory is put **before** the write at what
//! the group was and will be both (so that it says less than the rows for
//! somebody losing anything) and **after** it at what it will be: it says
//! less than the rows between the two, and never more.
//!
//! Each change is one write with what is said of it (`identity.group.*`:
//! the family `identity` is the program's own already, and a new family
//! would refuse every text somebody wrote that says or hears a topic
//! under it).

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use journal::{Journal, Writing, optional};
use kernel::principal::{covers, named};
use kernel::{Error, Principal, PrincipalKind, Result, Source};
use rusqlite::params;
use serde::Serialize;
use serde_json::json;

use crate::given::{actor_of, narrow_keys};
use crate::identity::{ACCOUNT, Identity, read_account};

pub(crate) const SCHEMA: &[&str] = &["
    CREATE TABLE groups (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        holds TEXT NOT NULL,
        created INTEGER NOT NULL
    );
    CREATE TABLE group_members (
        group_id TEXT NOT NULL,
        actor TEXT NOT NULL,
        added INTEGER NOT NULL,
        PRIMARY KEY (group_id, actor)
    ) WITHOUT ROWID;
    CREATE INDEX group_members_actor ON group_members (actor);
"];

/// The most letters a group's name has.
pub const LONGEST_NAME: usize = 60;

/// A group as it is handed out: its id, its name, what its members may
/// ask beside what each holds itself, and who they are, in order.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Group {
    pub id: String,
    pub name: String,
    pub holds: Vec<String>,
    pub members: Vec<String>,
    pub created: i64,
}

/// What a change of a group came to: the group as it is now (nothing
/// where it was taken away), whether anything changed, and how many keys
/// of the people who hold less by it were narrowed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Changed {
    pub group: Option<Group>,
    pub changed: bool,
    pub keys: usize,
}

/// Every group, held in memory, and the turn at changing one.
#[derive(Clone, Default)]
pub struct Groups {
    kept: Arc<Mutex<BTreeMap<String, Group>>>,
    turn: Arc<tokio::sync::Mutex<()>>,
}

impl Groups {
    pub(crate) async fn open(journal: &Journal) -> Result<Groups> {
        journal.db().migrate("groups", SCHEMA).await?;
        let rows: Vec<Group> = journal
            .db()
            .call(|connection| {
                let mut groups = connection.prepare("SELECT id, name, holds, created FROM groups")?;
                let mut all: Vec<Group> = groups
                    .query_map([], |row| {
                        Ok(Group {
                            id: row.get(0)?,
                            name: row.get(1)?,
                            holds: serde_json::from_str(&row.get::<_, String>(2)?).unwrap_or_default(),
                            members: Vec::new(),
                            created: row.get(3)?,
                        })
                    })?
                    .collect::<rusqlite::Result<_>>()?;
                let mut members =
                    connection.prepare("SELECT group_id, actor FROM group_members ORDER BY actor")?;
                let pairs: Vec<(String, String)> = members
                    .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
                    .collect::<rusqlite::Result<_>>()?;
                for (group, actor) in pairs {
                    if let Some(one) = all.iter_mut().find(|one| one.id == group) {
                        one.members.push(actor);
                    }
                }
                Ok(all)
            })
            .await?;
        let kept = rows.into_iter().map(|group| (group.id.clone(), group)).collect();
        Ok(Groups { kept: Arc::new(Mutex::new(kept)), turn: Arc::default() })
    }

    fn kept(&self) -> std::sync::MutexGuard<'_, BTreeMap<String, Group>> {
        self.kept.lock().expect("groups lock")
    }

    /// Every group, by name.
    pub fn all(&self) -> Vec<Group> {
        let mut all: Vec<Group> = self.kept().values().cloned().collect();
        all.sort_by_key(|group| group.name.to_lowercase());
        all
    }

    /// The group `word` names: by its id, else by its name whatever its
    /// capitals.
    pub fn found(&self, word: &str) -> Option<Group> {
        let kept = self.kept();
        kept.get(word).cloned().or_else(|| {
            let lower = word.trim().to_lowercase();
            kept.values().find(|group| group.name.to_lowercase() == lower).cloned()
        })
    }

    /// The groups `actor` is in.
    pub fn of(&self, actor: &str) -> Vec<Group> {
        self.all().into_iter().filter(|group| group.members.iter().any(|one| one == actor)).collect()
    }

    /// Who is in the group `id`: nobody, for one that is not there.
    pub fn members(&self, id: &str) -> Vec<String> {
        self.kept().get(id).map(|group| group.members.clone()).unwrap_or_default()
    }

    /// What `actor` holds by the groups it is in, each scope once.
    pub fn held_by(&self, actor: &str) -> Vec<String> {
        let mut held = Vec::new();
        for group in self.kept().values().filter(|group| group.members.iter().any(|one| one == actor)) {
            joined(&mut held, &group.holds);
        }
        held
    }

    /// Puts what is kept of a group at `to` (taken away for nothing).
    fn put(&self, id: &str, to: Option<Group>) {
        let mut kept = self.kept();
        match to {
            Some(group) => kept.insert(id.to_string(), group),
            None => kept.remove(id),
        };
    }

    /// Lets go of `actor` in every group, before the write that takes it
    /// out of each: what it was in, to be put back where that is not made.
    pub fn letting_go(&self, actor: &str) -> Vec<Group> {
        let mut kept = self.kept();
        let mut was = Vec::new();
        for group in kept.values_mut() {
            if group.members.iter().any(|one| one == actor) {
                was.push(group.clone());
                group.members.retain(|one| one != actor);
            }
        }
        was
    }

    /// Puts back what [`Groups::letting_go`] let go of, where the write
    /// was not made.
    pub fn stayed(&self, was: Vec<Group>) {
        for group in was {
            self.put(&group.id.clone(), Some(group));
        }
    }
}

/// Adds to `held` each of `more` that it does not cover already.
fn joined(held: &mut Vec<String>, more: &[String]) {
    for scope in more {
        if !covers(held, scope) {
            held.push(scope.clone());
        }
    }
}

/// What `actor` holds by the groups it is in, read where it is asked: in a
/// write that is under way, as that write finds the groups.
pub fn held_through(connection: &rusqlite::Connection, actor: &str) -> rusqlite::Result<Vec<String>> {
    let mut statement = connection.prepare(
        "SELECT g.holds FROM group_members m JOIN groups g ON g.id = m.group_id WHERE m.actor = ?1",
    )?;
    let lists: Vec<String> =
        statement.query_map([actor], |row| row.get(0))?.collect::<rusqlite::Result<_>>()?;
    let mut held = Vec::new();
    for list in lists {
        let list: Vec<String> = serde_json::from_str(&list).unwrap_or_default();
        joined(&mut held, &list);
    }
    Ok(held)
}

/// Takes `actor` out of every group it is in, in a write of somebody
/// else's that takes it away (an agent taken away: a later agent of the
/// name is no heir to what it was given), saying so of each.
pub fn forget_member(writing: &mut Writing<'_>, actor: &str, by: Source) -> rusqlite::Result<usize> {
    let groups: Vec<(String, String)> = {
        let mut statement = writing.transaction().prepare(
            "SELECT g.id, g.name FROM group_members m JOIN groups g ON g.id = m.group_id WHERE m.actor = ?1",
        )?;
        statement
            .query_map([actor], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<rusqlite::Result<_>>()?
    };
    writing.transaction().execute("DELETE FROM group_members WHERE actor = ?1", [actor])?;
    for (group, name) in &groups {
        let data = json!({ "group": group, "name": name, "member": actor, "keys": 0 });
        writing.say("identity.group.left", by.clone(), data)?;
    }
    Ok(groups.len())
}

/// The name `name` comes to as a group's, or why it is none.
fn name_of(name: &str) -> Result<String> {
    let name = name.trim();
    if name.is_empty() {
        return Err(Error::invalid("a group is called something: say what"));
    }
    if name.chars().any(char::is_control) {
        return Err(Error::invalid("a group's name is one line"));
    }
    let letters = name.chars().count();
    if letters > LONGEST_NAME {
        return Err(Error::invalid(format!(
            "a group's name is {LONGEST_NAME} letters at most, and this one is {letters}"
        )));
    }
    Ok(name.to_string())
}

/// What a group holds as it is kept: each a scope this factory checks,
/// once, in order.
fn holds_of(holds: &[String]) -> Result<Vec<String>> {
    let mut kept: Vec<String> = Vec::new();
    for scope in holds {
        let scope = scope.trim();
        if !named(scope) {
            return Err(Error::invalid(format!(
                "`{scope}` is not a scope this factory checks: a scope is an area, such as `board`, or one thing \
                 done in it, such as `board:read`"
            )));
        }
        if !kept.iter().any(|one| one == scope) {
            kept.push(scope.to_string());
        }
    }
    kept.sort();
    Ok(kept)
}

/// Refuses an agent changing a group it is in, or its own place in one.
fn not_its_own(by: &Principal, group: Option<&Group>, member: Option<&str>) -> Result<()> {
    if by.kind != PrincipalKind::Agent {
        return Ok(());
    }
    let actor = by.actor();
    if group.is_some_and(|group| group.members.contains(&actor)) || member == Some(actor.as_str()) {
        return Err(Error::denied(format!(
            "{} is an agent, and an agent never changes a group it is in, nor its own place in one, as it never \
             changes its own slot: a person who manages the factory does",
            by.name
        )));
    }
    Ok(())
}

/// Whether `by` holds `scope`: an area written whole is held where each
/// thing done in it is, as a table of areas writes one whose every part
/// is chosen.
fn holds(by: &Principal, scope: &str) -> bool {
    if by.can(scope) {
        return true;
    }
    let parts: Vec<&str> = kernel::principal::SCOPES
        .iter()
        .map(|(one, _)| *one)
        .filter(|one| one.split_once(':').is_some_and(|(area, _)| area == scope))
        .collect();
    !scope.contains(':') && !parts.is_empty() && parts.iter().all(|part| by.can(part))
}

/// Refuses giving what `by` does not hold.
fn gives(by: &Principal, scopes: &[String], what: &str) -> Result<()> {
    match scopes.iter().find(|scope| !holds(by, scope)) {
        Some(beyond) => Err(Error::denied(format!(
            "nobody gives what they do not hold, and {what} `{beyond}`, which you do not: it is done by somebody \
             who holds it"
        ))),
        None => Ok(()),
    }
}

fn no_group(word: &str) -> Error {
    Error::not_found(format!(
        "there is no group `{word}`: whoever manages the factory sees each in Settings, Groups"
    ))
}

/// What a group was and will be both: what the memory says while the
/// write that changes it is under way.
fn meet(old: &Option<Group>, new: &Option<Group>) -> Option<Group> {
    let (Some(old), Some(new)) = (old, new) else { return None };
    Some(Group {
        holds: old.holds.iter().filter(|scope| new.holds.contains(scope)).cloned().collect(),
        members: old.members.iter().filter(|one| new.members.contains(one)).cloned().collect(),
        ..new.clone()
    })
}

/// Narrows the keys of each person of `people` to what they hold now, as
/// this write finds their account and their groups: how many keys.
fn narrow_people(transaction: &rusqlite::Connection, people: &[String]) -> rusqlite::Result<usize> {
    let mut narrowed = 0;
    for actor in people {
        let Some(id) = actor.strip_prefix("user:") else { continue };
        let found = optional(transaction.query_row(
            &format!("SELECT {ACCOUNT} FROM accounts WHERE id = ?1"),
            [id],
            read_account,
        ))?;
        let Some(account) = found else { continue };
        let holds = account.principal_with(&held_through(transaction, actor)?).scopes;
        narrowed += narrow_keys(transaction, id, &holds)?;
    }
    Ok(narrowed)
}

impl Identity {
    /// Every group there is, by name.
    pub fn groups(&self) -> Vec<Group> {
        self.groups.all()
    }

    /// The group `word` names, by its id or its name.
    pub fn group(&self, word: &str) -> Result<Group> {
        self.groups.found(word).ok_or_else(|| no_group(word))
    }

    /// The groups kept in memory, for whoever asks synchronously.
    pub fn grouped(&self) -> &Groups {
        &self.groups
    }

    /// Makes a group called `name` holding `holds`, as `by`, who manages
    /// the factory and holds each of them.
    pub async fn make_group(&self, name: &str, holds: &[String], by: &Principal) -> Result<Group> {
        by.require("admin")?;
        let (name, holds) = (name_of(name)?, holds_of(holds)?);
        gives(by, &holds, "the group would hold")?;
        let _turn = self.groups.turn.lock().await;
        if self.groups.found(&name).is_some_and(|other| other.name.to_lowercase() == name.to_lowercase()) {
            return Err(Error::conflict(format!(
                "there is a group called `{name}` already: call this one something else"
            )));
        }
        let group = Group {
            id: self.journal.new_id("grp"),
            name,
            holds,
            members: Vec::new(),
            created: self.journal.now_ms(),
        };
        let (row, actor) = (group.clone(), actor_of(by));
        self.journal
            .write(move |writing| {
                writing.transaction().execute(
                    "INSERT INTO groups (id, name, holds, created) VALUES (?1, ?2, ?3, ?4)",
                    params![
                        row.id,
                        row.name,
                        serde_json::to_string(&row.holds).unwrap_or_default(),
                        row.created
                    ],
                )?;
                let data = json!({ "group": row.id, "name": row.name });
                writing.say("identity.group.made", Source::actor(actor.clone()), data)?;
                let data = json!({ "group": row.id, "name": row.name, "holds": row.holds, "keys": 0 });
                writing.say("identity.group.held", Source::actor(actor), data)?;
                Ok(())
            })
            .await?;
        self.groups.put(&group.id, Some(group.clone()));
        Ok(group)
    }

    /// Calls the group `word` names `name`.
    pub async fn rename_group(&self, word: &str, name: &str, by: &Principal) -> Result<Changed> {
        by.require("admin")?;
        let name = name_of(name)?;
        let _turn = self.groups.turn.lock().await;
        let old = self.group(word)?;
        not_its_own(by, Some(&old), None)?;
        if old.name == name {
            return Ok(Changed { group: Some(old), changed: false, keys: 0 });
        }
        if self
            .groups
            .found(&name)
            .is_some_and(|other| other.id != old.id && other.name.to_lowercase() == name.to_lowercase())
        {
            return Err(Error::conflict(format!(
                "there is a group called `{name}` already: call this one something else"
            )));
        }
        let new = Group { name: name.clone(), ..old.clone() };
        let (id, was, actor) = (old.id.clone(), old.name.clone(), actor_of(by));
        self.change(Some(old), Some(new), move |writing| {
            writing.transaction().execute("UPDATE groups SET name = ?2 WHERE id = ?1", params![id, name])?;
            let data = json!({ "group": id, "name": name, "was": was });
            writing.say("identity.group.renamed", Source::actor(actor), data)?;
            Ok(0)
        })
        .await
    }

    /// Has the group `word` names hold `holds` from now on, as `by`, who
    /// holds each that it did not hold before.
    pub async fn hold_group(&self, word: &str, holds: &[String], by: &Principal) -> Result<Changed> {
        by.require("admin")?;
        let holds = holds_of(holds)?;
        let _turn = self.groups.turn.lock().await;
        let old = self.group(word)?;
        not_its_own(by, Some(&old), None)?;
        if old.holds == holds {
            return Ok(Changed { group: Some(old), changed: false, keys: 0 });
        }
        let added: Vec<String> = holds.iter().filter(|scope| !covers(&old.holds, scope)).cloned().collect();
        gives(by, &added, "the group would hold")?;
        let narrows = old.holds.iter().any(|scope| !covers(&holds, scope));
        let new = Group { holds: holds.clone(), ..old.clone() };
        let (id, name, members, actor) =
            (old.id.clone(), old.name.clone(), old.members.clone(), actor_of(by));
        let changed = self
            .change(Some(old), Some(new), move |writing| {
                let kept = serde_json::to_string(&holds).unwrap_or_default();
                writing
                    .transaction()
                    .execute("UPDATE groups SET holds = ?2 WHERE id = ?1", params![id, kept])?;
                let keys = match narrows {
                    true => narrow_people(writing.transaction(), &members)?,
                    false => 0,
                };
                let data = json!({ "group": id, "name": name, "holds": holds, "keys": keys });
                writing.say("identity.group.held", Source::actor(actor), data)?;
                Ok(keys)
            })
            .await?;
        if narrows {
            self.say_withdrawn();
        }
        Ok(changed)
    }

    /// Puts `member` (`user:<id>` or `agent:<name>`) in the group `word`
    /// names, as `by`, who holds everything the group holds. Whether an
    /// agent of that name is there is the caller's to have asked: people
    /// keeps no list of agents.
    pub async fn join_group(&self, word: &str, member: &str, by: &Principal) -> Result<Changed> {
        by.require("admin")?;
        let _turn = self.groups.turn.lock().await;
        let old = self.group(word)?;
        not_its_own(by, Some(&old), Some(member))?;
        let account = match member.split_once(':') {
            Some(("user", id)) if !id.is_empty() => Some(id.to_string()),
            Some(("agent", name)) if !name.is_empty() && !name.contains(char::is_whitespace) => None,
            _ => {
                return Err(Error::invalid(format!(
                    "`{member}` is nobody a group holds: a member is a person (`user:<account>`) or an agent \
                     (`agent:<name>`), and a group holds no group"
                )));
            }
        };
        if old.members.iter().any(|one| one == member) {
            return Ok(Changed { group: Some(old), changed: false, keys: 0 });
        }
        gives(by, &old.holds, &format!("whoever is put in `{}` holds", old.name))?;
        let mut new = old.clone();
        new.members.push(member.to_string());
        new.members.sort();
        let (id, name, actor, now) = (old.id.clone(), old.name.clone(), actor_of(by), self.journal.now_ms());
        let member = member.to_string();
        let changed = self
            .change_or(Some(old), Some(new), move |writing| {
                if let Some(account) = &account {
                    let found = optional(writing.transaction().query_row(
                        "SELECT 1 FROM accounts WHERE id = ?1",
                        [account],
                        |row| row.get::<_, i64>(0),
                    ))?;
                    if found.is_none() {
                        return Ok(None);
                    }
                }
                writing.transaction().execute(
                    "INSERT INTO group_members (group_id, actor, added) VALUES (?1, ?2, ?3)",
                    params![id, member, now],
                )?;
                let data = json!({ "group": id, "name": name, "member": member });
                writing.say("identity.group.joined", Source::actor(actor), data)?;
                Ok(Some(0))
            })
            .await?;
        changed.ok_or_else(|| {
            Error::not_found(
                "there is no such account: whoever manages the factory sees each in Settings, People",
            )
        })
    }

    /// Takes `member` out of the group `word` names, and narrows the keys
    /// they made to what they hold without it.
    pub async fn leave_group(&self, word: &str, member: &str, by: &Principal) -> Result<Changed> {
        by.require("admin")?;
        let _turn = self.groups.turn.lock().await;
        let old = self.group(word)?;
        not_its_own(by, Some(&old), Some(member))?;
        if !old.members.iter().any(|one| one == member) {
            return Err(Error::conflict(format!("`{member}` is not in `{}`: nothing was changed", old.name)));
        }
        let mut new = old.clone();
        new.members.retain(|one| one != member);
        let (id, name, actor, member) = (old.id.clone(), old.name.clone(), actor_of(by), member.to_string());
        let changed = self
            .change(Some(old), Some(new), move |writing| {
                writing.transaction().execute(
                    "DELETE FROM group_members WHERE group_id = ?1 AND actor = ?2",
                    params![id, member],
                )?;
                let keys = narrow_people(writing.transaction(), std::slice::from_ref(&member))?;
                let data = json!({ "group": id, "name": name, "member": member, "keys": keys });
                writing.say("identity.group.left", Source::actor(actor), data)?;
                Ok(keys)
            })
            .await?;
        self.say_withdrawn();
        Ok(changed)
    }

    /// Takes the group `word` names away, with everybody's place in it,
    /// and narrows the keys its people made to what they hold without it.
    pub async fn remove_group(&self, word: &str, by: &Principal) -> Result<Changed> {
        by.require("admin")?;
        let _turn = self.groups.turn.lock().await;
        let old = self.group(word)?;
        not_its_own(by, Some(&old), None)?;
        let (id, name, members, actor) =
            (old.id.clone(), old.name.clone(), old.members.clone(), actor_of(by));
        let changed = self
            .change(Some(old), None, move |writing| {
                writing.transaction().execute("DELETE FROM group_members WHERE group_id = ?1", [&id])?;
                writing.transaction().execute("DELETE FROM groups WHERE id = ?1", [&id])?;
                let keys = narrow_people(writing.transaction(), &members)?;
                let data = json!({ "group": id, "name": name, "members": members, "keys": keys });
                writing.say("identity.group.removed", Source::actor(actor), data)?;
                Ok(keys)
            })
            .await?;
        self.say_withdrawn();
        Ok(changed)
    }

    /// Writes a change of a group from `old` to `new`, the memory put
    /// before and after it by the rule this module says.
    async fn change<F>(&self, old: Option<Group>, new: Option<Group>, write: F) -> Result<Changed>
    where
        F: FnOnce(&mut Writing<'_>) -> rusqlite::Result<usize> + Send + 'static,
    {
        let changed = self.change_or(old, new, move |writing| write(writing).map(Some)).await?;
        changed.ok_or_else(|| Error::internal("a change of a group came to nothing"))
    }

    /// The same, for a write that may find it is not to be made, and
    /// writes and says nothing then: nothing changed.
    async fn change_or<F>(&self, old: Option<Group>, new: Option<Group>, write: F) -> Result<Option<Changed>>
    where
        F: FnOnce(&mut Writing<'_>) -> rusqlite::Result<Option<usize>> + Send + 'static,
    {
        let id = old.as_ref().or(new.as_ref()).map(|group| group.id.clone()).unwrap_or_default();
        self.groups.put(&id, meet(&old, &new));
        let wrote = self.journal.write(write).await;
        match wrote {
            Ok((Some(keys), _)) => {
                self.groups.put(&id, new.clone());
                Ok(Some(Changed { group: new, changed: true, keys }))
            }
            Ok((None, _)) => {
                self.groups.put(&id, old);
                Ok(None)
            }
            Err(error) => {
                self.groups.put(&id, old);
                Err(error)
            }
        }
    }
}

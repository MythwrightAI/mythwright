//! Who changes a channel's details: what it is called, what it says it
//! is for, and who else manages it.
//!
//! **The rule** ([`manages_as`], held case for case with the client's to
//! one table, `ui/src/managedcases.json`): whoever may write in channels
//! (`channels:write`) and either manages the factory (`admin`) or is a
//! manager of the channel. A channel's managers are people in it: whoever
//! made it, from the write that makes it, and whoever a manager names one
//! since ([`Channels::manage`]). Every person in a conversation manages
//! it, and goes on doing so once it is opened as a channel. An agent and
//! a bot manage nothing, and a key acts as its maker held to what it was
//! given. Of a channel made before channels had managers nobody is one,
//! since who made it was never kept: whoever manages the factory may
//! change it, and name somebody in it a manager.
//!
//! Being a manager lets nobody read or write anything they could not:
//! it is asked beside the channel's own door ([`Channels::writable`]),
//! which still says whether whoever asks may write there at all, and of a
//! kept channel whether they are in it. It is kept with who is in the
//! channel (`channel_members.manages`), so somebody who leaves manages it
//! no longer, and what is said of it (`channel.managers`) is said as
//! anything of the channel is: of a kept one, to its members alone.
//!
//! **The last manager stays**: somebody is made no longer a manager only
//! where another is one. Leaving is not held by it: a channel its last
//! manager left is changed by whoever manages the factory, as one from
//! before managers is.
//!
//! **What it says it is for** (`topic`, shown under its head) is one
//! line, [`MAX_TOPIC`] letters at most, and may be nothing; changed, the
//! row and `channel.described` are one write ([`Channels::describe`]).
//! An archived channel is given back before either is changed, as it is
//! before it is renamed.

use kernel::{Error, Principal, Result, Source, principal::covers};
use serde_json::json;

use crate::channels::{Channel, Channels, kind_of};
use crate::door::{as_it_is, called};
use crate::kept::{Reader, WRITE};

/// The most letters what a channel says it is for may be.
pub const MAX_TOPIC: usize = 250;

/// What managing the factory takes, which manages every channel too.
pub const ADMIN: &str = "admin";

/// Whether the actor `actor`, holding `held`, manages `channel`: the one
/// rule, of plain values.
pub fn manages_as(channel: &Channel, actor: &str, held: &[String]) -> bool {
    let person = kind_of(actor) == "user";
    let named = channel
        .members
        .iter()
        .any(|member| member.actor == actor && (member.manages || channel.conversation));
    covers(held, WRITE) && (covers(held, ADMIN) || (person && named))
}

/// Whether `by` manages `channel`.
pub fn manages(channel: &Channel, by: &Principal) -> bool {
    manages_as(channel, &by.actor(), &by.scopes)
}

/// The people who manage `channel`, by name, in the order they came in.
pub fn managers(channel: &Channel) -> Vec<&str> {
    channel
        .members
        .iter()
        .filter(|member| kind_of(&member.actor) == "user" && (member.manages || channel.conversation))
        .map(|member| member.name.as_str())
        .collect()
}

/// Refuses `by` where they do not manage `channel`, in a sentence that
/// says who does. The scope of writing is asked first, in the words a
/// scope is always refused in.
pub fn may_manage(channel: &Channel, by: &Principal) -> Result<()> {
    if manages(channel, by) {
        return Ok(());
    }
    by.require(WRITE)?;
    let who = managers(channel);
    let they = match who.as_slice() {
        [] => format!(
            "{} has no managers (it was made before channels had any, or they left it)",
            called(channel)
        ),
        [one] => format!("{} is managed by {one}", called(channel)),
        [rest @ .., last] => format!("{} is managed by {} and {last}", called(channel), rest.join(", ")),
    };
    Err(Error::denied(format!(
        "{} is not one of its managers: {they}, who change what it is called, what it says it is for and who \
         else manages it, as whoever manages the factory does (needs `{ADMIN}`)",
        by.name
    )))
}

/// What `channel` is to say it is for once it is told `to`: nothing where
/// it says that already. Refused where it is more than a line.
pub fn describing(channel: &Channel, to: &str) -> Result<Option<String>> {
    let topic = to.trim();
    if topic.contains('\n') || topic.chars().count() > MAX_TOPIC {
        return Err(Error::invalid(format!(
            "what a channel says it is for is one line, in {MAX_TOPIC} letters at most"
        )));
    }
    Ok((topic != channel.topic.trim()).then(|| topic.to_string()))
}

/// What changing the archived `channel`'s details is refused with.
fn unarchive_first(channel: &Channel) -> Error {
    Error::conflict(format!(
        "{} is archived, and nothing of an archived channel is changed: whoever may write in it gives it back \
         first (`{} channel unarchive {}`)",
        called(channel),
        brand::CLI,
        channel.name
    ))
}

/// What is said of `now` beside what is said of every change of it here.
fn said_of(now: &Channel, by: &Principal, mut data: serde_json::Value) -> serde_json::Value {
    data["channel"] = json!(now.id);
    data["name"] = json!(now.name);
    data["by"] = json!(by.actor());
    data["by_name"] = json!(by.name);
    for (key, said) in [("title", now.title.as_deref()), ("project", now.project.as_deref())] {
        if let Some(said) = said {
            data[key] = json!(said);
        }
    }
    data
}

impl Channels {
    /// Has the channel `reader` is leave to write in say it is for `to`,
    /// as `by`, who manages it. One that says so already is as it was,
    /// and nothing is said.
    pub async fn describe(
        &self,
        reader: &Reader,
        channel: &str,
        to: &str,
        by: &Principal,
    ) -> Result<Channel> {
        reader.writing()?;
        // Taken as a channel is renamed, kept or given managers: its
        // details change one at a time.
        let _turn = self.memory.changing().await;
        let found = self.get(reader, channel).await?;
        may_manage(&found, by)?;
        if found.archived.is_some() {
            return Err(unarchive_first(&found));
        }
        let Some(topic) = describing(&found, to)? else { return Ok(found) };
        let (row, by_now, was) = (found.clone(), by.clone(), found.topic.clone());
        let wrote = self.journal.write(move |writing| {
            let described = writing.transaction().execute(
                "UPDATE channels SET topic = ?2 WHERE id = ?1 AND archived IS NULL",
                rusqlite::params![row.id, topic],
            )?;
            let mut now = as_it_is(writing.transaction(), row)?;
            if described == 1 {
                now.topic = topic.clone();
                let source = Source::actor(by_now.actor()).taking(now.takes(&[]).as_deref());
                let data = said_of(&now, &by_now, json!({ "topic": topic, "was": was }));
                writing.say_here("channel.described", source, data)?;
            }
            Ok((described == 1, now))
        });
        let ((described, now), _) = wrote.await?;
        if !described {
            return Err(unarchive_first(&now));
        }
        Ok(now)
    }

    /// Makes `member`, a person in the channel `reader` is leave to write
    /// in, one of its managers or no longer one, as `by`, who manages it.
    /// The last manager is not taken away; somebody who is so already is
    /// as they were, and nothing is said.
    pub async fn manage(
        &self,
        reader: &Reader,
        channel: &str,
        member: &str,
        manages: bool,
        by: &Principal,
    ) -> Result<Channel> {
        reader.writing()?;
        let _turn = self.memory.changing().await;
        let found = self.get(reader, channel).await?;
        may_manage(&found, by)?;
        if found.archived.is_some() {
            return Err(unarchive_first(&found));
        }
        if found.conversation {
            return Err(Error::invalid(format!(
                "every person in {} manages it while it is a conversation: open it as a channel first (`{} \
                 channel from {}`)",
                called(&found),
                brand::CLI,
                found.id
            )));
        }
        let Some(one) = found.members.iter().find(|one| one.actor == member) else {
            return Err(Error::invalid(format!(
                "`{member}` is not in {}: a channel is managed by people who are in it, so add them first",
                called(&found)
            )));
        };
        if kind_of(member) != "user" {
            return Err(Error::invalid(format!(
                "{} is not a person, and only a person manages a channel",
                one.name
            )));
        }
        if one.manages == manages {
            return Ok(found);
        }
        if !manages && managers(&found).len() == 1 {
            return Err(Error::conflict(format!(
                "{} is the only manager of {}, and a channel's last manager stays: make somebody else in it one \
                 first",
                one.name,
                called(&found)
            )));
        }
        let (row, by_now, member) = (found.clone(), by.clone(), member.to_string());
        let wrote = self.journal.write(move |writing| {
            let changed = writing.transaction().execute(
                "UPDATE channel_members SET manages = ?3 WHERE channel = ?1 AND member = ?2",
                rusqlite::params![row.id, member, manages],
            )?;
            let now = as_it_is(writing.transaction(), row)?;
            if changed == 1 {
                let mut managing = writing.transaction().prepare(
                    "SELECT member FROM channel_members WHERE channel = ?1 AND manages = 1 ORDER BY rowid",
                )?;
                let all: Vec<String> =
                    managing.query_map([&now.id], |found| found.get(0))?.collect::<rusqlite::Result<_>>()?;
                let source = Source::actor(by_now.actor()).taking(now.takes(&[]).as_deref());
                let data =
                    said_of(&now, &by_now, json!({ "member": member, "manages": manages, "managers": all }));
                writing.say_here("channel.managers", source, data)?;
            }
            Ok(())
        });
        wrote.await?;
        self.get(reader, &found.id).await
    }
}

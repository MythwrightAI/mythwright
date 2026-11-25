//! A channel called something else.
//!
//! **What changes.** An open channel is known among the names channels
//! share, so what it is called is its `name`: renamed, its name is the
//! new one, held to what every name is (lowercase words joined by
//! dashes, at least one letter or digit) and refused where another
//! channel has it ([`crate::channels::taken`]); the name it had is free
//! from then. A channel kept to its members, and a conversation, is
//! called its `title`, which another may be called too: renamed, its
//! title is the new one, and its name, which is its own id, does not
//! change. Its id never does, and everything that keeps something of a
//! channel by its id (who is in it, what was said there, a group, a pin,
//! a line of somebody's inbox, a workspace's own channels, an agent's
//! conversation held there) goes on as it was. What was written as
//! `#name` in what was said stays as it was written.
//!
//! **One write.** The row and what is said of it (`channel.renamed`, with
//! what it was called and what it is called now) are one write, said as
//! anything of the channel is: to whoever may read channels, and of one
//! kept to its members to its members alone. It is asked as the write
//! finds the channel: one that was archived, kept or opened again since
//! it was read is not renamed by what was asked of it as it was.
//!
//! **Who may, and when.** Whoever manages it ([`crate::managed`]: one
//! of its managers, or whoever manages the factory), asked beside its
//! door ([`Channels::may_write`]: for an open one whoever may write in
//! channels, and for one kept to its members one of them, or the
//! factory's owner). It changes nothing of who reads it, as archiving
//! changes nothing. Until the card that gave channels managers (401),
//! anybody who might say something there might call it something else,
//! as Slack had it; it is now held as a channel's details are. An archived
//! channel is not renamed: it is given back first ([`unarchive_first`]).
//! What is refused above this crate, which knows what runs: something
//! running that names the channel by what it is called (a blueprint
//! finds its channel by its name, and would make another of the old
//! name), which the route asks; and an agent being asked something
//! there, whose answer is said where it was asked, by whoever keeps
//! agents (`Agents::rename_channel`).

use kernel::{Error, Principal, Result, Source};
use serde_json::json;

use crate::channels::{Channel, Channels, MAX_TITLE, taken};
use crate::door::{as_it_is, called};
use crate::kept::Reader;

/// What `channel` is to be called once it is renamed to `to`: its new
/// name where it is open, its new title where it is kept to its members;
/// nothing where that is what it is called already. Refused where what
/// was asked comes to no name.
pub fn renaming(channel: &Channel, to: &str) -> Result<Option<String>> {
    if channel.kept {
        let title = to.trim();
        if title.is_empty() || title.chars().count() > MAX_TITLE || title.contains('\n') {
            return Err(Error::invalid(format!(
                "a channel is called something, on one line and in {MAX_TITLE} letters at most"
            )));
        }
        return Ok((channel.title.as_deref() != Some(title)).then(|| title.to_string()));
    }
    let name = blueprint::expr::slug(to);
    if name.is_empty() {
        return Err(Error::invalid("a channel needs a name with at least one letter or digit"));
    }
    Ok((name != channel.name).then_some(name))
}

/// What renaming the archived `channel` is refused with.
pub fn unarchive_first(channel: &Channel) -> Error {
    Error::conflict(format!(
        "{} is archived, and an archived channel is not called something else: whoever may write in it gives it \
         back first (`{} channel unarchive {}`)",
        called(channel),
        brand::CLI,
        channel.name
    ))
}

/// What is said of a channel renamed: what it is called now and what it
/// was called, as an open one is named and a kept one titled.
fn said_of(now: &Channel, was: &str, by: &Principal) -> serde_json::Value {
    let mut data = json!({
        "channel": now.id, "name": now.name, "was": was, "kept": now.kept, "by": by.actor(), "by_name": by.name,
    });
    for (key, said) in [("title", now.title.as_deref()), ("project", now.project.as_deref())] {
        if let Some(said) = said {
            data[key] = json!(said);
        }
    }
    data
}

impl Channels {
    /// Calls the channel `reader` is leave to write in `to`, as `by`: an
    /// open one by its name, a kept one by its title. One that is called
    /// so already is as it was, and nothing is said.
    pub async fn rename(&self, reader: &Reader, channel: &str, to: &str, by: &Principal) -> Result<Channel> {
        reader.writing()?;
        // Taken as a channel is kept or opened again: what it is called
        // is changed by one of them at a time.
        let _turn = self.memory.changing().await;
        let found = self.get(reader, channel).await?;
        crate::managed::may_manage(&found, by)?;
        if found.archived.is_some() {
            return Err(unarchive_first(&found));
        }
        let Some(called_now) = renaming(&found, to)? else { return Ok(found) };
        let was = match found.kept {
            true => found.title.clone().unwrap_or_default(),
            false => found.name.clone(),
        };
        let (row, by_now, asked) = (found.clone(), by.clone(), called_now.clone());
        let wrote = self.journal.write(move |writing| {
            let renamed = writing.transaction().execute(
                match row.kept {
                    true => "UPDATE channels SET title = ?2 WHERE id = ?1 AND kept = 1 AND archived IS NULL",
                    false => "UPDATE channels SET name = ?2 WHERE id = ?1 AND kept = 0 AND archived IS NULL",
                },
                rusqlite::params![row.id, asked],
            )?;
            // As the write finds it: archived, kept or opened again since
            // it was read, it is not renamed by what was asked of it then.
            let now = as_it_is(writing.transaction(), row)?;
            if renamed == 1 {
                let source = Source::actor(by_now.actor()).taking(now.takes(&[]).as_deref());
                writing.say_here("channel.renamed", source, said_of(&now, &was, &by_now))?;
            }
            Ok((renamed == 1, now))
        });
        let ((renamed, now), _) = wrote.await.map_err(|refused| taken(refused, &called_now))?;
        if !renamed {
            return Err(match now.archived {
                Some(_) => unarchive_first(&now),
                None => Error::conflict(format!(
                    "{} was made private or opened again as this was asked: look at it again, and say what it \
                     is to be called",
                    called(&now)
                )),
            });
        }
        // An archived one is known by its name: this one is not, and is so once more.
        self.memory.archived_as(&now);
        Ok(now)
    }
}

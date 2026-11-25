//! A conversation: a message written to one or several people and
//! agents, in each one's inbox.
//!
//! **It is a channel underneath, kept to its members** ([`crate::kept`]),
//! that says it is a conversation (`channels.conversation`): its subject
//! is what it is called, whoever wrote it and whoever it is to are its
//! members, and everything a kept channel has it has, by being one
//! (threads, what is attached, who reads it, an agent's turn and its
//! answer said where it was asked). What is its own is where it is
//! listed: in the inbox of each of its members, as one line that each
//! thing somebody else says there raises ([`crate::inboxed`]), and not
//! among the channels, which is whoever lists channels' to leave out.
//!
//! **Written as one write** ([`Channels::converse`]): the channel, who
//! is in it, the first thing said there and the line that puts in the
//! box of everybody it is to, with what is said of each, or none of it.
//! So nothing that fails leaves a conversation with nothing said in it,
//! and nobody is in one they were never told of. The memory that answers
//! who is told an event has its members from before that write, since
//! its first message is said in it: until the write nothing names the
//! channel, and where the write is not made the memory forgets it.
//!
//! **An agent** is in one as it is in any kept channel: whoever keeps
//! agents says first that it may be ([`Conversing::agents`]), and it is
//! brought into no conversation while a channel holds it to itself. The
//! same people written to again with the same subject are another
//! conversation, as with mail: nothing here looks for one that is there.
//!
//! **Opened as a channel** ([`Channels::open_as_channel`]), by one of
//! its members: the mark comes off, in one write with what is said of
//! it, and it is from then a channel kept to its members like any, with
//! everything said so far, listed among the channels for whoever is in
//! it. The lines it put in anybody's box stay as they are and open it
//! still; what is said there from then on raises none. It is not
//! undone: a channel is not made a conversation.

use kernel::{Error, Principal, Result, Source};

use crate::channels::{Channel, Channels, MAX_TITLE, Member, Message, kind_of};
use crate::door::{called, kept_of};
use crate::kept::Reader;

/// As many letters of what was first said as a conversation with no
/// subject is called by.
const FIRST_WORDS: usize = 60;

/// A conversation as whoever writes one asks for it.
#[derive(Clone, Copy, Debug, Default)]
pub struct Conversing<'a> {
    /// What it is about. Left empty, it is called by its first words.
    pub subject: &'a str,
    /// The first thing said in it.
    pub text: &'a str,
    /// The people it is to.
    pub people: &'a [Member],
    /// The agents it is to, each of which whoever keeps agents has said
    /// may be in a channel kept to its members.
    pub agents: &'a [Member],
}

/// What a conversation whose subject was left empty is called: the
/// first words of what was said in it, as far as a line of an inbox
/// has room for.
pub fn first_words(text: &str) -> String {
    let line: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if line.chars().count() <= FIRST_WORDS {
        return line;
    }
    let cut: String = line.chars().take(FIRST_WORDS).collect();
    // At the end of a word, where there is one to end at.
    let whole = cut.rfind(' ').filter(|at| *at > FIRST_WORDS / 2).map_or(cut.as_str(), |at| &cut[..at]);
    format!("{}…", whole.trim_end())
}

/// Whether `by` may open the conversation `channel` as a channel:
/// somebody who is in it.
pub fn may_open_as_channel(channel: &Channel, by: &Principal) -> Result<()> {
    if channel.has(&by.actor()) {
        return Ok(());
    }
    Err(Error::denied(format!(
        "{} is not in the conversation {}, and one is opened as a channel by somebody who is in it",
        by.name,
        called(channel)
    )))
}

impl Channels {
    /// The channels a list of a workspace's channels by id leaves out
    /// for `by`: those kept to their members that `by` does not read
    /// ([`Channels::kept_from`]), and every conversation, which is
    /// listed in its members' inboxes and nowhere else. Never handed to
    /// anybody itself.
    pub async fn unlisted_for(&self, by: &Principal) -> Result<std::collections::HashSet<String>> {
        let rows = self.rows().await?.into_iter().filter(|channel| channel.kept);
        let out = |channel: &Channel| channel.conversation || !channel.is_there_for(by, &self.memory);
        Ok(rows.filter(out).map(|one| one.id).collect())
    }

    /// Writes a conversation as `by`: a channel kept to its members that
    /// says it is one, with `by` and everybody it is to in it and its
    /// first message said, each of them told in their box, as one write.
    /// The conversation and the message.
    pub async fn converse(&self, asked: Conversing<'_>, by: &Principal) -> Result<(Channel, Message)> {
        by.require(crate::inbox::WRITE)?;
        by.require(crate::kept::WRITE)?;
        let body = Channels::may_say(asked.text, &[])?;
        let subject = match asked.subject.trim() {
            "" => first_words(body),
            said => said.to_string(),
        };
        if subject.is_empty() || subject.chars().count() > MAX_TITLE {
            return Err(Error::invalid(format!(
                "a conversation is about something, said in {MAX_TITLE} letters at most"
            )));
        }
        let maker = Member::new(by.actor(), by.name.clone());
        let mut members = vec![maker.clone()];
        let brought = |member: &Member| kind_of(&member.actor) == "agent";
        for (to, agents) in [(asked.people, false), (asked.agents, true)] {
            for one in to {
                if !crate::inbox::owns(&one.actor) || brought(one) != agents {
                    return Err(Error::invalid(format!(
                        "`{}` is nobody a conversation is written to: it is to people (`user:<account>`) and to \
                         agents (`agent:<name>`), which whoever keeps agents says may be in one",
                        one.actor
                    )));
                }
                if !members.iter().any(|member| member.actor == one.actor) {
                    members.push(one.clone());
                }
            }
        }
        if members.len() < 2 {
            return Err(Error::invalid(
                "a conversation is written to somebody: say who it is to, a person or an agent other than \
                 whoever writes it",
            ));
        }
        let id = self.journal.new_id("ch");
        let channel = Channel {
            // What it is called is not among the names channels share.
            name: id.clone(),
            id,
            project: None,
            topic: String::new(),
            created: self.journal.now_ms(),
            members,
            kept: true,
            title: Some(subject),
            alone: false,
            conversation: true,
            archived: None,
        };
        let message = Message {
            id: self.journal.new_id("msg"),
            channel: channel.id.clone(),
            author: maker.actor.clone(),
            author_name: maker.name.clone(),
            body: body.to_string(),
            reply_to: None,
            model: None,
            thread: None,
            replies: 0,
            attachments: Vec::new(),
            said_by: None,
            mark: None,
            created: channel.created,
        };
        let _turn = self.memory.changing().await;
        // Under the turn: no agent is brought in that a channel holds
        // to itself.
        for agent in channel.members.iter().filter(|member| brought(member)) {
            if let Some(why) = self.memory.not_brought(&channel.id, false, &agent.actor) {
                return Err(Error::conflict(why));
            }
        }
        // Before the write: its first message is said in it, to whoever
        // is in it as that is handed over and to nobody by name, so that
        // somebody who leaves does not go on reading it; and from the
        // write on what is said of it finds its members here.
        let within: Vec<String> = channel.members.iter().map(|member| member.actor.clone()).collect();
        self.memory.keep(&channel.id, within, false);
        let made = self.making(&channel, &maker);
        let said = self.saying(channel.clone(), message, Vec::new(), false);
        let wrote = self.journal.write(move |writing| {
            made(writing)?;
            said(writing)
        });
        match wrote.await {
            Ok((Ok(message), _)) => {
                // One that names the factory's bot is said to it.
                self.speaker.heard(by, &message);
                Ok((channel, message))
            }
            Ok((Err(refused), _)) => {
                self.memory.forget(&channel.id);
                Err(refused)
            }
            Err(refused) => {
                // Nothing was written: there is no such channel.
                self.memory.forget(&channel.id);
                Err(refused)
            }
        }
    }

    /// Opens the conversation `reader` is leave to write in as a channel,
    /// as `by`, who is in it: called `title`, or what its subject was,
    /// it is from here a channel kept to its members like any, with
    /// everything said in it so far. One that is no conversation is as
    /// it was.
    pub async fn open_as_channel(
        &self,
        reader: &Reader,
        channel: &str,
        title: Option<&str>,
        by: &Principal,
    ) -> Result<Channel> {
        reader.writing()?;
        let _turn = self.memory.changing().await;
        let mut channel = self.get(reader, channel).await?;
        if !channel.conversation {
            return Ok(channel);
        }
        may_open_as_channel(&channel, by)?;
        let was = channel.title.clone().unwrap_or_default();
        let title = title.map(str::trim).filter(|title| !title.is_empty()).unwrap_or(&was).to_string();
        if title.is_empty() || title.chars().count() > MAX_TITLE {
            return Err(Error::invalid(format!(
                "a channel is called something, in {MAX_TITLE} letters at most"
            )));
        }
        (channel.conversation, channel.title) = (false, Some(title));
        for member in &mut channel.members {
            member.manages |= crate::channels::kind_of(&member.actor) == "user";
        }
        let (row, actor) = (channel.clone(), by.actor());
        let wrote = self.journal.write(move |writing| {
            let opened = writing.transaction().execute(
                "UPDATE channels SET conversation = 0, title = ?2 WHERE id = ?1 AND conversation = 1",
                rusqlite::params![row.id, row.title],
            )?;
            if opened == 1 {
                // A conversation is managed by every person in it
                // ([`crate::managed`]): opened as a channel, each goes on
                // managing it, in the write that opens it.
                writing.transaction().execute(
                    "UPDATE channel_members SET manages = 1 WHERE channel = ?1 AND member LIKE 'user:%'",
                    [&row.id],
                )?;
                // Said to its members: it is theirs as it was, and is
                // listed among their channels from here.
                let mut data = kept_of(&row, (was != row.title.clone().unwrap_or_default()).then_some(&was));
                data["conversation"] = serde_json::json!(false);
                let source = Source::actor(&actor).taking(row.takes(&[]).as_deref());
                writing.say_here("channel.kept", source, data)?;
            }
            Ok(())
        });
        wrote.await.map(|_| channel)
    }
}

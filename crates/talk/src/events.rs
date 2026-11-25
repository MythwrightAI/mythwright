//! The events said of channels: one made or taken away, kept to its
//! members or opened again, archived or given back, called something
//! else, what it says it is for and who manages it, what documents it keeps to hand, something attached there made a
//! file of a project, who joined and left, and what was said; and of somebody's inbox:
//! a line told, read and put away, and what somebody follows.
//!
//! That a channel was said to belong to something else (`channel.placed`)
//! is not here: whatever groups channels says it (`studio::workspaces`),
//! and describes it.

use blueprint::{EventSpec, ValueType::*};

pub fn events() -> Vec<EventSpec> {
    let all = vec![
        EventSpec::new("channel.created", "A channel was made, or given back from the Trash.")
            .field("channel", String, "the channel's id")
            .field("name", String, "what it is called")
            .maybe("project", String, "the project it belongs to")
            .maybe("restored", Bool, "that it was given back from the Trash, with everything said there"),
        EventSpec::new(
            "channel.speaker_held",
            "What the factory's bot may ask was narrowed, or given back to everything its commands reach.",
        )
        .field("name", String, "what the bot is called")
        .maybe("holds", List, "the most it may ask, as scopes; nothing for everything its commands reach"),
        EventSpec::new("channel.removed", "A channel was taken away, with what was said in it.")
            .field("channel", String, "the channel's id")
            .field("name", String, "what it was called")
            .maybe("project", String, "the project it belonged to")
            .field("messages", Int, "how many messages went with it")
            .field("attachments", Int, "how many things attached to them went with it")
            .maybe("trash", String, "where it went into the Trash, what it is called there"),
        EventSpec::new(
            "channel.kept",
            "A channel was kept to its members or opened again, says another thing of whether its agents are \
             asked nowhere else, or was a conversation and is opened as a channel.",
        )
        .field("channel", String, "the channel's id")
        .field("name", String, "its name now: its own id while it is kept")
        .field("kept", Bool, "whether it is kept to its members now")
        .field("alone", Bool, "whether its agents are asked nowhere else")
        .maybe("title", String, "what it is called while it is kept")
        .maybe("was", String, "the name it had until this kept it, or what a conversation was about")
        .maybe("conversation", Bool, "not so, where it was a conversation and is opened as a channel")
        .maybe("project", String, "the project it belongs to"),
        EventSpec::new(
            "channel.archived",
            "A channel was archived: read by whoever read it, and written in by nobody until it is given back.",
        )
        .field("channel", String, "the channel's id")
        .field("name", String, "its name: for one kept to its members, its own id")
        .field("by", String, "who archived it, as an actor")
        .field("by_name", String, "who archived it, by name")
        .field("at", Int, "when, in milliseconds")
        .maybe("title", String, "what it is called, where it is kept to its members")
        .maybe("project", String, "the project it belongs to"),
        EventSpec::new("channel.unarchived", "An archived channel was given back, as it was.")
            .field("channel", String, "the channel's id")
            .field("name", String, "its name: for one kept to its members, its own id")
            .field("by", String, "who gave it back, as an actor")
            .maybe("title", String, "what it is called, where it is kept to its members")
            .maybe("project", String, "the project it belongs to"),
        EventSpec::new(
            "channel.renamed",
            "A channel was called something else: an open one by its name, the one it had free from then, and one \
             kept to its members by its title. Its id is as it was.",
        )
        .field("channel", String, "the channel's id")
        .field("name", String, "its name now: for one kept to its members, its own id, which is as it was")
        .field("was", String, "what it was called: its name, or of one kept to its members its title")
        .field("kept", Bool, "whether it is kept to its members, which is what says whether it is its title")
        .field("by", String, "who renamed it, as an actor")
        .field("by_name", String, "who renamed it, by name")
        .maybe("title", String, "what it is called now, where it is kept to its members")
        .maybe("project", String, "the project it belongs to"),
        EventSpec::new(
            "channel.described",
            "What a channel says it is for, shown under its head, was changed by one of its managers.",
        )
        .field("channel", String, "the channel's id")
        .field("name", String, "its name: for one kept to its members, its own id")
        .field("topic", String, "what it says it is for now: one line, or nothing")
        .field("was", String, "what it said before")
        .field("by", String, "who changed it, as an actor")
        .field("by_name", String, "who changed it, by name")
        .maybe("title", String, "what it is called, where it is kept to its members")
        .maybe("project", String, "the project it belongs to"),
        EventSpec::new(
            "channel.managers",
            "Somebody in a channel was made one of its managers, who change what it is called and says it is for, \
             or was made no longer one.",
        )
        .field("channel", String, "the channel's id")
        .field("name", String, "its name: for one kept to its members, its own id")
        .field("member", String, "who was made a manager or no longer one, as an actor")
        .field("manages", Bool, "whether they manage it now")
        .field("managers", List, "everybody who manages it now, as actors")
        .field("by", String, "who did it, as an actor")
        .field("by_name", String, "who did it, by name")
        .maybe("title", String, "what it is called, where it is kept to its members")
        .maybe("project", String, "the project it belongs to"),
        EventSpec::new(
            "channel.docs",
            "A document was added to what a channel keeps to hand, or taken from it: a library's record or a \
             project's file, the same for everybody in the channel.",
        )
        .field("channel", String, "the channel's id")
        .field("name", String, "its name: for one kept to its members, its own id")
        .field("doc", String, "the document: a library's record by its id, or a project's file as `<project>/<path>`")
        .field("did", String, "`added` or `removed`")
        .field("by", String, "who did it, as an actor")
        .field("by_name", String, "who did it, by name")
        .maybe("title", String, "what the channel is called, where it is kept to its members")
        .maybe("project", String, "the project the channel belongs to"),
        EventSpec::new(
            "channel.attachment.placed",
            "Something attached to a message as a copy was written among a project's files, and the message names \
             that file from then on.",
        )
        .field("channel", String, "the channel's id")
        .field("name", String, "its name: for one kept to its members, its own id")
        .field("message", String, "the message that carries it")
        .field("attachment", String, "what is attached")
        .field("project", String, "the project whose file it is now")
        .field("path", String, "its file, from the top of the project")
        .field("file_name", String, "what the file is called")
        .field("by", String, "who did it, as an actor")
        .field("by_name", String, "who did it, by name")
        .maybe("title", String, "what the channel is called, where it is kept to its members"),
        EventSpec::new("channel.joined", "Someone or something joined a channel.")
            .field("channel", String, "the channel's id")
            .field("name", String, "what the channel is called")
            .field("member", String, "who joined, as an actor")
            .field("member_name", String, "who joined, by name")
            .maybe("project", String, "the project the channel belongs to, where it belongs to one"),
        EventSpec::new("channel.left", "A member left a channel.")
            .field("channel", String, "the channel's id")
            .field("member", String, "who left, as an actor")
            .maybe("project", String, "the project the channel belongs to, where it belongs to one"),
        EventSpec::new("channel.message", "Something was said in a channel.")
            .field("id", String, "the message")
            .field("channel", String, "the channel's id")
            .field("channel_name", String, "what the channel is called")
            .maybe("project", String, "the project the channel belongs to")
            .field("author", String, "who said it, as an actor")
            .field("author_kind", String, "what kind of actor that is: `user`, `agent`, `bot` or `system`")
            .field("author_name", String, "who said it, by name")
            .field("body", String, "what was said")
            .maybe("reply_to", String, "the message it answers")
            .maybe("thread", String, "the message whose thread it is in")
            .maybe("model", String, "the model that wrote it, for an agent's")
            .maybe("attachments", List, "the files that came with it")
            .field("mentions", List, "who it names with `@`")
            .maybe("mark", String, "what its author marked it as: `plan`, or `discuss` for its agents to talk over")
            .maybe(
                "said_by",
                Object,
                "for one a blueprint's step said: its `blueprint`, `instance`, `run` and `step`, and any `detail`",
            ),
        EventSpec::new(
            "inbox.told",
            "Something was put in somebody's inbox, or a line there was told again: heard by its owner alone.",
        )
        .field("owner", String, "whose inbox, as an actor")
        .field("line", String, "the line")
        .field("kind", String, "what kind of line: `named`, `message`, `told`, `commented` or `followed`")
        .field("place", String, "what opens it: a channel's id, the record something was said on, or what a blueprint that told it said opens it")
        .field(
            "about",
            String,
            "what it is about: for somebody named, the message; for somebody named on a record, the comment by \
             its number in the journal; for what a blueprint told, the id it said, or nothing",
        )
        .field("count", Int, "how many times the line has been told")
        .field("unread", Int, "how many lines now wait unread in that inbox"),
        EventSpec::new("channel.discussion.offered", "A channel came to offer its agents discussions, or said them otherwise.")
            .field("channel", String, "the channel's id")
            .field("settings", Object, "what it offers: who takes part, what begins one, the turns, how it is agreed and told")
            .field("instance", String, "what offers them"),
        EventSpec::new("channel.discussion.withdrawn", "A channel offers its agents discussions no longer.")
            .field("channel", String, "the channel's id"),
        EventSpec::new("channel.discussion.begun", "Agents of a channel began talking something over in a thread.")
            .field("channel", String, "the channel's id")
            .field("channel_name", String, "what it is called")
            .maybe("project", String, "the project it belongs to")
            .field("root", String, "the message the thread hangs from")
            .field("message", String, "the message that began it")
            .field("why", String, "what began it: tagged, plans or asked")
            .field("agents", List, "who take part, whoever speaks first first")
            .field("spoken", List, "who has said something in it")
            .maybe("asker", String, "the person whose message it grew from")
            .field("stretch", Int, "which stretch of it this is")
            .field("first", String, "who speaks first"),
        EventSpec::new("channel.discussion.resumed", "A person answered a discussion that waited for them, which goes on.")
            .field("channel", String, "the channel's id")
            .field("channel_name", String, "what it is called")
            .maybe("project", String, "the project it belongs to")
            .field("root", String, "the message the thread hangs from")
            .field("message", String, "the person's answer")
            .field("why", String, "what began it")
            .field("agents", List, "who take part")
            .field("spoken", List, "who has said something in it")
            .maybe("asker", String, "the person whose message it grew from")
            .field("stretch", Int, "which stretch of it this is")
            .field("first", String, "who speaks first: whoever spoke last"),
        EventSpec::new("channel.discussion.turned", "An agent took a turn in a discussion.")
            .field("channel", String, "the channel's id")
            .field("root", String, "the message the thread hangs from")
            .field("stretch", Int, "which stretch")
            .field("nth", Int, "which turn of it")
            .field("slot", String, "whose turn it was")
            .field("word", String, "what it ended with: reply, agree or needs")
            .field("turns", Int, "how many it has had"),
        EventSpec::new("channel.discussion.ended", "A stretch of a discussion ended: agreed, needing a person, at its cap, or ended by a person.")
            .field("channel", String, "the channel's id")
            .field("root", String, "the message the thread hangs from")
            .field("stretch", Int, "which stretch")
            .field("came", String, "agreed, needs, capped, or stopped where a person ended it")
            .field("status", String, "what it is left as: agreed, waiting for a person, or ended by one")
            .field("turns", Int, "how many turns it has had")
            .field("agents", List, "who took part")
            .maybe("by", String, "who ended it, where a person did, as an actor")
            .maybe("by_name", String, "who that is, by name"),
        EventSpec::new(
            "inbox.follows",
            "Somebody follows something now, or follows it otherwise (how often they are told, a page with its \
             pages or alone), or follows it no longer: heard by them alone.",
        )
        .field("owner", String, "who follows it, as an actor")
        .field("kind", String, "what it is: `page`, `board`, `table`, `library`, `channel` or `project`")
        .field("thing", String, "which: a page by its id, a table as `<project>/<name>`, a channel or a project by its id")
        .field("follows", Bool, "whether they follow it now")
        .maybe("pages", Bool, "for a page, whether every page under it is followed with it")
        .maybe("often", String, "how often they are told: `once`, `hourly`, `daily` or `off`")
        .maybe("at", String, "the time of day a daily digest comes at, in their zone"),
        EventSpec::new("inbox.read", "Lines of somebody's inbox were read by their owner, who alone hears it.")
            .field("owner", String, "whose inbox, as an actor")
            .field("lines", List, "the lines that were read")
            .maybe("place", String, "what opens them, where they are all of one place")
            .field("unread", Int, "how many lines now wait unread in that inbox"),
        EventSpec::new(
            "inbox.unread",
            "A line of somebody's inbox that was read was made to wait unread again by its owner, who alone hears it.",
        )
        .field("owner", String, "whose inbox, as an actor")
        .field("lines", List, "the lines that wait unread again")
        .maybe("place", String, "what opens them, where they are all of one place")
        .field("unread", Int, "how many lines now wait unread in that inbox"),
        EventSpec::new(
            "inbox.away",
            "Lines of somebody's inbox were put away by their owner, or went with what they opened: heard by \
             their owner alone.",
        )
        .field("owner", String, "whose inbox, as an actor")
        .field("lines", List, "the lines that went")
        .maybe("place", String, "what opened them, where they were all of one place")
        .field("unread", Int, "how many lines now wait unread in that inbox"),
    ];
    // Who may be told of each: whoever may read channels, and of an
    // inbox whoever may read their own, which each event then holds to
    // its owner by what it takes (`crate::inbox`).
    blueprint::events::heard(all, &[("channel", "channels:read"), ("inbox", "inbox:read")])
}

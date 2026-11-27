//! Where the daemon reads a channel for itself, and where the node reads
//! or follows what was said, each place with why nothing kept to a
//! channel's members leaves by it: the lists the two tests of `kept.rs`
//! hold the source to, apart from them so that each file stays short.

/// Where the daemon reads a channel for itself, asking nobody: the
/// file, the words of the line, how many such lines the file has, and
/// why nothing of a channel kept to its members leaves the daemon by it,
/// or which piece of this work is to close it.
pub(super) const READS_AS_THE_DAEMON: &[(&str, &str, usize, &str)] = &[
    (
        "crates/agents/src/discussing.rs",
        "studio::Channels::daemon(\"who a message said in a discussion's channel answers\")",
        1,
        "what a message comes to in a channel that offers discussions: the author of the message it answers, read \
         to say whose the discussion's asker is, and handed to nobody",
    ),
    (
        "crates/agents/src/discussing.rs",
        "studio::Channels::daemon(\"who was last talking in a channel whose agents wait for a person\")",
        1,
        "who is told that a channel's agents wait for a person: the last person who spoke there, who reads it, \
         told in their own inbox; the bot's line is said there to whoever reads the channel",
    ),
    (
        "crates/agents/src/discussing.rs",
        "studio::Channels::daemon(\"what an agent asked in a thread is shown of it\")",
        1,
        "what an agent asked in a channel's thread is shown of it, as dispatch shows a thread: the agent's turn \
         is of that channel, and whoever asked for it reads the channel (`may_go_on`)",
    ),
    (
        "crates/talk/src/discussed.rs",
        "Channels::daemon(\"which channel a text that offered discussions stops offering them in\")",
        1,
        "which channel a text that stops stops offering discussions in: its id, and nothing of what was said",
    ),
    (
        "crates/talk/src/kept.rs",
        "pub fn daemon(why: &'static str) -> Reader {",
        1,
        "where such leave is made: it reads nothing",
    ),
    (
        "crates/talk/src/channels.rs",
        "Channels::daemon(\"the factory's bot is called something else in every channel it is in\")",
        1,
        "the bot renamed (the plan's row 25): it leaves and joins each channel it is in, and each is said to \
         that channel's members; whoever renamed it is handed its new name and nothing of any channel",
    ),
    (
        "crates/talk/src/channels.rs",
        "let own = Channels::daemon(\"an agent's answer to what it was asked in a channel, said there for it\");",
        1,
        "an agent's answer said where it was asked (`post_answer`), on the leave of whoever asked it there: it \
         reads nothing, and writes only the answer, as the agent, in that channel, to its members; in one kept \
         to its members only an agent that is in it says anything (`joined::comes_in`)",
    ),
    (
        "crates/search/src/lib.rs",
        "Channels::daemon(INDEXING)",
        3,
        "the search index (row 45): everything said is read into it, and what a search finds there is held to \
         what its asker reads as it is answered (`Search::readable`, row 18). Not held: the index is a file",
    ),
    (
        "crates/agents/src/heard.rs",
        "let own = &studio::Channels::daemon(",
        1,
        "which agents a message is for (row 43): the channel, its thread and its last messages are read to \
         choose them and are put into what each is asked. The agents asked are members of that channel, and \
         what such a turn says and shows of it is its members' alone (`agents::kept`: rows 16, 29, 30)",
    ),
    (
        "crates/agents/src/together.rs",
        "let own = &studio::Channels::daemon(",
        2,
        "an agent's answer said where it was asked, and whether it was said already (row 44): the message it \
         answers is read for its thread, and what is said is said in that channel, to its members",
    ),
    (
        "crates/agents/src/kept.rs",
        "let own = studio::Channels::daemon(",
        3,
        "which channel a conversation named by the message it hangs from is held in (row 39), read to say \
         whether a turn is of a kept channel and handed to nobody; an agent told to work with no wall \
         leaving each kept channel it is in (row 48), where its leaving and the factory's word of it are said \
         in that channel, to its members; and who is in a channel given back from the Trash, read to take an \
         agent taken away meanwhile out of it, which is said to its members",
    ),
    (
        "crates/agents/src/removed.rs",
        "let own = studio::Channels::daemon(",
        1,
        "which channels an agent is in, for whoever asks what taking it away comes to (row 17): only those \
         that are open are named",
    ),
    (
        "crates/agents/src/picking.rs",
        "let own = studio::Channels::daemon(",
        1,
        "an agent that stops leaving each channel it was in to say what it was asked there: which those are \
         is read and handed to nobody, and its leaving is said in each, to that channel's members",
    ),
];

/// Where this crate reads or follows what was said, and why nothing
/// said of a kept channel is handed to anybody by it unasked.
pub(super) const READS_WHAT_WAS_SAID: &[(&str, &str, usize, &str)] = &[
    (
        "crates/api/src/taken.rs",
        "let written = journal.read(query).await.ok()?;",
        1,
        "what a write of a row that steers waits for (`api::taken`): rows of tables that steer or are steered, \
         of the project just written to, read to wait for their hearers; nothing is handed on but, where the \
         wait runs out, the id of the row still waited for and who had not taken it",
    ),
    (
        "crates/api/src/followers.rs",
        "let mut heard = journal.follow_every(from, HEARD);",
        1,
        "what tells whoever follows something (`followers::tell`): it hands no event on, and tells a follower of \
         a change only where `followers::may` (`Journal::may_tell_as` and `Engine::may_hear`) says they may \
         hear it, as they hold things then",
    ),
    (
        "crates/api/src/routes/mod.rs",
        "let follow = app.journal.follow_every(since.clone(), query.topics.unwrap_or_default());",
        1,
        "the stream (row 20): each event is handed on only where `App::may_hear` says so",
    ),
    (
        "crates/api/src/routes/mod.rs",
        "let events = app.journal.read_within(&after.unwrap_or_default(), &now, asked).await?;",
        1,
        "what was said lately (row 21): filtered by `App::may_hear` in the line after",
    ),
    (
        "crates/api/src/routes/git.rs",
        "let mut heard = app.journal.follow_every(app.journal.now(), \"project.created,project.removed\");",
        1,
        "which projects' repositories are followed: two topics, neither a channel's, and nothing handed on",
    ),
    (
        "crates/api/src/loaded.rs",
        "let mut heard = app.journal.follow_every(app.journal.now(), topics);",
        1,
        "what an environment's load comes to: that Run began one, and that an instance ended, as a sign to look \
         at the environment it was loaded as, and nothing handed on",
    ),
    (
        "crates/api/src/drain.rs",
        "let mut ended = app.journal.follow_every(app.journal.now(), ENDINGS);",
        1,
        "a daemon draining to stop: that a run or a turn ended, as a sign to look again at what is in hand, and \
         nothing handed on",
    ),
    (
        "crates/api/src/routes/topics.rs",
        "let mut heard = app.journal.follow_every(app.journal.now(), \"project.removed,workspace.removed\");",
        1,
        "where topics are as projects and workspaces are taken away: two topics, neither a channel's, and nothing \
         handed on",
    ),
    (
        "crates/served/src/library.rs",
        "journal.follow_every(journal.now(), \"collection.ensured,project.removed\")",
        1,
        "which libraries there are: two topics, neither a channel's, and nothing handed on",
    ),
    (
        "crates/api/src/routes/secrets.rs",
        "for event in app.journal.read_every(query).await.unwrap_or_default() {",
        1,
        "when a secret was set and by whom: one topic, `secret.set`",
    ),
    (
        "crates/api/src/following.rs",
        "app.journal.follow_every(app.journal.now(), HEARD);",
        1,
        "which texts changed and which folder of a project holds its own: four topics, none a channel's, and nothing handed on",
    ),
    (
        "crates/folders/src/restarting.rs",
        "\"run.finished,instance",
        1,
        "why to look again; nothing handed on",
    ),
];

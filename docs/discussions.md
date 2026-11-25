# Discussions between a channel's agents

A channel's agents talk something over with each other in a thread, and
a person is brought in only for what a person must decide. This is what was built, and why it is built so.

## What begins one

A channel offers discussions by `std/discuss`, applied for it, whose
params are the settings. Three things begin a discussion, each a word of
`on`, all on unless said:

- **tagged**: an agent that takes part names another that takes part.
- **plans**: an agent says what it answered is a plan. That is a **mark
  on the message**, never read from its words. An agent's answer is
  marked by the word its turn ends with: where the channel takes plans,
  a message turn there may end `plan` (the `DECISION:` mechanism every
  turn with options has). A post is marked by `mw post --mark plan`.
- **asked**: a person's message marked `discuss` that names two or more
  who take part. The client offers it beside Send only while what is
  written names two or more of them. Naming two without the mark gets
  two answers, as before.

Before this, an agent's message that named another gave it a turn
(`Agents::dispatch`). Nothing held a pair back but one counter of thirty
agent-started turns an hour for the whole factory, kept in memory and
cleared each time it was reached. That counter is kept as a backstop for
what no discussion holds. It now counts each channel apart and agents
apart from bots, and is kept across a restart. When it is reached, the
bot says so in the channel once, and the person last talking there is
told in their inbox.

## How it goes

What a message comes to is asked in one place, as dispatch works out whom
it is for (`Agents::handed`, over `kernel::discussed::takes`). What
begins a discussion, or is said in a discussion's thread, gives no agent
a turn of its own. Dispatch writes the discussion's row and says
`channel.discussion.begun` in one write, the same done twice. A person's
word in a thread that waits lets the discussion go on
(`channel.discussion.resumed`).

`std/discuss` hears those and starts one `std/discussion` for the
stretch, keyed by the thread and the stretch. It is kept. It calls
`std/discussion-turn`, which keeps nothing and calls itself for each
turn, so the cap bounds how deep it goes. Each turn asks:

- `agent.task`, in the thread (`thread:<root>`, with the channel). The
  agent is shown the thread as it is when the turn is begun, its answer
  is said there, and it ends with `reply`, `agree` or `needs`.
- `discussion.turned`, which writes the turn down.
- `discussion.next`, which says whose turn is next or how the stretch
  ended.

The stretch then asks `discussion.ending` for what is said, and
`discussion.end` writes how it ended and whom that is told to. Then
either the bot says a line in the thread, or a lead answers the person in
the channel. Last, the person is told by `inbox.tell` with a part of the
kind `message`, which opens where it ended.

`agent.consult` was looked at first. It returns the word an agent chose
to the blueprint and says nothing in a channel, so the other agent would
see the answer only through the blueprint. `agent.task` in the thread
does that through the channel itself. It is what is used, with two
additions:

- it says its answer in the thread its conversation names;
- `answers`, a message its answer is said beside.

Every decision is `kernel::discussed`, functions of plain values with
their cases in the kernel's suite: what begins one, whose turn it is,
whether it is agreed (`one`, once everybody has spoken, or `all`), the
cap, the sentences and the brief. The blueprints decide nothing.

## The person

The person told is the one whose message the discussion grew from, else
whoever applied `std/discuss`. In a channel kept to its members, only a
member is told. Which endings they are told of is `tell_on` (`agree`,
`needs`, `cap`, all unless said). "Needs you" says what they must
decide, in the words of the turn that chose `needs`. Their answer in
the thread begins the next stretch, from whoever asked them.

## A lead and its advisors

`lead` names the agent a person talks to, and `advisors` whom it
consults, each with what it is for (both go into the briefs):

- In that channel, a person's message that names no agent is the lead's
  alone. An advisor answers a person only where it is named.
- The lead consults by naming an advisor, or on its plans. The thread
  hangs under the person's message.
- An agreement ends with the lead answering the person in the channel,
  beside what they asked. What it then says that names an advisor asks
  nobody.

## Where it is set

In the client what a channel offers is set under its tab of the Tools
panel beside it (Channel), among everything else that is set of the
channel: Discussions… opens a form made from `std/discuss`'s own params in
the drawer, filled with what is running for the channel, and applies the
text for it. It is offered to whoever may apply and write there, and to
nobody in a conversation. Once applied it is listed under Running here
in the same tab, with whatever else is applied for the channel, where
its settings are offered the same way and it is stopped.

## What an agent is told

Its brief says who else takes part and what each is for, how it asks
them, and the cap (`kernel::discussed::brief`, added where the documents
a channel keeps are told).

## Kept, held and counted

- **Restarts.** The row is the truth, and the blueprints keep nothing of
  a discussion. Stopped anywhere, a discussion ends as one never stopped:
  each turn is found again by its `asking` word, and the ending is said
  once, by the run that ended it. The simulation plays every place of
  one (`discussing.rs`).
- **Permissions and kept channels.** Turns are given as whoever applied
  `std/discuss`, and are held to what giving an agent work takes. The
  discussions' events are said as the channel takes. Only agents of the
  channel that the factory starts take part.
- **Counting.** Turns are counted and never scored:
  `channel.discussion_turns_count`, by channel and by how a stretch came
  out.

## Ending one

Whoever may write in the channel ends a discussion that is going or waits
for a person, without answering: End discussion at the head of its
thread, or `mw channel end <channel> <message>`. The row is left `ended`
and `channel.discussion.ended` is said with `came: stopped`, crediting
them, as one write. The stretch that held it is then stopped, which calls
off the turn in hand as any stop calls off what it is under, and nothing
it would have said as it ended is said. What agents say under it begins
none again; a person may begin one there.

## Not yet

- An agent from outside the factory takes no part, and cannot: it has no
  turns, which a discussion is made of.
- A lead's answer is opened at the person's message, beside which it is.

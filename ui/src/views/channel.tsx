// A channel: people, agents and bots talking in one place.

import { useEffect, useLayoutEffect, useRef, useState } from 'preact/hooks';
import { api } from '../api.ts';
import type { Attachment, Member, Message } from '../api.ts';
import { LEAST_WIDTH, threadsOf, writtenOf } from '../context.ts';
import { newline, toolFor } from '../format.ts';
import { countDrawn } from '../drawn.ts';
import { PANEL_WIDTH } from './edge.tsx';
import { command, commanding, complete, matching, mention, mentionable, taking, typing, under } from '../mentions.ts';
import type { Mentionable } from '../mentions.ts';
import { phone, touched } from '../phone.ts';
import { recall, sentHere } from '../recalled.ts';
import type { Recalling } from '../recalled.ts';
import { typingIn, typingUnder, workingIn } from '../typing.ts';
import { Typing, WorkingLine } from './typing.tsx';
import { effect, signal } from '@preact/signals';
import { mayChange } from '../holds.ts';
import { agents, attempt, channels, contextFront, contextOpen, contextWidth, contextWidthSaid, foldContext, keepContext, keptContext, SAVED_WRITTEN, loadMessages, loadThread, me, messages, speaker, onChannelRemoved, openTab, takeHint, threads, working } from '../store.ts';
import { Empty, Prose } from '../ui.tsx';
import { WordsOf } from './pathlink.tsx';
import { Formatting, use, write } from './formatting.tsx';
import { actorKind, agentState, classes, initials, when } from '../util.ts';
import { Asking } from './asking.tsx';
import { AddChannelMember, Participants } from './participants.tsx';
import { Context } from './context.tsx';
import { useSlid } from './toolsslide.ts';
import { calledOf, hashed } from '../kept.ts';
import { signs } from './signs.tsx';
import { FollowButton } from './followbutton.tsx';
import { Subject } from './conversed.tsx';
import { learnModels, modelOf, reads, runsOn, Standing, Who } from './who.tsx';
import { FailedRun } from './failedrun.tsx';
import { isExternal } from '../external.ts';
import { ArchivedHead, ArchivedLine } from './archivedpart.tsx';
import { Topic } from './topic.tsx';
import { useKept, useKeptRef } from './returned.tsx';
import { earlierSaid, folded, toggled } from '../folded.ts';
import { standsBy, useKeptSizes } from '../inview.ts';
import { Attaching, Clip, pasted } from './attaching.tsx';
import { DiscussionHead, DiscussSend, MarkTag } from './discussing.tsx';
import { Attached } from './attached.tsx';
import { Offered } from './offered.tsx';

/** The thread open beside each channel, by the id of the message it is under. Kept, so that it is open again when the page is read again. */
const openThread = signal<Record<string, string>>(threadsOf(keptContext('threads')));
effect(() => keepContext('threads', JSON.stringify(openThread.value)));
// A channel that was taken away has no thread open in it any more.
onChannelRemoved((id) => {
  const { [id]: _was, ...rest } = openThread.value;
  openThread.value = rest;
});
/** Opens the thread under `root` beside its channel: for what leads to a message that was said in one (`views/inbox.tsx`). */
export function showThread(channel: string, root: string): void {
  openThread.value = { ...openThread.value, [channel]: root };
  contextFront.value = 'thread';
  if (!contextOpen.value) foldContext();
}
/**
 * The message a finger last touched, by its id. What can be done with a
 * message is shown while it is pointed at, and a finger points at nothing:
 * touching the message shows it, and touching it again puts it away.
 */
const picked = signal<string | null>(null);
/** The channel whose composer is drawn tall in the panel at its right, where a long message has room; none when it is under the channel. */
const writingWide = signal<string | null>(keptContext('writing') || null);
effect(() => keepContext('writing', writingWide.value ?? ''));
/**
 * What is being written in each channel and thread, and what is attached
 * to go with it. Kept outside the box it is written in: that box is drawn
 * under the channel or in the panel beside it, and is gone while another
 * tab is showing.
 */
const written = new Map<string, { text: string; attached: Attachment[] }>(writtenOf<Attachment>(keptWritten()));
/** What was being written when the page was last read, in this tab of the browser. */
function keptWritten(): string | null {
  try {
    return sessionStorage.getItem(SAVED_WRITTEN);
  } catch {
    return null;
  }
}
/** Keeps what is being written for as long as the browser's tab is there: over reading the page again, not over closing it. */
function keepWritten(): void {
  try {
    sessionStorage.setItem(SAVED_WRITTEN, JSON.stringify([...written]));
  } catch {
    /* it is kept while the page is, as before */
  }
}
// What was being written in a channel that was taken away, or under a message of it, is written to nothing.
/** Where the reader is in what they sent in each channel and thread, as Up takes them back through it, with what they had written before (`recalled.ts`). */
const recalls = new Map<string, Recalling>();
onChannelRemoved((id) => {
  for (const kept of [...written.keys()]) if (kept === id || kept.startsWith(`${id}/`)) written.delete(kept);
  for (const kept of [...recalls.keys()]) if (kept === id || kept.startsWith(`${id}/`)) recalls.delete(kept);
  keepWritten();
});

export function ChannelView({ id }: { id: string }) {
  const channel = channels.value.find((entry) => entry.id === id);
  const list = messages.value[id];
  const [adding, setAdding] = useState(false);
  const scroller = useRef<HTMLDivElement>(null);
  // Whether it was at its newest message when it was last drawn: kept by its tab, so that it is shown again where it was left.
  const pinned = useKeptRef('channel.pinned', true);
  // Where it was last put at its newest message: a scroll that leaves it there is its own and not the reader's, though
  // what is under it has since come into sight at another height than it stood by (`inview.ts`).
  const putAt = useRef(-1);
  const toFoot = (box: HTMLElement) => {
    box.scrollTop = box.scrollHeight;
    putAt.current = box.scrollTop;
  };
  const [flash, setFlash] = useState<string | undefined>(undefined);
  // What a bot says again and again that the reader has shown, by the first of each run (`folded.ts`): kept by its tab.
  const [unfolded, setUnfolded] = useKept<string[]>('channel.unfolded', []);
  // The message whose press shows or folds what came before it, and where it was in the window: it is kept there as that is drawn.
  const steady = useRef<{ id: string; top: number } | null>(null);

  useEffect(() => {
    if (!messages.value[id]) void attempt(() => loadMessages(id));
    setFlash(takeHint(`channel:${id}`)?.message);
  }, [id]);

  // Stay at the newest message unless the reader has scrolled up to read.
  useLayoutEffect(() => {
    const box = scroller.current;
    if (!box) return;
    const target = flash ? box.querySelector(`[data-id="${flash}"]`) : null;
    if (target) target.scrollIntoView({ block: 'center' });
    else if (pinned.current) toFoot(box);
  }, [list?.length, flash]);
  // What is out of sight is not laid out (`inview.ts`): what comes into sight at another height than it stood by leaves the
  // channel at its newest message, if that is where the reader was.
  useKeptSizes(
    scroller,
    () => {
      const box = scroller.current;
      if (box && pinned.current && !flash && !steady.current) toFoot(box);
    },
    [!!channel],
  );
  // What was shown or folded under the pointer leaves the press where it was: what came before it is drawn over it.
  useLayoutEffect(() => {
    const held = steady.current;
    const box = scroller.current;
    if (!held || !box) return;
    steady.current = null;
    const now = box.querySelector(`:scope > [data-id="${held.id}"]`);
    if (now) box.scrollTop += now.getBoundingClientRect().top - held.top;
  });

  // Which model each agent here runs on is found out once it is looked at, and what each model its messages name is called.
  const names = channel?.members.map((member) => member.actor).join(',') ?? '';
  const said: Record<string, string[]> = {};
  for (const one of list ?? []) {
    if (!one.model || actorKind(one.author) !== 'agent') continue;
    const of = (said[one.author.slice('agent:'.length)] ??= []);
    if (!of.includes(one.model)) of.push(one.model);
  }
  const ids = Object.entries(said).map(([agent, models]) => `${agent}:${models.join('|')}`).join(',');
  useEffect(() => {
    if (channel) learnModels(channel.members, said);
  }, [names, ids]);

  if (!channel) return <Empty title="This channel is gone" />;
  const members = channel.members;
  // A bot's repeated updates folded under the latest of each (`folded.ts`), one asked for never hidden.
  const drawn = folded(list ?? [], unfolded, flash);
  const keys = drawn.flatMap((one) => (one.earlier ? [one.earlier.key] : []));
  // Those that answer by themselves when asked: agents and bots.
  const answering = members.filter((member) => ['agent', 'bot'].includes(actorKind(member.actor)));
  // Those that answer here: one the factory does not start is asked nothing by what is said.
  const agentsHere = members.filter((member) => actorKind(member.actor) === 'agent' && !isExternal(agents.value.find((slot) => slot.name === member.name)));

  // How the composer is given a name to ask, by whatever is outside it.
  const ask = useRef<((handle: string) => void) | null>(null);
  // And how it is given words to begin with, as they are.
  const say = useRef<((words: string) => void) | null>(null);
  const under = openThread.value[id];
  const root = under ? list?.find((message) => message.id === under) : undefined;
  const open = (message: string | null) => {
    const { [id]: _was, ...rest } = openThread.value;
    openThread.value = message ? { ...rest, [id]: message } : rest;
    // It is what is in front in the panel, from the moment it is opened.
    if (message) contextFront.value = 'thread';
    // A thread is read in the panel at the right, so opening one opens the
    // panel, and it stays open: closing the thread closes the thread.
    if (message && !contextOpen.value) foldContext();
  };

  // The panel at the right is there when it was opened, and whenever a thread is, which is read in it.
  const wide = writingWide.value === id;
  const beside = contextOpen.value || !!root || wide;
  // Coming in, it slides into the room it takes; going, a copy of it slides out over the room it gave back (`toolsslide.ts`).
  const slid = useSlid(beside);
  // Writing at length is asked for and given up from the composer: with the
  // panel as it was before, gone again if it was only there for this.
  // It is written there only while its tab is in front in the panel. With
  // another tab in front, a thread say, it is written under the channel as
  // ever, and the tab waits with what has been written so far.
  const writingThere = wide && contextFront.value === 'write';
  const writeWide = () => {
    if (writingThere) {
      writingWide.value = null;
      return;
    }
    writingWide.value = id;
    contextFront.value = 'write';
  };
  const hint =
    agentsHere.length === 1 && answering.length === 1
      ? `Message ${hashed(channel)}. ${agentsHere[0]!.name} answers what you say here.`
      : answering.length > 0
        ? `Message ${hashed(channel)}. Type @ to ask one of them.`
        : `Message ${hashed(channel)}`;
  const foldBeside = () => {
    // Folded away, it takes the thread that was open in it with it, and what is written goes back under the channel.
    if (root) open(null);
    if (wide) writingWide.value = null;
    if (contextOpen.value || !beside) foldContext();
  };

  // What a drag of the panel's edge is held to: this is not drawn again while the edge is held.
  countDrawn('channel');
  return (
    <div
      class="channel"
      ref={slid.box}
      // The channel, then the tools panel when it is there: as wide as it
      // was left, or as much of that as the window has room for once the
      // channel has what it needs.
      // The width is a property of the grid, so that the panel's edge can
      // set it straight onto the layout while it is held, with nothing
      // here drawn again until it is let go (`views/edge.tsx`).
      // Until the reader says otherwise it is the usual width as text is the usual size, and as much wider as text is
      // larger, so that the tabs it begins with are all on its line whatever a theme says of text.
      style={{ [PANEL_WIDTH]: contextWidthSaid.value ? `${contextWidth.value}px` : `calc(${contextWidth.value}px * var(--size))`, gridTemplateColumns: `minmax(360px, 1fr)${beside ? ` minmax(${LEAST_WIDTH}px, var(${PANEL_WIDTH}))` : ''}` }}
    >
    <div class="split" style="grid-template-rows:auto 1fr auto auto">
      <div class="view-head">
        {channel.conversation ? (
          <Subject channel={channel} />
        ) : channel.kept ? (
          <h1>
            <span class="kept-lock" role="img" aria-label="Private" title="Private: read by whoever is in it and nobody else">
              {signs.lock(15)}
            </span>
            {calledOf(channel)}
          </h1>
        ) : (
          <h1># {channel.name}</h1>
        )}
        {channel.topic && <Topic topic={channel.topic} />}
        <ArchivedHead channel={channel} />
        <span class="grow" />
        {!channel.conversation && <FollowButton thing={{ kind: 'channel', thing: channel.id }} />}
        <Participants members={members} channel={channel} />
        {/* The panel on the right, folded away or brought back. It says what it is: a
            window with a part at its right, and the word, lit while the panel is open. */}
        <button
          class="context-fold"
          aria-label={beside ? 'Collapse the tools panel' : 'Expand the tools panel'}
          aria-expanded={beside}
          aria-pressed={beside}
          title={beside ? 'Close the tools panel' : 'Open the tools panel: threads, what was attached, the channel and its people, the board, the documents'}
          onClick={foldBeside}
        >
          <svg width="16" height="16" viewBox="0 0 16 16" aria-hidden="true">
            <rect x="1.75" y="2.75" width="12.5" height="10.5" rx="2" fill="none" stroke="currentColor" stroke-width="1.3" />
            <path d="M10 2.75v10.5" fill="none" stroke="currentColor" stroke-width="1.3" />
            {beside && <path d="M10.6 3.4h2.2a1 1 0 0 1 1 1v7.2a1 1 0 0 1-1 1h-2.2z" fill="currentColor" />}
          </svg>
          Tools
        </button>
      </div>
      <div
        class="messages"
        ref={scroller}
        onScroll={(event) => {
          const box = event.currentTarget;
          if (Math.abs(box.scrollTop - putAt.current) < 1) return;
          pinned.current = box.scrollHeight - box.scrollTop - box.clientHeight < 60;
        }}
      >
        {list && list.length === 0 && (
          <Empty title="Nothing has been said here yet">
            <span>
              {agentsHere.length === 0
                ? 'Add an agent, then ask it for something, for example to set up a new project.'
                : `Say what you need. ${agentsHere[0]!.name} is listening.`}
            </span>
            {agentsHere.length === 0 && mayChange(me.value?.principal.scopes ?? [], false) && (
              <button class="primary" onClick={() => setAdding(true)}>
                Add an agent
              </button>
            )}
          </Empty>
        )}
        {drawn.map(({ message, follows, earlier }, index) => (
          <MessageRow
            key={message.id}
            message={message}
            follows={follows}
            far={index < drawn.length - NEAR}
            earlier={
              earlier && {
                count: earlier.count,
                open: earlier.open,
                onToggle: (event) => {
                  const at = (event.currentTarget as Element).closest('.message');
                  steady.current = at ? { id: message.id, top: at.getBoundingClientRect().top } : null;
                  setUnfolded((was) => toggled(was, earlier.key, !earlier.open, keys));
                },
              }
            }
            flash={flash === message.id}
            onThread={() => open(message.id)}
            // Lit while its thread is what is being read beside it; not while another tab of the panel is in front.
            inThread={under === message.id && contextFront.value === 'thread'}
          />
        ))}
      </div>
      {/* Who can be asked here, over where they are asked: the agents and the bots. The people are in the head.
          Each says by its mark whether it is working, and nothing else on the page does. */}
      {answering.length > 0 && (
        <div class="members" aria-label="Agents and bots in this channel">
          {answering.map((member) => (
            // Pressing one begins asking it: its name goes where the caret is.
            <Participant key={member.actor} member={member} onAsk={() => ask.current?.(member.actor.slice(member.actor.indexOf(':') + 1))} />
          ))}
        </div>
      )}
      {/* Under the channel, unless it is being written in the panel beside it just now. */}
      {/* In an archived channel nothing is said: one line says so where the composer was. */}
      {/* Who is writing in the channel just now: a line of its own over where one writes, beginning where the names of
          whoever said something begin (beside their pictures), since the box one writes in is narrower than they are and centred. Its room is there whether or not anybody is,
          so nothing above or below it moves as it comes and goes; while somebody is, three dots move before its words.
          Whoever is writing in a thread is said in the thread's line and not here (`typingIn`). */}
      {/* Who is at work for the channel and not writing in it, apart from who is typing: over the right of that line's room, taking none. */}
      {!channel.archived && !writingThere && <WorkingLine at={workingIn(working.value, id)} />}
      {!channel.archived && !writingThere && <Typing names={typingIn(working.value, id)} edge />}
      {channel.archived ? <ArchivedLine channel={channel} /> : !writingThere && <Composer key={id} channel={id} members={members} ask={ask} say={say} hint={hint} onWide={writeWide} />}
      {adding && <AddChannelMember begins="agent" channel={channel} onClose={() => setAdding(false)} />}
    </div>
    {beside && (
      <Context
        channel={channel}
        coming={slid.coming}
        messages={list ?? []}
        thread={root ? <Thread channel={id} root={root} members={members} /> : null}
        writing={wide && !channel.archived ? <Composer key={id} channel={id} members={members} ask={ask} say={say} hint={hint} wide onWide={writeWide} /> : null}
        onCloseWriting={() => {
          // The panel stays, as it does when a thread in it is closed.
          if (!contextOpen.value) foldContext();
          writingWide.value = null;
        }}
        onAway={foldBeside}
        // What is begun from the panel (a command of the bot's) is written where the message is.
        onSay={(words) => say.current?.(words)}
        onCloseThread={() => {
          // The panel stays, with the reader's own tabs, even where it was only there for the thread.
          if (!contextOpen.value) foldContext();
          open(null);
        }}
      />
    )}
    </div>
  );
}

/**
 * A thread: one message and what has been said under it. It is read in
 * the tools panel beside the channel, as a tab of it, so that there is
 * one thing at the right of a channel and not two. In a thread an agent that has been talking in it goes on
 * answering without being named again.
 */
function Thread(props: { channel: string; root: Message; members: Member[] }) {
  const { channel, root } = props;
  const said = threads.value[root.id];
  const archivedHere = channels.value.find((one) => one.id === channel && one.archived);
  const scroller = useRef<HTMLDivElement>(null);
  // Whether the reader is at what was said last, where the thread is kept as what is in sight comes to be laid out.
  const atFoot = useRef(true);
  useEffect(() => void attempt(() => loadThread(channel, root.id)), [channel, root.id]);
  useLayoutEffect(() => {
    if (scroller.current) scroller.current.scrollTop = scroller.current.scrollHeight;
  }, [said?.length]);
  useKeptSizes(scroller, () => {
    if (scroller.current && atFoot.current) scroller.current.scrollTop = scroller.current.scrollHeight;
  });
  return (
    <div class="thread">
      <DiscussionHead channel={channel} root={root.id} />
      <div
        class="messages"
        ref={scroller}
        onScroll={(event) => {
          const box = event.currentTarget;
          atFoot.current = box.scrollHeight - box.scrollTop - box.clientHeight < 60;
        }}
      >
        <MessageRow message={root} flash={false} />
        <div class="thread-count faint">{said ? (said.length === 1 ? '1 reply' : `${said.length} replies`) : 'Reading the thread'}</div>
        {said?.map((message, index) => (
          <MessageRow key={message.id} message={message} follows={said[index - 1]} flash={false} far={index < said.length - NEAR} />
        ))}
      </div>
      {/* Who is writing in the thread, as under the channel: a line of its own over where one writes, beginning where
          the names of whoever said something do, whose room is there whether or not anybody is. */}
      {!archivedHere && <Typing names={typingUnder(working.value, channel, root.id)} edge />}
      {archivedHere ? (
        <ArchivedLine channel={archivedHere} />
      ) : (
        <Composer key={root.id} channel={channel} thread={root.id} members={props.members} hint="Reply in the thread. Whoever was answering here goes on answering." />
      )}
    </div>
  );
}

/** An agent or a bot in the channel, in the row over its composer. Pointing at it says what it is. */
function Participant({ member, onAsk }: { member: Member; onAsk?: () => void }) {
  const kind = actorKind(member.actor);
  return (
    <Who member={member} class={classes('badge plain', kind)} onPress={onAsk} says={onAsk && `Ask ${member.name}`}>
      {kind === 'agent' && <Standing agent={member.actor.slice('agent:'.length)} />}
      {member.name}
    </Who>
  );
}

/**
 * How many of the newest messages are always laid out (`inview.ts`): what a reader is most often at and what arrives,
 * so that a list kept at its foot is as tall there as it is, whatever comes into sight above it.
 */
const NEAR = 50;

function MessageRow(props: { message: Message; follows?: Message; flash: boolean; onThread?: () => void; inThread?: boolean; far?: boolean; earlier?: { count: number; open: boolean; onToggle: (event: Event) => void } }) {
  const { message, follows, flash, onThread } = props;
  const replies = message.replies ?? 0;
  const kind = actorKind(message.author);
  // What the factory's bot says is the bot's, as anybody's message is theirs: which blueprint's step
  // had it said is kept with the message for whoever reads the journal, and is no part of the channel.
  // One that keeps more than it says (the whole of an error) has that opened under it.
  const more = message.said_by?.detail ? message.said_by : null;
  const [opened, setOpened] = useState(false);
  // What wrote it: the model put down with the message, by the name its provider gives it, else, for one put down with
  // none, what its agent runs on now (`modelOf`).
  const said = (one: Message) => (actorKind(one.author) === 'agent' ? modelOf(one.author.slice('agent:'.length), one.model) : null);
  const model = said(message);
  const author = { actor: message.author, name: message.author_name };
  // Consecutive messages from one author within five minutes read as one,
  // unless a different model wrote the second, as each is said.
  const continued = follows && follows.author === message.author && message.created - follows.created < 300_000 && said(follows) === model;
  // What can be done with the message, shown while it is pointed at. One
  // that has replies already has the way into its thread: their count.
  const threads = onThread && replies === 0;
  const actions = threads && (
    <span class={classes('message-actions', continued && 'unnamed')}>
      {threads && (
      <button
        class="ghost icon"
        aria-label="Reply in thread"
        title="Reply in thread"
        onClick={() => {
          // What a touch brought up has been used: it is not still there when the thread is left.
          picked.value = null;
          onThread();
        }}
      >
        <svg width="16" height="16" viewBox="0 0 16 16" aria-hidden="true">
          <path d="M2.5 3.5h11v7H8.2L5 13.2v-2.7H2.5z" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linejoin="round" />
          <path d="M5.5 6.2h5M5.5 8.2h3" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" />
        </svg>
      </button>
      )}
    </span>
  );
  return (
    <div
      class={classes('message', props.far && 'said-far', props.inThread && 'in-thread', onThread && picked.value === message.id && 'picked')}
      data-id={message.id}
      // Out of sight it is not laid out, and stands as tall as it was last laid out (`inview.ts`).
      style={`${standsBy(message.id, 60)}${flash ? ';background:var(--accent-soft)' : ''}`}
      onClick={
        touched && onThread
          ? (event) => {
              // A touch on something in the message that does something of its own is not a touch on the message.
              if ((event.target as Element).closest('a, button, .who-anchor, .attached')) return;
              picked.value = picked.value === message.id ? null : message.id;
            }
          : undefined
      }
    >
      {continued ? (
        <span />
      ) : (
        <Who member={author}>
          <div class={classes('avatar', kind)}>{kind === 'system' ? '◆' : initials(message.author_name)}</div>
        </Who>
      )}
      <div>
        {!continued && (
          <div>
            {/* Words, and nothing to point at or press: who somebody is, is the card over their picture beside this. */}
            <span class={classes('who', kind)}>{message.author_name}</span>
            {kind === 'agent' && (
              <span class="when">
                agent
                {model && (
                  <>
                    {' · '}
                    <span class="model" title="The model that wrote this">
                      {model}
                    </span>
                  </>
                )}
              </span>
            )}
            {kind === 'bot' && <span class="when">bot</span>}
            <span class="when">{when(message.created)}</span>
            {/* After who said it and when, on their line: further along under a long name than a short one. */}
            {actions}
          </div>
        )}
        <MarkTag mark={message.mark} />
        {message.body && (
          <div class="body">
            {/* A file named in it is of the channel's project, where it is one's. */}
            <WordsOf.Provider value={channels.value.find((one) => one.id === message.channel)?.project ?? null}>
              <Prose text={message.body} />
            </WordsOf.Provider>
          </div>
        )}
        {(message.attachments?.length ?? 0) > 0 && <Attached attachments={message.attachments!} channel={message.channel} />}
        {/* One that keeps more than it says (the whole of an error) says so where it can be seen without pointing. */}
        {more && (
          <button class="ghost link said-more" aria-expanded={opened} onClick={() => setOpened(!opened)}>
            {opened ? 'Hide details' : 'Details'}
          </button>
        )}
        {more && opened && (
          <div class="said-by">
            <pre>{more.detail}</pre>
            {/* What ran is looked at where what runs is, by whoever wants it; the channel does not speak of it. A run the
                message is about (one that failed) is opened first, at the step that failed. */}
            {more.about_run && <FailedRun run={more.about_run} />}
            <button class="ghost link" onClick={() => openTab({ kind: 'instance', id: more.instance })}>
              Open what was running
            </button>
          </div>
        )}
        {onThread && replies > 0 && (
          <button class="ghost link replies" onClick={onThread}>
            {replies === 1 ? '1 reply' : `${replies} replies`}
          </button>
        )}
        {/* The latest of what a bot says again and again: how many like it came before, shown or folded by a press (`folded.ts`). */}
        {props.earlier && (
          <button class="ghost link earlier-updates" aria-expanded={props.earlier.open} onClick={props.earlier.onToggle}>
            {earlierSaid(props.earlier.count, props.earlier.open)}
          </button>
        )}
      </div>
      {/* One that carries on from the last has no such line: there it is on the message's upper edge, where its words start. */}
      {continued && actions}
    </div>
  );
}

const KIND_GLYPH = { agent: '◈', bot: '⚙', user: '●', system: '◆' };

type Ask = { current: ((handle: string) => void) | null };

interface ComposerProps {
  channel: string;
  thread?: string;
  members: Member[];
  hint: string;
  ask?: Ask;
  /** How it is given words to begin with, by whatever is outside it. */
  say?: Ask;
  /** Drawn tall, in the panel beside the channel. */
  wide?: boolean;
  /** Moves it between there and under the channel; left out where it cannot be moved. */
  onWide?: () => void;
}

function Composer({ channel, thread, members, hint, ask, say, wide, onWide }: ComposerProps) {
  const kept = thread ? `${channel}/${thread}` : channel;
  const [text, setText] = useState(() => written.get(kept)?.text ?? '');
  // The mention being typed, if the caret is in one, and which choice is marked.
  const [at, setAt] = useState<{ from: number; typed: string } | null>(null);
  const [marked, setMarked] = useState(0);
  // What is being written to the factory's bot, if the caret is just after its name: its commands are offered then.
  const [toBot, setToBot] = useState<{ from: number; typed: string } | null>(null);
  const box = useRef<HTMLTextAreaElement>(null);
  // What is offered as it is written is placed by these, on the page's body (`offered.tsx`).
  const boxed = useRef<HTMLDivElement>(null);
  const standing = useRef<HTMLDivElement>(null);
  // Not under a finger: there the caret brings the keyboard up over half of what was opened to be read.
  useEffect(() => {
    const area = box.current;
    if (!area) return;
    // What was already written is as tall as it is, and is carried on from its end.
    if (area.value) {
      if (!wide) fit(area);
      area.setSelectionRange(area.value.length, area.value.length);
    }
    if (!touched) area.focus();
  }, [channel, thread]);
  // What has been put here to go with the message, and how many more are on their way.
  const [attached, setAttached] = useState<Attachment[]>(() => written.get(kept)?.attached ?? []);
  // Kept as it is written, not a moment after: the page may be read again in that moment.
  useLayoutEffect(() => {
    if (text || attached.length > 0) written.set(kept, { text, attached });
    else written.delete(kept);
    keepWritten();
  }, [kept, text, attached]);
  const [arriving, setArriving] = useState(0);
  const [over, setOver] = useState(false);
  // What a paste brought that could not be attached, said in a line under the box until the next key.
  const [note, setNote] = useState<string | null>(null);
  const picker = useRef<HTMLInputElement>(null);
  const attach = async (files: File[]) => {
    for (const file of files) {
      setArriving((now) => now + 1);
      const kept = await attempt(() => api.attach(channel, file));
      setArriving((now) => now - 1);
      if (kept) setAttached((now) => [...now, kept]);
    }
  };

  // Everyone in the channel but whoever is typing; and the factory's bot, which can be written to in any channel
  // and is of it from the first thing said to it there.
  const bot = speaker.value;
  const here = members.filter((member) => member.actor !== me.value?.actor);
  const others = bot && !here.some((member) => member.actor === `bot:${bot.name}`) ? [...here, { actor: `bot:${bot.name}`, name: bot.name }] : here;
  const offered: Mentionable[] = at ? matching(mentionable(others), at.typed) : [];
  // What it takes that what was typed could still become. Offered and never taken for the reader: Enter sends, as it does.
  const taken = !at && toBot && bot ? taking(bot.commands, toBot.typed) : [];
  const fit = (area: HTMLTextAreaElement) => {
    // Drawn tall it has the room of the panel, however much is written.
    if (wide) return;
    area.style.height = 'auto';
    area.style.height = `${area.scrollHeight}px`;
  };
  const look = (area: HTMLTextAreaElement) => {
    const one = area.selectionStart === area.selectionEnd;
    const now = one ? typing(area.value, area.selectionStart) : null;
    const said = one && !now ? commanding(area.value, area.selectionStart, speaker.value?.name) : null;
    if (now?.typed !== at?.typed || now?.from !== at?.from || said?.typed !== toBot?.typed) setMarked(0);
    setAt(now);
    setToBot(said);
  };
  const take = (written: string) => {
    const area = box.current;
    if (!area || !toBot) return;
    const done = command(area.value, area.selectionStart, toBot.from, written);
    write(area, { text: done.text, from: done.caret, to: done.caret });
    look(area);
  };
  // Words begun outside the composer are put where the caret was last, apart from what is before them.
  const begin = (words: string) => {
    const area = box.current;
    if (!area) return;
    const caret = area.selectionStart;
    const apart = caret > 0 && !/\s/.test(area.value[caret - 1]!) ? ' ' : '';
    const end = caret + apart.length + words.length;
    write(area, { text: area.value.slice(0, caret) + apart + words + area.value.slice(caret), from: end, to: end });
    area.focus();
    look(area);
  };
  useEffect(() => {
    if (!say) return;
    say.current = begin;
    return () => {
      say.current = null;
    };
  }, [say]);
  const choose = (one: Mentionable) => {
    const area = box.current;
    if (!area || !at) return;
    const done = complete(area.value, area.selectionStart, at.from, one.handle);
    setAt(null);
    write(area, { text: done.text, from: done.caret, to: done.caret });
  };
  // A name chosen outside the composer is put where the caret was last.
  const name = (handle: string) => {
    const area = box.current;
    if (!area) return;
    const done = mention(area.value, area.selectionStart, handle);
    write(area, { text: done.text, from: done.caret, to: done.caret });
  };
  useEffect(() => {
    if (!ask) return;
    ask.current = name;
    return () => {
      ask.current = null;
    };
  }, [ask]);

  const send = async (mark?: string) => {
    const body = text.trim();
    // With something attached a message need say nothing; while something is still arriving it waits.
    if ((!body && attached.length === 0) || arriving > 0) return;
    const with_ = attached;
    setText('');
    setAttached([]);
    setNote(null);
    // What was offered was of what was written, which is gone: neither who to name nor what the bot takes stays up.
    setAt(null);
    setToBot(null);
    setMarked(0);
    recalls.delete(kept);
    if (box.current) box.current.style.height = 'auto';
    const sent = await attempt(() => api.post(channel, body, thread, with_.map((one) => one.id), mark));
    if (!sent) {
      setText(body);
      setAttached(with_);
      return;
    }
    // A long message that has been sent is done with: the next is written under the channel again.
    if (wide) onWide?.();
  };

  return (
    <div class={classes('composer', wide && 'wide')}>
      <div
        ref={boxed}
        class={classes('composer-box', over && 'over')}
        // A file let go over the composer is attached.
        onDragOver={(event) => {
          if (!event.dataTransfer?.types.includes('Files')) return;
          event.preventDefault();
          setOver(true);
        }}
        onDragLeave={() => setOver(false)}
        onDrop={(event) => {
          setOver(false);
          const files = [...(event.dataTransfer?.files ?? [])];
          if (files.length === 0) return;
          event.preventDefault();
          void attach(files);
        }}
      >
        {/* What stands over the box and outside it, taking no room: what is waiting to go with the message
            (`attaching.tsx`), and over that who can be named and what the bot takes, offered, which are drawn on the
            page's body by where this is (`offered.tsx`), so that nothing they are in cuts them off. */}
        <div class="composer-over" ref={standing}>
          {at && offered.length > 0 && (
            <Offered box={boxed} over={standing} area={box} wide={wide} class="names" label="Mention">
              {offered.map((one, index) => (
                <button
                  key={one.handle}
                  role="option"
                  aria-selected={index === marked}
                  class={classes(index === marked && 'marked')}
                  // Before the box loses the caret, which a click would take.
                  onMouseDown={(event) => {
                    event.preventDefault();
                    choose(one);
                  }}
                  onMouseEnter={() => setMarked(index)}
                >
                  <span class={classes('glyph', one.kind)}>{KIND_GLYPH[one.kind]}</span>
                  {/* What is said of them is under the name and whole, as it is where the @ button lists who can be asked. */}
                  <span class="asked-words">
                    <strong class="asked-name">@{one.handle}</strong>
                    <span class="asked-under">
                      {under(one, {
                        state: one.kind === 'agent' && !reads(one.handle) ? agentState(one.handle in working.value, agents.value.find((slot) => slot.name === one.handle)) : null,
                        on: one.kind === 'agent' ? (reads(one.handle) ?? runsOn(one.handle)) : null,
                        takes: speaker.value?.name === one.handle,
                      })}
                    </span>
                  </span>
                </button>
              ))}
            </Offered>
          )}
          {taken.length > 0 && bot && (
            <Offered box={boxed} over={standing} area={box} wide={wide} class="commands" label={`What ${bot.name} takes`}>
              {taken.map((one, index) => (
                <button
                  key={one.written}
                  role="option"
                  aria-selected={index === marked}
                  class={classes(index === marked && 'marked')}
                  // Before the box loses the caret, which a click would take.
                  onMouseDown={(event) => {
                    event.preventDefault();
                    take(one.written);
                  }}
                  onMouseEnter={() => setMarked(index)}
                >
                  <code>{one.written}</code>
                  <span class="muted truncate">{one.does}</span>
                </button>
              ))}
              <span class="faint commands-how">Tab puts the marked one in. Enter sends what is written.</span>
            </Offered>
          )}
          <Attaching attached={attached} arriving={arriving} onRemove={(one) => setAttached(attached.filter((other) => other.id !== one.id))} />
        </div>
        <input
          ref={picker}
          type="file"
          multiple
          hidden
          aria-label="Files to attach"
          onChange={(event) => {
            void attach([...(event.currentTarget.files ?? [])]);
            // So that the same file can be chosen again.
            event.currentTarget.value = '';
          }}
        />
        {/* Written in the panel beside the channel, the box is the panel's whole width, and who is writing is a small line
            at its top, flush with its left. Under the channel it is a line over the box (in the channel's view, above).
            A thread says who is writing in it in a line over its box (in the thread's view, above). */}
        {!thread && wide && <Typing names={typingIn(working.value, channel)} />}
        <textarea
          ref={box}
          rows={1}
          value={text}
          placeholder={hint}
          aria-label={thread ? 'Reply' : 'Message'}
          // A picture copied from somewhere, pasted here, is attached; what could not be is said under the box.
          onPaste={(event) => pasted(event, attach, setNote)}
          onInput={(event) => {
            const area = event.currentTarget;
            setText(area.value);
            fit(area);
            look(area);
          }}
          onClick={(event) => look(event.currentTarget)}
          onKeyUp={(event) => {
            if (['ArrowLeft', 'ArrowRight', 'Home', 'End'].includes(event.key)) look(event.currentTarget);
          }}
          onBlur={() => {
            setAt(null);
            setToBot(null);
          }}
          onKeyDown={(event) => {
            setNote(null);
            if (taken.length > 0) {
              // While the bot's commands are offered, these keys choose among them; Enter is left to send.
              if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
                event.preventDefault();
                setMarked((marked + (event.key === 'ArrowDown' ? 1 : taken.length - 1)) % taken.length);
                return;
              }
              if (event.key === 'Tab' && !event.shiftKey) {
                event.preventDefault();
                take((taken[marked] ?? taken[0]!).written);
                return;
              }
              if (event.key === 'Escape') {
                event.preventDefault();
                event.stopPropagation();
                setToBot(null);
                return;
              }
            }
            if (at && offered.length > 0) {
              // While names are offered, these keys choose among them.
              if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
                event.preventDefault();
                setMarked((marked + (event.key === 'ArrowDown' ? 1 : offered.length - 1)) % offered.length);
                return;
              }
              if ((event.key === 'Enter' && !event.isComposing) || event.key === 'Tab') {
                event.preventDefault();
                choose(offered[marked] ?? offered[0]!);
                return;
              }
              if (event.key === 'Escape') {
                event.preventDefault();
                event.stopPropagation();
                setAt(null);
                return;
              }
            }
            // Up and Down go back and forth through what the reader sent here, where they are not the box's (`recalled.ts`).
            const area = event.currentTarget;
            const sent = sentHere(thread ? threads.value[thread] : messages.value[channel], me.value?.actor, thread);
            const asWritten = { text: area.value, from: area.selectionStart, to: area.selectionEnd };
            const back = recall(event.key, asWritten, sent, recalls.get(kept) ?? null, { offered: taken.length > 0 || (!!at && offered.length > 0), phone: touched || phone.value });
            if (back) {
              event.preventDefault();
              if (back.recalling) recalls.set(kept, back.recalling);
              else recalls.delete(kept);
              write(area, { text: back.text, from: back.text.length, to: back.text.length });
              fit(area);
              look(area);
              return;
            }
            const tool = toolFor(event);
            if (tool) {
              event.preventDefault();
              use(event.currentTarget, tool);
              return;
            }
            // Written at length, Enter ends a line as it does anywhere else something long is written; with Ctrl or ⌘ held it sends.
            if (wide && event.key === 'Enter' && (event.ctrlKey || event.metaKey) && !event.isComposing) {
              event.preventDefault();
              void send();
              return;
            }
            // A keyboard on glass has no Shift to hold: there Enter ends a line, and what sends is the button.
            if (event.key === 'Enter' && (event.shiftKey || touched || wide) && !event.isComposing) {
              // In a list, the next line is the next item.
              const area = event.currentTarget;
              const next = newline({ text: area.value, from: area.selectionStart, to: area.selectionEnd });
              if (next) {
                event.preventDefault();
                write(area, next);
              }
              return;
            }
            if (event.key === 'Enter' && !event.isComposing) {
              event.preventDefault();
              void send();
            }
          }}
        />
        <div class="composer-foot">
        <button class="ghost icon attach" aria-label="Attach a file" data-tip="Attach a picture, a document, anything. Files can also be dropped here or pasted." onClick={() => picker.current?.click()}>
          <Clip />
        </button>
        {/* Who can be asked here, behind one button: the agents and the bots. */}
        <Asking those={mentionable(others).filter((one) => one.kind === 'agent' || one.kind === 'bot')} onAsk={name} />
        <Formatting area={box} />
        {/* Sending is the last of them, after a rule: beside the other tools, not off by itself at the far edge. */}
        <span class="format-rule" />
        {onWide && (
          <button
            type="button"
            class={classes('ghost icon widen', wide && 'on')}
            aria-label={wide ? 'Write under the channel again' : 'Write in the panel at the right, where there is more room'}
            data-tip={wide ? 'Write under the channel again' : 'Open in the panel, with room to write'}
            aria-pressed={!!wide}
            onClick={onWide}
          >
            <svg width="16" height="16" viewBox="0 0 16 16" aria-hidden="true">
              <path d="M7 3.5H3.5v9h9V9" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round" />
              <path d="M9.5 3.5h3V6.5M12.5 3.5 8 8" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round" />
            </svg>
          </button>
        )}
        {/* Beside Send, while what is written names two or more of the agents who talk things over here. */}
        <DiscussSend channel={channel} text={text} disabled={arriving > 0} onDiscuss={() => void send('discuss')} />
        <button
          class="ghost icon send"
          aria-label="Send"
          data-tip={wide ? 'Send (Ctrl+Enter)' : 'Send (Enter)'}
          disabled={(!text.trim() && attached.length === 0) || arriving > 0}
          // Under a finger the caret stays in the box when this is pressed: taking it would put the keyboard away after every message.
          onMouseDown={touched ? (event) => event.preventDefault() : undefined}
          onClick={() => void send()}
        >
          <svg width="18" height="18" viewBox="0 0 18 18" aria-hidden="true">
            <path d="M2.6 15.2 16 9 2.6 2.8l2 6.2zM4.6 9H10" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round" stroke-linecap="round" />
          </svg>
        </button>
        </div>
        {note && (
          <div class="composer-note faint" role="status">
            {note}
          </div>
        )}
      </div>
    </div>
  );
}

// The tools panel: a second, smaller window beside a channel.
//
// A few tabs of what is worth having to hand while talking. Which tabs
// there are is the reader's (`context.ts`): they are added with the + and
// taken away with the cross on the one showing, and it comes back as it
// was left, in whichever channel it is opened.
//
// A thread is read here too. While one is open it is the first tab, put
// there by opening it and gone when it is closed: it is not one of the
// reader's tabs, and is not kept.

import type { ComponentChildren } from 'preact';
import { useEffect, useMemo, useRef, useState } from 'preact/hooks';
import { api } from '../api.ts';
import type { Attachment, Board, Channel, Member, Message } from '../api.ts';
import { addable, boardsShown, DEFAULT_WIDTH, firstOf, mediaKind, MEMBERS, PANELS, shown } from '../context.ts';
import { held } from '../chosen.ts';
import { workspaces } from '../spaces.ts';
import { coalesce } from '../events.ts';
import { kindOf, lacking, listed } from '../members.ts';
import { phone } from '../phone.ts';
import { Assignee } from './assignee.tsx';
import { agents, collections, contextAt, contextCard, contextFront, contextMember, contextTab, contextTabs, hasBoard, me, openTab, projectName, projects, setContextTabs, setContextWidth, showContextTab, showMember, speaker, working } from '../store.ts';
import { Modal, useEvents } from '../ui.tsx';
import { actorKind, classes, fileSize, initials, when } from '../util.ts';
import { Docs } from './channeldocs.tsx';
import { Edge } from './edge.tsx';
import { drawn } from '../drawer.ts';
import { TOOLS_COMING } from './toolsslide.ts';
import { ViewFrame } from './viewframe.tsx';
import { FullBox, FullButton, useFullScreen } from './fullscreen.tsx';
import { reveal, sideways } from './sideways.ts';
import { MembersDoor, RemoveAgent } from './keptdoor.tsx';
import { ChannelDetails } from './channeldetails.tsx';
import { ChannelSettings } from './channelset.tsx';
import { RunsIn } from './runsin.tsx';
import { runsOn, Standing } from './who.tsx';
import { AgentSeen, atWork, BotSeen, PersonSeen } from './seen.tsx';
import { isExternal, standingOf } from '../external.ts';
import { CardForm, useAbout } from './cardform.tsx';
import { forgetPart, Part, useScope } from './returned.tsx';

interface ContextProps {
  channel: Channel;
  messages: Message[];
  /** The thread that is open in this channel, drawn, or none. */
  thread: ComponentChildren | null;
  onCloseThread: () => void;
  /** The channel's composer, drawn tall, while what is written is written here; or none. */
  writing: ComponentChildren | null;
  /** Puts the composer back under the channel. */
  onCloseWriting: () => void;
  /** Puts the panel away, and any thread open in it. */
  onAway: () => void;
  /** Puts words where the message is being written, for whoever is asked something from here. */
  onSay?: (words: string) => void;
  /** Whether it comes in from the channel's right edge now, or is simply there (`toolsslide.ts`). */
  coming?: boolean;
}

export function Context({ channel, messages, thread, onCloseThread, writing, onCloseWriting, onAway, onSay, coming = false }: ContextProps) {
  const tabs = contextTabs.value;
  // A thread that is opened, or a message written at length, is what is
  // shown, until another tab is pressed: each is a tab of its own while it
  // is there, before the reader's. The last of them to be asked for is in
  // front. Somebody who is read about is under Channel, and is no tab.
  const front = contextFront.value;
  const setFront = (to: 'thread' | 'write' | null) => (contextFront.value = to);
  // Whatever opens a thread, or begins a message at length, says then that
  // it is in front. Nothing here does: a page that is read again has what
  // was in front before, and its thread is only found once its channel has been read.
  const reading = thread !== null && front === 'thread';
  // A message being written at length has the panel to itself while its tab is in front.
  const drafting = writing !== null && front === 'write';
  const tab = reading || drafting ? null : shown(tabs, contextTab.value);
  const more = addable(tabs);
  const [adding, setAdding] = useState(false);
  const box = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (!adding) return;
    const away = (event: Event) => {
      if (event instanceof KeyboardEvent ? event.key === 'Escape' : !box.current?.contains(event.target as Node)) setAdding(false);
    };
    window.addEventListener('pointerdown', away);
    window.addEventListener('keydown', away);
    return () => {
      window.removeEventListener('pointerdown', away);
      window.removeEventListener('keydown', away);
    };
  }, [adding]);
  const named = (id: string) => PANELS.find((panel) => panel.id === id)?.name ?? id;
  // With more tabs than the panel is wide, the one that is showing is kept in sight.
  const row = useRef<HTMLDivElement>(null);
  useEffect(() => reveal(row.current), [reading, drafting, tab, tabs.length]);
  // And as the row is made narrower or wider (the panel dragged, the window or the size of text changed), which the browser says.
  useEffect(() => {
    const at = row.current;
    if (!at || typeof ResizeObserver === 'undefined') return;
    const told = new ResizeObserver(() => reveal(at));
    told.observe(at);
    return () => told.disconnect();
  }, []);
  return (
    // It says when it is where it will be, as the drawer does: at once where nothing slides it in.
    <aside class={coming ? `context ${TOOLS_COMING}` : 'context'} aria-label="Tools" ref={drawn}>
      <Edge what="tools panel" usual={DEFAULT_WIDTH} onWidth={setContextWidth} />
      <div class="context-tabs tabs-ruled">
        {/* On a phone the panel is over the channel, and over the button that opened it: the way back is at its own head. */}
        {phone.value && (
          <button class="ghost icon context-away" aria-label="Back to the channel" onClick={onAway}>
            <svg width="20" height="20" viewBox="0 0 20 20" aria-hidden="true">
              <path d="M12.5 4.5 7 10l5.5 5.5" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" />
            </svg>
          </button>
        )}
        <div role="tablist" aria-label="Tools" class="view-tabs-list" ref={row} onWheel={sideways}>
          {writing !== null && (
            <button role="tab" class="view-tab" aria-selected={drafting} onClick={() => setFront('write')}>
              Composer
              <span
                class="context-drop"
                role="button"
                aria-label="Put the message back under the channel"
                title="Write it under the channel again. What is written is kept."
                onClick={(event) => {
                  event.stopPropagation();
                  onCloseWriting();
                }}
              >
                ×
              </span>
            </button>
          )}
          {thread !== null && (
            <button role="tab" class="view-tab" aria-selected={reading} onClick={() => setFront('thread')}>
              Thread
              <span
                class="context-drop"
                role="button"
                aria-label="Close the thread"
                title="Close the thread"
                onClick={(event) => {
                  event.stopPropagation();
                  onCloseThread();
                }}
              >
                ×
              </span>
            </button>
          )}
          {tabs.map((id) => (
            <button
              key={id}
              role="tab"
              class="view-tab"
              aria-selected={id === tab}
              onClick={() => {
                setFront(null);
                showContextTab(id);
              }}
            >
              {named(id)}
              <span
                class="context-drop"
                role="button"
                aria-label={`Remove the ${named(id)} tab`}
                title="Remove this tab. It can be added again with the +."
                onClick={(event) => {
                  event.stopPropagation();
                  setContextTabs(tabs.filter((one) => one !== id));
                }}
              >
                ×
              </span>
            </button>
          ))}
        </div>
        {more.length > 0 && (
          <div class="menu-anchor" ref={box}>
            <button class="ghost icon" aria-label="Add a tab" title="Add a tab to this panel" aria-haspopup="menu" aria-expanded={adding} onClick={() => setAdding(!adding)}>
              +
            </button>
            {adding && (
              <div class="menu" role="menu">
                {more.map((panel) => (
                  <button
                    key={panel.id}
                    role="menuitem"
                    onClick={() => {
                      setAdding(false);
                      setContextTabs([...tabs, panel.id]);
                      setFront(null);
                      showContextTab(panel.id);
                    }}
                  >
                    <strong>{panel.name}</strong>
                    <span class="muted">{panel.says}</span>
                  </button>
                ))}
              </div>
            )}
          </div>
        )}
      </div>
      {reading && (
        <div class="context-thread" role="tabpanel" aria-label="Thread">
          {thread}
        </div>
      )}
      {drafting && (
        <div class="context-write" role="tabpanel" aria-label="Composer">
          {writing}
        </div>
      )}
      <div class="context-body" role="tabpanel" aria-label={tab ? named(tab) : 'Nothing'} hidden={reading || drafting}>
        {tab === 'media' && <Media messages={messages} />}
        {tab === 'board' && <Plan channel={channel} />}
        {tab === 'docs' && <Docs channel={channel} />}
        {tab === MEMBERS && <Members channel={channel} messages={messages} onSay={onSay} />}
        {tab === null && !reading && !drafting && <p class="faint">This panel has no tabs. Add the ones you want to hand with the +.</p>}
      </div>
    </aside>
  );
}

/** What was attached in the channel, newest first: pictures to look at, pages and files to save. */
function Media({ messages }: { messages: Message[] }) {
  const [opened, setOpened] = useState<Attachment | null>(null);
  // The page being looked at here, in place of the list it was chosen from.
  const [page, setPage] = useState<Attachment | null>(null);
  const full = useFullScreen();
  const all = [...messages].reverse().flatMap((message) => (message.attachments ?? []).map((one) => ({ one, message })));
  const of = (kind: string) => all.filter(({ one }) => mediaKind(one.media) === kind);
  const about = (one: Attachment, message: Message) => (
    <>
      <span class="truncate grow">{one.name}</span>
      <span class="faint">{fileSize(one.size)}</span>
      <span class="faint">{when(message.created)}</span>
    </>
  );
  // A page is looked at here; a file can only be saved.
  const rows = (kind: 'page' | 'file', heading: string) =>
    of(kind).length > 0 && (
      <>
        <h3>{heading}</h3>
        {of(kind).map(({ one, message }) =>
          kind === 'page' ? (
            <button key={one.id} class="context-row" title="Look at it here" onClick={() => setPage(one)}>
              {about(one, message)}
            </button>
          ) : (
            <a key={one.id} class="context-row" href={api.attachmentUrl(one.id)} download={one.name} title="Save a copy of it">
              {about(one, message)}
            </a>
          ),
        )}
      </>
    );
  if (page) {
    return (
      <div class="context-page">
        <div class="row">
          <button class="ghost icon context-back" aria-label="Back to the media" title="Back to the media" onClick={() => setPage(null)}>
            ‹
          </button>
          <strong class="truncate grow">{page.name}</strong>
          <FullButton full={full} class="ghost link" />
          <a href={api.attachmentUrl(page.id)} download={page.name}>
            Save a copy
          </a>
        </div>
        {/* As tall as it asks to be, up to what the panel has; in full screen, the screen. */}
        <FullBox key={page.id} full={full} class="view-box">
          <ViewFrame attachment={page} tallest={Math.max(240, window.innerHeight - 170)} fill={full.on} />
        </FullBox>
      </div>
    );
  }
  if (all.length === 0) return <p class="faint">Nothing has been attached here yet. A picture, a page or a file attached to a message is listed here.</p>;
  return (
    <>
      {of('picture').length > 0 && (
        <>
          <h3>Pictures</h3>
          <div class="context-pictures">
            {of('picture').map(({ one }) => (
              <button key={one.id} title={`${one.name} · ${fileSize(one.size)}`} onClick={() => setOpened(one)}>
                <img src={api.attachmentUrl(one.id)} alt={one.name} loading="lazy" />
              </button>
            ))}
          </div>
        </>
      )}
      {rows('page', 'Pages')}
      {rows('file', 'Files')}
      {opened && (
        <Modal wide onClose={() => setOpened(null)}>
          <div class="picture-opened">
            <img src={api.attachmentUrl(opened.id)} alt={opened.name} />
            <div class="row">
              <strong class="truncate">{opened.name}</strong>
              <span class="faint">{fileSize(opened.size)}</span>
            </div>
          </div>
        </Modal>
      )}
    </>
  );
}

/**
 * Someone named in the panel, pressed to be read about, where everybody
 * of the channel is. Nobody has a card shown over them
 * while they are pointed at: that is for where they are named in what was
 * said, and here it lay over the list it was pointed at from.
 */
function Named({ member, class: name, children }: { member: Member; class?: string; children: ComponentChildren }) {
  // What kind somebody is, which a check reads by: what is said beside them is in words for whoever reads it.
  const kind = kindOf(member.actor);
  // What was said before there was a bot is under no one's name that can be read about.
  if (kind === 'nobody')
    return (
      <span class={classes(name, 'nobody-to-open')} data-kind={kind}>
        {children}
      </span>
    );
  return (
    <button class={name} data-kind={kind} onClick={() => showMember(member.actor)}>
      {children}
    </button>
  );
}

/**
 * Somebody where everybody of the channel is listed, as a card: their
 * picture, the same face drawn beside what they say in the channel, and
 * beside it who they are and what. Pressed, they are read about.
 */
function MemberCard({ member, children }: { member: Member; children: ComponentChildren }) {
  const kind = actorKind(member.actor);
  return (
    <Named member={member} class={classes('context-member', kind, kind === 'agent' && 'context-agent')}>
      <span class={classes('avatar', kind)} aria-hidden="true">
        {kind === 'system' ? '◆' : initials(member.name)}
      </span>
      <span class="context-member-says">{children}</span>
    </Named>
  );
}

/** A person where everybody is listed: who they are, and that they are a person, or whoever reads this. */
function PersonRow({ member }: { member: Member }) {
  return (
    <MemberCard member={member}>
      <span class="row">
        <span class="context-name truncate">{member.name}</span>
        <span class="grow" />
        <span class="faint">{member.actor === me.value?.actor ? 'person · you' : 'person'}</span>
      </span>
    </MemberCard>
  );
}

/** A bot where everybody is listed: the factory's own, or a blueprint's taking part under a name of its own. */
function BotRow({ member }: { member: Member }) {
  const own = speaker.value?.name === member.actor.slice('bot:'.length);
  return (
    <MemberCard member={member}>
      <span class="row">
        <span class="context-name truncate">{member.name}</span>
        <span class="grow" />
        <span class="faint">{own ? "the factory's bot" : 'bot'}</span>
      </span>
    </MemberCard>
  );
}

/**
 * An agent where everybody of the channel is listed: how it stands, what
 * it runs on, what it is at just now and what it may do, at a glance.
 * Pressed, it is read about, as anybody there is.
 */
function AgentRow({ member, slot }: { member: Member; slot: string }) {
  const turn = working.value[slot];
  const kept = agents.value.find((one) => one.name === slot);
  const on = runsOn(slot);
  return (
    <MemberCard member={member}>
      <span class="row">
        <Standing agent={slot} />
        <span class="context-name truncate">{member.name}</span>
        <span class="grow" />
        <span class="faint">{on ? `agent · ${on}` : 'agent'}</span>
      </span>
      <span class={classes('context-doing', turn ? 'working' : 'faint')}>
        {turn ? atWork(turn) : kept?.active === false ? 'not running' : isExternal(kept) ? standingOf(kept!, Date.now()) : 'idle'}
      </span>
      {kept && !isExternal(kept) && <span class="faint">may {kept.permission_mode}</span>}
    </MemberCard>
  );
}

/** The plan: the board's cards by where they stand. The channel's project's board, or the first there is. */
function Plan({ channel }: { channel: Channel }) {
  const boards = projects.value.filter((project) => hasBoard(project.id)).map((project) => project.id);
  // Those of the workspace the side panel is of, and the channel's own.
  const { offered, begins } = boardsShown(boards, channel.project ?? null, held.value, workspaces.value);
  const [picked, setPicked] = useState<string | null>(null);
  // A card that is being looked at is on its own project's board, whichever was showing.
  const asked = contextCard.value;
  const its = asked && boards.find((id) => asked.startsWith(`${id}-`) && /^\d+$/.test(asked.slice(id.length + 1)));
  // What was picked is kept while it is still offered: the panel may be of another workspace since.
  const project = its || (picked && offered.includes(picked) ? picked : null) || begins;
  const [board, setBoard] = useState<Board | null>(null);
  const reload = useMemo(
    () =>
      coalesce(() => {
        if (project) void api.boardIfAny(project).then((found) => setBoard(found?.board ?? null), () => setBoard(null));
      }, 50),
    [project],
  );
  useEffect(reload, [reload, collections.value]);
  useEvents(['board.**', 'collection.ensured'], (event) => event.data?.project === project && reload(), [project]);
  // What the card's form has to know of its board, read here as its page reads it (`cardform.tsx`).
  const about = useAbout(project ?? '', board);
  const scope = useScope();
  // Back at the board: what was typed in its form is let go of, as the drawer's form is with the drawer.
  const back = (id: string) => {
    forgetPart(scope, `card:${id}`);
    contextCard.value = null;
    contextAt.value = null;
  };
  if (!project) return <p class="faint">No project of this workspace has a board yet. A board is made by applying the `std/board` blueprint to a project.</p>;
  if (asked) {
    const item = board?.items.find((one) => one.id === asked);
    return (
      <div class="context-page">
        <div class="row">
          <button class="ghost icon context-back" aria-label="Back to the board" title="Back to the board" onClick={() => back(asked)}>
            ‹
          </button>
          <span class="id faint grow truncate" title={asked}>{asked}</span>
          <button class="ghost" onClick={() => openTab({ kind: 'board', project }, { item: asked })}>
            Open on the board
          </button>
        </div>
        {/* The card's own form, as the drawer beside its board has it: what is typed there is kept with the tab while it is away. */}
        {item && about ? (
          <Part name={`card:${item.id}`}>
            <CardForm key={item.id} item={item} about={about} at={contextAt.value ?? undefined} onChanged={reload} onDone={() => back(item.id)} />
          </Part>
        ) : (
          <p class="faint">{board && !item ? `There is no card ${asked} on this board. It may have been taken away.` : 'Reading the board'}</p>
        )}
      </div>
    );
  }
  return (
    <>
      <div class="row">
        {offered.length > 1 && offered.includes(project) ? (
          <select aria-label="Whose board" value={project} onChange={(event) => setPicked(event.currentTarget.value)}>
            {offered.map((id) => (
              <option key={id} value={id}>
                {projectName(id)}
              </option>
            ))}
          </select>
        ) : (
          <strong class="truncate">{projectName(project)}</strong>
        )}
        <span class="grow" />
        <button class="ghost" onClick={() => openTab({ kind: 'board', project })}>
          Open the board
        </button>
      </div>
      {board?.columns.map((column) => {
        const cards = board.items.filter((item) => item.status === column).sort((a, b) => a.position - b.position);
        // As many of a column's cards as the panel holds, and the rest said, opening the board at that column.
        const { shown: listing, more } = firstOf(cards.length, window.innerHeight, 28);
        return (
          <div key={column} class="plan-column" data-column={column}>
            <h3>
              {column} <span class="faint">{cards.length}</span>
            </h3>
            {cards.slice(0, listing).map((item) => (
              <button key={item.id} class="context-row plan-row" title={item.id} onClick={() => {
                  // Opened as itself, at no comment.
                  contextAt.value = null;
                  contextCard.value = item.id;
                }}>
                <span class="truncate grow">{item.title}</span>
                {item.assignee && <Assignee assignee={item.assignee} brief />}
              </button>
            ))}
            {more > 0 && (
              <button class="ghost link plan-more" onClick={() => openTab({ kind: 'board', project }, { column })}>
                and {more} more
              </button>
            )}
          </div>
        );
      })}
    </>
  );
}

/**
 * Everybody in the channel, people, agents and bots in one list; and,
 * once one of them is pressed (here, or where they are named in what was
 * said), that one, read about, with the way back to the rest over them.
 * It is the one place anybody of a channel is read about.
 */
function Members({ channel, messages, onSay }: { channel: Channel; messages: Message[]; onSay?: (words: string) => void }) {
  const asked = contextMember.value;
  // Somebody of this channel, or an agent wherever it is: anybody else is nobody to read about here.
  // An agent that is no longer there (it was taken away while it was being read about) is said to be gone.
  const member =
    asked === null ? null : (channel.members.find((one) => one.actor === asked) ?? (actorKind(asked) === 'agent' ? { actor: asked, name: asked.slice('agent:'.length) } : null));
  if (member) {
    const kind = actorKind(member.actor);
    return (
      <div class="context-page">
        <nav class="context-crumbs" aria-label="Where this is">
          <button class="ghost link" onClick={() => (contextMember.value = null)}>
            Channel
          </button>
          <span class="faint" aria-hidden="true">
            ›
          </span>
          <span class="truncate" aria-current="page">
            {member.name}
          </span>
        </nav>
        {kind === 'agent' && <RemoveAgent channel={channel} slot={member.actor.slice('agent:'.length)} />}
        {kind === 'agent' && <AgentSeen slot={member.actor.slice('agent:'.length)} />}
        {kind === 'bot' && <BotSeen member={member} messages={messages} onSay={onSay} project={channel.project ?? null} />}
        {kind === 'user' && <PersonSeen channel={channel} member={member} messages={messages} />}
      </div>
    );
  }
  const everybody = listed(channel.members);
  const none = lacking(everybody);
  return (
    <>
      <ChannelDetails channel={channel} />
      <section class="channel-members" aria-label="Members of this channel">
      <h3>Members</h3>
      <div class="context-members">
        {everybody.map((one) =>
          one.slot !== null ? (
            <AgentRow key={one.actor} member={one} slot={one.slot} />
          ) : one.kind === 'person' ? (
            <PersonRow key={one.actor} member={one} />
          ) : one.kind === 'bot' ? (
            <BotRow key={one.actor} member={one} />
          ) : (
            <MemberCard key={one.actor} member={one}>
              <span class="row">
                <span class="context-name truncate">{one.name}</span>
                <span class="grow" />
                <span class="faint">system</span>
              </span>
            </MemberCard>
          ),
        )}
      </div>
      {none && <p class="faint">{none}</p>}
      <MembersDoor channel={channel} />
      </section>
      <ChannelSettings channel={channel} />
      <RunsIn channel={channel} />
    </>
  );
}

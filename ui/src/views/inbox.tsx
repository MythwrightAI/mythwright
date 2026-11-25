// The inbox of whoever is signed in: a place of the rail, directly
// below Resources, with how many lines wait unread on it, which when it
// is pressed makes the side panel the box, as the rail's other places
// make the panel theirs.
//
// A line is who it is from, where it was said and its words; one of a
// conversation, what that is about and who else is in it, with the
// latest thing said there. At the head is what writes one. Pressed,
// it shows its channel at the message, in the thread where it was said
// in one, and has been read; beside it is what puts it away. A tab it
// opens is opened where the reader is, as anything opened from a search
// is: the box is its owner's in every workspace, and moves nothing.
//
// What a blueprint told is from the factory's bot, in the blueprint's
// own words, each web address in them a link of its own (`linked.ts`),
// opened by the browser in a tab of its own. Pressed anywhere but there,
// it has been read and opens a page of its own (`inboxpage.tsx`): who
// told it and when, its words whole, and what it is about, which that
// page opens (a card on its board with its drawer open, a row of another
// table in the drawer over its table, a channel, something running) as
// anything is asked for, so one that is not the reader's to read, or is
// gone, opens nothing. What a line says whole where the pointer rests on
// it is put beside the panel (`tips.ts`).
//
// The list holds still while it is pointed at, or the keyboard is in
// it: a line told meanwhile would come in at the top and push what is
// under the pointer down. What came is counted at the list's head, which
// is always there, and is listed when the pointer has left.
//
// What a line is and what the list is worked out from is `inbox.ts`;
// what reads and follows the box is `inboxed.ts`.

import { signal } from '@preact/signals';
import type { ComponentChildren } from 'preact';
import { useEffect, useRef, useState } from 'preact/hooks';
import { TIP_MORE, asListed, counted, fromOf, headSays, heldStill, inboxCalled, lineCalled, ofConversation, opens, opensItsPage, opensRecord, tipOf, whereOf, wordsOf } from '../inbox.ts';
import type { Line } from '../inbox.ts';
import { inbox, lines, readsInbox, waiting } from '../inboxed.ts';
import { api } from '../api.ts';
import { showReply } from '../topicshown.ts';
import { attempt, openTab, showCard, showSide, side, turn, turned } from '../store.ts';
import { grab } from '../titlebar.tsx';
import { SideFold, Twist } from '../ui.tsx';
import { ago, classes } from '../util.ts';
import { showThread } from './channel.tsx';
import { openRow } from './records.tsx';
import { SpaceFoot } from './chosen.tsx';
import { ComposeMark } from './conversed.tsx';
import { Linked } from './inboxpage.tsx';
import { signs } from './signs.tsx';
import { NothingFound, PanelSeek, typedIn } from './panelseek.tsx';
import { linesFound } from '../panelsought.ts';
import { addressesIn, shortOf } from '../linked.ts';

/**
 * The inbox's place on the rail: how many wait, and, pressed, the box in
 * the side panel; pressed again, the panel put away, as the rail's other
 * places do. Not drawn for whoever has no box, since it would open nothing.
 */
export function InboxRail({ icon }: { icon: ComponentChildren }) {
  if (!readsInbox()) return null;
  const unread = waiting.value;
  const [label, on] = [inboxCalled(unread), side.value === 'inbox'];
  return (
    <button class="rail-button rail-inbox" aria-label={label} aria-pressed={on} data-tip={on ? `${label} (click to hide)` : label} onClick={() => showSide('inbox')}>
      {icon}
      {/* The count lies over the place's corner: nothing on the rail moves as one comes. */}
      <span class="inbox-count" aria-hidden="true">
        {counted(unread)}
      </span>
    </button>
  );
}

/**
 * Shows what a line opens, and says it was read: a told line its own
 * page, which says what it is about and opens that; a line of something
 * said its channel at the message.
 */
function open(line: Line): void {
  if (line.read === null) void inbox.read(line.id).catch(() => undefined);
  if (opensItsPage(line)) return openTab({ kind: 'inbox', id: line.id });
  const record = opensRecord(line);
  if (record) return showComment(record);
  const to = opens(line);
  if (to?.thread) showThread(to.channel, to.thread);
  if (to) openTab({ kind: 'channel', id: to.channel }, { message: to.message });
}

/**
 * Shows a record at a comment on it: a card on its board, with its drawer
 * at the comment; a row of another table in its form, over its table.
 */
function showComment(to: { project: string; table: string; record: string; at: number | null }): void {
  // A forum's topic has a page of its own, where its replies are: a row of the factory's own table.
  // At the reply that named them, or that raised a line of a page they follow; at its head for any other change.
  if (to.project === '@' && to.table === 'topics') return to.at === null ? openTab({ kind: 'topic', id: to.record }) : showReply(to.record, to.at);
  // A card as a card named anywhere is shown: in the Tools panel beside a channel the reader is in, else on its board.
  if (to.table === 'board') return showCard(to.record, to.project, to.at ?? undefined);
  openTab({ kind: 'table', project: to.project, name: to.table });
  void api.record(to.record).then(
    (row) => openRow(to.project, to.table, row, undefined, to.at ?? undefined),
    () => undefined,
  );
}

/** Under a finger, each address of a told line's words again, a finger's size, under the words: one in the words is a word's size. */
function FingerLinks({ words }: { words: string }) {
  const addresses = addressesIn(words);
  if (addresses.length === 0) return null;
  return (
    <span class="inbox-links">
      {addresses.map((address) => (
        <a key={address} class="inbox-link inbox-link-row" href={address} target="_blank" rel="noopener noreferrer" title={address}>
          {signs.out()}
          <span class="truncate">{shortOf(address)}</span>
        </a>
      ))}
    </span>
  );
}

function Row({ line }: { line: Line }) {
  const [where, to] = [whereOf(line), opens(line) ?? opensRecord(line)];
  // What a blueprint told is pressed whatever it is about: it is then read, and its own page opened.
  const told = line.kind === 'told';
  const words = wordsOf(line);
  const head = (
    <>
      <span class="inbox-from truncate">{fromOf(line)}</span>
      <span class="inbox-when">{ago(line.raised)}</span>
      {where && (
        <span class="inbox-where truncate">
          {/* A private channel has its lock where an open one has its `#`, and a conversation its envelope. */}
          {ofConversation(line) ? signs.envelope(11) : line.said?.channel_kept && signs.lock(11)}
          {where}
        </span>
      )}
    </>
  );
  return (
    // Each in a place of its own: a row directly in a panel's list is what a project's tree is found by.
    <div class={classes('inbox-line', line.read === null && 'unread')} data-line={line.id} data-kind={ofConversation(line) ? 'conversation' : line.kind}>
      {told ? (
        // A link may not be inside a button: the line's press lies under everything of the line but its links, which lie over it.
        <div class="inbox-open inbox-told">
          {/* Pointed at, the start of its words beside the box, and that the whole is on its page; read out, the whole. */}
          <button
            class="inbox-press"
            title={tipOf(line).says}
            data-tip-more={tipOf(line).whole ? undefined : TIP_MORE}
            aria-label={lineCalled(line)}
            onClick={() => open(line)}
          />
          {head}
          <span class="inbox-words">
            <Linked words={words} />
          </span>
          <FingerLinks words={words} />
        </div>
      ) : to ? (
        <button
          class="inbox-open"
          title={lineCalled(line)}
          aria-label={lineCalled(line)}
          onClick={() => open(line)}
        >
          {head}
          <span class="inbox-words">{words}</span>
        </button>
      ) : (
        // What opens nothing is read where it is, and is nothing to press.
        <div class="inbox-open inbox-said">
          {head}
          <span class="inbox-words">{words}</span>
        </div>
      )}
      <button class="ghost icon inbox-away" aria-label={`Put away: ${fromOf(line)}, ${words}`} title="Put away" onClick={() => void attempt(() => inbox.away(line.id))}>
        {signs.cross()}
      </button>
    </div>
  );
}

/** Where the inbox's Archived part's fold is kept: folded until somebody unfolds it. */
const ARCHIVED = 'inbox:archived';

// Whether the pointer is on the list. A finger does not point: it presses, and the list is then as it was left.
const pointed = signal(false);

/** The side panel showing the box: its head with what writes a conversation, the lines, and the foot every side panel has. */
export function InboxSide() {
  const all = lines.value;
  const [within, setWithin] = useState(false);
  // A panel that is put away while it is pointed at is pointed at no longer.
  useEffect(() => () => void (pointed.value = false), []);
  // What the list showed when somebody began to point at it, or came into it by keyboard; nothing while nobody is there.
  const was = useRef<string[] | null>(null);
  if (!pointed.value && !within) was.current = null;
  else was.current ??= all.map((line) => line.id);
  const held = was.current ? heldStill(was.current, all) : { lines: all, kept: 0 };
  // What is typed in the box leaves out the lines that do not say it, after the list was held still (`panelseek.tsx`).
  const text = typedIn('inbox');
  const shown = { ...held, lines: linesFound(held.lines, text) };
  const parted = asListed(shown.lines);
  // Folded until it is unfolded; held open while something typed found a line in it.
  const archivedOpen = turned.value.has(ARCHIVED) || (text.trim() !== '' && parted.archived.length > 0);
  return (
    <nav class="sidebar inbox-side" aria-label="Inbox">
      <div class="brand" onMouseDown={grab}>
        <span>Inbox</span>
        <span class="brand-acts">
          <ComposeMark />
          <SideFold class="ghost icon side-fold" />
        </span>
      </div>
      <PanelSeek panel="inbox" />
      {/* Always there, at one height: what waits, what came while the list was held still, and reading all of it. */}
      <div class="inbox-head">
        <span class="inbox-says truncate" role="status" title={headSays(waiting.value, shown.kept)}>
          {headSays(waiting.value, shown.kept)}
        </span>
        <button class="ghost inbox-all" title="Mark all as read" disabled={waiting.value <= 0} onClick={() => void attempt(() => inbox.readIn())}>
          <span class="truncate">Mark all as read</span>
        </button>
      </div>
      <div
        class="inbox-lines"
        // What a line says of itself where the pointer rests on it is put beside the panel, and never over the rail (`tips.ts`).
        data-tip-beside
        onPointerEnter={(event) => {
          if (event.pointerType === 'mouse') pointed.value = true;
        }}
        onPointerLeave={() => (pointed.value = false)}
        onFocusIn={(event) => setWithin((event.target as Element).matches(':focus-visible'))}
        onFocusOut={(event) => !event.currentTarget.contains(event.relatedTarget as Node | null) && setWithin(false)}
      >
        {parted.live.map((line) => (
          <Row key={line.id} line={line} />
        ))}
        {/* A conversation that is archived is listed at the foot, folded, as an archived channel is in the side panel. */}
        {parted.archived.length > 0 && (
          <div class="grouped archived-part inbox-archived">
            <div class="nav-row group-row">
              <button class="nav-twist" aria-label={archivedOpen ? 'Fold Archived' : 'Unfold Archived'} aria-expanded={archivedOpen} onClick={() => turn(ARCHIVED)}>
                <Twist open={archivedOpen} />
              </button>
              <button class="nav-item group-name" title="Archived conversations: read as before, and said nothing in until given back" onClick={() => turn(ARCHIVED)}>
                <span class="glyph">{signs.archive(13)}</span>
                <span class="truncate">Archived</span>
                {!archivedOpen && <span class="hint">{parted.archived.length}</span>}
              </button>
            </div>
            {archivedOpen && parted.archived.map((line) => <Row key={line.id} line={line} />)}
          </div>
        )}
        {all.length > 0 && shown.lines.length === 0 && <NothingFound text={text} />}
        {all.length === 0 && <p class="inbox-none faint">Nothing is here. A message written to you, what names you with @ in a channel or on a card you read, and what the factory tells you, waits here until you put it away.</p>}
      </div>
      <SpaceFoot />
    </nav>
  );
}

// The Members line under Factory in the Resources panel, as the page draws
// it (it was the last branch of the Channels panel, which is for
// talking): who there is in the workspace the panel is of, folded under
// one line as the panel's folds are, whose name opens the Members page
// (`members.tsx`), where each person's line has the same menu. A member's line, pressed, opens a menu
// of what can be done with them, as a row's menu is
// (`RowMenu`): who they are, read about where anybody is
// (`readAbout`); a direct message with them; of an agent what its line on
// the Members page's Agents tab offers, by the one function (`agentActs`); of a person,
// for whoever manages the factory, what they hold (`showPermissions`).
// Nothing whoever reads may not do is offered.
//
// What is listed and what a menu offers is `direct.ts`. A direct message is
// a conversation of exactly two (`conversed.ts`), so the one there is
// opens as any conversation does, as a tab of its own; where there is none
// yet, a line opens a small dialog that asks for what is first said and
// nothing else, since a conversation is written with its first message in
// one asking (`POST /inbox/conversations`), and a direct message needs no
// list of who it is to: it is to the one pressed. Left without a subject,
// the daemon calls it by its first words, as it calls any conversation.
// What the daemon refuses (an agent it will have in no conversation) is
// said in its sentence with the dialog staying as it was written, and what
// was written for somebody is kept while the dialog is away.
//
// The people are read from the daemon's one list of who has an account
// here (`GET /people`, which whoever reads channels may read), once the
// stream is there and again when somebody registers; the agents are the
// store's, followed as they always are.

import { computed, effect, signal } from '@preact/signals';
import { useState } from 'preact/hooks';
import { api, call } from '../api.ts';
import type { Channel, Member, Message } from '../api.ts';
import { held } from '../chosen.ts';
import { mayWrite } from '../conversed.ts';
import { directWith, lineTitle, memberActs, membersListed, waitingAll, waitsIn } from '../direct.ts';
import type { MemberLine } from '../direct.ts';
import { stream } from '../events.ts';
import { FACTORY_FOLDS } from '../folds.ts';
import { lines as inboxLines } from '../inboxed.ts';
import { holds, ofMembers, shows } from '../panelsought.ts';
import type { Found } from '../panelsought.ts';
import { workspaces } from '../spaces.ts';
import { agents, channels, me, openTab, unread } from '../store.ts';
import { Modal, useFocus } from '../ui.tsx';
import { classes } from '../util.ts';
import { Standing } from './who.tsx';
import { signs } from './signs.tsx';
import { agentActs } from './agentacts.tsx';
import { showPermissions } from './given.tsx';
import { Fold } from './navtree.tsx';
import { offer, offeredBy, putAway } from './rowmenu.tsx';
import type { Offered } from './rowmenu.tsx';
import { readAbout } from './seen.tsx';

/** Everybody with an account here, as the daemon last said; nothing until that is read, or where it cannot be. */
const people = signal<Member[]>([]);

let reading = false;
let again = false;
/** Reads who has an account here: asked while a reading is on its way, it is read once more when that is in. */
export async function readPeople(): Promise<void> {
  if (reading) {
    again = true;
    return;
  }
  reading = true;
  try {
    do {
      again = false;
      const who = me.value?.actor;
      if (!who) break;
      const read = await api.people().catch(() => null);
      if (read && me.value?.actor === who) people.value = read;
    } while (again);
  } finally {
    reading = false;
  }
}
stream.onStatus((up) => {
  if (up && me.value) void readPeople();
});
// Somebody registering is said to whoever manages the factory; anybody else has them when the part is next drawn anew.
stream.on('identity.registered', () => void readPeople());
effect(() => {
  if (!me.value) people.value = [];
});

/** The part's lines, as things stand: of the workspace the panel is of. */
export const everybody = computed<MemberLine[]>(() => {
  const who = me.value;
  return membersListed(people.value, agents.value, who ? { actor: who.actor, name: who.principal.name } : null, held.value, workspaces.value);
});

/** Somebody a direct message is being written to, until it is sent or put away. */
const writing = signal<MemberLine | null>(null);
/** What was written to each, kept while the dialog is away. */
const drafts = signal<Record<string, string>>({});

/** What the daemon said, as a page says it: without the marks a terminal reads commands by. */
const said = (refused: unknown) => (refused instanceof Error ? refused.message : String(refused)).replaceAll('`', '');

/** The direct message with somebody, as the page has it, by its id. */
function directOf(actor: string): string | null {
  const who = me.value?.actor;
  return who ? directWith(channels.value, inboxLines.value, who, actor) : null;
}

/** What waits in the direct message with somebody. */
export function waitingWith(actor: string): number {
  const id = directOf(actor);
  return waitsIn(id ? channels.value.find((one) => one.id === id) : null, unread.value, inboxLines.value);
}

/**
 * The Members line under Factory: a line with a page, as a project's is,
 * whose name opens the Members page (`members.tsx`) and whose arrow
 * unfolds everybody under it, as the panel's other folds do, saying
 * folded how many wait in its direct messages; and held open, with only
 * what was found, while the panel's box finds somebody in it.
 */
export function MembersFold({ found }: { found: Found | null }) {
  const lines = everybody.value;
  if (!shows(found, ofMembers.part)) return null;
  const waiting = waitingAll(lines.map((line) => (line.kind === 'you' ? 0 : waitingWith(line.actor))));
  return (
    <div class="members-part">
      <Fold
        at={FACTORY_FOLDS.members}
        label="Members"
        tab={{ kind: 'members' }}
        held={holds(found, ofMembers.part)}
        count={waiting}
        // Read anew as it is unfolded by hand, so that whoever registered since is listed; a box that holds it open asks nothing.
        onOpen={() => void readPeople()} title="Everybody here, the people and the agents, on a page of their own. Its arrow lists them, each pressed for what can be done with them">
        {lines.filter((line) => shows(found, ofMembers.member(line.actor))).map((line) => (
          <One key={line.actor} line={line} />
        ))}
      </Fold>
    </div>
  );
}

/** What somebody's menu offers whoever reads this, in its order (`memberActs`, then an agent's own lines). */
export function offeredFor(line: MemberLine): Offered[] {
  const who = me.value;
  if (!who) return [];
  const scopes = who.principal.scopes;
  const manages = scopes.includes('*') || scopes.includes('admin');
  const direct = directOf(line.actor);
  const items: Offered[] = memberActs(line, direct, mayWrite(scopes), manages).map((act): Offered => {
    if (act === 'details') return { label: 'Details', mark: signs.person(), act: () => readAbout(line.actor, line.name) };
    if (act === 'open') return { label: 'Message', mark: signs.envelope(), hint: 'open', act: () => direct && openTab({ kind: 'channel', id: direct }) };
    if (act === 'write') return { label: 'Message…', mark: signs.envelope(), act: () => (writing.value = line) };
    return { label: 'Permissions…', mark: signs.key(), act: () => showPermissions(line.actor, line.name) };
  });
  const slot = line.slot ? agents.value.find((one) => one.name === line.slot) : undefined;
  // An agent's own lines, set apart from what anybody's menu has: what its line on the Members page's Agents tab offers.
  const theirs = slot ? agentActs(slot) : [];
  return [...items, ...theirs.map((item, at) => (at === 0 ? { ...item, apart: true } : item))];
}

/** One of the line's members: pressed, or Enter or Space on it, the menu of what can be done with them, under the line. */
function One({ line }: { line: MemberLine }) {
  const count = line.kind === 'you' ? 0 : waitingWith(line.actor);
  return (
    <button
      class={classes('nav-item member-line', `member-${line.kind}`)}
      data-actor={line.actor}
      title={lineTitle(line)}
      aria-haspopup="menu"
      aria-label={`${line.name}${line.said ? `, ${line.said}` : ''}${count ? `, ${count} unread` : ''}`}
      onClick={(event) => {
        const from = event.currentTarget;
        // Pressed again, it puts its menu away, and has the keyboard back.
        if (offeredBy(from)) return putAway();
        const at = from.getBoundingClientRect();
        offer({ x: at.left, y: at.bottom + 2 }, offeredFor(line), { from, label: `What can be done with ${line.name}` });
      }}
    >
      <span class="glyph">{line.slot ? <Standing agent={line.slot} /> : <span class="member-dot" aria-hidden="true" />}</span>
      <span class="truncate">{line.name}</span>
      {line.said && <span class="faint member-said">{line.said}</span>}
      {count ? <span class="count">{count}</span> : null}
    </button>
  );
}

/** The dialog that writes a direct message, drawn once in the frame. */
export function AskDirect() {
  const to = writing.value;
  return to ? <Write key={to.actor} to={to} onClose={() => (writing.value = null)} /> : null;
}

/** Writes the first message of a direct message with `to`: what is said, and nothing else. */
function Write({ to, onClose }: { to: MemberLine; onClose: () => void }) {
  const [wrong, setWrong] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const box = useFocus<HTMLTextAreaElement>();
  const text = drafts.value[to.actor] ?? '';
  const keep = (words: string) => (drafts.value = { ...drafts.value, [to.actor]: words });
  const send = async (event?: Event) => {
    event?.preventDefault();
    if (!text.trim() || busy) return;
    setBusy(true);
    setWrong(null);
    try {
      const sent = await call<{ channel: Channel; message: Message }>('POST', '/inbox/conversations', { to: [to.actor], text: text.trim() });
      // It is shown at once: what lists the channels reads it again when it hears of it.
      if (!channels.value.some((one) => one.id === sent.channel.id)) channels.value = [...channels.value, sent.channel];
      const { [to.actor]: _sent, ...rest } = drafts.value;
      drafts.value = rest;
      onClose();
      openTab({ kind: 'channel', id: sent.channel.id });
    } catch (refused) {
      // Nothing was made: what was written stays, to be put right or sent to somebody else.
      setBusy(false);
      setWrong(said(refused));
    }
  };
  return (
    <Modal onClose={onClose}>
      <h2 class="direct-head">
        <span class="kept-lock" aria-hidden="true">
          {signs.envelope()}
        </span>
        <span class="truncate">Message {to.name}</span>
        {to.said && <span class="faint direct-said">{to.said}</span>}
      </h2>
      <form class="stack direct-write" onSubmit={send}>
        <label>
          Message
          <textarea
            ref={box}
            required
            rows={4}
            value={text}
            onInput={(event) => keep(event.currentTarget.value)}
            onKeyDown={(event) => {
              // Enter sends, as it does where a channel is written in; Shift and Enter begins another line.
              if (event.key !== 'Enter' || event.shiftKey || event.isComposing) return;
              event.preventDefault();
              void send();
            }}
          />
        </label>
        <p class="faint conversed-note">Only {to.name} and you will read it. It is a conversation of the two of you, in the inbox of each, and is answered there.</p>
        {wrong && (
          <p role="alert" class="kept-wrong">
            {wrong}
          </p>
        )}
        <div class="row modal-foot" style="justify-content:flex-end">
          <button type="button" onClick={onClose}>
            Cancel
          </button>
          <button class="primary" type="submit" disabled={busy || !text.trim()}>
            Send
          </button>
        </div>
      </form>
    </Modal>
  );
}

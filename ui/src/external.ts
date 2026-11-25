// An agent the factory does not start: a program somebody runs for
// themselves, which works with the factory's own command under a name of
// its own (`agents::external` in the daemon). It is listed with the
// agents, is given cards, and is asked nothing: no turn is ever its.
//
// So what a list says of one is not worked out from turns. It is how it
// stands with the factory (invited, joined, stopped), which the daemon
// says with the agent, and which cards it has in hand, read from the
// board of the project it is listed with: a card given to it that is in
// a column past the first and not the last. This holds nothing of the
// page, so that its tests can read it.

import { covers } from './holds.ts';

/** What the slot of one says runs it: a word no package and no program may be called. */
export const EXTERNAL = 'external';

/** How one stands with the factory, as the daemon says it with the agent. */
export interface Standing {
  joined: boolean;
  invited_until: number | null;
}

/** As much of an agent as this reads. */
export interface Outside {
  name: string;
  adapter: string;
  active: boolean;
  external?: Standing | null;
}

/** Whether an agent is one the factory does not start. */
export function isExternal(slot: { adapter: string } | null | undefined): boolean {
  return slot?.adapter === EXTERNAL;
}

/**
 * What somebody offered to be added to a channel is, in a word or two: a
 * person, an agent, or an agent the factory does not start, which is said
 * so that nobody takes it for one the factory runs.
 */
export function kindSaid(actor: string, slots: readonly { name: string; adapter: string }[]): string {
  if (!actor.startsWith('agent:')) return 'person';
  const name = actor.slice('agent:'.length);
  return isExternal(slots.find((slot) => slot.name === name)) ? 'agent (external)' : 'agent';
}

/** As much of a card, and of its board, as this reads. */
export interface Carded {
  id: string;
  status: string;
  assignee: string | null;
}
export interface Boarded {
  columns: string[];
  items: Carded[];
}

/**
 * The cards given to `name` that are in hand: in a column past the
 * board's first and not its last, which is all that is known of a column
 * here, as where an order's cards are begun. One in the first column
 * nobody has begun and one in the last is done; a board of one column
 * or two has nothing in hand. In the board's own order.
 */
export function inHand<C extends Carded>(board: { columns: string[]; items: C[] } | null | undefined, name: string): C[] {
  if (!board) return [];
  const last = board.columns.length - 1;
  const actor = `agent:${name}`;
  return board.items.filter((card) => {
    const at = board.columns.indexOf(card.status);
    return card.assignee === actor && at > 0 && at < last;
  });
}

/** The cards given to `name` that are not done: in any column but the last. */
export function givenTo<C extends Carded>(board: { columns: string[]; items: C[] } | null | undefined, name: string): C[] {
  if (!board) return [];
  const last = board.columns.length - 1;
  const actor = `agent:${name}`;
  return board.items.filter((card) => card.assignee === actor && board.columns.indexOf(card.status) !== last);
}

/** How long from `now` until `until`, in the words an invitation is said in. */
export function within(until: number, now: number): string {
  const hours = (until - now) / 3_600_000;
  if (hours < 1.5) return 'the next hour';
  if (hours < 47.5) return `the next ${Math.round(hours)} hours`;
  return `the next ${Math.round(hours / 24)} days`;
}

/** How one stands, in a few words: whether it has joined, and whether an invitation waits. */
export function standingOf(slot: Outside, now: number): string {
  if (!slot.active) return 'stopped';
  const until = slot.external?.invited_until ?? null;
  const waits = until !== null && until > now;
  if (slot.external?.joined) return waits ? 'joined · invited again' : 'joined';
  return waits ? 'invited · has yet to join' : 'not joined';
}

/** What a list says an external agent is doing, as it says it of any agent. */
export interface Doing {
  state: 'stopped' | 'working' | 'idle';
  words: string;
}

/**
 * What a list says one is doing: stopped, whatever else is known of it;
 * else at work, with the card it has in hand, and how many more where it
 * has several; else how it stands with the factory. Nothing here says it
 * is idle: what it is at is its own to know, and only a card says.
 */
export function externalDoing(slot: Outside, cards: Carded[], now: number): Doing {
  if (!slot.active) return { state: 'stopped', words: 'stopped' };
  const [first, ...more] = cards;
  if (first) return { state: 'working', words: more.length > 0 ? `working on ${first.id} and ${more.length} more` : `working on ${first.id}` };
  return { state: 'idle', words: standingOf(slot, now) };
}

/** How long an invitation may be said to work, each as the daemon reads it and as it is offered. */
export const LASTS: [string, string][] = [
  ['1h', 'an hour'],
  ['12h', '12 hours'],
  ['1d', 'a day'],
  ['3d', '3 days'],
  ['7d', 'a week'],
  ['14d', '14 days'],
];

/** What a new one is offered to hold: what an agent usually does, as far as whoever invites holds it. Nobody gives what they do not hold. */
export function offeredScopes(usual: string[], mine: string[]): string[] {
  return usual.filter((scope) => covers(mine, scope));
}

/** What a name comes to as the daemon makes one of it: lowercase letters, digits and dashes. */
export function nameOf(typed: string): string {
  return typed
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '');
}

/** What the command that joins is, where its program reaches the daemon at `origin`. */
export function joinCommand(cli: string, origin: string): string {
  return `${cli} agent join --url ${origin}`;
}

/** Something said in the journal, as much of it as this reads. */
export interface Said {
  seq: number;
  ts: number;
  topic: string;
  source?: { actor?: string };
  data: any;
}

/** One thing an agent did, in words, and when. */
export interface Did {
  key: string;
  ts: number;
  words: string;
}

/**
 * What `name` did, of what was read of the journal, newest first as it
 * was handed: the cards it moved, changed, added and spoke on, and what
 * it said in channels, as `channelName` calls them. Only what says it
 * did it: an event of somebody else's about a card of its own is not its
 * doing.
 */
export function didOf(events: Said[], name: string, channelName: (id: string) => string | null): Did[] {
  const actor = `agent:${name}`;
  const text = (value: unknown) => (typeof value === 'string' ? value : '');
  const out: Did[] = [];
  for (const event of events) {
    const data = event.data ?? {};
    const by = event.topic === 'channel.message' ? text(data.author) : (event.source?.actor ?? text(data.by));
    if (by !== actor) continue;
    let words = '';
    if (event.topic === 'board.item.moved') words = `moved ${text(data.id)} from ${text(data.from)} to ${text(data.to)}`;
    else if (event.topic === 'board.item.created') words = `added ${text(data.id)}`;
    else if (event.topic === 'board.item.updated') words = text(data.assignee) === actor && Array.isArray(data.changed) && data.changed.includes('assignee') ? `took ${text(data.id)}` : `changed ${text(data.id)}`;
    else if (event.topic === 'record.commented') words = `said on ${text(data.id)}: ${text(data.body)}`;
    else if (event.topic === 'channel.message') {
      const channel = channelName(text(data.channel));
      words = `said in ${channel ? `#${channel}` : 'a channel'}: ${text(data.body)}`;
    }
    if (words) out.push({ key: `${event.seq} ${event.topic}`, ts: event.ts, words });
  }
  return out;
}

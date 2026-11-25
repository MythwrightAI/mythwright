// The inbox of whoever is signed in, as the page has it: read once the
// stream of events is there, and followed. What a line is, and what
// anything said of the box comes to, is `inbox.ts`.
//
// The count of what waits unread is what the daemon says with each
// thing that changes the box (`inbox.*`), so it is right on a page that
// reads nothing again. The list is a reading: what is said of the box
// carries no words, so a line told is read for, one reading at a time,
// and of what is asked while one is on its way only the last is made.
// A reading that something was said during is not taken for the count,
// which was said after it was taken, and is made once more.
//
// Nothing here is of a workspace: a box is its owner's wherever they are
// looking, and the daemon is asked for no box but that of whoever asks.

import { effect, signal } from '@preact/signals';
import { call } from './api.ts';
import { coalesce, stream } from './events.ts';
import { covers } from './holds.ts';
import { heard, readBySeeing, sentenceOf, titleOf } from './inbox.ts';
import type { Line } from './inbox.ts';
import { me, showSide, showing, side } from './store.ts';
import type { Row } from './table.ts';

const at = (id: string) => encodeURIComponent(id);

export const inbox = {
  list: () => call<{ lines: Line[]; unread: number }>('GET', '/inbox'),
  /** One line by its id, put away or not, for its page: its owner's, and to anybody else one that is not there. */
  one: (line: string) => call<Line>('GET', `/inbox/${at(line)}`),
  /** Has a line that was read wait unread again. */
  unread: (line: string) => call<{ unread: number }>('POST', `/inbox/${at(line)}/unread`, {}),
  read: (line: string) => call<{ unread: number }>('POST', `/inbox/${at(line)}/read`, {}),
  /** Every line that opens `place`, or with none every line there is. */
  readIn: (place?: string) => call<{ read: number; unread: number }>('POST', '/inbox/read', place ? { place } : {}),
  away: (line: string) => call<{ unread: number }>('POST', `/inbox/${at(line)}/away`, {}),
  /** The record a line a blueprint told is about, with whose it is: as whoever asks may read it, like any record. */
  about: (record: string) => call<Row & { project: string; collection: string }>('GET', `/records/${at(record)}`),
};

/** The lines of the box that are not put away, the latest told first. Empty until they are read. */
export const lines = signal<Line[]>([]);
/** How many of them wait unread, as the daemon last said. */
export const waiting = signal(0);
/** Each line a page of its own read last, by id: what its tab is called by where the box no longer lists it. */
export const opened = signal<Record<string, Line>>({});

/** What the tab of a line's page is called: its first words, as the box or its page last read it. */
export function lineTitle(id: string): string {
  const line = lines.value.find((one) => one.id === id) ?? opened.value[id];
  return line ? titleOf(line) : 'Inbox message';
}

/** What the tab of a line's page says where it is pointed at: the line's first sentence whole, as its page is headed. */
export function lineSentence(id: string): string {
  const line = lines.value.find((one) => one.id === id) ?? opened.value[id];
  return line ? sentenceOf(line) : 'Inbox message';
}

/** Whether whoever is signed in reads a box at all: a page signed in with less has no inbox on its rail. */
export function readsInbox(): boolean {
  return covers(me.value?.principal.scopes ?? [], 'inbox:read');
}

let under = false;
let again = false;
/** How many things have been said of the box: a reading that this moved during was taken before the last of them. */
let said = 0;

/** Reads the box. Asked while a reading is on its way, it is read once more when that is in, however often it was asked. */
export async function readInbox(): Promise<void> {
  if (under) {
    again = true;
    return;
  }
  under = true;
  try {
    do {
      again = false;
      const [before, who] = [said, me.value?.actor];
      if (!who || !readsInbox()) break;
      const read = await inbox.list().catch(() => null);
      // Whoever asked has signed out meanwhile, or it could not be read: what is shown is left as it is.
      if (!read || me.value?.actor !== who) break;
      lines.value = read.lines;
      if (said === before) waiting.value = read.unread;
      else again = true;
    } while (again);
  } finally {
    under = false;
  }
}

stream.on('inbox.*', (event) => {
  const who = me.value?.actor;
  if (!who) return;
  said += 1;
  const next = heard(lines.value, waiting.value, who, event, Date.now());
  lines.value = next.lines;
  waiting.value = next.unread;
  if (next.again || under) void readInbox();
});
// A line is passed over while its channel is kept from its owner, and is back when they read it again: neither
// is said of the box, which says nothing of what is not theirs to read, so it is read for when a channel is kept
// or opened, or somebody joins or leaves one. And what waits in a channel is counted by nobody while it is archived;
// and a line says where by what its channel is called, which a channel renamed changes.
const moved = coalesce(() => void readInbox());
stream.on(['channel.kept', 'channel.joined', 'channel.left', 'channel.archived', 'channel.unarchived', 'channel.renamed'], moved);
// And a line of something said on a page of a wiki says it by its path, which a page moved, or one
// above it called something else, changes: read again where the box has such a line.
stream.on(['topic.moved', 'topic.revised'], () => {
  if (lines.peek().some((line) => line.on_record?.table === 'topics')) moved();
});
// Read once the stream is there, so that what is said while it is read is heard; and again whenever the stream
// is back, since what was said while the page could not hear was not heard.
stream.onStatus((up) => {
  if (up && me.value) void readInbox();
});
// Nobody's once nobody is signed in.
effect(() => {
  if (me.value) return;
  lines.value = [];
  waiting.value = 0;
  opened.value = {};
});
// The box is read again as the side panel comes to show it: a reading that failed while the page could not ask
// is not what is shown. And a panel this browser left showing the inbox, for somebody signed in now who has
// none, shows the channels.
effect(() => {
  if (side.value !== 'inbox' || !me.value) return;
  if (readsInbox()) void readInbox();
  else showSide('channels', false);
});

/** The channels being asked to count their lines as read: one asking for each at a time. */
const marking = new Set<string>();
// A channel that has come into view has been read, and so have the lines of the box that open it.
effect(() => {
  const seen = [...showing.value].flatMap((key) => (key?.startsWith('channel:') ? [key.slice('channel:'.length)] : []));
  for (const place of readBySeeing(lines.value, seen)) {
    if (marking.has(place)) continue;
    marking.add(place);
    void inbox
      .readIn(place)
      .catch(() => undefined)
      .finally(() => marking.delete(place));
  }
});

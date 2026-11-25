// Somebody's inbox, as the page says it: the box of things addressed to
// whoever is signed in, with nothing of the page in it.
//
// A line is one thing that was addressed to them: a conversation they
// are in, which each thing somebody else says there raises; a message
// that names them; or something a blueprint told them. A line of
// something said keeps no words: what it says is the message as they
// may read it when the box is read, so a line that is listed says
// something, and one of a channel kept from them since is not listed at
// all. What a blueprint told is the blueprint's own words, from the
// factory's bot, and says what it is about by an id: a card, a record,
// a channel or something running, which is opened as anything is, by
// whoever may read it. Pressed, a told line opens a page of its own (the
// message, who told it and when, and what it is about); a line of
// something said opens its channel at the message. And a line of
// something said on a record that names them (`commented`) keeps no
// words either: it is shown with them from the record's history as they
// may read the record now, and opens it at that comment, a card on its
// board with its drawer, a row of another table in its form. A line of a page
// of a wiki its owner follows (`followed`) is one line a page, raised by each
// change of it: shown with the change that raised it last, from the page's
// history (a reply by its words, anything else by what it did), and opens
// the page, at the reply where that was a reply. What is said of the
// box as it changes (`inbox.told`, `inbox.read`, `inbox.unread`,
// `inbox.away`) carries no words either, and how many lines then wait
// unread: the count is settled from that, and the list from a reading.
// What reads and follows is `inboxed.ts`.

import { partsOf } from './codeblocks.ts';
import { withOf } from './conversed.ts';
import { hashed } from './kept.ts';
import { ago, when } from './util.ts';

/** What a line of something said is shown with: the message, and what its channel is called. */
export interface Said {
  author: string;
  author_name: string;
  body: string;
  /** The message it was said under, where it is part of a thread. */
  thread?: string | null;
  created: number;
  channel_name: string;
  channel_title?: string;
  channel_kept?: boolean;
  project?: string;
  /** Whether its channel is a conversation, and then who is in it. */
  conversation?: boolean;
  /** Whether its channel is archived: the line opens it, and waits unread for nobody. */
  channel_archived?: boolean;
  members?: { actor: string; name: string }[];
}

/** What a line of something said on a record is shown with: the record, and the comment as its history keeps it. */
export interface OnRecord {
  record: string;
  project: string;
  table: string;
  /** What the record is called, where it says. */
  title?: string;
  /** Where it is, where it is a page of a wiki: the titles of the pages above it as they are now, its wiki first. */
  path?: string[];
  who: string;
  body: string;
  /** The comment's place in the record's history. */
  seq: number;
  created: number;
  /** What the line of the record's history is of: `record.commented`, or for a page somebody follows whatever changed it last. */
  topic?: string;
  /** What that change did, in a few words, where it is no comment (`Revised the page`): said in place of words. */
  did?: string;
}

/** One line of the box, as it was read. */
export interface Line {
  id: string;
  owner: string;
  /** `following`: something else followed changed, said in words of its own, or a digest of what did (`why` `digest`). */
  kind: 'named' | 'message' | 'told' | 'commented' | 'followed' | 'following';
  /** What it is about: for somebody named, the message. */
  about: string;
  /** What opens it: a channel's id, or for what a blueprint told the id it said. */
  place: string;
  from: string;
  why: string;
  /** Its own words, for what a blueprint told. */
  says?: string;
  /** What told it last: for a conversation, the latest message. */
  said_by?: string;
  count: number;
  created: number;
  raised: number;
  read: number | null;
  away: number | null;
  said?: Said;
  /** What was said, for a line of something said on a record. */
  on_record?: OnRecord;
  /** What a blueprint told it with beside its words, each a part with a `kind` (`reviewed.ts`): as kept, and read for what each comes to now. */
  parts?: unknown[];
}

/** What is said of a box as it changes, as far as these go by it. */
export interface Changed {
  topic: string;
  data: { owner?: string; line?: string; lines?: string[]; place?: string; unread?: number };
}

/** The most a count is written as: more than that many is said to be more. */
const MOST = 99;

/** How a count is written on the inbox's place on the rail: nothing for none, and no more digits than two. */
export function counted(unread: number): string {
  if (unread <= 0) return '';
  return unread > MOST ? `${MOST}+` : String(unread);
}

/** What the inbox's place on the rail is called, for whoever hears it read out. */
export function inboxCalled(unread: number): string {
  if (unread <= 0) return 'Inbox';
  return `Inbox, ${unread} unread`;
}

/** Whether a line is of a conversation: what it is about is then its subject, and it says who is in it. */
export function ofConversation(line: Line): boolean {
  return line.said?.conversation === true;
}

/** Where a line is of, in a few words: its channel as it is called, with that it is a thread where it is one; a conversation by what it is about and who else is in it. */
export function whereOf(line: Line): string {
  const on = line.on_record;
  if (on) return `on ${[...(on.path ?? []), on.title ?? on.record].join(' › ')}`;
  const said = line.said;
  if (!said) return '';
  if (said.conversation) return `${said.channel_title ?? ''} · with ${withOf(said.members ?? [], line.owner)}`;
  const channel = hashed({ name: said.channel_name, kept: said.channel_kept, title: said.channel_title });
  return said.thread ? `${channel}, in a thread` : channel;
}

/** Who a line is from, by name where the message says one. */
export function fromOf(line: Line): string {
  if (line.kind === 'following') return line.why === 'digest' ? 'Digest' : 'Following';
  return line.said?.author_name ?? line.on_record?.who ?? line.from.slice(line.from.indexOf(':') + 1);
}

/** What a line says: the message's words, or its own; of a change of a page that is no reply, what it did. */
export function wordsOf(line: Line): string {
  return (line.said?.body ?? line.on_record?.did ?? line.on_record?.body ?? line.says ?? '').replace(/\s+/g, ' ').trim();
}

/** What a line is called, whole, for whoever hears it read out or points at it. */
export function lineCalled(line: Line): string {
  const where = whereOf(line);
  const unread = line.read === null ? 'Unread: ' : '';
  const at = line.on_record ? ` ${where}` : where ? ` in ${where}` : '';
  return `${unread}${fromOf(line)}${at}: ${wordsOf(line)}`;
}

/** How much of a told line its tip says, in letters: some six lines of a tip at the usual size. */
export const TIP_LETTERS = 280;

/** What a tip that says only the first of a told line's words says after them, faint. */
export const TIP_MORE = 'The whole of it is on its page, which pressing the line opens.';

/**
 * The first of `words`, at most `most` letters, cut where a word ends and
 * said to go on (`…`); the words whole where they are no longer. A word
 * longer than half of it is cut where it falls, since nothing else would
 * be said.
 */
export function cutAtWord(words: string, most: number): string {
  if (words.length <= most) return words;
  const room = words.slice(0, most);
  const space = room.search(/\s\S*$/);
  const kept = (space > most / 2 ? room.slice(0, space) : room).trimEnd().replace(/[\s,;:.\-–—]+$/, '');
  return `${kept}…`;
}

/**
 * What a told line says where it is pointed at: the start of what it is
 * called (`lineCalled`), cut at a word, and whether that is the whole of
 * it. A told line may be thousands of letters (what a preview checked),
 * and said whole its card was taller than half the window.
 */
export function tipOf(line: Line): { says: string; whole: boolean } {
  const called = lineCalled(line);
  const says = cutAtWord(called, TIP_LETTERS);
  return { says, whole: says === called };
}

/**
 * What pressing a line opens: the channel, at the message. One said
 * under another is shown where that one is, with the thread it is part
 * of open beside it. Nothing, for a line that opens nothing.
 */
export function opens(line: Line): { channel: string; message: string; thread: string | null } | null {
  if (line.kind === 'told' || !line.said) return null;
  const thread = line.said.thread ?? null;
  // A line of a conversation is about the conversation: what it shows is the message that raised it last.
  const said = line.kind === 'message' ? (line.said_by ?? '') : line.about;
  return { channel: line.place, message: thread ?? said, thread };
}

/**
 * What pressing a line of something said on a record opens: the record,
 * a card on its board with its drawer and a row of another table in its
 * form over its table, at the comment (`at`, its place in the record's
 * history). A line of a page somebody follows opens the page, at the
 * reply where a reply raised it last, and otherwise at its head (`at` is
 * nothing). Nothing for any other line, and for one shown without what
 * was said.
 */
export function opensRecord(line: Line): { project: string; table: string; record: string; at: number | null } | null {
  const on = line.kind === 'commented' || line.kind === 'followed' ? line.on_record : undefined;
  if (!on) return null;
  const at = line.kind === 'commented' || on.topic === 'record.commented' ? on.seq : null;
  return { project: on.project, table: on.table, record: on.record, at };
}

/**
 * Whether pressing a line opens a page of the line itself: what a
 * blueprint told does, since it is a message of its own (who told it,
 * when, its words whole and what it is about). A line of something said
 * opens its channel at the message, as a mention does in a chat.
 */
export function opensItsPage(line: Line): boolean {
  // A line of something followed opens a page of its own too: what it is of, and the list of the changes it told of.
  return line.kind === 'told' || line.kind === 'following';
}

/** How many words a line's page is called by, and how many letters at most. */
const TITLE_WORDS = 6;
const TITLE_LETTERS = 40;

/**
 * The words a line's page is called by: a block of code is no words, so
 * they are those before its first block, or after it where there are none
 * before; a piece of code in a sentence is its word without its marks; and
 * the lines are one line.
 */
function titleWords(words: string): string {
  return wordsBefore(words).replace(/\s+/g, ' ').trim();
}

/** The words before a line's first block of code, or after it where none are before, with the marks of code in a sentence taken off and its lines as they were. */
function wordsBefore(words: string): string {
  const parts = partsOf(words);
  const first = parts.findIndex((part) => part.is === 'code');
  const before = first < 0 ? parts : parts.slice(0, first);
  const said = (before.some((part) => part.is === 'words') ? before : parts).find((part) => part.is === 'words')?.text ?? '';
  return said.replace(/`([^`\n]+)`/g, '$1');
}

/** How long a line's first sentence is said at most where its page is headed by it, in letters. */
export const SENTENCE_LETTERS = 200;

/**
 * The first sentence of `words`: up to its first line break, or its first
 * full stop that ends a sentence (followed by a space or nothing: never
 * the dot of an address or a version), the stop kept; at most `most`
 * letters, cut where a word ends with an ellipsis (`cutAtWord`).
 */
export function firstSentence(words: string, most = SENTENCE_LETTERS): string {
  const line = words.trim().split('\n')[0]!.replace(/\s+/g, ' ').trim();
  const stop = /\.(?=\s|$)/.exec(line);
  return cutAtWord(stop ? line.slice(0, stop.index + 1) : line, most);
}

/**
 * What a told line's page is headed by, and its tab's tip says: its first
 * sentence whole, where its tab is called by its
 * first words (`titleOf`). What it is, where it says none.
 */
export function sentenceOf(line: Line): string {
  return firstSentence(wordsBefore(line.said?.body ?? line.on_record?.body ?? line.says ?? '')) || 'Inbox message';
}

/** What a line's page is called: its first few words, with `…` where there are more; what it is where it says none. */
export function titleOf(line: Line): string {
  // Read as it was written, with its lines, where a block of code can be told from words.
  const words = titleWords(line.said?.body ?? line.on_record?.body ?? line.says ?? '');
  if (!words) return 'Inbox message';
  const first = words.split(' ').slice(0, TITLE_WORDS).join(' ');
  const cut = first.length > TITLE_LETTERS ? `${first.slice(0, TITLE_LETTERS - 1).trimEnd()}` : first;
  return cut === words ? words : `${cut}…`;
}

/** When a line was last told, on the clock and from now; and how often it was told, from when, where more than once. */
export function toldWhen(line: Line, now: number): { at: string; ago: string; times: string | null } {
  const times = line.count > 1 ? `Told ${line.count} times, first ${when(line.created, now)}` : null;
  return { at: when(line.raised, now), ago: ago(line.raised, now), times };
}

/** What a line's page offers to be done with it: made to wait unread again or read, and put away. Nothing for one put away. */
export function offeredOf(line: Line): { unread: boolean; read: boolean; away: boolean } {
  if (line.away !== null) return { unread: false, read: false, away: false };
  return { unread: line.read !== null, read: line.read === null, away: true };
}

/** Whether something said of a box is of the line `id`: its page then reads it again. */
export function isOf(id: string, event: Changed): boolean {
  const data = event.data;
  return data.line === id || (data.lines ?? []).includes(id);
}

/** What a line a blueprint told opens. */
export type Thing = { kind: 'channel'; id: string } | { kind: 'running'; id: string } | { kind: 'run'; id: string } | { kind: 'record'; id: string };

/**
 * What pressing a line a blueprint told opens, by the id it says:
 * a channel, where it is one of those the reader has (`channels`, by
 * id); something running, and one run of something, each of which an
 * id says it is by how it begins; and otherwise a record, a card or a row of another table, which is asked
 * for by its id and says whose it is to whoever may read it. Nothing,
 * for a line of something said and for one that is about nothing.
 */
export function thingOf(line: Line, channels: string[]): Thing | null {
  const id = line.kind === 'told' ? line.place.trim() : '';
  if (!id) return null;
  if (channels.includes(id)) return { kind: 'channel', id };
  // As an id of something running is told wherever one is written (`running.ts`): long, with a figure in it.
  if (/^in_(?=[a-z]*\d)[0-9a-z]{10,}$/.test(id)) return { kind: 'running', id };
  // A run is told the same way, beginning as a run's id does.
  if (/^run_(?=[a-z]*\d)[0-9a-z]{10,}$/.test(id)) return { kind: 'run', id };
  return { kind: 'record', id };
}

/** A record a told line is about, as it was read: whose it is, and what it is called where it says. */
export interface AboutRow {
  id: string;
  project: string;
  collection: string;
  data?: Record<string, unknown>;
}

/**
 * What a told line's page says of the record it is about: what the record
 * is called (its title, else its name, else its id) and where it is, a
 * card on a project's board or a row of one of its tables; or, where it
 * could not be read (it is gone, or is not the reader's to read), its id
 * and that, with nothing to open.
 */
export function recordSaid(id: string, row: AboutRow | null, projectName: string | null): { says: string; where: string; opens: boolean } {
  if (!row) return { says: id, where: 'which is gone, or is not yours to read', opens: false };
  const text = (value: unknown) => (typeof value === 'string' && value.trim() ? value.trim() : null);
  const says = text(row.data?.title) ?? text(row.data?.name) ?? row.id;
  const project = projectName ?? row.project;
  const where = row.collection === 'board' ? `card ${row.id} on the ${project} board` : `${row.id}, a row of ${project} ${row.collection}`;
  return { says, where, opens: true };
}

/** The channels that lines waiting unread open, by id, without repeats. */
export function waitingIn(lines: Line[]): string[] {
  return [...new Set(lines.filter((line) => line.read === null && line.away === null).map((line) => line.place))];
}

/** The channels among `shown` that a line waiting unread opens: what coming into view has read. */
export function readBySeeing(lines: Line[], shown: string[]): string[] {
  const waiting = new Set(waitingIn(lines));
  return [...new Set(shown)].filter((channel) => waiting.has(channel));
}

/** The lines as they are once those of `ids` were read at `at`. */
export function afterRead(lines: Line[], ids: string[], at: number): Line[] {
  const read = new Set(ids);
  if (!lines.some((line) => read.has(line.id) && line.read === null)) return lines;
  return lines.map((line) => (read.has(line.id) && line.read === null ? { ...line, read: at } : line));
}

/** The lines as they are once those of `ids` were made to wait unread again. */
export function afterUnread(lines: Line[], ids: string[]): Line[] {
  const again = new Set(ids);
  if (!lines.some((line) => again.has(line.id) && line.read !== null)) return lines;
  return lines.map((line) => (again.has(line.id) && line.read !== null ? { ...line, read: null } : line));
}

/** The lines as they are once those of `ids` were put away, or went with what they opened. */
export function afterAway(lines: Line[], ids: string[]): Line[] {
  const away = new Set(ids);
  return lines.some((line) => away.has(line.id)) ? lines.filter((line) => !away.has(line.id)) : lines;
}

/**
 * What something said of a box comes to, for the list and the count as
 * they stand: the count is what the event says; lines read are read and
 * lines put away are gone; and a line told is not in what is said (it
 * carries no words), so the list is to be read again. Nothing of
 * somebody else's box changes anything: none is ever heard, and one
 * that were is not taken.
 */
export function heard(lines: Line[], unread: number, owner: string, event: Changed, now: number): { lines: Line[]; unread: number; again: boolean } {
  const data = event.data;
  if (data.owner !== owner) return { lines, unread, again: false };
  const counted = typeof data.unread === 'number' ? data.unread : unread;
  switch (event.topic) {
    case 'inbox.told':
      return { lines, unread: counted, again: true };
    case 'inbox.read':
      return { lines: afterRead(lines, data.lines ?? [], now), unread: counted, again: false };
    case 'inbox.unread':
      return { lines: afterUnread(lines, data.lines ?? []), unread: counted, again: false };
    case 'inbox.away':
      return { lines: afterAway(lines, data.lines ?? []), unread: counted, again: false };
    default:
      return { lines, unread, again: false };
  }
}

/**
 * The list as it is drawn while it is pointed at, or the keyboard is in
 * it: what it listed when that began (`was`, by id), in that order, each
 * line as it is now. A line told meanwhile is kept back, so that nothing
 * comes in at the top and pushes what is under the pointer down; only a
 * line that is listed no longer leaves. How many are kept back is said
 * with it, for the list's head to say.
 */
export function heldStill(was: string[], lines: Line[]): { lines: Line[]; kept: number } {
  const by = new Map(lines.map((line) => [line.id, line]));
  const held = was.flatMap((id) => by.get(id) ?? []);
  return { lines: held, kept: lines.length - held.length };
}

/** What the list's head says of it: how many wait, and how many came while it was held still. */
export function headSays(unread: number, kept: number): string {
  const waits = unread <= 0 ? 'Nothing unread' : `${unread} unread`;
  return kept > 0 ? `${waits} · ${kept} new` : waits;
}

/**
 * The lines as the box lists them: those of a conversation that is
 * archived under the Archived part at its foot, and every other where it
 * always was, each in the order it was given.
 */
export function asListed(lines: Line[]): { live: Line[]; archived: Line[] } {
  const archived = (line: Line) => line.said?.conversation === true && line.said.channel_archived === true;
  return { live: lines.filter((line) => !archived(line)), archived: lines.filter(archived) };
}

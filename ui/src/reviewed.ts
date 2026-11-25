// What a line a blueprint told shows beside its words: its parts, as the
// daemon says each comes to now (`GET /api/inbox/<line>/parts`,
// `kernel::parts`), and what a card such a part lists offers whoever
// reads it. Nothing here asks anything or holds anything of the page,
// so that its tests can read it: what is offered is what the daemon
// says may be done, and what a press is refused for is its sentence.

import type { ChangePart } from './asked.ts';
import type { GoPart } from './goasked.ts';
import type { MeasuresPart, RowsPart, RunsPart } from './toldread.ts';

/** Something said on a card since it came where its part lists it. */
export interface Said {
  who: string;
  actor: string;
  words: string;
  ts: number;
  /** `record.commented`, or `card.asked` for what its agent asked. */
  topic: string;
}

/** A card a part lists, as the daemon says it is now. */
export interface Card {
  id: string;
  title: string;
  status: string;
  assignee: string | null;
  labels: string[];
  thread: Said[];
  /** Whether whoever reads may pass or fail it, and why not where they may not. */
  may: boolean;
  why_not?: string;
  /** A card the part kept that is on its board no longer: deleted, or in the Trash. */
  gone?: boolean;
}

export interface CardsPart {
  kind: 'cards';
  project: string;
  column: string;
  label: string;
  fail_to: string;
  pass_to: string;
  /** What of its column is pressed on a card that passes; nothing (or not said, by a daemon from before) where nothing is. */
  pass_press?: string;
  reason: boolean;
  /** Whether it kept the cards its column held as it was told, and lists those wherever each is now; a line from before, and a part of a label, list what is there now. */
  kept?: boolean;
  /** The cards it lists; nothing where the reader may not read the board. */
  cards?: Card[];
  /** Why the reader is shown none of them. */
  refused?: string;
}

export interface LinksPart {
  kind: 'links';
  links: string[];
}

/** One message of a channel, opened where it was said: what a discussion between agents came to. */
export interface MessagePart {
  kind: 'message';
  channel: string;
  message: string;
  /** The message its thread hangs from, where it is in one. */
  thread?: string | null;
  /** Why the reader is shown nothing of it. */
  refused?: string;
}

/** A part as the daemon hands it, with the tab of the line's page it is in where it says one (`toldread.ts`). */
export type Part = (CardsPart | LinksPart | MessagePart | ChangePart | GoPart | RowsPart | RunsPart | MeasuresPart) & { tab?: string };

/** The parts a reading said, those of a kind this page draws; any other is passed over. */
export function partsOf(read: { parts?: unknown } | null | undefined): Part[] {
  const listed = Array.isArray(read?.parts) ? (read.parts as { kind?: unknown }[]) : [];
  const drawn = ['links', 'cards', 'message', 'change', 'go', 'rows', 'runs', 'measures'];
  return listed.filter((part): part is Part => typeof part?.kind === 'string' && drawn.includes(part.kind));
}

/** What a part of cards is headed with: which cards, and where those passed and failed go. */
export function headOf(part: CardsPart): { title: string; goes: string } {
  const title = !part.column ? `Cards labelled ${part.label}` : part.kept ? `Cards that were in ${part.column}` : `Cards in ${part.column}`;
  return { title, goes: goesOf(part) };
}

/** What pressing does, by what the part says: where a card passed goes, or what is pressed on it, or that it stays; and where one failed goes. */
export function goesOf(part: Pick<CardsPart, 'pass_to' | 'pass_press' | 'fail_to' | 'reason'>): string {
  const passed = part.pass_to ? `Passed are approved and go to ${part.pass_to}` : part.pass_press ? `Passed are approved and have ${part.pass_press} pressed` : 'Passed are approved and stay';
  const failed = `failed go back to ${part.fail_to}${part.reason ? ', with why' : ''}`;
  return `${passed}; ${failed}.`;
}

/** Where a card the part lists is, where that is not where the part says (one it kept that moved on, or is gone): said where Pass and Fail would be. */
export function whereNow(part: CardsPart, card: Card): string | null {
  if (card.gone) return 'no longer on the board';
  return part.column && card.status !== part.column ? `in ${card.status}` : null;
}

/** What a part that lists no card says. */
export function noneListed(part: CardsPart): string {
  return part.kept ? `No cards were in ${part.column} when this was told.` : 'No cards are there now.';
}

/** What a card's thread says where nothing was said: since it came where the part lists it, or at all. */
export function nothingSaid(part: CardsPart): string {
  return part.column ? `Nothing was said on it since it came to ${part.column}.` : 'Nothing was said on it.';
}

/** The last thing said on a card, as a row shows it, or nothing where nothing was. */
export function lastSaid(card: Card): { who: string; words: string } | null {
  const last = card.thread.at(-1);
  return last ? { who: last.who || last.actor, words: last.words } : null;
}

/** How many things were said on a card, as its thread's button says it. */
export function threadCalled(card: Card): string {
  const count = card.thread.length;
  return count === 0 ? 'Discuss' : count === 1 ? '1 comment' : `${count} comments`;
}

/** Why Fail is closed with what is written as why, or nothing where it may be pressed. */
export function failClosed(part: CardsPart, reason: string): string | null {
  return part.reason && reason.trim() === '' ? 'Say why it fails: that is what its agent is told when it next has it.' : null;
}

/** What a card is offered to whoever reads it: Pass and Fail, or neither and why. */
export function offeredFor(card: Card): { press: boolean; why: string | null } {
  return card.may ? { press: true, why: null } : { press: false, why: card.why_not ?? null };
}

/** What a press came to, said once the daemon has answered. */
export function doneSaid(card: string, did: { moved?: string; pressed?: string; said?: string | null }, verdict: 'pass' | 'fail'): string {
  if (verdict === 'fail') return `${card} failed: moved to ${did.moved ?? 'where its part says'}.`;
  if (did.moved) return `${card} is approved, and moved to ${did.moved}.`;
  if (did.pressed) return `${card} is approved, and ${did.pressed} is pressed on it.`;
  return `${card} is approved.`;
}

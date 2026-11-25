// An agent's history: the times it was asked, and what they come to.
//
// The daemon hands an agent's latest turns, newest first, each with how
// it stands, where and when it was asked, what its program says it read
// and wrote in tokens, and what it cost (`GET /api/agents/{name}/turns`).
// One asked in a channel kept to its members is handed to somebody who
// does not read that channel without its words (`kept_from`): when, how
// long and what it cost, and nothing of what was said. This is what a
// page says of those: what the list comes to, counted and never scored;
// the days it falls in; how each turn ended and where it was asked; and
// how long a bar is drawn for each turn's tokens. It holds nothing of the
// page, so that its tests can read it.

import { heldIn } from './heldin.ts';
import { amount } from './util.ts';
import { dateIn, dayIn, worn } from './clock.ts';

/** One time an agent was asked, as the daemon hands it. */
export interface Turn {
  id: string;
  slot: string;
  kind: string;
  /** The conversation it is part of: for a card, `lane:<card>`, `plan:<card>` or `review:<card>:<seat>`. */
  thread?: string;
  /** The whole of what its agent was sent. */
  prompt: string;
  /** The words that asked for it, apart from the rest of what was sent: what the message it answers said. Nothing, for a turn no message asked for, and from a daemon from before. */
  asked?: string;
  channel: string | null;
  status: string;
  reply: string;
  error: string | null;
  decision: string | null;
  cost_usd: number;
  /** Whether its program said what it cost: false where it said nothing of it, which is no cost of nought; nothing from a daemon from before. */
  cost_said?: boolean;
  input_tokens: number;
  output_tokens: number;
  created: number;
  started: number | null;
  finished: number | null;
  /** That its words were left out, for somebody who does not read the kept channel it was asked in. */
  kept_from?: boolean;
}

/** How many of an agent's turns the daemon hands: its latest so many. A list as long as this may not be all there is. */
export const LATEST = 50;

/** How a turn stands: it went well, it failed, somebody called it off, or it is still in hand (waiting, at work, or answered and not yet said to have ended). */
export type Ended = 'done' | 'failed' | 'called off' | 'in hand';

// A turn's row keeps why it did not go well and not that somebody called it off, so that is read from the daemon's two sentences for it.
const CALLED_OFF = ['the turn was cancelled', 'cancelled before it started'];

export function endedAs(turn: Pick<Turn, 'status' | 'error'>): Ended {
  if (turn.status === 'done') return 'done';
  if (turn.status !== 'failed') return 'in hand';
  return CALLED_OFF.some((words) => (turn.error ?? '').includes(words)) ? 'called off' : 'failed';
}

/** What is said where a turn's place would be, of one whose words are kept from whoever looks. */
export const KEPT = 'asked in a channel kept to its members';

const DID = { works: 'worked', plans: 'planned', reviews: 'reviewed' } as const;

/**
 * Where a turn was asked, in a few words: the card its conversation
 * names, with what was done with it; else the channel it answers in, as
 * `channelName` calls it; else what kind of turn it was.
 */
export function whereAsked(turn: Pick<Turn, 'kind' | 'thread' | 'channel' | 'kept_from'>, channelName: (id: string) => string | null): string {
  if (turn.kept_from) return KEPT;
  const held = heldIn(turn.thread);
  if (held) return `${DID[held.how]} ${held.card}`;
  const channel = turn.channel ? channelName(turn.channel) : null;
  return channel ? `#${channel}` : turn.kind || 'asked';
}

/** What a list of turns comes to. Counted: nothing here says whether any of it is good. */
export interface Totals {
  asked: number;
  failed: number;
  calledOff: number;
  inHand: number;
  /** Tokens sent to its program, and written by it. */
  sent: number;
  written: number;
  cost: number;
  /** How many of them said nothing of what they cost. */
  costUnsaid: number;
}

/** Whether a turn that has ended said nothing of what it cost: one in hand has not said yet. */
export function costUnreported(turn: { cost_said?: boolean; finished?: number | null }): boolean {
  return turn.cost_said === false && turn.finished !== null;
}

export function totals(turns: Turn[]): Totals {
  const all: Totals = { asked: turns.length, failed: 0, calledOff: 0, inHand: 0, sent: 0, written: 0, cost: 0, costUnsaid: 0 };
  for (const turn of turns) {
    const how = endedAs(turn);
    if (how === 'failed') all.failed += 1;
    else if (how === 'called off') all.calledOff += 1;
    else if (how === 'in hand') all.inHand += 1;
    all.sent += tokensOf(turn.input_tokens);
    all.written += tokensOf(turn.output_tokens);
    if (costUnreported(turn)) all.costUnsaid += 1;
    else all.cost += Number.isFinite(turn.cost_usd) && turn.cost_usd > 0 ? turn.cost_usd : 0;
  }
  return all;
}

function tokensOf(count: number): number {
  return Number.isFinite(count) && count > 0 ? Math.round(count) : 0;
}

/** A count of tokens in a few letters: `980`, `12.4k`, `340k`, `1.2M`. */
export function short(count: number): string {
  const tokens = tokensOf(count);
  if (tokens < 1000) return String(tokens);
  for (const [size, letter] of [
    [1000, 'k'],
    [1_000_000, 'M'],
    [1_000_000_000, 'B'],
  ] as const) {
    const value = tokens / size;
    if (value < 99.95) return `${value.toFixed(1)}${letter}`;
    if (value < 999.5) return `${Math.round(value)}${letter}`;
  }
  return `${Math.round(tokens / 1_000_000_000)}B`;
}

/** The least a bar is drawn at where its turn says any tokens: enough to be seen. */
export const LEAST = 2;

/** A turn's bar: how long it is in all, and how much of that is what was sent to it and what it wrote. */
export interface Bar {
  whole: number;
  sent: number;
  written: number;
}

/**
 * How long each turn's bar is, in the order the turns were given, for a
 * track `track` long: a turn's tokens, sent and written together, against
 * the largest turn of the list, which has the whole track. Lengths are
 * in proportion and nothing else (twice the tokens is twice the bar), so
 * that one bar is read against another; a turn far smaller than the
 * largest is drawn at the least a bar can be, and its figure beside it
 * says the rest. A turn that says no tokens has no bar. Of a bar, each of
 * its two parts that has anything is at least one of the track's units.
 */
export function lengths(turns: Pick<Turn, 'input_tokens' | 'output_tokens'>[], track: number): (Bar | null)[] {
  const sizes = turns.map((turn) => [tokensOf(turn.input_tokens), tokensOf(turn.output_tokens)] as const);
  const most = Math.max(0, ...sizes.map(([sent, written]) => sent + written));
  return sizes.map(([sent, written]) => {
    const all = sent + written;
    if (all === 0 || most === 0) return null;
    const whole = Math.max(LEAST, Math.round((track * all) / most));
    let first = Math.round((whole * sent) / all);
    if (sent > 0 && first === 0) first = 1;
    if (written > 0 && first === whole) first = whole - 1;
    return { whole, sent: first, written: whole - first };
  });
}

/** The turns of one day, under what the day is called. */
export interface Day<T> {
  day: string;
  turns: T[];
}

/**
 * The turns by the day each was asked, in the order they were given:
 * `Today`, `Yesterday`, then the date, with its year where that is not
 * this one. A day is whoever looks's own, in their zone (`clock.ts`).
 */
export function byDay<T extends { created: number }>(turns: T[], now = Date.now(), zone = worn.value.zone): Day<T>[] {
  const today = dayIn(now, zone);
  const called = (ms: number) => {
    const days = today - dayIn(ms, zone);
    if (days <= 0) return 'Today';
    if (days === 1) return 'Yesterday';
    return dateIn(ms, zone, now);
  };
  const days: Day<T>[] = [];
  for (const turn of turns) {
    const day = called(turn.created);
    const last = days[days.length - 1];
    if (last?.day === day) last.turns.push(turn);
    else days.push({ day, turns: [turn] });
  }
  return days;
}

/** A turn's figures as they are, each with what it is of: what was sent to its program, what it wrote, and what it cost where it cost anything, or that its program did not report it. */
export function exactly(turn: Pick<Turn, 'input_tokens' | 'output_tokens' | 'cost_usd' | 'cost_said'> & { finished?: number | null }): [string, string][] {
  const tokens = (count: number) => (tokensOf(count) === 0 ? 'no tokens said' : `${tokensOf(count).toLocaleString('en-US')} ${tokensOf(count) === 1 ? 'token' : 'tokens'}`);
  const figures: [string, string][] = [
    ['Sent to it', tokens(turn.input_tokens)],
    ['Written by it', tokens(turn.output_tokens)],
  ];
  if (costUnreported(turn)) figures.push(['Cost', 'not reported']);
  else if (turn.cost_usd > 0) figures.push(['Cost', amount(turn.cost_usd, 'usd')]);
  return figures;
}

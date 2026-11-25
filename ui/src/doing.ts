// What an agent is doing now, in a few words.
//
// The daemon says when an agent's turn begins and when it ends
// (`agent.turn.started`, `agent.turn.finished`), each with which turn it
// is, where it was asked and which conversation it is part of. From the
// latest of those read once, and each one heard after, this works out
// which turn every agent has in hand and when each last ended one; and
// from that, with whether the agent is stopped, the words a list says of
// it. A turn asked in a channel kept to its members is said to nobody
// outside it, so to them such an agent reads as idle, which is what they
// may know. This holds nothing of the page, so that its tests can read it.

import { heldIn } from './heldin.ts';
import { ago } from './util.ts';

/** A turn an agent has in hand: which, the channel it answers in, and the conversation it is part of. */
export interface InHand {
  turn: string;
  channel: string | null;
  thread: string | null;
}

/** Which turn each agent has in hand, and when each last ended one, by the agent's name. */
export interface TurnsNow {
  going: Record<string, InHand>;
  ended: Record<string, number>;
}

/** What is said of a turn, as much of an event as this reads. */
export interface SaidOfTurn {
  seq: number;
  ts: number;
  topic: string;
  data: { slot?: unknown; turn?: unknown; channel?: unknown; thread?: unknown } | null | undefined;
}

export const NOTHING: TurnsNow = { going: {}, ended: {} };

/** How things stand once `event` is heard. Anything that is not the beginning or the end of an agent's turn changes nothing. */
export function heard(now: TurnsNow, event: SaidOfTurn): TurnsNow {
  const slot = typeof event.data?.slot === 'string' ? event.data.slot : '';
  if (!slot) return now;
  const turn = typeof event.data?.turn === 'string' ? event.data.turn : '';
  if (event.topic === 'agent.turn.started') {
    const text = (value: unknown) => (typeof value === 'string' && value ? value : null);
    return { ...now, going: { ...now.going, [slot]: { turn, channel: text(event.data?.channel), thread: text(event.data?.thread) } } };
  }
  if (event.topic !== 'agent.turn.finished') return now;
  const ended = { ...now.ended, [slot]: Math.max(now.ended[slot] ?? 0, event.ts) };
  // The end of another turn than the one in hand (one called off before it began, behind the one at work) leaves that one in hand.
  const has = now.going[slot];
  if (!has || (has.turn && turn && has.turn !== turn)) return { ...now, ended };
  const { [slot]: _over, ...going } = now.going;
  return { going, ended };
}

/**
 * How things stand from a reading of the latest such events, newest
 * first as the daemon hands them, and whatever arrived while the reading
 * was on its way, in the order it arrived: what arrived is put after
 * what was read, but for what the reading already holds.
 */
export function turnsNow(read: SaidOfTurn[], arrived: SaidOfTurn[] = []): TurnsNow {
  const key = (event: SaidOfTurn) => `${event.seq} ${event.topic}`;
  const held = new Set(read.map(key));
  const all = [...read].reverse().concat(arrived.filter((event) => !held.has(key(event))));
  return all.reduce(heard, NOTHING);
}

/** How an agent stands, and the words for it. */
export interface Doing {
  state: 'stopped' | 'working' | 'idle';
  words: string;
}

const HOW = { works: 'working on', plans: 'planning', reviews: 'reviewing' } as const;

/**
 * What a list says an agent is doing: stopped, whatever else is known of
 * it; else at work, with the card its turn's conversation names or the
 * channel it answers in, as `channelName` calls it; else idle, with how
 * long ago it last ended a turn where it is known to have ended one.
 */
export function doingNow(active: boolean, turn: Pick<InHand, 'channel' | 'thread'> | undefined, ended: number | undefined, now: number, channelName: (id: string) => string | null): Doing {
  if (!active) return { state: 'stopped', words: 'stopped' };
  if (turn) {
    const held = heldIn(turn.thread);
    if (held) return { state: 'working', words: `${HOW[held.how]} ${held.card}` };
    const channel = turn.channel ? channelName(turn.channel) : null;
    return { state: 'working', words: channel ? `working in #${channel}` : 'working' };
  }
  return { state: 'idle', words: ended === undefined ? 'idle' : `idle · ${ago(ended, now)}` };
}

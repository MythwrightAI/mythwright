// Who is writing in a channel, or in a thread of it, just now, and the line
// that says so over where one writes there; and, apart from it, who is at
// work on a card with what it does said there.
//
// An agent is writing in a channel only while it is in the middle of a
// turn whose answer is said there: one a message there asked for, or one
// that goes on with the channel's conversation or a thread's (`channel:`,
// `thread:`). A turn on a card names the channel its fleet reports in,
// and its agent is at work and not writing there: said as writing, a
// worker was typing under its fleet's channel for the whole of a card.
// Such a turn is said in a quiet line of its own,
// and never in the line of who is typing.

import { heldIn } from './heldin.ts';

type Working = Record<string, { channel: string | null; thread?: string | null; kind?: string | null }>;

/** Whether a turn is in a thread: a thread is a conversation of its own (`thread:<root>`), and a turn there says so. */
const inAThread = (turn: { thread?: string | null }) => turn.thread?.startsWith('thread:') ?? false;

/** Whether a turn's answer is said where it was asked: asked by a message, or going on with a channel's or a thread's conversation. */
export function writesThere(turn: { thread?: string | null; kind?: string | null }): boolean {
  return turn.kind === 'message' || /^(?:channel|thread):/.test(turn.thread ?? '');
}

/**
 * The agents writing in `channel` itself, by name, in the order of their
 * names. One whose turn is in a thread of it is said there and not here,
 * as Slack has it: an agent is said once, where it writes.
 */
export function typingIn(working: Working, channel: string): string[] {
  return Object.keys(working)
    .filter((name) => working[name]!.channel === channel && writesThere(working[name]!) && !inAThread(working[name]!))
    .sort();
}

/** The agents writing in the thread under the message `root` of `channel`, by name, in the order of their names. */
export function typingUnder(working: Working, channel: string, root: string): string[] {
  return Object.keys(working)
    .filter((name) => working[name]!.channel === channel && working[name]!.thread === `thread:${root}`)
    .sort();
}

/** An agent at work on something that names `channel` and not writing there: the card its turn is about, if one, and what it does with it. */
export interface AtWork {
  name: string;
  card: string | null;
  words: string;
}

const HOW = { works: 'working on', plans: 'planning', reviews: 'reviewing' } as const;

/** The agents at work for `channel` and not writing in it, by name, each with what it is doing in the words the Agents tab says it in. */
export function workingIn(working: Working, channel: string): AtWork[] {
  return Object.keys(working)
    .filter((name) => working[name]!.channel === channel && !writesThere(working[name]!))
    .sort()
    .map((name) => {
      const held = heldIn(working[name]!.thread);
      return { name, card: held?.card ?? null, words: held ? `${name} is ${HOW[held.how]} ${held.card}` : `${name} is working` };
    });
}

/** The project a card is of, among those there are: the longest whose name and a dash begin the card's id and a number ends it. */
export function projectOfCard(card: string, projects: string[]): string | null {
  const of = projects.filter((id) => card.startsWith(`${id}-`) && /^\d+$/.test(card.slice(id.length + 1)));
  return of.sort((a, b) => b.length - a.length)[0] ?? null;
}

/** The line: everyone by name up to three, and beyond that the first two and how many more. Nothing when nobody is. */
export function typingLine(names: string[]): string | null {
  const [first, second, third] = names;
  if (first === undefined) return null;
  if (second === undefined) return `${first} is typing…`;
  if (third === undefined) return `${first} and ${second} are typing…`;
  if (names.length === 3) return `${first}, ${second} and ${third} are typing…`;
  return `${first}, ${second} and ${names.length - 2} others are typing…`;
}

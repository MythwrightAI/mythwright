// What is said before an agent is taken away for good, and who is asked.
//
// The daemon says what would go with it and what would stay, counted
// (`Removal`), and why it cannot be where it cannot. This is those said
// as lines for somebody to read through before they say yes. It holds
// nothing of the page, so that its tests can read it.

import { covers } from './holds.ts';

/** What taking an agent away came to, or would, as the daemon answers it: what goes with it, and what stays under its name. */
export interface Removal {
  name: string;
  project: string | null;
  removed: boolean;
  /** Why it cannot be taken away as things stand, in the daemon's words. */
  refused: string | null;
  active: boolean;
  /** The channels it is in, by name. */
  channels: string[];
  /** How many pieces of work it was asked. */
  turns: number;
  /** The cards given to it that whoever asks may read. */
  cards: string[];
  /** Why the folder that was its own is still there, where it could not be taken away. */
  folder_left: string | null;
}

/** Whether whoever is signed in is offered taking an agent away: a person who may change agents and manages the factory. The daemon holds them to the same. */
export function mayTakeAway(who: { kind: string; scopes: string[] }): boolean {
  return who.kind === 'user' && covers(who.scopes, 'agents:write') && covers(who.scopes, 'admin');
}

/** `#a`, `#a and #b`, `#a, #b and #c`. */
function channelsSaid(names: string[]): string {
  const said = names.map((name) => `#${name}`);
  return said.length <= 1 ? said.join('') : `${said.slice(0, -1).join(', ')} and ${said[said.length - 1]}`;
}

/** What goes with it, a line for each thing. */
export function goes(found: Removal): string[] {
  const lines = ['Its slot: what it was told, what it may do and what it holds.', 'Its credential, which is nobody’s from then on.'];
  if (found.channels.length > 0) lines.push(`Its place in ${channelsSaid(found.channels)}.`);
  lines.push(
    found.project
      ? 'The folder that was its own on this device: the home its program was shown.'
      : 'The folder that was its own on this device: the home its program was shown and, since it works in no project, whatever it wrote there.',
  );
  return lines;
}

/** What stays, under its name, a line for each thing. */
export function stays(found: Removal): string[] {
  const lines = ['What it said in channels, and the history of what it changed.'];
  if (found.turns === 1) lines.push('The one piece of work it was asked, with what it answered.');
  if (found.turns > 1) lines.push(`The ${found.turns} pieces of work it was asked, with what it answered.`);
  if (found.cards.length === 1) lines.push(`${found.cards[0]} is given to it, and stays so until somebody gives it to another.`);
  if (found.cards.length > 1) lines.push(`${found.cards.length} cards are given to it (${found.cards.join(', ')}), and stay so until somebody gives them to another.`);
  return lines;
}

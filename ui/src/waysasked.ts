// What somebody has written of a comparison they have not started yet.
//
// It is asked in the drawer, and the drawer is put away when another tab
// is shown: what was typed is kept here, by the blueprint and the project
// it is of, and is there again when the drawer is opened. It is let go
// when the comparison is started, and not kept across a reload of the
// page.

import type { Written, WrittenRule } from './compared.ts';

/** What is asked before a comparison is started, as it is written. */
export interface Asking {
  name: string;
  /** The param that says which thing a start is for, or nothing: each place a step starts it. */
  per: string;
  /** Other blueprints counted with a thing's way, as they were typed. */
  also: string;
  /** What is counted of what comes of each thing: nothing, the usual ones for a board's cards, or events said here. */
  counts: '' | 'cards' | 'said';
  events: string;
  ways: Written[];
  /** A rule its shares grow by, as it is written, or none. */
  rule: WrittenRule | null;
}

/** A way with nothing said of it but what it is called. */
export const way = (name: string): Written => ({ name, blueprint: '', share: '1', hands: '', reads: '' });

/** What is asked where nothing has been written: two ways, each as the blueprint is written. */
export const unasked = (): Asking => ({ name: '', per: '', also: '', counts: '', events: '', ways: [way('a'), way('b')], rule: null });

/** What the next way added is called: the first letter no way has. */
export function nextName(ways: Written[]): string {
  const taken = new Set(ways.map((one) => one.name));
  for (let at = 0; at < 26; at += 1) {
    const name = String.fromCharCode(97 + at);
    if (!taken.has(name)) return name;
  }
  return '';
}

const kept = new Map<string, Asking>();

/** What stands for one blueprint of one project. */
export const askingKey = (of: string, project: string | null): string => `${project ?? ''}\n${of}`;

/** What was written of it and not started, or what is asked where nothing was. */
export function askingOf(key: string): Asking {
  return kept.get(key) ?? unasked();
}

/** Keeps what was written, or with `null` lets it go. */
export function keepAsking(key: string, asking: Asking | null): void {
  if (asking) kept.set(key, asking);
  else kept.delete(key);
}

/** Lets go of everything written: whoever was signed in is no longer. */
export function forgetAsking(): void {
  kept.clear();
}

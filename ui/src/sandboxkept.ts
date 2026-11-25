// The sandbox test of each tab, kept by the tab's key.
//
// A view is on the page only while its tab is showing, and a sandbox
// test outlives that: the sandbox the daemon has for the tab, what came
// of it, and what it was run with (what goes first, what the text was
// told, and each thing done to it since) are kept here by the tab, so
// that they are there when its view comes back and so that running it
// again plays the same acts. They are let go when the tab is closed, or
// lost with its set of tabs, and the daemon is then told to end the
// sandbox; nothing of it is kept across a reload of the page, after which
// the daemon ends one that nobody asks anything.
//
// This is the part of a sandbox test every page has, so it is small:
// what runs one and what draws one are asked for when one is first run
// (`sandboxed.ts`, `views/sandboxed.tsx`).

import { signal } from '@preact/signals';
import { call } from './api.ts';
import type { Diagnostic } from './api.ts';
import type { Plan, Shown } from './sandbox.ts';
import type { SideBySide, Word } from './sidebyside.ts';

/** What a tab has of a sandbox test. */
export interface Kept {
  /** What it is run with, and what was done to it since: played again when it is run again. */
  plan: Plan;
  /** The sandbox the daemon has for this tab, where it has one. */
  id: string | null;
  /** What came of it, as it was last read; and of the run before it, for what differs. */
  shown: Shown | null;
  before: Shown | null;
  /** What is being answered, in a few words, while something is. */
  busy: string | null;
  /** Why the last thing asked was not done, in the daemon's words. */
  refused: string | null;
  /** Each act that was not played again, with why. */
  skipped: string[];
  /** What is wrong with the text, where it does not check: nothing was run. */
  problems: Diagnostic[] | null;
  /** Why the daemon has it no longer, where it has not: it was ended, or the daemon was started since. */
  gone: string | null;
  /** How many things asked of it have been answered and read: what waits for one goes by it. */
  answered: number;
  /** It was asked for from the page's head, and has not been run for that yet: its view runs it when it is shown. */
  pressed: boolean;
  /** What goes first was begun from what the daemon says the text needs. */
  begun: boolean;
}

/** A tab that has run none. */
export const NOTHING: Kept = { plan: { first: [], told: {}, answers: 'asked', acts: [] }, id: null, shown: null, before: null, busy: null, refused: null, skipped: [], problems: null, gone: null, answered: 0, pressed: false, begun: false };

/** Every tab's, by its key. */
export const sandboxes = signal<Record<string, Kept>>({});

/** What a tab has of a sandbox test; nothing where none was run there. */
export function sandboxOf(tab: string): Kept | null {
  return sandboxes.value[tab] ?? null;
}

/** Keeps something of a tab's sandbox test, beside what it has. */
export function keepSandbox(tab: string, change: Partial<Kept>): Kept {
  const next = { ...(sandboxes.value[tab] ?? NOTHING), ...change };
  sandboxes.value = { ...sandboxes.value, [tab]: next };
  return next;
}

/** Tells the daemon to end a sandbox. One it has no longer is ended already. */
export function endSandbox(id: string): Promise<void> {
  return call<unknown>('DELETE', `/sandboxes/${encodeURIComponent(id)}`).then(
    () => {},
    () => {},
  );
}

/** What a tab has of sandbox tests side by side: which texts are chosen, and what the daemon answered for them. */
export interface Beside {
  /** The texts chosen to be played, by their words; nothing where nothing was chosen yet. */
  chosen: Word[] | null;
  answer: SideBySide | null;
  busy: string | null;
  refused: string | null;
  /** How many times it was played and read: what waits for one goes by it. */
  answered: number;
  /** It was asked for from the page's head, and has not been played for that yet. */
  pressed: boolean;
}

export const NONE_BESIDE: Beside = { chosen: null, answer: null, busy: null, refused: null, answered: 0, pressed: false };

/** Every tab's, by its key. */
export const besides = signal<Record<string, Beside>>({});

/** Keeps something of a tab's side by side, beside what it has. */
export function keepBeside(tab: string, change: Partial<Beside>): Beside {
  const next = { ...(besides.value[tab] ?? NONE_BESIDE), ...change };
  besides.value = { ...besides.value, [tab]: next };
  return next;
}

/** Has the daemon end the sandboxes a side by side was played in. */
export function endBeside(answer: SideBySide | null): void {
  for (const column of answer?.sandboxes ?? []) void endSandbox(column.sandbox);
}

/** Lets go of a tab's sandbox test and of what it played side by side, and has the daemon end their sandboxes: the tab was closed, or is gone. */
export function forgetSandbox(tab: string): void {
  const had = sandboxes.value[tab];
  if (had) {
    const { [tab]: _gone, ...rest } = sandboxes.value;
    sandboxes.value = rest;
    if (had.id) void endSandbox(had.id);
  }
  const beside = besides.value[tab];
  if (beside) {
    const { [tab]: _gone, ...rest } = besides.value;
    besides.value = rest;
    endBeside(beside.answer);
  }
}

/** Lets go of all of them: whoever was signed in is no longer. */
export function forgetSandboxes(): void {
  for (const tab of new Set([...Object.keys(sandboxes.value), ...Object.keys(besides.value)])) forgetSandbox(tab);
}

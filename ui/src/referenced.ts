// The Reference as a page has it: which entry somebody asked for.
//
// The Reference is one tab, whatever it is opened at (`tab.ts`). An
// entry is asked for from where a question arises (a word pointed at in a
// blueprint's text, an address somebody was handed), and the page brings
// each asking into view once: so it is said here, apart from the tab,
// and a tab that is put back, or shown again, asks for nothing.

import { signal } from '@preact/signals';
import { openTab } from './store.ts';

/** The entry somebody asked for, by its id (`reference.ts`), and which asking it was. */
export const shownEntry = signal<{ at: string; asked: number } | null>(null);

/** Opens the Reference at an entry, or with nothing said as it stands. */
export function showEntry(at = ''): void {
  if (at) shownEntry.value = { at, asked: (shownEntry.peek()?.asked ?? 0) + 1 };
  openTab({ kind: 'reference' });
}

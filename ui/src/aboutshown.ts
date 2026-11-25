// The page about the program: what it read, and the tab somebody asked
// for by an address (`/about#docs/blueprints`), said apart from the tab as
// a Git page's is (`gitshown.ts`), so that the page brings each asking to
// its tab once and a tab shown again asks for nothing. What a tab is
// known by is `about.ts`.

import { signal } from '@preact/signals';
import type { Told } from './about.ts';
import { call } from './api.ts';
import { openTab } from './store.ts';

/** The tab asked for, and which asking it was. */
export const aboutAsked = signal<{ at: string; asked: number } | null>(null);

/** Opens the page about the program at a tab, by what the address says after its `#`; with nothing, as it stands. */
export function showAbout(at = ''): void {
  if (at) aboutAsked.value = { at, asked: (aboutAsked.peek()?.asked ?? 0) + 1 };
  openTab({ kind: 'about' });
}

/** What the address bar says after the `#` while the page is showing: its tab, and nothing for the first. */
export const aboutShowing = signal('');

/** Says which tab the page shows now, as the page chooses it. */
export function sayAboutShowing(at: string): void {
  if (aboutShowing.peek() !== at) aboutShowing.value = at;
}

/** What the program says of itself, as whoever is signed in may be told it. */
export const readAbout = () => call<Told>('GET', '/about');

/** One guide, as a page. */
export const readGuide = (name: string) => call<{ name: string; title: string; html: string }>('GET', `/about/guides/${encodeURIComponent(name)}`);

// The client on a phone: a screen too narrow for things to be side by side,
// and touched, not pointed at.
//
// How it looks there is `phone.css`. This is the little that is not looks:
// whether the screen is that narrow just now, which the browser says when
// it changes, and what follows from it that can be worked out without a
// screen. The side panel lies over the page and is put away when something
// is chosen (`store.ts`); the tabs are one row, whatever groups they are in
// on a wider screen; a card is moved by saying where it goes, not carried.

import { signal } from '@preact/signals';
import { group, groups } from './panes.ts';
import type { Group, Layout } from './panes.ts';

/**
 * What a phone is, to the stylesheet and to this: too narrow for a column
 * beside the page, or held on its side and too short for one above it. The
 * second is only ever a touch screen: a short window on a desk is a window.
 * `phone.css` asks the same question in the same words.
 */
export const NARROW = '(max-width: 760px), (pointer: coarse) and (max-height: 520px)';

const asked = typeof matchMedia === 'undefined' ? null : matchMedia(NARROW);
/** Whether the client is on a screen that narrow just now. */
export const phone = signal(asked?.matches ?? false);
asked?.addEventListener('change', (event) => (phone.value = event.matches));

/** Whether what points at the page is a finger: there are no keys held with it, and nothing is hovered over. */
export const touched = typeof matchMedia !== 'undefined' && matchMedia('(pointer: coarse)').matches;

/**
 * Every open tab in one row, for a screen with no room for two groups. They
 * stay in the groups they were put in, in the order those are in, and the
 * one showing is the one showing in the group last used: choosing a tab of
 * another group makes that the group last used, so it shows. Nothing is
 * lost for a wider screen to come back to.
 */
export function oneRow(layout: Layout, focused: string): Group {
  const all = groups(layout);
  const front = all.find((entry) => entry.id === focused) ?? all[0]!;
  return group(front.id, all.flatMap((entry) => entry.tabs), front.active);
}

/**
 * Where a card stands among the columns, and where it can be sent. A card
 * left in a column the board no longer has is still said to be there.
 */
export function placesFor(columns: string[], now: string): string[] {
  return columns.includes(now) ? columns : [now, ...columns];
}

/**
 * How far along a row that scrolls must be for one thing in it to be seen
 * whole: where it is if the thing already is, and otherwise the least that
 * brings it in. `from` and `to` are the thing's edges within the row.
 */
export function inView(scrolled: number, width: number, from: number, to: number): number {
  if (from < scrolled) return Math.max(0, from);
  if (to > scrolled + width) return Math.max(0, Math.min(from, to - width));
  return scrolled;
}

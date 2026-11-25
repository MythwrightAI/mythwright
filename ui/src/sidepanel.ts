// The side panel as the page has it: which view it shows (`null` is folded
// away, for more room), the one it opens to again, and what in its trees
// has been turned from how it starts. All of it is the reader's, kept in
// this browser. It came out of the store, which offers it under the names
// it always had, and stands on nothing of the store's.

import { signal } from '@preact/signals';
import { turnedNow } from './folds.ts';
import { phone } from './phone.ts';

// What in the side panel's trees has been turned from how it starts, by a
// name for each thing that folds: in the Resources panel everything starts
// folded, so what is kept there is what was unfolded (`folds.ts`). Kept here and not in the tree,
// which is taken off the page whenever the panel shows something else.
const SAVED_TURNED = 'client.turned';
function savedTurned(): Set<string> {
  try {
    return new Set(turnedNow(JSON.parse(localStorage.getItem(SAVED_TURNED) ?? '[]')));
  } catch {
    return new Set();
  }
}
export const turned = signal<Set<string>>(typeof localStorage === 'undefined' ? new Set() : savedTurned());
/** Folds what is unfolded, and the other way round. */
export function turn(key: string): void {
  const next = new Set(turned.value);
  if (!next.delete(key)) next.add(key);
  turnedTo(next);
}
/** What is turned is these and nothing else: several folds folded or unfolded at once (Collapse all, Expand all). */
export function turnedTo(keys: Iterable<string>): void {
  const next = new Set(keys);
  turned.value = next;
  try {
    localStorage.setItem(SAVED_TURNED, JSON.stringify([...next]));
  } catch {
    /* not remembered */
  }
}

/**
 * What the channels' panel is called, on the rail, at its head and for
 * whoever hears it read out: Channels, since it holds the channels alone
 * again now that the topics have a place of their own. It was
 * Messaging while it held both; the owner has not said which he
 * wants, so it is said here once, and is one line to change.
 */
export const CHANNELS_CALLED = 'Channels';

/**
 * Which view the side panel shows; `null` is closed, for more room. The
 * inbox is one of them, the reader's own box (`views/inbox.tsx`), and so
 * are a forum's topics (`views/topics.tsx`). `channels` is the
 * channels' panel, whatever it is called on the page (`CHANNELS_CALLED`):
 * a browser kept it as that while it was called Messaging and held the
 * topics too, and shows the channels for it now.
 */
export type SideView = 'channels' | 'topics' | 'resources' | 'inbox' | 'extensions';
/** A view the panel was left showing, as this browser kept it. */
function viewOf(saved: string | null): SideView {
  return saved === 'extensions' || saved === 'resources' || saved === 'inbox' || saved === 'topics' ? saved : 'channels';
}
const SAVED_SIDE = 'client.side';
function savedSide(): SideView | null {
  try {
    const saved = localStorage.getItem(SAVED_SIDE);
    if (saved === 'closed') return null;
    return viewOf(saved);
  } catch {
    return 'channels';
  }
}
// On a phone the panel lies over the page, so it starts put away, whatever it was left as.
export const side = signal<SideView | null>(typeof localStorage === 'undefined' ? 'channels' : phone.value ? null : savedSide());
// The view the panel last showed, which is the one it opens to again.
const SAVED_LAST_SIDE = 'client.side.last';
function savedLastSide(): SideView {
  try {
    return viewOf(localStorage.getItem(SAVED_LAST_SIDE));
  } catch {
    return 'channels';
  }
}
let lastSide: SideView = side.value ?? (typeof localStorage === 'undefined' ? 'channels' : savedLastSide());
function rememberSide(): void {
  if (side.value) lastSide = side.value;
  try {
    localStorage.setItem(SAVED_SIDE, side.value ?? 'closed');
    localStorage.setItem(SAVED_LAST_SIDE, lastSide);
  } catch {
    /* not remembered */
  }
}
/** Shows a view in the side panel; asking for the one already shown closes the panel. */
export function showSide(view: SideView, toggle = true): void {
  side.value = toggle && side.value === view ? null : view;
  rememberSide();
}
/** Folds the side panel away for more room, or brings it back showing what it last showed. */
export function foldSide(): void {
  side.value = side.value ? null : lastSide;
  rememberSide();
}
/**
 * On a phone, takes the side panel from over the page: something in it was
 * chosen, or the screen has just become that narrow. What it showed is what
 * it opens to again. Nothing is kept: it was not folded away by anyone.
 */
export function clearSide(): void {
  if (!phone.value || !side.value) return;
  lastSide = side.value;
  side.value = null;
}
phone.subscribe(clearSide);

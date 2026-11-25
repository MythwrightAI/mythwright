// How wide the drawer is.
//
// The drawer floats over the right of the workbench, and whatever is in
// it (a card, a record, a step of a graph, what a blueprint is asked) is
// as wide as it was last dragged to, which is kept in this browser as the
// side panel's width is. Dragged or not, it is never too narrow to fill a
// form in, and never wider than the workbench it lies over.

import { signal } from '@preact/signals';

/** How wide it is until its edge is dragged. */
export const USUAL_DRAWER = 400;
/** The least it can be, where there is room for that. */
export const LEAST_DRAWER = 300;
/** The most it is ever kept at: wider than any window it has been dragged across makes no sense. */
const MOST_DRAWER = 4000;
/** What it stays clear of at the workbench's two sides, together. */
export const CLEAR = 20;

/** A width the drawer can have over a workbench `room` across. */
export function fitDrawer(asked: number, room: number): number {
  const most = Math.max(0, room - CLEAR);
  return Math.round(Math.min(Math.max(asked, Math.min(LEAST_DRAWER, most)), most));
}

/** The width that was kept, or the usual one where none was or it makes no sense. */
export function drawerOf(saved: string | null | undefined): number {
  const kept = Number(saved);
  return saved && Number.isFinite(kept) && kept >= LEAST_DRAWER && kept <= MOST_DRAWER ? Math.round(kept) : USUAL_DRAWER;
}

const SAVED = 'client.drawer.width';
function kept(): string | null {
  try {
    return localStorage.getItem(SAVED);
  } catch {
    return null;
  }
}

export const drawerWidth = signal<number>(drawerOf(typeof localStorage === 'undefined' ? null : kept()));

/** Makes it this wide, and keeps that: it is told once, when the edge is let go of. */
export function setDrawerWidth(width: number): void {
  drawerWidth.value = width;
  try {
    localStorage.setItem(SAVED, String(width));
  } catch {
    /* not remembered */
  }
}

/** What a drawer says of itself once it is where it will be: something that measures it waits for this. */
export const AT_REST = 'rest';

/**
 * The drawer comes in from the right edge (`drawer.css`), and until it
 * has it is not where it will be. It says when it is: at once where
 * nothing moves it there (less movement was asked for), and otherwise
 * when what slides it in is over, however that ended. Nothing here
 * counts time: the style sheet's animation is asked when it is over.
 */
export function drawn(drawer: HTMLElement | null): void {
  if (!drawer) return;
  const rest = () => void (drawer.dataset[AT_REST] = '');
  const moving = typeof drawer.getAnimations === 'function' ? drawer.getAnimations() : [];
  if (moving.length === 0) rest();
  else void Promise.allSettled(moving.map((one) => one.finished)).then(rest);
}

/** The least room a page keeps for itself left of an open drawer, beyond which the drawer lies over it as on a phone, where it has the screen. */
export const LEAST_BESIDE = 160;
/** What a page keeps between its right edge and an open drawer. */
export const BESIDE_GAP = 10;

/**
 * How much of its right a page, from `left` to `right` across the window,
 * keeps clear for a drawer whose left edge is at `drawer` (none where no
 * drawer is open), so that nothing of it is under the drawer: as much as
 * the drawer lies over it, and the gap. Nothing where the drawer does not
 * reach it, and nothing where what would be left is less than
 * `LEAST_BESIDE`: the drawer then lies over it, as it has the screen on a
 * phone.
 */
export function clearOfDrawer(left: number, right: number, drawer: number | null): number {
  if (drawer === null || drawer >= right) return 0;
  if (drawer - left - BESIDE_GAP < LEAST_BESIDE) return 0;
  return Math.ceil(right - drawer + BESIDE_GAP);
}

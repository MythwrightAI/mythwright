// How wide the side panel is.
//
// It is as wide as it was last dragged to, which is kept in this browser.
// Dragged or not, it is never too narrow to read a name in, and never so
// wide that what is beside it has no room.

import { signal } from '@preact/signals';

/** How wide it is until its edge is dragged. */
export const USUAL_SIDE = 252;
/** The least it can be, and the most whatever the window. */
export const LEAST_SIDE = 180;
const MOST_SIDE = 560;
/** The rail beside it, and what the page keeps for itself. */
const RAIL = 44;
const LEAST_PAGE = 420;

/** A width the side panel can have in a window `room` across. */
export function fitSide(asked: number, room: number): number {
  const most = Math.max(LEAST_SIDE, Math.min(MOST_SIDE, room - RAIL - LEAST_PAGE));
  return Math.round(Math.min(Math.max(asked, LEAST_SIDE), most));
}

/** The width that was kept, or the usual one where none was or it makes no sense. */
export function sideOf(saved: string | null | undefined): number {
  const kept = Number(saved);
  return saved && Number.isFinite(kept) && kept >= LEAST_SIDE && kept <= MOST_SIDE ? Math.round(kept) : USUAL_SIDE;
}

const SAVED = 'client.side.width';
function kept(): string | null {
  try {
    return localStorage.getItem(SAVED);
  } catch {
    return null;
  }
}

export const sideWidth = signal<number>(sideOf(typeof localStorage === 'undefined' ? null : kept()));

/** Makes it this wide. It is kept when the edge is let go of, not at every step of the way. */
export function setSideWidth(width: number, done: boolean): void {
  sideWidth.value = width;
  if (!done) return;
  try {
    localStorage.setItem(SAVED, String(width));
  } catch {
    /* not remembered */
  }
}

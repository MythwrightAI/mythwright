// How things are shown: as tabs beside one another, in the desktop app's
// window; or, in a browser, one at a time in place, as a web page shows
// what is opened, the browser's own tabs being the tabs there.
//
// In a browser one thing shows at a time. Opening something shows it in
// place: the address bar says it (`address.ts`), Back and Forward go
// between what was shown, and each is shown again as it was left, by what
// is kept for it by its key (`returned.ts`), as a tab shown again is in the
// window. Ctrl or ⌘ with a press, the middle button, or "Open in a new
// tab" in a line's menu opens it in a tab of the browser's own. There is
// no row of tabs, no split into groups and no set of tabs for each
// workspace (`tabsof.ts` keeps nothing there); what was lately opened and
// what is pinned are the way back to things.
//
// The desktop window keeps its tabs, groups and the sets as they were.
// Which of the two the page is in is said by the window as it loads the
// page (`desktop.ts`): what is here is worked out from plain values, so
// that it can be tested without a page.

import { desktop } from './desktop.ts';
import type { Tab } from './tab.ts';

/** Whether things are shown one at a time in place: in a browser, and not in the desktop app's window, which keeps tabs. */
export const inPlace = typeof window !== 'undefined' && desktop === null;

/**
 * How many things shown before are kept as they were left, to be shown so
 * again by Back: what each read, where it was scrolled, its drawer. One
 * left further back than this is let go of, and shown again it is read
 * anew, as a tab opened afresh is.
 */
export const KEPT_BEHIND = 20;

/**
 * The things shown before, the latest first, once `left` was left for
 * `showing`: what is shown is not among them, nothing is there twice, and
 * no more than `most` are; what that leaves out is `gone`, to be let go
 * of. What is half written in one is not let go of with it (`keepDraft`).
 */
export function parked(behind: readonly string[], left: string, showing: string, most = KEPT_BEHIND): { behind: string[]; gone: string[] } {
  const all = [left, ...behind].filter((key, at, list) => key !== showing && list.indexOf(key) === at);
  return { behind: all.slice(0, most), gone: all.slice(most) };
}

/** What a press is, as far as it says where what is pressed opens. */
export interface Press {
  /** 0 for the main button, 1 for the middle one. */
  button: number;
  ctrlKey: boolean;
  metaKey: boolean;
}

/**
 * Whether a press opens what is pressed in a new tab of the browser's own,
 * as a link pressed so does on any web page: with Ctrl or ⌘ held, or by
 * the middle button. Any other press opens it in place.
 */
export function opensApart(press: Press): boolean {
  if (press.button === 1) return true;
  return press.button === 0 && (press.ctrlKey || press.metaKey);
}

/**
 * What a line that opens a thing says it opens, for a press that opens
 * it in a new tab of the browser's (`data-names`): the thing, written
 * out. The page reads it back with `named`.
 */
export const names = (tab: Tab): string => JSON.stringify(tab);

/** The thing a line says it opens; nothing where it says none, or nothing that is one. */
export function named(said: string | null | undefined): Tab | null {
  if (!said) return null;
  try {
    const tab = JSON.parse(said) as unknown;
    return tab && typeof tab === 'object' && typeof (tab as { kind?: unknown }).kind === 'string' ? (tab as Tab) : null;
  } catch {
    return null;
  }
}

/**
 * Whether the bar over what is shown is drawn, in a browser: where it has
 * something to hold, which is what brings the side panel back while it is
 * folded away; and on a phone always, where the panel is away whenever
 * what is shown is. In the window the row of tabs is that bar, always.
 */
export function barShown(side: boolean, phone: boolean): boolean {
  return phone || !side;
}

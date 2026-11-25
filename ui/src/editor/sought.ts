// What somebody last searched a tab's text for.
//
// A view is on the page only while its tab is showing, and its editor
// with it: the words searched for, the switches and what they were to be
// replaced with are kept here by the tab, so that the bar holds them when
// it is opened again, whichever editor of the tab opens it. They are let
// go when the tab is closed, and are not kept across a reload of the page.

/** What is searched for, as the bar asks it. */
export interface Sought {
  search: string;
  caseSensitive: boolean;
  wholeWord: boolean;
  regexp: boolean;
  replace: string;
}

/** Nothing searched for, and no switch on. */
export const UNSOUGHT: Sought = { search: '', caseSensitive: false, wholeWord: false, regexp: false, replace: '' };

const kept = new Map<string, Sought>();

/** What was last searched for in a tab; nothing where nothing was. */
export function soughtIn(tab: string): Sought | null {
  return kept.get(tab) ?? null;
}

/** Keeps what is searched for in a tab, or with `null` lets it go. */
export function keepSought(tab: string, sought: Sought | null): void {
  if (sought) kept.set(tab, sought);
  else kept.delete(tab);
}

/** Lets go of all of it: whoever was signed in is no longer. */
export function forgetSought(): void {
  kept.clear();
}

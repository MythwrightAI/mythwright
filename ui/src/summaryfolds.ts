// Which folds of a blueprint's Summary somebody opened, kept by the tab.
//
// A view is on the page only while its tab is showing: what was unfolded
// (a handler's steps, the things it may be told) is kept here by the tab's
// key, so that it is as it was left when the tab comes back. It is let go
// when the tab is closed and when whoever was signed in is no longer, and
// is not kept across a reload of the page, as what is searched for is not.

const kept = new Map<string, Set<string>>();

/** The folds opened in a tab's Summary. */
export function openedIn(tab: string): ReadonlySet<string> {
  return kept.get(tab) ?? new Set();
}

/** Keeps a fold of a tab's Summary as opened or as folded again. */
export function keepOpened(tab: string, fold: string, open: boolean): void {
  const now = new Set(kept.get(tab) ?? []);
  if (open) now.add(fold);
  else now.delete(fold);
  if (now.size === 0) kept.delete(tab);
  else kept.set(tab, now);
}

/** Lets go of what a tab kept: it was closed. */
export function forgetOpened(tab: string): void {
  kept.delete(tab);
}

/** Lets go of all of it: whoever was signed in is no longer. */
export function forgetAllOpened(): void {
  kept.clear();
}

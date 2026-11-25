// What is unfolded of what an applied text was handed (`handed.ts`), kept
// by the tab its History is in: the heading, and each mapping or list
// under it by where it is. A view is on the page only while its tab is
// showing, so this outlives it; it is let go when the tab is closed and
// when whoever was signed in is no longer, and is not kept across a
// reload of the page. Folded is how each begins.

const kept = new Map<string, Set<string>>();

/** What is unfolded in a tab. The heading is the empty path. */
export function openIn(tab: string): ReadonlySet<string> {
  return kept.get(tab) ?? new Set();
}

/** Unfolds or folds one thing in a tab, and hands back what is unfolded there then. */
export function turnIn(tab: string, path: string): ReadonlySet<string> {
  const now = new Set(kept.get(tab) ?? []);
  if (now.has(path)) now.delete(path);
  else now.add(path);
  kept.set(tab, now);
  return now;
}

/** Lets go of what was kept for a tab, or with nothing said, for every tab. */
export function forgetOpen(tab?: string): void {
  if (tab === undefined) kept.clear();
  else kept.delete(tab);
}

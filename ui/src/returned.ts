// What a tab showed, kept while it is away, so that it is shown again as it was left.
//
// A view is on the page only while its tab is showing: another tab shown
// takes it off, and showing it again makes it anew. Kept alive off the
// page, a view would go on following what is said and the checks would
// find two of everything, so it is not; what it showed is kept here
// instead, by the tab's key, and handed back as it is made again. Each
// scope is one tab's (its view's, and its drawer's beside it, which is
// `drawerScope` of its key), and holds what the view says it keeps by a
// name of its own: what it last read, which of its views showed, what was
// folded, and where each part of it was scrolled to.
//
// What was read is shown at once from here and read again behind: while a
// view was away it heard nothing, so the reading again is what puts right
// what was said meanwhile, and what was kept is never taken for what the
// thing has come to.
//
// It is let go of with the tab, where whatever else is kept for a tab is
// (`closeTab` and signing out in the store, `lose` in `tabsof.ts`), so there
// is one scope a tab that is open and none for one that is not. It is not
// kept over reading the page again.

const memory = new Map<string, Map<string, unknown>>();

/** The scope of the drawer of the tab of `key`, which is let go of with the tab. */
export const drawerScope = (key: string): string => `${key}\u0000drawer`;

/** The scope of a part of a tab's view that keeps what it holds apart (a card's form in the Tools panel), let go of by itself or with the tab. */
export const partScope = (scope: string, name: string): string => `${scope}\u0000${name}`;

/** What a scope keeps under `name`, where it keeps anything. */
export function recall<T>(scope: string | null, name: string): T | undefined {
  return scope === null ? undefined : (memory.get(scope)?.get(name) as T | undefined);
}

/** Keeps `value` under `name` in a scope. */
export function remember(scope: string | null, name: string, value: unknown): void {
  if (scope === null) return;
  let kept = memory.get(scope);
  if (!kept) memory.set(scope, (kept = new Map()));
  kept.set(name, value);
}

/**
 * Whether the drawer open over the tab `owner` is kept for it as `showing`
 * comes to show: where it is of an open tab, and that tab is not the one
 * coming to show, whose press again puts its drawer away as it always did.
 * Where the whole set of tabs changes (another workspace's) nothing is
 * coming to show by a press, and `showing` is nothing.
 */
export function keepsDrawer(owner: string | null, showing: string): boolean {
  return owner !== null && owner !== '' && owner !== showing;
}

/**
 * What the store tells of the drawer as tabs are shown and closed, which
 * `drawerkept.ts` answers: it stands on the store, which cannot ask it.
 * `away` puts the drawer away for the tab `showing` to show, `back` gives
 * the tab that shows its own, and `forget` lets go of a tab's.
 */
export const drawerKeeper = { away: (_showing: string): void => {}, back: (): void => {}, forget: (_key: string | null): void => {} };

/** Lets go of one scope, and nothing else. */
export function forgetScope(scope: string): void {
  memory.delete(scope);
}

/** Lets go of everything kept for the tab of `key`: its view's, its drawer's and the drawer itself. */
export function forgetReturned(key: string): void {
  memory.delete(key);
  // Its drawer's, and each part's that keeps what it holds apart.
  for (const scope of [...memory.keys()]) if (scope.startsWith(`${key}\u0000`)) memory.delete(scope);
  drawerKeeper.forget(key);
}

/** Lets go of everything, as somebody signs out. */
export function forgetAllReturned(): void {
  memory.clear();
  drawerKeeper.forget(null);
}

/** The scopes there are, for a test to count. */
export const scopes = (): string[] => [...memory.keys()];

// ---- where things were scrolled to ----------------------------------------------

/**
 * Where one part of a view was scrolled to. A part is found again by what
 * it is (its element and its classes) and how many of those come before
 * it, since the view it is in is made anew and nothing of the one before
 * is there to point at.
 */
export interface Scrolled {
  sign: string;
  nth: number;
  top: number;
  left: number;
}

/** What a part is, to be found again by: its element and its classes in order. */
export function signOf(tag: string, classes: Iterable<string>): string {
  return `${tag.toLowerCase()}.${[...classes].filter(Boolean).sort().join('.')}`;
}

/** What is still to be put back of what was kept, and what is done, as each part stands now. */
export interface Part {
  /** How far it is scrolled now. */
  top: number;
  left: number;
  /** How far it can be scrolled. */
  most: number;
  mostLeft: number;
}

/**
 * What becomes of a part kept as scrolled, as it is found: put back there
 * (`to`), left where it is because the view put it somewhere itself (a
 * channel at its newest message, a line it was asked to show), or waited
 * for because it is not yet long enough to be scrolled that far. A part
 * not found yet is waited for too. One that was put back already (`held`)
 * is held there while what is above it is still being laid out: the
 * browser moves a part to keep in view what came into view as the part
 * grows above it, and what came into view then is not what was left.
 */
export function putBack(kept: Scrolled, found: Part | null, held = false): 'wait' | 'leave' | 'there' | { top: number; left: number } {
  if (!found) return 'wait';
  if (held && Math.abs(found.top - kept.top) <= 1 && Math.abs(found.left - kept.left) <= 1) return 'there';
  if (!held && (found.top !== 0 || found.left !== 0)) return 'leave';
  if (found.most < kept.top || found.mostLeft < kept.left) return 'wait';
  return { top: kept.top, left: kept.left };
}

/**
 * Said by a view that brings something into view itself once it is drawn
 * (the Reference at the entry it was opened at): what was kept of where
 * it was scrolled is put back no more, and held there no longer, since
 * held it took the place of what the view was asked to show.
 */
export const SCROLLS_ITSELF = 'returned:scrolls-itself';
export const scrollsItself = (): void => {
  if (typeof window !== 'undefined') window.dispatchEvent(new Event(SCROLLS_ITSELF));
};

/** How many frames a view is given to come to be as long as it was, before what was not put back is let be. */
export const FRAMES = 120;

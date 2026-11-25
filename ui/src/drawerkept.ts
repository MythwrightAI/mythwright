// The drawer is the tab's it was opened beside.
//
// It is put away when another tab is shown, as it always was, and kept
// meanwhile under the tab that was showing when it was opened: shown
// again, that tab has it back, as it was. It is simply there (`back`, which
// the frame draws with no way in, and which says at once that it is at
// rest), what it holds is drawn again from what it was handed, and how far
// its parts were scrolled is put back (`views/returned.tsx`, over the scope
// of the tab's drawer). Only a tab that is open has one kept, and only one:
// what is kept for a tab goes with it (`forget`), and a drawer put away by
// itself (its button, Escape, whatever said so, what it held opened as a
// tab of its own) is not kept anywhere.
//
// Its tab shown again by the same press that would put it away (the tab
// that is showing pressed again) puts it away as before: it is kept only
// where another tab came to show.
//
// What it shows is drawn again from what it was handed. A drawer of
// something that can be taken away (a card, a row, something running) says
// how it is asked whether that is gone (`gone`); given back, it is asked,
// and a drawer of what is gone is put away and kept for nobody, as one put
// away by its own button is. What it shows meanwhile is what it showed.

import { effect, signal } from '@preact/signals';
import { ApiError } from './api.ts';
import { drawerKeeper, drawerScope, forgetScope, keepsDrawer } from './returned.ts';
import { active, drawer } from './store.ts';
import type { Drawer } from './store.ts';

/** The drawer of each tab that is not showing, as it stood when another was shown. */
const away = new Map<string, Drawer>();
/** The tab the open drawer is of. */
let owner: string | null = null;
/** The drawer last seen open, to tell one given back from one opened. */
let seen: Drawer | null = null;
let giving: Drawer | null = null;

/** Whether the open drawer is one given back to its tab: drawn with no way in. */
export const back = signal(false);

/** The tab the open drawer is of, for what it keeps of itself (its scope). */
export const drawerOwner = signal<string | null>(null);

effect(() => {
  const now = drawer.value;
  if (now === seen) return;
  seen = now;
  if (now === null) {
    owner = null;
    back.value = false;
  } else if (now === giving) {
    back.value = true;
  } else {
    // A drawer opened afresh is of the tab showing, and keeps nothing of the one it took the place of.
    owner = active.peek() || null;
    if (owner) forgetScope(drawerScope(owner));
    back.value = false;
  }
  drawerOwner.value = owner;
  giving = null;
});

drawerKeeper.away = (showing: string) => {
  const open = drawer.peek();
  // The tab showing pressed again puts its drawer away and keeps nothing, as it always did.
  if (open && keepsDrawer(owner, showing)) away.set(owner!, open);
  drawer.value = null;
};

drawerKeeper.back = () => {
  const key = active.peek();
  const kept = away.get(key);
  if (!kept) return;
  away.delete(key);
  giving = kept;
  owner = key;
  drawer.value = kept;
  void kept.gone?.().then((yes) => {
    if (yes && drawer.peek() === kept) drawer.value = null;
  });
};

drawerKeeper.forget = (key: string | null) => {
  if (key === null) {
    away.clear();
    owner = drawerOwner.value = null;
    return;
  }
  away.delete(key);
  // A drawer open over the tab that was closed is nobody's: it stays as it was, and is not kept when another tab shows.
  if (owner === key) owner = drawerOwner.value = null;
};

/** How a drawer of a thing read by `read` asks whether it is gone: the daemon says there is no such thing. */
export const goneBy = (read: () => Promise<unknown>) => (): Promise<boolean> =>
  read().then(
    () => false,
    (err) => err instanceof ApiError && err.status === 404,
  );

/** How many tabs have a drawer kept, for a test to count. */
export const keptDrawers = (): string[] => [...away.keys()];

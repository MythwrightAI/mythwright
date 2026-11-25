// The open tabs of each workspace, as the page has them.
//
// They are whoever is looking's: kept in this browser under who is signed
// in, as the workspace the side panel is of is (`chosen.ts`), and asked
// of nobody. No route is told which tabs are open or which workspace is
// looked at. What is worked out is `tabskept.ts`; here is where the sets
// are read and written, what puts one on the page, and what forgets.
//
// The order as the page is read. Which set is shown rests on which
// workspace the panel is of, and that, where nothing was chosen, rested
// on the tab that was showing. So the workspace is settled first, with
// no tab on the page: what was chosen while it is there; else, where a
// browser kept one set from before each workspace had its own
// (`client.tabs`), the workspace of the tab that was showing in it; else
// the usual one (`beginAt`). It is kept as the choice there and then,
// and only then are tabs put back: that workspace's own set, or, where
// it has none, the set from before, which is its from then on and is
// taken out of where it was kept, so that no other workspace is ever
// shown it. That is why the frame waits for the workspaces before it has
// entered (`spacesAsked`): put back sooner, the tabs would be shown and
// then changed for another workspace's. Somebody who signs in is shown
// the frame before that, so what they open in the meantime (Settings,
// pressed on the rail as soon as it is there) is opened among the tabs
// put back (`among`), and is not put away with the start page it was
// opened beside. Where the workspaces cannot be read, or nobody owns the
// factory yet, the panel is of none and there is one set, kept where it
// always was.
//
// After that the set on the page changes only when the panel comes to be
// of another workspace, whatever made it so (the foot's menu, the count
// of what waits unread, a workspace made there, the one showing taken
// away). A reading of the workspaces that fails for a while changes
// nothing: the set stays, and stays that workspace's.
//
// All of this is the desktop window's. In a browser one thing shows at a
// time and the browser's own tabs are the tabs (`inplace.ts`): nothing is
// kept here, nothing is put back but the workspace the panel is of, and
// another workspace chosen leaves what is shown as it is.

import { effect, untracked } from '@preact/signals';
import { beginAt, held, hold } from './chosen.ts';
import { keepSought } from './editor/sought.ts';
import { forgetOpened } from './summaryfolds.ts';
import { forgetSandbox } from './sandboxkept.ts';
import { forgetReturned } from './returned.ts';
import { forgetOpen } from './handedopen.ts';
import { spacesAsked, spacesRead, workspaces } from './spaces.ts';
import { channels, drawer, entered, focused, forgetBehind, keepDraft, layout, me, onChannelRemoved, projects, showTabs, tabKey, tabs, tabsKeeper } from './store.ts';
import { inPlace } from './inplace.ts';
import { among, before, first, forgot, keptOf, showingIn, standingSet, switched } from './tabskept.ts';
import type { Known, Sets, TabSet } from './tabskept.ts';

/** Where the set of each workspace is kept, under who is signed in. */
const SAVED = 'client.tabs.kept';
/** Where the one set is kept while the panel is of no workspace; and where a browser kept it before each workspace had its own. */
const BEFORE = 'client.tabs';
const keptAt = (actor: string) => `${SAVED}:${actor}`;

/** The set of each workspace as last kept: the one on the page among them. */
let sets: Sets = {};
/** The workspace the set on the page is of; none while the panel is of none. */
let of: string | null = null;
/** Whether what was kept has been put back for whoever is signed in: until then nothing on the page is theirs to keep. */
let back = false;

const showing = (): TabSet => ({ tabs: tabs.peek(), layout: layout.peek(), focused: focused.peek() });
const known = (): Known => ({ projects: projects.peek().map((one) => one.id), channels: channels.peek().map((one) => one.id) });

/** Keeps what there is: the one set where the panel is of no workspace, and the set of each workspace where any is kept. */
function write(): void {
  const actor = me.peek()?.actor;
  if (!actor) return;
  try {
    if (of === null) localStorage.setItem(BEFORE, JSON.stringify(showing()));
    if (of !== null || Object.keys(sets).length > 0) localStorage.setItem(keptAt(actor), JSON.stringify(sets));
  } catch {
    /* private window or blocked storage: the sets are kept while the page is, and not over reading it again */
  }
}

/** What was kept for tabs that are gone goes with them, as it does when a tab is closed. */
function lose(keys: string[]): void {
  for (const key of keys) {
    keepDraft(key, null);
    keepSought(key, null);
    forgetOpened(key);
    forgetOpen(key);
    forgetSandbox(key);
    forgetReturned(key);
  }
}

/** Waits until the workspaces were read, or could not be. */
function asked(): Promise<void> {
  if (spacesAsked.peek()) return Promise.resolve();
  return new Promise((done) => {
    const stop = spacesAsked.subscribe((yes) => {
      if (!yes) return;
      done();
      queueMicrotask(() => stop());
    });
  });
}

tabsKeeper.arranged = () => {
  if (!back || inPlace) return;
  if (of !== null) sets = { ...sets, [of]: showing() };
  write();
};

tabsKeeper.back = async () => {
  const actor = me.peek()?.actor;
  await asked();
  // Somebody else by now, or nobody: what is put back is theirs to ask for.
  if (!actor || me.peek()?.actor !== actor) return;
  // In a browser there are no sets: the workspace is settled as it would be with none kept, and what shows is what the address names.
  if (inPlace) {
    if (spacesRead.peek()) beginAt(undefined);
    back = true;
    return;
  }
  let was: TabSet | null = null;
  try {
    sets = keptOf(localStorage.getItem(keptAt(actor)));
    was = before(localStorage.getItem(BEFORE));
  } catch {
    sets = {};
  }
  const there = workspaces.peek().map((one) => one.id);
  // Settled before any tab is on the page, by the tab that was showing in what a browser kept from before, and kept.
  const begins = spacesRead.peek() ? beginAt(was ? showingIn(was) : undefined) : null;
  of = begins?.kind === 'space' ? begins.id : null;
  const put = first(sets, was, of, there, known());
  // The frame is there before this, and what was pressed meanwhile is opened among what is put back, not put away.
  const meanwhile = showing();
  const shown = among(put.show, meanwhile);
  sets = of === null ? put.sets : { ...put.sets, [of]: shown };
  const aside = drawer.peek();
  showTabs(shown);
  // A drawer opened meanwhile is of what was showing, and stays where that still shows.
  const [then, now] = [showingIn(meanwhile), showingIn(shown)];
  if (aside && then && now && tabKey(then) === tabKey(now)) drawer.value = aside;
  back = true;
  if (of !== null) {
    // What a browser kept from before is that workspace's from now, or was passed over for the set it has: either way it is no other's.
    try {
      localStorage.removeItem(BEFORE);
    } catch {
      /* nothing was kept there */
    }
  }
  write();
};

tabsKeeper.forget = () => {
  const actor = me.peek()?.actor;
  localStorage.removeItem(BEFORE);
  if (actor) localStorage.removeItem(keptAt(actor));
};

// Nobody signed in has no sets: the next to sign in is read their own.
effect(() => {
  if (me.value) return;
  sets = {};
  of = null;
  back = false;
});

/** The panel is of workspace `to`: the set on the page is kept under the one it was of, and that workspace's is put there. */
function turn(to: string): void {
  const there = workspaces.peek().map((one) => one.id);
  const next = switched(sets, showing(), of, to, there);
  sets = next.sets;
  const was = of;
  of = to;
  if (next.show) {
    // Kept as the choice before anything is shown: where the one that was chosen is gone, the panel went by the
    // tab that was showing, and would go on to the workspace of whichever tab shows next.
    hold({ kind: 'space', id: to });
    const now = standingSet(next.show, known());
    sets = { ...sets, [to]: now.set };
    lose(now.lost);
    showTabs(now.set);
  }
  if (was === null) {
    try {
      localStorage.removeItem(BEFORE);
    } catch {
      /* nothing was kept there */
    }
  }
  write();
}

// The panel comes to be of another workspace, or the workspaces there are change (one taken away has its set
// forgotten): only once everything was read, and never from a list that was not read, which says nothing of
// what is gone and nothing of which workspace the panel is of.
effect(() => {
  const now = held.value;
  workspaces.value;
  if (inPlace || !entered.value || !spacesRead.value || now.kind !== 'space') return;
  untracked(() => turn(now.id));
});

// A channel that was taken away is a tab in no set: the store closes the one on the page, and here it is
// taken out of the sets that are not.
onChannelRemoved((id) => forgetTab(`channel:${id}`));

/**
 * Takes a tab out of every set that is kept, for whoever took its thing away and so knows it is gone (a file
 * deleted from its own page, a channel removed). The one on the page is closed by whoever asks, as it always was.
 */
export function forgetTab(key: string): void {
  if (inPlace) {
    forgetBehind(key);
    if (!tabs.peek().some((tab) => tabKey(tab) === key)) lose([key]);
    return;
  }
  const next = forgot(sets, key);
  if (next === sets) return;
  const here = of === null ? null : sets[of];
  sets = here && of !== null ? { ...next, [of]: here } : next;
  // Where it is not on the page nobody closes it: what was kept for it goes here.
  if (!tabs.peek().some((tab) => tabKey(tab) === key)) lose([key]);
  if (back) write();
}

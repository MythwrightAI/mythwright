// Which workspace the side panel is of.
//
// The panel is of one workspace at a time, and which one is whoever is
// looking's to say, as what they folded is: it is kept in this browser under who is
// signed in, and the factory knows nothing of it. What it does to the
// lists is `workspaces.ts` (`Held`, and `listed` and `gathered` handed
// it); here is only what was chosen, where it is kept, and what it comes
// to as things stand.

import { computed, effect, signal } from '@preact/signals';
import { spacesRead, workspaces } from './spaces.ts';
import { active, channels, entered, me, projects, tabKey, tabs, unread } from './store.ts';
import type { Tab } from './store.ts';
import { ALL, heldOf, mostWaiting, sameHeld, spaceNear, standing, unseen, waitingIn } from './workspaces.ts';
import type { Held } from './workspaces.ts';

const SAVED = 'client.workspace.held';
const kept = (actor: string) => `${SAVED}:${actor}`;

/** What was chosen, as it was kept: nothing, or something that may have gone since. */
const asked = signal<Held>(ALL);
// Each person who signs in here has their own, read once it is known who that is.
effect(() => {
  const actor = me.value?.actor;
  if (!actor || typeof localStorage === 'undefined') {
    asked.value = ALL;
    return;
  }
  try {
    asked.value = heldOf(localStorage.getItem(kept(actor)));
  } catch {
    asked.value = ALL;
  }
});

/** What a tab is of: a project, or a channel, or neither. */
function of(tab: Tab | undefined): { project?: string | null; channel?: { id: string; project: string | null } | null } {
  if (!tab) return {};
  if (tab.kind === 'project') return { project: tab.id };
  if (tab.kind === 'channel') return { channel: channels.value.find((one) => one.id === tab.id) ?? null };
  return 'project' in tab ? { project: tab.project } : {};
}

/** The workspace of what is showing beside the panel, in the group last used: where what was chosen is gone, the panel goes there. */
const near = computed(() => spaceNear(of(tabs.value.find((tab) => tabKey(tab) === active.value)), workspaces.value, projects.value));

/** What the side panel is of as things stand: one workspace, and nothing only until the workspaces are read. */
export const held = computed<Held>(() => standing(asked.value, workspaces.value, near.value));

/**
 * What the panel is first of, as the page is read: what was chosen
 * while that workspace is there; else the workspace of `showing`, the
 * tab that was showing in what a browser kept before each workspace had
 * its own tabs; else the usual one. It is kept at once and not when
 * the tabs are back: the tabs that are put back are those of that
 * workspace, and one of another's that is showing among them must not
 * move the panel after they are.
 */
export function beginAt(showing: Tab | undefined): Held {
  const begins = standing(asked.peek(), workspaces.peek(), spaceNear(of(showing), workspaces.peek(), projects.peek()));
  if (begins.kind === 'space' && !sameHeld(asked.peek(), begins)) hold(begins);
  return begins;
}

/**
 * Has the side panel be of a workspace. The tabs that are open follow (`tabsof.ts`: each workspace has its own,
 * and nothing is closed by showing another's); nothing folded, grouped or pinned is touched.
 */
export function hold(next: Held): void {
  asked.value = next;
  const actor = me.value?.actor;
  if (!actor) return;
  try {
    if (next.kind === 'all') localStorage.removeItem(kept(actor));
    else localStorage.setItem(kept(actor), JSON.stringify(next));
  } catch {
    /* not remembered */
  }
}

// What the panel is of is kept as soon as it is known, and put right
// when what was kept is gone. Kept, it stays what it is whatever is
// opened beside it: a panel that followed the tab that is showing would
// change under whoever was reading it. And a workspace made later with
// the name of one that was taken away does not find itself chosen. Only
// once everything was read, and not where a reading failed: an empty
// list that was never read says nothing of what is gone.
effect(() => {
  if (!me.value || !spacesRead.value || !entered.value) return;
  if (!sameHeld(asked.value, held.value)) hold(held.value);
});

/** How many wait unread in channels the panel leaves out, whichever choice they are of. */
export const outOfSight = computed(() => unseen(channels.value, unread.value, held.value, workspaces.value, projects.value));

/** The workspace where the most wait unread out of sight, where any do: the count at the foot, pressed, shows it. */
export const mostOutOfSight = computed(() => mostWaiting(channels.value, unread.value, held.value, workspaces.value, projects.value));

/** How many wait unread in the channels of one choice. */
export function waiting(choice: Held): number {
  return waitingIn(channels.value, unread.value, choice, workspaces.value, projects.value);
}

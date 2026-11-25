// What whoever is signed in pinned and lately opened, as the page has it.
//
// What they pinned is kept with their account, with their groups of
// channels (`arrangedkept.ts`), so that it is the same in every window and
// browser they sign in to; what they lately opened is each browser's own,
// kept in it under who is signed in, as what is folded is. What the two
// lists are, and what is gone from them, is `lately.ts`; here is where
// what is kept is read and written, what tells it that something was
// opened, and what was read of what there is.

import { computed, effect, signal } from '@preact/signals';
import { thingsFrom, withThings } from './arranged.ts';
import { arrange, arrangement, goneThing, OLD_LATELY } from './arrangedkept.ts';
import { cleaned, forgot, isGone, isPinned, isThing, keptOf, NOTHING, opened, pin, unlisted } from './lately.ts';
import type { Kept, Known } from './lately.ts';
import { known as running } from './running.ts';
import { collections, entered, me, onOpened, projects } from './store.ts';
import { topics, topicsRead, wantTopics } from './topicsread.ts';
import type { Tab } from './tab.ts';

const kept = (actor: string) => `${OLD_LATELY}:${actor}`;

/** What was lately opened here. */
const recent = signal<Tab[]>([]);
// Each person who signs in here has their own, read once it is known who that is.
effect(() => {
  const actor = me.value?.actor;
  if (!actor || typeof localStorage === 'undefined') {
    recent.value = [];
    return;
  }
  try {
    recent.value = keptOf(localStorage.getItem(kept(actor))).recent;
  } catch {
    recent.value = [];
  }
});

/** Both lists as they are kept: what was lately opened here, and what is pinned, which is the account's. */
export const lately = computed<Kept>(() => {
  const pinned = thingsFrom(arrangement.value);
  return recent.value.length || pinned.length ? { recent: recent.value, pinned } : NOTHING;
});

/** Keeps what was lately opened here. The pins of `next` are not this browser's to keep. */
function keepRecent(next: Tab[]): void {
  if (next === recent.peek()) return;
  recent.value = next;
  const actor = me.peek()?.actor;
  if (!actor) return;
  try {
    // What a browser kept before of what was pinned is left where it is until it is taken up (`arrangedkept.ts`).
    const was = keptOf(localStorage.getItem(kept(actor)));
    localStorage.setItem(kept(actor), JSON.stringify(was.pinned.length ? { recent: next, pinned: was.pinned } : { recent: next }));
  } catch {
    /* not remembered */
  }
}

/** Keeps what a change to both lists came to: what was lately opened here, and the pins with the account. */
function keep(change: (now: Kept) => Kept): void {
  const now = lately.peek();
  const next = change(now);
  keepRecent(next.recent);
  if (next.pinned !== now.pinned) void arrange((was) => withThings(was, change({ recent: next.recent, pinned: thingsFrom(was) }).pinned));
}

// What somebody opens or shows is the latest thing they opened.
onOpened((tab) => {
  if (me.peek() && isThing(tab)) keepRecent(opened(lately.peek(), tab).recent);
});

/** Pins a thing, or takes its pin away. */
export function pinThing(thing: Tab, on: boolean): void {
  keep((now) => pin(now, thing, on));
}

/** Takes out of both lists a thing that whoever asks has just taken away. */
export function forgetThing(thing: Tab): void {
  keep((now) => forgot(now, thing));
}

/** Takes a thing out of what was lately opened, at its line's Remove from Recents: opened again, it is the latest. */
export function removeFromRecents(thing: Tab): void {
  keepRecent(unlisted(lately.peek(), thing).recent);
}

/** What the line of a menu that pins a thing, or takes its pin away, says and does. */
export function pinning(thing: Tab): { label: string; act: () => void } {
  const on = isPinned(lately.value, thing);
  return { label: on ? 'Unpin' : 'Pin to the top', act: () => pinThing(thing, !on) };
}

/** What was read of what is running: whose each is, or that it is gone. One told of before its project was asked for says nothing yet. */
function ran(): Known['running'] {
  const read: Known['running'] = {};
  for (const [id, what] of Object.entries(running.value)) {
    if (what === 'gone') read[id] = 'gone';
    else if (what.project !== undefined) read[id] = { project: what.project };
  }
  return read;
}

/** What was read of what there is: the projects, what each has declared, and what is running, as far as each was read. */
export const knownNow = computed<Known>(() => ({
  projects: entered.value ? projects.value.map((project) => project.id) : null,
  declared: Object.fromEntries(Object.entries(collections.value).map(([project, all]) => [project, all.map((one) => one.name)])),
  running: ran(),
  topics: topicsRead.value ? Object.fromEntries(topics.value.map((topic) => [topic.id, { workspace: topic.workspace, project: topic.project }])) : null,
}));

// A topic kept to hand is listed once the topics are read, which is otherwise done as the Topics panel is shown.
effect(() => {
  if (me.value && entered.value && [...lately.value.recent, ...lately.value.pinned].some((thing) => thing.kind === 'topic')) wantTopics();
});

// What is gone is taken out of what was lately opened when a reading says it is: a project forgotten, a table no
// longer declared, something running that names nothing any more. It follows the readings and not what is kept:
// something opened a moment before what declares it has been read is not taken for gone by having been opened. What
// is pinned is the account's, and is passed over where it is shown (`pinnedNow`): it may come back from the Trash.
// The browser's copy of it keeps nothing of what is gone.
effect(() => {
  const known = knownNow.value;
  if (!me.value || !entered.value) return;
  keepRecent(cleaned({ recent: recent.peek(), pinned: [] }, known).recent);
});
effect(() => {
  const known = knownNow.value;
  goneThing.value = entered.value ? (thing: Tab) => isGone(thing, known) : () => false;
});

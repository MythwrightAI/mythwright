// What the activity, the measurements and the agents show, as the page has
// it: of the workspace the side panel is of, what somebody chose there.
//
// The choice is whoever is looking's own, as which workspace the panel is
// of is (`chosen.ts`): it is kept in this browser under who is signed in,
// for each workspace, and the factory knows nothing of it. What a choice
// is, what it comes to and what is offered are `narrowing.ts`; here is
// only what was kept, and the three pages reading one thing from it.

import { computed, effect, signal } from '@preact/signals';
import { held, hold } from './chosen.ts';
import { chosenIn, cleaned, fromBefore, keptOf, lines, settled, shownBy, USUAL } from './narrowing.ts';
import type { Choice, Kept, Offered, Shown } from './narrowing.ts';
import { spacesAsked, spacesRead, workspaces } from './spaces.ts';
import { entered, me, narrowsBy, projects } from './store.ts';
import { spaceOf } from './workspaces.ts';

const SAVED = 'client.factory.shown';
const at = (actor: string) => `${SAVED}:${actor}`;
/** Where a browser kept the one project the factory's pages were narrowed to, when the choice was one for the whole factory. */
const BEFORE = 'client.factory.project';

/** What was chosen in each workspace, as it was kept: some of it may have gone since. */
const kept = signal<Kept>({});
// Each person who signs in here has their own, read once it is known who that is.
effect(() => {
  const actor = me.value?.actor;
  if (!actor || typeof localStorage === 'undefined') {
    kept.value = {};
    return;
  }
  try {
    kept.value = keptOf(localStorage.getItem(at(actor)));
  } catch {
    kept.value = {};
  }
});

function keep(next: Kept): void {
  kept.value = next;
  const actor = me.value?.actor;
  if (!actor) return;
  try {
    if (Object.keys(next).length === 0) localStorage.removeItem(at(actor));
    else localStorage.setItem(at(actor), JSON.stringify(next));
  } catch {
    /* not remembered */
  }
}

// What is shown is said as text first, so that it is another thing only
// when it comes to something else: a list of the same projects made again
// (the workspaces read once more, a project renamed) is nothing for a
// page to read again for.
const said = computed(() => JSON.stringify(shownBy(kept.value, held.value, workspaces.value, projects.value, spacesAsked.value)));
/** What the three pages show as things stand: nothing yet, everything, or the projects what was chosen comes to. */
export const shown = computed<Shown>(() => JSON.parse(said.value) as Shown);

/** What was chosen in the workspace the panel is of, as far as it can still be chosen. */
export const choice = computed<Choice>(() => {
  const of = held.value;
  return of.kind === 'space' ? settled(kept.value[of.id], of.id, workspaces.value, projects.value) : USUAL;
});

/** The lines the choice is offered as; `more` is whether the page that asks has something to show that the workspace's projects are not. */
export function offered(more: boolean): Offered | null {
  return lines(held.value, workspaces.value, projects.value, choice.value, more);
}

/** Chooses, in the workspace the panel is of. Nothing else changes by it. */
export function choose(next: Choice): void {
  const of = held.value;
  if (of.kind === 'space') keep(chosenIn(kept.value, of.id, next));
}

// One project asked for by name, from its own page (`Activity of Shop`):
// it is chosen in its own workspace, and the panel is put on that
// workspace where it was of another, since the three pages are of the
// workspace the panel is of and would otherwise go on showing the other.
// With no project it is all of the workspace showing.
narrowsBy((project) => {
  if (!project) return choose(USUAL);
  const space = spaceOf(project, workspaces.value);
  if (!space) return;
  keep(chosenIn(kept.value, space.id, { kind: 'project', id: project }));
  if (held.value.kind !== 'space' || held.value.id !== space.id) hold({ kind: 'space', id: space.id });
});

// What was kept is put right once everything was read, and not where a
// reading failed: an empty list that was never read says nothing of what
// is gone. What a browser kept from before the choice was a workspace's
// is taken in here, once, and is then no longer there to be read.
effect(() => {
  if (!me.value || !spacesRead.value || !entered.value) return;
  let before: string | null = null;
  try {
    before = localStorage.getItem(BEFORE);
    if (before !== null) localStorage.removeItem(BEFORE);
  } catch {
    /* nothing was kept */
  }
  const now = cleaned(fromBefore(kept.value, before, workspaces.value, projects.value), workspaces.value, projects.value);
  if (now !== kept.value) keep(now);
});

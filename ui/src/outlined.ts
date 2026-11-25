// Each project's outline as the page has it (`outline.ts`): read once a
// project is listed, and read again for a project when it is said to have
// been arranged (`project.arranged`, which carries no outline, since it
// may name what some who hear it may not read). The daemon hands it as
// whoever reads it may see it. One reading at a time for a project is
// kept, the one asked last.
//
// It is what the side panel goes by to draw a project's things in the
// folders somebody put them in, and what its menus change, a change at a
// time (`changeOutline`): drawn at once from the change as the daemon
// would make it (`apply`, held to the daemon's by `outlinecases.json`),
// and then as the daemon answers, which is the outline as whoever asked
// may see it. A reading asked before a change was answered says how it
// was, and is not kept.

import { effect, signal } from '@preact/signals';
import { call } from './api.ts';
import { stream } from './events.ts';
import { outlineFoldsGone } from './folds.ts';
import { apply, foldersOf, seenOf, shownAs } from './outline.ts';
import type { Op, Outline } from './outline.ts';
import { projects, turned, turnedTo } from './store.ts';

/** Each project's outline, by the project's id, as last read: nothing where nobody arranged it. */
export const outlines = signal<Record<string, Outline | null>>({});

/** `project`'s outline, as far as it was read. */
export const outlineOf = (project: string): Outline | null => outlines.value[project] ?? null;

/** What a thing of `project` (by its key: `board`, `library:docs`, …) is shown as for everybody, where somebody named it: nothing where it is shown as it always was. */
export const thingShown = (project: string, key: string): string | null => shownAs(outlineOf(project), key);

// Which reading of each project was last asked for, so that one in late says nothing.
const asked: Record<string, number> = {};

/** Reads `project`'s outline again. */
export async function readOutline(project: string): Promise<void> {
  const mine = (asked[project] = (asked[project] ?? 0) + 1);
  try {
    const read = await call<{ outline: Outline | null }>('GET', `/projects/${encodeURIComponent(project)}/outline`);
    if (mine !== asked[project]) return;
    outlines.value = { ...outlines.value, [project]: read.outline };
    // A folder that is not there any more is folded for whoever later makes one of its id.
    const gone = outlineFoldsGone(turned.value, project, foldersOf(read.outline?.items ?? []));
    if (gone.length > 0) turnedTo([...turned.value].filter((one) => !gone.includes(one)));
  } catch {
    // Not one whoever is signed in may read, or a daemon from before: it is drawn as it always was.
  }
}

// Each project listed is read once; one that is gone is let go of, and
// all of them as somebody signs out (the list is then empty).
effect(() => {
  const listed = projects.value.map((project) => project.id);
  const held = outlines.peek();
  const kept = Object.fromEntries(Object.entries(held).filter(([project]) => listed.includes(project)));
  if (Object.keys(kept).length !== Object.keys(held).length) outlines.value = kept;
  for (const project of listed) if (!(project in asked)) void readOutline(project);
  for (const project of Object.keys(asked)) if (!listed.includes(project)) delete asked[project];
});

stream.on('project.arranged', (event) => {
  const project = event.data.project;
  if (typeof project === 'string') void readOutline(project);
});

// Each project's changes, one after another.
const turns: Record<string, Promise<unknown>> = {};

/** What a change came to: the outline as it is now, and what was done (the folder made, what was removed). */
export interface Changed {
  outline: Outline | null;
  done: Record<string, unknown>;
}

/**
 * Changes `project`'s outline by `op`, sent with what the page draws at
 * the top that the outline does not name (`listed` is every line drawn,
 * in order: `keysOf`). Drawn at once, then as the daemon answers; what it
 * refuses is thrown in its sentence, the outline read again.
 */
export function changeOutline(project: string, op: Op, listed: string[]): Promise<Changed> {
  const done = (turns[project] ?? Promise.resolve()).then(async () => {
    const now = outlineOf(project);
    const seen = seenOf(listed, now);
    // A reading on its way says how it was before this change.
    asked[project] = (asked[project] ?? 0) + 1;
    const came = apply(now, op, seen);
    if ('wrong' in came) throw new Error(came.wrong);
    outlines.value = { ...outlines.value, [project]: came.outline };
    try {
      const answer = await call<Changed>('POST', `/projects/${encodeURIComponent(project)}/outline`, { ...op, seen });
      outlines.value = { ...outlines.value, [project]: answer.outline };
      return answer;
    } catch (refused) {
      void readOutline(project);
      throw refused;
    }
  });
  turns[project] = done.catch(() => {});
  return done;
}

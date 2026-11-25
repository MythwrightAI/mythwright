// Each project's environments as the page has them: read once a project
// is listed, and read again for a project when something is said of its
// environments (`project.environment.*`). What one is, and what is shown
// of it, is `projectenvs.ts`; the daemon answers with no secret's value.
//
// It is what the side panel goes by to list a project's Environments,
// what an apply goes by to ask which one, and what Activity goes by to
// say that something runs under none.

import { effect, signal } from '@preact/signals';
import { call } from './api.ts';
import { stream } from './events.ts';
import type { Environment } from './projectenvs.ts';
import { projects } from './store.ts';

/** Each project's environments, by the project's id, as last read. */
export const environmentsOf = signal<Record<string, Environment[]>>({});

/** The names of `project`'s environments, as far as they were read. */
export const namesOf = (project: string | null | undefined): string[] =>
  project ? (environmentsOf.value[project] ?? []).map((one) => one.name) : [];

// Which reading of each project was last asked for, so that one in late says nothing.
const asked: Record<string, number> = {};

/** Reads `project`'s environments again. */
export async function readEnvironments(project: string): Promise<void> {
  const mine = (asked[project] = (asked[project] ?? 0) + 1);
  try {
    const read = await call<{ environments: Environment[] }>('GET', `/projects/${encodeURIComponent(project)}/environments`);
    if (mine !== asked[project]) return;
    environmentsOf.value = { ...environmentsOf.value, [project]: read.environments };
  } catch {
    // Not one whoever is signed in may read: nothing is listed of it.
  }
}

// Each project listed is read once; one that is gone is let go of, and
// all of them as somebody signs out (the list is then empty).
effect(() => {
  const listed = projects.value.map((project) => project.id);
  const held = environmentsOf.peek();
  const kept = Object.fromEntries(Object.entries(held).filter(([project]) => listed.includes(project)));
  if (Object.keys(kept).length !== Object.keys(held).length) environmentsOf.value = kept;
  for (const project of listed) if (!(project in asked)) void readEnvironments(project);
  for (const project of Object.keys(asked)) if (!listed.includes(project)) delete asked[project];
});

stream.on('project.environment.*', (event) => {
  const project = event.data.project;
  if (typeof project === 'string') void readEnvironments(project);
});

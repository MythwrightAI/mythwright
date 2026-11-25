// What the factory's pages show: the activity, the measurements and the
// agents.
//
// The three are of the workspace the side panel is of, and what they
// show is a choice within it: all of the workspace, which is the usual
// and what there is with nothing chosen; one project of it; or every
// workspace, which is everything the factory has, what was applied for
// no project and the factory's own figures with it. A project of
// another workspace is not a choice: the panel is put on that workspace,
// at its foot, and the pages follow.
//
// What somebody chose is theirs and is kept for each workspace, so that
// going to another and back finds each as it was left. It is kept in the
// browser (`narrowed.ts`) and asked of nobody: the daemon is never told
// which workspace anybody is looking at, and no route takes one. What a
// page asks for is the projects a choice comes to, and those are worked
// out here, from what was read and what was kept, with nothing of the
// page in it.

import { holdsProject, spaceOf } from './workspaces.ts';
import type { Held, Workspace } from './workspaces.ts';

/** What was chosen within a workspace: all of it, one of its projects, or every workspace. */
export type Choice = { kind: 'space' } | { kind: 'project'; id: string } | { kind: 'everything' };

/** All of the workspace: what there is where nothing was chosen. */
export const USUAL: Choice = { kind: 'space' };

/** What was chosen, by the workspace it was chosen in (its id). A workspace that is not there has all of itself shown. */
export type Kept = Record<string, Choice>;

/**
 * What the pages show as things stand: nothing yet, while it is not
 * known which workspace the panel is of; everything; or some projects,
 * which are those of the workspace (`space`, its id) or the one chosen
 * of them (`project`).
 */
export type Shown = { kind: 'waiting' } | { kind: 'everything' } | { kind: 'projects'; projects: string[]; space: string; project: string | null };

function choiceOf(read: unknown): Choice | null {
  const { kind, id } = (typeof read === 'object' && read !== null ? read : {}) as { kind?: unknown; id?: unknown };
  if (kind === 'everything') return { kind };
  return kind === 'project' && typeof id === 'string' && id ? { kind, id } : null;
}

/** What a browser kept. Anything that is no choice is nothing chosen, which is all of the workspace and is not kept. */
export function keptOf(saved: string | null | undefined): Kept {
  if (!saved) return {};
  let read: unknown;
  try {
    read = JSON.parse(saved);
  } catch {
    return {};
  }
  if (typeof read !== 'object' || read === null || Array.isArray(read)) return {};
  const kept: Kept = {};
  for (const [space, one] of Object.entries(read)) {
    const choice = choiceOf(one);
    if (choice) kept[space] = choice;
  }
  return kept;
}

/** What was chosen with one more choice made in a workspace. All of the workspace is nothing kept. */
export function chosenIn(kept: Kept, space: string, choice: Choice): Kept {
  const { [space]: _, ...rest } = kept;
  return choice.kind === 'space' ? rest : { ...rest, [space]: choice };
}

/**
 * What a choice made in a workspace comes to as things stand: itself
 * while it can still be made, and otherwise all of the workspace. A
 * project can be chosen while it is one there is and is of that
 * workspace: forgotten, or put in another, it is no longer what the
 * pages of this one are of.
 */
export function settled(choice: Choice | undefined, space: string, all: Workspace[], projects: { id: string }[]): Choice {
  if (!choice || choice.kind === 'space') return USUAL;
  if (choice.kind === 'everything') return choice;
  const there = projects.some((project) => project.id === choice.id);
  return there && spaceOf(choice.id, all)?.id === space ? choice : USUAL;
}

/**
 * What was kept, with what can no longer be chosen taken out: the choice
 * of a workspace that is gone, and a project that is gone or is another
 * workspace's now. It is asked only once the workspaces and the projects
 * were read: an empty list that was never read says nothing of what is
 * gone. The same thing is handed back where nothing is taken out.
 */
export function cleaned(kept: Kept, all: Workspace[], projects: { id: string }[]): Kept {
  const still: Kept = {};
  for (const [space, choice] of Object.entries(kept)) {
    if (!all.some((one) => one.id === space)) continue;
    const now = settled(choice, space, all, projects);
    if (now.kind !== 'space') still[space] = now;
  }
  return Object.keys(still).length === Object.keys(kept).length ? kept : still;
}

/**
 * What the pages show: of the workspace the panel is of (`held`), what
 * was chosen there. While the panel is of nothing, it is not known yet
 * (`waiting`), until the reading of the workspaces is `over`: where
 * there was none to read, or it could not be read, there is no workspace
 * to be of and everything is shown.
 */
export function shownBy(kept: Kept, held: Held, all: Workspace[], projects: { id: string }[], over: boolean): Shown {
  if (held.kind === 'all') return over ? { kind: 'everything' } : { kind: 'waiting' };
  const choice = settled(kept[held.id], held.id, all, projects);
  if (choice.kind === 'everything') return { kind: 'everything' };
  if (choice.kind === 'project') return { kind: 'projects', projects: [choice.id], space: held.id, project: choice.id };
  const its = projects.filter((project) => holdsProject(held, project.id, all)).map((project) => project.id);
  return { kind: 'projects', projects: its, space: held.id, project: null };
}

/** What `Shown` comes to for whatever asks by project: the projects, or nothing where everything is shown. Asked only of what is not waiting. */
export function only(shown: Shown): string[] | null {
  return shown.kind === 'projects' ? shown.projects : null;
}

/**
 * What a browser kept before the pages were of a workspace: one project
 * for the whole factory. It is read as the choice of that project's own
 * workspace, where nothing was chosen there since, and never as
 * another's: taken for the choice of whichever workspace is showing, it
 * would show a project of another workspace there, which is what the
 * choice is now kept by workspace to end. A project that is gone was
 * nothing chosen.
 */
export function fromBefore(kept: Kept, project: string | null | undefined, all: Workspace[], projects: { id: string }[]): Kept {
  if (!project || !projects.some((one) => one.id === project)) return kept;
  const space = spaceOf(project, all);
  return !space || kept[space.id] ? kept : { ...kept, [space.id]: { kind: 'project', id: project } };
}

/** A line of the choice as it is offered: what it is called, what choosing it chooses, and what it is written as in the list. */
export interface Line {
  value: string;
  label: string;
  choice: Choice;
}

/** What all of the workspace is written as in the list: no project is called nothing. */
export const SPACE = '';
/** What every workspace is written as: no project's id has a `*` in it. */
export const EVERYTHING = '*';

export function valueOf(choice: Choice): string {
  return choice.kind === 'project' ? choice.id : choice.kind === 'everything' ? EVERYTHING : SPACE;
}

/** What is offered: the workspace first, then its projects, then every workspace where that is offered. */
export interface Offered {
  space: Line;
  projects: Line[];
  everything: Line | null;
}

/**
 * The lines of the choice, for the workspace the panel is of: the
 * workspace itself by its name, which is what is chosen with nothing
 * kept; each project of it, as the projects were given; and last every
 * workspace. Projects of other workspaces are not there: the panel is
 * put on another workspace at its foot.
 *
 * Every workspace is offered where choosing it would show something all
 * of this workspace does not: there is another workspace, or the page
 * says it has `more` to show than this one's projects (something
 * applied for no project, the factory's own log and figures). Where the
 * two would show the same it is not a line, unless it is what is chosen.
 * Nothing is offered while the panel is of no workspace.
 */
export function lines<P extends { id: string; name: string }>(held: Held, all: Workspace[], projects: P[], choice: Choice, more: boolean): Offered | null {
  const space = held.kind === 'space' ? all.find((one) => one.id === held.id) : null;
  if (!space) return null;
  const its = projects.filter((project) => holdsProject(held, project.id, all));
  const beyond = all.length > 1 || more || choice.kind === 'everything';
  return {
    space: { value: SPACE, label: space.name, choice: USUAL },
    projects: its.map((project) => ({ value: project.id, label: project.name, choice: { kind: 'project', id: project.id } })),
    everything: beyond ? { value: EVERYTHING, label: 'All workspaces', choice: { kind: 'everything' } } : null,
  };
}

/** The choice a line of the list stands for, by what it is written as. */
export function chosenBy(value: string, offered: Offered): Choice {
  return [offered.space, ...offered.projects, ...(offered.everything ? [offered.everything] : [])].find((line) => line.value === value)?.choice ?? USUAL;
}

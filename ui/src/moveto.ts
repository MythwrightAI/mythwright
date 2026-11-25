// Where something of the side panel can be moved to, and what is asked
// before it is: a project to another workspace, a channel to a workspace
// or a project. Its row's menu has one line for it, `Move to…`, which
// opens these as a menu beside it (`views/spaces.tsx`); choosing one asks
// the question here, which says what moving it means and what it does
// not. A workspace is grouping (`workspaces.ts`): it says where a thing
// is listed, for everybody, and lets nobody in and keeps nobody out.
//
// Holds nothing of the page, so that its tests can read it.

import type { Workspace } from './workspaces.ts';
import { belongsTo, spaceOf } from './workspaces.ts';

/**
 * A line of what something can be moved to, in the order it is offered:
 * a workspace or a project (`here` where it is there already, which is
 * marked and is nothing to press), a heading that only says, and a new
 * workspace, set apart at the end.
 */
export type Place =
  | { kind: 'workspace'; id: string; name: string; here: boolean }
  | { kind: 'project'; id: string; name: string; here: boolean }
  | { kind: 'heading'; said: string }
  | { kind: 'new' };

/** Where a project can be moved: each workspace, the one it is in marked, and then a new one. It is always in one, so there is no line that takes it out of all. */
export function projectPlaces(project: string, all: Workspace[]): Place[] {
  const now = spaceOf(project, all);
  return [...all.map((one): Place => ({ kind: 'workspace', id: one.id, name: one.name, here: now?.id === one.id })), { kind: 'new' }];
}

/**
 * Where a channel can be moved: each workspace, as its own, and then
 * under a heading each project. What it belongs to now is marked. It
 * always belongs to one of them, so there is no line for nothing; and
 * nothing is offered before the workspaces are read, since what it
 * belongs to is not known until then.
 */
export function channelPlaces(channel: { id: string; project: string | null }, all: Workspace[], projects: { id: string; name: string }[]): Place[] {
  if (all.length === 0) return [];
  const now = belongsTo(channel as Parameters<typeof belongsTo>[0], all, projects);
  const places: Place[] = all.map((one) => ({ kind: 'workspace', id: one.id, name: one.name, here: 'workspace' in now && now.workspace === one.id }));
  if (projects.length === 0) return places;
  return [...places, { kind: 'heading', said: 'Projects' }, ...projects.map((one): Place => ({ kind: 'project', id: one.id, name: one.name, here: 'project' in now && now.project === one.id }))];
}

/** What is asked before something is moved: a heading that names it and where it goes, and what moving it means. */
export interface Question {
  title: string;
  says: string;
}

/** What is so of every move, whatever it is of: it is where a thing is listed, and nothing more. */
const ONLY = 'nobody is let in or kept out by where it is listed';

/** Asked before a project is put in another workspace. */
export function projectQuestion(project: string, to: string): Question {
  return {
    title: `Move ${project} to ${to}?`,
    says: `${project} is listed in ${to} from now on, for everybody, and the channels that are its go with it. Nothing else of it changes: its board, its tables, its files and what runs for it stay as they are, and ${ONLY}.`,
  };
}

/**
 * Asked before a channel is moved: to a workspace, as that one's own; or
 * to a project, which it is listed with in the project's workspace and
 * whose the factory's bot is there. `was` is the project it is of now,
 * where it is one's.
 */
export function channelQuestion(channel: string, to: { kind: 'workspace'; name: string } | { kind: 'project'; name: string; space: string | null }, was: string | null = null): Question {
  const rest = `Nothing else of it changes: what was said in it stays, and ${ONLY}.`;
  if (to.kind === 'project') {
    const where = to.space ? `, listed with it in ${to.space}` : '';
    return { title: `Move ${channel} to ${to.name}?`, says: `${channel} is ${to.name}'s from now on${where}, and the factory's bot there is ${to.name}'s. ${rest}` };
  }
  const left = was ? ` It is ${was}'s no longer, and the factory's bot there is no project's.` : '';
  return { title: `Move ${channel} to ${to.name}?`, says: `${channel} is ${to.name}'s own from now on, and is listed there for everybody.${left} ${rest}` };
}

// What somebody keeps to hand in the side panel: what they pinned, and
// what they lately opened.
//
// Both are theirs: what they pinned is kept with their account, with their
// groups of channels, and what they lately opened in this browser under
// who is signed in (`latelykept.ts`); the daemon acts on neither. Here is what the two lists are worked out from, with nothing
// of the page in it: what was kept, what was read (which things there
// are), and which workspace the panel is of.
//
// A thing is anything of the factory's work that opens as a tab and is
// not always listed: a project's page, its board, a table, a library,
// mockups, its files, one file, a blueprint, something running, its
// pipelines, a forum's topic, which is listed in the workspace
// of its place as a channel would be (`holdsTopic`). One run is not: it is one of a great many records of
// something having happened, listed on its pipeline's page and its
// instance's, and whose it is would have to be read for each. The
// factory's own pages (activity, measurements, members, the Trash) are not:
// they are lines of the panel that are always there, directly under
// these lists, and one listed again above itself would be the same line
// twice. A channel is not one either: channels have a panel of their
// own, with pins of their own. Nor is what is no work of the factory's:
// the start page, settings, a package's page.

import type { Tab } from './tab.ts';
import { tabKey } from './tab.ts';
import { holdsProject, holdsTopic } from './workspaces.ts';
import type { Held, Workspace } from './workspaces.ts';

/** How many of what was lately opened are listed. */
export const MOST = 8;
/**
 * How many are kept: more than are listed, since the list is of one
 * workspace at a time and what was opened in another is still what was
 * lately opened there.
 */
export const KEPT = 48;

export interface Kept {
  /** What was opened, the latest first, each once. */
  recent: Tab[];
  /** What was pinned, in the order it was pinned. */
  pinned: Tab[];
}

export const NOTHING: Kept = { recent: [], pinned: [] };

const text = (value: unknown): value is string => typeof value === 'string' && value !== '';

/** What was kept of one thing, as a tab: nothing where it is not one that is kept, or makes no sense. */
function thingOf(read: unknown): Tab | null {
  if (typeof read !== 'object' || read === null) return null;
  const { kind, id, project, name, path } = read as Record<string, unknown>;
  switch (kind) {
    case 'project':
    case 'instance':
    case 'topic':
      return text(id) ? { kind, id } : null;
    case 'board':
    case 'repo':
    case 'git':
    case 'environments':
    case 'pipelines':
      return text(project) ? { kind, project } : null;
    case 'library':
    case 'mockups':
    case 'table':
      return text(project) && text(name) ? { kind, project, name } : null;
    case 'file':
      return text(project) && text(path) ? { kind, project, path } : null;
    case 'blueprint':
      return text(name) ? (text(project) ? { kind, name, project } : { kind, name }) : null;
    default:
      return null;
  }
}

/** Whether something that opens as a tab is one of the things kept to hand. */
export function isThing(tab: Tab): boolean {
  return thingOf(tab) !== null;
}

const once = (things: Tab[]): Tab[] => {
  const seen = new Set<string>();
  return things.filter((thing) => !seen.has(tabKey(thing)) && seen.add(tabKey(thing)));
};

/** What a browser kept, read. One from before, which kept neither, has none; and what of it makes no sense is left out. */
export function keptOf(saved: string | null | undefined): Kept {
  if (!saved) return NOTHING;
  let read: unknown;
  try {
    read = JSON.parse(saved);
  } catch {
    return NOTHING;
  }
  if (typeof read !== 'object' || read === null) return NOTHING;
  const { recent, pinned } = read as { recent?: unknown; pinned?: unknown };
  const things = (list: unknown) => once((Array.isArray(list) ? list : []).flatMap((one) => thingOf(one) ?? []));
  return { recent: things(recent).slice(0, KEPT), pinned: things(pinned) };
}

/** Something was opened: it is the latest, and is there once. What is no thing changes nothing. */
export function opened(kept: Kept, tab: Tab): Kept {
  const thing = thingOf(tab);
  if (!thing) return kept;
  const key = tabKey(thing);
  if (kept.recent[0] && tabKey(kept.recent[0]) === key) return kept;
  return { ...kept, recent: [thing, ...kept.recent.filter((one) => tabKey(one) !== key)].slice(0, KEPT) };
}

/** Whether a thing is pinned. */
export function isPinned(kept: Kept, tab: Tab): boolean {
  const key = tabKey(tab);
  return kept.pinned.some((one) => tabKey(one) === key);
}

/**
 * A thing pinned, after those pinned before it, or no longer pinned. When
 * it was last opened is kept either way, so that one that is unpinned is
 * among what was lately opened where it belongs; one that was never
 * opened is the latest, since pinning it was looking at it.
 */
export function pin(kept: Kept, tab: Tab, on: boolean): Kept {
  const thing = thingOf(tab);
  if (!thing) return kept;
  const key = tabKey(thing);
  const rest = kept.pinned.filter((one) => tabKey(one) !== key);
  if (!on) return { ...kept, pinned: rest };
  if (rest.length !== kept.pinned.length) return kept;
  const known = kept.recent.some((one) => tabKey(one) === key);
  return { recent: known ? kept.recent : [thing, ...kept.recent].slice(0, KEPT), pinned: [...rest, thing] };
}

/**
 * A thing taken out of both lists, by whoever took the thing itself away
 * and so knows it is gone: a file deleted here is known to be gone at
 * once, where a reading says so of a file only when its project goes.
 */
export function forgot(kept: Kept, tab: Tab): Kept {
  const key = tabKey(tab);
  const [recent, pinned] = [kept.recent.filter((one) => tabKey(one) !== key), kept.pinned.filter((one) => tabKey(one) !== key)];
  return recent.length === kept.recent.length && pinned.length === kept.pinned.length ? kept : { recent, pinned };
}

/**
 * A thing taken out of what was lately opened by whoever pressed Remove
 * from Recents on its line. It is still there, and opened again it is the
 * latest, as anything opened is. What is pinned is not in Recents, and is
 * left pinned: its line offers no such thing.
 */
export function unlisted(kept: Kept, tab: Tab): Kept {
  if (isPinned(kept, tab)) return kept;
  const key = tabKey(tab);
  const recent = kept.recent.filter((one) => tabKey(one) !== key);
  return recent.length === kept.recent.length ? kept : { ...kept, recent };
}

/** What was read of what there is. What was not read is not here, and says nothing of what is gone. */
export interface Known {
  /** The projects there are, by id. Nothing until they are read. */
  projects: string[] | null;
  /** What each project has declared, by its name (its board is `board`), for the projects that were read. */
  declared: Record<string, string[]>;
  /** What is running, by id: whose it is, or that it is gone, for those that were read. */
  running: Record<string, { project: string | null } | 'gone'>;
  /** The topics there are, by id, with where each is: nothing until they are read. */
  topics?: Record<string, { workspace: string | null; project: string | null }> | null;
}

/** The project a thing is of: none for what is the factory's own, and nothing known (`undefined`) for something running that was not read yet. */
export function projectOf(thing: Tab, known: Known): string | null | undefined {
  switch (thing.kind) {
    case 'project':
      return thing.id;
    case 'instance': {
      const what = known.running[thing.id];
      return what === undefined || what === 'gone' ? undefined : what.project;
    }
    case 'board':
    case 'repo':
    case 'git':
    case 'environments':
    case 'pipelines':
    case 'library':
    case 'mockups':
    case 'table':
    case 'file':
      return thing.project;
    case 'blueprint':
      return thing.project ?? null;
    default:
      return null;
  }
}

/**
 * Whether a thing is gone, as far as was read: its project forgotten, a
 * table (a board, a library, mockups) no longer declared, something
 * running that names nothing any more. A file goes with its project and
 * not before: the list of a project's files is read a moment after a
 * file is made, and one just written and opened would be taken for gone.
 */
export function isGone(thing: Tab, known: Known): boolean {
  if (thing.kind === 'instance') return known.running[thing.id] === 'gone';
  if (thing.kind === 'topic') return !!known.topics && !(thing.id in known.topics);
  const project = projectOf(thing, known);
  if (!project) return false;
  if (known.projects !== null && !known.projects.includes(project)) return true;
  const declared = known.declared[project];
  if (!declared) return false;
  if (thing.kind === 'board') return !declared.includes('board');
  return (thing.kind === 'table' || thing.kind === 'library' || thing.kind === 'mockups') && !declared.includes(thing.name);
}

/** What is kept without what is gone: the same thing where nothing is. */
export function cleaned(kept: Kept, known: Known): Kept {
  const there = (thing: Tab) => !isGone(thing, known);
  const [recent, pinned] = [kept.recent.filter(there), kept.pinned.filter(there)];
  return recent.length === kept.recent.length && pinned.length === kept.pinned.length ? kept : { recent, pinned };
}

/**
 * Whether a thing is listed while the panel is of `held`: what is of one
 * of that workspace's projects, and what is of no project, which is the
 * factory's own.
 * Something running whose project was not read yet is not listed until it
 * is, so that nothing of another workspace is shown for a moment.
 */
function listedIn(thing: Tab, known: Known, held: Held, all: Workspace[]): boolean {
  if (isGone(thing, known)) return false;
  // A topic is a workspace's or a project's: listed in the workspace of its place, once the topics are read.
  if (thing.kind === 'topic') {
    const topic = known.topics?.[thing.id];
    return !!topic && holdsTopic(held, topic, all, (known.projects ?? []).map((id) => ({ id })));
  }
  const project = projectOf(thing, known);
  if (project === undefined) return false;
  return project === null || holdsProject(held, project, all);
}

/** What is pinned, as it is listed: in the order it was pinned, bounded as the panel is. */
export function pinnedNow(kept: Kept, known: Known, held: Held, all: Workspace[]): Tab[] {
  return kept.pinned.filter((thing) => listedIn(thing, known, held, all));
}

/** Everything that was lately opened and could be listed: the latest first, bounded as the panel is, without what is pinned. */
export function recentAll(kept: Kept, known: Known, held: Held, all: Workspace[]): Tab[] {
  return kept.recent.filter((thing) => !isPinned(kept, thing) && listedIn(thing, known, held, all));
}

/** What was lately opened, as it is listed: the latest of those, and no more than `MOST`. */
export function recentNow(kept: Kept, known: Known, held: Held, all: Workspace[]): Tab[] {
  return recentAll(kept, known, held, all).slice(0, MOST);
}

/**
 * The list as it is drawn while somebody is pointing at the panel: what
 * it listed when they began (`was`, by key), in that order, so that a
 * line pressed does not jump to the top from under the pointer, and
 * nothing that is opened meanwhile comes into the list and pushes what
 * is under the pointer down. Only what can no longer be listed at all
 * (it is gone, or was pinned) is gone from it. `could` is everything
 * that could be listed (`recentAll`): the list is put right from that
 * when the pointer has left.
 */
export function heldStill(was: string[], could: Tab[]): Tab[] {
  const by = new Map(could.map((thing) => [tabKey(thing), thing]));
  return was.flatMap((key) => by.get(key) ?? []);
}

export interface Line {
  thing: Tab;
  key: string;
  /** What its tab is called. */
  title: string;
  /** Whose it is, said beside the name where another line reads the same; nothing otherwise. */
  whose: string | null;
}

/**
 * The lines of both lists as they are said: each with what its tab is
 * called, and, where two of them read alike, whose each is. `whose` gives
 * that for a thing (its project's name), or nothing for what is nobody's.
 */
export function said(things: Tab[], title: (thing: Tab) => string, whose: (thing: Tab) => string | null): Line[] {
  const titles = things.map(title);
  return things.map((thing, at) => ({
    thing,
    key: tabKey(thing),
    title: titles[at]!,
    whose: titles.filter((one) => one === titles[at]).length > 1 ? whose(thing) : null,
  }));
}

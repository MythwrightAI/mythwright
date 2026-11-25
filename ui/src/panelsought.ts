// What a side panel's box comes to: which of what the panel lists is
// shown while something is typed in it, and which folds are held open
// for it.
//
// Each of the three panels that list things (Resources, Channels, the
// inbox) has one box under its head, and what is typed there narrows
// everything that panel lists. Nothing is asked of the daemon for it: it
// is held to what the panel has already read. The words are looked for
// as the factory's blueprints have always been looked for
// (`wordsOf` and `says` in `blueprintlist.ts`): every word typed, in
// what something says, whatever case it is written in.
//
// What a panel lists is handed here as plain values: a tree of things,
// each with what it says and what is under it. A thing is shown where it
// says every word itself; then what is under it is shown as it is, folded
// as it was left. A thing that does not, but something under it does, is
// shown with only what does under it, held open for as long as something
// is typed. A thing may carry what it is called down to what is under it
// (a project, a folder, a group), so that `shop board` finds the board of
// Shop; what is only a heading of a kind (Repo, Blueprints) carries
// nothing, or every file would be found by `repo`.
//
// The factory's blueprints keep their own rule (`sought`), which is the
// same words over a list in two levels.

import { says, wordsOf } from './blueprintlist.ts';
import { fromOf, whereOf, wordsOf as wordsSaid } from './inbox.ts';
import type { Line } from './inbox.ts';
import { calledOf, hashed } from './kept.ts';
import { keysOf, laidOut } from './outline.ts';
import type { Laid, Outline } from './outline.ts';
import { shown } from './shortcuts.ts';

/** One thing a panel lists, with what is under it. */
export interface Item {
  /** What it is known by in what is found: unique in the panel. */
  id: string;
  /** What it says: what it is called, first, and anything else it is found by. */
  says: (string | null | undefined)[];
  /** What it is called, said for what is under it too. */
  carries?: string;
  under?: Item[];
}

/** What a panel shows while something is typed. */
export interface Found {
  /** Every thing that is shown, by its id. */
  shown: Set<string>;
  /** What is held open, whatever was kept of its fold: what something under it was found in. */
  held: Set<string>;
}

/** The panel held to what is typed; nothing where nothing is, which is the whole panel as it was. */
export function narrowed(items: Item[], typed: string): Found | null {
  const words = wordsOf(typed);
  if (words.length === 0) return null;
  const found: Found = { shown: new Set(), held: new Set() };
  const whole = (item: Item) => {
    found.shown.add(item.id);
    for (const under of item.under ?? []) whole(under);
  };
  const visit = (item: Item, above: string[]): boolean => {
    if (says(words, [...item.says, ...above])) {
      whole(item);
      return true;
    }
    const carried = item.carries ? [...above, item.carries] : above;
    let any = false;
    for (const under of item.under ?? []) if (visit(under, carried)) any = true;
    if (any) {
      found.shown.add(item.id);
      found.held.add(item.id);
    }
    return any;
  };
  for (const item of items) visit(item, []);
  return found;
}

/** Whether a thing is shown: everything is, while nothing is typed. */
export function shows(found: Found | null, id: string): boolean {
  return found === null || found.shown.has(id);
}

/** Whether a fold is held open by what was typed. */
export function holds(found: Found | null, id: string): boolean {
  return found !== null && found.held.has(id);
}

/** Whether anything at all is shown of what was handed. */
export function anyShown(found: Found | null): boolean {
  return found === null || found.shown.size > 0;
}

// ---- Resources ------------------------------------------------------------------

/** A thing of a project's that is a line under it: its name, and what it is called there. */
export interface Named {
  name: string;
  label: string;
}

/** A file or a folder of a project, as the page has read it. */
export interface Filed {
  path: string;
  name: string;
  dir: boolean;
}

/** What a project lists under it, as the panel has read it. */
export interface ProjectHas {
  id: string;
  name: string;
  board: boolean;
  tables: Named[];
  libraries: Named[];
  mockups: Named[];
  /** The blueprints it uses, with what each says it is. */
  blueprints: { name: string; description?: string | null }[];
  /** Its files where its Repo line is listed, as far as they were read; nothing where it has no such line. */
  files: Filed[] | null;
  /** The names of its environments, where it has any. */
  environments?: string[];
  /** Whether its folder is a git repository of its own, which has a Git page. */
  git?: boolean;
  /** Whether it has pipelines of its own (`pipelines.ts`), and so a line for them. */
  pipelines?: boolean;
  /** How its things are arranged, where somebody arranged them (`outline.ts`). */
  outline?: Outline | null;
}

/** What each line of a project's tree is known by, in what is found. */
export const ofProject = {
  project: (id: string) => `project:${id}`,
  board: (id: string) => `project:${id}:board`,
  table: (id: string, name: string) => `project:${id}:table:${name}`,
  library: (id: string, name: string) => `project:${id}:library:${name}`,
  mockups: (id: string, name: string) => `project:${id}:mockups:${name}`,
  blueprints: (id: string) => `project:${id}:blueprints`,
  blueprint: (id: string, name: string) => `project:${id}:blueprint:${name}`,
  files: (id: string) => `project:${id}:repo`,
  git: (id: string) => `project:${id}:git`,
  environments: (id: string) => `project:${id}:environments`,
  pipelines: (id: string) => `project:${id}:pipelines`,
  entry: (id: string, path: string) => `project:${id}:entry:${path}`,
  /** A folder of its outline. */
  folder: (id: string, folder: string) => `project:${id}:outline:${folder}`,
  /** A thing of its outline, by its key: the same as the line's own (`board`, `table:<name>`, …, and `repo` for its files). */
  thing: (id: string, key: string) => `project:${id}:${key}`,
};

const parentOf = (path: string) => (path.includes('/') ? path.slice(0, path.lastIndexOf('/')) : '');

/** A project's files as a tree, from the flat list the page read: each folder carries its name to what is in it. */
function filesUnder(id: string, files: Filed[], under: string): Item[] {
  return files
    .filter((entry) => parentOf(entry.path) === under)
    .map((entry) =>
      entry.dir
        ? { id: ofProject.entry(id, entry.path), says: [entry.name], carries: entry.name, under: filesUnder(id, files, entry.path) }
        : { id: ofProject.entry(id, entry.path), says: [entry.name] },
    );
}

/**
 * A project and everything listed under it, as it is drawn (`laidOut`):
 * where it was arranged, each folder of its outline carries its name to
 * what is in it, so that `insights runs` finds a table of runs in
 * Insights and holds Insights open for it. A thing somebody named
 * something else is found by that name too.
 */
export function projectItem(has: ProjectHas): Item {
  const { id } = has;
  const lines = new Map<string, Item>();
  if (has.board) lines.set('board', { id: ofProject.board(id), says: ['Board'] });
  for (const one of has.tables) lines.set(`table:${one.name}`, { id: ofProject.table(id, one.name), says: [one.label, one.name] });
  for (const one of has.libraries) lines.set(`library:${one.name}`, { id: ofProject.library(id, one.name), says: [one.label, one.name] });
  for (const one of has.mockups) lines.set(`mockups:${one.name}`, { id: ofProject.mockups(id, one.name), says: [one.label, one.name] });
  if (has.environments && has.environments.length > 0) {
    lines.set('environments', { id: ofProject.environments(id), says: ['Environments', ...has.environments] });
  }
  if (has.pipelines) lines.set('pipelines', { id: ofProject.pipelines(id), says: ['Pipelines'] });
  if (has.blueprints.length > 0) {
    lines.set('blueprints', {
      id: ofProject.blueprints(id),
      says: ['Blueprints'],
      under: has.blueprints.map((one) => ({ id: ofProject.blueprint(id, one.name), says: [one.name, one.description] })),
    });
  }
  if (has.git) lines.set('git', { id: ofProject.git(id), says: ['Git'] });
  if (has.files) lines.set('repo', { id: ofProject.files(id), says: ['Repo'], under: filesUnder(id, has.files, '') });
  const listed = keysOf({
    board: lines.has('board'),
    tables: has.tables.map((one) => one.name),
    libraries: has.libraries.map((one) => one.name),
    mockups: has.mockups.map((one) => one.name),
    environments: lines.has('environments'),
    pipelines: lines.has('pipelines'),
    blueprints: lines.has('blueprints'),
    git: lines.has('git'),
    repo: lines.has('repo'),
  });
  const items = (laid: Laid[]): Item[] =>
    laid.map((one) =>
      'folder' in one
        ? { id: ofProject.folder(id, one.folder), says: [one.name], carries: one.name, under: items(one.items) }
        : // A thing somebody named is found by that name, and by what it always said.
          withName(lines.get(one.thing)!, one.name),
    );
  return { id: ofProject.project(id), says: [has.name], carries: has.name, under: items(laidOut(listed, has.outline ?? null)) };
}

const withName = (line: Item, name: string | undefined): Item => (name ? { ...line, says: [name, ...line.says] } : line);

/** How many matches under one project's Repo are drawn at first, and how many more each press of the line after them draws. */
export const MOST_FILES = 200;

/** The order a project's files are drawn in: folders first, then blueprints, then by name. */
export function drawnFirst(isBlueprint: (path: string) => boolean): (a: Filed, b: Filed) => number {
  return (a, b) => Number(b.dir) - Number(a.dir) || Number(isBlueprint(b.path)) - Number(isBlueprint(a.path)) || a.name.localeCompare(b.name);
}

/** What of a project's files is drawn while something is typed, and how many matches are left for the line after them. */
export interface Bounded {
  /** The entries drawn, by their ids. */
  kept: Set<string>;
  /** How many files that are found are not drawn yet. */
  more: number;
}

/**
 * A project's files held to the first `most` that are found, in the order
 * they are drawn: a search that finds thousands of files draws a few
 * hundred and says how many more there are. A folder held open only for
 * what is under it is drawn where something under it is; one found by its
 * own name is drawn while there is room as it is come to. Nothing where
 * nothing is typed: then nothing is bounded.
 */
export function filesBounded(id: string, files: Filed[], found: Found | null, most: number, order: (a: Filed, b: Filed) => number): Bounded | null {
  if (!found) return null;
  const by = new Map<string, Filed[]>();
  for (const entry of files) by.set(parentOf(entry.path), [...(by.get(parentOf(entry.path)) ?? []), entry]);
  const kept = new Set<string>();
  let [drawn, more] = [0, 0];
  const walk = (under: string): boolean => {
    let any = false;
    for (const entry of [...(by.get(under) ?? [])].sort(order)) {
      const key = ofProject.entry(id, entry.path);
      if (!found.shown.has(key)) continue;
      if (entry.dir) {
        const room = drawn < most;
        const inside = walk(entry.path);
        if (inside || (room && !found.held.has(key))) {
          kept.add(key);
          any = true;
        }
      } else if (drawn < most) {
        kept.add(key);
        drawn += 1;
        any = true;
      } else more += 1;
    }
    return any;
  };
  walk('');
  return { kept, more };
}

/** What the line after the files drawn says. */
export function moreSaid(more: number): string {
  return more === 1 ? '1 more file found' : `${more} more files found`;
}

/** A line kept to hand, pinned or lately opened: what its tab is called, and whose it is. */
export interface KeptLine {
  key: string;
  title: string;
  whose: string | null;
}

/** What a line kept to hand is known by, in what is found. */
export const ofKept = {
  pinned: (key: string) => `pinned:${key}`,
  recent: (key: string) => `recent:${key}`,
};

/** The lines kept to hand, found by what they are called and whose they are. */
export function keptItems(pinned: KeptLine[], recent: KeptLine[]): Item[] {
  return [
    ...pinned.map((line) => ({ id: ofKept.pinned(line.key), says: [line.title, line.whose] })),
    ...recent.map((line) => ({ id: ofKept.recent(line.key), says: [line.title, line.whose] })),
  ];
}

// ---- Channels -------------------------------------------------------------------

/** A channel as the panel lists it. */
export interface Listed {
  id: string;
  name: string;
  kept?: boolean;
  title?: string;
}

/** What the Channels panel lists: what is pinned, what is in no group (its workspace's own, then each project's), and each group. */
export interface ChannelsListed<C extends Listed> {
  pinned: C[];
  loose: C[];
  projects: { project: { id: string; name: string }; channels: C[] }[];
  groups: { group: { id: string; name: string }; channels: C[] }[];
  /** Those that are archived, under the Archived part at the foot. */
  archived?: C[];
}

/** What a line of the Channels panel is known by, in what is found: a channel by where it is, since a pinned one is listed twice. */
export const ofChannels = {
  pinned: 'group:pinned',
  archived: 'group:archived',
  group: (id: string) => `group:${id}`,
  project: (id: string) => `project-channels:${id}`,
  channel: (under: string, id: string) => `${under}/channel:${id}`,
  loose: '',
};

/** The Channels panel as things: each channel by what it is called, a private one by its title; each group, each project's branch and the Archived part carrying its name. */
export function channelItems<C extends Listed>(listed: ChannelsListed<C>): Item[] {
  const channel = (under: string) => (one: C): Item => ({ id: ofChannels.channel(under, one.id), says: [calledOf(one), hashed(one)] });
  const branch = (id: string, name: string, inside: C[]): Item => ({ id, says: [name], carries: name, under: inside.map(channel(id)) });
  return [
    ...(listed.pinned.length > 0 ? [branch(ofChannels.pinned, 'Pinned', listed.pinned)] : []),
    ...listed.loose.map(channel(ofChannels.loose)),
    ...listed.projects.map(({ project, channels }) => branch(ofChannels.project(project.id), project.name, channels)),
    ...listed.groups.map(({ group, channels }) => branch(ofChannels.group(group.id), group.name, channels)),
    ...(listed.archived?.length ? [branch(ofChannels.archived, 'Archived', listed.archived)] : []),
  ];
}

// ---- Topics ------------------------------------------------------------

/** What each line of the Topics panel is known by in what is found. */
export const ofTopics = {
  closed: 'topics:closed',
  topic: (id: string) => `topic:${id}`,
};

/** A topic as the panel lists it, as much as it is found by, with the pages under it. */
export interface TopicListed {
  id: string;
  title: string;
  status: string | null;
  summary: string | null;
  /** Where it is, said beside it: its project's name, for a project's. */
  where?: string | null;
  pages?: TopicListed[];
}

/**
 * The Topics panel as things: each page by its title, the first line of its
 * summary, its status and the project it is of, with the pages under it,
 * to which it carries its title, so that `deploying rolling` finds *Rolling
 * back* under *Deploying* and holds *Deploying* and its wiki open; the
 * wikis that are closed under a branch of their own, which carries nothing
 * (`closed` would find every one of them anyway).
 */
export function topicItems(listed: { listed: TopicListed[]; closed: TopicListed[] }): Item[] {
  const topic = (one: TopicListed): Item => ({
    id: ofTopics.topic(one.id),
    says: [one.title, one.summary, one.status, one.where],
    ...(one.pages?.length ? { carries: one.title, under: one.pages.map(topic) } : {}),
  });
  return [...listed.listed.map(topic), ...(listed.closed.length > 0 ? [{ id: ofTopics.closed, says: ['Closed'], under: listed.closed.map(topic) }] : [])];
}

// ---- Members ----------------------------------------------------------------------

/** What the Members line under Factory in the Resources panel, and each line under it, is known by in what is found. */
export const ofMembers = {
  part: 'members',
  member: (actor: string) => `members/${actor}`,
};

/**
 * The Members line as a thing of the Resources panel: everybody under it,
 * each found by what they are called and by the faint words beside them
 * (`you`, `agent`, `agent (external)`, `stopped`). The line carries its
 * name, as a folder does, so that `members` finds everybody.
 */
export function memberItems(lines: { actor: string; name: string; said: string | null }[]): Item[] {
  if (lines.length === 0) return [];
  return [{ id: ofMembers.part, says: ['Members'], carries: 'Members', under: lines.map((line) => ({ id: ofMembers.member(line.actor), says: [line.name, line.said] })) }];
}

// ---- Inbox ----------------------------------------------------------------------

/** The inbox's lines held to what is typed: each found by who said it, where, and its words. */
export function linesFound(lines: Line[], typed: string): Line[] {
  const words = wordsOf(typed);
  if (words.length === 0) return lines;
  return lines.filter((line) => says(words, [fromOf(line), whereOf(line), wordsSaid(line)]));
}

/** What a panel says where nothing it lists says what was typed. */
export function nothingFound(typed: string): string {
  return `Nothing listed here says “${typed.trim()}”.`;
}

/**
 * What a panel's box says at its right edge while nothing is typed in it:
 * the keys that search everything, pressed to do so. Nothing under a
 * finger, which holds no keys; nothing once something is typed, which is
 * then offered to be searched for everywhere instead (`everywhere`).
 */
export function seekHint(typed: string, touched: boolean, mac: boolean): string | null {
  if (touched || typed !== '') return null;
  return `${shown('Mod', mac)} K`;
}

/** How many letters of what is typed the line that searches everything says whole: past that it is cut, and the search is still handed all of it. */
export const MOST_SAID = 60;

/**
 * The line under a panel's box while something is typed in it, which
 * searches everything for that: what it says whole, and the words typed as
 * they are shown in it, which wrap onto a line of their own rather than
 * being cut. Nothing while nothing is typed.
 */
export function everywhere(typed: string): { says: string; words: string } | null {
  const said = typed.trim();
  if (said === '') return null;
  const letters = [...said];
  const words = `“${letters.length > MOST_SAID ? `${letters.slice(0, MOST_SAID).join('').trimEnd()}…` : said}”`;
  return { says: `Search everything for ${words}`, words };
}

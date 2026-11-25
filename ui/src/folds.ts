// What a browser kept of the side panel's folds, read as the panel is now.
//
// Holds nothing of the page, so that its test can read it.

/** The fold a project's board, tables, libraries, mockups and files were gathered under, by what it was kept as. */
const GATHERED = /^project:[^:]+:app$/;
/**
 * A project folded, when a project started unfolded: everything of the
 * Resources panel starts folded now, so that is how it starts, and the
 * project's own fold is kept under another name (`projectFold`), whose
 * being kept says it was unfolded.
 */
const FOLDED_BEFORE = /^project:[^:]+$/;
/**
 * A branch of the Channels panel folded, when its branches started
 * unfolded: a group of somebody's (`group:<id>`, Pinned as `group:pinned`)
 * and a project's channels (`project-channels:<id>`). Everything of that
 * panel starts folded now too, so that is how they start, and what is
 * kept of one is that it was unfolded, under another name (`CHANNELS`).
 */
const BRANCH_FOLDED_BEFORE = /^(group|project-channels):/;

/**
 * What was turned from how it starts, for what a browser kept from
 * before. A project's things were under one fold and are each directly
 * under the project, so that fold is nothing now and whether it was open
 * is not kept: a name nothing folds by is dropped, not left for something
 * later to be called the same and find itself turned. Its files' fold is
 * kept under the name it always had, and is open where it was left open.
 * Anything else is what it was, and what is no list at all is nothing.
 */
export function turnedNow(kept: unknown): string[] {
  if (!Array.isArray(kept)) return [];
  return kept.filter((one): one is string => typeof one === 'string' && !GATHERED.test(one) && !FOLDED_BEFORE.test(one) && !BRANCH_FOLDED_BEFORE.test(one) && !OPENED_BEFORE.has(one) && !NODES_BEFORE.has(one));
}

// The Resources panel folds everything it lists. Three of its parts start
// unfolded (Projects, Factory, Blueprints, so that a browser that kept nothing shows what there is,
// one line each), and everything under them starts folded: each project,
// what a project has that folds (its blueprints, its files and their
// folders) and the blueprints of each family; and so do the two parts
// over them (what was pinned, what was lately opened), each drawn only
// while it has anything. What is kept of a fold is that it was turned
// from how it starts: of those three parts, that it was folded, under a
// name of its own (`…:folded`, `STARTS_UNFOLDED`); of everything else,
// that it was unfolded. So Collapse all is everything else forgotten and
// those three kept as folded, and Expand all the other way round.

/** The parts of the panel, each a heading that folds what is under it, in the order the panel draws them. */
export const SECTIONS = {
  pinned: 'section:pinned',
  recents: 'section:recents',
  projects: 'section:projects:folded',
  factory: 'section:factory:folded',
  blueprints: 'section:blueprints:folded',
} as const;

/** The parts of the Resources panel that start unfolded, whose being kept says they were folded. */
export const STARTS_UNFOLDED: readonly string[] = [SECTIONS.projects, SECTIONS.factory, SECTIONS.blueprints];

/** Whether a fold starts unfolded, so that its being kept says it was folded. */
export const startsUnfolded = (fold: string): boolean => STARTS_UNFOLDED.includes(fold);

/**
 * What a browser kept of those three parts when every part started
 * folded: their names, kept where they were unfolded. Unfolded is how
 * they start now, so each still says what it said, and is dropped as
 * it is read, not left to be taken for folded.
 */
const OPENED_BEFORE = new Set(['section:projects', 'section:factory', 'section:blueprints', 'channels:members']);

/**
 * The two nodes of the channels' panel while it was called Messaging and
 * held the topics beside the channels: Channels, kept where it
 * was folded, and Topics, kept where it was unfolded. The topics have a
 * place of their own on the rail now and the channels' panel
 * holds the channels alone again, so neither folds anything and each is
 * dropped as it is read. What was kept of the closed topics' fold is the
 * Topics panel's, under the name it had (`TOPICS.closed`).
 */
const NODES_BEFORE = new Set(['channels:node:folded', 'channels:topics']);

/**
 * The folds of the Factory part, each a line that unfolds what is under
 * it: the Members line, everybody here and the agents of the
 * workspace (`direct.ts`), which starts folded as the panel's folds do.
 * It was the Members part of the Channels panel, kept as
 * `channels:members` where it was unfolded; that is dropped as it is read
 * (`OPENED_BEFORE`), and it starts folded where it is now.
 */
export const FACTORY_FOLDS = {
  members: 'factory:members',
} as const;

/** What a project's own fold is kept as: its being kept says the project is unfolded. */
export function projectFold(project: string): string {
  return `project:${project}:tree`;
}

/**
 * What is kept of the folds once a project's name was pressed, which
 * opens its page: the project unfolded too, so that what it has is in
 * view under it. One that is unfolded already stays so, since its arrow
 * is the way to fold it; and one held open by what was typed in the
 * panel's box changes nothing that is kept, as its arrow does not.
 * Nothing (`null`) where nothing is to be kept anew.
 */
export function openedByName(kept: ReadonlySet<string>, fold: string, held: boolean): Set<string> | null {
  if (held || kept.has(fold)) return null;
  return new Set([...kept, fold]);
}

/** Whether a fold that is kept is one of the Resources panel's: everything else (a channel's, a group's) is another panel's. */
export function ofResources(fold: string): boolean {
  return /^(section|project|blueprints|parts|factory):/.test(fold);
}

/** Collapse all: what is kept without any fold of a panel (the Resources panel's unless said), the folds of the other panels as they were, and of that panel's those that start unfolded kept as folded. */
export function foldedAll(kept: Iterable<string>, of: (fold: string) => boolean = ofResources): string[] {
  return [...[...kept].filter((one) => !of(one)), ...STARTS_UNFOLDED.filter(of)];
}

/** Expand all: what is kept with every one of `folds` unfolded, each once: kept, or of one that starts unfolded, not kept. */
export function unfoldedAll(kept: Iterable<string>, folds: Iterable<string>): string[] {
  const opening = [...folds];
  return [...new Set([...kept, ...opening])].filter((one) => !(startsUnfolded(one) && opening.includes(one)));
}

/** What a folder of a project's outline is kept as (`outline.ts`): it starts folded, as everything of the panel does. */
export function outlineFold(project: string, folder: string): string {
  return `project:${project}:outline:${folder}`;
}

/**
 * The folds kept of a project's outline whose folders are not there, once
 * its outline was read: a folder removed, or the outline forgotten. Its
 * id is not given again, but the outline forgotten begins again at `f1`,
 * and that one would find itself unfolded.
 */
export function outlineFoldsGone(kept: Iterable<string>, project: string, folders: Iterable<string>): string[] {
  const there = new Set([...folders].map((folder) => outlineFold(project, folder)));
  return [...kept].filter((one) => one.startsWith(`project:${project}:outline:`) && !there.has(one));
}

/** What is read of a project listed in the panel, as much as says what of it folds. */
export interface Read {
  id: string;
  /** Whether it uses any blueprints, which are under a fold of their own. */
  blueprints: boolean;
  /** Its files where its folder was given and was looked at on this page; nothing otherwise, since unfolded they would have it looked at, which Expand all does not ask. */
  files: { path: string; dir: boolean }[] | null;
  /** The folders of its outline, by id, where somebody arranged it. */
  folders?: string[];
}

/**
 * Every fold the panel has, of what was read: its parts, each project
 * listed, the folders it was arranged in, the blueprints each uses, its files and their folders where
 * its folder was looked at, and every fold of the factory's blueprints
 * (`blueprintFolds`). What was not is not unfolded, so that Expand all
 * asks the daemon for nothing.
 */
export function resourcesFolds(projects: Read[], blueprints: string[]): string[] {
  return [
    ...Object.values(SECTIONS),
    ...Object.values(FACTORY_FOLDS),
    ...projects.flatMap((project) => [
      projectFold(project.id),
      ...(project.blueprints ? [`project:${project.id}:blueprints`] : []),
      ...(project.folders ?? []).map((folder) => outlineFold(project.id, folder)),
      ...(project.files ? [`project:${project.id}:repo`, ...project.files.filter((one) => one.dir).map((one) => `project:${project.id}:folder:${one.path}`)] : []),
    ]),
    ...blueprints,
  ];
}

// The Channels panel folds the same way. Its branches (what is pinned to
// the top, each group somebody made, each project's channels, and what is
// archived) start folded, as each channel's own fold of who is in it
// always did; so what is kept of one is that it was unfolded. Under a
// channel, its people, agents and bots start unfolded, and what is kept
// of those is that they were folded, as it always was.

/** What each fold of the Channels panel is kept as. */
export const CHANNELS = {
  pinned: 'channels:pinned',
  archived: 'channels:archived',
  group: (group: string) => `channels:group:${group}`,
  project: (project: string) => `channels:project:${project}`,
  channel: (channel: string) => `channel:${channel}`,
  members: (channel: string, kind: string) => `channel:${channel}:${kind}`,
} as const;

/** The kinds of who is in a channel, each folded under the channel and starting unfolded there. */
export const MEMBERS = ['user', 'agent', 'bot'] as const;

/** Whether a fold that is kept is one of the Channels panel's: not the Topics panel's, whose names began as that panel's (`TOPICS`). */
export function ofChannels(fold: string): boolean {
  return /^channels?:/.test(fold) && !ofTopicsPanel(fold);
}

// The Topics panel folds what its list has under it: each page
// of a wiki that holds pages, and the wikis of the workspace
// showing that are closed, a branch of their own. Each starts folded, as
// everything of the panels does, so what is kept is that it was unfolded.
// Closed is kept under the name it had while it was under the Messaging
// panel's Topics node, so what a browser kept of it then is the Topics
// panel's now; a page's fold is the panel's by the same beginning.

/** What each fold of the Topics panel is kept as. */
export const TOPICS = {
  closed: 'channels:topics:closed',
  page: (id: string) => `channels:topics:page:${id}`,
} as const;

/** Whether a fold that is kept is one of the Topics panel's. */
export function ofTopicsPanel(fold: string): boolean {
  return fold.startsWith('channels:topics:');
}

/** Expand all in the Topics panel: what is kept with every page that holds pages unfolded (`pages`), and its Closed branch, where any wiki showing is closed. */
export function topicsUnfolded(kept: Iterable<string>, closed: boolean, pages: Iterable<string> = []): string[] {
  return unfoldedAll(kept, [...[...pages].map(TOPICS.page), ...(closed ? [TOPICS.closed] : [])]);
}

/**
 * The folds of pages that are not there, among what is kept, once the
 * topics were read: a page deleted, or one that holds none. A page's id is
 * never given again, but nothing would ever take the fold away.
 */
export function topicFoldsGone(kept: Iterable<string>, pages: Iterable<string>): string[] {
  const there = new Set([...pages].map(TOPICS.page));
  return [...kept].filter((one) => one.startsWith('channels:topics:page:') && !there.has(one));
}

/** What the Channels panel draws, as much as says what of it folds. */
export interface Drawn {
  /** Whether anything is pinned, which is a branch then. */
  pinned: boolean;
  /** The groups drawn, by id. */
  groups: string[];
  /** The projects whose channels are a branch, by id. */
  projects: string[];
  /** Whether anything is archived, which is a branch then. */
  archived: boolean;
  /** Every channel listed, under whichever branch, by id: each folds who is in it. */
  channels: string[];
}

/**
 * Expand all in the Channels panel: what is kept with every branch it
 * draws unfolded and every channel it lists unfolded, each once, so that
 * every channel and who is in it is in view: under each of those
 * channels its people, agents and bots are unfolded too, whatever was
 * kept of them. It used to leave who is in each channel as it was; now
 * it applies to the channels too. Collapse
 * all is `foldedAll` with `ofChannels`: everything of the panel starts
 * folded but who is in a channel, which is then under a folded channel,
 * so forgetting it all folds every branch and every channel, and a
 * channel unfolded by hand afterwards is as one nobody touched.
 */
export function channelsUnfolded(kept: Iterable<string>, drawn: Drawn): string[] {
  const under = new Set(drawn.channels.flatMap((channel) => MEMBERS.map((kind) => CHANNELS.members(channel, kind))));
  return unfoldedAll(
    [...kept].filter((one) => !under.has(one)),
    [
      ...(drawn.pinned ? [CHANNELS.pinned] : []),
      ...drawn.groups.map(CHANNELS.group),
      ...drawn.projects.map(CHANNELS.project),
      ...(drawn.archived ? [CHANNELS.archived] : []),
      ...drawn.channels.map(CHANNELS.channel),
    ],
  );
}

/**
 * The folds of groups that are not there, among what is kept: a group's
 * id is given again once the last is taken away (`withGroup`), and the
 * next would find itself unfolded.
 */
export function groupFoldsGone(kept: Iterable<string>, groups: Iterable<string>): string[] {
  const there = new Set([...groups].map(CHANNELS.group));
  return [...kept].filter((one) => one.startsWith('channels:group:') && !there.has(one));
}

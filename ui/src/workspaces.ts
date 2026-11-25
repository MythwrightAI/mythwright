// Workspaces: what gathers the projects, and the channels, that belong
// together.
//
// A project is one folder with its board, its fleet and its channels. A
// workspace is a name for several that are one piece of work to whoever
// works on them, and a channel may be a workspace's too, for talk that is
// of no one project of it. It is grouping and nothing else: it changes
// where a project and a channel are listed, and nothing of who may read
// either.
//
// A factory always has a workspace, every project is in exactly one,
// and every channel is a project's or a workspace's own: the daemon
// holds that (`studio::workspaces`), and nothing here offers, lists or
// says a project in none or a channel that belongs to nothing. Where the
// daemon answers one all the same (it is a program from before, or it
// has made the thing and the page has not read where it put it yet), the
// page lists it under the usual workspace, which is where the daemon
// puts it: `spaceOf` for a project, `spaceOfChannel` for a channel.
//
// Unlike a person's own groups of channels (`channelgroups.ts`), which
// are kept in the browser and asked of nobody, this is the factory's: it
// is read once and followed (`spaces.ts`), and is the same for
// everybody. What is here is what the lists are worked out from, with
// nothing of the page in it.
//
// The side panel is of one workspace at a time: which one somebody is
// looking at is theirs again (`Held`). The lists are functions of what
// was read and of that, and what is chosen is kept in the browser
// (`chosen.ts`) and asked of nobody. Until the workspaces are read the
// lists are held to nothing, and are everything.
//
// A conversation (`conversed.ts`) is a channel that is in no list here:
// it is listed in its members' inboxes, so it is neither gathered nor
// left out, and nothing that waits in it is counted.

import type { Channel } from './api.ts';

export interface Workspace {
  /** A slug of the name it was made with, which does not change when it is called something else. */
  id: string;
  name: string;
  created: number;
  /** Whether it is the usual one, where a project goes when nobody says. A daemon from before says nothing. */
  usual?: boolean;
  /** The projects in it, by id. */
  projects: string[];
  /** The channels that are its own, by id. A channel of one of its projects is the project's. */
  channels: string[];
}

/** What a channel belongs to: a project or a workspace, and never both. Nothing is what is said before the workspaces are read, and of no channel. */
export type Belongs = { project: string } | { workspace: string } | Record<string, never>;

/**
 * The usual workspace: where a project goes when nobody says, which is
 * the one made first that is still there. The daemon says which; from one
 * that does not, the earliest made, and of two made in one moment the
 * first as they were given.
 */
export function usualOf(all: Workspace[]): Workspace | null {
  const said = all.find((one) => one.usual);
  if (said) return said;
  return all.reduce<Workspace | null>((first, one) => (first && first.created <= one.created ? first : one), null);
}

/**
 * The workspace a project is listed in: the one that names it, and where
 * none does, the usual one. A project is always in a workspace, so one
 * that no workspace names is one whose place the page has not read: from
 * a daemon of an earlier program, which leaves it in none until the
 * program of now starts and places it, or made a moment ago. Either way
 * it is where the daemon puts it. Nothing only where there is no
 * workspace at all, which is before they are read.
 */
export function spaceOf(project: string, all: Workspace[]): Workspace | null {
  return all.find((one) => one.projects.includes(project)) ?? usualOf(all);
}

/**
 * The workspace a channel is listed in: its project's, where it is a
 * project's and that project is one there is; the one it is the own of;
 * and where neither, the usual one. A channel is always a project's or a
 * workspace's, so one that is neither is one whose place the page has
 * not read: from a daemon of an earlier program, which leaves it
 * belonging to nothing (or to a project that is gone) until the program
 * of now starts and places it, or made a moment ago. Either way it is
 * where the daemon puts it. Nothing only where there is no workspace at
 * all, which is before they are read.
 */
export function spaceOfChannel<C extends { id: string; project: string | null }>(channel: C, all: Workspace[], projects: { id: string }[]): Workspace | null {
  if (channel.project && projects.some((project) => project.id === channel.project)) return spaceOf(channel.project, all);
  return all.find((one) => one.channels.includes(channel.id)) ?? usualOf(all);
}

/**
 * The workspace a topic is listed in, as a channel's is: its
 * project's, where it is a project's and that project is one there is;
 * the one it is the own of, where that is one there is; and where neither
 * (a project or a workspace that is gone, which the daemon settles, or a
 * topic read a moment before its place was), the usual one. Nothing only
 * before the workspaces are read.
 */
export function spaceOfTopic(topic: { workspace: string | null; project: string | null }, all: Workspace[], projects: { id: string }[]): Workspace | null {
  if (topic.project && projects.some((project) => project.id === topic.project)) return spaceOf(topic.project, all);
  return all.find((one) => one.id === topic.workspace) ?? usualOf(all);
}

/** Whether the panel held so lists a topic: everything until the workspaces are read, and then those of the workspace it is of (`spaceOfTopic`). */
export function holdsTopic(held: Held, topic: { workspace: string | null; project: string | null }, all: Workspace[], projects: { id: string }[]): boolean {
  if (held.kind === 'all') return true;
  return spaceOfTopic(topic, all, projects)?.id === held.id;
}

/**
 * What a channel belongs to, as it is asked of the daemon: its project,
 * where that is one there is, and otherwise the workspace it is listed
 * in (`spaceOfChannel`). Nothing only before the workspaces are read.
 */
export function belongsTo(channel: Channel, all: Workspace[], projects: { id: string }[]): Belongs {
  if (channel.project && projects.some((project) => project.id === channel.project)) return { project: channel.project };
  const space = spaceOfChannel(channel, all, projects);
  return space ? { workspace: space.id } : {};
}

/**
 * What the side panel's lists are held to: one workspace (by its id,
 * which is what it was when it is called something else). Held to
 * nothing (`all`) is before the workspaces are read, and nothing else:
 * once they are, the panel is of one.
 */
export type Held = { kind: 'all' } | { kind: 'space'; id: string };

export const ALL: Held = { kind: 'all' };

export function sameHeld(one: Held, other: Held): boolean {
  if (one.kind === 'space') return other.kind === 'space' && other.id === one.id;
  return one.kind === other.kind;
}

/** What a browser kept of it. Anything that is not a workspace is nothing chosen: so is what one kept when a project could be in none. */
export function heldOf(saved: string | null | undefined): Held {
  if (!saved) return ALL;
  let read: unknown;
  try {
    read = JSON.parse(saved);
  } catch {
    return ALL;
  }
  const { kind, id } = (typeof read === 'object' && read !== null ? read : {}) as { kind?: unknown; id?: unknown };
  return kind === 'space' && typeof id === 'string' && id ? { kind, id } : ALL;
}

/** The workspace something is of, by its id: a project's, or a channel's (`spaceOfChannel`). Nothing for what is of neither, which is the factory's own. */
export function spaceNear(of: { project?: string | null; channel?: { id: string; project: string | null } | null }, all: Workspace[], projects: { id: string }[]): string | null {
  if (of.channel) return spaceOfChannel(of.channel, all, projects)?.id ?? null;
  if (of.project) return spaceOf(of.project, all)?.id ?? null;
  return null;
}

/**
 * What the lists are held to as things stand. With no workspace, nothing.
 * Otherwise what was chosen, while that workspace is there. And where
 * nothing was chosen, or what was chosen is gone: the workspace of what
 * is showing beside the panel (`near`), where that is of one, else the
 * usual one.
 */
export function standing(held: Held, all: Workspace[], near: string | null = null): Held {
  const usual = usualOf(all);
  if (!usual) return ALL;
  if (held.kind === 'space' && all.some((one) => one.id === held.id)) return held;
  return { kind: 'space', id: all.find((one) => one.id === near)?.id ?? usual.id };
}

/** Whether a project is listed: held to a workspace, those in it. */
export function holdsProject(held: Held, project: string, all: Workspace[]): boolean {
  if (held.kind === 'all') return true;
  return spaceOf(project, all)?.id === held.id;
}

/**
 * The projects that are listed, in the order they were given: every one
 * where the lists are held to nothing, and otherwise those of the
 * workspace chosen, each as a project always was listed, under no heading.
 */
export function listed<P extends { id: string }>(projects: P[], all: Workspace[], held: Held = ALL): P[] {
  return projects.filter((project) => holdsProject(held, project.id, all));
}

/**
 * Which workspace a channel is listed under (`spaceOfChannel`), as a
 * choice of the panel. Nothing (`null`) only before the workspaces are
 * read: no channel is the factory's own, listed whatever is chosen.
 */
export function choiceOf<C extends { id: string; project: string | null }>(channel: C, all: Workspace[], projects: { id: string }[]): Held | null {
  const space = spaceOfChannel(channel, all, projects);
  return space ? { kind: 'space', id: space.id } : null;
}

/** A channel as these go by it. */
type Listed = { id: string; project: string | null; conversation?: boolean; archived?: unknown };

/** Whether a channel is listed: where the lists are held to nothing, and where the workspace it is of is the one chosen. Never a conversation. */
export function holdsChannel<C extends Listed>(held: Held, channel: C, all: Workspace[], projects: { id: string }[]): boolean {
  if (channel.conversation) return false;
  if (held.kind === 'all') return true;
  const of = choiceOf(channel, all, projects);
  return of === null || sameHeld(of, held);
}

/** The channels a choice leaves out of the list: not a conversation, which no choice lists. */
export function leftOut<C extends Listed>(channels: C[], held: Held, all: Workspace[], projects: { id: string }[]): C[] {
  return channels.filter((channel) => !channel.conversation && !holdsChannel(held, channel, all, projects));
}

// An archived channel waits for nobody: what was unread there as it was archived is counted nowhere (`archived.ts`).
const count = <C extends { id: string; archived?: unknown }>(channels: C[], unread: Record<string, number>) =>
  channels.reduce((sum, channel) => sum + (channel.archived ? 0 : (unread[channel.id] ?? 0)), 0);

/** How many wait unread in channels a choice leaves out, so that choosing never hides that somebody spoke. */
export function unseen<C extends Listed>(channels: C[], unread: Record<string, number>, held: Held, all: Workspace[], projects: { id: string }[]): number {
  return count(leftOut(channels, held, all, projects), unread);
}

/** How many wait unread in the channels of a choice, for saying so beside it where it is offered. */
export function waitingIn<C extends Listed>(channels: C[], unread: Record<string, number>, of: Held, all: Workspace[], projects: { id: string }[]): number {
  return count(
    channels.filter((channel) => {
      const its = channel.conversation ? null : choiceOf(channel, all, projects);
      return its !== null && sameHeld(its, of);
    }),
    unread,
  );
}

/**
 * The workspace where the most wait unread in channels the panel leaves
 * out, by its id: where one press at the foot takes the panel. Of two
 * with as many, the first as they were given, which is by name. Nothing
 * where nothing waits out of sight.
 */
export function mostWaiting<C extends Listed>(channels: C[], unread: Record<string, number>, held: Held, all: Workspace[], projects: { id: string }[]): string | null {
  let most: { id: string; waits: number } | null = null;
  for (const one of all) {
    const of: Held = { kind: 'space', id: one.id };
    if (sameHeld(of, held)) continue;
    const waits = waitingIn(channels, unread, of, all, projects);
    if (waits > 0 && (!most || waits > most.waits)) most = { id: one.id, waits };
  }
  return most?.id ?? null;
}

export interface Gathered<C, P> {
  /** Those under no heading, as they were given: the own channels of the workspace the panel is of. */
  loose: C[];
  /** Each project that has a channel, in the order the projects are listed. */
  projects: { project: P; channels: C[] }[];
  /** Those that are archived, of the workspace's own and its projects', the latest archived first: under the Archived part at the foot. */
  archived: C[];
}

/**
 * The channels of the workspace the panel is of, gathered by what they
 * belong to: the workspace's own first, under no heading, since the
 * panel is of that workspace and says so; then its projects', under each
 * project, as projects always were the headings. Only what has a channel
 * is there: a project with none is not a heading over nothing. Nothing
 * else is listed: no channel is the factory's own, and what is of
 * another workspace is left out and counted (`unseen`).
 *
 * One whose place was not read (`spaceOfChannel`: it belongs to nothing,
 * or to a project that is not listed) is with the usual workspace's own.
 * Held to nothing, which is before the workspaces are read, every
 * channel is gathered: those of no project under no heading. One that
 * is archived is under none of those, and is in `archived` alone.
 */
export function gathered<C extends Listed, P extends { id: string }>(
  every: C[],
  all: Workspace[],
  projects: P[],
  held: Held = ALL,
): Gathered<C, P> {
  const within = every.filter((channel) => holdsChannel(held, channel, all, projects));
  const channels = within.filter((channel) => !channel.archived);
  const known = new Set(projects.map((project) => project.id));
  const byProject = (project: P) => ({ project, channels: channels.filter((channel) => channel.project === project.id) });
  const at = (channel: C) => (channel.archived as { at?: number } | undefined)?.at ?? 0;
  return {
    loose: channels.filter((channel) => !channel.project || !known.has(channel.project)),
    projects: projects.map(byProject).filter((one) => one.channels.length > 0),
    archived: within.filter((channel) => channel.archived).sort((one, other) => at(other) - at(one)),
  };
}

/**
 * Why the button that removes a workspace is closed, or nothing where it
 * may be pressed: what is typed has to be the workspace's name exactly
 * before a workspace is deleted, so that the one taken
 * away is the one meant. It is the client's question alone: the daemon
 * and the command line take a workspace away by its id, as they did.
 */
export function removeClosed(typed: string, name: string): string | null {
  if (typed === name) return null;
  if (typed === '') return `Type ${name} to remove it.`;
  return `What is typed is not ${name}: type its name as it is written.`;
}

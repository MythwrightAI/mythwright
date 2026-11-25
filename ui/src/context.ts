// The tools panel: a second, smaller window beside a channel. It is called
// Tools where it is shown; the names here say `context`, which is what it
// was called first, and what is kept in the browser is kept under that.
//
// It holds a few tabs of things worth having to hand while talking: what
// was attached, who is in the channel and what its agents are at, the
// board, the documents the channel keeps to hand, the fleet at work. Which tabs it
// has is the reader's: they add the ones they want and take away the ones
// they do not, and it comes back as they left it. This is the list of what
// a tab can be, and the keeping of which there are.

import { holdsProject } from './workspaces.ts';
import type { Held, Workspace } from './workspaces.ts';

export interface Panel {
  id: string;
  name: string;
  says: string;
}

/**
 * The tab of the channel: everybody in it, which is also where any one of
 * them is read about, and everything that is set of it (`views/channelset.tsx`).
 * It was called Channel members, and kept as `members`, until what is set of
 * a channel came to be there too; a browser that kept it under that name has it here.
 */
export const MEMBERS = 'channel';

/** Everything a tab of the panel can be. */
export const PANELS: Panel[] = [
  { id: 'media', name: 'Media', says: 'The pictures, pages and files attached in this channel.' },
  { id: MEMBERS, name: 'Channel', says: 'Everybody in this channel, how each agent stands and what it is at, and everything that is set of it.' },
  { id: 'board', name: 'Board', says: 'The plan: the cards on the board, by where they stand.' },
  { id: 'docs', name: 'Docs', says: 'The documents this channel keeps to hand, the same for everybody in it and told to its agents.' },
];

/** The tabs it has until the reader says otherwise. */
export const DEFAULT_TABS = ['media', MEMBERS, 'board', 'docs'];

/**
 * The usual tabs that came to be usual after readers had kept tabs of their
 * own, each given once to a reader who kept a list before it was usual: the
 * Docs tab. What is kept says which of
 * these the reader was given (`offered`, `keptOf`), so that one they then
 * take away is not given again.
 */
export const GIVEN_ONCE = ['docs'];

/**
 * What a tab is called now, for one a browser kept under a name from
 * before: agents and people were two tabs, and are the one of everybody
 * in the channel, which was Channel members (`members`) and is Channel.
 * Anything else is what it was.
 */
export function tabNow(id: string): string {
  return id === 'agents' || id === 'people' || id === 'members' ? MEMBERS : id;
}

/**
 * The tabs that were kept, without any that is no longer something a tab
 * can be, or the usual ones where none were. What was kept is the tabs and
 * which of `GIVEN_ONCE` were given (`keptOf`); a list kept before that was
 * kept is the tabs alone, and is given each of them, once, at its end.
 */
export function tabsOf(saved: string | null | undefined): string[] {
  if (saved === null || saved === undefined) return [...DEFAULT_TABS];
  let kept: unknown;
  try {
    kept = JSON.parse(saved);
  } catch {
    return [...DEFAULT_TABS];
  }
  const whole = kept !== null && typeof kept === 'object' && !Array.isArray(kept) ? (kept as { tabs?: unknown; offered?: unknown }) : null;
  const list = whole ? whole.tabs : kept;
  if (!Array.isArray(list)) return [...DEFAULT_TABS];
  const given = whole && Array.isArray(whole.offered) ? whole.offered.filter((id): id is string => typeof id === 'string') : [];
  // One kept under a name from before is where it was, under the name it has now, once.
  const now = list.map((id) => (typeof id === 'string' ? tabNow(id) : id));
  const tabs = now.filter((id): id is string => PANELS.some((panel) => panel.id === id));
  return [...new Set([...tabs, ...GIVEN_ONCE.filter((id) => !given.includes(id))])];
}

/** What is kept of the tabs a reader has: the tabs, and that each of `GIVEN_ONCE` was given. */
export function keptOf(tabs: string[]): string {
  return JSON.stringify({ tabs, offered: GIVEN_ONCE });
}

/** What can still be added: every panel that is not a tab already. */
export function addable(tabs: string[]): Panel[] {
  return PANELS.filter((panel) => !tabs.includes(panel.id));
}

/** The tab to show: the one asked for if it is still there, else the first. */
export function shown(tabs: string[], asked: string | null): string | null {
  return asked && tabs.includes(asked) ? asked : (tabs[0] ?? null);
}

/**
 * Whose boards the Board tab offers, and which it begins on. Those of the
 * workspace the side panel is of, by the rule the panel lists projects by
 * (`holdsProject`), in the order they are handed; and the channel's own
 * project first among them where it has a board, whatever workspace it is
 * of, since the tab is beside that channel. It begins on the channel's own
 * where that is offered, and otherwise on the first.
 */
export function boardsShown(boards: string[], channelProject: string | null, held: Held, all: Workspace[]): { offered: string[]; begins: string | null } {
  const own = channelProject && boards.includes(channelProject) ? channelProject : null;
  const rest = boards.filter((id) => id !== own && holdsProject(held, id, all));
  const offered = own ? [own, ...rest] : rest;
  return { offered, begins: offered[0] ?? null };
}

/** What an attachment is, for where it goes among the media: a picture, a page, or a file. */
export function mediaKind(media: string): 'picture' | 'page' | 'file' {
  if (['image/png', 'image/jpeg', 'image/gif', 'image/webp', 'image/avif'].includes(media)) return 'picture';
  return media === 'text/html' ? 'page' : 'file';
}

/**
 * How wide the panel is until the reader drags its edge, and the least it
 * can be. The tabs it starts with are more than it is wide (the one of
 * everybody in the channel has a long name): their row scrolls sideways,
 * under the `+` where one was taken away and can be added again, which
 * stays whole at its end, and the tab showing is kept in sight
 * (`sideways.ts`).
 */
export const DEFAULT_WIDTH = 388;
export const LEAST_WIDTH = 240;
/** What the channel beside it is never squeezed below. */
const LEAST_BESIDE = 360;

/**
 * A width the panel can have where there is `room` across for it and what
 * is beside it: no narrower than it can be read at, and no wider than
 * leaves the channel its share.
 */
export function fitWidth(asked: number, room: number): number {
  const most = Math.max(LEAST_WIDTH, room - LEAST_BESIDE);
  return Math.round(Math.min(Math.max(asked, LEAST_WIDTH), most));
}

/** The width that was kept, or the usual one where none was or it makes no sense. */
export function widthOf(saved: string | null | undefined, usual = DEFAULT_WIDTH): number {
  const kept = Number(saved);
  return saved && Number.isFinite(kept) && kept >= LEAST_WIDTH ? Math.round(kept) : usual;
}

/** The passing tabs that are kept when the page is read again: a thread that was open, a message being written at length. */
export type Passing = 'thread' | 'write';

/** Which passing tab was in front, as it was kept; none where nothing was or it is not one of them. */
export function frontOf(saved: string | null | undefined): Passing | null {
  return saved === 'thread' || saved === 'write' ? saved : null;
}

/** The thread that was open beside each channel, as it was kept: channel to message. Anything else that was kept there is nothing. */
export function threadsOf(saved: string | null | undefined): Record<string, string> {
  try {
    const kept: unknown = JSON.parse(saved ?? '{}');
    if (!kept || typeof kept !== 'object' || Array.isArray(kept)) return {};
    return Object.fromEntries(Object.entries(kept).filter((one): one is [string, string] => typeof one[1] === 'string' && one[1] !== ''));
  } catch {
    return {};
  }
}

/** What was being written in each channel and thread, as it was kept, with what was attached to go with it. */
export function writtenOf<Attached>(saved: string | null | undefined): [string, { text: string; attached: Attached[] }][] {
  try {
    const kept: unknown = JSON.parse(saved ?? '[]');
    if (!Array.isArray(kept)) return [];
    return kept.filter(
      (one): one is [string, { text: string; attached: Attached[] }] =>
        Array.isArray(one) && typeof one[0] === 'string' && typeof one[1]?.text === 'string' && Array.isArray(one[1]?.attached),
    );
  } catch {
    return [];
  }
}

/**
 * How many of a column's cards the Board tab lists, of `count`, where the
 * panel is `room` high and a line `line`: as many as the panel
 * holds, at least a few, and all of them where only a few more would be
 * left; the rest are said as "and N more", which opens the board.
 */
export function firstOf(count: number, room: number, line: number): { shown: number; more: number } {
  const holds = Math.max(5, Math.ceil(room / Math.max(1, line)));
  const shown = count <= holds + 2 ? count : holds;
  return { shown, more: count - shown };
}

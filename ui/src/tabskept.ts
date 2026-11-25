// The open tabs of each workspace.
//
// Which tabs are open, how they are split into groups and which is
// showing are whoever is looking's, as the workspace the side panel is of
// is: kept in this browser under who is signed in (`tabsof.ts`), and the
// factory knows nothing of them. Each workspace has its own set: the
// panel coming to be of another keeps the set that was showing under the
// one it was of and shows the set kept for the other, so that going back
// is going back to what was open. That is not closing anything: what a
// tab kept while it was away (something half written, what was searched
// for) is kept by the tab's key and is there when its set is shown
// again.
//
// A set is of the workspace it was opened in and not of what its tabs
// are of: something of another workspace opened where the reader is
// (from a link, a mention, a search) is a tab there.
//
// Here is what is worked out, with nothing of the page in it: what a
// browser kept, read; what a set comes to as things stand; and which set
// is shown as the panel comes to be of a workspace.

import { close, group, groups, mend, show } from './panes.ts';
import type { Layout } from './panes.ts';
import { tabKey } from './tab.ts';
import type { Tab } from './tab.ts';

/** One set of open tabs: which there are, how they are arranged, and the group last used. */
export interface TabSet {
  tabs: Tab[];
  layout: Layout;
  focused: string;
}

/** The set kept for each workspace, by the workspace's id, which is what it was when it is called something else. */
export type Sets = Record<string, TabSet>;

/** What a workspace with nothing kept shows, and a new browser: the start page alone. */
export function start(): TabSet {
  return { tabs: [{ kind: 'home' }], layout: group('g1', ['home']), focused: 'g1' };
}

/**
 * The kinds of tab there were and are no longer, which a browser may
 * still keep: the fleet's page, taken out of the application
 * to come back as a package. Such a tab is left out as a set is read,
 * and is never shown as a second start page.
 */
export const TAKEN_OUT = new Set(['fleet']);

/**
 * The kinds of tab there were whose page is now a part of another's,
 * which a browser may still keep, each as the tab that took its place and
 * the key it is known by: the Agents page, now the Members page at its
 * Agents tab.
 */
const BECAME: Record<string, { tab: Tab; key: string }> = {
  agents: { tab: { kind: 'members', at: 'agents' }, key: 'members' },
};

/** What was kept of an arrangement, with each key of a tab that became another's said as that one's. */
function renamed(layout: unknown): unknown {
  if (!layout || typeof layout !== 'object') return layout;
  const node = layout as Record<string, unknown>;
  const as = (key: unknown) => (typeof key === 'string' ? (BECAME[key]?.key ?? key) : key);
  if (Array.isArray(node.tabs)) return { ...node, tabs: [...new Set(node.tabs.map(as))], active: as(node.active) };
  return { ...node, first: renamed(node.first), second: renamed(node.second) };
}

/**
 * One set as a browser kept it, read: each tab once (a kind of tab that
 * used to be several can be kept twice), a kind there is no longer left
 * out (`TAKEN_OUT`), one that became a tab of another page that tab
 * where it is (`BECAME`), the arrangement made to fit the tabs there are, and
 * the group last used one of its groups. Nothing where nothing was kept
 * or what was kept is no set.
 */
export function setOf(saved: unknown): TabSet | null {
  if (!saved || typeof saved !== 'object') return null;
  const read = saved as { tabs?: unknown; layout?: unknown; focused?: unknown; active?: unknown };
  if (!Array.isArray(read.tabs)) return null;
  const once = new Map<string, Tab>();
  for (const tab of read.tabs) {
    const kind = tab && typeof tab === 'object' ? (tab as { kind?: unknown }).kind : undefined;
    if (typeof kind !== 'string' || TAKEN_OUT.has(kind)) continue;
    // One that became a tab of another page is that tab, where the page was not kept open already as it is now.
    const became = BECAME[kind];
    if (became) {
      if (!once.has(became.key)) once.set(became.key, became.tab);
    } else once.set(tabKey(tab as Tab), tab as Tab);
  }
  if (once.size === 0) return null;
  const keys = [...once.keys()];
  let layout = mend(renamed(read.layout), keys);
  // Before there were groups, one row of tabs and the one showing were kept.
  const active = typeof read.active === 'string' ? (BECAME[read.active]?.key ?? read.active) : null;
  if (!read.layout && active !== null && keys.includes(active)) layout = show(layout, active, 'g1');
  const all = groups(layout);
  return { tabs: [...once.values()], layout, focused: all.some((entry) => entry.id === read.focused) ? (read.focused as string) : all[0]!.id };
}

function parsed(saved: string | null | undefined): unknown {
  if (!saved) return null;
  try {
    return JSON.parse(saved);
  } catch {
    return null;
  }
}

/** The one set a browser kept before each workspace had its own, and keeps still while the panel is of none. */
export function before(saved: string | null | undefined): TabSet | null {
  return setOf(parsed(saved));
}

/** What a browser kept for each workspace, read: what of it is no set is left out. */
export function keptOf(saved: string | null | undefined): Sets {
  const read = parsed(saved);
  if (!read || typeof read !== 'object' || Array.isArray(read)) return {};
  const sets: Sets = {};
  for (const [id, one] of Object.entries(read)) {
    const set = setOf(one);
    if (set) sets[id] = set;
  }
  return sets;
}

/** The tab showing in the group last used. */
export function showingIn(set: TabSet): Tab | undefined {
  const all = groups(set.layout);
  const key = (all.find((entry) => entry.id === set.focused) ?? all[0])?.active;
  return set.tabs.find((tab) => tabKey(tab) === key);
}

/** What was read of what there is, by id. What was not read (`null`) says nothing of what is gone. */
export interface Known {
  projects: string[] | null;
  channels: string[] | null;
}

/**
 * Whether what a tab is of is gone, as far as was read: a channel that
 * is listed no longer, and anything of a project that was forgotten.
 * Nothing else: a table, a file or something running that is not there
 * has a page that says so, as it always had.
 */
export function isGone(tab: Tab, known: Known): boolean {
  if (tab.kind === 'channel') return known.channels !== null && !known.channels.includes(tab.id);
  const project = tab.kind === 'project' ? tab.id : 'project' in tab ? tab.project : undefined;
  return project !== undefined && known.projects !== null && !known.projects.includes(project);
}

/** A set without some of its tabs, as closing each would leave it, and which they were, by key. With none left it is the start page alone. */
function less(set: TabSet, goes: (tab: Tab) => boolean): { set: TabSet; lost: string[] } {
  const lost = set.tabs.filter(goes).map(tabKey);
  if (lost.length === 0) return { set, lost };
  const tabs = set.tabs.filter((tab) => !goes(tab));
  if (tabs.length === 0) return { set: start(), lost };
  const layout = lost.reduce(close, set.layout);
  const all = groups(layout);
  return { set: { tabs, layout, focused: all.some((entry) => entry.id === set.focused) ? set.focused : all[0]!.id }, lost };
}

/**
 * What a kept set comes to as things stand: the tabs of what is gone
 * taken out, as closing each would have. A set is kept while another is
 * showing, and what happens meanwhile closes nothing in it: a project
 * forgotten then is found gone when its set is shown. `lost` is those
 * tabs by key, for whatever was kept for them to go with them.
 */
export function standingSet(set: TabSet, known: Known): { set: TabSet; lost: string[] } {
  return less(set, (tab) => isGone(tab, known));
}

/** Every kept set without one tab, whose thing whoever asks knows to be gone: the same sets where none had it. */
export function forgot(sets: Sets, key: string): Sets {
  let changed = false;
  const next: Sets = {};
  for (const [id, set] of Object.entries(sets)) {
    const without = less(set, (tab) => tabKey(tab) === key);
    next[id] = without.set;
    changed ||= without.lost.length > 0;
  }
  return changed ? next : sets;
}

const those = (sets: Sets, there: string[]): Sets => Object.fromEntries(Object.entries(sets).filter(([id]) => there.includes(id)));

/**
 * What is first shown as the page is read, once it is known which
 * workspace the panel is of (`to`; none where the workspaces could not
 * be read, or nobody owns the factory yet), and what is kept from then.
 *
 * Of no workspace there is one set, as there always was: what a browser
 * kept as that (`was`), else the start page. Of a workspace it is the
 * set kept for it; and where none is, what a browser kept before each
 * workspace had its own, which is that workspace's from then on and is
 * never shown under another (whoever asks takes it out of where it was
 * kept). Either as things stand, and with the sets of workspaces that
 * are gone forgotten.
 */
export function first(sets: Sets, was: TabSet | null, to: string | null, there: string[], known: Known): { sets: Sets; show: TabSet } {
  if (to === null) return { sets, show: standingSet(was ?? start(), known).set };
  const kept = those(sets, there);
  const show = standingSet(kept[to] ?? was ?? start(), known).set;
  return { sets: { ...kept, [to]: show }, show };
}

/**
 * The set put back as the page is read, with what somebody opened on the
 * page before it could be put back (`meanwhile`: the set the page had,
 * which began as the start page alone) opened among it, as opening each
 * there would have: in the group last used where it is not open already,
 * and the one that was showing in `meanwhile` showing there. Somebody who
 * signs in is shown the frame while what was kept for them waits for the
 * workspaces to be read, and a press in that moment (Settings, from the
 * rail) was put away with the start page. Where something opened then
 * was not what was showing, the group last used shows what it was put
 * back showing. With nothing opened it is the set put back, as it was.
 */
export function among(put: TabSet, meanwhile: TabSet): TabSet {
  const begun = new Set(start().tabs.map(tabKey));
  const opened = meanwhile.tabs.filter((tab) => !begun.has(tabKey(tab)));
  if (opened.length === 0) return put;
  const keys = new Set(put.tabs.map(tabKey));
  const was = showingIn(meanwhile);
  const front = was && opened.some((tab) => tabKey(tab) === tabKey(was)) ? tabKey(was) : null;
  const into = groups(put.layout).find((entry) => entry.id === put.focused) ?? groups(put.layout)[0]!;
  let layout = put.layout;
  for (const tab of opened) if (tabKey(tab) !== front) layout = show(layout, tabKey(tab), into.id);
  layout = show(layout, front ?? into.active ?? tabKey(opened.at(-1)!), into.id);
  const all = groups(layout);
  const focused = (front && all.find((entry) => entry.tabs.includes(front))?.id) || (all.some((entry) => entry.id === into.id) ? into.id : all[0]!.id);
  return { tabs: [...put.tabs, ...opened.filter((tab) => !keys.has(tabKey(tab)))], layout, focused };
}

/**
 * The panel has come to be of workspace `to`, with `showing` on the page
 * as the set of `from` (none where the panel was of no workspace so
 * far): what is kept from then, and the set to put on the page, or
 * nothing where what is there stays.
 *
 * The set that was showing is kept under the workspace it was of, while
 * that is still there, and the other's is shown: the start page alone
 * where nothing was kept for it. Where the panel was of none so far,
 * what is showing is that workspace's from now. The set of a workspace
 * that was taken away is forgotten, the one showing too.
 */
export function switched(sets: Sets, showing: TabSet, from: string | null, to: string, there: string[]): { sets: Sets; show: TabSet | null } {
  const kept = those(sets, there);
  if (from === null || from === to) return { sets: { ...kept, [to]: showing }, show: null };
  if (there.includes(from)) kept[from] = showing;
  const show = kept[to] ?? start();
  return { sets: { ...kept, [to]: show }, show };
}

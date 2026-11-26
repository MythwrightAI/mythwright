// What the client knows right now, and how it stays current.
//
// Each list is read once when the user signs in. After that the event stream
// says when something changed, and only that thing is read again.

import { batch, computed, effect, signal } from '@preact/signals';
import type { ComponentChildren } from 'preact';
import { api, ApiError, session } from './api.ts';
import type { BlueprintLanguage, Channel, Collection, Described, Entry, Me, Meta, PermissionMode, Progress, Project, Provider, Slot, Speaker } from './api.ts';
import { addMessage, loadMessages, loadThread, messages, placedFile, threads } from './said.ts';
import { NOTHING } from './compare.ts';
import { forgetApplying } from './applyasked.ts';
import { forgetSought, keepSought } from './editor/sought.ts';
import { forgetAllOpened, forgetOpened } from './summaryfolds.ts';
import { forgetSandbox, forgetSandboxes } from './sandboxkept.ts';
import { forgetOpen } from './handedopen.ts';
import { forgetRunning } from './running.ts';
import { drawerKeeper, forgetAllReturned, forgetReturned } from './returned.ts';
import { forgetAsking } from './waysasked.ts';
import type { Choice } from './compare.ts';
import { titleOf } from './profiles.ts';
import { coalesce, stream } from './events.ts';
import { close, dock, fresh, group, groups, holding, move, resize, show } from './panes.ts';
import type { Layout, Zone } from './panes.ts';
import type { Standing } from './standing.ts';
import { frontOf, keptOf, MEMBERS, tabNow, tabsOf, widthOf } from './context.ts';
import { tabKey } from './tab.ts';
import type { Tab } from './tab.ts';
import { loadedFrom, verdict } from './update.ts';
import { gone } from './kept.ts';
import { spendOnce } from './opened.ts';
import { sendsOn } from './senton.ts';
import { isLibrary } from './library.ts';
import { isMockups } from './mockups.ts';
import { sitsBeside } from './sits.ts';
import { reportedNow } from './modelsaid.ts';
import { sayToast, type Toast } from './toasted.ts';
import { changesUse } from './uses.ts';
import { isNamedBlueprint } from './util.ts';
import { inPlace, parked } from './inplace.ts';
import { askMembers } from './membersshown.ts';
import type { Uses } from './uses.ts';

// What a tab is, and the key it is known by, are in a file with nothing of the page in it (`tab.ts`).
export { tabKey };
export type { Tab };

export const meta = signal<Meta | null>(null);
export const me = signal<Me | null>(null);
/** False until the first answers are in; nothing is drawn before then. */
export const ready = signal(false);
export const connected = signal(false);
/** How the stream of events stands: only its being lost is ever said (`standing.ts`). */
export const standing = signal<Standing>('waiting');

export const projects = signal<Project[]>([]);
/**
 * Whether whoever is signed in has everything the frame shows: the
 * projects and the channels read, and the tabs that were open back,
 * which are those of the workspace the panel is of (`tabsof.ts`).
 * Until then an empty list says nothing of what there is, and no tab is
 * yet the one that is showing.
 */
export const entered = signal(false);
export const channels = signal<Channel[]>([]);
export const agents = signal<Slot[]>([]);
export const adapters = signal<{ names: string[]; fallback: string }>({ names: [], fallback: '' });
/** What an agent can run on here, with what a person choosing needs to know. */
export const providers = signal<Provider[]>([]);
/** How much an agent may be let do unasked, least first. */
export const permissionModes = signal<PermissionMode[]>([]);
export const trees = signal<Record<string, Entry[]>>({});
// What is said in channels is kept by `said.ts`, and read from here as it always was.
export { loadMessages, loadThread, messages, threads };
export const unread = signal<Record<string, number>>({});
/** Agents in the middle of a turn: slot name to what it has said so far. */
export const working = signal<Record<string, { channel: string | null; text: string; thread?: string | null; kind?: string | null }>>({});
export const indexing = signal<Progress | null>(null);
/** Bumped when a file changes on disk, keyed `project:path`; open views reload. */
export const fileVersions = signal<Record<string, number>>({});
/** What blueprints have declared for each project: its board, once one has made it. */
export const collections = signal<Record<string, Collection[]>>({});
/** Whether a project has a board. None does until a blueprint makes it one. */
export function hasBoard(project: string): boolean {
  return (collections.value[project] ?? []).some((one) => one.name === 'board');
}
/** A project's libraries, by name: the collections whose records are documents. None has one until a blueprint makes it one. */
export function librariesOf(project: string): string[] {
  return (collections.value[project] ?? []).filter(isLibrary).map((one) => one.name);
}
/** A project's other tables, by name: what a blueprint declared that is neither its board, a library nor its mockups, nor sits beside another table, whose page it is reached from (`sits.ts`). */
export function tablesOf(project: string): string[] {
  const all = collections.value[project] ?? [];
  return all.filter((one) => one.name !== 'board' && !isLibrary(one) && !isMockups(one) && !sitsBeside(one, all)).map((one) => one.name);
}
/** A project's mockups, by the collection's name: the collections whose records are folders that hold a page. */
export function mockupsOf(project: string): string[] {
  return (collections.value[project] ?? []).filter(isMockups).map((one) => one.name);
}

// What in the side panel's trees is folded, and which view it shows, are `sidepanel.ts`'s, offered here under the names they had.
import { clearSide, foldSide, showSide, side, turn, turned, turnedTo } from './sidepanel.ts';
export { foldSide, showSide, side, turn, turned, turnedTo };
export type { SideView } from './sidepanel.ts';

// The tools panel beside a channel: whether it is open, which tabs the
// reader gave it, and which of them is showing. All three are theirs, and
// are kept in this browser.
const SAVED_CONTEXT = 'client.context';
export function keptContext(key: string): string | null {
  try {
    return localStorage.getItem(`${SAVED_CONTEXT}.${key}`);
  } catch {
    return null;
  }
}
export function keepContext(key: string, value: string): void {
  try {
    localStorage.setItem(`${SAVED_CONTEXT}.${key}`, value);
  } catch {
    /* not remembered */
  }
}
const browser = typeof localStorage !== 'undefined';
export const contextOpen = signal<boolean>(browser && keptContext('open') === 'yes');
export const contextTabs = signal<string[]>(tabsOf(browser ? keptContext('tabs') : null));
// One kept under a name from before (agents and people were two tabs) is the tab that took their place.
const keptTab = browser ? keptContext('tab') : null;
export const contextTab = signal<string | null>(keptTab === null ? null : tabNow(keptTab));
/** How wide it is. It is kept when the reader lets go of its edge, not at every step of the way. */
export const contextWidth = signal<number>(widthOf(browser ? keptContext('width') : null));
/** Whether the reader has said how wide it is: until they do it is the usual width, which grows with the size of text. */
export const contextWidthSaid = signal<boolean>(browser && keptContext('width') !== null);
export function setContextWidth(width: number, done: boolean): void {
  contextWidth.value = width;
  contextWidthSaid.value = true;
  if (done) keepContext('width', String(width));
}
/**
 * Whoever is being read about in the tools panel, as they take part
 * (`agent:<slot>`, `bot:<name>`, `user:<id>`); none when none is. They are
 * read about under Channel there: the list, and then the one who was pressed.
 */
export const contextMember = signal<string | null>(null);
/**
 * Which of the panel's passing tabs is in front of the reader's own: the
 * thread that is open, the message being written at length, or neither.
 * Whatever was asked for last.
 */
export const contextFront = signal<'thread' | 'write' | null>(browser ? frontOf(keptContext('front')) : null);
// Which of them it was is kept, for the two that are still there when the page is read again.
if (browser) effect(() => keepContext('front', contextFront.value ?? ''));
/** What was being written in channels, kept while the browser's tab is: it is gone with the tab, and when whoever wrote it leaves. */
export const SAVED_WRITTEN = 'client.channel.written';
/** What of Settings somebody asked to be shown (the workspace the panel is of; who is here, and inviting them), until that page has brought it into view. */
export const settingsPart = signal<'workspace' | 'people' | null>(null);
/** Shows what there is to know about an agent: in the tools panel where the reader is in a channel, and anywhere else on the Members page's Agents tab. */
export function showAgent(slot: string): void {
  if (!active.value.startsWith('channel:')) {
    askMembers('agents');
    return openTab({ kind: 'members' });
  }
  showMember(`agent:${slot}`);
}
/**
 * Shows somebody who takes part in a channel, where the reader is in one:
 * under Channel in the tools panel, which is given that tab if the reader
 * had taken it away. Anywhere else there is no channel for them to be of.
 */
export function showMember(actor: string): void {
  if (!active.value.startsWith('channel:')) return;
  contextMember.value = actor;
  contextFront.value = null;
  if (!contextTabs.value.includes(MEMBERS)) setContextTabs([...contextTabs.value, MEMBERS]);
  showContextTab(MEMBERS);
  clearSide();
  if (!contextOpen.value) foldContext();
}
/** The factory's own bot: what it is called and what it takes, as the daemon says; nothing until that is read. */
export const speaker = signal<Speaker | null>(null);
async function refreshSpeaker(): Promise<void> {
  speaker.value = await api.speaker().catch(() => null);
}
// The bot called something else joins every channel it is in under its new
// name, which is what the page hears of it: what it did is said by that name.
stream.on(['channel.speaker_held'], () => void refreshSpeaker());
stream.on(['channel.joined'], (event) => {
  const member = event.data?.member;
  if (typeof member === 'string' && member.startsWith('bot:') && member !== `bot:${speaker.value?.name}`) void refreshSpeaker();
});
/** The card being looked at in the tools panel's board, by its id; none when the board is. */
export const contextCard = signal<string | null>(null);
/** The comment on that card it was opened at, by its place in the card's history: none for a card opened as itself. */
export const contextAt = signal<number | null>(null);
/**
 * Shows a card that was named somewhere: in the tools panel where the
 * reader is in a channel, which has one, and on its board anywhere else.
 * `at` is a comment on it to be shown at, as a line of the inbox opens one.
 */
export function showCard(card: string, project: string, at?: number): void {
  if (!active.value.startsWith('channel:')) {
    openTab({ kind: 'board', project }, at === undefined ? { item: card } : { item: card, at });
    return;
  }
  contextAt.value = at ?? null;
  contextCard.value = card;
  contextFront.value = null;
  if (!contextTabs.value.includes('board')) setContextTabs([...contextTabs.value, 'board']);
  showContextTab('board');
  if (!contextOpen.value) foldContext();
}
export function foldContext(): void {
  contextOpen.value = !contextOpen.value;
  keepContext('open', contextOpen.value ? 'yes' : 'no');
}
export function showContextTab(id: string): void {
  contextTab.value = id;
  keepContext('tab', id);
}
export function setContextTabs(tabs: string[]): void {
  contextTabs.value = tabs;
  keepContext('tabs', keptOf(tabs));
}

// The theme the client is in, and its look, are `wearing.ts`'s, offered here under the names they had.
export { madeThemes, offeredThemes, prefersLight, setTheme, theme, themesRead, wearLook, wearable } from './wearing.ts';


// The factory's views (activity, measurements, agents) are of the workspace
// the side panel is of: what they show, and where that is kept, is
// `narrowed.ts`, which stands on the workspaces, which stand on this.
let narrows: (project: string | null) => void = () => undefined;
/** Says how the factory's views are had to show one project (`narrowed.ts` does, as it is read). */
export const narrowsBy = (how: typeof narrows) => void (narrows = how);
/** Has the factory's views show one project, or with none all of the workspace the panel is of. */
export const narrowTo = (project: string | null) => narrows(project);
/** What the measurements are set side by side by, kept here so that looking at another tab does not lose it. */
export const comparing = signal<Choice>(NOTHING);

/** The folder each project's page of files is showing, by project; none is its top. */
export const folders = signal<Record<string, string>>({});
/** Opens a project's files at one of its folders. */
export function openFolder(project: string, folder: string): void {
  folders.value = { ...folders.value, [project]: folder };
  openTab({ kind: 'repo', project });
}

/**
 * The projects whose folders have been looked at since this page was
 * loaded. The daemon does not watch every project's folder: what something
 * else changed there (an editor, a terminal) is found when it looks. It
 * looks the first time a project's page or its files are opened, and when
 * asked to.
 */
const looked = new Set<string>();
/** Whether a project's folder was looked at on this page: its files unfolded ask for no look, which Expand all does not ask. */
export const hasLooked = (project: string): boolean => looked.has(project);
export function lookAt(project: string): void {
  if (looked.has(project)) return;
  looked.add(project);
  void api.scanProject(project).catch(() => {});
}

/** Every tab that is open, in no particular order: `layout` says where each is. */
export const tabs = signal<Tab[]>([{ kind: 'home' }]);
/** How the open tabs are arranged: in one group, or in several side by side. */
export const layout = signal<Layout>(group('g1', ['home']));
/** The group last used. A tab that is not open yet opens in it. */
export const focused = signal('g1');
/** The tab showing in the group last used. */
export const active = computed(() => groups(layout.value).find((entry) => entry.id === focused.value)?.active ?? '');
/** The tabs that can be seen: the one showing in each group. */
export const showing = computed(() => new Set(groups(layout.value).map((entry) => entry.active)));
/** Something a view should do when it is next shown, e.g. go to a line. */
type Hint = { line?: number; item?: string; message?: string; comments?: boolean; at?: number; column?: string };
export const hints = signal<Record<string, Hint>>({});

export interface Drawer {
  title: string;
  body: () => ComponentChildren;
  /**
   * Of something that can be taken away (a record, something running): whether it is gone, asked as the drawer is
   * given back to its tab, which then puts it away (`drawerkept.ts`).
   */
  gone?: () => Promise<boolean>;
}
export const drawer = signal<Drawer | null>(null);
export const paletteOpen = signal(false);
/** What the search of everything opens with: what was typed in a side panel's box where it was handed on from there, and nothing otherwise. */
export const paletteWith = signal('');
/** Opens the search of everything, with `text` in it already. */
export function searchEverything(text = ''): void {
  paletteWith.value = text;
  paletteOpen.value = true;
}
/** Whether the list of keyboard shortcuts is open over the page. */
export const shortcutsOpen = signal(false);
/** The "new channel" or "new project" form, when it is open. */
export const asking = signal<'channel' | 'project' | null>(null);
export const toasts = signal<Toast[]>([]);
/**
 * Why this page should be reloaded, when it should: a newer client is being
 * served (`ready`), or a part of this one could not be fetched (`broken`).
 */
export const reload = signal<'ready' | 'broken' | null>(null);
/** The daemon turned out to be older than this client. */
export const daemonBehind = signal(false);
/** The tabs holding changes that have not been saved, by key. */
export const edited = signal<ReadonlySet<string>>(new Set());
/** A tab somebody asked to close while it held unsaved changes; they are being asked what to do. */
export const closing = signal<string | null>(null);
/**
 * In a browser, where one thing shows at a time: what somebody pressed to open in place of the thing showing while
 * it held unsaved changes. They are being asked what to do, as closing a tab with them asks (`inplace.ts`).
 */
export const leaving = signal<{ tab: Tab; hint?: Hint } | null>(null);

let toastId = 0;
export function toast(text: string, bad = false, undo?: () => void): void {
  const id = ++toastId;
  toasts.value = sayToast(toasts.value, { id, text, bad, undo });
  // One that can be taken back stays long enough to be.
  setTimeout(() => (toasts.value = toasts.value.filter((entry) => entry.id !== id)), bad || undo ? 7000 : 3500);
}

/** Runs something the user asked for and says so if it could not be done. */
export async function attempt<T>(work: () => Promise<T>): Promise<T | undefined> {
  try {
    return await work();
  } catch (err) {
    toast(err instanceof Error ? err.message : String(err), true);
    return undefined;
  }
}

// ---- tabs ------------------------------------------------------------------

// Where the open tabs are kept is not here: each workspace has its own set (`tabsof.ts`, over `tabskept.ts`).
// Whoever keeps them says so here: told when they are arranged, asked to put back what it kept as somebody
// enters, and to forget it as they sign out.
export const tabsKeeper = { arranged: (): void => {}, back: async (): Promise<void> => {}, forget: (): void => {} };

/**
 * Puts a whole set of tabs on the page as one change: which there are, how they are arranged and the group last
 * used. Nobody opened any of them by it, so nobody is told that they did (`onOpened`), and nothing is closed.
 */
export function showTabs(set: { tabs: Tab[]; layout: Layout; focused: string }): void {
  drawerKeeper.away('');
  batch(() => {
    tabs.value = set.tabs;
    closing.value = null;
    arrange(set.layout, set.focused);
  });
  drawerKeeper.back();
}

/** Puts an arrangement of the tabs in place, with everything that follows from one. */
function arrange(next: Layout, focus: string = focused.value): void {
  const all = groups(next);
  layout.value = next;
  focused.value = all.some((entry) => entry.id === focus) ? focus : all[0]!.id;
  // A channel that has come into view has been read, however it came.
  const counts = { ...unread.value };
  let read = false;
  for (const entry of all) {
    const id = entry.active?.startsWith('channel:') ? entry.active.slice('channel:'.length) : null;
    if (id && counts[id]) {
      counts[id] = 0;
      read = true;
    }
  }
  if (read) unread.value = counts;
  tabsKeeper.arranged();
}

// ---- one thing at a time, in a browser -------------------------------------------

/**
 * Whoever keeps the browser's history says so here (`address.ts`): told of a thing somebody opens in place, before
 * it shows, so that the address bar is given an entry of its own for it, which Back comes back from.
 */
export const placeKeeper = { opening: (_tab: Tab): void => {} };

/** In a browser, the keys of what was shown before, the latest first: each kept as it was left, to be shown so again by Back. */
let behind: string[] = [];

/** Lets go of what was kept for a thing while it was not shown (what it read, where it was scrolled, its drawer), but what is half written in it. */
function letGoOf(key: string): void {
  keepSought(key, null);
  forgetOpened(key);
  forgetOpen(key);
  // A sandbox test run from it is ended with it, as with a tab closed.
  forgetSandbox(key);
  forgetReturned(key);
}

/** In a browser, a thing taken away by whoever knows it is gone is no longer among what was shown before. */
export function forgetBehind(key: string): void {
  behind = behind.filter((one) => one !== key);
}

/**
 * Shows a thing in place, in a browser: the one thing on the page. Pressed (`arrived` false), the browser's history is
 * given an entry for it, and where the thing showing holds unsaved changes somebody is asked first (`leaving`), unless
 * `sure`; come to by Back or Forward, or by an address, it is shown with nothing asked, what is half written being
 * kept by its key to come back to. What was left is kept as it was, to be shown so again (`returned.ts`).
 */
function showInPlace(tab: Tab, hint: Hint | undefined, arrived: boolean, sure = false): void {
  const key = tabKey(tab);
  const was = active.value;
  if (key !== was && !arrived && !sure && edited.value.has(was)) {
    leaving.value = { tab, hint };
    return;
  }
  leaving.value = null;
  for (const told of openers) told(tab);
  if (hint) hints.value = { ...hints.value, [key]: hint };
  drawerKeeper.away(key);
  clearSide();
  if (key !== was) {
    if (!arrived) placeKeeper.opening(tab);
    const next = was ? parked(behind, was, key) : { behind: behind.filter((one) => one !== key), gone: [] };
    behind = next.behind;
    for (const gone of next.gone) letGoOf(gone);
    batch(() => {
      tabs.value = [tab];
      closing.value = null;
      arrange(group(focused.value, [key]));
    });
  } else arrange(layout.value);
  drawerKeeper.back();
}

/** Goes on to what somebody was asked about leaving for: its changes saved or thrown away by then, or kept to come back to. */
export function leaveFor(asked: { tab: Tab; hint?: Hint }): void {
  showInPlace(asked.tab, asked.hint, false, true);
}

// Whoever keeps what somebody opened (what was lately opened, in the side panel) is told of each thing as it is
// opened or shown by them: by something pressed, and not by the page putting back the tabs it had, nor by a tab
// that comes to show because the one before it was closed.
const openers: ((tab: Tab) => void)[] = [];
/** Has `told` told of each tab somebody opens or shows. */
export function onOpened(told: (tab: Tab) => void): void {
  openers.push(told);
}

/** Whether what is opened now is what the browser came to by itself: see `arriving`. */
let arrivingNow = false;

/** Does `work`, which opens what the browser came to by itself (Back, Forward, an address): shown with no entry made in its history and nothing asked. */
export function arriving(work: () => void): void {
  const was = arrivingNow;
  arrivingNow = true;
  try {
    work();
  } finally {
    arrivingNow = was;
  }
}

/** Opens a thing: as a tab beside the others in the desktop window, and in place in a browser (`inplace.ts`). */
export function openTab(tab: Tab, hint?: Hint): void {
  if (inPlace) return showInPlace(tab, hint, arrivingNow);
  const key = tabKey(tab);
  if (!tabs.value.some((open) => tabKey(open) === key)) tabs.value = [...tabs.value, tab];
  for (const told of openers) told(tab);
  if (hint) hints.value = { ...hints.value, [key]: hint };
  drawerKeeper.away(key);
  clearSide();
  const next = show(layout.value, key, focused.value);
  arrange(next, holding(next, key)?.id);
  drawerKeeper.back();
}

/** Shows a tab that is open, in the group it is in. */
export function select(key: string): void {
  const home = holding(layout.value, key);
  if (!home) return;
  const tab = tabs.value.find((open) => tabKey(open) === key);
  if (tab) for (const told of openers) told(tab);
  drawerKeeper.away(key);
  clearSide();
  arrange(show(layout.value, key, home.id), home.id);
  drawerKeeper.back();
}

/** Makes a group the one new tabs open in. */
export function focusGroup(id: string): void {
  if (focused.value !== id) arrange(layout.value, id);
}

/** Moves a tab to sit before another in its own group, or to the end of it. */
export function moveTab(key: string, before: string | null): void {
  const home = holding(layout.value, key);
  if (home) placeTab(key, home.id, before);
}

/** Moves a tab into a group, before one of its tabs or at the end. */
export function placeTab(key: string, to: string, before: string | null): void {
  arrange(move(layout.value, key, to, before), to);
}

/** Splits a group and gives the new half to a tab; in the middle, the tab joins the group. */
export function dockTab(key: string, onto: string, zone: Zone): void {
  const next = dock(layout.value, key, onto, zone, fresh(layout.value));
  arrange(next, holding(next, key)?.id);
}

/** Moves a divider. It is remembered when it is let go: see `rememberLayout`. */
export function resizeSplit(path: string, share: number): void {
  layout.value = resize(layout.value, path, share);
}

export function rememberLayout(): void {
  tabsKeeper.arranged();
}

/** Closes a tab. One holding unsaved changes is asked about first, unless `sure`. */
export function closeTab(key: string, sure = false): void {
  // In a browser one thing is on the page: one that was shown before and is closed (its file deleted) is let go of where it is kept.
  if (inPlace && !tabs.value.some((tab) => tabKey(tab) === key) && behind.includes(key)) {
    forgetBehind(key);
    keepDraft(key, null);
    letGoOf(key);
    return;
  }
  if (!tabs.value.some((tab) => tabKey(tab) === key)) return;
  if (!sure && edited.value.has(key)) {
    closing.value = key;
    return;
  }
  if (closing.value === key) closing.value = null;
  keepDraft(key, null);
  // A sandbox test run from the tab is ended with it; what it showed, kept for when it showed again, goes, drawer and all.
  letGoOf(key);
  forgetBehind(key);
  const rest = tabs.value.filter((tab) => tabKey(tab) !== key);
  if (rest.length === 0) {
    // Nothing open is the start page, not an empty window.
    tabs.value = [{ kind: 'home' }];
    arrange(group(focused.value, ['home']));
    return;
  }
  tabs.value = rest;
  arrange(close(layout.value, key));
}

// ---- unsaved changes --------------------------------------------------------

/** What was typed in a tab and not saved, and what the file said when it was. */
export interface Draft {
  text: string;
  base: string;
  /** For something being said on a record: that it is to be marked public. */
  public?: boolean;
}

// A view is only on the page while its tab is showing. What was typed in it
// is kept here, so that looking at another tab does not throw it away. It is
// not kept across a reload of the page.
const drafts = new Map<string, Draft>();

export function draftOf(key: string): Draft | undefined {
  return drafts.get(key);
}

/** Keeps what was typed in a tab, or with `null` lets it go. */
export function keepDraft(key: string, draft: Draft | null): void {
  if (draft) drafts.set(key, draft);
  else drafts.delete(key);
  // Told only when a tab starts or stops having changes, not on every key.
  if (edited.value.has(key) !== drafts.has(key)) edited.value = new Set(drafts.keys());
}

export function takeHint(key: string): Hint | undefined {
  const hint = hints.value[key];
  if (hint) {
    const { [key]: _taken, ...rest } = hints.value;
    hints.value = rest;
  }
  return hint;
}

// ---- what a blueprint's editor is told ---------------------------------------

let described: Promise<BlueprintLanguage | null> | null = null;

/**
 * The blueprint language as the daemon describes it: read once, and again
 * after a package that may have brought actions was installed or removed,
 * which is all that changes it while the daemon runs. `null` when it could
 * not be read, in which case it is asked for again next time.
 */
export function blueprintLanguage(): Promise<BlueprintLanguage | null> {
  described ??= api.language().catch(() => {
    described = null;
    return null;
  });
  return described;
}

/** The blueprints that can be called from each project, as last read. Those for no project are under `''`. */
export const callable = signal<Record<string, Described[]>>({});
/** The other devices of this fleet, by name. */
export const deviceNames = signal<string[]>([]);

/** Reads which blueprints can be called from a project, and after that keeps it as the project's files change. */
export async function learnBlueprints(project: string | null): Promise<void> {
  try {
    const found = await api.blueprints(project);
    callable.value = { ...callable.value, [project ?? '']: found };
  } catch {
    /* the editor offers what it knew */
  }
}

/** What each project uses of the blueprints there are, as last read: what its list in the side panel is. */
export const used = signal<Record<string, Uses>>({});

/** Reads what each project uses. It is asked for again as instances come and go and as blueprint files do. */
export async function learnUsed(): Promise<void> {
  try {
    used.value = await api.blueprintsUsed();
  } catch {
    /* the lists stay as they were; a daemon older than this client has none to give */
  }
}

/** Something this page asked to have applied for a project was answered as alive there: the project uses it from now, before the reading that follows what its instance said is in. */
export function nowUses(project: string, blueprint: string): void {
  const uses = used.value[project] ?? { applied: [], own: [] };
  if (!uses.applied.includes(blueprint)) used.value = { ...used.value, [project]: { ...uses, applied: [...uses.applied, blueprint] } };
}

/** Reads which devices there are, for an editor that offers them by name. */
export async function learnDevices(): Promise<void> {
  try {
    deviceNames.value = (await api.devices()).devices.map((device) => device.name);
  } catch {
    /* not everyone may see the devices; none are offered */
  }
}

// ---- reading ---------------------------------------------------------------

export async function refreshProjects(): Promise<void> {
  projects.value = await api.projects();
  for (const project of projects.value) {
    if (!(project.id in trees.value)) void refreshTree(project.id);
    if (!(project.id in collections.value)) void refreshCollections(project.id);
  }
}

// Which reading of each project's tables was last asked for, and which is in.
const tablesAsked: Record<string, number> = {};
const tablesIn: Record<string, number> = {};
// How many changes to each project's tables this page has had answered (a view saved): a reading
// asked for before one and in after it says how the tables were, and is taken again (`inturn.ts`).
const tablesChanged: Record<string, number> = {};
/** A change this page made to one of a project's tables was answered. */
export function tablesChangedHere(project: string): void {
  tablesChanged[project] = (tablesChanged[project] ?? 0) + 1;
}

/** Reads what a project's blueprints have declared for it. */
export async function refreshCollections(project: string): Promise<void> {
  const asked = (tablesAsked[project] = (tablesAsked[project] ?? 0) + 1);
  const changes = tablesChanged[project] ?? 0;
  try {
    // Read first and put in afterwards: two projects are read at once, and
    // what the others' were is whatever it is when this one's has come.
    const declared = await api.collections(project);
    // One asked for before the reading that is in says how the tables were before it.
    if (asked < (tablesIn[project] ?? 0)) return;
    if (changes !== (tablesChanged[project] ?? 0)) return void refreshCollections(project);
    tablesIn[project] = asked;
    collections.value = { ...collections.value, [project]: declared };
  } catch {
    /* it keeps what it had: a project that is gone is taken out by its own event */
  }
}

export async function refreshTree(project: string): Promise<void> {
  try {
    const entries = await api.tree(project);
    trees.value = { ...trees.value, [project]: entries };
  } catch (err) {
    if (!(err instanceof ApiError && err.status === 404)) throw err;
  }
}

export async function refreshChannels(): Promise<void> {
  const read = await api.channels();
  // One that was listed and is not any more is kept by nothing here: it was taken away, kept from whoever reads this, or left.
  const lost = gone(channels.value, read);
  channels.value = read;
  lost.forEach(forgetChannel);
}

export async function refreshAgents(): Promise<void> {
  const answer = await api.agents();
  agents.value = answer.agents;
  adapters.value = { names: answer.adapters, fallback: answer.default_adapter };
  providers.value = answer.providers ?? [];
  permissionModes.value = answer.permissions ?? [];
}

// ---- staying current ---------------------------------------------------------

const projectsChanged = coalesce(() => void refreshProjects());
const channelsChanged = coalesce(() => void refreshChannels());
const agentsChanged = coalesce(() => void refreshAgents());
const treeChanged = new Map<string, () => void>();
const usedChanged = coalesce(() => void learnUsed(), 150);

stream.onStatus((up) => (connected.value = up));
stream.onStanding((now) => (standing.value = now));

// What a project uses is what is alive there: an instance that came or
// went may have changed it. Many do at once, so it is read once for them.
stream.on('instance.*', (event) => {
  if (changesUse(event.topic)) usedChanged();
});

// Trusting a project changes which of its own blueprints are its own, and so does where it keeps them.
stream.on(['project.created', 'project.removed', 'project.trusted', 'project.untrusted', 'project.blueprints'], (event) => {
  projectsChanged();
  usedChanged();
  if (event.topic === 'project.removed') {
    const { [event.data.project]: _gone, ...rest } = trees.value;
    trees.value = rest;
    const { [event.data.project]: _had, ...others } = collections.value;
    collections.value = others;
  }
});

// A project called something else, and a table: what the event says is what it is called, with nothing read again.
stream.on('project.renamed', (event) => {
  projects.value = projects.value.map((one) => (one.id === event.data.project ? { ...one, name: event.data.name } : one));
});
stream.on('collection.titled', (event) => {
  const { project, name, title } = event.data;
  const of = (one: Collection) => (one.name === name ? { ...one, title_chosen: title !== undefined && title !== one.title ? title : null } : one);
  if (collections.value[project]) collections.value = { ...collections.value, [project]: collections.value[project].map(of) };
});

// A project's folder made a git repository, or one no longer, is said as its repository changing: whether it has a Git page is read with the projects.
stream.on('project.git.changed', (event) => {
  const project = projects.value.find((one) => one.id === event.data.project);
  if (project && project.repository !== true) projectsChanged();
});

stream.on(['project.file.changed', 'project.folder.made', 'project.rescanned'], (event) => {
  const project: string = event.data.project;
  if (!treeChanged.has(project)) treeChanged.set(project, coalesce(() => void refreshTree(project)));
  treeChanged.get(project)!();
  if (event.data.path) {
    const key = `${project}:${event.data.path}`;
    fileVersions.value = { ...fileVersions.value, [key]: (fileVersions.value[key] ?? 0) + 1 };
  }
  // A blueprint that was written or removed changes what can be called,
  // and what the project's own are; any file the factory keeps for them, what each links.
  if (!event.data.path || isBlueprint(event.data.path) || event.data.path.startsWith('@blueprints/')) usedChanged();
  if (project in callable.value && (!event.data.path || isBlueprint(event.data.path))) void learnBlueprints(project);
});

// Where a project keeps its own is what its own are, and where a new one is written.
stream.on('project.blueprints', () => usedChanged());

// A package brings blueprints with it, and takes them away again; and
// actions, which are part of what an editor is told of the language.
stream.on(['extension.added', 'extension.removed'], () => {
  described = null;
  for (const project of Object.keys(callable.value)) void learnBlueprints(project || null);
});

// A blueprint has declared something for a project, or declared it again:
// a board that was not there may be now.
stream.on('collection.ensured', (event) => void refreshCollections(event.data.project));
// Somebody chose the icon of a table's column, or saved a view of a table, here or on another page: both are the table's, for everybody.
stream.on(['collection.icon', 'collection.views', 'collection.fields'], (event) => void refreshCollections(event.data.project));

// A daemon that has come back may be a newer one, with other actions.
stream.onStatus((up) => {
  if (!up) return;
  described = null;
  // And what came and went while it was away was not heard.
  if (me.value) usedChanged();
});

// Renamed, said what it is for or given managers, it is read again: its row, its tab and its page say it from the channels as they are read.
stream.on(['channel.created', 'channel.joined', 'channel.left', 'channel.kept', 'channel.unarchived', 'channel.renamed', 'channel.described', 'channel.managers'], channelsChanged);
// Archived, a channel waits for nobody: nothing of it is counted as unread from here, given back or not. It is read again
// as any channel is, and nothing kept of it is let go (`onChannelRemoved` is for a channel taken away).
stream.on('channel.archived', (event) => {
  const { [String(event.data.channel)]: _was, ...rest } = unread.value;
  unread.value = rest;
  channelsChanged();
});

// A channel that was taken away is kept by nothing here: its tab is
// closed, what was read of it is let go, and whatever else keeps something
// of a channel (its group and pin, its open thread, what was being written
// in it) is told, each where it keeps it. So is one that is listed no
// longer: kept to its members, of whom whoever reads this is not one.
const forgetters: ((channel: string) => void)[] = [];
/** Has `forget` told of each channel that is taken away, by its id. */
export function onChannelRemoved(forget: (channel: string) => void): void {
  forgetters.push(forget);
}
function forgetChannel(id: string): void {
  closeTab(`channel:${id}`, true);
  for (const read of [messages, unread] as { value: Record<string, unknown> }[]) {
    const { [id]: _gone, ...rest } = read.value;
    read.value = rest;
  }
  // A thread that was read is kept by the message it hangs from, which was the channel's.
  threads.value = Object.fromEntries(Object.entries(threads.value).filter(([, said]) => said[0]?.channel !== id));
  if (turned.value.has(`channel:${id}`)) turn(`channel:${id}`);
  for (const forget of forgetters) forget(id);
}
stream.on('channel.removed', (event) => {
  forgetChannel(String(event.data.channel));
  channelsChanged();
});

stream.on('channel.message', (event) => {
  const data = event.data;
  addMessage({
    id: data.id,
    channel: data.channel,
    author: data.author,
    author_name: data.author_name,
    body: data.body,
    reply_to: data.reply_to ?? null,
    model: data.model ?? null,
    thread: data.thread ?? null,
    attachments: data.attachments ?? [],
    said_by: data.said_by ?? null,
    mark: data.mark ?? null,
    created: event.ts,
  });
  const archived = channels.value.find((channel) => channel.id === data.channel)?.archived;
  if (!archived && !showing.value.has(`channel:${data.channel}`) && data.author !== me.value?.actor) {
    unread.value = { ...unread.value, [data.channel]: (unread.value[data.channel] ?? 0) + 1 };
  }
});

// Something attached made a file of a project: the message names that file from then on.
stream.on('channel.attachment.placed', (event) => placedFile(event.data));

// Which model an agent's program said it used is known at once, ahead of the reading: its answer comes next (`reportedNow`).
stream.on(['agent.slot.*', 'extension.*'], (event) => {
  if (event.topic === 'agent.slot.updated') agents.value = reportedNow(agents.value, event.data.slot, event.data.model);
  agentsChanged();
});

stream.on(['agent.turn.started', 'agent.output', 'agent.turn.finished'], (event) => {
  const slot: string = event.data.slot;
  if (event.topic === 'agent.turn.finished') {
    const { [slot]: _done, ...rest } = working.value;
    working.value = rest;
  } else if (event.topic === 'agent.turn.started') {
    working.value = { ...working.value, [slot]: { channel: event.data.channel ?? null, text: '', thread: event.data.thread ?? null, kind: event.data.kind ?? null } };
  } else if (working.value[slot]) {
    working.value = { ...working.value, [slot]: { ...working.value[slot]!, text: String(event.data.text ?? '') } };
  }
});

stream.on(['search.progress', 'search.indexed'], (event) => {
  if (event.topic === 'search.progress') indexing.value = event.data;
});

// ---- the client itself ---------------------------------------------------------

/** Acts on hearing which build of the client the daemon is serving. */
function clientIs(build: unknown, mayReload: boolean): void {
  const what = verdict(loadedFrom(document), build, edited.value.size > 0);
  if (what === 'reload' && mayReload) location.reload();
  else if (what !== 'current') reload.value = 'ready';
  // Back to the build this page is: there is nothing to reload for.
  else if (reload.value === 'ready') reload.value = null;
}

// The event is a stored one, so a page that was away (asleep, or cut off
// while the daemon restarted) is told when its stream resumes.
stream.on('client.updated', (event) => clientIs(event.data.build, true));

/**
 * For a part of the client that is fetched when first used. If it cannot be
 * fetched, the build this page is of has most likely been put away (it was
 * replaced more than a week ago, or twenty builds since), and the way on is
 * to reload.
 */
export function partMissing(err: unknown): void {
  console.warn('a part of the client could not be loaded', err);
  reload.value = 'broken';
}

// ---- signing in and out -------------------------------------------------------

/** Reads everything the shell shows and starts listening. */
async function enter(): Promise<void> {
  // A page that signed in before pages were given what they show beside
  // the cookie asks for it, or its stream and its pictures are refused.
  await api.learnShows();
  if (sendsOn()) return;
  // Listen first, then read: anything that changes while reading arrives as
  // an event and is read again, so nothing falls between the two.
  stream.open();
  await Promise.all([refreshProjects(), refreshChannels(), refreshAgents(), learnUsed(), refreshSpeaker()]);
  indexing.value = await api.searchStatus().catch(() => null);
  await tabsKeeper.back();
  entered.value = true;
}

export async function boot(): Promise<void> {
  // Opened at a link that signs somebody in once (`mw open`): traded for this page's session before anything is asked.
  await spendOnce();
  meta.value = await api.meta();
  document.title = titleOf(meta.value.name, meta.value.profile);
  session.onExpired = () => {
    if (me.value) leave();
  };
  session.onBehind = () => (daemonBehind.value = true);
  // A page brought back from the browser's memory can be older than what is
  // served. It only offers here: reloading while starting could go round.
  clientIs(meta.value.client_build, false);
  if (meta.value.signed_in) {
    try {
      me.value = await api.me();
      await enter();
    } catch (err) {
      if (!(err instanceof ApiError && err.status === 401)) throw err;
    }
  }
  ready.value = true;
}

export async function signedIn(): Promise<void> {
  me.value = await api.me();
  meta.value = await api.meta();
  await enter();
}

function leave(): void {
  stream.close();
  me.value = null;
  projects.value = [];
  entered.value = false;
  channels.value = [];
  agents.value = [];
  trees.value = {};
  collections.value = {};
  callable.value = {};
  messages.value = {};
  tabs.value = [{ kind: 'home' }];
  layout.value = group('g1', ['home']);
  focused.value = 'g1';
  drafts.clear();
  forgetAsking();
  forgetApplying();
  forgetSought();
  forgetAllOpened();
  forgetOpen();
  forgetSandboxes();
  forgetRunning();
  forgetAllReturned();
  edited.value = new Set();
  closing.value = null;
  leaving.value = null;
  behind = [];
  drawer.value = null;
}

/** Signs out here; or, asked to, out of every session this person has, wherever each was begun. */
export async function signOut(everywhere = false): Promise<void> {
  // Everywhere is asked as whoever is signed in, and can be refused: then nobody is signed out, here either.
  if (everywhere) await api.logoutEverywhere();
  else await api.logout().catch(() => {});
  try {
    tabsKeeper.forget();
    // What was open beside the channels, and what was being written in them, is the one who is leaving's.
    for (const key of ['threads', 'writing', 'front']) localStorage.removeItem(`${SAVED_CONTEXT}.${key}`);
    sessionStorage.removeItem(SAVED_WRITTEN);
  } catch {
    /* nothing was remembered */
  }
  leave();
}

export function isBlueprint(path: string): boolean {
  const said = meta.value;
  return isNamedBlueprint(path, said?.blueprint_exts ?? (said ? [said.blueprint_ext] : []));
}

export function projectName(id: string | null | undefined): string {
  return projects.value.find((project) => project.id === id)?.name ?? id ?? '';
}

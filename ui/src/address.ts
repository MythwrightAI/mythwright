// An address that opens a thing: a path that names it by the names people
// know it by, with how it is shown after the `#` (`tab.ts`: `addressOf`,
// `tabAt`): `/shop/test-runs#newest`, and a row of it in its form over it
// by its id (`/shop/test-runs#shop-test-runs-4`), whose address is copied
// from the form's head (`rowLink`).
//
// The address bar follows the tab that is showing (`history.replaceState`
// as tabs are shown, and as a view of a table is chosen), so that it can be
// copied as it is; Copy link in a page's head writes the
// same. A tab that no address opens (the start, a channel, settings) leaves
// the bar at the top of the page. Which tabs are open is still the
// reader's, kept in the browser for each workspace: an address opened
// shows its thing as a tab beside whatever the reader had open. It is
// opened once everything has been read and the tabs are back (`entered`),
// so an address opened by somebody who is not signed in waits in the bar
// and comes to its tab when they have signed in (signing out takes it out
// of the bar, with every set of tabs); and again whenever only
// what is after the `#` is changed under an open page. A link of the page
// to one of this factory's addresses is opened the same way, beside what
// is open, and not as the whole client again (`opens`).
//
// What the path names may take a project's tables to say (a table, a
// library or mockups by its name; where a library's documents are): those
// are read first where the page has not read them.
//
// An address written before addresses were paths (`/#/table/shop/tickets`)
// opens what it always did, and the bar then says it the new way.
//
// In a browser, where one thing shows at a time (`inplace.ts`), the
// address bar is also the way back: a thing opened in place is given an
// entry of the browser's history of its own (`placeKeeper`, told before it
// shows), Back and Forward show again what the bar then names, with
// nothing asked and nothing made in the history (`arriving`), and a line
// that says what it opens (`data-names`) pressed with Ctrl or ⌘ or the
// middle button opens it in a tab of the browser's own (`openApart`), as a
// link does.
//
// An address lets nobody in: it names a thing, and what is shown is what
// whoever opens it may read. One that names something that is not there
// shows that thing's own page saying so.

import { effect, signal, untracked } from '@preact/signals';
import { shownEntry, showEntry } from './referenced.ts';
import { aboutShowing, showAbout } from './aboutshown.ts';
import { gitShowing, showGit } from './gitshown.ts';
import { askMembers, membersShowing, membersTabOf } from './membersshown.ts';
import { inPlace, named, opensApart } from './inplace.ts';
import { active, arriving, collections, edited, entered, learnUsed, leaving, openTab, placeKeeper, refreshCollections, showCard, tabs, used } from './store.ts';
import { addressOf, tabAt, tabKey, thingOfLink } from './tab.ts';
import type { Asked, Known, Owns, Shown, Tab } from './tab.ts';
import { showView, viewShown } from './viewshown.ts';
import { isTopicId } from './topicpage.ts';
import { askTopic, topicShowing } from './topicshown.ts';
import { topics, topicsKnown } from './topicsread.ts';
import { addressNames, namedIn } from './wikipage.ts';
import { showRow } from './views/records.tsx';

/** What the page has read of each project's tables. */
const known: Known = (project) => collections.value[project];

/** What the page has read of each project's own blueprints: what its Blueprints line lists. */
const owns: Owns = (project) => used.value[project]?.own;

/** The view of a table this reader looks at, where it is not the one the table is shown by first, which an address need not say. */
function viewNow(project: string, name: string): string | undefined {
  const view = viewShown(project, name);
  const first = collections.value[project]?.find((one) => one.name === name)?.views[0]?.name ?? name;
  return view === null || view === first ? undefined : view;
}

/** How a thing is shown on this page now, as an address says it. */
function shownNow(tab: Tab): Shown {
  switch (tab.kind) {
    case 'board':
      return { view: viewNow(tab.project, 'board') };
    case 'table':
    case 'library':
    case 'mockups':
      return { view: viewNow(tab.project, tab.name) };
    case 'git':
      return { part: tab.at ?? (gitShowing.value[tab.project] || undefined) };
    case 'about':
      return { part: tab.at ?? (aboutShowing.value || undefined) };
    case 'members':
      // The tab that shows, where the page is drawn; and where it is not yet, the one it was kept at.
      return { part: active.value === 'members' ? membersShowing.value : tab.at };
    case 'topic': {
      const showing = topicShowing.value[tab.id];
      if (showing?.section) return { section: showing.section };
      return showing?.at !== undefined ? { part: showing.part, at: showing.at } : { part: showing?.part || undefined };
    }
    default:
      return {};
  }
}

/** A topic as its address names it (`addressNames`): by its wiki's title and its own as last read, where each says which it is, and else by its id. */
export function topicAsNamed(tab: Extract<Tab, { kind: 'topic' }>): Extract<Tab, { kind: 'topic' }> {
  // One the page has not read (several are called what its address said, or none) keeps the address it was opened by.
  if (!topics.value.some((one) => one.id === tab.id)) return tab;
  return { kind: 'topic', id: tab.id, ...addressNames(topics.value, tab.id) };
}

/** The address of a thing from the top of this page, as the bar says it; nothing for what has none. */
function here(tab: Tab): string | null {
  // The Reference is one tab: the entry last asked of it is how it is shown.
  const named = tab.kind === 'reference' && !tab.at && shownEntry.value ? { kind: 'reference' as const, at: shownEntry.value.at } : tab.kind === 'topic' ? topicAsNamed(tab) : tab;
  return addressOf(named, known, shownNow(named), owns);
}

/** The whole address that opens a thing here, as it is shown now, or nothing for what has none. */
export function linkTo(tab: Tab): string | null {
  const after = here(tab);
  return after === null ? null : `${location.origin}${after}`;
}

/** The whole address that opens a topic's article at one of its headings, by its slug. */
export function sectionLink(topic: string, section: string): string {
  return `${location.origin}${addressOf(topicAsNamed({ kind: 'topic', id: topic }), known, { section }, owns) ?? '/'}`;
}

/** The whole address that opens a row of a table in its form, over its table. */
export function rowLink(project: string, name: string, row: string): string {
  return `${location.origin}${addressOf({ kind: 'table', project, name }, known, { row }, owns) ?? '/'}`;
}

/** The address of a thing for a link on the page: opened beside what is open when it is pressed (`opens`). */
export function hrefOf(tab: Tab): string {
  return addressOf(tab.kind === 'topic' ? topicAsNamed(tab) : tab, known, {}, owns) ?? '/';
}

/** Shows what an address asks for, as it asks it to be shown. */
function show(asked: Extract<Asked, { tab: Tab }>): void {
  const { tab, shown } = asked;
  // The Reference is one tab: which entry it is opened at is said to the page, and is nothing a tab keeps.
  if (tab.kind === 'reference') {
    // Read again at the entry it was showing, it is left where the reader had it.
    if (active.peek() === 'reference' && (tab.at ?? '') === (shownEntry.peek()?.at ?? '')) return;
    return showEntry(tab.at);
  }
  if (tab.kind === 'git' && shown.part) return showGit(tab.project, shown.part);
  if (tab.kind === 'about' && shown.part) return showAbout(shown.part);
  // The Members page is asked for the tab the address says, its people where it says none: Back from the agents is the people.
  if (tab.kind === 'members') askMembers(membersTabOf(shown.part ?? tab.at));
  if (tab.kind === 'board' && shown.card) return showCard(shown.card, tab.project, shown.at);
  if (tab.kind === 'table' && shown.row) return showRow(tab.project, tab.name, shown.row);
  // A topic is asked for the tab the address says, its article where it says none, at the reply, revision or heading it says:
  // so that a link to a topic open on its talk shows its article, and one to a reply its talk there.
  if (tab.kind === 'topic') askTopic(tab.id, shown.part === 'talk' || shown.part === 'history' ? shown.part : 'article', shown.at ?? null, shown.section);
  if (shown.view && (tab.kind === 'board' || tab.kind === 'table' || tab.kind === 'library' || tab.kind === 'mockups'))
    showView(tab.project, tab.kind === 'board' ? 'board' : tab.name, shown.view);
  if (shown.line || shown.comments) openTab(tab, shown.line ? { line: shown.line } : { comments: true });
  // The tab showing already is not opened again: a page read again finds its own address in the bar, and nobody
  // opened anything by that (what was lately opened stays as it was).
  else if (tabKey(tab) !== active.peek()) openTab(tab);
}

/** What an address asks for, with a project's tables read first where the page has not read them and the path needs them. */
async function asked(address: string, read: (address: string, known: Known, owns: Owns) => Asked | null): Promise<Extract<Asked, { tab: Tab }> | null> {
  let first = read(address, known, owns);
  // A blueprint opened for a project whose own the page has not read (one added since) is read for first: it may be
  // the project's own, which is the file it is.
  if (first && 'tab' in first && first.tab.kind === 'blueprint' && first.tab.project && !used.peek()[first.tab.project]) {
    await learnUsed();
    first = read(address, known, owns);
  }
  // A page named by its title, and its wiki's, is the one the page reads to be called so, reading the topics first
  // where it has not: its tab is known by its id. One no page is called, and one several are, is left to its page,
  // which says so, and which they are.
  if (first && 'tab' in first && first.tab.kind === 'topic' && (first.tab.wiki || !isTopicId(first.tab.id))) {
    const found = namedIn(await topicsKnown(), first.tab.wiki ?? null, first.tab.id);
    return found && 'found' in found ? { tab: topicAsNamed({ kind: 'topic', id: found.found.id }), shown: first.shown } : first;
  }
  if (!first || 'tab' in first) return first;
  await refreshCollections(first.reads);
  // A project whose tables cannot be read is taken to have none: what is named is then shown saying so.
  const reads = first.reads;
  const second = read(address, (project) => known(project) ?? (project === reads ? [] : undefined), owns);
  return second && 'tab' in second ? second : null;
}

// While an address is being opened, the bar is left as it is.
const opening = signal(true);

/** Opens what the address in the bar names, beside what is open: or the one said, as it was when the bar changed to it. */
async function take(address = location.pathname + location.search + location.hash): Promise<void> {
  if (!entered.peek()) return;
  opening.value = true;
  try {
    const found = await asked(address, tabAt);
    // What the bar names is where the browser is already: nothing is made in its history for it.
    if (found && entered.peek()) arriving(() => show(found));
    // In a browser the top of the page is the start page, come back to by Back as anything is.
    else if (!found && inPlace && entered.peek() && location.pathname === '/' && !location.hash) arriving(() => openTab({ kind: 'home' }));
  } finally {
    opening.value = false;
    // Said the new way even where nothing changed: an address written before, or one that names nothing.
    follow();
  }
}

/** Puts the address of the tab showing in the bar. */
function follow(): void {
  if (!entered.peek() || opening.peek()) return;
  const tab = tabs.peek().find((open) => tabKey(open) === active.peek());
  const address = (tab && here(tab)) ?? '/';
  const now = location.pathname + location.search + location.hash;
  if (address !== now) history.replaceState(history.state, '', address);
}

/** Opens a thing in a tab of the browser's own, at its address: nothing where it has none. */
export function openApart(tab: Tab): void {
  const link = linkTo(tab) ?? `${location.origin}/`;
  window.open(link, '_blank', 'noopener');
}

/**
 * Opens one of this factory's addresses beside what is open, where `href`
 * is one: true where it is, so that whoever was asked does not follow it
 * as a link.
 */
export function opens(href: string): boolean {
  let whole: string;
  try {
    whole = new URL(href, location.href).href;
  } catch {
    return false;
  }
  const found = thingOfLink(whole, location, known, owns);
  if (!found) return false;
  void asked(whole, (address, known, owns) => thingOfLink(address, location, known, owns)).then((thing) => thing && show(thing));
  return true;
}

if (typeof window !== 'undefined') {
  let had = false;
  effect(() => {
    // Only for having entered: what opening a tab reads is not this effect's to follow.
    if (entered.value) untracked(() => void take());
    else {
      opening.value = true;
      // Signing out forgets every set of tabs, and the address of the one that was showing with them: whoever signs
      // in next begins where nothing was asked. One the page was opened at waits in the bar, never having entered.
      if (had) history.replaceState(history.state, '', '/');
    }
    had = entered.value;
  });
  // The bar follows the tab showing, the view chosen of it and what is read of the tables and the blueprints that say what it is.
  effect(() => {
    void [entered.value, opening.value, active.value, tabs.value, collections.value, used.value, shownEntry.value, topics.value, topicShowing.value];
    // What it reads of a table's view is followed too, by reading it here.
    const tab = tabs.value.find((open) => tabKey(open) === active.value);
    if (tab) void shownNow(tab);
    untracked(follow);
  });
  // Only what is after the `#` changed under an open page (somebody wrote it, or an address of before was opened here):
  // what it changed to, which the bar may say otherwise by now, Back and Forward having been followed first and the
  // bar put to the tab showing before that tab took up what it was asked (`#history/2` said as `#history`).
  window.addEventListener('hashchange', (event) => {
    const to = new URL(event.newURL);
    void take(to.pathname + to.search + to.hash);
  });
  if (inPlace) {
    // A thing opened in place is given an entry of the history of its own, said before it shows.
    placeKeeper.opening = (tab) => {
      if (!entered.peek()) return;
      history.pushState(null, '', here(tab) ?? '/');
    };
    // Back and Forward show what the bar names then, as it was left; a question about leaving goes, as the page it was about did.
    window.addEventListener('popstate', () => {
      leaving.value = null;
      void take();
    });
    // A line that says what it opens, pressed with Ctrl or ⌘ or by the middle button, opens it in a tab of the browser's own.
    const apart = (event: MouseEvent) => {
      if (!opensApart(event)) return;
      const line = (event.target as Element | null)?.closest?.('[data-names]');
      const tab = named(line?.getAttribute('data-names'));
      if (!tab) return;
      event.preventDefault();
      event.stopPropagation();
      openApart(tab);
    };
    document.addEventListener('click', apart, true);
    document.addEventListener('auxclick', apart, true);
    // The middle button held over such a line scrolls nothing: it opens what the line names as it comes up.
    document.addEventListener('mousedown', (event) => event.button === 1 && (event.target as Element | null)?.closest?.('[data-names]') && event.preventDefault(), true);
    // What is half written and not saved is asked about before the page itself is left, as any web page asks.
    window.addEventListener('beforeunload', (event) => {
      if (edited.peek().size === 0) return;
      event.preventDefault();
      event.returnValue = '';
    });
  }
  // A link on the page to one of this factory's addresses is opened beside what is open; with a key held, or in a
  // tab of the browser's own, it is the browser's, and the page it opens opens it.
  document.addEventListener('click', (event) => {
    if (event.defaultPrevented || event.button !== 0 || event.metaKey || event.ctrlKey || event.shiftKey || event.altKey) return;
    const link = (event.target as Element | null)?.closest?.('a[href]');
    if (!(link instanceof HTMLAnchorElement) || (link.target && link.target !== '_self') || link.hasAttribute('download')) return;
    if (opens(link.href)) event.preventDefault();
  });
}

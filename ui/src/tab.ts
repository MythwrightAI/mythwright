// What can be shown in a tab, and the key each is known by.
//
// Holds nothing of the page, so that what is worked out from tabs (which
// were lately opened, which are pinned) can be tested without one.

import { isSlug } from './contents.ts';

export type Tab =
  | { kind: 'home' }
  | { kind: 'channel'; id: string }
  | { kind: 'file'; project: string; path: string }
  | { kind: 'board'; project: string }
  /** A library of a project: the collection called `name`, whose records are documents. */
  | { kind: 'library'; project: string; name: string }
  /** The mockups of a project: the collection called `name`, whose records are folders that each hold a page. */
  | { kind: 'mockups'; project: string; name: string }
  /** Any other table of a project: the collection called `name`, as its rows. */
  | { kind: 'table'; project: string; name: string }
  /** A project's own page. */
  | { kind: 'project'; id: string }
  | { kind: 'instance'; id: string }
  /** One run of any blueprint, as a pipeline run: its stages, each step's status, time and whole output. By the run's id. */
  | { kind: 'run'; id: string }
  | { kind: 'activity' }
  | { kind: 'metrics' }
  /**
   * Everybody in the factory, a tab each for the people and the agents: the factory's page, under Resources. `at` is the
   * tab it shows (`membersshown.ts`: `people`, `agents`), said where it was opened at the agents (a tab kept from when they
   * were a page of their own) and no part of what the tab is known by.
   */
  | { kind: 'members'; at?: string }
  | { kind: 'extension'; name: string }
  /** A blueprint that is not a file: opened for a project, or for none, from the factory's list. */
  | { kind: 'blueprint'; name: string; project?: string }
  /** A project's files. Kept under the word open tabs were first kept by, which a browser may still hold. */
  | { kind: 'repo'; project: string }
  /**
   * A project's repository, where its folder is a git repository of its own: what git says of it, and what installed
   * packages show of it, a tab each. `at` is the tab that shows (`gittabs.ts` says how one is known: `pulls`), said
   * to whoever asks for the page's address and no part of what the tab is known by; the page keeps which shows.
   */
  | { kind: 'git'; project: string; at?: string }
  /** A project's environments (`dev`, `staging`, `prod`), each with its variables and secrets. */
  | { kind: 'environments'; project: string }
  /** A project's pipelines (its own blueprints under `pipelines/`), each with its latest runs and Run, and what else runs there. */
  | { kind: 'pipelines'; project: string }
  | { kind: 'settings' }
  /** What was deleted, kept to be given back: the factory's page, under Resources. */
  | { kind: 'trash' }
  /** The factory's secrets: the owner's page, reached from Settings. */
  | { kind: 'secrets' }
  | { kind: 'profile' }
  /** The notes of each release, which came with the program: opened from Help, at the panel's foot. */
  | { kind: 'releases' }
  /**
   * What the program is for, its guides and what is technical of it, opened
   * from its mark on the rail. One page, whatever it
   * is opened at: `at` is the tab it shows (`about.ts`: `docs`,
   * `docs/<guide>`, `technical`), said in its address after the `#`, and no
   * part of what the tab is known by.
   */
  | { kind: 'about'; at?: string }
  /**
   * One line of the inbox of whoever is signed in, as a page of its own:
   * what a blueprint told them, who told it and when, its words whole and
   * what it is about. By the line's id, which only its owner may read.
   * It has an address and is no thing kept to hand (`lately.ts`): it is
   * a message, and no thing of the factory's work.
   */
  | { kind: 'inbox'; id: string }
  /**
   * A forum's topic, a page of a wiki, by its id
   * (`@-topics-3`), which says nothing: `title` is what it was called as
   * it was opened and `wiki` what its wiki was called, where it is under
   * one, said in its address (`/topic/<wiki>/<title>`, `/topic/<title>`
   * for a wiki), and no part of what the tab is known by.
   */
  | { kind: 'topic'; id: string; title?: string; wiki?: string }
  /**
   * Everything a blueprint can say, as the daemon describes it: the words
   * of the language, the actions and the events. One page, whatever it is
   * opened at. `at` is the entry an address names (`reference.ts` says
   * how one is named: `action/collection.ensure/fields/quiet`): said as
   * the page is opened there, and no part of what the tab is known by.
   */
  | { kind: 'reference'; at?: string };

export function tabKey(tab: Tab): string {
  switch (tab.kind) {
    case 'channel':
      return `channel:${tab.id}`;
    case 'file':
      return `file:${tab.project}:${tab.path}`;
    case 'board':
      return `board:${tab.project}`;
    case 'library':
      return `library:${tab.project}:${tab.name}`;
    case 'mockups':
      return `mockups:${tab.project}:${tab.name}`;
    case 'table':
      return `table:${tab.project}:${tab.name}`;
    case 'project':
      return `project:${tab.id}`;
    case 'instance':
      return `instance:${tab.id}`;
    case 'run':
      return `run:${tab.id}`;
    case 'inbox':
      return `inbox:${tab.id}`;
    case 'topic':
      return `topic:${tab.id}`;
    case 'extension':
      return `extension:${tab.name}`;
    case 'blueprint':
      return `blueprint:${tab.project ?? ''}:${tab.name}`;
    case 'repo':
      return `repo:${tab.project}`;
    case 'git':
      return `git:${tab.project}`;
    case 'environments':
      return `environments:${tab.project}`;
    case 'pipelines':
      return `pipelines:${tab.project}`;
    default:
      return tab.kind;
  }
}

const part = (text: string) => text.split('/').map(encodeURIComponent).join('/');

/**
 * How a thing is kept where it is written down for the page alone (what is
 * pinned in Resources, with the account), and what an address written
 * before addresses were paths said after its `#`: `/table/shop/tickets`.
 * Nothing for what no address opens: what is always listed, a channel,
 * settings. A file's path and a blueprint's name keep their `/`; the
 * project a blueprint was opened for is said after a `?`. The Reference
 * is `/reference`, and one entry of it `/reference/<entry>`, the entry
 * as it is named with its `/` kept. A line of the inbox is
 * `/inbox/<line>`: it opens only for the line's owner. A run is
 * `/run/<run>`, and a project's pipelines `/pipelines/<project>`.
 */
export function keptAs(tab: Tab): string | null {
  switch (tab.kind) {
    case 'project':
    case 'instance':
    case 'run':
    case 'inbox':
      return `/${tab.kind}/${part(tab.id)}`;
    case 'pipelines':
      return `/pipelines/${part(tab.project)}`;
    case 'board':
      return `/board/${part(tab.project)}`;
    case 'repo':
      return `/files/${part(tab.project)}`;
    case 'git':
      return `/git/${part(tab.project)}`;
    case 'environments':
      return `/environments/${part(tab.project)}`;
    case 'library':
    case 'mockups':
    case 'table':
      return `/${tab.kind}/${part(tab.project)}/${part(tab.name)}`;
    case 'file':
      return `/file/${part(tab.project)}/${part(tab.path)}`;
    case 'blueprint':
      return `/blueprint/${part(tab.name)}${tab.project ? `?project=${encodeURIComponent(tab.project)}` : ''}`;
    case 'reference':
      return tab.at ? `/reference/${part(tab.at)}` : '/reference';
    case 'topic':
      return `/topic/${encodeURIComponent(tab.id)}`;
    default:
      return null;
  }
}

/** The thing what is kept says (`keptAs`), as an address written before addresses were paths said it after its `#`; nothing where it names none or makes no sense. */
export function fromKept(hash: string): Tab | null {
  const [way = '', asked = ''] = hash.replace(/^#/, '').split('?');
  if (!way.startsWith('/')) return null;
  let parts: string[];
  try {
    parts = way.slice(1).split('/').map(decodeURIComponent);
  } catch {
    return null;
  }
  const [kind, first, ...rest] = parts;
  // The Reference is one page: with nothing after it, the page; else the entry it is opened at.
  if (kind === 'reference') return first ? { kind, at: [first, ...rest].join('/') } : { kind };
  // So is the Members page, which has no word of its own a project could not take (`addressOf`).
  if (kind === 'members') return first ? null : { kind };
  if (!first || rest.some((one) => one === '')) return null;
  const more = rest.join('/');
  switch (kind) {
    case 'project':
    case 'instance':
    case 'run':
    case 'inbox':
      return more ? null : { kind, id: first };
    case 'pipelines':
      return more ? null : { kind, project: first };
    case 'topic':
      return more ? null : { kind, id: first };
    case 'board':
      return more ? null : { kind, project: first };
    case 'files':
      return more ? null : { kind: 'repo', project: first };
    case 'git':
      return more ? null : { kind, project: first };
    case 'environments':
      return more ? null : { kind, project: first };
    case 'library':
    case 'mockups':
    case 'table':
      return rest.length === 1 ? { kind, project: first, name: more } : null;
    case 'file':
      return more ? { kind, project: first, path: more } : null;
    case 'blueprint': {
      const project = new URLSearchParams(asked).get('project');
      const name = [first, ...rest].join('/');
      return project ? { kind, name, project } : { kind, name };
    }
    default:
      return null;
  }
}

// ---- addresses ----------------------------------------------------------------

/**
 * The first words of a path that are not a project's: the daemon's own
 * (its routes, `crates/api/src/routes/mod.rs`), the folders of the
 * client's files, the grammar's own beside projects, and those kept for
 * the factory's pages that have no address yet. The daemon refuses a
 * project whose id would be one (`kernel::paths::RESERVED`, which a test
 * holds this list to word for word), so that a path is the daemon's, the
 * client's or a thing's, and never two of them.
 */
export const RESERVED = {
  /** What the daemon answers itself: what is asked of it, a library's pages, mockups, what is said from outside. */
  daemon: ['api', 'library', 'mockups', 'in'],
  /** The folders the client's own files are served from (`ui/build.mjs`). */
  client: ['assets', 'lib'],
  /** The grammar's own, for what is no project's (`addressOf`). */
  grammar: ['run', 'instance', 'inbox', 'reference', 'blueprint', 'topic'],
  /**
   * The factory's own pages (`settings`, `activity`, ...), a channel (`channels/<id>`) and a package's page (`extensions/<name>`),
   * one taken out (`fleet`), and the agents' (`agents`), which opens the Members page at its Agents tab: kept so that no project
   * takes their word first.
   */
  pages: ['settings', 'activity', 'measurements', 'agents', 'fleet', 'trash', 'secrets', 'releases', 'about', 'profile', 'extensions', 'channels'],
} as const;

const RESERVED_WORDS: ReadonlySet<string> = new Set(Object.values(RESERVED).flat());

/** Whether a path's first word is not a project's: a project made before the word was kept has its addresses written the old way. */
export function isReserved(word: string): boolean {
  return RESERVED_WORDS.has(word);
}

/** Whether a path is the daemon's own or one of the client's files, and so no thing the client opens. */
export function isServedApart(path: string): boolean {
  const first = path.replace(/^\/+/, '').split('/')[0] ?? '';
  return (RESERVED.daemon as readonly string[]).includes(first) || (RESERVED.client as readonly string[]).includes(first);
}

/** The words after a project that are no table's: what a project has besides its tables. */
const UNDER_A_PROJECT = ['board', 'repo', 'git', 'environments', 'pipelines', 'blueprints', 'tables'] as const;

/** What the client knows of a project's tables, to say which a name is and where a library's documents are; nothing for a project whose tables it has not read. */
export type Known = (project: string) => readonly { name: string; store: string; at?: string | null }[] | undefined;
const NOTHING_KNOWN: Known = () => undefined;

/**
 * What the client knows of a project's own blueprints, each by its name
 * and the file it is (`/api/blueprints/used`, what its Blueprints line
 * in the side panel lists): nothing for a project whose list it has not
 * read. A fork of one that ships is among them, under the name it forks.
 */
export type Owns = (project: string) => readonly { name: string; file: string }[] | undefined;
const NOTHING_OWNED: Owns = () => undefined;

/**
 * How a thing is shown, after the `#`: a view of a table by its name
 * (`#newest`); a line of a file or a blueprint (`#L12`); a document's
 * comments (`#comments`); a card of a board, at a comment of it where
 * one is said (`#shop-17`, `#shop-17/c-482`); a row of any other table
 * in its form, by its id (`#shop-test-runs-4`); a topic's talk, at a reply
 * of it where one is said (`#talk`, `#talk/r-42`), and its history, at a
 * revision of it where one is said (`#history`, `#history/3`), its article
 * being what shows where nothing is said, and its article at one of its
 * headings by the heading's slug (`#s/how-it-began`, `contents.ts`): under
 * `s/`, so that no heading's words are ever taken for a tab.
 */
export interface Shown {
  view?: string;
  /** Of a table: the row whose form is opened over it, by its id. */
  row?: string;
  line?: number;
  comments?: boolean;
  card?: string;
  /** Of a card, the comment it is opened at; of a topic's talk, the reply, by the number it was said at; of its history, the revision, by its number. */
  at?: number;
  /**
   * Of a project's Git page, and of the page about the program: the tab it shows, by what the tab is known by
   * (`gittabs.ts`, `about.ts`); of a topic, `talk` or `history` where that shows.
   */
  part?: string;
  /** Of a topic's article, the heading it is read at, by its slug. */
  section?: string;
}

const libraryOf = (tables: ReturnType<Known>, name: string) => tables?.find((one) => one.name === name && one.store === 'documents');
const folderOfLibrary = (library: { name: string; at?: string | null }) => (library.at === '.' ? '' : (library.at ?? library.name));

/** The path that names a thing, by the names people know it by: nothing for what no address opens. */
function pathOf(tab: Tab, known: Known, owns: Owns): string | null {
  const project = 'project' in tab && typeof tab.project === 'string' ? tab.project : tab.kind === 'project' ? tab.id : null;
  // A project made before its word was kept is written as addresses were then, which still opens.
  if (project !== null && isReserved(project)) {
    const kept = keptAs(tab);
    return kept === null ? null : `/#${kept}`;
  }
  const at = (more: string) => `/${part(project!)}${more}`;
  switch (tab.kind) {
    case 'project':
      return at('');
    case 'board':
    case 'repo':
    case 'git':
    case 'environments':
    case 'pipelines':
      return at(`/${tab.kind}`);
    case 'library':
    case 'mockups':
    case 'table':
      return at(UNDER_A_PROJECT.includes(tab.name as (typeof UNDER_A_PROJECT)[number]) ? `/tables/${part(tab.name)}` : `/${part(tab.name)}`);
    case 'file': {
      // A project's own blueprint is named by its name, as it is called, wherever its file is kept.
      const own = owns(tab.project)?.find((one) => one.file === tab.path);
      if (own) return at(`/blueprints/${part(own.name)}`);
      // A document is named by its library and where it is in it; any other file by where it is in the project.
      for (const library of (known(tab.project) ?? []).filter((one) => one.store === 'documents')) {
        if (UNDER_A_PROJECT.includes(library.name as (typeof UNDER_A_PROJECT)[number])) continue;
        const folder = folderOfLibrary(library);
        if (folder && !tab.path.startsWith(`${folder}/`)) continue;
        const rest = folder ? tab.path.slice(folder.length + 1) : tab.path;
        if (rest.endsWith('.md')) return at(`/${part(library.name)}/${part(rest)}`);
      }
      return at(`/repo/${part(tab.path)}`);
    }
    case 'blueprint':
      if (!tab.project) return `/blueprint/${part(tab.name)}`;
      // Where the project has a blueprint of its own under the name (its fork of one that ships), the name under the
      // project is that one's; what ships is the factory's, said as opened for the project.
      if (owns(tab.project)?.some((one) => one.name === tab.name)) return `/blueprint/${part(tab.name)}?project=${encodeURIComponent(tab.project)}`;
      return at(`/blueprints/${part(tab.name)}`);
    case 'instance':
    case 'run':
    case 'inbox':
      return `/${tab.kind}/${part(tab.id)}`;
    case 'reference':
      return tab.at ? `/reference/${part(tab.at)}` : '/reference';
    case 'topic':
      // By its wiki's title and its own, each whole in one part of the path whatever it holds: never the pages between,
      // so that a page moved within its wiki keeps its address. Said by its id where it has no title to be said by.
      if (tab.title && tab.wiki) return `/topic/${encodeURIComponent(tab.wiki)}/${encodeURIComponent(tab.title)}`;
      return `/topic/${encodeURIComponent(tab.title ?? tab.id)}`;
    case 'channel':
      return `/channels/${part(tab.id)}`;
    case 'extension':
      return `/extensions/${part(tab.name)}`;
    default: {
      const page = PAGES.find(([, kind]) => kind === tab.kind);
      return page ? `/${page[0]}` : null;
    }
  }
}

/**
 * The factory's own pages, by the word an address names each by: they are
 * reached from the side panel and are no project's, and a browser keeps
 * them by these as it keeps a project's things by their paths.
 */
const PAGES = [
  ['settings', 'settings'],
  ['activity', 'activity'],
  ['measurements', 'metrics'],
  ['trash', 'trash'],
  ['secrets', 'secrets'],
  ['releases', 'releases'],
  ['about', 'about'],
  ['profile', 'profile'],
] as const satisfies readonly (readonly [(typeof RESERVED.pages)[number], Tab['kind']])[];

/** What goes after the `#` for how a thing is shown, without the `#`; nothing where nothing is said. */
function shownOf(tab: Tab, shown: Shown): string {
  const way = (text: string) => encodeURIComponent(text);
  if (tab.kind === 'board' && shown.card) return `${way(shown.card)}${shown.at !== undefined ? `/c-${shown.at}` : ''}`;
  if (tab.kind === 'table' && shown.row) return way(shown.row);
  if ((tab.kind === 'board' || tab.kind === 'table' || tab.kind === 'library' || tab.kind === 'mockups') && shown.view) return way(shown.view);
  if (tab.kind === 'git' && shown.part) return way(shown.part);
  // A guide is under the docs of the page about the program, with its `/` kept: `#docs/blueprints`.
  if (tab.kind === 'about' && shown.part) return part(shown.part);
  if (tab.kind === 'topic' && shown.part === 'talk') return shown.at !== undefined ? `talk/r-${shown.at}` : 'talk';
  if (tab.kind === 'topic' && shown.part === 'history') return shown.at !== undefined ? `history/${shown.at}` : 'history';
  if (tab.kind === 'topic' && shown.section) return `s/${encodeURIComponent(shown.section)}`;
  if ((tab.kind === 'file' || tab.kind === 'blueprint') && shown.line) return `L${shown.line}`;
  if (tab.kind === 'file' && shown.comments) return 'comments';
  return '';
}

/**
 * The address that opens a thing, from the top of this factory's page:
 * the path names the thing (`/shop/test-runs`) and what follows the `#`
 * how it is shown (`#newest`). Nothing for the start page, which is the
 * top of the page (`/`).
 *
 * Under a project (`/<project>`, its page): `board`, `repo` (its files),
 * `repo/<path>` (one of them), `git`, `environments`, `pipelines`,
 * `blueprints/<name>` (a blueprint opened for it: the project's own of
 * the name where it has one, its fork of one that ships among them, and
 * else what ships), `<table>` (a table, a
 * library or mockups by its name; `tables/<table>` for one called as one
 * of those words), and `<library>/<path>` (a document, by where it is in
 * its library); a row of a table is its table's address with the row's
 * id after the `#`. Of what is no project's: `run/<id>`, `instance/<id>`,
 * `inbox/<line>`, `reference` and `reference/<entry>`,
 * `blueprint/<name>` (with `?project=<project>` for what ships opened for
 * a project whose own stands in for it), `topic/<wiki>` and `topic/<wiki>/<page>` (a wiki
 * and a page of one, by their titles whole, a `/` in one written `%2F`;
 * by the id where a title does not say which), `channels/<id>`, `extensions/<name>`,
 * and the factory's own pages by their words (`settings`, `activity`,
 * `measurements`, `trash`, `secrets`, `releases`, `about`, `profile`), with
 * the Members page at `agents` (its Agents tab) and `/#/members` (its People):
 * a browser keeps each by its address, so that a thing is shown again by
 * Back and Forward and opened in a tab of the browser's own. A file's
 * path, a blueprint's name and an entry keep their `/`.
 */
export function addressOf(tab: Tab, known: Known = NOTHING_KNOWN, shown: Shown = {}, owns: Owns = NOTHING_OWNED): string | null {
  // The agents are where the Agents page was; the people have no word of the factory's own, which a project could have
  // taken, and are said as a thing is said whose word a project took (`/#/...`).
  if (tab.kind === 'members') return (shown.part ?? tab.at) === 'agents' ? '/agents' : '/#/members';
  const path = pathOf(tab, known, owns);
  if (path === null) return null;
  const after = path.startsWith('/#') ? '' : shownOf(tab, shown);
  return after ? `${path}#${after}` : path;
}

/** What an address asks for: a thing and how it is shown, or that a project's tables have to be read first to say which thing it is. */
export type Asked = { tab: Tab; shown: Shown } | { reads: string };

/** Words as they are to be found in an expression, and not what they would say there. */
const escaped = (words: string) => words.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');

function shownAt(tab: Tab, after: string): Shown {
  if (!after) return {};
  let text: string;
  try {
    text = decodeURIComponent(after);
  } catch {
    return {};
  }
  if (tab.kind === 'file' || tab.kind === 'blueprint') {
    const line = /^L([1-9][0-9]*)$/.exec(text);
    if (line) return { line: Number(line[1]) };
    if (tab.kind === 'file' && text === 'comments') return { comments: true };
    return {};
  }
  if (tab.kind === 'git' || tab.kind === 'about') return { part: text };
  if (tab.kind === 'topic') {
    const talk = /^talk(?:\/r-([0-9]+))?$/.exec(text);
    if (talk) return talk[1] ? { part: 'talk', at: Number(talk[1]) } : { part: 'talk' };
    const history = /^history(?:\/([1-9][0-9]*))?$/.exec(text);
    if (history) return history[1] ? { part: 'history', at: Number(history[1]) } : { part: 'history' };
    const section = /^s\/(.+)$/.exec(text);
    if (section && isSlug(section[1]!)) return { section: section[1] };
    return {};
  }
  if (tab.kind === 'board') {
    // A card of the board is known by its id, which is the project's and a number.
    const card = new RegExp(`^(${escaped(tab.project)}-[0-9]+)(?:/c-([0-9]+))?$`).exec(text);
    if (card) return card[2] ? { card: card[1], at: Number(card[2]) } : { card: card[1] };
  }
  // A row of a table is known by its id, which is the project's, the table's and a number, as a record's is (`record_id`).
  if (tab.kind === 'table' && new RegExp(`^${escaped(tab.project)}-${escaped(tab.name)}-[0-9]+$`).test(text)) return { row: text };
  if (tab.kind === 'board' || tab.kind === 'table' || tab.kind === 'library' || tab.kind === 'mockups') return { view: text };
  return {};
}

/**
 * What an address opens: the thing its path names and how it is shown,
 * from the path, what is after its `?` and what is after its `#`
 * (`/shop/test-runs#newest`). An address written before addresses were
 * paths (`/#/table/shop/tickets`) opens what it always did. Nothing where
 * it names nothing or makes no sense.
 */
export function tabAt(address: string, known: Known = NOTHING_KNOWN, owns: Owns = NOTHING_OWNED): Asked | null {
  const hashAt = address.indexOf('#');
  const [path, after] = hashAt < 0 ? [address, ''] : [address.slice(0, hashAt), address.slice(hashAt + 1)];
  if (after.startsWith('/')) {
    const tab = fromKept(after);
    return tab ? { tab, shown: {} } : null;
  }
  const [way = '', asked = ''] = path.split('?');
  if (!way.startsWith('/')) return null;
  let parts: string[];
  try {
    parts = way.slice(1).replace(/\/$/, '').split('/').map(decodeURIComponent);
  } catch {
    return null;
  }
  if (parts.some((one) => one === '')) return null;
  const named = (tab: Tab): Asked => ({ tab, shown: shownAt(tab, after) });
  const [first, second, ...rest] = parts;
  if (!first) return null;
  const more = rest.join('/');
  switch (first) {
    case 'reference':
      return named(second ? { kind: 'reference', at: [second, ...rest].join('/') } : { kind: 'reference' });
    case 'blueprint': {
      if (!second) return null;
      // As opened for a project: what ships, where the project's own blueprint of the name stands in for it.
      const project = new URLSearchParams(asked).get('project');
      const name = [second, ...rest].join('/');
      return named(project ? { kind: 'blueprint', name, project } : { kind: 'blueprint', name });
    }
    case 'run':
    case 'instance':
    case 'inbox':
      return second && !more ? named({ kind: first, id: second }) : null;
    case 'topic': {
      // By its title, or its id, and its wiki's before it where it is under one: which page that is, is the page's to
      // find among the topics it read (`address.ts`).
      if (!second || rest.length > 1) return null;
      const page = rest[0];
      return named(page ? { kind: 'topic', id: page, title: page, wiki: second } : { kind: 'topic', id: second, title: second });
    }
    case 'channels':
      return second && !more ? named({ kind: 'channel', id: second }) : null;
    case 'extensions':
      return second && !more ? named({ kind: 'extension', name: second }) : null;
  }
  // The Agents page's word, which a bookmark or a link from before may say: the Members page at its Agents tab.
  if (first === 'agents') return second === undefined ? { tab: { kind: 'members', at: 'agents' }, shown: { part: 'agents' } } : null;
  const page = PAGES.find(([word]) => word === first);
  if (page) return second === undefined ? named({ kind: page[1] } as Tab) : null;
  if (isReserved(first)) return null;
  const project = first;
  if (second === undefined) return named({ kind: 'project', id: project });
  switch (second) {
    case 'board':
    case 'git':
    case 'environments':
    case 'pipelines':
      return more ? null : named({ kind: second, project });
    case 'repo':
      return named(more ? { kind: 'file', project, path: more } : { kind: 'repo', project });
    case 'blueprints': {
      if (!more) return null;
      // The project's own of the name, its fork where it is one, is the file it is, as its Blueprints line opens it.
      const own = owns(project)?.find((one) => one.name === more);
      return named(own ? { kind: 'file', project, path: own.file } : { kind: 'blueprint', name: more, project });
    }
  }
  const tables = known(project);
  const name = second === 'tables' ? rest[0] : second;
  if (second === 'tables' ? rest.length === 1 : rest.length === 0) {
    if (!tables) return { reads: project };
    const table = tables.find((one) => one.name === name);
    if (name === 'board' && table) return named({ kind: 'board', project });
    const kind = table?.store === 'documents' ? 'library' : table?.store === 'mockups' ? 'mockups' : 'table';
    return named({ kind, project, name: name! });
  }
  if (second === 'tables') return null;
  // A document, by its library and where it is in it: where the library is not known, its folder is its name, as a library's is unless it says.
  if (!tables) return { reads: project };
  const library = libraryOf(tables, second);
  const folder = library ? folderOfLibrary(library) : second;
  return named({ kind: 'file', project, path: folder ? `${folder}/${more}` : more });
}

/**
 * What a link opens where it is an address of this very factory's page
 * (`here`, its origin) that names a thing: a told line's link to a run is
 * opened as a tab beside what is open, and not as the whole client again
 * in a tab of the browser's. Nothing for any other address, and for what
 * the daemon serves itself (a library's pages, mockups).
 */
export function thingOfLink(href: string, here: { origin: string }, known: Known = NOTHING_KNOWN, owns: Owns = NOTHING_OWNED): Asked | null {
  let to: URL;
  try {
    to = new URL(href);
  } catch {
    return null;
  }
  if (to.origin !== here.origin || isServedApart(to.pathname)) return null;
  return tabAt(`${to.pathname}${to.search}${to.hash}`, known, owns);
}

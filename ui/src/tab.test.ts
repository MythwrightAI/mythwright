import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import { RESERVED, addressOf, fromKept, isReserved, isServedApart, keptAs, tabAt, tabKey, thingOfLink } from './tab.ts';
import type { Known, Owns, Shown, Tab } from './tab.ts';

// What the page has read of the projects' tables: shop's, with a library kept in a folder of its own name, one
// kept somewhere else, mockups, a table and the board; and nothing of yard's.
const SHOP = [
  { name: 'board', store: 'local' },
  { name: 'docs', store: 'documents' },
  { name: 'handbook', store: 'documents', at: 'guides/handbook' },
  { name: 'mockups', store: 'mockups' },
  { name: 'test-runs', store: 'local' },
  { name: 'git', store: 'local' },
];
const known: Known = (project) => (project === 'shop' ? SHOP : undefined);

// A thing, how it is shown, and the address that opens it so.
const THINGS: [Tab, Shown, string][] = [
  [{ kind: 'project', id: 'shop' }, {}, '/shop'],
  [{ kind: 'board', project: 'shop' }, {}, '/shop/board'],
  [{ kind: 'board', project: 'shop' }, { view: 'by label' }, '/shop/board#by%20label'],
  [{ kind: 'board', project: 'shop' }, { card: 'shop-17' }, '/shop/board#shop-17'],
  [{ kind: 'board', project: 'shop' }, { card: 'shop-17', at: 482 }, '/shop/board#shop-17/c-482'],
  [{ kind: 'table', project: 'shop', name: 'test-runs' }, {}, '/shop/test-runs'],
  [{ kind: 'table', project: 'shop', name: 'test-runs' }, { view: 'newest' }, '/shop/test-runs#newest'],
  [{ kind: 'table', project: 'shop', name: 'test-runs' }, { row: 'shop-test-runs-4' }, '/shop/test-runs#shop-test-runs-4'],
  [{ kind: 'table', project: 'shop', name: 'git' }, {}, '/shop/tables/git'],
  [{ kind: 'table', project: 'shop', name: 'git' }, { row: 'shop-git-2' }, '/shop/tables/git#shop-git-2'],
  [{ kind: 'library', project: 'shop', name: 'docs' }, {}, '/shop/docs'],
  [{ kind: 'mockups', project: 'shop', name: 'mockups' }, { view: 'gallery' }, '/shop/mockups#gallery'],
  [{ kind: 'file', project: 'shop', path: 'docs/guides/install.md' }, {}, '/shop/docs/guides/install.md'],
  [{ kind: 'file', project: 'shop', path: 'docs/guides/install.md' }, { comments: true }, '/shop/docs/guides/install.md#comments'],
  [{ kind: 'file', project: 'shop', path: 'guides/handbook/first.md' }, {}, '/shop/handbook/first.md'],
  [{ kind: 'file', project: 'shop', path: 'src/main.rs' }, { line: 12 }, '/shop/repo/src/main.rs#L12'],
  [{ kind: 'file', project: 'shop', path: 'docs/notes.txt' }, {}, '/shop/repo/docs/notes.txt'],
  [{ kind: 'file', project: 'shop', path: 'a#b?.md' }, {}, '/shop/repo/a%23b%3F.md'],
  [{ kind: 'file', project: 'shop', path: '@blueprints/notes.bp' }, {}, '/shop/repo/%40blueprints/notes.bp'],
  [{ kind: 'repo', project: 'shop' }, {}, '/shop/repo'],
  [{ kind: 'git', project: 'shop' }, {}, '/shop/git'],
  [{ kind: 'environments', project: 'shop' }, {}, '/shop/environments'],
  [{ kind: 'pipelines', project: 'shop' }, {}, '/shop/pipelines'],
  [{ kind: 'blueprint', name: 'std/board', project: 'shop' }, { line: 3 }, '/shop/blueprints/std/board#L3'],
  [{ kind: 'blueprint', name: 'std/board' }, {}, '/blueprint/std/board'],
  [{ kind: 'instance', id: 'in_4f2a' }, {}, '/instance/in_4f2a'],
  [{ kind: 'run', id: 'run_1m85442mg8j93j1c' }, {}, '/run/run_1m85442mg8j93j1c'],
  [{ kind: 'inbox', id: 'inb_01h2x' }, {}, '/inbox/inb_01h2x'],
  [{ kind: 'reference' }, {}, '/reference'],
  [{ kind: 'reference', at: 'action/collection.ensure/fields/quiet' }, {}, '/reference/action/collection.ensure/fields/quiet'],
  [{ kind: 'reference', at: 'event/board.item.moved' }, {}, '/reference/event/board.item.moved'],
];

test('a thing has an address that names it by its path and how it is shown after the #, and the address opens it so', () => {
  for (const [tab, shown, address] of THINGS) {
    assert.equal(addressOf(tab, known, shown), address);
    const opened = tabAt(address, known);
    assert.ok(opened && 'tab' in opened, address);
    assert.equal(tabKey(opened.tab), tabKey(tab));
    assert.deepEqual(opened.tab, tab, address);
    assert.deepEqual(opened.shown, Object.fromEntries(Object.entries(shown).filter(([, value]) => value !== undefined)), address);
  }
});

// What the page has read of the projects' own blueprints: shop's, one kept by the factory, its fork of the board that
// ships and one in a folder of its repository it said; and nothing of yard's.
const OWN = [
  { name: 'nudge', file: '@blueprints/nudge.bp' },
  { name: 'std/board', file: '@blueprints/std/board.bp' },
  { name: 'ideas', file: 'blueprints/ideas.bp.yml' },
];
const owns: Owns = (project) => (project === 'shop' ? OWN : undefined);

test('a project’s own blueprint is opened by its name under the project as the file it is, and its file says that address', () => {
  const pairs: [Tab, Shown, string][] = [
    [{ kind: 'file', project: 'shop', path: '@blueprints/nudge.bp' }, {}, '/shop/blueprints/nudge'],
    [{ kind: 'file', project: 'shop', path: '@blueprints/nudge.bp' }, { line: 4 }, '/shop/blueprints/nudge#L4'],
    // A fork of one that ships is under the name it forks.
    [{ kind: 'file', project: 'shop', path: '@blueprints/std/board.bp' }, {}, '/shop/blueprints/std/board'],
    // One in a folder of its repository is named the same way: the name is what it is called, wherever it is kept.
    [{ kind: 'file', project: 'shop', path: 'blueprints/ideas.bp.yml' }, {}, '/shop/blueprints/ideas'],
    // What ships, opened for a project that has a fork of it, is what ships as opened for that project.
    [{ kind: 'blueprint', name: 'std/board', project: 'shop' }, {}, '/blueprint/std/board?project=shop'],
    [{ kind: 'blueprint', name: 'std/board', project: 'shop' }, { line: 3 }, '/blueprint/std/board?project=shop#L3'],
    // What ships, opened for a project with none of its own under the name, is under the project as it always was.
    [{ kind: 'blueprint', name: 'std/cards', project: 'shop' }, {}, '/shop/blueprints/std/cards'],
    // A file of the place for its own that is no blueprint of it (a prompt one links) is a file of the project.
    [{ kind: 'file', project: 'shop', path: '@blueprints/prompts/nudge.md' }, {}, '/shop/repo/%40blueprints/prompts/nudge.md'],
  ];
  for (const [tab, shown, address] of pairs) {
    assert.equal(addressOf(tab, known, shown, owns), address);
    assert.deepEqual(tabAt(address, known, owns), { tab, shown }, address);
  }
  // Its file's own address opens the same tab, whose address is then its name's.
  assert.deepEqual(tabAt('/shop/repo/%40blueprints/nudge.bp', known, owns), { tab: { kind: 'file', project: 'shop', path: '@blueprints/nudge.bp' }, shown: {} });
  // A name the project has nothing of is opened as what ships under it, whose page says where there is none.
  assert.deepEqual(tabAt('/shop/blueprints/nowhere', known, owns), { tab: { kind: 'blueprint', name: 'nowhere', project: 'shop' }, shown: {} });
  // Nothing read of a project's own: what ships under the name, as before.
  assert.deepEqual(tabAt('/yard/blueprints/nudge', known, owns), { tab: { kind: 'blueprint', name: 'nudge', project: 'yard' }, shown: {} });
  assert.equal(addressOf({ kind: 'file', project: 'shop', path: '@blueprints/nudge.bp' }), '/shop/repo/%40blueprints/nudge.bp');
  // A link to it on this very page is opened as it.
  assert.deepEqual(thingOfLink('http://here:7700/shop/blueprints/nudge', { origin: 'http://here:7700' }, known, owns), {
    tab: { kind: 'file', project: 'shop', path: '@blueprints/nudge.bp' },
    shown: {},
  });
});

test('a slash at the end of a path, as an address is often written, names the same thing', () => {
  assert.deepEqual(tabAt('/shop/test-runs/#newest', known), { tab: { kind: 'table', project: 'shop', name: 'test-runs' }, shown: { view: 'newest' } });
  assert.deepEqual(tabAt('/shop/', known), { tab: { kind: 'project', id: 'shop' }, shown: {} });
});

test('which a table is, and where a library keeps its documents, wait for the project’s tables to be read', () => {
  assert.deepEqual(tabAt('/yard/test-runs', known), { reads: 'yard' });
  assert.deepEqual(tabAt('/yard/docs/guides/install.md', known), { reads: 'yard' });
  // Read and none there: a table of that name (whose page says it is not there), and a library's folder is its name.
  const none: Known = (project) => (project === 'yard' ? [] : undefined);
  assert.deepEqual(tabAt('/yard/test-runs', none), { tab: { kind: 'table', project: 'yard', name: 'test-runs' }, shown: {} });
  assert.deepEqual(tabAt('/yard/docs/guides/install.md', none), { tab: { kind: 'file', project: 'yard', path: 'docs/guides/install.md' }, shown: {} });
  // What needs none of them is named at once.
  assert.deepEqual(tabAt('/yard/board#by%20label', known), { tab: { kind: 'board', project: 'yard' }, shown: { view: 'by label' } });
  assert.deepEqual(tabAt('/yard/repo/README.md#L4', known), { tab: { kind: 'file', project: 'yard', path: 'README.md' }, shown: { line: 4 } });
  // A file's address with nothing read of its project is written as a file of the project.
  assert.equal(addressOf({ kind: 'file', project: 'yard', path: 'docs/a.md' }), '/yard/repo/docs/a.md');
});

test('what is after the # says only what that kind of thing can be shown as', () => {
  assert.deepEqual(tabAt('/shop/repo/src/main.rs#nothing', known), { tab: { kind: 'file', project: 'shop', path: 'src/main.rs' }, shown: {} });
  assert.deepEqual(tabAt('/run/run_1#L3', known), { tab: { kind: 'run', id: 'run_1' }, shown: {} });
  assert.deepEqual(tabAt('/shop/board#yard-2', known), { tab: { kind: 'board', project: 'shop' }, shown: { view: 'yard-2' } }, 'another project’s card is no card of this board');
  assert.equal(addressOf({ kind: 'run', id: 'run_1' }, known, { view: 'x', line: 3 }), '/run/run_1');
  // A row is of its table and its project: one of another is no row of it, and is read as a view's name, as a card is.
  const runs = { kind: 'table', project: 'shop', name: 'test-runs' } as const;
  assert.deepEqual(tabAt('/shop/test-runs#yard-test-runs-2', known), { tab: runs, shown: { view: 'yard-test-runs-2' } });
  assert.deepEqual(tabAt('/shop/test-runs#shop-notes-2', known), { tab: runs, shown: { view: 'shop-notes-2' } });
  assert.deepEqual(tabAt('/shop/board#shop-board-2', known), { tab: { kind: 'board', project: 'shop' }, shown: { view: 'shop-board-2' } });
  assert.equal(addressOf({ kind: 'library', project: 'shop', name: 'docs' }, known, { row: 'shop-docs-1' }), '/shop/docs', 'a document has an address of its own');
});

test('a project’s Git page says the tab it shows after the #, and an address opens it at that tab', () => {
  assert.equal(addressOf({ kind: 'git', project: 'shop' }, known, { part: 'pulls' }), '/shop/git#pulls');
  assert.equal(addressOf({ kind: 'git', project: 'shop' }, known, {}), '/shop/git');
  assert.deepEqual(tabAt('/shop/git#pulls', known), { tab: { kind: 'git', project: 'shop' }, shown: { part: 'pulls' } });
  assert.deepEqual(tabAt('/shop/git#sample.pulls', known), { tab: { kind: 'git', project: 'shop' }, shown: { part: 'sample.pulls' } });
  assert.deepEqual(tabAt('/shop/git', known), { tab: { kind: 'git', project: 'shop' }, shown: {} });
});

test('an address written before addresses were paths opens what it always did', () => {
  for (const [old, tab] of [
    ['/#/table/shop/tickets', { kind: 'table', project: 'shop', name: 'tickets' }],
    ['/#/board/shop', { kind: 'board', project: 'shop' }],
    ['/#/file/shop/docs/guides/how%20to.md', { kind: 'file', project: 'shop', path: 'docs/guides/how to.md' }],
    ['/#/blueprint/std/board?project=shop', { kind: 'blueprint', name: 'std/board', project: 'shop' }],
    ['/#/reference/action/kv%2Fget/key', { kind: 'reference', at: 'action/kv/get/key' }],
    ['/#/run/run_1', { kind: 'run', id: 'run_1' }],
    ['/#/files/shop', { kind: 'repo', project: 'shop' }],
    // Written into the bar under an open page: what is after the # is what it names.
    ['/shop/test-runs#/project/shop', { kind: 'project', id: 'shop' }],
  ] as [string, Tab][]) {
    assert.deepEqual(tabAt(old, known), { tab, shown: {} }, old);
  }
});

test('an address that names nothing, or makes no sense, opens nothing', () => {
  for (const address of ['', '/', '#', '/#', '/#/', 'shop/board', '/shop//board', '/shop/board/more', '/shop/git/more', '/shop/tables', '/shop/tables/a/b', '/shop/blueprints', '/run', '/run/a/b', '/inbox/', '/blueprint', '/settings/shop', '/api/meta', '/assets/app.js', '/shop/repo/%E0%A4%A', '/#/table/shop', '/#/settings']) {
    assert.equal(tabAt(address, known), null, address);
  }
});

test('the Reference is one tab whatever entry it is opened at', () => {
  assert.equal(tabKey({ kind: 'reference' }), 'reference');
  assert.equal(tabKey({ kind: 'reference', at: 'event/board.item.moved' }), 'reference');
  assert.deepEqual(tabAt('/reference/', known), { tab: { kind: 'reference' }, shown: {} });
  assert.deepEqual(tabAt('/reference/action/kv%2Fget/key', known), { tab: { kind: 'reference', at: 'action/kv/get/key' }, shown: {} });
});

test('the page about the program is one tab, and its address says the tab it shows and the guide read under Docs', () => {
  assert.equal(tabKey({ kind: 'about', at: 'technical' }), 'about');
  assert.equal(addressOf({ kind: 'about' }, known, { part: 'technical' }), '/about#technical');
  assert.equal(addressOf({ kind: 'about' }, known, { part: 'docs/blueprints' }), '/about#docs/blueprints');
  assert.equal(addressOf({ kind: 'about' }, known), '/about');
  assert.deepEqual(tabAt('/about#docs/blueprints', known), { tab: { kind: 'about' }, shown: { part: 'docs/blueprints' } });
  assert.deepEqual(tabAt('/about#technical', known), { tab: { kind: 'about' }, shown: { part: 'technical' } });
  assert.equal(tabAt('/about/more', known), null);
});

test('the start page has no address: it is the top of the page', () => {
  assert.equal(addressOf({ kind: 'home' }, known), null);
});

test('a channel, a package’s page and the factory’s own pages each have an address that opens them', () => {
  const pages: [Tab, string][] = [
    [{ kind: 'channel', id: 'c1' }, '/channels/c1'],
    [{ kind: 'channel', id: 'shop general' }, '/channels/shop%20general'],
    [{ kind: 'extension', name: 'weather' }, '/extensions/weather'],
    [{ kind: 'settings' }, '/settings'],
    [{ kind: 'activity' }, '/activity'],
    [{ kind: 'metrics' }, '/measurements'],
    [{ kind: 'trash' }, '/trash'],
    [{ kind: 'secrets' }, '/secrets'],
    [{ kind: 'releases' }, '/releases'],
    [{ kind: 'about' }, '/about'],
    [{ kind: 'profile' }, '/profile'],
  ];
  for (const [tab, address] of pages) {
    assert.equal(addressOf(tab, known), address);
    assert.deepEqual(tabAt(address, known), { tab, shown: {} }, address);
    // Each word is one kept from projects, so that no project's page is ever one of these.
    assert.ok(isReserved(address.split('/')[1]!), address);
  }
  for (const address of ['/settings/more', '/channels', '/channels/a/b', '/extensions', '/fleet', '/activity/shop']) assert.equal(tabAt(address, known), null, address);
});

test('the Members page is at its People tab by the old way of saying a thing, and at its Agents tab where the Agents page was', () => {
  // Nothing says which tab: the people, which is where the page opens.
  assert.equal(addressOf({ kind: 'members' }, known), '/#/members');
  assert.equal(addressOf({ kind: 'members' }, known, { part: 'people' }), '/#/members');
  assert.equal(addressOf({ kind: 'members' }, known, { part: 'agents' }), '/agents');
  // A tab kept at the agents, with nothing said of what shows now, is at the agents.
  assert.equal(addressOf({ kind: 'members', at: 'agents' }, known), '/agents');
  assert.deepEqual(tabAt('/#/members', known), { tab: { kind: 'members' }, shown: {} });
  // A bookmark, a link in a guide or `mw open` from when the agents were a page of their own opens them where they are now.
  assert.deepEqual(tabAt('/agents', known), { tab: { kind: 'members', at: 'agents' }, shown: { part: 'agents' } });
  assert.deepEqual(thingOfLink('http://127.0.0.1:7700/agents', { origin: 'http://127.0.0.1:7700' }, known), { tab: { kind: 'members', at: 'agents' }, shown: { part: 'agents' } });
  for (const address of ['/agents/more', '/#/members/more']) assert.equal(tabAt(address, known), null, address);
  // A project may be called `members`: its page is its own, since the Members page took no word from projects.
  assert.equal(addressOf({ kind: 'project', id: 'members' }, known), '/members');
  assert.deepEqual(tabAt('/members', known), { tab: { kind: 'project', id: 'members' }, shown: {} });
});

test('a project made before its word was kept has its addresses written as they were then, and they open', () => {
  for (const tab of [{ kind: 'project', id: 'run' }, { kind: 'board', project: 'reference' }, { kind: 'table', project: 'agents', name: 'notes' }, { kind: 'file', project: 'inbox', path: 'a.md' }] as Tab[]) {
    const address = addressOf(tab, known, { view: 'newest' })!;
    assert.ok(address.startsWith('/#/'), address);
    assert.deepEqual(tabAt(address, known), { tab, shown: {} });
  }
});

test('what is kept of a thing (what is pinned) is as it always was', () => {
  const tab: Tab = { kind: 'table', project: 'shop', name: 'open-questions' };
  assert.equal(keptAs(tab), '/table/shop/open-questions');
  assert.deepEqual(fromKept('/table/shop/open-questions'), tab);
  assert.deepEqual(fromKept('#/table/shop/open-questions'), tab);
});

test('a link to this factory’s page that names a thing opens that thing, and any other link nothing', () => {
  const here = { origin: 'http://127.0.0.1:7700' };
  assert.deepEqual(thingOfLink('http://127.0.0.1:7700/run/run_1m85442mg8j93j1c', here), { tab: { kind: 'run', id: 'run_1m85442mg8j93j1c' }, shown: {} });
  assert.deepEqual(thingOfLink('http://127.0.0.1:7700/shop/board', here), { tab: { kind: 'board', project: 'shop' }, shown: {} });
  assert.deepEqual(thingOfLink('http://127.0.0.1:7700/#/run/run_1', here), { tab: { kind: 'run', id: 'run_1' }, shown: {} }, 'a link written before');
  assert.deepEqual(thingOfLink('http://127.0.0.1:7700/shop/test-runs#newest', here, known), { tab: { kind: 'table', project: 'shop', name: 'test-runs' }, shown: { view: 'newest' } });
  assert.equal(thingOfLink('http://127.0.0.1:7760/run/run_1', here), null, 'another factory');
  assert.equal(thingOfLink('http://127.0.0.1:7700/library/shop/docs/', here), null, 'a library’s pages are the daemon’s');
  assert.equal(thingOfLink('http://127.0.0.1:7700/mockups/in/abc/', here), null, 'so are mockups');
  assert.equal(thingOfLink('http://127.0.0.1:7700/api/meta', here), null);
  assert.equal(thingOfLink('http://127.0.0.1:7700/#/nonsense', here), null);
  assert.equal(thingOfLink('https://example.com/run/run_1', here), null);
  assert.equal(thingOfLink('not an address', here), null);
});

test('no project’s path is the daemon’s or the client’s, and the words kept are the daemon’s list word for word', () => {
  for (const word of ['api', 'library', 'mockups', 'in', 'assets', 'lib', 'run', 'reference']) assert.ok(isReserved(word), word);
  assert.ok(!isReserved('shop'));
  assert.ok(isServedApart('/api/meta') && isServedApart('/library/shop/docs/') && isServedApart('/assets/app.js') && isServedApart('/in/key/topic'));
  assert.ok(!isServedApart('/shop/board') && !isServedApart('/run/run_1'));
  // The daemon refuses a project whose id is one (`kernel::paths::RESERVED`): the two lists are one.
  const rust = readFileSync(new URL('../../crates/kernel/src/paths.rs', import.meta.url), 'utf8');
  const list = rust.slice(rust.indexOf('pub const RESERVED'), rust.indexOf('];', rust.indexOf('pub const RESERVED')));
  const theirs = [...list.matchAll(/"([a-z]+)"/g)].map((found) => found[1]).sort();
  assert.deepEqual(theirs, Object.values(RESERVED).flat().slice().sort());
});

test('the client’s own files are served from no folder a project could be called', () => {
  // What the client's build writes at the top of what is served: each folder of it is a word kept.
  const build = readFileSync(new URL('../build.mjs', import.meta.url), 'utf8');
  const folders = new Set([...build.matchAll(/dist\/([a-z]+)(?=[/'])/g)].map((found) => found[1]!));
  for (const folder of folders) assert.ok((RESERVED.client as readonly string[]).includes(folder), folder);
});

test("a forum's topic is a tab by its id, and its address names it by its title whole, a `/` in it and all", () => {
  const tab: Tab = { kind: 'topic', id: '@-topics-3', title: 'Builds / Windows: why slow?' };
  assert.equal(tabKey(tab), 'topic:@-topics-3');
  assert.equal(tabKey({ kind: 'topic', id: '@-topics-3' }), tabKey(tab), 'the title is no part of what the tab is known by');
  const address = addressOf(tab);
  assert.equal(address, '/topic/Builds%20%2F%20Windows%3A%20why%20slow%3F');
  // Opened, the address names it by its title: which topic that is, is the page's to find among those it read.
  assert.deepEqual(tabAt(address!), { tab: { kind: 'topic', id: 'Builds / Windows: why slow?', title: 'Builds / Windows: why slow?' }, shown: {} });
  // One that says no title is named by its id, which opens it too.
  assert.equal(addressOf({ kind: 'topic', id: '@-topics-3' }), '/topic/%40-topics-3');
  assert.equal(tabAt('/topic'), null);
  assert.equal(tabAt('/topic/a/b/c'), null, 'never the pages between a page and its wiki');
  // `topic` is no project's: a project of that name would have its addresses written as before.
  assert.ok(isReserved('topic'));
  assert.equal(keptAs({ kind: 'topic', id: '@-topics-3' }), '/topic/%40-topics-3');
  assert.deepEqual(fromKept('#/topic/%40-topics-3'), { kind: 'topic', id: '@-topics-3' });
});

test("a page of a wiki is addressed by its wiki's title and its own, two parts whatever is between, and every address before still opens", () => {
  const page: Tab = { kind: 'topic', id: '@-topics-9', title: 'Rolling back', wiki: 'Handbook' };
  assert.equal(tabKey(page), 'topic:@-topics-9', 'what it is called is no part of what the tab is known by');
  const address = addressOf(page);
  assert.equal(address, '/topic/Handbook/Rolling%20back');
  assert.deepEqual(tabAt(address!), { tab: { kind: 'topic', id: 'Rolling back', title: 'Rolling back', wiki: 'Handbook' }, shown: {} });
  // A `/` in either title is one part of the path still.
  const slashed = addressOf({ kind: 'topic', id: '@-topics-9', title: 'A/B', wiki: 'Ops / Infra' });
  assert.equal(slashed, '/topic/Ops%20%2F%20Infra/A%2FB');
  assert.deepEqual(tabAt(slashed!), { tab: { kind: 'topic', id: 'A/B', title: 'A/B', wiki: 'Ops / Infra' }, shown: {} });
  // Its wiki said by its id where two wikis share a title: the page's own title after it still.
  assert.equal(addressOf({ kind: 'topic', id: '@-topics-9', title: 'Rolling back', wiki: '@-topics-1' }), '/topic/%40-topics-1/Rolling%20back');
  // With no title of its own to be said by, by its id alone, whatever its wiki: an id says which it is.
  assert.equal(addressOf({ kind: 'topic', id: '@-topics-9', wiki: 'Handbook' }), '/topic/%40-topics-9');
  // A wiki is its title alone, as a topic was.
  assert.equal(addressOf({ kind: 'topic', id: '@-topics-1', title: 'Handbook' }), '/topic/Handbook');
  // Every address there was opens: a title alone, an id, and one written before addresses were paths.
  assert.deepEqual(tabAt('/topic/Rolling%20back'), { tab: { kind: 'topic', id: 'Rolling back', title: 'Rolling back' }, shown: {} });
  assert.deepEqual(tabAt('/topic/%40-topics-9'), { tab: { kind: 'topic', id: '@-topics-9', title: '@-topics-9' }, shown: {} });
  assert.deepEqual(tabAt('/#/topic/%40-topics-9'), { tab: { kind: 'topic', id: '@-topics-9' }, shown: {} });
  // What is kept of it (a pin) is by its id, wherever it is moved.
  assert.equal(keptAs(page), '/topic/%40-topics-9');
});

test("a topic's address names its talk after the `#`, and a reply in it; the topic alone is its article", () => {
  const tab: Tab = { kind: 'topic', id: '@-topics-3', title: 'Which linker' };
  assert.equal(addressOf(tab), '/topic/Which%20linker');
  assert.equal(addressOf(tab, undefined, { part: 'talk' }), '/topic/Which%20linker#talk');
  assert.equal(addressOf(tab, undefined, { part: 'talk', at: 42 }), '/topic/Which%20linker#talk/r-42');
  const opened = { kind: 'topic', id: 'Which linker', title: 'Which linker' };
  assert.deepEqual(tabAt('/topic/Which%20linker'), { tab: opened, shown: {} });
  assert.deepEqual(tabAt('/topic/Which%20linker#talk'), { tab: opened, shown: { part: 'talk' } });
  assert.deepEqual(tabAt('/topic/Which%20linker#talk/r-42'), { tab: opened, shown: { part: 'talk', at: 42 } });
  const shownBy = (address: string) => {
    const asked = tabAt(address);
    return asked && 'shown' in asked ? asked.shown : null;
  };
  assert.deepEqual(shownBy('/topic/Handbook/Rolling%20back#talk/r-7'), { part: 'talk', at: 7 });
  // What it does not know after the `#` shows the article.
  for (const after of ['article', 'talk/42', 'talk/r-', 'replies']) assert.deepEqual(shownBy(`/topic/Which%20linker#${after}`), {}, after);
});

test("a topic's address names its history after the `#`, and a revision in it by its number", () => {
  const tab: Tab = { kind: 'topic', id: '@-topics-3', title: 'Which linker' };
  assert.equal(addressOf(tab, undefined, { part: 'history' }), '/topic/Which%20linker#history');
  assert.equal(addressOf(tab, undefined, { part: 'history', at: 3 }), '/topic/Which%20linker#history/3');
  const opened = { kind: 'topic', id: 'Which linker', title: 'Which linker' };
  assert.deepEqual(tabAt('/topic/Which%20linker#history'), { tab: opened, shown: { part: 'history' } });
  assert.deepEqual(tabAt('/topic/Which%20linker#history/3'), { tab: opened, shown: { part: 'history', at: 3 } });
  const shownBy = (address: string) => {
    const asked = tabAt(address);
    return asked && 'shown' in asked ? asked.shown : null;
  };
  assert.deepEqual(shownBy('/topic/Handbook/Rolling%20back#history/12'), { part: 'history', at: 12 });
  // A number that names no revision, or what is not a number, shows the article.
  for (const after of ['history/0', 'history/', 'history/r-3', 'history/3/4', 'histories']) assert.deepEqual(shownBy(`/topic/Which%20linker#${after}`), {}, after);
});

test("a topic's address names a heading of its article after `#s/`, by its slug, which no tab is taken for", () => {
  const tab: Tab = { kind: 'topic', id: '@-topics-3', title: 'Which linker' };
  assert.equal(addressOf(tab, undefined, { section: 'how-it-began' }), '/topic/Which%20linker#s/how-it-began');
  assert.equal(addressOf(tab, undefined, { section: 'über-straße' }), '/topic/Which%20linker#s/%C3%BCber-stra%C3%9Fe');
  // A heading called History is a heading, and the tab History is the tab.
  assert.equal(addressOf(tab, undefined, { section: 'history' }), '/topic/Which%20linker#s/history');
  const opened = { kind: 'topic', id: 'Which linker', title: 'Which linker' };
  assert.deepEqual(tabAt('/topic/Which%20linker#s/history'), { tab: opened, shown: { section: 'history' } });
  assert.deepEqual(tabAt('/topic/Which%20linker#history'), { tab: opened, shown: { part: 'history' } });
  const shownBy = (address: string) => {
    const asked = tabAt(address);
    return asked && 'shown' in asked ? asked.shown : null;
  };
  assert.deepEqual(shownBy('/topic/Handbook/Rolling%20back#s/%C3%BCber-stra%C3%9Fe'), { section: 'über-straße' });
  assert.deepEqual(shownBy('/topic/Handbook/Rolling%20back#s/notes-2'), { section: 'notes-2' });
  // What could be no heading's slug shows the article at its top.
  for (const after of ['s/', 's', 's/a/b', 's/-a', 's/A', 's/a%20b', 'section/a']) assert.deepEqual(shownBy(`/topic/Which%20linker#${after}`), {}, after);
  // Said of anything but a topic, it is no heading.
  assert.equal(addressOf({ kind: 'channel', id: 'general' }, undefined, { section: 'a' }), '/channels/general');
});

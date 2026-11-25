import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import { besideOf, between, everyPage, holding, landing, listedWikis, movesFor, pathOf, readTree, subtreeOf, UNTITLED, wikiQuestion, withPages } from './wikis.ts';
import type { Drawn } from './wikis.ts';
import type { Line } from './wikis.ts';

const table = JSON.parse(readFileSync(new URL('./wikicases.json', import.meta.url), 'utf8')) as {
  pages: Line[];
  read: [Line[] | null, Record<string, string[]>][];
  held: [string | null, string[]][];
  moved: [Line[] | null, { page: string; under: string | null; before: string | null }, unknown][];
  between: [number | null, number | null, number | null][];
};

const pages = (said: Line[] | null): Line[] => said ?? table.pages;

test('a page is read under what its row says, and one under nothing there or in a ring is a wiki, as the daemon has it', () => {
  assert.ok(table.read.length >= 5);
  for (const [lines, meant] of table.read) {
    const tree = readTree(pages(lines));
    for (const [id, path] of Object.entries(meant)) assert.deepEqual(pathOf(tree, id), path, `${id} of ${JSON.stringify(lines)}`);
  }
});

test('what a page holds is every page under it, each before its own, in the order they stand', () => {
  const tree = readTree(table.pages);
  for (const [page, meant] of table.held) {
    const came = page === null ? besideOf(tree, null).map((line) => line.id) : subtreeOf(tree, page);
    assert.deepEqual(came, meant, String(page));
  }
});

test('a page moved lands where its case says, or is refused by its word, as the daemon has it', () => {
  assert.ok(table.moved.length > 20);
  for (const [lines, asked, meant] of table.moved) {
    const came = landing(readTree(pages(lines)), asked.page, asked.under, asked.before);
    assert.deepEqual(came, meant, JSON.stringify(asked));
  }
});

test('a page lands between two, and where there is no room the level is numbered again', () => {
  for (const [before, after, meant] of table.between) assert.equal(between(before, after), meant, JSON.stringify([before, after]));
});

// ---- What the Topics panel draws ----

type Page = Line & { status: string };
const statused = (closed: string[] = []): Page[] => table.pages.map((line) => ({ ...line, status: closed.includes(line.id) ? 'closed' : 'open' }));
const shape = (wikis: Drawn<Page>[]): unknown => wikis.map((one) => (one.pages.length ? { [one.line.id]: shape(one.pages) } : one.line.id));

test("the panel draws each wiki of its workspace in the order it stands, with every page under it to any depth", () => {
  const { listed, closed } = listedWikis(statused(), (line) => line.space, (line) => line.space === 'w');
  assert.deepEqual(shape(listed), [{ h: ['g', { d: ['s', 'r'] }] }, 'p']);
  assert.deepEqual(closed, []);
  // Another workspace's wikis, and the pages under them, are not drawn there.
  assert.deepEqual(shape(listedWikis(statused(), (line) => line.space, (line) => line.space === 'other').listed), [{ o: ['x'] }]);
});

test('a closed wiki stands apart with its pages under it, and a closed page stays where it is', () => {
  const { listed, closed } = listedWikis(statused(['h', 'r']), (line) => line.space, (line) => line.space === 'w');
  assert.deepEqual(shape(listed), ['p']);
  assert.deepEqual(shape(closed), [{ h: ['g', { d: ['s', 'r'] }] }]);
  const keptIn = listedWikis(statused(['r']), (line) => line.space, (line) => line.space === 'w');
  assert.deepEqual(shape(keptIn.listed), [{ h: ['g', { d: ['s', 'r'] }] }, 'p'], 'a closed page under an open one is not taken out of its wiki');
});

test('Expand all unfolds every page that holds pages, and nothing that holds none', () => {
  const { listed } = listedWikis(statused(), (line) => line.space, () => true);
  assert.deepEqual(holding(listed), ['h', 'd', 'o']);
  assert.deepEqual(everyPage(listed), ['h', 'g', 'd', 's', 'r', 'p', 'o', 'x']);
});

test('Move to… offers the top and every page of its workspace at its depth, never the page itself or what is under it, where it is marked', () => {
  const tree = readTree(table.pages);
  assert.deepEqual(movesFor(tree, 'd'), [
    { kind: 'top', here: false },
    { kind: 'page', id: 'h', depth: 0, here: true },
    { kind: 'page', id: 'g', depth: 1, here: false },
    { kind: 'page', id: 'p', depth: 0, here: false },
  ]);
  assert.deepEqual(movesFor(tree, 'p'), [
    { kind: 'top', here: true },
    { kind: 'page', id: 'h', depth: 0, here: false },
    { kind: 'page', id: 'g', depth: 1, here: false },
    { kind: 'page', id: 'd', depth: 1, here: false },
    { kind: 'page', id: 's', depth: 2, here: false },
    { kind: 'page', id: 'r', depth: 2, here: false },
  ]);
  assert.deepEqual(movesFor(tree, 'x'), [{ kind: 'top', here: false }, { kind: 'page', id: 'o', depth: 0, here: true }], 'another workspace offers its own wikis alone');
  assert.deepEqual(movesFor(tree, 'gone'), []);
  // Every place offered is one the daemon would take, at the end of it.
  for (const page of ['h', 'g', 'd', 's', 'r', 'p', 'o', 'x']) {
    for (const move of movesFor(tree, page)) {
      if (move.here) continue;
      const came = landing(tree, page, move.kind === 'top' ? null : move.id, null);
      assert.ok(!('refused' in came), `${page} to ${JSON.stringify(move)}: ${JSON.stringify(came)}`);
    }
  }
});

test('a page made is Untitled, and a wiki moved to another workspace is said with how many pages go with it', () => {
  assert.equal(UNTITLED, 'Untitled');
  assert.equal(withPages('Handbook', 0), 'Handbook');
  assert.equal(withPages('Handbook', 1), 'Handbook and its page');
  assert.equal(withPages('Handbook', 14), 'Handbook and its 14 pages');
  assert.deepEqual(wikiQuestion('Handbook', 14, 'Back office').title, 'Move Handbook to Back office?');
  assert.match(wikiQuestion('Handbook', 14, 'Back office').says, /^Handbook and its 14 pages are listed in Back office from now on/);
  assert.match(wikiQuestion('Notes', 0, 'Back office').says, /^Notes is listed in Back office from now on, for everybody\. Nothing else of it changes/);
  assert.match(wikiQuestion('Handbook', 2, 'Back office', 'Shop').says, /for everybody\. They are Shop's no longer\. Nothing else of them changes/);
});

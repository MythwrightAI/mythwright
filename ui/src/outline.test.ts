import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import { apply, checked, DEEPEST, folderNamed, foldersLaid, foldersOf, keysOf, laidOut, landing, LONGEST_NAME, namesIn, placesFor, seenOf, shownAs, shownName, stepped, usualOf, whatIs } from './outline.ts';
import type { Laid, Listing, Outline } from './outline.ts';
import { outlineFold, outlineFoldsGone, projectFold, resourcesFolds } from './folds.ts';
import { ofProject, projectItem, narrowed } from './panelsought.ts';

const table = JSON.parse(readFileSync(new URL('./outlinecases.json', import.meta.url), 'utf8')) as {
  longest: number;
  deepest: number;
  checked: [unknown, string | null][];
  applied: [unknown, unknown, string[], Record<string, unknown>][];
};

const kept = (said: unknown): Outline => {
  const read = checked(said);
  if ('wrong' in read) throw new Error(read.wrong);
  return read.outline;
};

test('an outline is kept or refused in its sentence, case for case with the daemon', () => {
  assert.equal(table.longest, LONGEST_NAME);
  assert.equal(table.deepest, DEEPEST);
  for (const [said, refused] of table.checked) {
    const read = checked(said);
    assert.equal('wrong' in read ? read.wrong : null, refused, JSON.stringify(said));
  }
});

test('each change comes to what its case says, or is refused in its sentence, as the daemon has it', () => {
  for (const [outline, op, seen, meant] of table.applied) {
    const came = apply(outline === null ? null : kept(outline), op, seen);
    const said = 'wrong' in came ? { refused: came.wrong } : { items: came.outline?.items ?? null, changed: came.changed, done: came.done };
    assert.deepEqual(JSON.parse(JSON.stringify(said)), meant, JSON.stringify([outline, op, seen]));
  }
});

const shop: Listing = { board: true, tables: ['test-runs', 'deployments', 'notes'], libraries: ['docs'], mockups: [], environments: false, pipelines: false, blueprints: true, git: false, repo: true };

test('a project nobody arranged is drawn as it always was, line for line', () => {
  assert.deepEqual(keysOf(shop), ['board', 'table:test-runs', 'table:deployments', 'table:notes', 'library:docs', 'blueprints', 'repo']);
  assert.deepEqual(laidOut(keysOf(shop), null), keysOf(shop).map((thing) => ({ thing })));
  // Its environments and its pipelines, where it has them, between what a blueprint keeps for it and the blueprints it uses.
  assert.deepEqual(keysOf({ ...shop, environments: true, pipelines: true, git: true }), ['board', 'table:test-runs', 'table:deployments', 'table:notes', 'library:docs', 'environments', 'pipelines', 'blueprints', 'git', 'repo']);
});

test('an arranged project is drawn as its outline says, and what nobody placed after it in the usual order', () => {
  const outline = kept({
    items: [
      { thing: 'board' },
      { folder: 'f1', name: 'Insights', items: [{ thing: 'table:test-runs' }, { thing: 'table:deployments' }, { thing: 'table:gone' }] },
      { folder: 'f2', name: 'Empty' },
      { thing: 'repo' },
    ],
  });
  assert.deepEqual(laidOut(keysOf(shop), outline), [
    { thing: 'board' },
    { folder: 'f1', name: 'Insights', items: [{ thing: 'table:test-runs' }, { thing: 'table:deployments' }] },
    { folder: 'f2', name: 'Empty', items: [] },
    { thing: 'repo' },
    { thing: 'table:notes' },
    { thing: 'library:docs' },
    { thing: 'blueprints' },
  ]);
  // A thing the page does not list (one kept from the reader, one gone) is not drawn wherever it is placed.
  const less = laidOut(['board', 'repo'], outline);
  assert.deepEqual(less, [{ thing: 'board' }, { folder: 'f1', name: 'Insights', items: [] }, { folder: 'f2', name: 'Empty', items: [] }, { thing: 'repo' }]);
});

test('a folder of an outline folds as everything of the panel does, is in Expand all, and its fold goes when it does', () => {
  assert.equal(outlineFold('shop', 'f1'), 'project:shop:outline:f1');
  const folds = resourcesFolds([{ id: 'shop', blueprints: false, files: null, folders: ['f1', 'f2'] }], []);
  assert.ok(folds.includes(projectFold('shop')) && folds.includes('project:shop:outline:f2'));
  const turned = ['project:shop:outline:f1', 'project:shop:outline:f9', 'project:other:outline:f9', 'project:shop:tree'];
  assert.deepEqual(outlineFoldsGone(turned, 'shop', ['f1']), ['project:shop:outline:f9']);
  assert.deepEqual(foldersOf(kept({ items: [{ folder: 'f1', name: 'A', items: [{ folder: 'f3', name: 'B' }] }] }).items), ['f1', 'f3']);
});

test('the panel box finds a thing by the name of the folder it is in, and holds the folder open for it', () => {
  const outline = kept({ items: [{ folder: 'f1', name: 'Insights', items: [{ thing: 'table:test-runs' }] }] });
  const item = projectItem({
    id: 'shop',
    name: 'Shop',
    board: true,
    tables: [{ name: 'test-runs', label: 'Test runs' }],
    libraries: [],
    mockups: [],
    blueprints: [],
    files: null,
    outline,
  });
  const found = narrowed([item], 'insights runs');
  assert.ok(found);
  assert.ok(found.shown.has(ofProject.table('shop', 'test-runs')));
  assert.ok(found.held.has(ofProject.folder('shop', 'f1')) && found.held.has(ofProject.project('shop')));
  assert.ok(!found.shown.has(ofProject.board('shop')));
  // The folder found by its own name is shown whole, folded as it was left.
  const whole = narrowed([item], 'insights');
  assert.ok(whole && whole.shown.has(ofProject.folder('shop', 'f1')) && !whole.held.has(ofProject.folder('shop', 'f1')));
});

test('a thing somebody named is drawn, found and told by that name, and one nobody named as it always was', () => {
  const outline = kept({ items: [{ thing: 'board', name: 'Roadmap' }, { folder: 'f1', name: 'Docs', items: [{ thing: 'library:docs', name: 'Handbook' }] }] });
  assert.deepEqual(laidOut(keysOf(shop), outline).slice(0, 2), [
    { thing: 'board', name: 'Roadmap' },
    { folder: 'f1', name: 'Docs', items: [{ thing: 'library:docs', name: 'Handbook' }] },
  ]);
  assert.equal(shownAs(outline, 'board'), 'Roadmap');
  assert.equal(shownAs(outline, 'library:docs'), 'Handbook');
  assert.equal(shownAs(outline, 'repo'), null);
  assert.equal(shownAs(null, 'board'), null);
  // What each always said, and what it is, said beside a name so that it can still be told.
  assert.deepEqual(['board', 'library:docs', 'mockups:pages', 'environments', 'pipelines', 'blueprints', 'git', 'repo'].map(usualOf), ['Board', 'Docs', 'Pages', 'Environments', 'Pipelines', 'Blueprints', 'Git', 'Repo']);
  assert.deepEqual(['board', 'library:docs', 'mockups:pages', 'environments', 'git'].map(whatIs), ['Board · table board', 'Library · table docs', 'Mockups · table pages', 'Environments', 'Git']);
  // What is typed in a rename: trimmed, empty brings back the usual, refused where too long or more than a line.
  assert.deepEqual(shownName('  Roadmap '), { name: 'Roadmap' });
  assert.deepEqual(shownName('   '), { name: null });
  assert.ok('wrong' in shownName('a'.repeat(61)) && 'wrong' in shownName('Road\nmap'));
  const item = projectItem({ id: 'shop', name: 'Shop', board: true, tables: [], libraries: [], mockups: [], blueprints: [], files: null, outline });
  for (const typed of ['roadmap', 'board']) {
    const found = narrowed([item], typed);
    assert.ok(found && found.shown.has(ofProject.board('shop')), typed);
  }
});

test('a change is sent with what the page draws at the top that the outline does not name, in the order drawn', () => {
  const listed = keysOf(shop);
  assert.deepEqual(seenOf(listed, null), listed, 'with none, every line drawn');
  const outline = kept({ items: [{ folder: 'f1', name: 'Insights', items: [{ thing: 'table:test-runs' }] }, { thing: 'repo' }] });
  assert.deepEqual(seenOf(listed, outline), ['board', 'table:deployments', 'table:notes', 'library:docs', 'blueprints']);
});

test('Move to… offers the top and each folder at its depth, where something is marked, and a folder not itself or what is in it', () => {
  const outline = kept({
    items: [
      { folder: 'f1', name: 'Insights', items: [{ thing: 'table:test-runs' }, { folder: 'f2', name: 'Runs', items: [{ thing: 'table:deployments' }] }] },
      { folder: 'f3', name: 'Notes' },
      { thing: 'board' },
    ],
  });
  const laid = laidOut(keysOf(shop), outline);
  const lines = (node: string) => placesFor(laid, node).map((place) => (place.kind === 'top' ? `top${place.here ? ' ✓' : ''}` : `${'  '.repeat(place.depth)}${place.name}${place.here ? ' ✓' : ''}`));
  assert.deepEqual(lines('table:deployments'), ['top', 'Insights', '  Runs ✓', 'Notes']);
  assert.deepEqual(lines('board'), ['top ✓', 'Insights', '  Runs', 'Notes']);
  // What nobody placed is at the top.
  assert.deepEqual(lines('library:docs'), ['top ✓', 'Insights', '  Runs', 'Notes']);
  // A folder is not offered itself nor what is in it.
  assert.deepEqual(lines('f1'), ['top ✓', 'Notes']);
  assert.deepEqual(lines('f2'), ['top', 'Insights ✓', 'Notes']);
  assert.deepEqual(foldersLaid(laid), [
    { id: 'f1', name: 'Insights' },
    { id: 'f2', name: 'Runs' },
    { id: 'f3', name: 'Notes' },
  ]);
  // With none, there is the top alone.
  assert.deepEqual(placesFor(laidOut(keysOf(shop), null), 'board'), [{ kind: 'top', here: true }]);
});

test('a folder is called something of its own beside the folders where it is, refused before anything is asked', () => {
  const outline = kept({ items: [{ folder: 'f1', name: 'Insights', items: [{ folder: 'f2', name: 'Runs' }] }, { folder: 'f3', name: 'Notes' }] });
  const laid = laidOut([], outline);
  assert.deepEqual(namesIn(laid, null), ['Insights', 'Notes']);
  assert.deepEqual(namesIn(laid, 'f1'), ['Runs']);
  assert.deepEqual(namesIn(laid, 'f9'), []);
  assert.deepEqual(folderNamed('  Runs ', namesIn(laid, null)), { name: 'Runs' });
  assert.deepEqual(folderNamed('runs', namesIn(laid, 'f1')), { wrong: 'Another folder here is called runs: call this one something of its own.' });
  assert.deepEqual(folderNamed(' ', []), { wrong: 'A folder is called something: say what this one is called.' });
  assert.ok('wrong' in folderNamed('x'.repeat(LONGEST_NAME + 1), []));
});

test("what is carried in a tree is offered to a project's outline under the names it always had", () => {
  const drawn: Laid[] = [{ thing: 'board' }, { folder: 'f1', name: 'Insights', items: [{ thing: 'table:a' }] }];
  assert.deepEqual(stepped(drawn, 'board', 'in'), null);
  assert.deepEqual(stepped(drawn, 'f1', 'up'), { into: null, before: 'board' });
  const at = (node: string, into: string | null, top: number) => ({ node, into, folder: false, open: false, top, bottom: top + 20, end: top + 20, left: 10 });
  assert.equal(landing([at('board', null, 0), at('repo', null, 20)], 'board', 10, 5)?.stays, true);
});

import assert from 'node:assert/strict';
import { test } from 'node:test';
import { UNGROUPED, bandCalled, bands, columnName, groupable, heldTo, holdable, holdableBy, holding, holds, ordered, shown, standing } from './ordered.ts';
import type { Waits } from './ordered.ts';
import { goes, layouts, offered } from './table.ts';
import type { Field, Named, Row, View } from './table.ts';

const waits = (id: string, after: string[] = [], done = false): Waits => ({ id, after, done });
const field = (name: string, kind: string, of: string[] = []): Field => ({ name, kind, of, required: false });
const row = (number: number, data: Record<string, unknown>, id = `r-${number}`): Row => ({ id, number, data, position: number, created: 0, updated: 0 });

// What a table comes to, by what its records wait on: each case a table, the columns it stands in, and what is said beside.
const CASES: { of: string; rows: Waits[]; columns: string[][]; done?: string[]; ring?: string[]; turning?: string[]; missing?: Record<string, string[]> }[] = [
  { of: 'nothing', rows: [], columns: [] },
  { of: 'none waiting: all side by side', rows: [waits('a'), waits('b'), waits('c')], columns: [['a', 'b', 'c']] },
  { of: 'a chain: one a column', rows: [waits('c', ['b']), waits('a'), waits('b', ['a'])], columns: [['a'], ['b'], ['c']] },
  { of: 'a diamond: the two in the middle side by side', rows: [waits('a'), waits('b', ['a']), waits('c', ['a']), waits('d', ['b', 'c'])], columns: [['a'], ['b', 'c'], ['d']] },
  { of: 'two apart: each chain from the first column', rows: [waits('a'), waits('b', ['a']), waits('x'), waits('y', ['x'])], columns: [['a', 'x'], ['b', 'y']] },
  { of: 'one that waits on two of different depth stands after the deeper', rows: [waits('a'), waits('b', ['a']), waits('c', ['a', 'b'])], columns: [['a'], ['b'], ['c']] },
  { of: 'one waiting on something gone is placed by the rest, and says so', rows: [waits('a'), waits('b', ['a', 'z']), waits('c', ['q'])], columns: [['a', 'c'], ['b']], missing: { b: ['z'], c: ['q'] } },
  { of: 'something done in the middle holds nothing up', rows: [waits('a', [], true), waits('b', ['a']), waits('c', ['b'])], columns: [['b'], ['c']], done: ['a'] },
  { of: 'what is done is done, whatever it waits on', rows: [waits('a'), waits('b', ['a'], true), waits('c', ['b', 'a'])], columns: [['a'], ['c']], done: ['b'] },
  { of: 'everything done', rows: [waits('a', [], true), waits('b', ['a'], true)], columns: [], done: ['a', 'b'] },
  { of: 'a ring of two', rows: [waits('a', ['b']), waits('b', ['a']), waits('c')], columns: [['c']], ring: ['a', 'b'], turning: ['a', 'b'] },
  { of: 'one that waits on itself', rows: [waits('a', ['a']), waits('b')], columns: [['b']], ring: ['a'], turning: ['a'] },
  { of: 'a ring of three, and one behind it that is in none', rows: [waits('a', ['c']), waits('b', ['a']), waits('c', ['b']), waits('d', ['c']), waits('e')], columns: [['e']], ring: ['a', 'b', 'c', 'd'], turning: ['a', 'b', 'c'] },
  { of: 'a ring one of which is done is no ring', rows: [waits('a', ['b'], true), waits('b', ['a'])], columns: [['b']], done: ['a'] },
  { of: 'the same one named twice is waited on once', rows: [waits('a'), waits('b', ['a', 'a'])], columns: [['a'], ['b']] },
];

for (const one of CASES) {
  test(`records by what comes first: ${one.of}`, () => {
    const found = ordered(one.rows);
    assert.deepEqual(found.columns, one.columns);
    assert.deepEqual(found.done, one.done ?? []);
    assert.deepEqual(found.ring, one.ring ?? []);
    assert.deepEqual(found.turning, one.turning ?? []);
    for (const row of one.rows) assert.deepEqual(found.missing[row.id], one.missing?.[row.id] ?? [], `what ${row.id} waits on that is not there`);
    // Every record is somewhere, once.
    assert.deepEqual([...found.done, ...found.columns.flat(), ...found.ring].sort(), one.rows.map((row) => row.id).sort());
    // And nothing stands beside, or before, what it still waits on.
    const at = new Map(found.columns.flatMap((ids, nth) => ids.map((id): [string, number] => [id, nth])));
    for (const [id, nth] of at) for (const before of found.after[id]!) assert.ok(found.done.includes(before) || (at.get(before) ?? Infinity) < nth, `${id} after ${before}`);
  });
}

test('what a record waits on and what waits on it are each said, done or not', () => {
  const found = ordered([waits('a', [], true), waits('b', ['a']), waits('c', ['a', 'b'])]);
  assert.deepEqual(found.after, { a: [], b: ['a'], c: ['a', 'b'] });
  assert.deepEqual(found.before, { a: ['b', 'c'], b: ['c'], c: [] });
});

test('a thousand in a chain are put in order without going round for ever', () => {
  const chain = Array.from({ length: 1000 }, (_, nth) => waits(`n${nth}`, nth === 0 ? [] : [`n${nth - 1}`]));
  assert.equal(ordered(chain).columns.length, 1000);
});

test('a record is done at the last of what can be chosen, and begun anywhere past the first', () => {
  const status = field('status', 'choice', ['backlog', 'ready', 'doing', 'done']);
  assert.deepEqual(standing(status, 'backlog'), { done: false, begun: false, at: 'backlog' });
  assert.deepEqual(standing(status, 'ready'), { done: false, begun: true, at: 'ready' });
  assert.deepEqual(standing(status, 'doing'), { done: false, begun: true, at: 'doing' });
  assert.deepEqual(standing(status, 'done'), { done: true, begun: false, at: 'done' });
  assert.deepEqual(standing(status, null), { done: false, begun: false, at: '' });
  assert.deepEqual(standing(status, 'elsewhere'), { done: false, begun: false, at: 'elsewhere' });
  assert.equal(standing(undefined, 'done').done, false, 'a table with no such choice has nothing that is done');
  assert.equal(standing(field('only', 'choice', ['one']), 'one').done, false, 'a choice of one thing says nothing of being done');
});

test('columns are headed by their place', () => {
  assert.deepEqual([0, 1, 2, 3, 4].map(columnName), ['Can begin now', 'Then', 'After that', 'Step 4', 'Step 5']);
});

const features: Named = {
  features: {
    fields: [field('title', 'text')],
    rows: [row(1, { title: 'Checkout' }, 'shop-features-1'), row(2, { title: 'Search' }, 'shop-features-2')],
  },
};

test('records are gathered by the record a field names, called what that one is, with those that name nothing last', () => {
  const part = field('part_of', 'record', ['features']);
  const rows = [row(1, { part_of: 'shop-features-2' }), row(2, {}), row(3, { part_of: 'shop-features-1' }), row(4, { part_of: 'shop-features-2' }), row(5, { part_of: '' }), row(6, { part_of: 'shop-features-9' })];
  assert.deepEqual(
    bands(rows, part, features).map((band) => [band.name, band.gone, band.rows.map((one) => one.id)]),
    [
      ['Checkout', false, ['r-3']],
      ['Search', false, ['r-1', 'r-4']],
      ['shop-features-9', true, ['r-6']],
      [UNGROUPED, false, ['r-2', 'r-5']],
    ],
  );
});

test('a text that is the id of a record somebody looked up is called what that one is, and any other what it says', () => {
  const part = field('part_of', 'text');
  const rows = [row(1, { part_of: 'shop-features-2' }), row(2, { part_of: 'loose words' }), row(3, { part_of: 'shop-features-2' })];
  assert.deepEqual(
    bands(rows, part, {}, { 'shop-features-2': 'Search' }).map((band) => [band.key, band.name, band.rows.length]),
    [
      ['loose words', 'loose words', 1],
      ['shop-features-2', 'Search', 2],
    ],
  );
  assert.deepEqual(bands([], part, {}), [], 'nothing to gather is no band at all');
  assert.deepEqual(bands([row(1, {})], part, {}).map((band) => band.name), [UNGROUPED]);
});

test('a choice comes in the order of what can be chosen, and who by their name', () => {
  const size = field('size', 'choice', ['small', 'large']);
  const sized = [row(1, { size: 'large' }), row(2, { size: 'small' }), row(3, { size: 'odd' })];
  assert.deepEqual(bands(sized, size, {}).map((band) => band.name), ['small', 'large', 'odd']);
  const who = field('assignee', 'actor');
  assert.deepEqual(bands([row(1, { assignee: 'agent:worker-1' }), row(2, { assignee: 'user:ada' })], who, {}).map((band) => band.name), ['worker-1', 'ada']);
});

const card = [field('after', 'list'), field('assignee', 'actor'), field('labels', 'list'), field('part_of', 'record', ['features']), field('status', 'choice', ['backlog', 'doing', 'done']), field('title', 'text'), field('size', 'choice', ['s', 'l'])];

test('a view is offered to be gathered by what names a record of another table, a choice its columns do not come from, or who', () => {
  assert.deepEqual(groupable(card, 'board', 'status').map((one) => one.name), ['assignee', 'part_of', 'size']);
  assert.deepEqual(groupable([field('title', 'text'), field('parent', 'record', ['notes'])], 'notes', null), [], 'a table that names itself has nothing to be gathered by');
  assert.deepEqual(groupable(card, 'board', 'status', 'title').map((one) => one.name), ['assignee', 'part_of', 'title', 'size'], 'and what it is gathered by now, whatever kind that is');
  // A text is offered where two records share a word of it: then it names something.
  const kept = [...card, field('piece', 'text'), { ...field('note', 'text'), hidden: true }];
  const shared = [row(1, { title: 'One', piece: 'f-1', note: 'x' }), row(2, { title: 'Two', piece: 'f-1', note: 'x' }), row(3, { title: 'Two', piece: '' }), row(4, { piece: '' })];
  assert.deepEqual(groupable(kept, 'board', 'status', null, shared).map((one) => one.name), ['assignee', 'part_of', 'title', 'size', 'piece'], 'two called alike share their title; what is left out of a view is not offered');
  assert.deepEqual(groupable(kept, 'board', 'status', null, shared.slice(0, 1)).map((one) => one.name), ['assignee', 'part_of', 'size']);
});

test('an order is offered where a field names records, or a view of the table already is one', () => {
  const kinds = (fields: Field[], views?: { kind: string }[]) => layouts(fields, views, true).map((one) => one.kind);
  assert.ok(!kinds(card, []).includes('order'));
  assert.ok(kinds(card, [{ kind: 'board' }, { kind: 'order' }]).includes('order'));
  assert.deepEqual(kinds(card, [{ kind: 'order' }]), ['table', 'board', 'list', 'gallery', 'calendar', 'order'], 'offered last');
  assert.ok(kinds([field('title', 'text'), field('before', 'records', ['steps'])]).includes('order'));
});

test('an order is not offered for now, as a view or as a way to lay one out, unless the browser was told so', () => {
  const views = [{ kind: 'board' }, { kind: 'order' }, { kind: 'table' }];
  // With nothing said (and where there is no browser to have been told), it is not.
  assert.deepEqual(offered(views).map((one) => one.kind), ['board', 'table']);
  assert.deepEqual(offered(views, true), views, 'told so, every view is offered as it was');
  assert.deepEqual(offered([{ kind: 'order' }]), [], 'a table whose one view is an order is shown as one with none');
  assert.ok(!layouts(card, views).map((one) => one.kind).includes('order'));
  assert.ok(!layouts([field('title', 'text'), field('before', 'records', ['steps'])]).map((one) => one.kind).includes('order'));
});

test('what a view is saved with goes with it where its kind still has a use for it', () => {
  const view = (kind: string, more: Partial<View> = {}): View => ({ name: 'v', kind, by: null, ...more });
  const order = view('order', { by: 'after', done: 'status', group: 'part_of' });
  // Nothing was said of its columns, so nothing of them goes anywhere.
  const columns = { columns: null, freezes: null, figures: null, widths: null };
  assert.deepEqual(goes('order', card, order), { by: 'after', group: 'part_of', done: 'status', only: null, ...columns });
  // Laid out as a board it keeps its bands, and goes by a choice; as a table its rows are gathered by the same; a list has neither.
  assert.deepEqual(goes('board', card, order), { by: 'status', group: 'part_of', done: null, only: null, ...columns });
  assert.deepEqual(goes('table', card, order), { by: null, group: 'part_of', done: null, only: null, ...columns });
  assert.deepEqual(goes('list', card, order), { by: null, group: null, done: null, only: null, ...columns });
  // A table made an order takes what another order of the table goes by.
  assert.deepEqual(goes('order', card, view('table'), [view('board', { by: 'status' }), order]), { by: 'after', group: null, done: 'status', only: null, ...columns });
  // What names a field that is gone, or of another kind, is not carried.
  assert.deepEqual(goes('order', card, view('order', { by: 'title', done: 'labels', group: 'nowhere' })), { by: null, group: null, done: null, only: null, ...columns });
});

// A board whose cards are marked with labels: one holds two, some hold none.
const labels = field('labels', 'list');
const marked = [
  row(1, { title: 'Floor', labels: ['landed'], status: 'done' }),
  row(2, { title: 'Shelves', labels: ['before-preview', 'in-progress'], status: 'doing' }),
  row(3, { title: 'Doors', labels: ['before-preview'], status: 'backlog' }),
  row(4, { title: 'Lights', labels: [], status: 'backlog' }),
  row(5, { title: 'Signs', status: 'backlog' }),
];
const ids = (rows: Row[]) => rows.map((one) => one.id);

test('a record holds the one thing a field of one says, each of what a field of several holds, and nothing where it says nothing', () => {
  assert.deepEqual(holds(marked[1]!, labels), ['before-preview', 'in-progress']);
  assert.deepEqual(holds(marked[3]!, labels), []);
  assert.deepEqual(holds(marked[4]!, labels), []);
  assert.deepEqual(holds(row(9, { labels: ['a', ' a ', '', 'b'] }), labels), ['a', 'b'], 'each thing once, and no empty one');
  assert.deepEqual(holds(marked[0]!, field('status', 'choice', ['backlog', 'doing', 'done'])), ['done']);
  assert.deepEqual(holds(row(9, { spans: ['y-1', 'y-2'] }), field('spans', 'records', ['years'])), ['y-1', 'y-2']);
});

test('records are gathered by what a list holds: a band for each thing, by name, one that holds two in both, and those that hold none last', () => {
  assert.deepEqual(
    bands(marked, labels, {}).map((band) => [band.name, ids(band.rows)]),
    [
      ['before-preview', ['r-2', 'r-3']],
      ['in-progress', ['r-2']],
      ['landed', ['r-1']],
      [UNGROUPED, ['r-4', 'r-5']],
    ],
  );
  // The bands stay where they were whichever record comes first.
  assert.deepEqual(bands([...marked].reverse(), labels, {}).map((band) => band.name), ['before-preview', 'in-progress', 'landed', UNGROUPED]);
  // Several records named are called what each is, and one that is gone says so.
  const spans = field('spans', 'records', ['features']);
  const spanning = [row(1, { spans: ['shop-features-2', 'shop-features-1'] }), row(2, { spans: ['shop-features-9'] })];
  assert.deepEqual(
    bands(spanning, spans, features).map((band) => [band.name, band.gone, ids(band.rows)]),
    [
      ['Checkout', false, ['r-1']],
      ['Search', false, ['r-1']],
      ['shop-features-9', true, ['r-2']],
    ],
  );
});

test('a list is offered to gather by where two records share a thing of it', () => {
  const fields = [field('title', 'text'), labels, field('after', 'list'), { ...field('again', 'list'), hidden: true }];
  assert.deepEqual(groupable(fields, 'board', null, null, marked).map((one) => one.name), ['labels']);
  assert.deepEqual(groupable(fields, 'board', null, null, marked.slice(0, 2)).map((one) => one.name), [], 'no two of these share a label');
});

// What a view held to something shows: each case what it is held to, and the records left.
const status = field('status', 'choice', ['backlog', 'doing', 'done']);
const table = [field('title', 'text'), labels, status];
const HELD: { of: string; only: Record<string, string[]> | null; shows: string[] }[] = [
  { of: 'nothing: every record', only: null, shows: ['r-1', 'r-2', 'r-3', 'r-4', 'r-5'] },
  { of: 'an empty map: every record', only: {}, shows: ['r-1', 'r-2', 'r-3', 'r-4', 'r-5'] },
  { of: 'one label', only: { labels: ['before-preview'] }, shows: ['r-2', 'r-3'] },
  { of: 'two labels: those that hold either', only: { labels: ['landed', 'in-progress'] }, shows: ['r-1', 'r-2'] },
  { of: 'a label nothing holds: none', only: { labels: ['someday'] }, shows: [] },
  { of: 'a choice', only: { status: ['backlog'] }, shows: ['r-3', 'r-4', 'r-5'] },
  { of: 'two fields: those that are held by both', only: { labels: ['before-preview'], status: ['backlog'] }, shows: ['r-3'] },
  { of: 'a field the table no longer has holds nothing back', only: { size: ['large'], labels: ['landed'] }, shows: ['r-1'] },
  { of: 'a field with nothing said holds nothing back', only: { labels: [] }, shows: ['r-1', 'r-2', 'r-3', 'r-4', 'r-5'] },
];
for (const one of HELD) {
  test(`a view held to ${one.of}`, () => assert.deepEqual(ids(shown(marked, one.only, table)), one.shows));
}

test('a view is offered to be held to what its lists hold and what its choices offer, less the choice its columns come from', () => {
  const offered = (columnsBy: string | null, now?: Record<string, string[]>) => holdable(table, columnsBy, marked, now).map((one) => [one.field.name, one.things]);
  assert.deepEqual(offered('status'), [['labels', ['before-preview', 'in-progress', 'landed']]]);
  assert.deepEqual(offered(null), [
    ['labels', ['before-preview', 'in-progress', 'landed']],
    ['status', ['backlog', 'doing', 'done']],
  ]);
  // What it is held to now is offered still, so that it can be turned off, though no record holds it any more.
  assert.deepEqual(offered('status', { labels: ['someday'] }), [['labels', ['before-preview', 'in-progress', 'landed', 'someday']]]);
  assert.deepEqual(holdable([field('title', 'text'), labels], null, [row(1, { title: 'One' })]), [], 'a list nothing is held in offers nothing');
});

test('a column offers its own things to hold a view to: what a choice offers, what two rows share of anything else that names something, and nothing of a number or a day', () => {
  const things = (one: Field, now?: Record<string, string[]>) => holdableBy(one, marked, now)?.things ?? null;
  const status = table.find((one) => one.name === 'status')!;
  assert.deepEqual(things(status), ['backlog', 'doing', 'done']);
  assert.deepEqual(things(labels), ['before-preview', 'in-progress', 'landed']);
  // A text two rows share a word of names something; one in which every row says something of its own does not.
  const shelf = field('shelf', 'text');
  const shelved = [row(1, { title: 'One', shelf: 'S2' }), row(2, { title: 'Two', shelf: 'S10' }), row(3, { title: 'Three', shelf: 'S2' })];
  assert.deepEqual(holdableBy(shelf, shelved)?.things, ['S2', 'S10']);
  assert.deepEqual(holdableBy(field('title', 'text'), shelved)?.things, []);
  // What it is held to now is offered still, though no row says it or shares it.
  assert.deepEqual(holdableBy(field('title', 'text'), shelved, { title: ['Gone'] })?.things, ['Gone', 'One', 'Three', 'Two']);
  assert.deepEqual(things(labels, { labels: ['someday'] }), ['before-preview', 'in-progress', 'landed', 'someday']);
  // A view is never held by a number, a day or a tick.
  for (const kind of ['number', 'date', 'flag']) assert.equal(holdableBy(field('size', kind), shelved), null);
});

test('a thing turned on or off holds a view to more or less, and to nothing when none is left', () => {
  assert.deepEqual(holding(null, 'labels', 'landed', true), { labels: ['landed'] });
  assert.deepEqual(holding({ labels: ['landed'] }, 'labels', 'soon', true), { labels: ['landed', 'soon'] });
  assert.deepEqual(holding({ labels: ['landed'] }, 'labels', 'landed', true), { labels: ['landed'] }, 'on twice is on once');
  assert.deepEqual(holding({ labels: ['landed', 'soon'], status: ['doing'] }, 'labels', 'landed', false), { labels: ['soon'], status: ['doing'] });
  assert.deepEqual(holding({ labels: ['landed'], status: ['doing'] }, 'labels', 'landed', false), { status: ['doing'] });
  assert.equal(holding({ labels: ['landed'] }, 'labels', 'landed', false), null);
});

test('what a view is held to is said in words', () => {
  assert.equal(heldTo({ labels: ['before-preview'] }, table), 'labels before-preview');
  assert.equal(heldTo({ labels: ['before-preview', 'landed'], status: ['doing'] }, table), 'labels before-preview or landed, and status doing');
  assert.equal(heldTo({ gone: ['x'] }, table), '', 'a field that is gone is not said');
  assert.equal(heldTo(null, table), '');
});

test('what a view is held to goes with it however it is laid out, less what names a field that is gone', () => {
  const view: View = { name: 'v', kind: 'board', by: 'status', only: { labels: ['landed'], nowhere: ['x'], part_of: [] } };
  for (const kind of ['board', 'order', 'table', 'gallery']) assert.deepEqual(goes(kind, card, view).only, { labels: ['landed'] }, kind);
  assert.equal(goes('table', card, { name: 'v', kind: 'table', by: null }).only, null);
  // And a view is gathered by a list as by anything else that names something.
  assert.equal(goes('board', card, { name: 'v', kind: 'board', by: 'status', group: 'labels' }).group, 'labels');
});

test('what a band the daemon counted is called: a record by what it is known by, somebody by their name, a looked-up id by what it is, anything else as it is, and nothing Ungrouped', () => {
  const rec: Field = { name: 'part_of', kind: 'record', of: ['features'], required: false };
  const others: Named = { features: { fields: [{ name: 'title', kind: 'text', of: [], required: true }], rows: [{ id: 'f-1', number: 1, data: { title: 'Checkout' }, position: 1, created: 0, updated: 0 }] } };
  assert.deepEqual(bandCalled('f-1', rec, others), { name: 'Checkout', gone: false });
  assert.deepEqual(bandCalled('f-9', rec, others), { name: 'f-9', gone: true });
  assert.deepEqual(bandCalled('agent:shop-worker-1', { name: 'who', kind: 'actor', of: [], required: false }, {}), { name: 'shop-worker-1', gone: false });
  assert.deepEqual(bandCalled('ab-3', { name: 'note', kind: 'text', of: [], required: false }, {}, { 'ab-3': 'Looked up' }), { name: 'Looked up', gone: false });
  assert.deepEqual(bandCalled('ready', { name: 'status', kind: 'choice', of: ['ready'], required: false }, {}), { name: 'ready', gone: false });
  assert.deepEqual(bandCalled('', { name: 'status', kind: 'choice', of: ['ready'], required: false }, {}), { name: UNGROUPED, gone: false });
});

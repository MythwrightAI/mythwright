import assert from 'node:assert/strict';
import { test } from 'node:test';
import { bandOfRow, drawnAgain, heardBoth, holding, lookedSaid, markSaid, moves, NONE, outSaid, said, seenOf } from './rowslive.ts';
import type { Live, Seen } from './rowslive.ts';
import type { Counted } from './rowsapi.ts';
import type { Field, Row } from './table.ts';
import type { Run } from './window.ts';

const row = (number: number, data: Record<string, unknown> = {}): Row => ({ id: `t-${number}`, number, data, position: number, created: 0, updated: 0 });
const run = (start: number, rows: Row[]): Run => ({ start, rows, before: null, after: 'a' });
const seen = (id: string, shown: boolean, data: Record<string, unknown> | null): Seen => ({ id, in: shown, row: data ? { ...row(Number(id.slice(2)), data), id } : null });

test('being added or taken away stands over being changed, and being taken away over everything', () => {
  assert.equal(heardBoth(undefined, 'updated'), 'updated');
  assert.equal(heardBoth('created', 'updated'), 'created');
  assert.equal(heardBoth('updated', 'removed'), 'removed');
  assert.equal(heardBoth('created', 'removed'), 'removed');
  assert.equal(heardBoth('removed', 'created'), 'created');
});

test('a row would stand elsewhere where what it says changed in the column the view is sorted or gathered by, and never by its number alone', () => {
  const was = row(1, { points: 3, stage: 'ready', title: 'a' });
  assert.equal(moves(was, row(1, { points: 4, stage: 'ready', title: 'a' }), { field: 'points', down: false }, null), true);
  assert.equal(moves(was, row(1, { points: 3, stage: 'ready', title: 'b' }), { field: 'points', down: false }, null), false);
  assert.equal(moves(was, row(1, { points: 3, stage: 'done', title: 'a' }), null, 'stage'), true);
  assert.equal(moves(was, row(1, { points: 9, stage: 'done', title: 'z' }), null, null), false);
});

test('a row held in any band is found, and drawn again where it stands with what it now says', () => {
  const bands = { '=a': [run(0, [row(1), row(2)])], '=b': [run(0, [row(2)])] };
  assert.deepEqual(holding(bands, 't-2').map((one) => one.key), ['=a', '=b']);
  assert.deepEqual(holding(bands, 't-9'), []);
  const again = drawnAgain(bands['=a'], 't-2', row(2, { title: 'now' }));
  assert.deepEqual(again[0]!.rows[1]!.data, { title: 'now' });
  assert.equal(again[0]!.rows[0], bands['=a'][0]!.rows[0], 'the rest as they were');
});

test('what the daemon says of the ids heard: drawn in place, marked as moved, struck, or counted as new', () => {
  const bands = { '*': [run(0, [row(1, { points: 1 }), row(2, { points: 2 }), row(3, { points: 3 })])] };
  const sort = { field: 'points', down: false };
  const came = seenOf(
    bands,
    NONE,
    [seen('t-1', true, { points: 1, title: 'changed' }), seen('t-2', true, { points: 99 }), seen('t-3', false, null), seen('t-7', true, { points: 0 }), seen('t-8', false, { points: 0 })],
    { 't-1': 'updated', 't-2': 'updated', 't-3': 'removed', 't-7': 'created', 't-8': 'created' },
    new Set(),
    sort,
    null,
  );
  const rows = came.bands['*']![0]!.rows;
  // Nothing moved on the screen: each where it was, with what it now says.
  assert.deepEqual(rows.map((one) => one.id), ['t-1', 't-2', 't-3']);
  assert.deepEqual(rows[0]!.data, { points: 1, title: 'changed' });
  assert.deepEqual(rows[1]!.data, { points: 99 });
  assert.deepEqual(came.live.marks, { 't-2': 'moved', 't-3': 'removed' });
  // Added and shown is new; added and not shown is nothing.
  assert.deepEqual(came.live.fresh, ['t-7']);
  assert.equal(said(came.live), '1 new row · 1 moved · 1 no longer here');
});

test('a row the view no longer shows is struck, and one shown again is not; a row the reader changed is no move to them, and one they took out is taken out', () => {
  const bands = { '*': [run(0, [row(1, { points: 1 }), row(2, { points: 2 })])] };
  const sort = { field: 'points', down: false };
  const out = seenOf(bands, NONE, [seen('t-1', false, { points: 1 })], { 't-1': 'updated' }, new Set(), sort, null);
  assert.deepEqual(out.live.marks, { 't-1': 'out' });
  const back = seenOf(out.bands, out.live, [seen('t-1', true, { points: 1 })], { 't-1': 'updated' }, new Set(), sort, null);
  assert.deepEqual(back.live.marks, {});
  const mine = seenOf(bands, NONE, [seen('t-2', true, { points: 50 })], { 't-2': 'updated' }, new Set(['t-2']), sort, null);
  assert.deepEqual(mine.live.marks, {});
  assert.deepEqual(mine.bands['*']![0]!.rows[1]!.data, { points: 50 });
  // Taken out of the view by the reader is taken out, and nothing is said of it.
  const theirs = seenOf(bands, NONE, [seen('t-2', false, null)], { 't-2': 'removed' }, new Set(['t-2']), sort, null);
  assert.deepEqual([theirs.live.marks, theirs.dropped], [{}, ['t-2']]);
  // One moved stays moved however it changes after.
  const moved = seenOf(bands, NONE, [seen('t-1', true, { points: 7 })], { 't-1': 'updated' }, new Set(), sort, null);
  const later = seenOf(moved.bands, moved.live, [seen('t-1', true, { points: 7, title: 'x' })], { 't-1': 'updated' }, new Set(), sort, null);
  assert.deepEqual(later.live.marks, { 't-1': 'moved' });
});

test('a new row counted once, and not at all once the view no longer shows it', () => {
  const live: Live = { marks: {}, fresh: ['t-9'], many: false };
  const sort = { field: 'a', down: false };
  const again = seenOf({}, live, [seen('t-9', true, { a: 1 })], { 't-9': 'created' }, new Set(), sort, null);
  assert.deepEqual(again.live.fresh, ['t-9']);
  const gone = seenOf({}, live, [seen('t-9', false, null)], { 't-9': 'removed' }, new Set(), sort, null);
  assert.deepEqual(gone.live.fresh, []);
});

test('where nobody sorts the view a row added comes after every row and is read in, and one the reader took out is taken out', () => {
  const bands = { '*': [run(0, [row(1), row(2)])] };
  const added = seenOf(bands, NONE, [seen('t-3', true, { a: 1 })], { 't-3': 'created' }, new Set(), null, null);
  assert.deepEqual([added.appended, added.live.fresh], [['t-3'], []]);
  // Changed and not added, it is nothing to read in.
  assert.deepEqual(seenOf(bands, NONE, [seen('t-4', true, { a: 1 })], { 't-4': 'updated' }, new Set(), null, null).appended, []);
  const theirs = seenOf(bands, NONE, [seen('t-2', false, null)], { 't-2': 'removed' }, new Set(['t-2']), null, null);
  assert.deepEqual([theirs.dropped, theirs.live.marks], [['t-2'], {}]);
  // Somebody else's, struck where it is.
  const others = seenOf(bands, NONE, [seen('t-2', false, null)], { 't-2': 'removed' }, new Set(), null, null);
  assert.deepEqual([others.dropped, others.live.marks], [[], { 't-2': 'removed' }]);
});

test('the line at the top says what is counted, or that many changed, and nothing where nothing is', () => {
  assert.equal(said(NONE), null);
  assert.equal(said({ marks: { a: 'moved', b: 'moved' }, fresh: ['c', 'd', 'e'], many: false }), '3 new rows · 2 moved');
  assert.equal(said({ marks: { a: 'out' }, fresh: [], many: false }), '1 no longer here');
  assert.equal(said({ marks: {}, fresh: [], many: true }), 'Many rows changed');
  assert.equal(markSaid('removed'), 'removed');
  assert.equal(markSaid('out'), 'no longer in this view');
});

test('how far a search has looked is said while it goes on, and what keeps a row out in the words of its columns', () => {
  const counted = (whole: boolean, looked?: number): Counted => ({ count: 3, of: 1000, bands: null, more_bands: 0, figures: {}, whole, looked });
  assert.equal(lookedSaid(counted(false, 400), 'ab'), `Looked through ${(400).toLocaleString()} of ${(1000).toLocaleString()}`);
  assert.equal(lookedSaid(counted(true, 1000), 'ab'), null);
  assert.equal(lookedSaid(counted(false, 400), ' '), null);
  assert.equal(lookedSaid(null, 'ab'), null);
  const fields = [{ name: 'result', kind: 'choice', of: [] } as unknown as Field, { name: 'when_run', kind: 'date', label_chosen: 'Ran', of: [] } as unknown as Field];
  assert.equal(outSaid(['result'], fields), 'It is not in this view: Result keeps it out.');
  assert.equal(outSaid(['result', 'when_run', 'find'], fields), 'It is not in this view: Result, Ran and what is searched for keep it out.');
});

test('the band a row is gone to in is the first thing it holds in the column the view is gathered by', () => {
  assert.equal(bandOfRow('ready'), 'ready');
  assert.equal(bandOfRow([' ', 'b', 'c']), 'b');
  assert.equal(bandOfRow(null), '');
  assert.equal(bandOfRow([]), '');
  assert.equal(bandOfRow(true), 'yes');
  assert.equal(bandOfRow(3), '3');
});

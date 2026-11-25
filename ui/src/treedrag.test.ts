import assert from 'node:assert/strict';
import { test } from 'node:test';
import { landing, stepped, steppedRows } from './treedrag.ts';
import type { Row, Tree } from './treedrag.ts';

// A tree as a carry measures it, each line 20 high (a project's, as its outline draws it): the board; Insights unfolded
// with two tables in it, one step in; Archive folded; Empty unfolded with nothing in it; and the Repo unfolded on its files.
const row = (node: string, into: string | null, top: number, more: Partial<Row> = {}): Row => ({
  node,
  into,
  folder: false,
  open: false,
  top,
  bottom: top + 20,
  end: top + 20,
  left: into ? 30 : 10,
  ...more,
});
const rows: Row[] = [
  row('board', null, 0),
  row('f1', null, 20, { folder: true, open: true, end: 80 }),
  row('table:a', 'f1', 40),
  row('table:b', 'f1', 60),
  row('f2', null, 80, { folder: true }),
  row('f3', null, 100, { folder: true, open: true, end: 140 }),
  row('repo', null, 140, { end: 240 }),
];
const lands = (carried: string, x: number, y: number) => {
  const to = landing(rows, carried, x, y);
  return to && (to.stays ? 'stays' : { into: to.into, before: to.before, line: to.line, folder: to.folder });
};

test('a line carried lands before the line whose upper half the pointer is over, at that line', () => {
  assert.deepEqual(lands('board', 40, 45), { into: 'f1', before: 'table:a', line: { top: 40, left: 30 }, folder: null });
  assert.deepEqual(lands('table:a', 15, 22), { into: null, before: 'f1', line: { top: 20, left: 10 }, folder: null });
});

test('over the middle of a folder it lands in it at its end, and the folder is said and not unfolded', () => {
  assert.deepEqual(lands('board', 40, 30), { into: 'f1', before: null, line: null, folder: 'f1' });
  assert.deepEqual(lands('board', 40, 86), { into: 'f2', before: null, line: null, folder: 'f2' });
  // An unfolded folder with nothing drawn in it is landed in below its line too.
  assert.deepEqual(lands('board', 40, 118), { into: 'f3', before: null, line: null, folder: 'f3' });
});

test('over the lower part of an unfolded folder it lands first in it, and of a folded one after it', () => {
  assert.deepEqual(lands('board', 40, 38), { into: 'f1', before: 'table:a', line: { top: 40, left: 30 }, folder: null });
  assert.deepEqual(lands('board', 40, 98), { into: null, before: 'f3', line: { top: 100, left: 10 }, folder: null });
});

test('after the last line of a folder it stays in it, or goes out as far as the pointer is further left', () => {
  assert.deepEqual(lands('board', 40, 75), { into: 'f1', before: null, line: { top: 80, left: 30 }, folder: null });
  assert.deepEqual(lands('board', 15, 75), { into: null, before: 'f2', line: { top: 80, left: 10 }, folder: null });
});

test('over what is drawn under a thing it lands after that thing, and outside the tree nowhere', () => {
  assert.deepEqual(lands('board', 40, 200), { into: null, before: null, line: { top: 240, left: 10 }, folder: null });
  assert.equal(lands('board', 40, -1), null);
  assert.equal(lands('board', 40, 240), null);
  assert.equal(landing(rows, 'table:z', 40, 45), null);
});

test('a line carried over itself, into itself or to where it is stays, and nobody is told', () => {
  assert.equal(lands('board', 40, 5), 'stays');
  assert.equal(lands('f1', 40, 45), 'stays');
  assert.equal(lands('f1', 40, 30), 'stays');
  // The last of Insights let go at its end, or in Insights, is where it was.
  assert.equal(lands('table:b', 40, 30), 'stays');
  assert.equal(lands('table:b', 40, 75), 'stays');
  // Before the line after it is where it is.
  assert.equal(lands('table:a', 40, 65), 'stays');
  assert.equal(lands('board', 40, 22), 'stays');
});

const drawn: Tree[] = [
  { thing: 'board' },
  { folder: 'f1', items: [{ thing: 'table:a' }, { thing: 'table:b' }] },
  { folder: 'f2', items: [] },
  { thing: 'repo' },
];

test('a step by the keyboard goes up or down its level, into the folder above it, or out of its folder', () => {
  assert.deepEqual(stepped(drawn, 'f2', 'up'), { into: null, before: 'f1' });
  assert.deepEqual(stepped(drawn, 'board', 'down'), { into: null, before: 'f2' });
  assert.deepEqual(stepped(drawn, 'f2', 'down'), { into: null, before: null });
  assert.deepEqual(stepped(drawn, 'table:b', 'up'), { into: 'f1', before: 'table:a' });
  assert.deepEqual(stepped(drawn, 'table:a', 'down'), { into: 'f1', before: null });
  assert.deepEqual(stepped(drawn, 'f2', 'in'), { into: 'f1', before: null });
  assert.deepEqual(stepped(drawn, 'table:a', 'out'), { into: null, before: 'f2' });
});

test('a step that has nowhere to go goes nowhere', () => {
  assert.equal(stepped(drawn, 'board', 'up'), null);
  assert.equal(stepped(drawn, 'repo', 'down'), null);
  assert.equal(stepped(drawn, 'board', 'in'), null);
  assert.equal(stepped(drawn, 'table:a', 'in'), null);
  assert.equal(stepped(drawn, 'table:b', 'in'), null);
  assert.equal(stepped(drawn, 'board', 'out'), null);
  assert.equal(stepped(drawn, 'table:z', 'up'), null);
});

// The same tree as a carry measures it: Archive folded, so what is in it is not drawn.
const drawnRows: Row[] = [
  row('board', null, 0),
  row('f1', null, 20, { folder: true, open: true, end: 80 }),
  row('table:a', 'f1', 40),
  row('table:b', 'f1', 60),
  row('f2', null, 80, { folder: true }),
  row('repo', null, 100),
];
const folded: Tree[] = [{ thing: 'board' }, { folder: 'f1', items: [{ thing: 'table:a' }, { thing: 'table:b' }] }, { folder: 'f2', items: [{ thing: 'table:c' }] }, { thing: 'repo' }];

test('a step worked out from the lines as drawn comes to what it comes to from the tree laid out', () => {
  for (const node of ['board', 'f1', 'table:a', 'table:b', 'f2', 'repo', 'table:z'])
    for (const way of ['up', 'down', 'in', 'out'] as const) {
      assert.deepEqual(steppedRows(drawnRows, node, way), stepped(drawn, node, way), `${node} ${way}`);
      // What is folded away is not drawn, and changes no step of what is.
      assert.deepEqual(steppedRows(drawnRows, node, way), stepped(folded, node, way), `${node} ${way}, Archive holding something`);
    }
});

test('a step from the lines as drawn goes into a folded folder at its end and out of a folder after it', () => {
  assert.deepEqual(steppedRows(drawnRows, 'repo', 'in'), { into: 'f2', before: null });
  assert.deepEqual(steppedRows(drawnRows, 'table:b', 'out'), { into: null, before: 'f2' });
  assert.equal(steppedRows([], 'board', 'up'), null);
});

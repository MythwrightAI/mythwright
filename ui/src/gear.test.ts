import assert from 'node:assert/strict';
import { test } from 'node:test';
import { besideLines, boardLines, headed } from './gear.ts';
import type { Sat, Sitting } from './sits.ts';
import type { Row } from './table.ts';

const table = (name: string): Sitting => ({ name, fields: [], beside: 'board' });
const row = (of: string, number: number): Row => ({ id: `shop-${of}-${number}`, number, data: {}, position: number, created: 0, updated: 0 });

test('the tables beside a table are lines under a heading, each with how many rows it has', () => {
  const sat: Sat[] = [
    { table: table('agents'), rows: [row('agents', 1), row('agents', 2)] },
    { table: table('segments'), rows: [] },
    { table: table('stages'), rows: [row('stages', 1)] },
  ];
  assert.deepEqual(besideLines(sat), [
    { says: 'Its tables' },
    { label: 'Agents', hint: '2 agents', does: 'agents' },
    { label: 'Segments', hint: 'no segments yet', does: 'segments' },
    { label: 'Stages', hint: '1 stage', does: 'stages' },
  ]);
});

test('a table called something for people is that in its line, with its rows counted in its words', () => {
  const tests = { ...table('contests'), title: 'A/B tests' };
  assert.deepEqual(besideLines([{ table: tests, rows: [] }])[1], { label: 'A/B tests', hint: 'no tests yet', does: 'contests' });
  assert.deepEqual(besideLines([{ table: { ...tests, title_chosen: 'Bake-offs' }, rows: null }])[1], { label: 'Bake-offs', hint: 'being read', does: 'contests' });
});

test('where no table sits beside it there is no heading and no line', () => {
  assert.deepEqual(besideLines([]), []);
});

test('a table whose rows are not read yet says so, and is opened all the same', () => {
  assert.deepEqual(besideLines([{ table: table('agents'), rows: null }]), [{ says: 'Its tables' }, { label: 'Agents', hint: 'being read', does: 'agents' }]);
});

test('a table is headed by its name, and opened by its name as it is kept', () => {
  assert.equal(headed('open-questions'), 'Open questions');
  assert.equal(headed('work_log'), 'Work log');
  assert.deepEqual(besideLines([{ table: table('open-questions'), rows: [] }])[1], { label: 'Open questions', hint: 'no open-questions yet', does: 'open-questions' });
});

test('a board\'s tools are four lines in one order, each with its word on what it does: at the end of its line, or under it where the name is long', () => {
  assert.deepEqual(boardLines(['backlog', 'ready', 'doing', 'review', 'done'], 'Board'), [
    { label: 'Add a card', hint: 'to backlog', does: 'card' },
    { label: 'Add a column', hint: 'after done', does: 'column' },
    { label: 'Arrange the columns', note: 'Change their order, or take one away.', does: 'arrange' },
    { label: 'Layout', hint: 'Board', does: 'layout' },
  ]);
  // How the view is laid out is said as it is now.
  assert.equal(boardLines(['todo'], 'Table')[3]!.hint, 'Table');
});

test('a board with one column adds a card to it and a column after it; one with none says of none', () => {
  const one = boardLines(['todo'], 'Board');
  assert.deepEqual([one[0]!.hint, one[1]!.hint], ['to todo', 'after todo']);
  const none = boardLines([]);
  assert.deepEqual(none.map((line) => line.hint), [undefined, undefined, undefined, undefined]);
  // What is said under a line is said whatever columns there are, and of no other line.
  assert.deepEqual(none.map((line) => line.note), [undefined, undefined, 'Change their order, or take one away.', undefined]);
  // Every line is still there, and each does what it did.
  assert.deepEqual(none.map((line) => line.does), ['card', 'column', 'arrange', 'layout']);
});

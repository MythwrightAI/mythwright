import assert from 'node:assert/strict';
import { test } from 'node:test';
import { aboutOf, aboutThis, besideOf, counted, oneOf, rowWords, sitsBeside } from './sits.ts';
import type { Sat, Sitting } from './sits.ts';
import type { Field, Row } from './table.ts';

const text = (name: string, required = false): Field => ({ name, kind: 'text', of: [], required });
const table = (name: string, more: Partial<Sitting> = {}): Sitting => ({ name, fields: [text('title', true), text('column')], ...more });
const row = (table: string, number: number, data: Record<string, unknown>): Row => ({ id: `shop-${table}-${number}`, number, data, position: number, created: 0, updated: 0 });

const board = table('board');
const agents = table('agents', { beside: 'board' });
const stages = table('stages', { beside: 'board', about: 'column' });
const segments = table('segments', { beside: 'board' });
const notes = table('notes');

test('a table sits beside the one it names, where its project has that one', () => {
  const all = [board, agents, stages, segments, notes];
  assert.equal(sitsBeside(agents, all), 'board');
  assert.equal(sitsBeside(stages, all), 'board');
  // One that says nothing sits beside nothing, and so does the one others sit beside.
  assert.equal(sitsBeside(notes, all), null);
  assert.equal(sitsBeside(board, all), null);
  assert.equal(sitsBeside(table('old', { beside: null, about: null }), all), null);
});

test('a table that names one that is not there, or itself, sits beside nothing and is shown as any table is', () => {
  // A fleet's tables in a project whose board was never made, or has gone.
  assert.equal(sitsBeside(agents, [agents, stages]), null);
  assert.deepEqual(besideOf('board', [agents, stages]), []);
  const own = table('own', { beside: 'own' });
  assert.equal(sitsBeside(own, [board, own]), null);
});

test('tables that sit beside each other, each reached only from the other, are neither taken at their word', () => {
  const [a, b] = [table('a', { beside: 'b' }), table('b', { beside: 'a' })];
  assert.equal(sitsBeside(a, [a, b, board]), null);
  assert.equal(sitsBeside(b, [a, b, board]), null);
  // Nor a longer round; and one that sits beside a table of such a round is not reached either.
  const ring = [table('x', { beside: 'y' }), table('y', { beside: 'z' }), table('z', { beside: 'x' }), table('w', { beside: 'x' })];
  for (const one of ring) assert.equal(sitsBeside(one, ring), null, one.name);
  // A table beside one that itself sits beside another is reached from that one, which is reached from the last.
  const chain = [board, stages, table('steps', { beside: 'stages' })];
  assert.equal(sitsBeside(chain[2]!, chain), 'stages');
  assert.deepEqual(besideOf('stages', chain).map((one) => one.name), ['steps']);
  assert.deepEqual(besideOf('board', chain).map((one) => one.name), ['stages']);
});

test('what sits beside a table is listed by name, whatever order its tables were read in', () => {
  assert.deepEqual(besideOf('board', [stages, notes, segments, board, agents]).map((one) => one.name), ['agents', 'segments', 'stages']);
  assert.deepEqual(besideOf('notes', [stages, notes, segments, board, agents]), []);
});

test('a row is about what its table\'s field says, and about what it is called where that says nothing', () => {
  assert.equal(aboutOf(stages, row('stages', 1, { title: 'work', column: 'ready' })), 'ready');
  // A cell never filled, emptied, or filled with spaces says nothing.
  assert.equal(aboutOf(stages, row('stages', 2, { title: 'review' })), 'review');
  assert.equal(aboutOf(stages, row('stages', 2, { title: 'review', column: '' })), 'review');
  assert.equal(aboutOf(stages, row('stages', 2, { title: 'review', column: '  ' })), 'review');
  // A table that says no field, or one it has not got, has rows that are about nothing.
  assert.equal(aboutOf(agents, row('agents', 1, { title: 'worker', column: 'ready' })), '');
  assert.equal(aboutOf(table('odd', { beside: 'board', about: 'lane' }), row('odd', 1, { title: 'ready' })), '');
});

test('a column\'s head is said the rows about it, a table at a time, each by what it is called', () => {
  const limits = table('limits', { beside: 'board', about: 'column' });
  const sat: Sat[] = [
    { table: agents, rows: [row('agents', 1, { title: 'ready' })] },
    { table: limits, rows: [row('limits', 2, { title: 'three waiting', column: 'ready' }), row('limits', 1, { title: 'done' })] },
    { table: stages, rows: [row('stages', 1, { title: 'work', column: 'ready' }), row('stages', 2, { title: 'review', column: 'review' }), row('stages', 3, { title: 'check', column: 'ready' })] },
  ];
  assert.deepEqual(aboutThis(sat, 'ready').map((one) => `${one.table}:${one.called}`), ['limits:three waiting', 'stages:work', 'stages:check']);
  assert.deepEqual(aboutThis(sat, 'review').map((one) => one.row.id), ['shop-stages-2']);
  assert.deepEqual(aboutThis(sat, 'done').map((one) => one.called), ['done']);
  // No row about a column says nothing of it; nor is anything about the column with no name.
  assert.deepEqual(aboutThis(sat, 'doing'), []);
  assert.deepEqual(aboutThis(sat, ''), []);
  // Rows that have not been read yet say nothing, and nothing fails.
  assert.deepEqual(aboutThis([{ table: stages, rows: null }], 'ready'), []);
});

test('how many rows a table has is said in its own words', () => {
  const ones: [string, string][] = [
    ['agents', 'agent'],
    ['stages', 'stage'],
    ['entries', 'entry'],
    ['contests', 'contest'],
    ['statuses', 'status'],
    ['boxes', 'box'],
    ['branches', 'branch'],
    ['wishes', 'wish'],
    ['classes', 'class'],
    ['actions', 'action'],
    ['segments', 'segment'],
    // What it cannot tell is a row: a word that is no plural, one that ends as no plural does, and one with no s.
    ['news', 'row'],
    ['status', 'row'],
    ['access', 'row'],
    ['staff', 'row'],
    ['s', 'row'],
  ];
  for (const [name, one] of ones) assert.equal(oneOf(name), one, name);
  assert.equal(counted('entries', [row('entries', 1, {}), row('entries', 2, {})]), '2 entries');
  assert.equal(counted('entries', [row('entries', 1, {})]), '1 entry');
  assert.equal(counted('boxes', []), 'no boxes yet');
  assert.equal(counted('staff', []), 'no rows yet');
  assert.equal(counted('agents', [row('agents', 1, {}), row('agents', 2, {})]), '2 agents');
  assert.equal(counted('stages', [row('stages', 1, {})]), '1 stage');
  assert.equal(counted('segments', []), 'no segments yet');
  assert.equal(counted('staff', [row('staff', 1, {})]), '1 row');
  assert.equal(counted('agents', null), '');
});

test('what a table called something for people counts its rows by is the last word of that, made plural as English makes it', () => {
  // A case met in use: the A/B attempts and tests of a card, beside a table called by its name alone.
  const attempts = { name: 'entries', title: 'A/B attempts' };
  const rows = (many: number) => Array.from({ length: many }, (_, at) => row('entries', at + 1, {}));
  assert.equal(counted(attempts, rows(4)), '4 attempts');
  assert.equal(counted(attempts, rows(1)), '1 attempt');
  assert.equal(counted(attempts, []), 'no attempts yet');
  assert.equal(counted({ name: 'contests', title: 'A/B tests' }, rows(1)), '1 test');
  // Somebody's word stands over the blueprint's; a title that ends in one is made plural; one that is no word goes by the name.
  assert.equal(counted({ ...attempts, title_chosen: 'Tries' }, rows(2)), '2 tries');
  assert.equal(counted({ name: 'test-runs', title: 'Test run' }, rows(2)), '2 runs');
  assert.equal(counted({ name: 'boxes', title: 'Lunch box' }, rows(3)), '3 boxes');
  assert.equal(counted({ name: 'tallies', title: 'Daily tally' }, rows(1)), '1 tally');
  assert.equal(counted({ name: 'entries', title: 'Q3' }, rows(2)), '2 entries');
  assert.equal(counted({ name: 'staff', title: 'Our status' }, rows(2)), '2 rows');
  assert.equal(counted({ name: 'git', title: 'Git' }, rows(1)), '1 row', 'a title that is only its name says nothing of a row');
  assert.deepEqual(rowWords({ name: 'entries', title: 'A/B attempts' }), { one: 'attempt', many: 'attempts' });
});

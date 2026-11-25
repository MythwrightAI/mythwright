import { strict as assert } from 'node:assert';
import { test } from 'node:test';
import { actionSays, rowSays, setUpOf, staged, stageSays } from './columnset.ts';
import { actionsOf } from './cardactions.ts';
import type { SetTable } from './columnset.ts';
import type { Field, Row } from './table.ts';

const field = (name: string, kind = 'text', extra: Partial<Field> = {}): Field => ({ name, kind, of: [], required: name === 'title', ...extra });
const row = (id: string, data: Record<string, unknown>, position = 0): Row => ({ id, number: Number(id.split('-').pop()), data, position, created: 0, updated: 0 });

const stages: SetTable = { name: 'stages', beside: 'board', about: 'column', takes: 'agents:write', fields: [field('title'), field('column'), field('does'), field('role'), field('pass'), field('fail'), field('at_once', 'number'), field('rounds', 'number'), field('merge', 'flag')] };
const actions: SetTable = { name: 'actions', beside: 'board', about: 'column', takes: 'agents:write', fields: [field('title'), field('name'), field('column'), field('who', 'choice'), field('holds', 'flag'), field('again', 'flag')] };
const segments: SetTable = { name: 'segments', beside: 'board', fields: [field('title'), field('stage')] };

const sat = [
  { table: actions, rows: [row('shop-actions-1', { title: 'A/B test laid', column: 'ready', holds: true }), row('shop-actions-2', { title: 'Looked', column: 'review' })] },
  { table: segments, rows: [row('shop-segments-1', { title: 'tests', stage: 'work' })] },
  { table: stages, rows: [row('shop-stages-2', { title: 'work', column: 'ready', does: 'work', role: 'worker' }, 1), row('shop-stages-1', { title: 'review', does: 'review', merge: true })] },
];
const WRITES = ['records', 'agents'];

test("a column's rows are under a heading for each table that is about columns, each with a few words, and a dot where one holds its cards", () => {
  const ready = setUpOf(sat, 'ready', WRITES);
  assert.deepEqual(
    ready.groups.map((group) => [group.heading, group.rows.map((one) => [one.called, one.says])]),
    [
      ['Actions', [['A/B test laid', 'Holds each card here until A/B test laid is pressed · once for good']]],
      ['Stages', [['work', 'Worked by the worker role, as many at a time as its agents can take → review on pass']]],
    ],
  );
  assert.equal(ready.holds, true);
  // A row that says no column is about the one it is called, as a column's head always read it.
  const review = setUpOf(sat, 'review', WRITES);
  assert.deepEqual(review.groups.map((group) => group.rows.map((one) => one.says)), [
    ['Looked may be pressed on each card here · once for good'],
    ['Reviewed by the review role, as many at a time as its agents can take · stays here on pass · back to doing on fail'],
  ]);
  assert.equal(review.holds, false);
});

test('a column with nothing about it has nothing set up, and nothing to add from its head', () => {
  assert.deepEqual(setUpOf(sat, 'done', WRITES), { groups: [], holds: false, adds: [] });
  assert.deepEqual(setUpOf(sat, '', WRITES).groups, []);
  assert.deepEqual(setUpOf([], 'ready', WRITES).groups, []);
});

test('a row of each table the menu lists may be added for the column by whoever may write it, and none by anybody else', () => {
  assert.deepEqual(setUpOf(sat, 'ready', WRITES).adds, [
    { table: 'actions', one: 'action' },
    { table: 'stages', one: 'stage' },
  ]);
  // A table about something that is not a column has no row about one, and is not offered.
  const contests: SetTable = { name: 'contests', beside: 'board', about: 'card', fields: [field('card')] };
  assert.deepEqual(setUpOf([...sat, { table: contests, rows: [row('shop-contests-1', { card: 'shop-3' })] }], 'ready', WRITES).adds.map((one) => one.table), ['actions', 'stages']);
  assert.deepEqual(setUpOf(sat, 'review', WRITES).adds.map((one) => one.table), ['actions', 'stages']);
  // What writing the table takes, beside writing records.
  assert.deepEqual(setUpOf(sat, 'ready', ['records']).adds, []);
  assert.deepEqual(setUpOf(sat, 'ready', ['board:read', 'agents']).adds, []);
  assert.deepEqual(setUpOf(sat, 'ready', ['*']).adds.length, 2);
  // A table whose rows are written by what keeps it is added to by nobody here.
  const kept = sat.map((one) => (one.table === stages ? { ...one, table: { ...stages, by_hand: false } } : one));
  assert.deepEqual(setUpOf(kept, 'ready', WRITES).adds, [{ table: 'actions', one: 'action' }]);
});

test("what a row's few words say leaves out what it is known by, its column, a list and a long cell, and says one of a table called nothing as no row", () => {
  const notes: SetTable = { name: 'staff', about: 'column', fields: [field('title'), field('column'), field('tags', 'list'), field('note'), field('where')] };
  assert.equal(rowSays(notes, row('x-1', { title: 'a', column: 'ready', tags: ['x'], note: 'n'.repeat(30), where: 'here' }), false, false), 'where here');
  assert.equal(rowSays(notes, row('x-1', { title: 'a' }), false, false), '');
});

test("a stage's rule is said in words from what its row says, and what it leaves empty as the fleet fills it in", () => {
  const stage = (data: Record<string, unknown>) => row('shop-stages-9', data);
  // What the fleet starts with: a worker and a reviewer.
  assert.equal(stageSays(stage({ title: 'work', column: 'ready', does: 'work', role: 'worker', pass: 'review' })), 'Worked by the worker role, as many at a time as its agents can take → review on pass');
  assert.equal(stageSays(stage({ title: 'review', column: 'review', does: 'review', role: 'reviewer', pass: 'done', fail: 'doing', rounds: 2, merge: true })), 'Reviewed by the reviewer role, as many at a time as its agents can take → done on pass · back to doing on fail');
  // A number of cards at once is at most that many: never more than the role's agents can take.
  assert.equal(stageSays(stage({ title: 'work', does: 'work', role: 'worker', pass: 'review', at_once: 2 })), 'Worked by the worker role, at most 2 at a time → review on pass');
  assert.equal(stageSays(stage({ title: 'work', does: 'work', at_once: 0 })), 'Worked by the work role, as many at a time as its agents can take → review on pass');
  // Nothing said of what is done is a review; of the role, what the row is called written as a name is; a worked card
  // goes on to review where nothing is said, a sent back one to doing, and a card passed elsewhere stays where it is.
  assert.equal(stageSays(stage({ title: 'Look It Over' })), 'Reviewed by the look-it-over role, as many at a time as its agents can take · stays here on pass · back to doing on fail');
  assert.equal(stageSays(stage({ title: 'plan', does: 'plan', role: 'architect', pass: 'ready' })), 'Planned by the architect role, as many at a time as its agents can take → ready on pass');
  assert.equal(stageSays(stage({ title: 'plan', does: 'plan', role: 'architect', pass: '' })), 'Planned by the architect role, as many at a time as its agents can take · stays here on pass');
  // An emptied cell is nothing said, as `rows.read` takes it.
  assert.equal(stageSays(stage({ title: 'review', does: ' ', role: '', pass: 'done', fail: '' })), 'Reviewed by the review role, as many at a time as its agents can take → done on pass · back to doing on fail');
  // Any other word names the blueprint begun for each card; only where cards are reviewed is one sent back.
  assert.equal(stageSays(stage({ title: 'probe', does: 'pipelines/probe', pass: 'review', fail: 'ready' })), 'pipelines/probe is begun for each card here, as many at a time as its agents can take → review on pass');
  // A column the board has not got is said to be none of its.
  assert.equal(stageSays(stage({ title: 'review', does: 'review', pass: 'shipped', fail: 'doing' }), ['ready', 'doing', 'review', 'done']), 'Reviewed by the review role, as many at a time as its agents can take → shipped (no such column) on pass · back to doing on fail');
  assert.equal(stageSays(stage({ title: 'review', does: 'review', pass: 'done', fail: 'doing' }), ['ready', 'doing', 'review', 'done']), 'Reviewed by the review role, as many at a time as its agents can take → done on pass · back to doing on fail');
});

test('what is pressed on a card is said by whether it holds the card and whether a press is for good', () => {
  const said = (data: Record<string, unknown>) => actionSays(actionsOf(actions, [row('shop-actions-9', data)])[0]!);
  assert.equal(said({ title: 'A/B test laid', column: 'ready', holds: true, again: true }), 'Holds each card here until A/B test laid is pressed · each time it comes');
  assert.equal(said({ title: 'Signed', column: 'ready', holds: true }), 'Holds each card here until Signed is pressed · once for good');
  assert.equal(said({ title: 'Looked', column: 'review', again: true }), 'Looked may be pressed on each card here · each time it comes');
  assert.equal(said({ name: 'tidy', column: 'review' }), 'tidy may be pressed on each card here · once for good');
});

test('a table is one of stages by the fields it says and never by its name', () => {
  assert.equal(staged(stages), true);
  assert.equal(staged({ ...stages, name: 'steps' }), true);
  assert.equal(staged(actions), false);
  // One a program from before declared, which says no `does`, is said as any table's rows are.
  const before: SetTable = { name: 'stages', beside: 'board', about: 'column', fields: [field('title'), field('column'), field('fail')] };
  assert.equal(staged(before), false);
  assert.deepEqual(setUpOf([{ table: before, rows: [row('shop-stages-1', { title: 'review', fail: 'ready' })] }], 'review', WRITES).groups[0]!.rows[0]!.says, 'a stage · fail ready');
});

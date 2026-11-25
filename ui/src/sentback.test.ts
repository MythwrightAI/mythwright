import assert from 'node:assert/strict';
import { test } from 'node:test';
import { asStage, backTo, sendingBack } from './sentback.ts';
import type { Sitting } from './sits.ts';
import type { Field, Row } from './table.ts';

const columns = ['backlog', 'ready', 'doing', 'review', 'check', 'done'];
const stage = (data: Record<string, unknown>, number = 1): Row => ({ id: `shop-stages-${number}`, number, data, position: number, created: 0, updated: 0 });
const work = stage({ title: 'work', column: 'ready', does: 'work', role: 'worker', pass: 'review' });

test('a card is sent back where the stage that has its column says a card that did not pass goes', () => {
  const review = stage({ title: 'review', column: 'review', does: 'review', pass: 'done', fail: 'ready' }, 2);
  assert.equal(backTo('review', columns, [work, review]), 'ready');
  // Whatever the column is called: the row says which is the stage's, and where from there.
  const check = stage({ title: 'second look', column: 'check', fail: 'doing' }, 3);
  assert.equal(backTo('check', columns, [work, review, check]), 'doing');
});

test('a stage that says no column has the one it is called after', () => {
  assert.equal(backTo('check', columns, [stage({ title: 'check', fail: 'ready' })]), 'ready');
  // A cell somebody emptied says nothing, as one never filled does.
  assert.equal(backTo('check', columns, [stage({ title: 'check', column: '', fail: 'ready' })]), 'ready');
});

test('where no stage has the column it is as it was: a card in review goes back to doing', () => {
  assert.equal(backTo('review', columns, null), 'doing');
  assert.equal(backTo('review', columns, []), 'doing');
  assert.equal(backTo('review', columns, [work]), 'doing');
  // And nothing else is sent back at all.
  assert.equal(backTo('ready', columns, null), null);
  assert.equal(backTo('check', columns, [work]), null);
});

test('a stage that has the column and names nowhere leaves it as it was', () => {
  assert.equal(backTo('review', columns, [stage({ title: 'review', column: 'review', does: 'review', pass: 'done' })]), 'doing');
  assert.equal(backTo('review', columns, [stage({ title: 'review', fail: '' })]), 'doing');
  // A stage that works its cards says nothing of where one that did not pass goes, and its cards are not sent back.
  assert.equal(backTo('ready', columns, [work]), null);
});

test('of two stages that have a column the first says, as the fleet takes it', () => {
  const first = stage({ title: 'review', column: 'review', fail: 'ready' }, 1);
  const second = stage({ title: 'again', column: 'review', fail: 'backlog' }, 2);
  assert.equal(backTo('review', columns, [first, second]), 'ready');
});

test('a card is not sent somewhere its board has not got, nor where it is', () => {
  assert.equal(backTo('review', columns, [stage({ title: 'review', fail: 'rework' })]), null);
  assert.equal(backTo('review', columns, [stage({ title: 'review', fail: 'review' })]), null);
  // A board with no column where cards are worked, and no row to say another.
  assert.equal(backTo('review', ['todo', 'review', 'done'], null), null);
});

const field = (name: string): Field => ({ name, kind: 'text', of: [], required: false });
const tableOf = (name: string, fields: string[], more: Partial<Sitting> = {}): Sitting => ({ name, fields: fields.map(field), ...more });

test('the rows that say where a card goes back are those of a table beside the board that is about its columns, whatever it is called', () => {
  const board = tableOf('board', ['title', 'status']);
  const lanes = tableOf('lanes', ['title', 'lane', 'fail'], { beside: 'board', about: 'lane' });
  const all = [board, tableOf('agents', ['title'], { beside: 'board' }), lanes, tableOf('limits', ['title', 'column'], { beside: 'board', about: 'column' }), tableOf('stages', ['title', 'column', 'fail'])];
  // Not one that is about nothing, nor one with nowhere to say where a card goes back; and no table by its name while one says so.
  assert.deepEqual(sendingBack(all).map((one) => one.name), ['lanes']);
  const second = asStage(lanes, stage({ title: 'second look', lane: 'check', fail: 'ready' }, 3));
  assert.equal(backTo('check', columns, [second]), 'ready');
  // A row that says no column is about the one it is called after, as it always was.
  assert.equal(backTo('check', columns, [asStage(lanes, stage({ title: 'check', fail: 'doing' }))]), 'doing');
  // A table that sits beside something else than the board says nothing of its cards.
  assert.deepEqual(sendingBack([board, tableOf('notes', ['title']), tableOf('lanes', ['title', 'lane', 'fail'], { beside: 'notes', about: 'lane' })]), []);
});

test('a fleet declared before a table could say what it sits beside is read as it always was: the table called stages, by its column', () => {
  const stages = tableOf('stages', ['title', 'column', 'fail']);
  const found = sendingBack([tableOf('board', ['title', 'status']), tableOf('agents', ['title']), stages]);
  assert.deepEqual(found.map((one) => [one.name, one.about]), [['stages', 'column']]);
  const review = asStage(found[0]!, stage({ title: 'review', column: 'review', fail: 'ready' }, 2));
  assert.equal(backTo('review', columns, [review]), 'ready');
  assert.equal(backTo('check', columns, [asStage(found[0]!, stage({ title: 'check', fail: 'ready' }))]), 'ready');
  // And a project with neither has no such rows.
  assert.deepEqual(sendingBack([tableOf('board', ['title', 'status'])]), []);
});

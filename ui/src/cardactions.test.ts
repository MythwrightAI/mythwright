import assert from 'node:assert/strict';
import { test } from 'node:test';
import { actionsOf, after, among, comingOf, keptOf, mayPress, named, offered, pressedOf, pressing, waitsFor, whoPressed } from './cardactions.ts';
import type { Sitting } from './sits.ts';
import type { Field, Row } from './table.ts';

const field = (name: string, kind = 'text'): Field => ({ name, kind, of: [], required: false }) as Field;
const actions: Sitting = {
  name: 'actions',
  beside: 'board',
  about: 'column',
  fields: ['title', 'name', 'column', 'scope', 'says'].map((name) => field(name)).concat([field('who', 'choice'), field('holds', 'flag'), field('again', 'flag')]),
};
const row = (id: string, data: Record<string, unknown>): Row => ({ id, number: 1, position: 1, data }) as unknown as Row;
const rows = [
  row('shop-actions-1', { title: 'Signed', column: 'ready', who: 'person', holds: true }),
  row('shop-actions-2', { title: 'Cleared to go', column: 'ready', holds: true, again: true }),
  row('shop-actions-3', { title: 'Noted', name: 'note', column: 'ready' }),
  row('shop-actions-4', { title: 'Looked over', column: 'review', scope: 'admin', holds: true }),
  row('shop-actions-5', { title: '  ' }),
];

test('the tables whose rows are pressed on a card are those beside the board that say a column and who may press', () => {
  const stages: Sitting = { name: 'stages', beside: 'board', about: 'column', fields: [field('title'), field('column'), field('fail')] };
  const board: Sitting = { name: 'board', fields: [field('title')] };
  const elsewhere: Sitting = { ...actions, name: 'elsewhere', beside: 'notes' };
  const notes: Sitting = { name: 'notes', fields: [field('title')] };
  const aboutNothing: Sitting = { ...actions, name: 'loose', about: null };
  assert.deepEqual(
    pressing([board, stages, actions, elsewhere, notes, aboutNothing]).map((one) => one.name),
    ['actions'],
    'by what each says, and by no name',
  );
  assert.deepEqual(pressing([board, stages]), [], 'a board with nothing to press');
  // As the board's page has them: the tables beside it with their rows, those not yet read pressing nothing.
  const read = among([{ table: stages, rows: [] }, { table: actions, rows }, { table: { ...actions, name: 'later' }, rows: null }]);
  assert.deepEqual(read.map((one) => one.name), ['signed', 'cleared-to-go', 'note', 'looked-over']);
});

test('a row is read as the daemon reads it, by its name or what it is called', () => {
  const read = actionsOf(actions, rows);
  assert.deepEqual(
    read.map((one) => [one.name, one.title, one.column, one.who, one.holds, one.again, one.scope]),
    [
      ['signed', 'Signed', 'ready', 'person', true, false, ''],
      ['cleared-to-go', 'Cleared to go', 'ready', 'anyone', true, true, ''],
      ['note', 'Noted', 'ready', 'anyone', false, false, ''],
      ['looked-over', 'Looked over', 'review', 'anyone', true, false, 'admin'],
    ],
    'one called nothing is nothing to press',
  );
  assert.equal(named(' Signed off! '), 'signed-off');
  // A row that says no column is offered in none, as the daemon reads it.
  assert.equal(actionsOf(actions, [row('x', { title: 'ready' })])[0]!.column, '');
});

test('what a card keeps of presses is read and written as the board writes it, and a hold is said once for each coming', () => {
  const kept = ['pressed signed 2 1000 user:ada Ada Lovelace', 'nonsense', 'pressed note 2 10 key:k', 'held 2', 'pressed signed 3 2000 agent:x x'];
  const pressed = pressedOf(kept);
  assert.deepEqual(pressed.presses.map((one) => [one.action, one.coming, one.by, one.name]), [
    ['note', 2, 'key:k', ''],
    ['signed', 3, 'agent:x', 'x'],
  ]);
  assert.equal(pressed.held, 2);
  assert.deepEqual(pressedOf(keptOf(pressed)), pressed, 'written back, it reads the same');
  assert.deepEqual(pressedOf(undefined), { presses: [], held: null });
  assert.equal(comingOf(['ready', 'user', '', '3']), 3);
  assert.equal(comingOf([]), 0, 'never moved, it is the first coming');
  assert.equal(comingOf(['yesterday']), 0);
});

test('what a card is offered stands pressed for good, or for the time it came, and what holds and stands unpressed is what it waits for', () => {
  const all = actionsOf(actions, rows);
  const pressed = pressedOf(['pressed signed 2 1 user:ada Ada', 'pressed cleared-to-go 2 2 user:ada Ada']);
  const at = (coming: number) => offered(all, 'ready', pressed, coming).map(({ action, press }) => [action.name, press?.coming ?? null]);
  assert.deepEqual(at(2), [['signed', 2], ['cleared-to-go', 2], ['note', null]]);
  // Sent back and come again: what is for one coming is to be pressed again.
  assert.deepEqual(at(5), [['signed', 2], ['cleared-to-go', null], ['note', null]]);
  assert.deepEqual(waitsFor(all, 'ready', pressed, 5).map((one) => one.title), ['Cleared to go']);
  assert.deepEqual(waitsFor(all, 'ready', pressedOf([]), 0).map((one) => one.title), ['Signed', 'Cleared to go'], 'what holds nothing is not waited for');
  assert.deepEqual(waitsFor(all, 'doing', pressedOf([]), 0), [], 'a column no row is of holds nothing');
  assert.deepEqual(waitsFor([], 'ready', pressedOf([]), 0), [], 'with no row, nothing waits');
});

test('the button is drawn for whoever the daemon would let press it', () => {
  const [signed, cleared, , looked] = actionsOf(actions, rows) as [ReturnType<typeof actionsOf>[number], ReturnType<typeof actionsOf>[number], unknown, ReturnType<typeof actionsOf>[number]];
  assert.equal(mayPress(signed, ['board'], 'user'), true);
  assert.equal(mayPress(signed, ['board:read'], 'user'), false, 'one who may not change cards');
  assert.equal(mayPress(signed, ['*'], 'agent'), false, 'a person presses it, and not an agent');
  assert.equal(mayPress(cleared, ['board'], 'agent'), true);
  assert.equal(mayPress(looked, ['board', 'records'], 'user'), false, 'its scope is not held');
  assert.equal(mayPress(looked, ['*'], 'user'), true);
});

test('what is said of a press is taken in with nothing read again', () => {
  const was = ['pressed signed 2 1 user:ada Ada'];
  const pressed = after(was, 'board.item.pressed', { action: 'cleared-to-go', coming: 2, by: 'user:bob', name: 'Bob', back: false }, 50);
  assert.deepEqual(pressed, ['pressed signed 2 1 user:ada Ada', 'pressed cleared-to-go 2 50 user:bob Bob']);
  assert.deepEqual(after(pressed, 'board.item.pressed', { action: 'signed', coming: 2, back: true }, 60), ['pressed cleared-to-go 2 50 user:bob Bob']);
  assert.deepEqual(after(was, 'board.item.held', { coming: 2 }, 70), ['pressed signed 2 1 user:ada Ada', 'held 2']);
  assert.deepEqual(after(undefined, 'board.item.moved', {}, 1), [], 'nothing else is a press');
  const press = pressedOf(pressed).presses[1]!;
  assert.equal(whoPressed(press, 'user:bob'), 'you');
  assert.equal(whoPressed(press, 'user:ada'), 'Bob');
  assert.equal(whoPressed({ ...press, name: '' }, 'user:ada'), 'bob');
});

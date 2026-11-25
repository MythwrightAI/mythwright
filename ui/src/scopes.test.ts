import assert from 'node:assert/strict';
import { test } from 'node:test';
import type { Area } from './holds.ts';
import { afterLine, briefly, brieflyHeld, comesTo, isScopes, lacking, saidIn } from './scopes.ts';
import type { Field } from './table.ts';

const AREAS: Area[] = [
  { area: 'admin', does: 'manage accounts', verbs: [] },
  { area: 'board', verbs: [{ verb: 'read', does: 'read work items' }, { verb: 'write', does: 'create, change and move work items' }] },
  { area: 'blueprints', verbs: [{ verb: 'read', does: 'read blueprints' }, { verb: 'run', does: 'start, apply and stop' }] },
  { area: 'records', verbs: [{ verb: 'read', does: 'read records' }, { verb: 'write', does: 'change records' }] },
  { area: 'shell', does: 'run commands on a device', verbs: [] },
];
const holds: Field = { name: 'holds', kind: 'list', of: [], required: false, takes_each: true, unsaid: ['board', 'records', 'blueprints:read'] };
const labels: Field = { name: 'labels', kind: 'list', of: [], required: false };

test('a list is a list of scopes where its table says a record gives them, whatever the field is called', () => {
  assert.equal(isScopes(holds), true);
  assert.equal(isScopes({ ...holds, name: 'may' }), true);
  assert.equal(isScopes(labels), false);
  // Called `holds` and saying nothing of the kind, it is a list like any other.
  assert.equal(isScopes({ ...labels, name: 'holds' }), false);
  assert.equal(isScopes({ ...holds, kind: 'text' }), false);
});

test('a cell never filled, or emptied, says nothing, and comes to what its field says that stands for', () => {
  for (const nothing of [undefined, null, [], '']) {
    assert.equal(saidIn(nothing), null);
    assert.deepEqual(comesTo(holds, nothing), ['board', 'records', 'blueprints:read']);
    assert.equal(briefly(holds, nothing, AREAS), 'the usual');
  }
  // A list that says nothing of what an empty one stands for comes to nothing.
  assert.deepEqual(comesTo({ ...holds, unsaid: undefined }, null), []);
  assert.equal(briefly({ ...holds, unsaid: [] }, null, AREAS), 'nothing');
  assert.deepEqual(comesTo(holds, ['board:read']), ['board:read']);
});

test('a cell says in a few words the areas something is held of, and how many', () => {
  assert.equal(briefly(holds, ['board:read'], AREAS), '1 area: board');
  assert.equal(briefly(holds, ['records', 'board:read', 'blueprints:run'], AREAS), '3 areas: board, blueprints, records');
  assert.equal(briefly(holds, ['*'], AREAS), 'everything');
  // What is written is said even where it is what an empty cell would stand for: the row says it.
  assert.equal(briefly(holds, ['board', 'records', 'blueprints:read'], AREAS), '3 areas: board, blueprints, records');
  // An area the factory does not list is counted and said as it is written.
  assert.equal(briefly(holds, ['board', 'emit:shop.sold'], AREAS), '2 areas: board, emit');
  // A daemon from before lists no areas.
  assert.equal(briefly(holds, ['board', 'records:read'], []), 'board, records:read');
});

test('what a row gives that somebody does not hold is theirs to see and not to change', () => {
  assert.deepEqual(lacking(['*'], ['admin', 'shell']), []);
  assert.deepEqual(lacking(['board', 'records', 'blueprints'], ['board:read', 'records', 'blueprints:read']), []);
  assert.deepEqual(lacking(['board:read', 'records'], ['board', 'records:write', 'admin']), ['board', 'admin']);
  assert.deepEqual(lacking([], comesTo(holds, null)), ['board', 'records', 'blueprints:read']);
});

test('a line changed writes the whole list, with what the row left unsaid written out', () => {
  // The row said nothing: it comes to the usual, and the one line is changed in that.
  assert.deepEqual(afterLine(holds, null, AREAS, 'board', ['read']), ['board:read', 'blueprints:read', 'records']);
  assert.deepEqual(afterLine(holds, [], AREAS, 'shell', ['all']), ['board', 'blueprints:read', 'records', 'shell']);
  // The row said something: that is what is changed, and the rest is as it was.
  assert.deepEqual(afterLine(holds, ['board:read', 'admin'], AREAS, 'board', ['read', 'write']), ['admin', 'board']);
  assert.deepEqual(afterLine(holds, ['board:read', 'admin'], AREAS, 'admin', []), ['board:read']);
  // Taken to nothing, the list is empty, which is the row saying nothing again.
  assert.deepEqual(afterLine(holds, ['board:read'], AREAS, 'board', []), []);
});

test('a whole list is the usual where it is what one usually holds, and is otherwise said by its areas', () => {
  const usual = ['board', 'records', 'blueprints:read'];
  assert.equal(brieflyHeld(usual, usual, AREAS), 'the usual');
  // In any order, and with a thing said twice.
  assert.equal(brieflyHeld(['blueprints:read', 'records', 'board', 'board'], usual, AREAS), 'the usual');
  // One thing more or less is no longer the usual.
  assert.equal(brieflyHeld([...usual, 'shell'], usual, AREAS), '4 areas: board, blueprints, records, shell');
  assert.equal(brieflyHeld(['board', 'records'], usual, AREAS), '2 areas: board, records');
  assert.equal(brieflyHeld(['board:read'], usual, AREAS), '1 area: board');
  assert.equal(brieflyHeld(['*'], usual, AREAS), 'everything');
  // A whole list that is empty holds nothing: there is no cell here for it to have left unsaid.
  assert.equal(brieflyHeld([], usual, AREAS), 'nothing');
  // A daemon from before says no usual and lists no areas: nothing is called the usual, and the scopes are said as written.
  assert.equal(brieflyHeld([], [], AREAS), 'nothing');
  assert.equal(brieflyHeld(['board'], [], AREAS), '1 area: board');
  assert.equal(brieflyHeld(['board', 'shell'], usual, []), 'board, shell');
});

import assert from 'node:assert/strict';
import { test } from 'node:test';
import { byHand, closedWhy, KEPT_SAID, KEPT_WORDS, keptSaid, offered } from './byhand.ts';

const every = { adds: true, changes: true, columns: true, readsIn: true, removes: true };

test('a table that says nothing, or says its rows are written by hand, is offered everything it always was', () => {
  for (const table of [{}, { by_hand: true }, { by_hand: null }, null, undefined]) {
    assert.equal(byHand(table), true, JSON.stringify(table));
    assert.deepEqual(offered(table), every, JSON.stringify(table));
    assert.equal(keptSaid(table), null, JSON.stringify(table));
    assert.equal(closedWhy(table), null, JSON.stringify(table));
  }
});

test('a table whose rows are not written by hand is offered no new row, no change of a row or of its columns and no CSV read in, and is still offered taking a row away', () => {
  const table = { by_hand: false };
  assert.equal(byHand(table), false);
  assert.deepEqual(offered(table), { adds: false, changes: false, columns: false, readsIn: false, removes: true });
});

test('a table whose rows are not written by hand says so at its head in a few words, and why where they are pointed at and where a field is closed', () => {
  const table = { by_hand: false };
  assert.deepEqual(keptSaid(table), { words: KEPT_WORDS, more: KEPT_SAID });
  assert.equal(closedWhy(table), KEPT_SAID);
  // A few words, and a sentence that names what writes the rows and what is not done here.
  assert.ok(KEPT_WORDS.split(' ').length <= 5, KEPT_WORDS);
  assert.match(KEPT_SAID, /commands and blueprints/);
  assert.match(KEPT_SAID, /nothing here adds a row/);
});

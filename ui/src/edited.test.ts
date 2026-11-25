import assert from 'node:assert/strict';
import { test } from 'node:test';
import { worn } from './clock.ts';
import { edited, lastOf, newest, saidOf, SLACK } from './edited.ts';

test('the newest record is the one changed last, and a table with none has none', () => {
  assert.equal(newest([]), null);
  assert.deepEqual(newest([{ id: 'a', updated: 5 }, { id: 'b', updated: 9 }, { id: 'c', updated: 7 }]), { id: 'b', updated: 9 });
});

test('a table was last changed when it or its newest record was, whichever is later', () => {
  assert.deepEqual(edited([100, 400], []), { at: 400, by: null });
  assert.deepEqual(edited([900, 400], []), { at: 900, by: null });
  assert.deepEqual(edited([undefined, 400], [null]), { at: 400, by: null });
});

test('nothing is said of when where nothing says, whatever line is known', () => {
  assert.equal(edited([], []), null);
  assert.equal(edited([undefined, null, 0], []), null);
  assert.equal(edited([], [{ who: 'Ada', ts: 700 }]), null);
});

test('who changed it is a line of the same moment as the change', () => {
  assert.deepEqual(edited([1000, 60_000], [{ who: 'Ada', ts: 60_010 }]), { at: 60_000, by: 'Ada' });
  assert.deepEqual(edited([60_000], [{ who: 'Ada', ts: 60_000 - SLACK }]), { at: 60_000, by: 'Ada' });
  assert.deepEqual(edited([60_000], [{ who: 'Ada', ts: 60_000 + SLACK }]), { at: 60_000, by: 'Ada' });
});

test('a change that wrote no line is nobody\'s to name', () => {
  // A view saved long after the last thing that happened to a record; a file another program wrote.
  assert.deepEqual(edited([90_000, 60_000], [{ who: 'Ada', ts: 60_000 }]), { at: 90_000, by: null });
  assert.deepEqual(edited([60_000], [{ who: 'Ada', ts: 60_000 - SLACK - 1 }]), { at: 60_000, by: null });
  assert.deepEqual(edited([60_000], [{ who: ' ', ts: 60_000 }]), { at: 60_000, by: null });
});

test('a line heard late, of something that happened long before, neither says when nor names anybody', () => {
  // A history filled in now, for a record last changed long ago: the line read of that change still names who made it.
  assert.deepEqual(edited([60_000], [{ who: 'Ada', ts: 60_005 }, { who: 'Bo', ts: 9_000_000 }]), { at: 60_000, by: 'Ada' });
  assert.deepEqual(edited([60_000], [{ who: 'Bo', ts: 9_000_000 }]), { at: 60_000, by: null });
});

test('of two lines of the moment the later names who', () => {
  assert.deepEqual(edited([60_000], [{ who: 'Ada', ts: 59_000 }, { who: 'Bo', ts: 60_001 }]), { at: 60_000, by: 'Bo' });
});

test('a history\'s last line is its last', () => {
  assert.equal(lastOf([]), null);
  assert.deepEqual(lastOf([{ who: 'a', ts: 1 }, { who: 'b', ts: 2 }]), { who: 'b', ts: 2 });
});

test('it is said short in a head and whole where there is room', () => {
  // In the reader's zone and as they have a time written: here UTC, on a clock of twelve hours.
  worn.value = { zone: 'UTC', hours: '12h' };
  const now = Date.UTC(2026, 10, 19, 15, 0);
  const at = Date.UTC(2026, 10, 19, 14, 2);
  assert.deepEqual(saidOf({ at, by: 'Ada' }, now), { short: 'Edited 2:02 PM UTC', whole: 'Last edited by Ada · 2:02 PM UTC' });
  assert.deepEqual(saidOf({ at, by: null }, now), { short: 'Edited 2:02 PM UTC', whole: 'Last edited 2:02 PM UTC' });
});

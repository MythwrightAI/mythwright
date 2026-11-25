import assert from 'node:assert/strict';
import { test } from 'node:test';
import { drainSaid, drainingOf, stoppedSaid, waitsSaid } from './drained.ts';

test('only a drain that says when it began and when it stops is one', () => {
  assert.equal(drainingOf(null), null);
  assert.equal(drainingOf(undefined), null);
  assert.equal(drainingOf({ since: 1 }), null);
  assert.equal(drainingOf({ since: '1', until: 2 }), null);
  assert.deepEqual(drainingOf({ since: 1, until: 2, turns: 3 }), { since: 1, until: 2 });
});

test("a drain is said with the latest it stops, in the reader's time", () => {
  const said = drainSaid({ since: 0, until: Date.UTC(2026, 10, 23, 15, 30) }, { zone: 'UTC', hours: '24h' });
  assert.match(said, /begins nothing new/);
  assert.match(said, /by 15:30 UTC at the latest/);
});

test('what a drain waits for is a line for each turn and each run, with what becomes of it', () => {
  const standing = {
    turns: [{ slot: 'builder', turn: 'turn-1', becomes: 'it is begun again by the next start' }],
    runs: [{ of: 'std/work', flow: 'main', run: 'run-1', becomes: 'the next start takes it up, as it does at any stop' }],
  };
  assert.deepEqual(waitsSaid(standing), [
    "builder's turn: if it is cut off, it is begun again by the next start.",
    'A run of std/work (main): if it is cut off, the next start takes it up, as it does at any stop.',
  ]);
  assert.deepEqual(waitsSaid({ turns: [], runs: [] }), ['Nothing is in hand.']);
  assert.deepEqual(waitsSaid(null), ['Nothing is in hand.']);
});

test('a stop is said only of a daemon that drained, as it came out', () => {
  assert.equal(stoppedSaid({}), null);
  assert.equal(stoppedSaid({ at: 'build' }), null);
  assert.match(stoppedSaid({ drained: true, cut: [] })![0]!, /drained and stopped: nothing was left in hand/);
  assert.deepEqual(stoppedSaid({ drained: false, cut: ['builder\'s turn turn-1: it is begun again by the next start', 3] }), [
    'The daemon stopped at its bound with this still in hand:',
    "builder's turn turn-1: it is begun again by the next start",
  ]);
});

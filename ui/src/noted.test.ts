import assert from 'node:assert/strict';
import { test } from 'node:test';
import { anyNoted, noteOf } from './noted.ts';

const WENT_ON = '`std/looked` cannot be applied here as it is now, so `wrong` went on with it as it was applied.';

test('one that failed says why as a failure', () => {
  assert.deepEqual(noteOf({ status: 'failed', error: 'the step `build` failed: exit 1' }), { says: 'the step `build` failed: exit 1', is: 'failure' });
});

test('one that went on past what it could not take says it as a warning, whatever it is doing now', () => {
  for (const status of ['active', 'starting', 'done', 'stopped'] as const) assert.deepEqual(noteOf({ status, error: WENT_ON }), { says: WENT_ON, is: 'warning' });
});

test('a row with nothing to say has no note, and nor has one that says only blanks', () => {
  assert.equal(noteOf({ status: 'active', error: null }), null);
  assert.equal(noteOf({ status: 'failed', error: '' }), null);
  assert.equal(noteOf({ status: 'failed', error: '  \n' }), null);
});

test('the column is drawn where any row has something to say', () => {
  assert.equal(anyNoted([]), false);
  assert.equal(anyNoted([{ status: 'active', error: null }]), false);
  assert.equal(anyNoted([{ status: 'active', error: null }, { status: 'active', error: WENT_ON }]), true);
});

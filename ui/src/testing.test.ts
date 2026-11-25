import assert from 'node:assert/strict';
import { test } from 'node:test';
import { SAYS, testingLines } from './testing.ts';
import type { Standing } from './testing.ts';

const checked: Standing = { reads: true, instance: false, checked: true, errors: 0, named: true, unpreviewed: null, previewing: false };

test('a text that checks offers four lines in one order, each with its word on what it does, none closed', () => {
  assert.deepEqual(testingLines(checked), [
    { label: 'Preview', note: SAYS.preview, does: 'preview' },
    { label: 'Sandbox test', note: SAYS.sandbox, does: 'sandbox' },
    { label: 'Side by side', note: SAYS.beside, does: 'beside' },
    { label: 'Compare on real work', note: SAYS.compare, does: 'compare' },
  ]);
});

test('before its text is checked, Preview and Compare on real work are offered and closed, saying why', () => {
  const lines = testingLines({ ...checked, checked: false, named: false });
  assert.deepEqual(
    lines.map((one) => [one.label, one.closed ?? null]),
    [
      ['Preview', 'Once its text has been checked.'],
      ['Sandbox test', null],
      ['Side by side', null],
      ['Compare on real work', 'Once its text checks.'],
    ],
  );
});

test('a text that does not check is not previewed, saying why in the check\'s words, and is not compared', () => {
  const lines = testingLines({ ...checked, errors: 1, unpreviewed: 'unknown action `nope`' });
  assert.equal(lines[0]!.closed, 'It is previewed once its text checks: unknown action nope');
  assert.equal(lines[3]!.closed, 'Once its text checks.');
  assert.equal(lines[1]!.closed, undefined);
});

test('a text with no name of its own is not compared, since a comparison is of a blueprint by its name', () => {
  assert.equal(testingLines({ ...checked, named: false })[3]!.closed, 'Once its text checks.');
});

test('while its preview shows, Preview says that it is pressed to put it away', () => {
  assert.equal(testingLines({ ...checked, previewing: true })[0]!.note, 'Showing beside the page: pressed, it is put away.');
});

test('something running is offered its preview alone', () => {
  assert.deepEqual(
    testingLines({ ...checked, instance: true }).map((one) => one.does),
    ['preview'],
  );
});

test('whoever may not read blueprints is offered only what compares, and something running nothing at all', () => {
  assert.deepEqual(
    testingLines({ ...checked, reads: false }).map((one) => one.does),
    ['compare'],
  );
  assert.deepEqual(testingLines({ ...checked, reads: false, instance: true }), []);
});

import assert from 'node:assert/strict';
import { test } from 'node:test';
import { sayToast, type Toast } from './toasted.ts';

const said = (id: number, text: string, undo?: () => void): Toast => ({ id, text, bad: false, undo });

test('a sentence said while it is showing takes the place of the one there, the newest last', () => {
  const first = said(1, 'Add a till is back.');
  const other = said(2, 'Opening hours: Moved to Trash, kept 30 days.');
  const again = said(3, 'Add a till is back.');
  assert.deepEqual(sayToast([first, other], again), [other, again]);
});

test('of two that each offer Undo, the one showing is the newest, which takes back what was done last', () => {
  const [one, two] = [() => {}, () => {}];
  const showing = sayToast([said(1, 'Add a till: Moved to Trash, kept 30 days.', one)], said(2, 'Add a till: Moved to Trash, kept 30 days.', two));
  assert.equal(showing.length, 1);
  assert.equal(showing[0]?.undo, two);
});

test('what says something else is said beside what is showing', () => {
  const showing = sayToast([said(1, 'Saved.')], said(2, 'Deleted.'));
  assert.deepEqual(showing.map((entry) => entry.text), ['Saved.', 'Deleted.']);
});

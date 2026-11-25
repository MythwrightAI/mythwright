import assert from 'node:assert/strict';
import { test } from 'node:test';
import { editAll, openOf, opened } from './topicediting.ts';

test('nothing is open to be written until it is opened', () => {
  assert.deepEqual(openOf({}, 'body'), { open: false, focus: undefined, at: null });
  assert.deepEqual(openOf({}, 'summary'), { open: false, focus: undefined, at: null });
});

test('a document opened by a press takes the cursor once, each press anew', () => {
  const once = opened({}, 'summary', true);
  assert.deepEqual(openOf(once, 'summary'), { open: true, focus: 1, at: null });
  assert.equal(openOf(once, 'body').focus, undefined, 'the other is not given the cursor');
  const twice = opened(opened(once, 'summary', false), 'summary', true);
  assert.equal(openOf(twice, 'summary').focus, 2, 'opened again, it is asked for anew');
});

test('a document left is closed, and the article left forgets the heading it was opened at', () => {
  const at = opened({}, 'body', true, 'rolling-back');
  assert.equal(openOf(at, 'body').at, 'rolling-back');
  const left = opened(at, 'body', false);
  assert.deepEqual(openOf(left, 'body'), { open: false, focus: 1, at: null });
  assert.equal(openOf(opened(at, 'summary', true), 'body').at, 'rolling-back', 'the summary opened leaves the article where it was');
});

test('Edit at the head opens the article and its summary where it has one; at a heading, the article alone', () => {
  const all = editAll({}, null, true);
  assert.equal(openOf(all, 'body').open && openOf(all, 'summary').open, true);
  assert.equal(openOf(all, 'body').focus, 1, 'the cursor in the article');
  assert.equal(openOf(editAll({}, null, false), 'summary').open, false, 'a summary that is not there is not opened');
  const at = editAll({}, 'Steps', true);
  assert.deepEqual(openOf(at, 'body'), { open: true, focus: 1, at: 'Steps' });
  assert.equal(openOf(at, 'summary').open, false);
  assert.equal(openOf(editAll({ summary: true }, 'Steps', true), 'summary').open, true, 'one open already stays open');
});

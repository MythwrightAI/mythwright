import assert from 'node:assert/strict';
import { test } from 'node:test';
import { appliedSaid, shownOnceApplied } from './applied.ts';

test('an apply that has answered shows the graph, or the text where it said something beside going ahead', () => {
  assert.equal(shownOnceApplied(0, false), 'node');
  assert.equal(shownOnceApplied(1, false), 'text');
  assert.equal(shownOnceApplied(3, false), 'text');
});

test('a view somebody pressed while the apply was being answered is left showing, whatever the apply said', () => {
  // The steps are drawn as done before the answer is in: whoever went on to the runs then stays there.
  assert.equal(shownOnceApplied(0, true), null);
  assert.equal(shownOnceApplied(2, true), null);
});

test('what is already running is said as the instance it is, by its blueprint and its key', () => {
  const same = { created: false, changed: false, blueprint: 'caller', key: 'night' };
  assert.equal(appliedSaid(same, 'caller', ''), 'caller · night is already running this; nothing to do.');
  assert.equal(appliedSaid(same, 'caller', ' for Shop'), 'caller · night is already running this for Shop; nothing to do.');
  // What the page last checked is not what is running: the daemon says which instance.
  assert.equal(appliedSaid(same, null, ''), 'caller · night is already running this; nothing to do.');
});

test('one started or updated is said by its blueprint, as it always was', () => {
  assert.equal(appliedSaid({ created: true, changed: true, blueprint: 'fleet', key: 'fleet' }, 'fleet', ''), 'Started fleet.');
  assert.equal(appliedSaid({ created: false, changed: true, blueprint: 'fleet', key: 'fleet' }, null, ' for Shop'), 'Updated fleet for Shop.');
});

test('a daemon from before that names no instance is said by what the page checked, and never as null', () => {
  assert.equal(appliedSaid({ created: false, changed: false }, 'caller', ''), 'caller is already running this; nothing to do.');
  assert.equal(appliedSaid({ created: false, changed: false }, null, ''), 'What is running is already running this; nothing to do.');
  assert.equal(appliedSaid({ created: true, changed: true }, null, ''), 'Started it.');
});

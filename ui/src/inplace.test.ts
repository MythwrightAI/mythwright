import assert from 'node:assert/strict';
import { test } from 'node:test';
import { KEPT_BEHIND, barShown, named, names, opensApart, parked } from './inplace.ts';
import type { Tab } from './tab.ts';

test('what was left is the latest of what was shown before, and what is shown is not among them', () => {
  assert.deepEqual(parked([], 'home', 'board:shop'), { behind: ['home'], gone: [] });
  assert.deepEqual(parked(['home'], 'board:shop', 'table:shop:notes'), { behind: ['board:shop', 'home'], gone: [] });
  // Back to the board: it is shown, and so no longer behind; what was left for it is.
  assert.deepEqual(parked(['board:shop', 'home'], 'table:shop:notes', 'board:shop'), { behind: ['table:shop:notes', 'home'], gone: [] });
});

test('a thing shown twice is behind once, where it was last left', () => {
  assert.deepEqual(parked(['a', 'b', 'c'], 'c', 'd'), { behind: ['c', 'a', 'b'], gone: [] });
});

test('no more than so many are kept as they were left, and what is further back is let go of', () => {
  const behind = Array.from({ length: KEPT_BEHIND }, (_, at) => `k${at}`);
  assert.deepEqual(parked(behind, 'new', 'shown'), { behind: ['new', ...behind.slice(0, KEPT_BEHIND - 1)], gone: [`k${KEPT_BEHIND - 1}`] });
  assert.deepEqual(parked(['a', 'b', 'c'], 'd', 'x', 2), { behind: ['d', 'a'], gone: ['b', 'c'] });
});

test('a press opens in a tab of the browser’s own with Ctrl or ⌘ held, or by the middle button, and in place otherwise', () => {
  const press = (button: number, ctrlKey = false, metaKey = false) => ({ button, ctrlKey, metaKey });
  assert.equal(opensApart(press(0)), false);
  assert.equal(opensApart(press(0, true)), true);
  assert.equal(opensApart(press(0, false, true)), true);
  assert.equal(opensApart(press(1)), true);
  // The other button opens the browser's own menu, and is no press that opens anything.
  assert.equal(opensApart(press(2, true)), false);
});

test('what a line says it opens is read back as that thing, and anything else as none', () => {
  for (const tab of [{ kind: 'board', project: 'shop' }, { kind: 'channel', id: 'c1' }, { kind: 'settings' }, { kind: 'file', project: 'shop', path: 'a "b".md' }] as Tab[]) {
    assert.deepEqual(named(names(tab)), tab);
  }
  for (const said of [null, undefined, '', 'board:shop', '{"project":"shop"}', '[1]', '7', '{']) assert.equal(named(said), null, String(said));
});

test('the bar over what is shown in a browser is there while the side panel is folded away, and always on a phone', () => {
  assert.equal(barShown(true, false), false);
  assert.equal(barShown(false, false), true);
  assert.equal(barShown(true, true), true);
  assert.equal(barShown(false, true), true);
});

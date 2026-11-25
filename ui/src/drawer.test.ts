import assert from 'node:assert/strict';
import { test } from 'node:test';
import { BESIDE_GAP, CLEAR, clearOfDrawer, drawerOf, fitDrawer, LEAST_BESIDE, LEAST_DRAWER, USUAL_DRAWER } from './drawer.ts';

test('the drawer is as wide as it was dragged to, within what a form can be filled in at and the workbench it lies over', () => {
  assert.equal(fitDrawer(520, 1200), 520);
  assert.equal(fitDrawer(40, 1200), LEAST_DRAWER, 'no narrower than a form can be filled in at');
  assert.equal(fitDrawer(5000, 1200), 1200 - CLEAR, 'no wider than the workbench, clear of its edges');
  assert.equal(fitDrawer(520, 500), 500 - CLEAR, 'a narrow window gives it what there is');
  assert.equal(fitDrawer(520, 200), 200 - CLEAR, 'and one narrower than its least gives it that');
  assert.equal(fitDrawer(520, 0), 0, 'never less than nothing');
});

test('the width that was kept is the width the drawer comes back with', () => {
  assert.equal(drawerOf('520'), 520);
  assert.equal(drawerOf(null), USUAL_DRAWER);
  assert.equal(drawerOf('wide'), USUAL_DRAWER);
  assert.equal(drawerOf('20'), USUAL_DRAWER, 'one narrower than it can be is not one it was left at');
  assert.equal(drawerOf('90000'), USUAL_DRAWER);
});

test('a page keeps clear as much of its right as an open drawer lies over, and the gap', () => {
  // A page across the whole workbench, from 340 to 1280, and a drawer from 870.
  assert.equal(clearOfDrawer(340, 1280, 870), 1280 - 870 + BESIDE_GAP);
  // A drawer a little over its right edge.
  assert.equal(clearOfDrawer(0, 900, 899.5), Math.ceil(900 - 899.5 + BESIDE_GAP));
});

test('a page keeps nothing clear with no drawer open, or one that does not reach it', () => {
  assert.equal(clearOfDrawer(340, 1280, null), 0);
  assert.equal(clearOfDrawer(0, 600, 610), 0);
  assert.equal(clearOfDrawer(0, 600, 600), 0);
});

test('where too little of a page would be left beside the drawer, the drawer lies over it as on a phone', () => {
  // A phone, where the drawer has the screen.
  assert.equal(clearOfDrawer(0, 390, 0), 0);
  assert.equal(clearOfDrawer(300, 900, 300 + BESIDE_GAP + LEAST_BESIDE - 1), 0);
  assert.ok(clearOfDrawer(300, 900, 300 + BESIDE_GAP + LEAST_BESIDE) > 0);
});

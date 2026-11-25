import assert from 'node:assert/strict';
import { test } from 'node:test';
import { fitSide, LEAST_SIDE, sideOf, USUAL_SIDE } from './side.ts';

test('the side panel is as wide as it was dragged to, within what can be read and what leaves the page room', () => {
  assert.equal(fitSide(320, 1600), 320);
  assert.equal(fitSide(40, 1600), LEAST_SIDE, 'no narrower than a name can be read in');
  assert.equal(fitSide(900, 1600), 560, 'no wider than it is ever useful');
  assert.equal(fitSide(500, 800), 336, 'the page keeps its share of a small window');
  assert.equal(fitSide(300, 500), LEAST_SIDE, 'in a window with no room it is as narrow as it goes');
});

test('the width that was kept is the width it comes back with', () => {
  assert.equal(sideOf('310'), 310);
  assert.equal(sideOf(null), USUAL_SIDE);
  assert.equal(sideOf('wide'), USUAL_SIDE);
  assert.equal(sideOf('20'), USUAL_SIDE);
  assert.equal(sideOf('5000'), USUAL_SIDE);
});

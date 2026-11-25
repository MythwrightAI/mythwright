import assert from 'node:assert/strict';
import { test } from 'node:test';
import { heldIn } from './heldin.ts';

test('what an agent has in hand is told from the conversation its turn is part of', () => {
  assert.deepEqual(heldIn('lane:shop-4'), { card: 'shop-4', how: 'works' });
  assert.deepEqual(heldIn('plan:shop-4'), { card: 'shop-4', how: 'plans' });
  assert.deepEqual(heldIn('review:shop-4:1'), { card: 'shop-4', how: 'reviews' });
  assert.deepEqual(heldIn('chain:shop-4'), { card: 'shop-4', how: 'reviews' });
  // One of several pieces of work on it at once works it; its judges look at it, alone or seated.
  assert.deepEqual(heldIn('contend:shop-4:otter'), { card: 'shop-4', how: 'works' });
  assert.deepEqual(heldIn('judge:shop-4'), { card: 'shop-4', how: 'reviews' });
  assert.deepEqual(heldIn('judge:shop-4:2'), { card: 'shop-4', how: 'reviews' });
  // An answer in a channel is no card's, and neither is a turn that says nothing of where it belongs.
  assert.equal(heldIn('ch_1'), null);
  assert.equal(heldIn('feature:shop-features-1'), null);
  assert.equal(heldIn(null), null);
  assert.equal(heldIn('lane:'), null);
});

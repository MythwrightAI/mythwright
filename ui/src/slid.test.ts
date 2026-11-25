import assert from 'node:assert/strict';
import { test } from 'node:test';
import { arrangementOf, leaves, moved } from './slid.ts';
import type { Box } from './slid.ts';

const box = (left: number, top: number, width = 200, height = 60): Box => ({ left, top, width, height });
const boxes = (pairs: [string, Box][]) => new Map(pairs);

test('a card that changed columns moved by how far it was from where it is now', () => {
  const before = boxes([['shop-1', box(20, 100)], ['shop-2', box(20, 170)], ['shop-3', box(240, 100)]]);
  const after = boxes([['shop-1', box(240, 170)], ['shop-2', box(20, 100)], ['shop-3', box(240, 100)]]);
  assert.deepEqual(moved(before, after), [
    { id: 'shop-1', x: -220, y: -70 },
    // The card it left closes up.
    { id: 'shop-2', x: 0, y: 70 },
  ]);
});

test('nothing moved is nothing, and less than a pixel is nothing', () => {
  const same = boxes([['a', box(0, 0)]]);
  assert.deepEqual(moved(same, same), []);
  assert.deepEqual(moved(same, boxes([['a', box(0.4, 0.6)]])), []);
  assert.deepEqual(moved(new Map(), new Map()), []);
});

test('a card on the page only once goes at once: added, taken away, in a column folded or a view held to fewer', () => {
  assert.deepEqual(moved(boxes([['a', box(0, 0)]]), boxes([['b', box(0, 80)]])), []);
});

test('a card out of sight either time goes at once', () => {
  const view = box(0, 0, 1000, 600);
  assert.deepEqual(moved(boxes([['a', box(20, 100)]]), boxes([['a', box(240, 900)]]), view), [], 'into a column scrolled far');
  assert.deepEqual(moved(boxes([['a', box(20, -400)]]), boxes([['a', box(240, 100)]]), view), [], 'from far');
  assert.deepEqual(moved(boxes([['a', box(20, 100)]]), boxes([['a', box(240, 550)]]), view), [{ id: 'a', x: -220, y: -450 }], 'partly in sight is in sight');
});

test('nothing under the pointer moves: a card it is on, or one coming to where it is', () => {
  const before = boxes([['a', box(20, 100)], ['b', box(20, 170)]]);
  const after = boxes([['a', box(240, 100)], ['b', box(20, 100)]]);
  assert.deepEqual(moved(before, after, null, { x: 30, y: 110 }), [], 'a leaves from under it, b comes to it');
  assert.deepEqual(moved(before, after, null, { x: 30, y: 190 }), [{ id: 'a', x: -220, y: 0 }], 'b leaves from under it');
  assert.deepEqual(moved(before, after, null, { x: 900, y: 900 }), [{ id: 'a', x: -220, y: 0 }, { id: 'b', x: 0, y: 70 }]);
});

test('the arrangement of the cards is one word, another for any card elsewhere or in another order', () => {
  const one = arrangementOf([['ready', ['a', 'b']], ['doing', []]]);
  assert.equal(one, 'ready:a,b\ndoing:');
  assert.notEqual(one, arrangementOf([['ready', ['b', 'a']], ['doing', []]]));
  assert.notEqual(one, arrangementOf([['ready', ['a']], ['doing', ['b']]]));
});

test('a card is flown over the page where where it was is outside the list it is in now', () => {
  const list = box(240, 80, 220, 500);
  assert.equal(leaves(box(250, 300), list), false, 'within its column');
  assert.equal(leaves(box(20, 300), list), true, 'from another column');
  assert.equal(leaves(box(250, 560), list), true, 'from below what the list shows');
});

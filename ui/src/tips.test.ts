import assert from 'node:assert/strict';
import { test } from 'node:test';
import { place, placeBeside } from './tips.ts';

const within = { width: 400, height: 300 };
const tip = { width: 200, height: 30 };

test('a tip goes under what it is about, centred on it', () => {
  const over = { left: 150, top: 40, width: 100, height: 28 };
  assert.deepEqual(place(over, tip, within), { left: 100, top: 74 });
});

test('a tip about something at the edge of the window is moved in until all of it shows', () => {
  const right = { left: 300, top: 40, width: 90, height: 28 };
  assert.equal(place(right, tip, within).left, 192, 'its right side stops short of the window\'s');
  const left = { left: 4, top: 40, width: 36, height: 28 };
  assert.equal(place(left, tip, within).left, 8);
});

test('a tip with no room under what it is about goes over it', () => {
  const low = { left: 150, top: 262, width: 100, height: 28 };
  assert.equal(place(low, tip, within).top, 226);
});

test('a tip with room neither under nor over stays under', () => {
  const tall = { left: 150, top: 20, width: 100, height: 270 };
  assert.equal(place(tall, tip, within).top, 296);
});

test('a tip about a line of a list that says its tips beside itself goes beside the list, level with the line', () => {
  const wide = { width: 1280, height: 800 };
  const list = { left: 44, top: 100, width: 252, height: 600 };
  const line = { left: 50, top: 114, width: 239, height: 66 };
  const tip = { width: 320, height: 44 };
  assert.deepEqual(placeBeside(list, line, tip, wide), { left: 302, top: 114 });
});

test('a tip beside a list goes on its left where there is no room on its right, and under the line where neither has room', () => {
  const tip = { width: 200, height: 40 };
  const line = { left: 610, top: 50, width: 180, height: 30 };
  assert.deepEqual(placeBeside({ left: 600, top: 0, width: 200, height: 300 }, line, tip, { width: 900, height: 300 }), { left: 394, top: 50 });
  // A phone's width: the list is the screen, and the tip is under the line, inside the window.
  const phone = { width: 390, height: 800 };
  const at = placeBeside({ left: 0, top: 0, width: 390, height: 800 }, { left: 0, top: 50, width: 390, height: 60 }, tip, phone);
  assert.deepEqual(at, place({ left: 0, top: 50, width: 390, height: 60 }, tip, phone));
  assert.ok(at.left >= 8 && at.left + tip.width <= 390 - 8);
});

test('a tip beside a list is moved up to be whole in the window', () => {
  const at = placeBeside({ left: 44, top: 0, width: 252, height: 800 }, { left: 50, top: 780, width: 239, height: 30 }, { width: 300, height: 60 }, { width: 1280, height: 800 });
  assert.deepEqual(at, { left: 302, top: 732 });
});

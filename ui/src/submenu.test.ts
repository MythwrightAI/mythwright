import assert from 'node:assert/strict';
import { test } from 'node:test';
import { besideAt, towards } from './submenu.ts';

const box = (left: number, top: number, width: number, height: number) => ({ left, top, right: left + width, bottom: top + height });
const window = { width: 1350, height: 860 };
const size = { width: 200, height: 120 };

test('it is put on the right of the menu, its first line level with the line that opened it', () => {
  assert.deepEqual(besideAt(box(100, 200, 232, 150), box(105, 240, 222, 28), size, window), { left: 334, top: 235, most: 852, side: 'right' });
});

test('where the right has no room it is put on the left', () => {
  assert.deepEqual(besideAt(box(1100, 200, 232, 150), box(1105, 240, 222, 28), size, window), { left: 898, top: 235, most: 852, side: 'left' });
});

test('near the foot of the window it is moved up, to be whole on the screen', () => {
  assert.deepEqual(besideAt(box(100, 700, 232, 150), box(105, 800, 222, 28), size, window), { left: 334, top: 736, most: 852, side: 'right' });
});

test('where neither side has room, as at a phone\'s width, it lies over the menu under its line', () => {
  const phone = { width: 390, height: 844 };
  assert.deepEqual(besideAt(box(154, 300, 232, 150), box(159, 340, 222, 28), size, phone), { left: 170, top: 370, most: 470, side: 'over' });
});

test('over the menu with too little room under its line, it is moved up over the line, on the screen', () => {
  const phone = { width: 390, height: 844 };
  assert.deepEqual(besideAt(box(154, 700, 232, 140), box(159, 760, 222, 28), size, phone), { left: 170, top: 720, most: 836, side: 'over' });
});

test('one taller than the window is as tall as the window allows, and scrolls in itself', () => {
  const tall = besideAt(box(100, 200, 232, 150), box(105, 240, 222, 28), { width: 200, height: 2000 }, window);
  assert.deepEqual(tall, { left: 334, top: 4, most: 852, side: 'right' });
});

test('it is never put past the window\'s left on a screen narrower than itself', () => {
  const at = besideAt(box(4, 300, 232, 150), box(9, 340, 222, 28), { width: 300, height: 100 }, { width: 240, height: 600 });
  assert.equal(at.left, 4);
  assert.equal(at.side, 'over');
});

test('the pointer moving from the line that opened it towards the menu beside it is on its way, across other lines too', () => {
  const beside = box(334, 235, 200, 120);
  // From the line, down and to the right, over the line under it.
  assert.equal(towards({ x: 250, y: 254 }, { x: 270, y: 270 }, beside, 'right'), true);
  // Straight across.
  assert.equal(towards({ x: 250, y: 254 }, { x: 290, y: 254 }, beside, 'right'), true);
  // Down the menu and away from it is not.
  assert.equal(towards({ x: 250, y: 254 }, { x: 250, y: 300 }, beside, 'right'), false);
  assert.equal(towards({ x: 250, y: 254 }, { x: 230, y: 270 }, beside, 'right'), false);
  // Below where the menu beside ends.
  assert.equal(towards({ x: 250, y: 254 }, { x: 260, y: 420 }, beside, 'right'), false);
});

test('to a menu on the left the way goes left, and to one over the menu there is none', () => {
  const beside = box(898, 235, 200, 120);
  assert.equal(towards({ x: 1200, y: 254 }, { x: 1180, y: 270 }, beside, 'left'), true);
  assert.equal(towards({ x: 1200, y: 254 }, { x: 1220, y: 270 }, beside, 'left'), false);
  assert.equal(towards({ x: 200, y: 254 }, { x: 210, y: 260 }, box(170, 370, 200, 120), 'over'), false);
});

import assert from 'node:assert/strict';
import { test } from 'node:test';
import { dock, group } from './panes.ts';
import { inView, oneRow, placesFor } from './phone.ts';

const one = group('g1', ['home', 'board', 'activity'], 'board');

test('a single group is the one row it already was', () => {
  assert.deepEqual(oneRow(one, 'g1'), one);
});

test('groups side by side become one row, in their order, showing what the group last used shows', () => {
  const two = dock(one, 'activity', 'g1', 'right', 'g2');
  assert.deepEqual(oneRow(two, 'g2'), group('g2', ['home', 'board', 'activity'], 'activity'));
  assert.deepEqual(oneRow(two, 'g1'), group('g1', ['home', 'board', 'activity'], 'board'));
});

test('a group that is no longer there is not the one last used: the first is', () => {
  const two = dock(one, 'activity', 'g1', 'bottom', 'g2');
  assert.deepEqual(oneRow(two, 'gone'), group('g1', ['home', 'board', 'activity'], 'board'));
});

test('making one row of the groups leaves them as they were', () => {
  const two = dock(one, 'activity', 'g1', 'right', 'g2');
  const before = JSON.stringify(two);
  oneRow(two, 'g2');
  assert.equal(JSON.stringify(two), before);
});

test('a card can be sent to any column of its board', () => {
  assert.deepEqual(placesFor(['backlog', 'doing', 'done'], 'doing'), ['backlog', 'doing', 'done']);
});

test('a card in a column the board no longer has is still said to be in it', () => {
  assert.deepEqual(placesFor(['backlog', 'done'], 'blocked'), ['blocked', 'backlog', 'done']);
});

test('a row is not scrolled for something already seen whole', () => {
  assert.equal(inView(40, 300, 60, 180), 40);
  assert.equal(inView(0, 300, 0, 300), 0);
});

test('a row is scrolled the least that brings something in', () => {
  assert.equal(inView(0, 300, 280, 400), 100, 'past the right-hand edge');
  assert.equal(inView(200, 300, 120, 240), 120, 'before the left-hand edge');
});

test('something wider than the row is shown from its start', () => {
  assert.equal(inView(0, 100, 150, 400), 150);
});

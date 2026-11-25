import assert from 'node:assert/strict';
import { test } from 'node:test';
import { handedOf, headingOf, saidOf } from './handed.ts';
import { forgetOpen, openIn, turnIn } from './handedopen.ts';

test('the heading says in words how many it was applied with, and there is none for nothing', () => {
  assert.equal(headingOf({ project: 'shop', columns: [] }), 'Applied with 2 params');
  assert.equal(headingOf({ project: 'shop' }), 'Applied with 1 param');
  assert.equal(headingOf({}), null);
});

test('a value that holds nothing more is said as it is written', () => {
  assert.equal(saidOf('shop'), 'shop');
  assert.equal(saidOf(''), '""');
  assert.equal(saidOf(3), '3');
  assert.equal(saidOf(false), 'false');
  assert.equal(saidOf(null), 'null');
});

test('each param is a name and its value, in the order handed, and a mapping or a list says how much it holds with what is in it under it', () => {
  assert.deepEqual(handedOf({ project: 'shop', columns: ['backlog', 'done'], review: { seats: 2, needed: 1 }, none: [], empty: {} }), [
    { name: 'project', path: 'project', said: 'shop' },
    {
      name: 'columns',
      path: 'columns',
      holds: 'a list of 2',
      under: [
        { name: '#1', path: 'columns/0', said: 'backlog' },
        { name: '#2', path: 'columns/1', said: 'done' },
      ],
    },
    {
      name: 'review',
      path: 'review',
      holds: '2 names',
      under: [
        { name: 'seats', path: 'review/seats', said: '2' },
        { name: 'needed', path: 'review/needed', said: '1' },
      ],
    },
    { name: 'none', path: 'none', holds: 'an empty list', under: [] },
    { name: 'empty', path: 'empty', holds: 'nothing in it', under: [] },
  ]);
});

test('what is inside is opened a level at a time, by where it is', () => {
  const [rows] = handedOf({ stages: [{ name: 'doing', does: ['work'] }] });
  assert.equal(rows!.under![0]!.path, 'stages/0');
  assert.deepEqual(rows!.under![0]!.under![1], { name: 'does', path: 'stages/0/does', holds: 'a list of 1', under: [{ name: '#1', path: 'stages/0/does/0', said: 'work' }] });
});

test('what is unfolded is kept by its tab, folded where nothing was kept, and let go with the tab', () => {
  forgetOpen();
  assert.equal(openIn('blueprint:std/board').size, 0);
  turnIn('blueprint:std/board', '');
  turnIn('blueprint:std/board', 'columns');
  assert.deepEqual([...openIn('blueprint:std/board')], ['', 'columns']);
  assert.equal(openIn('blueprint:std/fleet').size, 0);
  turnIn('blueprint:std/board', 'columns');
  assert.deepEqual([...openIn('blueprint:std/board')], ['']);
  forgetOpen('blueprint:std/board');
  assert.equal(openIn('blueprint:std/board').size, 0);
});

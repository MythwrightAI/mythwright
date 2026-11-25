import assert from 'node:assert/strict';
import { test } from 'node:test';
import { addressed, withIds } from './rowsapi.ts';

test('a window is asked by the view, the sort, the band and where it is read from, each said once and in one order', () => {
  assert.equal(addressed({}), '');
  assert.equal(addressed({ view: 'All runs', sort: { field: 'title', down: true }, from: { after: 'ab12' }, limit: 100 }), '?view=All%20runs&sort=title&down=1&after=ab12&limit=100');
  assert.equal(addressed({ sort: { field: 'points', down: false }, from: { at: 4200 } }), '?sort=points&at=4200');
  assert.equal(addressed({ from: { before: 'cd' } }), '?before=cd');
  // From the start says nothing of where.
  assert.equal(addressed({ view: 'v', from: { start: true } }), '?view=v');
});

test('a band of the rows that say nothing is said as nothing, and is a band all the same', () => {
  assert.equal(addressed({ band: '' }), '?band=');
  assert.equal(addressed({ band: 'in review' }), '?band=in%20review');
  assert.equal(addressed({ band: null }), '');
});

test('figures are a column and what is counted of it, by commas', () => {
  assert.equal(addressed({ view: 'v' }, { points: 'sum', due: 'filled' }), '?view=v&figures=points%3Asum%2Cdue%3Afilled');
});

test('what is searched for and a row gone to are said too, and rows asked by id after what the view is', () => {
  assert.equal(addressed({ find: 'ab 12', from: { around: 'shop-runs-7' } }), '?find=ab%2012&around=shop-runs-7');
  assert.equal(addressed({ find: '' }), '');
  assert.equal(withIds({}, ['a-1', 'a-2']), '?ids=a-1,a-2');
  assert.equal(withIds({ view: 'v', find: 'x' }, ['a b']), '?view=v&find=x&ids=a%20b');
});

import assert from 'node:assert/strict';
import { test } from 'node:test';
import { FIRST, labelsSaid, measureSaid, noRows, rowsHead, runAddress, runsHead, secondsSaid, showing, tabsOf, tookSaid, wentSaid } from './toldread.ts';
import type { RowsPart } from './toldread.ts';

test('the parts with no tab are on a first tab, and the rest on tabs in the order each is first said', () => {
  const tabs = tabsOf([{ tab: 'To test' }, {}, { tab: 'Links' }, { tab: 'To test' }, { tab: '' }]);
  assert.deepEqual(tabs, [
    { name: '', called: FIRST, parts: [1, 4] },
    { name: 'To test', called: 'To test', parts: [0, 3] },
    { name: 'Links', called: 'Links', parts: [2] },
  ]);
  assert.deepEqual(tabsOf([{}, {}]), [{ name: '', called: FIRST, parts: [0, 1] }], 'one tab: no row of tabs is drawn');
  assert.deepEqual(tabsOf([{ tab: 'Links' }, { tab: 'Insights' }]).map((tab) => tab.name), ['Links', 'Insights'], 'no first tab where every part says one');
  assert.deepEqual(tabsOf([]), []);
});

test('the tab showing is the one chosen while it is there, and the first otherwise', () => {
  const tabs = tabsOf([{ tab: 'To test' }, { tab: 'Links' }]);
  assert.equal(showing(tabs, 'Links')?.name, 'Links');
  assert.equal(showing(tabs, 'Gone')?.name, 'To test');
  assert.equal(showing(tabs, null)?.name, 'To test');
  assert.equal(showing([], 'Links'), null);
});

test('a part of rows says what it is of, how many more it chose than it lists, and the columns its table has not got', () => {
  const part: RowsPart = { kind: 'rows', project: 'shop', table: 'test-runs', saying: { commit: 'a1' } };
  assert.deepEqual(rowsHead(part), { title: 'Rows of test-runs', note: null });
  assert.deepEqual(rowsHead({ ...part, more: 1, unknown: ['lost'] }), {
    title: 'Rows of test-runs',
    note: '1 more row says the same and is not listed: the table has every one. test-runs has no column called lost.',
  });
  assert.equal(rowsHead({ ...part, more: 30 }).note, '30 more rows say the same and are not listed: the table has every one.');
  assert.equal(noRows(part), 'No row says that now.');
  assert.equal(noRows({ ...part, saying: null, ids: ['shop-test-runs-1'] }), 'None of these rows is there now.');
});

test('a cell of seconds is a length of time, and one that says no number says nothing', () => {
  const cases: [unknown, string | null][] = [
    [96, '1m 36s'],
    [3700, '1h 01m'],
    [4, '4.0s'],
    ['12', '12s'],
    ['', null],
    [null, null],
    ['soon', null],
  ];
  for (const [value, said] of cases) assert.equal(secondsSaid(value), said, String(value));
});

test('a run is said by how it went and how long it took, and opens its page by its address', () => {
  assert.deepEqual(['ok', 'failed', 'cancelled', 'running', 'held'].map(wentSaid), ['went well', 'failed', 'was cancelled', 'running', 'held']);
  assert.equal(tookSaid({ id: 'r', called: 'preview', status: 'ok', started: 1_000, finished: 97_000 }), '1m 36s');
  assert.equal(tookSaid({ id: 'r', called: 'preview', status: 'running', started: 1_000, finished: null }), 'still running');
  assert.equal(runAddress('run_1 2'), '/run/run_1%202');
  assert.equal(runsHead({ kind: 'runs', project: 'shop', key: 'deploy-dev', most: 10 }), 'Runs of deploy-dev');
  assert.equal(runsHead({ kind: 'runs', project: 'shop', environment: 'dev', most: 10 }), 'Runs under dev');
  assert.equal(runsHead({ kind: 'runs', project: 'shop', instance: 'in_1', most: 10 }), 'Its runs');
});

test('a measurement is its latest value beside the median of its last few, written as what it is of', () => {
  const time = { name: 'run', unit: null, measured: { latest: 96_000, at: 0, median: 40_000, of: 5 } };
  assert.deepEqual(measureSaid(time), { latest: '1m 36s', median: '40s, the median of its last 5' });
  const cost = { name: 'agent.cost_usd', unit: 'usd', measured: { latest: 0.5, at: 0, median: 0.25, of: 1 } };
  assert.deepEqual(measureSaid(cost), { latest: '$0.50', median: '$0.25, the median of its last one' });
  assert.equal(measureSaid({ name: 'run', unit: null, measured: null }), null, 'nothing measured, nothing said');
  assert.equal(labelsSaid({ blueprint: 'deploy', project: 'shop' }), 'Measured with blueprint deploy, project shop');
  assert.equal(labelsSaid({}), null);
});

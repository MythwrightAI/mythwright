import assert from 'node:assert/strict';
import { test } from 'node:test';
import { decidedSaid, headOf, offered, rowsOf, standing, valueSaid, wholeSaid } from './asked.ts';
import type { Change } from './asked.ts';
import { partsOf } from './reviewed.ts';

function change(over: Partial<Change> = {}): Change {
  return {
    ask: 'ask_1',
    project: 'shop',
    record: 'shop-agents-2',
    collection: 'agents',
    title: 'reviewer',
    asker: { actor: 'agent:shop-worker-1', name: 'shop-worker-1' },
    why: 'it runs the build',
    cells: [{ name: 'wall', before: null, now: null, after: 'off', takes: [{ scope: 'shell', held: true }] }],
    status: 'waiting',
    decided: null,
    at: 1,
    may: true,
    ...over,
  };
}

test('a change is a part the page draws', () => {
  const read = { parts: [{ kind: 'change', project: 'shop', change: change() }, { kind: 'pictures' }] };
  assert.deepEqual(
    partsOf(read).map((part) => part.kind),
    ['change'],
  );
});

test('a value is said as words, nothing as a dash, and anything else as it is written', () => {
  assert.equal(valueSaid('off'), 'off');
  assert.equal(valueSaid(null), '—');
  assert.equal(valueSaid(''), '—');
  assert.equal(valueSaid(['shell', 'board']), 'shell, board');
  assert.equal(valueSaid(2), '2');
  assert.equal(valueSaid({ a: 1 }), '{"a":1}');
});

test('a cell says what it holds now only where that is not what it held when it was asked', () => {
  assert.deepEqual(rowsOf(change()), [{ name: 'wall', was: '—', now: null, to: 'off', takes: [{ scope: 'shell', held: true }] }]);
  const moved = change({ cells: [{ name: 'count', before: 1, now: 3, after: 2, takes: [] }] });
  assert.deepEqual(rowsOf(moved)[0], { name: 'count', was: '1', now: '3', to: '2', takes: [] });
});

test('a part is headed by whose change of which record, by what the record is called where it is', () => {
  assert.equal(headOf(change()), 'shop-worker-1 asks to change shop-agents-2 (reviewer) of agents');
  assert.equal(headOf(change({ title: null })), 'shop-worker-1 asks to change shop-agents-2 of agents');
  assert.equal(headOf(change({ title: ' ' })), 'shop-worker-1 asks to change shop-agents-2 of agents');
});

test('how it stands is said for each way it can', () => {
  const by = { by: 'user:ada', name: 'Ada', at: 2, why: '' };
  assert.equal(standing(change()), 'It waits for somebody who could make it to decide.');
  assert.equal(standing(change({ status: 'approved', decided: by })), 'Ada approved it, and it is made.');
  assert.equal(standing(change({ status: 'declined', decided: { ...by, why: ' not today ' } })), 'Ada declined it: not today.');
  assert.equal(standing(change({ status: 'declined', decided: by })), 'Ada declined it.');
  assert.equal(standing(change({ status: 'stale' })), 'It was not made: the record was changed since it was asked.');
  assert.equal(standing(change({ status: 'withdrawn' })), 'Whoever asked withdrew it.');
  const gone = { by: 'system', name: '', at: 2, why: 'shop-worker-1 was taken away' };
  assert.equal(standing(change({ status: 'withdrawn', decided: gone })), 'It was withdrawn: shop-worker-1 was taken away.');
  assert.equal(standing(change({ status: 'expired' })), 'It was not decided in seven days and goes no further.');
});

test('Approve and Decline are offered only where it waits and the daemon says the reader may', () => {
  assert.deepEqual(offered(change()), { decide: true, why: null });
  assert.deepEqual(offered(change({ may: false, why_not: 'needs `shell`' })), { decide: false, why: 'needs `shell`' });
  assert.deepEqual(offered(change({ may: false })), { decide: false, why: null });
  assert.deepEqual(offered(change({ status: 'approved' })), { decide: false, why: null });
});

test('what an approval came to is said once it is answered, and anything else by how it stands alone', () => {
  assert.equal(decidedSaid(change({ status: 'approved' })), "Approved: shop-agents-2 is changed, as shop-worker-1's.");
  assert.equal(decidedSaid(change({ status: 'declined' })), null);
  assert.equal(decidedSaid(change({ status: 'stale' })), null);
});

test('what changing the record at all takes is said only where the reader does not hold all of it', () => {
  assert.equal(wholeSaid(change()), null);
  assert.equal(wholeSaid(change({ takes: [{ scope: 'board:write', held: true }] })), null);
  assert.equal(
    wholeSaid(change({ takes: [{ scope: 'board:write', held: false }, { scope: 'records:read', held: true }] })),
    'Changing it at all takes board:write (not yours), records:read.',
  );
});

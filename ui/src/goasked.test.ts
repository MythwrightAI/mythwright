import assert from 'node:assert/strict';
import { test } from 'node:test';
import { goDecidedSaid, goHead, goOffered, goStanding, left, runAsks } from './goasked.ts';
import type { Go } from './goasked.ts';
import { partsOf } from './reviewed.ts';

function go(over: Partial<Go> = {}): Go {
  return {
    ask: 'ask_1',
    kind: 'go',
    project: 'shop',
    run: 'run_1',
    step: 'approve',
    instance: 'in_1',
    blueprint: 'pipelines/deploy',
    says: 'Ship the build?',
    environment: '',
    links: [],
    until: 3 * 86_400_000,
    asker: { actor: 'user:ada', name: 'Ada' },
    status: 'waiting',
    decided: null,
    at: 0,
    may: true,
    ...over,
  };
}

test('a run that waits for a go-ahead is a part the page draws', () => {
  const read = { parts: [{ kind: 'go', project: 'shop', go: go() }, { kind: 'pictures' }] };
  assert.deepEqual(
    partsOf(read).map((part) => part.kind),
    ['go'],
  );
});

test('the part is headed with whose run of what, and the environment it is to go under where it names one', () => {
  assert.equal(goHead(go()), "Ada's run of pipelines/deploy waits for a go-ahead");
  assert.equal(goHead(go({ environment: 'prod' })), "Ada's run of pipelines/deploy waits for a go-ahead, to go under prod");
});

test('how long is left is said in the largest whole unit', () => {
  for (const [ms, said] of [
    [0, 'under a minute'],
    [59_999, 'under a minute'],
    [60_000, '1 minute'],
    [59 * 60_000, '59 minutes'],
    [60 * 60_000, '1 hour'],
    [47 * 3_600_000, '47 hours'],
    [48 * 3_600_000, '2 days'],
    [7 * 86_400_000, '7 days'],
  ] as const) {
    assert.equal(left(ms), said, `${ms}`);
  }
});

test('how it stands says what waits and until when, or what it came to and by whom', () => {
  assert.equal(goStanding(go(), 86_400_000), 'It waits for somebody who may let it go on, for 2 days more.');
  const decided = (status: Go['status'], why = '') => go({ status, decided: { by: 'user:joan', name: 'Joan', at: 1, why } });
  assert.equal(goStanding(decided('approved'), 0), 'Joan approved it, and the run goes on.');
  assert.equal(goStanding(decided('declined', ' not today '), 0), 'Joan declined to let it go on: not today.');
  assert.equal(goStanding(decided('declined'), 0), 'Joan declined to let it go on.');
  assert.equal(goStanding(go({ status: 'expired' }), 0), 'Nobody let it go on in the time it waits, and it goes no further.');
});

test('Approve and Decline are offered only where the daemon says the reader may, and why not where it says', () => {
  assert.deepEqual(goOffered(go()), { decide: true, why: null });
  assert.deepEqual(goOffered(go({ may: false, why_not: 'you started this run' })), { decide: false, why: 'you started this run' });
  assert.deepEqual(goOffered(go({ may: false })), { decide: false, why: null });
  assert.deepEqual(goOffered(go({ status: 'approved', may: true })), { decide: false, why: null });
});

test('what a decision came to is said once the daemon answers', () => {
  assert.equal(goDecidedSaid(go({ status: 'approved' })), 'Approved: the run of pipelines/deploy goes on.');
  assert.equal(goDecidedSaid(go({ status: 'declined' })), 'Declined: the run of pipelines/deploy goes no further.');
  assert.equal(goDecidedSaid(go()), null);
  assert.equal(goDecidedSaid(go({ status: 'expired' })), null);
});

test("a run's asks are those that wait first and then the newest, and nothing that is not one", () => {
  const read = {
    asked: [go({ ask: 'a', status: 'declined', at: 1 }), go({ ask: 'b', at: 2 }), go({ ask: 'c', status: 'approved', at: 3 }), { kind: 'cells', at: 9 }],
  };
  assert.deepEqual(
    runAsks(read).map((one) => one.ask),
    ['b', 'c', 'a'],
  );
  assert.deepEqual(runAsks(null), []);
});

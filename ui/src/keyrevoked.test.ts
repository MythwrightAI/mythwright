import assert from 'node:assert/strict';
import { test } from 'node:test';
import { type Revoking, goesWith, revokedSaid } from './keyrevoked.ts';

const found = (more: Partial<Revoking>): Revoking => ({ name: 'ci', keys: [], running: [], ...more });
const fleet = { id: 'in_1', blueprint: 'std/fleet', key: 'fleet', project: 'shop' };

test('what goes with a key is each key made with it and what it applied, stopped or left running', () => {
  const cases: [Revoking, boolean, string[]][] = [
    [found({}), false, []],
    [found({ running: [fleet] }), false, ['std/fleet in shop (in_1), stopped']],
    [found({ running: [fleet] }), true, ['std/fleet in shop (in_1), left running']],
    [
      found({ keys: [{ id: 'cred_2', name: 'nightly' }], running: [{ ...fleet, project: null }] }),
      false,
      ['the key nightly, which was made with it', 'std/fleet (in_1), stopped'],
    ],
  ];
  for (const [asked, leave, said] of cases) assert.deepEqual(goesWith(asked, leave), said);
});

test('what is said once a key is revoked counts what went with it', () => {
  const cases: [Revoking, string][] = [
    [found({ stopped: [] }), 'ci is revoked.'],
    [found({ stopped: [fleet] }), 'ci is revoked; one thing it applied stopped.'],
    [found({ keys: [{ id: 'a', name: 'a' }, { id: 'b', name: 'b' }], stopped: [fleet, fleet] }), 'ci is revoked with 2 keys made with it; 2 things it applied stopped.'],
    [found({ running: [fleet], stopped: [] }), 'ci is revoked; one thing it applied still running.'],
  ];
  for (const [done, said] of cases) assert.equal(revokedSaid(done), said);
});

import { strict as assert } from 'node:assert';
import { test } from 'node:test';
import { type Listed, linesOf, offered, saidOf } from './given.ts';
import type { Area } from './holds.ts';

// As the daemon lists them: areas with parts, and areas that are one thing.
const AREAS: Area[] = [
  { area: 'admin', does: 'manage accounts', verbs: [] },
  { area: 'board', verbs: [{ verb: 'read', does: 'read work items' }, { verb: 'write', does: 'change work items' }] },
  { area: 'private', does: 'read what is kept private', verbs: [] },
  { area: 'environments', verbs: [{ verb: 'secrets', does: 'set the secrets of environments' }] },
];
const OWNER = ['board', 'admin', 'private', 'environments:secrets'];
const account = (role: string, holds: string[], given: string[] = []): Listed => ({
  id: 'acct_1',
  email: 'grace@example.com',
  name: 'Grace',
  role,
  tier: 'premium',
  created: 0,
  holds,
  given,
});

test('every scope there is is offered as it is written to be held', () => {
  assert.deepEqual(
    linesOf(AREAS).map((line) => line.scope),
    ['admin', 'board:read', 'board:write', 'private', 'environments:secrets'],
  );
});

test('a member is offered what the giver holds and she does not, and nothing more', () => {
  const grace = account('member', ['board']);
  assert.deepEqual(
    offered(AREAS, OWNER, grace).map((line) => line.scope),
    ['admin', 'private', 'environments:secrets'],
  );
  // Somebody who manages the factory and holds less offers less.
  assert.deepEqual(
    offered(AREAS, ['admin', 'board', 'environments'], grace).map((line) => line.scope),
    ['admin', 'environments:secrets'],
  );
  // What she was given is not offered again.
  const given = account('member', ['board', 'environments:secrets'], ['environments:secrets']);
  assert.deepEqual(offered(AREAS, OWNER, given).map((line) => line.scope), ['admin', 'private']);
});

test('nothing is offered by somebody who does not manage the factory, nor to its owner', () => {
  assert.deepEqual(offered(AREAS, ['board', 'environments:secrets'], account('member', ['board'])), []);
  assert.deepEqual(offered(AREAS, OWNER, account('owner', OWNER)), []);
});

test('what somebody holds is said by their role, and leads to what they were given', () => {
  assert.equal(saidOf(account('member', ['board'])), 'Holds what a member holds');
  assert.equal(saidOf(account('member', ['board'], ['environments:secrets'])), 'Holds what a member holds, and');
  assert.equal(saidOf(account('owner', OWNER)), 'Holds everything, as the owner');
});

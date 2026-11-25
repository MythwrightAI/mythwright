import { strict as assert } from 'node:assert';
import { test } from 'node:test';
import type { Area } from './holds.ts';
import { addClosed, addable, holdsAll, countSaid, holdsSaid, joiners, mayManage, memberOf, nameRefused, type PeopleGroup } from './peoplegroups.ts';

const AREAS: Area[] = [
  { area: 'admin', does: 'manage', verbs: [] },
  { area: 'private', does: 'kept private', verbs: [] },
  { area: 'board', verbs: [{ verb: 'read', does: 'read' }, { verb: 'write', does: 'change' }] },
  { area: 'environments', verbs: [{ verb: 'secrets', does: 'secrets' }] },
];
const group = (members: string[], holds?: string[]): PeopleGroup => ({ id: 'grp_1', name: 'Deploys', members, holds, created: 0 });
const KNOWN = joiners(
  [
    { id: 'acct_2', name: 'Grace' },
    { id: 'acct_1', name: 'Ada' },
  ],
  [{ name: 'helper' }],
);

test('everybody who can be in a group is the people by name and then the agents', () => {
  assert.deepEqual(
    KNOWN.map((one) => [one.actor, one.kind]),
    [
      ['user:acct_1', 'person'],
      ['user:acct_2', 'person'],
      ['agent:helper', 'agent'],
    ],
  );
});

test('a member is drawn by name, and one nobody knows as the daemon wrote it', () => {
  assert.equal(memberOf('user:acct_2', KNOWN).name, 'Grace');
  assert.deepEqual(memberOf('agent:gone', KNOWN), { actor: 'agent:gone', name: 'gone', kind: 'agent' });
  assert.equal(memberOf('user:acct_9', KNOWN).name, 'user:acct_9');
});

test('whoever is in a group already is not offered again', () => {
  assert.deepEqual(
    addable(group(['user:acct_1', 'agent:helper']), KNOWN).map((one) => one.actor),
    ['user:acct_2'],
  );
});

test('only whoever manages the factory changes a group, and puts nobody in one that holds what they do not', () => {
  assert.equal(mayManage(['board', 'admin']), true);
  assert.equal(mayManage(['board']), false);
  assert.equal(mayManage(['*']), true);
  assert.equal(addClosed(['admin', 'board'], group([], ['board:read']), AREAS), null);
  assert.match(addClosed(['admin', 'board'], group([], ['environments:secrets']), AREAS) ?? '', /holds environments:secrets, which you do not/);
  assert.equal(addClosed(['admin'], group([]), AREAS), null, 'a group that holds nothing gives nothing');
  // An area written whole is held where each thing done in it is.
  assert.equal(addClosed(['admin', 'environments:secrets'], group([], ['environments']), AREAS), null);
  assert.equal(holdsAll(['board:read'], 'board', AREAS), false);
  assert.equal(holdsAll(['board:read', 'board:write'], 'board', AREAS), true);
  assert.equal(holdsAll(['admin'], 'private', AREAS), false, 'an area that is one thing is held by itself alone');
});

test('a name is something, not too long, and no other group has it whatever its capitals', () => {
  const all = [group([]), { ...group([]), id: 'grp_2', name: 'Readers' }];
  assert.match(nameRefused('  ', all) ?? '', /called something/);
  assert.match(nameRefused('x'.repeat(61), all) ?? '', /60 letters at most/);
  assert.match(nameRefused('deploys', all) ?? '', /called Deploys already/);
  assert.equal(nameRefused('DEPLOYS', all, 'grp_1'), null, 'its own name, called otherwise');
  assert.equal(nameRefused('Writers', all), null);
});

test('how many are in a group and what it holds are said in words', () => {
  assert.equal(countSaid(group([])), 'nobody in it yet');
  assert.equal(countSaid(group(['a'])), '1 member');
  assert.equal(countSaid(group(['a', 'b'])), '2 members');
  assert.equal(holdsSaid(group([], [])), 'nothing beyond what each holds');
  assert.equal(holdsSaid(group([], ['admin', 'board:read'])), 'admin, board:read');
});

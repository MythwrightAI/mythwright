import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import { describing, managedSaid, managerLine, managersOf, manages, MOST_TOPIC } from './managed.ts';

interface Case {
  said: string;
  channel: { kept?: boolean; conversation?: boolean; members: { actor: string; manages?: boolean }[] };
  actor: string;
  scopes: string[];
  manages: boolean;
}

test('who manages a channel is the one rule the daemon is held to, case for case', () => {
  const cases = JSON.parse(readFileSync(new URL('./managedcases.json', import.meta.url), 'utf8')) as Case[];
  assert.ok(cases.length > 10, 'the table is there');
  for (const one of cases) assert.equal(manages(one.channel, { actor: one.actor, scopes: one.scopes }), one.manages, one.said);
});

const of = (...members: [string, boolean?][]) => ({ members: members.map(([actor, manages]) => ({ actor, name: actor.slice(5), manages })) });
const ada = { actor: 'user:ada', scopes: ['channels'] };

test('who manages it is said in words, the reader as you', () => {
  assert.deepEqual(managersOf(of(['user:ada', true], ['user:mira'], ['agent:scout', true])).map((one) => one.actor), ['user:ada']);
  assert.equal(managedSaid(of(['user:ada', true], ['user:mira', true]), 'user:ada'), 'Managed by you and mira.');
  assert.equal(managedSaid(of(['user:ada', true], ['user:mira', true], ['user:nell', true]), null), 'Managed by ada, mira and nell.');
  assert.equal(managedSaid(of(['user:ada']), null), 'Nobody in it manages it: whoever manages the factory does.');
  assert.equal(managedSaid({ conversation: true, ...of(['user:ada']) }, null), 'Everybody in it manages it.');
});

test('a manager is offered making a person one or no longer one, the last staying, and nobody else anything', () => {
  const channel = of(['user:ada', true], ['user:mira'], ['agent:scout']);
  assert.deepEqual(managerLine(channel, channel.members[1]!, ada), { label: 'Make manager', manages: true, closed: null });
  assert.equal(managerLine(channel, channel.members[0]!, ada)?.closed, 'The last manager stays: make somebody else one first.');
  assert.equal(managerLine(channel, channel.members[2]!, ada), null, 'an agent manages nothing');
  assert.equal(managerLine(channel, channel.members[0]!, { actor: 'user:mira', scopes: ['channels'] }), null, 'a member who manages nothing');
  assert.equal(managerLine({ conversation: true, ...channel }, channel.members[1]!, ada), null, 'a conversation is everybody’s');
  const two = of(['user:ada', true], ['user:mira', true]);
  assert.deepEqual(managerLine(two, two.members[0]!, ada), { label: 'Not a manager', manages: false, closed: null });
});

test('what a channel says it is for is one line, of so many letters at most', () => {
  assert.deepEqual(describing('fruit', '  quinces  '), { topic: 'quinces', changes: true, says: null });
  assert.deepEqual(describing('quinces', 'quinces '), { topic: 'quinces', changes: false, says: null });
  assert.deepEqual(describing('fruit', ''), { topic: '', changes: true, says: null });
  assert.equal(describing('', 'one\ntwo').says, 'It is one line.');
  assert.equal(describing('', 'q'.repeat(MOST_TOPIC + 1)).says, `It is ${MOST_TOPIC} letters at most.`);
});

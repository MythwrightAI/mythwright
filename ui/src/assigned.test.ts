import assert from 'node:assert/strict';
import { test } from 'node:test';

import { assignedTo } from './assigned.ts';

const agents = [
  { name: 'walls', adapter: 'external' },
  { name: 'shop-worker-1', adapter: 'builtin' },
];
const me = { actor: 'user:acct_1', name: 'Ada' };

test('an agent the factory does not start is said to be external', () => {
  assert.deepEqual(assignedTo('agent:walls', agents, me), { name: 'walls', kind: 'external', agent: true });
});

test('an agent the factory starts is said to be an agent', () => {
  assert.deepEqual(assignedTo('agent:shop-worker-1', agents, me), { name: 'shop-worker-1', kind: 'agent', agent: true });
});

test('an agent the page has not read is an agent all the same, by what the card says', () => {
  assert.deepEqual(assignedTo('agent:gone', agents, me), { name: 'gone', kind: 'agent', agent: true });
  assert.deepEqual(assignedTo('agent:walls', [], null), { name: 'walls', kind: 'agent', agent: true });
});

test('the reader is said by their name', () => {
  assert.deepEqual(assignedTo('user:acct_1', agents, me), { name: 'Ada', kind: 'you', agent: false });
});

const people = [
  { actor: 'user:acct_1', name: 'Ada' },
  { actor: 'user:acct_2', name: 'Grace' },
  { actor: 'user:acct_3', name: '  ' },
];

test('another person is said by their name', () => {
  assert.deepEqual(assignedTo('user:acct_2', agents, me, people), { name: 'Grace', kind: 'person', agent: false });
  assert.deepEqual(assignedTo('user:acct_1', agents, null, people), { name: 'Ada', kind: 'person', agent: false });
  assert.deepEqual(assignedTo('user:acct_1', agents, { actor: 'user:acct_1', name: null }, people), { name: 'Ada', kind: 'person', agent: false });
});

test('a person nothing names is somebody, and never the id of their account', () => {
  assert.deepEqual(assignedTo('user:acct_9', agents, me, people), { name: 'someone', kind: 'person', agent: false });
  assert.deepEqual(assignedTo('user:acct_2', agents, me), { name: 'someone', kind: 'person', agent: false });
  assert.deepEqual(assignedTo('user:acct_3', agents, me, people), { name: 'someone', kind: 'person', agent: false });
  for (const one of ['user:acct_2', 'user:acct_9', 'user:acct_3']) assert.ok(!assignedTo(one, agents, null, people).name.startsWith('acct_'));
});

test('a bot is said to be one', () => {
  assert.deepEqual(assignedTo('bot:factory', agents, me), { name: 'factory', kind: 'bot', agent: false });
});

test('a word that says no kind is said as it is', () => {
  assert.deepEqual(assignedTo('somebody', agents, me), { name: 'somebody', kind: null, agent: false });
  assert.deepEqual(assignedTo('agent:', agents, me).name, '');
});

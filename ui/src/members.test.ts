import assert from 'node:assert/strict';
import { test } from 'node:test';
import { addable, DEFAULT_TABS, MEMBERS, PANELS, shown, tabNow, tabsOf } from './context.ts';
import { kindOf, lacking, listed } from './members.ts';

const CHANNEL = [
  { actor: 'agent:builder', name: 'builder' },
  { actor: 'user:acct_1', name: 'Ada' },
  { actor: 'bot:conductor', name: 'conductor' },
  { actor: 'agent:reviewer', name: 'reviewer' },
  { actor: 'user:acct_2', name: 'Grace' },
];

test('everybody of a channel is in one list: people, then agents, then bots, each as the channel has them', () => {
  assert.deepEqual(
    listed(CHANNEL).map((one) => `${one.name}:${one.kind}`),
    ['Ada:person', 'Grace:person', 'builder:agent', 'reviewer:agent', 'conductor:bot'],
  );
});

test('each says its kind, and an agent the slot its state is kept by', () => {
  assert.equal(kindOf('user:acct_1'), 'person');
  assert.equal(kindOf('agent:builder'), 'agent');
  assert.equal(kindOf('bot:conductor'), 'bot');
  assert.equal(kindOf('system'), 'nobody');
  assert.deepEqual(
    listed(CHANNEL).map((one) => one.slot),
    [null, null, 'builder', 'reviewer', null],
  );
});

test('a channel with no agent says how one is added, and one with an agent says nothing', () => {
  assert.match(lacking(listed([{ actor: 'user:acct_1', name: 'Ada' }])) ?? '', /Add one from who is in it/);
  assert.equal(lacking(listed(CHANNEL)), null);
  assert.equal(listed([]).length, 0);
});

test('agents and people are one tab, which the panel has unless it is taken away', () => {
  assert.ok(DEFAULT_TABS.includes(MEMBERS));
  assert.equal(PANELS.find((panel) => panel.id === MEMBERS)?.name, 'Channel');
  assert.ok(!PANELS.some((panel) => panel.id === 'agents' || panel.id === 'people'));
});

test('a browser that kept either of the two tabs it took the place of has the one, once, where the first was', () => {
  assert.equal(tabNow('agents'), MEMBERS);
  assert.equal(tabNow('people'), MEMBERS);
  assert.equal(tabNow('board'), 'board');
  assert.deepEqual(tabsOf('["media","agents","board"]'), ['media', MEMBERS, 'board', 'docs']);
  assert.deepEqual(tabsOf('["media","board","people"]'), ['media', 'board', MEMBERS, 'docs']);
  assert.deepEqual(tabsOf('["agents","media","people"]'), [MEMBERS, 'media', 'docs']);
  // The tab that was open is the one that took its place.
  assert.equal(shown(tabsOf('["media","board","people"]'), tabNow('people')), MEMBERS);
  assert.equal(shown(tabsOf('["media","agents"]'), tabNow('agents')), MEMBERS);
  assert.deepEqual(addable(['media', MEMBERS, 'board']).map((panel) => panel.id), ['docs']);
});

import assert from 'node:assert/strict';
import { test } from 'node:test';
import { sameScopes, sectionKey, settingsOf } from './membersettings.ts';
import type { Section } from './membersettings.ts';

const USUAL = ['projects', 'channels', 'board', 'records', 'blueprints:read', 'agents:read', 'fleet:read', 'search'];
const MEMBER = ['projects', 'channels', 'board', 'records', 'blueprints', 'agents', 'fleet', 'search'];
const holds = (sections: Section[]) => sections.map((one) => (one.section === 'holds' ? `holds ${one.may ? 'may' : 'may not'}${one.sent ? ' sent' : ''}` : `given ${one.account.name}`)).join(' | ');

test('an agent made or invited begins on what one usually holds as far as whoever adds holds it, changed and sent with the add', () => {
  const [made] = settingsOf({ kind: 'agent', coming: 'new', usual: USUAL }, ['*']);
  assert.ok(made && made.section === 'holds');
  assert.deepEqual(made.scopes, USUAL);
  assert.equal(holds([made]), 'holds may sent');
  // Somebody who holds less is offered less: what they do not hold is not begun on.
  const [less] = settingsOf({ kind: 'agent', coming: 'invited', usual: [...USUAL, 'inbox'] }, ['channels', 'board', 'agents']);
  assert.ok(less && less.section === 'holds');
  assert.deepEqual(less.scopes, ['channels', 'board', 'agents:read']);
  assert.match(less.said, /^It holds this once it joins\./);
});

test('an agent that is there shows what it holds, changeable where the Tools panel would let it be, and never one a fleet counts', () => {
  const slot = { name: 'helper', scopes: USUAL };
  assert.equal(holds(settingsOf({ kind: 'agent', coming: 'there', slot }, ['*'])), 'holds may sent');
  assert.equal(holds(settingsOf({ kind: 'agent', coming: 'there', slot }, MEMBER)), 'holds may sent');
  assert.equal(holds(settingsOf({ kind: 'agent', coming: 'there', slot }, ['channels', 'agents:read'])), 'holds may not');
  // One this factory runs is changed by applying its text as whoever asks, and one from outside is not.
  const noApplying = ['channels', 'agents', 'blueprints:read'];
  assert.equal(holds(settingsOf({ kind: 'agent', coming: 'there', slot: { ...slot, adapter: 'builtin' } }, noApplying)), 'holds may not');
  assert.equal(holds(settingsOf({ kind: 'agent', coming: 'there', slot: { ...slot, name: 'walls', adapter: 'external' } }, noApplying)), 'holds may sent');
  const counted = settingsOf({ kind: 'agent', coming: 'there', slot: { ...slot, name: 'shop-worker-1', held_by: 'fleet' } }, ['*']);
  assert.equal(holds(counted), 'holds may not');
  assert.match(counted[0]!.section === 'holds' ? counted[0]!.said : '', /changed in its row of the fleet's agents/);
});

test('a person shows what they hold, with what may be given them only to whoever may give', () => {
  const grace = { id: 'grace', email: 'grace@example.com', name: 'Grace', role: 'member', tier: 'premium', created: 0, given: [], holds: MEMBER };
  assert.equal(holds(settingsOf({ kind: 'person', name: 'Grace', account: grace }, ['*'])), 'holds may not | given Grace');
  assert.equal(holds(settingsOf({ kind: 'person', name: 'Grace', account: grace }, ['admin', 'channels'])), 'holds may not | given Grace');
  assert.equal(holds(settingsOf({ kind: 'person', name: 'Grace', account: grace }, MEMBER)), 'holds may not');
  // The owner holds everything and is given nothing.
  assert.equal(holds(settingsOf({ kind: 'person', name: 'Ada', account: { ...grace, name: 'Ada', role: 'owner' } }, ['*'])), 'holds may not');
  // Whoever may not read what an account holds is told where it is said.
  const unread = settingsOf({ kind: 'person', name: 'Grace', account: null }, MEMBER);
  assert.equal(holds(unread), 'holds may not');
  assert.match(unread[0]!.section === 'holds' ? unread[0]!.said : '', /whoever manages the factory reads and changes in Settings/);
});

test('a bot has nothing to set yet', () => {
  assert.deepEqual(settingsOf({ kind: 'bot' }, ['*']), []);
});

test('what was changed is kept while the same one is shown and begun again for another', () => {
  assert.equal(sectionKey({ kind: 'agent', coming: 'new', usual: [] }), 'agent:new');
  assert.equal(sectionKey({ kind: 'agent', coming: 'there', slot: { name: 'walls', scopes: [] } }), 'agent:walls');
  assert.equal(sectionKey({ kind: 'person', name: 'Grace', account: null }), 'person:Grace');
  assert.ok(sameScopes(['a', 'b'], ['b', 'a']));
  assert.ok(!sameScopes(['a'], ['a', 'b']));
});

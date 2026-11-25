import assert from 'node:assert/strict';
import { test } from 'node:test';
import { didOf, externalDoing, givenTo, inHand, isExternal, joinCommand, kindSaid, nameOf, offeredScopes, standingOf, within } from './external.ts';
import type { Outside, Said } from './external.ts';

const NOW = Date.UTC(2026, 10, 20, 12, 0, 0);
const HOUR = 3_600_000;
const columns = ['backlog', 'ready', 'doing', 'review', 'done'];
const card = (id: string, status: string, assignee: string | null = 'agent:walls') => ({ id, status, assignee });
const walls = (more: Partial<Outside> = {}): Outside => ({ name: 'walls', adapter: 'external', active: true, external: { joined: true, invited_until: null }, ...more });

test('an agent is external by what its slot says runs it', () => {
  assert.equal(isExternal({ adapter: 'external' }), true);
  assert.equal(isExternal({ adapter: 'builtin' }), false);
  assert.equal(isExternal(null), false);
});

test('a card is in hand where it is given to the agent and in a column past the first and not the last', () => {
  const board = {
    columns,
    items: [card('shop-1', 'backlog'), card('shop-2', 'ready'), card('shop-3', 'doing'), card('shop-4', 'review'), card('shop-5', 'done'), card('shop-6', 'doing', 'agent:helper'), card('shop-7', 'doing', null), card('shop-8', 'gone')],
  };
  assert.deepEqual(
    inHand(board, 'walls').map((one) => one.id),
    ['shop-2', 'shop-3', 'shop-4'],
  );
  // Given, and not done: the one nobody has begun with them.
  assert.deepEqual(
    givenTo(board, 'walls').map((one) => one.id),
    ['shop-1', 'shop-2', 'shop-3', 'shop-4', 'shop-8'],
  );
  // A board of two columns has a first and a last and nothing between; one that was not read has nothing.
  assert.deepEqual(inHand({ columns: ['todo', 'done'], items: [card('a-1', 'todo'), card('a-2', 'done')] }, 'walls'), []);
  assert.deepEqual(inHand(null, 'walls'), []);
  assert.deepEqual(givenTo(undefined, 'walls'), []);
  // A person of the same name is not the agent.
  assert.deepEqual(inHand({ columns, items: [card('shop-1', 'doing', 'user:walls')] }, 'walls'), []);
});

test('how it stands is whether it has joined and whether an invitation still waits', () => {
  assert.equal(standingOf(walls(), NOW), 'joined');
  assert.equal(standingOf(walls({ external: { joined: false, invited_until: NOW + HOUR } }), NOW), 'invited · has yet to join');
  assert.equal(standingOf(walls({ external: { joined: true, invited_until: NOW + HOUR } }), NOW), 'joined · invited again');
  // An invitation that has stopped working waits for nobody.
  assert.equal(standingOf(walls({ external: { joined: false, invited_until: NOW - 1 } }), NOW), 'not joined');
  assert.equal(standingOf(walls({ external: { joined: false, invited_until: null } }), NOW), 'not joined');
  // A daemon that says nothing of it, and one that is stopped whatever else is said.
  assert.equal(standingOf(walls({ external: undefined }), NOW), 'not joined');
  assert.equal(standingOf(walls({ active: false }), NOW), 'stopped');
});

test('what it is doing is the card it has in hand, and otherwise how it stands', () => {
  assert.deepEqual(externalDoing(walls(), [card('shop-3', 'doing')], NOW), { state: 'working', words: 'working on shop-3' });
  assert.deepEqual(externalDoing(walls(), [card('shop-3', 'doing'), card('shop-4', 'review'), card('shop-9', 'doing')], NOW).words, 'working on shop-3 and 2 more');
  assert.deepEqual(externalDoing(walls(), [], NOW), { state: 'idle', words: 'joined' });
  assert.deepEqual(externalDoing(walls({ external: { joined: false, invited_until: null } }), [], NOW), { state: 'idle', words: 'not joined' });
  // Stopped, whatever card says it.
  assert.deepEqual(externalDoing(walls({ active: false }), [card('shop-3', 'doing')], NOW), { state: 'stopped', words: 'stopped' });
});

test('how long an invitation works is said as the command line says it', () => {
  assert.equal(within(NOW + HOUR, NOW), 'the next hour');
  assert.equal(within(NOW + 12 * HOUR, NOW), 'the next 12 hours');
  assert.equal(within(NOW + 24 * HOUR, NOW), 'the next 24 hours');
  assert.equal(within(NOW + 72 * HOUR, NOW), 'the next 3 days');
  assert.equal(within(NOW + 14 * 24 * HOUR, NOW), 'the next 14 days');
});

test('a new one is offered what an agent usually holds, as far as whoever invites holds it', () => {
  const usual = ['projects', 'board', 'blueprints:read', 'search'];
  assert.deepEqual(offeredScopes(usual, ['*']), usual);
  assert.deepEqual(offeredScopes(usual, ['agents', 'board', 'blueprints']), ['board', 'blueprints:read']);
  assert.deepEqual(offeredScopes(usual, ['agents', 'board:read']), []);
});

test('a name is what the daemon makes of it, and the command that joins says where', () => {
  assert.equal(nameOf('Walls 2'), 'walls-2');
  assert.equal(nameOf('  --Outer_Gate!! '), 'outer-gate');
  assert.equal(nameOf('!!'), '');
  assert.equal(joinCommand('mw', 'http://127.0.0.1:7700'), 'mw agent join --url http://127.0.0.1:7700');
});

test('what it did is what says it did it: its moves, what it took and said, and nothing of anybody else', () => {
  let seq = 0;
  const said = (topic: string, actor: string, data: Record<string, unknown>): Said => ({ seq: (seq += 1), ts: NOW + seq, topic, source: { actor }, data });
  const events = [
    said('channel.message', 'agent:walls', { author: 'agent:walls', channel: 'c1', body: 'The gate is hung.' }),
    said('record.commented', 'agent:walls', { id: 'shop-3', body: 'Hinges are on.' }),
    said('board.item.moved', 'agent:walls', { id: 'shop-3', from: 'backlog', to: 'doing' }),
    said('board.item.updated', 'agent:walls', { id: 'shop-3', assignee: 'agent:walls', changed: ['assignee'] }),
    said('board.item.updated', 'agent:walls', { id: 'shop-3', assignee: 'agent:walls', changed: ['title'] }),
    said('board.item.created', 'agent:walls', { id: 'shop-9' }),
    // Somebody else's, about a card of its own; a channel the page has not read; and what is nothing it did.
    said('board.item.moved', 'user:ada', { id: 'shop-3', from: 'doing', to: 'done' }),
    said('channel.message', 'agent:walls', { author: 'agent:walls', channel: 'c9', body: 'Elsewhere.' }),
    said('channel.message', 'agent:walls', { author: 'user:ada', channel: 'c1', body: '@walls and the latch?' }),
    said('agent.slot.joined', 'agent:walls', { slot: 'walls' }),
  ];
  const named = (id: string) => (id === 'c1' ? 'yard' : null);
  assert.deepEqual(
    didOf(events, 'walls', named).map((one) => one.words),
    ['said in #yard: The gate is hung.', 'said on shop-3: Hinges are on.', 'moved shop-3 from backlog to doing', 'took shop-3', 'changed shop-3', 'added shop-9', 'said in a channel: Elsewhere.'],
  );
  assert.deepEqual(didOf(events, 'helper', named), []);
});

test('who is offered to a channel says an agent the factory does not start is external', () => {
  const slots = [walls(), { name: 'helper', adapter: 'stand-in', active: true }];
  assert.equal(kindSaid('agent:walls', slots), 'agent (external)');
  assert.equal(kindSaid('agent:helper', slots), 'agent');
  assert.equal(kindSaid('agent:gone', slots), 'agent');
  assert.equal(kindSaid('user:acct_1', slots), 'person');
});

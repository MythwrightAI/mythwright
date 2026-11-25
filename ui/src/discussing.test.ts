import assert from 'node:assert/strict';
import { test } from 'node:test';
import { asksToDiscuss, discussionOf, markSaid, named } from './discussing.ts';

const offered = { channel: 'ch_1', settings: { on: ['tagged', 'plans', 'asked'] }, taking: ['ann', 'bo', 'carl'] };

test('who is named is read as the daemon reads it, each once, and not in an address', () => {
  assert.deepEqual(named('@ann and @Bo, ask @ann again; mail ann@example.com'), ['ann', 'bo']);
});

test('agents are offered to be asked to discuss only where two or more who take part are named and the channel takes it', () => {
  assert.equal(asksToDiscuss(offered, '@ann and @bo, which table?'), true);
  assert.equal(asksToDiscuss(offered, '@ann which table?'), false);
  assert.equal(asksToDiscuss(offered, '@ann and @dave?'), false);
  assert.equal(asksToDiscuss({ ...offered, settings: { on: ['tagged'] } }, '@ann and @bo'), false);
  assert.equal(asksToDiscuss({ ...offered, settings: null }, '@ann and @bo'), false);
  assert.equal(asksToDiscuss(null, '@ann and @bo'), false);
});

test('a mark is said as a word, and one this page does not know as nothing', () => {
  assert.equal(markSaid('plan'), 'Plan');
  assert.equal(markSaid('discuss'), 'To discuss');
  assert.equal(markSaid('shouted'), null);
  assert.equal(markSaid(null), null);
});

test('a thread is said to be a discussion going or waiting only while it is one', () => {
  const discussing = { ...offered, discussions: [{ root: 'm1', status: 'going', stretch: 1 }, { root: 'm2', status: 'waiting', stretch: 2 }] };
  assert.equal(discussionOf(discussing, 'm1'), 'going');
  assert.equal(discussionOf(discussing, 'm2'), 'waiting');
  assert.equal(discussionOf(discussing, 'm3'), null);
  assert.equal(discussionOf({ ...offered, discussions: [{ root: 'm1', status: 'ended', stretch: 1 }] }, 'm1'), null);
  assert.equal(discussionOf(offered, 'm1'), null);
});

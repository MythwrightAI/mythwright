import assert from 'node:assert/strict';
import { test } from 'node:test';
import { cameSaid, costSaid, orderSaid, roundBars, spentSaid, touches, turnSaid } from './cardcost.ts';
import type { CostTurn, Spent } from './cardcost.ts';

const spent = (more: Partial<Spent> = {}): Spent => ({ turns: 2, sent: 1400, wrote: 240, cost_usd: 0.27, took_ms: 1400, unsaid: 0, ...more });
const turn = (more: Partial<CostTurn> = {}): CostTurn => ({
  id: 't1',
  agent: 'shop-worker-1',
  does: 'works',
  status: 'done',
  chose: null,
  sent: 1000,
  wrote: 200,
  cost_usd: 0.25,
  asked: 0,
  took_ms: 1100,
  says: true,
  ...more,
});

test('what was decided is called by a word or two', () => {
  assert.deepEqual(
    (['passed', 'back', 'left', 'open'] as const).map(cameSaid),
    ['Passed', 'Sent back', 'Left for a person', 'Under way'],
  );
});

test('a round is drawn as long as its tokens against the largest round, and one that said none has no bar', () => {
  const bars = roundBars([{ spent: spent() }, { spent: spent({ sent: 700, wrote: 120 }) }, { spent: spent({ sent: 0, wrote: 0, unsaid: 2 }) }], 96);
  assert.equal(bars[0]?.whole, 96);
  assert.equal(bars[1]?.whole, Math.round((96 * 820) / 1640));
  assert.equal(bars[2], null);
  // Of each bar, what was sent and what was written.
  assert.equal((bars[0]?.sent ?? 0) + (bars[0]?.written ?? 0), 96);
});

test('what some turns came to is said in a few words, and turns that said nothing of what they took are said so', () => {
  assert.equal(spentSaid(spent()), '2 turns · 1.4k sent · 240 written · $0.27 · 1.4s');
  assert.equal(spentSaid(spent({ turns: 1, sent: 0, wrote: 0, cost_usd: 0, took_ms: 0, unsaid: 1 })), '1 turn · which said nothing of what it took');
  assert.equal(spentSaid(spent({ turns: 3, sent: 0, wrote: 0, cost_usd: 0, unsaid: 3 })), '3 turns · 1.4s · none of which said what it took');
  assert.equal(spentSaid(spent({ unsaid: 1 })), '2 turns · 1.4k sent · 240 written · $0.27 · 1.4s · 1 said nothing of what it took');
  assert.equal(spentSaid(spent({ turns: 0, sent: 0, wrote: 0, cost_usd: 0, took_ms: 0 })), '0 turns');
  // What they cost where their programs said nothing of it: not reported, and never nought.
  assert.equal(spentSaid(spent({ cost_usd: 0, cost_unsaid: 2 })), '2 turns · 1.4k sent · 240 written · cost not reported · 1.4s');
  assert.equal(spentSaid(spent({ cost_unsaid: 1 })), '2 turns · 1.4k sent · 240 written · $0.27, cost not reported for 1 turn · 1.4s');
  assert.equal(costSaid({ turns: 3, cost_usd: 0, cost_unsaid: 2 }), 'cost not reported for 2 turns');
  assert.equal(costSaid({ turns: 2, cost_usd: 0 }), null, 'a daemon from before says nothing of it');
});

test('a turn is said by who took it, what it did and chose, and what it took', () => {
  assert.equal(turnSaid(turn()), 'shop-worker-1 worked · 1.2k tokens · $0.25 · 1.1s');
  assert.equal(turnSaid(turn({ agent: 'shop-reviewer-1', does: 'looks', chose: 'send_back', sent: 400, wrote: 40, cost_usd: 0.02, took_ms: 300 })), 'shop-reviewer-1 looked · chose send_back · 440 tokens · $0.02 · 300ms');
  assert.equal(turnSaid(turn({ says: false, sent: 0, wrote: 0, cost_usd: 0, took_ms: null })), 'shop-worker-1 worked · said nothing of what it took');
  assert.equal(turnSaid(turn({ status: 'failed', does: 'plans' })), 'shop-worker-1 planned · failed · 1.2k tokens · $0.25 · 1.1s');
  assert.equal(turnSaid(turn({ status: 'running', took_ms: null })), 'shop-worker-1 worked · in hand · 1.2k tokens · $0.25');
  assert.equal(turnSaid(turn({ kept_from: true })), 'shop-worker-1 worked · 1.2k tokens · $0.25 · 1.1s · asked in a channel kept from you');
  assert.equal(turnSaid(turn({ cost_usd: 0, cost_said: false })), 'shop-worker-1 worked · 1.2k tokens · cost not reported · 1.1s');
  // One of several pieces of work on the card at once says which; a judge of them says it judged.
  assert.equal(turnSaid(turn({ does: 'contends', way: 'otter' })), 'shop-worker-1 worked as otter · 1.2k tokens · $0.25 · 1.1s');
  assert.equal(turnSaid(turn({ agent: 'shop-reviewer-1', does: 'judges', chose: 'A' })), 'shop-reviewer-1 judged · chose A · 1.2k tokens · $0.25 · 1.1s');
});

test('what is read again for: a line of the card’s history, and a turn of its conversations or one already read', () => {
  const known = new Set(['t9']);
  const cases: [string, Record<string, unknown> | undefined, boolean][] = [
    ['collection.history', { record: 'shop-4' }, true],
    ['collection.history', { record: 'shop-41' }, false],
    ['agent.turn.started', { thread: 'lane:shop-4', turn: 't1' }, true],
    ['agent.turn.queued', { thread: 'plan:shop-4', turn: 't1' }, true],
    ['agent.turn.started', { thread: 'review:shop-4:2', turn: 't2' }, true],
    ['agent.turn.started', { thread: 'lane:shop-41', turn: 't3' }, false],
    ['agent.turn.started', { thread: 'contend:shop-4:otter', turn: 't4' }, true],
    ['agent.turn.queued', { thread: 'judge:shop-4:1', turn: 't5' }, true],
    // One that says no conversation, ending a turn already read.
    ['agent.turn.finished', { turn: 't9' }, true],
    ['agent.turn.finished', { turn: 't8' }, false],
    ['board.item.moved', { id: 'shop-4' }, false],
    ['collection.history', undefined, false],
  ];
  for (const [topic, data, then] of cases) assert.equal(touches('shop-4', topic, data, known), then, `${topic} ${JSON.stringify(data)}`);
});

test('turns from before put in rounds by the order they were written are said to be, and nothing is said where none was', () => {
  assert.equal(orderSaid(0), null);
  assert.equal(orderSaid(undefined), null);
  assert.equal(orderSaid(1), '1 turn from before says nothing of what began it, and is put in a round by the order it was written.');
  assert.equal(orderSaid(3), '3 turns from before say nothing of what began them, and are put in rounds by the order they were written.');
});

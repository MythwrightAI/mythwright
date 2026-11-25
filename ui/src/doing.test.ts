import assert from 'node:assert/strict';
import { test } from 'node:test';
import { NOTHING, doingNow, heard, turnsNow } from './doing.ts';
import type { SaidOfTurn } from './doing.ts';

const NOW = Date.UTC(2026, 10, 19, 12, 0, 0);
const HOUR = 3_600_000;
const named = (id: string) => ({ c1: 'workshop', c2: 'shop' })[id] ?? null;
let seq = 0;
const began = (slot: string, turn: string, more: { channel?: string; thread?: string; ts?: number } = {}): SaidOfTurn => ({
  seq: (seq += 1),
  ts: more.ts ?? NOW,
  topic: 'agent.turn.started',
  data: { slot, turn, channel: more.channel, thread: more.thread },
});
const over = (slot: string, turn: string, ts = NOW): SaidOfTurn => ({ seq: (seq += 1), ts, topic: 'agent.turn.finished', data: { slot, turn } });

test('one that is stopped is said to be stopped, whatever else is known of it', () => {
  assert.deepEqual(doingNow(false, undefined, undefined, NOW, named), { state: 'stopped', words: 'stopped' });
  assert.deepEqual(doingNow(false, { channel: 'c1', thread: null }, NOW - HOUR, NOW, named), { state: 'stopped', words: 'stopped' });
});

test('one at work says where: the card its conversation names, else the channel it answers in, else nothing more', () => {
  assert.deepEqual(doingNow(true, { channel: null, thread: 'lane:shop-4' }, undefined, NOW, named), { state: 'working', words: 'working on shop-4' });
  assert.equal(doingNow(true, { channel: null, thread: 'plan:shop-4' }, undefined, NOW, named).words, 'planning shop-4');
  assert.equal(doingNow(true, { channel: null, thread: 'review:shop-4:2' }, undefined, NOW, named).words, 'reviewing shop-4');
  // A card's conversation says more than the channel its round reports in.
  assert.equal(doingNow(true, { channel: 'c2', thread: 'lane:shop-4' }, undefined, NOW, named).words, 'working on shop-4');
  assert.equal(doingNow(true, { channel: 'c1', thread: null }, undefined, NOW, named).words, 'working in #workshop');
  // A conversation in a channel is no card's; a channel the page has not read is not named by its id.
  assert.equal(doingNow(true, { channel: 'c1', thread: 'channel:c1' }, NOW - HOUR, NOW, named).words, 'working in #workshop');
  assert.deepEqual(doingNow(true, { channel: 'c9', thread: null }, undefined, NOW, named), { state: 'working', words: 'working' });
  assert.equal(doingNow(true, { channel: null, thread: null }, undefined, NOW, named).words, 'working');
});

test('one with nothing in hand is idle, with how long ago it last ended a turn where it ever did', () => {
  assert.deepEqual(doingNow(true, undefined, undefined, NOW, named), { state: 'idle', words: 'idle' });
  assert.deepEqual(doingNow(true, undefined, NOW - 2 * HOUR, NOW, named), { state: 'idle', words: 'idle · 2h ago' });
  assert.equal(doingNow(true, undefined, NOW - 10_000, NOW, named).words, 'idle · just now');
  assert.equal(doingNow(true, undefined, NOW - 3 * 24 * HOUR, NOW, named).words, 'idle · 3d ago');
});

test('a turn that began is in hand until it is said to have ended, and that is when its agent last ended one', () => {
  const one = began('maker', 't1', { channel: 'c1' });
  const going = heard(NOTHING, one);
  assert.deepEqual(going, { going: { maker: { turn: 't1', channel: 'c1', thread: null } }, ended: {} });
  assert.deepEqual(heard(going, over('maker', 't1', NOW + 5)), { going: {}, ended: { maker: NOW + 5 } });
  // What is neither, or names no agent, changes nothing; and nothing handed in is changed.
  assert.equal(heard(going, { seq: 0, ts: NOW, topic: 'agent.output', data: { slot: 'maker', turn: 't1' } }), going);
  assert.equal(heard(going, { seq: 0, ts: NOW, topic: 'agent.turn.finished', data: null }), going);
  assert.deepEqual(NOTHING, { going: {}, ended: {} });
});

test('the end of another turn than the one in hand leaves that one in hand', () => {
  // One called off before it began, behind the one at work.
  const now = [began('maker', 't1', { thread: 'lane:shop-4' }), over('maker', 't2', NOW + 9)].reduce(heard, NOTHING);
  assert.equal(now.going.maker?.turn, 't1');
  assert.equal(now.ended.maker, NOW + 9);
  // An earlier ending heard late does not put the time back.
  assert.equal(heard(now, over('maker', 't0', NOW - HOUR)).ended.maker, NOW + 9);
});

test('a reading is newest first, and says who has what in hand and when each last ended one', () => {
  const said = [began('maker', 't1', { channel: 'c1', ts: NOW - 50 }), over('maker', 't1', NOW - 40), began('helper', 't2', { thread: 'plan:shop-1', ts: NOW - 30 }), began('maker', 't3', { channel: 'c2', ts: NOW - 20 })];
  const now = turnsNow([...said].reverse());
  assert.deepEqual(Object.keys(now.going).sort(), ['helper', 'maker']);
  assert.deepEqual(now.going.maker, { turn: 't3', channel: 'c2', thread: null });
  assert.deepEqual(now.ended, { maker: NOW - 40 });
  assert.deepEqual(turnsNow([]), NOTHING);
});

test('what arrived while a reading was on its way is put after it, and what the reading already holds is not heard twice', () => {
  const first = began('maker', 't1', { channel: 'c1' });
  const ends = over('maker', 't1', NOW + 3);
  const next = began('maker', 't2', { channel: 'c2' });
  // The reading was taken before the turn ended, and its end was said while the reading was on its way: it is not in hand.
  assert.deepEqual(turnsNow([first], [ends]), { going: {}, ended: { maker: NOW + 3 } });
  // The reading was taken after all three, and the first two also arrived meanwhile: the last is in hand all the same.
  assert.equal(turnsNow([next, ends, first], [ends]).going.maker?.turn, 't2');
  assert.equal(turnsNow([next, ends, first], [first, ends, next]).going.maker?.turn, 't2');
  // Something new that arrived is heard.
  assert.equal(turnsNow([first], [ends, next]).going.maker?.turn, 't2');
});

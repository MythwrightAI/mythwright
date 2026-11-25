import assert from 'node:assert/strict';
import { test } from 'node:test';
import { KEPT, LEAST, byDay, endedAs, exactly, lengths, short, totals, whereAsked } from './agenthistory.ts';
import type { Turn } from './agenthistory.ts';

const turn = (more: Partial<Turn> = {}): Turn => ({
  id: 't1',
  slot: 'maker',
  kind: 'message',
  thread: '',
  prompt: 'three palettes, please',
  channel: null,
  status: 'done',
  reply: 'Here it is.',
  error: null,
  decision: null,
  cost_usd: 0,
  input_tokens: 0,
  output_tokens: 0,
  created: 0,
  started: null,
  finished: null,
  ...more,
});
const sized = (input_tokens: number, output_tokens: number) => ({ input_tokens, output_tokens });
const named = (id: string) => ({ c1: 'workshop' })[id] ?? null;

test('a turn went well, failed, was called off, or is in hand', () => {
  assert.equal(endedAs(turn()), 'done');
  assert.equal(endedAs(turn({ status: 'failed', error: 'the built-in assistant cannot do this work' })), 'failed');
  assert.equal(endedAs(turn({ status: 'failed', error: null })), 'failed');
  // Called off is not the agent failing at it: the daemon's two sentences for it.
  assert.equal(endedAs(turn({ status: 'failed', error: 'the turn was cancelled' })), 'called off');
  assert.equal(endedAs(turn({ status: 'failed', error: 'cancelled before it started' })), 'called off');
  for (const status of ['queued', 'running', 'answered']) assert.equal(endedAs(turn({ status })), 'in hand', status);
});

test('where a turn was asked is its card, else its channel by name, else what kind of turn it was', () => {
  assert.equal(whereAsked(turn({ thread: 'lane:shop-4', channel: 'c1' }), named), 'worked shop-4');
  assert.equal(whereAsked(turn({ thread: 'plan:shop-4' }), named), 'planned shop-4');
  assert.equal(whereAsked(turn({ thread: 'review:shop-4:2' }), named), 'reviewed shop-4');
  assert.equal(whereAsked(turn({ thread: 'channel:c1', channel: 'c1' }), named), '#workshop');
  // A channel the page has not read is not named by its id.
  assert.equal(whereAsked(turn({ channel: 'c9', kind: 'task' }), named), 'task');
  assert.equal(whereAsked(turn({ kind: 'consult' }), named), 'consult');
  assert.equal(whereAsked(turn({ kind: '' }), named), 'asked');
  // One whose words are kept from whoever looks says so, and nothing of where.
  assert.equal(whereAsked(turn({ kept_from: true, thread: '', channel: null }), named), KEPT);
  assert.equal(KEPT, 'asked in a channel kept to its members');
});

test('what a list comes to is counted: asked, failed, called off, in hand, tokens each way and what it cost', () => {
  assert.deepEqual(totals([]), { asked: 0, failed: 0, calledOff: 0, inHand: 0, sent: 0, written: 0, cost: 0, costUnsaid: 0 });
  const all = totals([
    turn({ input_tokens: 12_000, output_tokens: 400, cost_usd: 0.25 }),
    turn({ status: 'failed', error: 'no', input_tokens: 3000, output_tokens: 0, cost_usd: 0.05 }),
    turn({ status: 'failed', error: 'the turn was cancelled' }),
    turn({ status: 'running', input_tokens: Number.NaN, cost_usd: -1 }),
    // One whose words are kept from whoever looks is counted like any other.
    turn({ kept_from: true, prompt: '', reply: '', input_tokens: 1000, output_tokens: 100, cost_usd: 0.5 }),
    // One whose program said nothing of what it cost is counted apart, and its nought is no cost.
    turn({ input_tokens: 2000, output_tokens: 10, cost_usd: 0, cost_said: false, finished: 1 }),
    // One in hand has not said yet, and is no cost not reported.
    turn({ status: 'running', input_tokens: 0, output_tokens: 0, cost_usd: 0, cost_said: false }),
  ]);
  assert.deepEqual(all, { asked: 7, failed: 1, calledOff: 1, inHand: 2, sent: 18_000, written: 510, cost: 0.8, costUnsaid: 1 });
});

test('a count of tokens is said in a few letters', () => {
  const said = [0, 7, 999, 1000, 1049, 12_449, 99_949, 99_950, 340_400, 999_499, 999_500, 1_234_567, 45_000_000, 2_500_000_000].map(short);
  assert.deepEqual(said, ['0', '7', '999', '1.0k', '1.0k', '12.4k', '99.9k', '100k', '340k', '999k', '1.0M', '1.2M', '45.0M', '2.5B']);
  assert.equal(short(Number.NaN), '0');
  assert.equal(short(-5), '0');
});

test('with no turn, or none that says tokens, there is no bar', () => {
  assert.deepEqual(lengths([], 100), []);
  assert.deepEqual(lengths([sized(0, 0), sized(0, 0)], 100), [null, null]);
});

test('one turn has the whole track, in the parts of what was sent to it and what it wrote', () => {
  assert.deepEqual(lengths([sized(750, 250)], 100), [{ whole: 100, sent: 75, written: 25 }]);
  assert.deepEqual(lengths([sized(0, 40)], 100), [{ whole: 100, sent: 0, written: 100 }]);
  assert.deepEqual(lengths([sized(40, 0)], 100), [{ whole: 100, sent: 100, written: 0 }]);
});

test('a bar is as long as its turn is large against the largest, and a turn that says no tokens has none among the others', () => {
  const bars = lengths([sized(4000, 1000), sized(0, 0), sized(2000, 500), sized(1000, 250)], 100);
  assert.deepEqual(bars.map((bar) => bar?.whole ?? null), [100, null, 50, 25]);
  assert.deepEqual(bars[2], { whole: 50, sent: 40, written: 10 });
  // Twice the tokens is twice the bar: nothing is drawn longer to be seen better.
  assert.equal(bars[2]!.whole, 2 * bars[3]!.whole);
});

test('a small turn beside a very large one is drawn at the least a bar can be, each part that has anything seen', () => {
  const bars = lengths([sized(1_900_000, 100_000), sized(900, 100), sized(5, 0), sized(1_999_999, 1)], 96);
  assert.deepEqual(bars[0], { whole: 96, sent: 91, written: 5 });
  assert.deepEqual(bars[1], { whole: LEAST, sent: 1, written: 1 });
  assert.deepEqual(bars[2], { whole: LEAST, sent: LEAST, written: 0 });
  // What it wrote is a two-millionth of it, and is drawn all the same.
  assert.deepEqual(bars[3], { whole: 96, sent: 95, written: 1 });
  for (const bar of bars) assert.ok(bar && bar.whole <= 96 && bar.sent + bar.written === bar.whole);
});

test('turns fall under the day each was asked: today, yesterday, then the date, with its year where it is another', () => {
  const at = (year: number, month: number, day: number, hour = 12) => new Date(year, month - 1, day, hour).getTime();
  const now = at(2026, 11, 19, 9);
  const asked = [at(2026, 11, 19, 8), at(2026, 11, 19, 0), at(2026, 11, 18, 23), at(2026, 11, 17), at(2026, 11, 17, 1), at(2026, 1, 2), at(2025, 12, 31)].map((created, n) => ({ id: n, created }));
  const days = byDay(asked, now);
  assert.deepEqual(days.map((day) => `${day.day}: ${day.turns.map((one) => one.id).join(' ')}`), ['Today: 0 1', 'Yesterday: 2', '17 Nov: 3 4', '2 Jan: 5', '31 Dec 2025: 6']);
  assert.deepEqual(byDay([], now), []);
  // One asked a moment from now, by a clock that is ahead, is today's.
  assert.equal(byDay([{ created: now + 60_000 }], now)[0]!.day, 'Today');
  // A day is the reader's, in their zone: ten at night in Denver is the next morning in UTC.
  const late = Date.UTC(2026, 6, 2, 4, 7);
  const noon = Date.UTC(2026, 6, 2, 12, 0);
  assert.equal(byDay([{ created: late }], noon, 'America/Denver')[0]!.day, 'Yesterday');
  assert.equal(byDay([{ created: late }], noon, 'UTC')[0]!.day, 'Today');
});

test('the exact figures of a turn say what each is of, and its cost only where it cost anything', () => {
  assert.deepEqual(exactly({ input_tokens: 12_345, output_tokens: 1, cost_usd: 0.0123 }), [
    ['Sent to it', '12,345 tokens'],
    ['Written by it', '1 token'],
    ['Cost', '$0.0123'],
  ]);
  assert.deepEqual(exactly({ input_tokens: 0, output_tokens: 0, cost_usd: 0 }), [
    ['Sent to it', 'no tokens said'],
    ['Written by it', 'no tokens said'],
  ]);
  assert.deepEqual(exactly({ input_tokens: 0, output_tokens: 0, cost_usd: 2.5 }).at(-1), ['Cost', '$2.50']);
  // Not reported, it is said so, and never nought.
  assert.deepEqual(exactly({ input_tokens: 14_492, output_tokens: 5, cost_usd: 0, cost_said: false }).at(-1), ['Cost', 'not reported']);
  assert.deepEqual(exactly({ input_tokens: 0, output_tokens: 0, cost_usd: 0, cost_said: true }).length, 2, 'one that said nought cost nothing to say');
});

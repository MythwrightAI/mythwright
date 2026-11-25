import assert from 'node:assert/strict';
import { test } from 'node:test';
import { earlierSaid, folded, foldsBy, namesSomebody, toggled } from './folded.ts';
import type { Said } from './folded.ts';

const BOT = 'bot:conductor';
const status = (id: string, more: Partial<Said> = {}): Said => ({ id, author: BOT, body: `Status ${id}: ✅ build, ❌ deploy`, said_by: { instance: 'in_status', step: 'say', blueprint: 'status' }, ...more });
const person = (id: string): Said => ({ id, author: 'user:acc_1', body: `Something said, ${id}` });
const shown = (list: Said[], opened: string[] = [], asked?: string) =>
  folded(list, opened, asked).map((one) => `${one.message.id}${one.earlier ? `(${one.earlier.count}${one.earlier.open ? ' open' : ''})` : ''}<${one.follows?.id ?? '-'}`);

test('a message names somebody where @ begins a word and not in an address', () => {
  const cases: [string, boolean][] = [
    ['@ada look at this', true],
    ['done, thanks @grace.', true],
    ['(@shop-worker-1) has it', true],
    ['write to ada@example.com', false],
    ['an @ alone', false],
    ['email@ nobody', false],
    ['nothing at all', false],
    ['@émile', true],
  ];
  for (const [body, names] of cases) assert.equal(namesSomebody(body), names, body);
});

test('what folds is said by who said it and the step of the running blueprint, and nothing answered or naming folds', () => {
  assert.equal(foldsBy(status('a')), `${BOT}\nin_status\nsay`);
  assert.equal(foldsBy(status('a', { said_by: { instance: 'in_status', step: 'other' } })), `${BOT}\nin_status\nother`);
  assert.equal(foldsBy(person('p')), null, 'what a person says never folds');
  assert.equal(foldsBy(status('a', { replies: 2 })), null, 'one somebody answered');
  assert.equal(foldsBy(status('a', { body: '@ada the deploy failed' })), null, 'one that names somebody');
  assert.equal(foldsBy(status('a', { said_by: null })), null, 'one nothing says was said by a step');
});

test('a run of updates is its latest, with how many came before it under it', () => {
  assert.deepEqual(shown([person('p1'), status('s1'), status('s2'), status('s3'), person('p2')]), ['p1<-', 's3(2)<p1', 'p2<s3']);
  assert.deepEqual(shown([status('s1')]), ['s1<-'], 'one alone is itself');
  assert.deepEqual(shown([status('s1'), status('s2')]), ['s2(1)<-'], 'two fold');
  assert.deepEqual(shown([]), []);
});

test('what breaks a run: somebody else, another step, one answered or naming somebody', () => {
  assert.deepEqual(shown([status('s1'), status('s2'), person('p'), status('s3'), status('s4')]), ['s2(1)<-', 'p<s2', 's4(1)<p']);
  assert.deepEqual(shown([status('s1'), status('s2'), status('x', { said_by: { instance: 'in_status', step: 'warn' } }), status('s3')]), ['s2(1)<-', 'x<s2', 's3<x']);
  assert.deepEqual(shown([status('s1'), status('s2'), status('a', { replies: 1 }), status('s3'), status('s4')]), ['s2(1)<-', 'a<s2', 's4(1)<a'], 'an answered one stands by itself');
  assert.deepEqual(shown([status('s1'), status('n', { body: 'over to @ada' }), status('s2')]), ['s1<-', 'n<s1', 's2<n'], 'one that names somebody stands by itself');
  assert.deepEqual(shown([status('s1'), status('o', { said_by: { instance: 'in_other', step: 'say' } }), status('s2')]), ['s1<-', 'o<s1', 's2<o'], 'another instance of the same blueprint is another run');
});

test('a run shown again draws each earlier one before the latest, by the first of it, or where one of them is asked for', () => {
  const list = [person('p1'), status('s1'), status('s2'), status('s3')];
  assert.deepEqual(shown(list, ['s1']), ['p1<-', 's1<p1', 's2<s1', 's3(2 open)<s2']);
  assert.deepEqual(shown(list, ['s2']), ['p1<-', 's3(2)<p1'], 'kept by the first of the run, not another of it');
  assert.deepEqual(shown(list, [], 's2'), ['p1<-', 's1<p1', 's2<s1', 's3(2 open)<s2'], 'one asked for is never hidden');
  assert.deepEqual(shown(list, [], 's3'), ['p1<-', 's3(2)<p1'], 'the latest asked for is there already');
  // More said keeps what was kept: the run is still known by its first.
  assert.deepEqual(shown([...list, status('s4')], ['s1']).at(-1), 's4(3 open)<s3');
});

test('what the press under the latest says', () => {
  assert.equal(earlierSaid(1, false), '1 earlier update');
  assert.equal(earlierSaid(12, false), '12 earlier updates');
  assert.equal(earlierSaid(12, true), 'Hide 12 earlier updates');
});

test('what is kept as shown is what folds in the channel now, with the one changed', () => {
  assert.deepEqual(toggled([], 's1', true, ['s1']), ['s1']);
  assert.deepEqual(toggled(['s1', 'gone'], 's1', false, ['s1']), []);
  assert.deepEqual(toggled(['t1'], 's1', true, ['s1', 't1']), ['t1', 's1']);
});

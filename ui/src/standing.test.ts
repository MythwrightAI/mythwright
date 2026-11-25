import assert from 'node:assert/strict';
import { test } from 'node:test';
import { saidOf, standingAfter } from './standing.ts';

test('nothing is said of the stream while events arrive', () => {
  assert.equal(saidOf(standingAfter('opened')), null);
});

test('nothing is said before the stream was asked for, or after it was put away on purpose', () => {
  assert.equal(saidOf('waiting'), null);
  assert.equal(saidOf(standingAfter('closed')), null);
});

test('a stream that was asked for and could not be had is said to be lost', () => {
  assert.equal(standingAfter('failed'), 'lost');
  assert.match(saidOf('lost') ?? '', /not arriving/);
});

test('nothing is said again once the stream is back', () => {
  const heard = ['opened', 'failed', 'opened'] as const;
  assert.deepEqual(
    heard.map((one) => saidOf(standingAfter(one)) !== null),
    [false, true, false],
  );
});

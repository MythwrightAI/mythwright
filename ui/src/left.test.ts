import assert from 'node:assert/strict';
import { test } from 'node:test';
import { innermost, leftFor } from './left.ts';

const byRound = ['ready', 'blueprint', '', '2'];

test('a card is left for a person where its row says so and nobody has moved it since', () => {
  // What the row keeps of its rounds; how it was last moved; where it was taken from; what is said.
  const cases: [unknown, unknown, unknown, ReturnType<typeof leftFor>][] = [
    // Left by its round, which had moved it: why, and where it is taken up again from.
    [['left', 'no agent is available to take this card'], byRound, 'ready', { why: 'no agent is available to take this card', from: 'ready' }],
    // The reason is the innermost: the steps it came up through are not a person's to read.
    [['left', 'step `one` failed: step `work` failed: shop-worker-1 could not do it: it cannot'], byRound, 'ready', { why: 'shop-worker-1 could not do it: it cannot', from: 'ready' }],
    // Left with no words kept, and by a row that says nothing of where it was taken from.
    [['left', ''], byRound, undefined, { why: '', from: null }],
    [['left'], byRound, '', { why: '', from: null }],
    // A board that keeps nothing of how a card was moved, or something else under that name: still left.
    [['left', 'its round failed'], undefined, 'ready', { why: 'its round failed', from: 'ready' }],
    [['left', 'its round failed'], ['somewhere'], 'ready', { why: 'its round failed', from: 'ready' }],
    // Moved since by a person, a key, an agent or a move said late: it has been seen to.
    [['left', 'its round failed'], ['doing', 'user', '', '3'], 'ready', null],
    [['left', 'its round failed'], ['doing', 'key', 'done by hand', '3'], 'ready', null],
    [['left', 'its round failed'], ['doing', 'agent', '', '3'], 'ready', null],
    [['left', 'its round failed'], ['doing', 'system', '', '3'], 'ready', null],
    // In hand: a round was told of its move.
    [['heard', '2'], byRound, 'ready', null],
    // Given to its agent again, with what was said and which try it is; and because what was done could not be kept.
    [['again', 'the tests fail', '2'], byRound, 'ready', null],
    [['unkept', 'git has no name to commit as', '1'], byRound, 'ready', null],
    // Taken up again where a stage that had it was stopped; and one whose agent asked something.
    [['resumed', ''], byRound, 'ready', null],
    [['resumed'], byRound, 'ready', null],
    [['asked', ''], byRound, 'ready', null],
    // A row a text from before left.
    [['back', 'not what was asked'], byRound, 'ready', null],
    [['said', 'use the other shelf'], byRound, 'ready', null],
    // Nothing kept, a card never worked, and what is no list of words.
    [[], byRound, 'ready', null],
    [undefined, undefined, undefined, null],
    [null, null, null, null],
    ['left', byRound, 'ready', null],
    [['left', 3], byRound, 'ready', null],
    [{ 0: 'left' }, byRound, 'ready', null],
  ];
  for (const [again, moved, taken, said] of cases) assert.deepEqual(leftFor(again, moved, taken), said, JSON.stringify([again, moved, taken]));
});

test('the reason said is the innermost, with the steps it came up through taken off', () => {
  assert.equal(innermost('step `one` failed: step `work` failed: it cannot'), 'it cannot');
  assert.equal(innermost('  it cannot  '), 'it cannot');
  assert.equal(innermost('the step `one` failed: it cannot'), 'the step `one` failed: it cannot');
  assert.equal(innermost('step `a` failed: '), 'step `a` failed:');
  assert.equal(innermost(''), '');
});

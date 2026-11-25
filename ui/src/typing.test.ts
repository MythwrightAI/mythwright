import assert from 'node:assert/strict';
import { test } from 'node:test';
import { projectOfCard, typingIn, typingLine, typingUnder, workingIn, writesThere } from './typing.ts';

test('whoever is answering something said in a channel is typing there, and nowhere else', () => {
  const working = {
    reviewer: { channel: 'c1', thread: 'channel:c1', kind: 'message' },
    builder: { channel: 'c1', thread: 'channel:c1' },
    planner: { channel: 'c2', thread: 'channel:c2', kind: 'message' },
    loose: { channel: null, thread: 'channel:c1' },
  };
  assert.deepEqual(typingIn(working, 'c1'), ['builder', 'reviewer']);
  assert.deepEqual(typingIn(working, 'c2'), ['planner']);
  assert.deepEqual(typingIn(working, 'c3'), []);
});

test('a turn on a card that names a channel is not typing there: it is at work, said apart with its card', () => {
  const working = {
    asked: { channel: 'c1', thread: 'thread:m1', kind: 'message' },
    worker: { channel: 'c1', thread: 'lane:shop-2', kind: 'task' },
    planner: { channel: 'c1', thread: 'plan:shop-3', kind: 'task' },
    seat: { channel: 'c1', thread: 'review:shop-4:1', kind: 'task' },
    stepped: { channel: 'c1', thread: 'research:q', kind: 'task' },
    talking: { channel: 'c1', thread: 'channel:c1', kind: 'task' },
    old: { channel: 'c1' },
  };
  assert.deepEqual(typingIn(working, 'c1'), ['talking']);
  assert.deepEqual(typingUnder(working, 'c1', 'm1'), ['asked']);
  assert.deepEqual(workingIn(working, 'c1'), [
    { name: 'old', card: null, words: 'old is working' },
    { name: 'planner', card: 'shop-3', words: 'planner is planning shop-3' },
    { name: 'seat', card: 'shop-4', words: 'seat is reviewing shop-4' },
    { name: 'stepped', card: null, words: 'stepped is working' },
    { name: 'worker', card: 'shop-2', words: 'worker is working on shop-2' },
  ]);
  assert.deepEqual(workingIn(working, 'c2'), []);
});

test('a turn writes where it was asked when a message asked it or it goes on with a channel or a thread', () => {
  const cases: [Parameters<typeof writesThere>[0], boolean][] = [
    [{ kind: 'message', thread: 'channel:c1' }, true],
    [{ kind: 'message' }, true],
    [{ kind: 'task', thread: 'thread:m1' }, true],
    [{ kind: 'consult', thread: 'channel:c1' }, true],
    [{ kind: 'task', thread: 'lane:shop-2' }, false],
    [{ kind: 'task', thread: 'channelled:x' }, false],
    [{}, false],
  ];
  for (const [turn, writes] of cases) assert.equal(writesThere(turn), writes, JSON.stringify(turn));
});

test('a card is of the longest project whose name and a dash begin it and a number ends it', () => {
  assert.equal(projectOfCard('shop-front-3', ['shop', 'shop-front']), 'shop-front');
  assert.equal(projectOfCard('shop-3', ['shop', 'shop-front']), 'shop');
  assert.equal(projectOfCard('shop-front', ['shop']), null);
  assert.equal(projectOfCard('mill-3', ['shop']), null);
});

test('whoever is in the middle of a turn in a thread is typing there, and in no other thread', () => {
  const working = {
    reviewer: { channel: 'c1', thread: 'thread:m1' },
    builder: { channel: 'c1', thread: 'channel:c1' },
    planner: { channel: 'c1', thread: 'thread:m2' },
    other: { channel: 'c2', thread: 'thread:m1' },
    old: { channel: 'c1' },
  };
  assert.deepEqual(typingUnder(working, 'c1', 'm1'), ['reviewer']);
  assert.deepEqual(typingUnder(working, 'c1', 'm2'), ['planner']);
  assert.deepEqual(typingUnder(working, 'c1', 'm3'), []);
});

test('the line names everyone up to three, and counts the rest', () => {
  assert.equal(typingLine([]), null);
  assert.equal(typingLine(['builder']), 'builder is typing…');
  assert.equal(typingLine(['builder', 'reviewer']), 'builder and reviewer are typing…');
  assert.equal(typingLine(['a', 'b', 'c']), 'a, b and c are typing…');
  assert.equal(typingLine(['a', 'b', 'c', 'd']), 'a, b and 2 others are typing…');
});

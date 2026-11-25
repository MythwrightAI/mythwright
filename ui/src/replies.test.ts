import assert from 'node:assert/strict';
import { test } from 'node:test';
import { answerOf, chainTo, counted, DEPTH, replyOf, startsFolded, tree } from './replies.ts';
import type { Node, Reply } from './replies.ts';

const reply = (seq: number, parent: number | null = null): Reply => ({ seq, parent, by: 'user:ada', who: 'ada', body: `said at ${seq}`, at: seq });
const drawn = (nodes: Node[]): string => nodes.map((node) => `${node.reply.seq}@${node.depth}${node.under.length ? `(${drawn(node.under)})` : ''}`).join(' ');

test('a reply is read from what the daemon handed, what it answers nothing where it says none', () => {
  assert.deepEqual(replyOf({ seq: 12, parent: 9, by: 'user:ada', who: 'ada', body: 'yes', at: 5 }), { seq: 12, parent: 9, by: 'user:ada', who: 'ada', body: 'yes', at: 5 });
  assert.equal(replyOf({ seq: 12, parent: null }).parent, null);
  assert.equal(replyOf({ seq: 12 }).parent, null);
  assert.equal(replyOf({ seq: 12, body: 3 }).body, '');
});

test("the replies are a tree, each under the one it answers, oldest first at each level and the topic's own at the top", () => {
  const replies = [reply(1), reply(2, 1), reply(3), reply(4, 1), reply(5, 2)];
  assert.equal(drawn(tree(replies)), '1@1(2@2(5@3) 4@2) 3@1');
});

test('the tree is the same whatever order the replies were handed in, each reply once', () => {
  const replies = [reply(5, 2), reply(3), reply(2, 1), reply(1), reply(4, 1), reply(2, 1)];
  assert.equal(drawn(tree(replies)), '1@1(2@2(5@3) 4@2) 3@1');
});

test("a reply whose parent is not listed is the topic's own, and so is one that says it answers something said after it", () => {
  assert.equal(drawn(tree([reply(1), reply(3, 2)])), '1@1 3@1');
  assert.equal(drawn(tree([reply(2, 3), reply(3)])), '2@1 3@1');
  assert.equal(drawn(tree([reply(2, 2)])), '2@1');
});

test('no reply is drawn deeper than replies go: one answering a reply at the deepest is drawn beside it', () => {
  const chain = Array.from({ length: DEPTH + 1 }, (_, at) => reply(at + 1, at === 0 ? null : at));
  let deepest = 0;
  const walk = (nodes: Node[]) => nodes.forEach((node) => ((deepest = Math.max(deepest, node.depth)), walk(node.under)));
  walk(tree(chain));
  assert.equal(deepest, DEPTH);
  const eighth = (nodes: Node[]): Node => (nodes[0]!.depth === DEPTH - 1 ? nodes[0]! : eighth(nodes[0]!.under));
  assert.deepEqual(
    eighth(tree(chain)).under.map((node) => node.reply.seq),
    [DEPTH, DEPTH + 1],
  );
});

test('the chain to a reply runs from the topic\'s own down to it', () => {
  const replies = [reply(1), reply(2, 1), reply(3, 2), reply(4, 1)];
  assert.deepEqual(chainTo(replies, 3), [1, 2, 3]);
  assert.deepEqual(chainTo(replies, 4), [1, 4]);
  assert.deepEqual(chainTo(replies, 9), []);
});

test('a chain longer than five starts folded past its third, and a shorter one not at all', () => {
  const long = [reply(1), reply(2, 1), reply(3, 2), reply(4, 3), reply(5, 4), reply(6, 5)];
  assert.deepEqual([...startsFolded(tree(long), [])], [3]);
  assert.deepEqual([...startsFolded(tree(long.slice(0, 5)), [])], []);
});

test('a chain that holds the reply that answered the topic, or one an inbox line opened, never starts folded', () => {
  const long = [reply(1), reply(2, 1), reply(3, 2), reply(4, 3), reply(5, 4), reply(6, 5)];
  assert.deepEqual([...startsFolded(tree(long), [6])], []);
  assert.deepEqual([...startsFolded(tree(long), [4])], []);
  // The reply that folds is shown itself: one kept there folds nothing it holds.
  assert.deepEqual([...startsFolded(tree(long), [3])], [3]);
});

test('a reply says how many are under it, all the way down', () => {
  const top = tree([reply(1), reply(2, 1), reply(3, 2), reply(4, 1)]);
  assert.equal(counted(top[0]!), 3);
  assert.equal(counted(top[0]!.under[1]!), 0);
});

test('the reply that answered the topic is found among its replies, and nothing where none is listed', () => {
  const replies = [reply(1), reply(2, 1)];
  assert.equal(answerOf(replies, 2)?.seq, 2);
  assert.equal(answerOf(replies, 7), null);
  assert.equal(answerOf(replies, null), null);
});

import assert from 'node:assert/strict';
import { test } from 'node:test';
import { commentedOn, Latest, markCalled } from './commented.ts';

test('a comment mark is called how many comments its record has, and what it is for where there is none', () => {
  assert.equal(markCalled(0, 'Install'), 'Comment on Install');
  assert.equal(markCalled(1, 'Install'), '1 comment on Install');
  assert.equal(markCalled(3, 'Install'), '3 comments on Install');
});

test('a count is read again when a line of something said is written for a record of its table, and for nothing else', () => {
  const written = (data: Record<string, unknown>) => ({ topic: 'collection.history', data });
  const line = { project: 'shop', name: 'docs', record: 'shop-docs:welcome', topic: 'record.commented' };
  assert.equal(commentedOn(written(line), 'shop', 'docs'), true);
  // What a card's comment was called before anything else could be spoken on is one too.
  assert.equal(commentedOn(written({ ...line, name: 'board', topic: 'board.item.commented' }), 'shop', 'board'), true);
  // A line of something that happened, another table's, another project's.
  assert.equal(commentedOn(written({ ...line, topic: 'record.updated' }), 'shop', 'docs'), false);
  assert.equal(commentedOn(written(line), 'shop', 'notes'), false);
  assert.equal(commentedOn(written(line), 'depot', 'docs'), false);
  // That it was said comes before its line is written: a count read then would not hold it.
  assert.equal(commentedOn({ topic: 'record.commented', data: { project: 'shop', collection: 'docs', id: 'shop-docs:welcome' } }, 'shop', 'docs'), false);
  assert.equal(commentedOn({ topic: 'collection.history' }, 'shop', 'docs'), false);
});

test('of two readings on their way the one asked for last is kept, whichever comes in first', async () => {
  const latest = new Latest();
  let early!: (count: number) => void;
  let late!: (count: number) => void;
  const first = latest.read(() => new Promise<number>((done) => (early = done)));
  const second = latest.read(() => new Promise<number>((done) => (late = done)));
  // The later comes in first, and then the earlier, which says how things were.
  late(2);
  early(1);
  assert.deepEqual(await second, { read: 2 });
  assert.equal(await first, null, 'the earlier is let go');
  // In the order they were asked, too: the earlier was let go when the later was asked for.
  const third = latest.read(() => new Promise<number>((done) => (early = done)));
  const fourth = latest.read(() => new Promise<number>((done) => (late = done)));
  early(3);
  late(4);
  assert.equal(await third, null);
  assert.deepEqual(await fourth, { read: 4 });
  // One alone is kept.
  assert.deepEqual(await latest.read(async () => 5), { read: 5 });
});

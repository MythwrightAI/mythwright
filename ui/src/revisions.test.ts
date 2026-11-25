import assert from 'node:assert/strict';
import { test } from 'node:test';
import { changedParts, changedSaid, choosing, compared, doorSaid, historyOf, pair, partName, restoring, revisionOf } from './revisions.ts';
import type { Revision } from './revisions.ts';

const revision = (nth: number, said: Partial<Revision> = {}): Revision => ({
  nth,
  title: 'Rolling back',
  body: 'Roll back by the tag.',
  summary: '',
  by: 'user:acct_1',
  who: 'Ada',
  through: null,
  at: 1000 * nth,
  ...said,
});

test('a revision is read from what the daemon hands, whatever it hands of it', () => {
  const read = revisionOf({ nth: 3, title: 'T', body: 'B', summary: 'S', by: 'agent:scout', who: 'scout', through: 'inst-1', at: 42 });
  assert.deepEqual(read, { nth: 3, title: 'T', body: 'B', summary: 'S', by: 'agent:scout', who: 'scout', through: 'inst-1', at: 42 });
  const bare = revisionOf({ nth: 1 });
  assert.equal(bare.who, 'the factory', 'one nobody is named for is the factory’s');
  assert.equal(bare.through, null);
  assert.equal(revisionOf({ by: 'user:acct_9' }).who, 'user:acct_9', 'by its actor where no name came with it');
});

test('a revision changed the parts that say something else than the one before; the first, every part it says', () => {
  assert.deepEqual(changedParts(null, revision(1, { summary: '' })), ['title', 'article']);
  assert.deepEqual(changedParts(revision(1), revision(2, { body: 'Roll back by the release tag.' })), ['article']);
  assert.deepEqual(changedParts(revision(1), revision(2, { title: 'Rollbacks', summary: 'Known now.' })), ['title', 'summary']);
  assert.deepEqual(changedParts(revision(1), revision(2)), []);
});

test('History lists the revisions newest first, each with what it changed', () => {
  const lines = historyOf([revision(2, { body: 'Roll back by the release tag.' }), revision(1), revision(3, { body: 'Roll back by the release tag.', summary: 'Use the tag.' })]);
  assert.deepEqual(
    lines.map((line) => [line.revision.nth, changedSaid(line), line.first, line.latest]),
    [
      [3, 'Summary', false, true],
      [2, 'Article', false, false],
      [1, 'Made', true, false],
    ],
  );
  assert.equal(changedSaid({ first: false, changed: ['title', 'summary', 'article'] }), 'Title, Summary and Article');
  assert.equal(changedSaid({ first: false, changed: [] }), 'No change');
  assert.deepEqual(historyOf([]), []);
});

test("History names what a topic says as its page does: an article's Article, a thread's (a tracked question's) Post", () => {
  const cases: [Parameters<typeof partName>[0], Parameters<typeof partName>[1], string][] = [
    ['article', 'article', 'Article'],
    ['article', 'thread', 'Post'],
    ['summary', 'thread', 'Summary'],
    ['title', 'thread', 'Title'],
    ['article', undefined, 'Article'],
  ];
  for (const [part, of, said] of cases) assert.equal(partName(part, of), said, `${part} on ${of}`);
  assert.equal(changedSaid({ first: false, changed: ['title', 'article'] }, 'thread'), 'Title and Post');
  assert.equal(changedSaid({ first: false, changed: ['article'] }, 'article'), 'Article');
});

test('a revision that says all an earlier one said, after one that said otherwise, is that one restored', () => {
  const old = revision(1);
  const lines = historyOf([old, revision(2, { body: 'Something else.' }), revision(3), revision(4, { title: 'Rollbacks' })]);
  assert.equal(lines.find((line) => line.revision.nth === 3)!.restored?.nth, 1);
  assert.equal(lines.find((line) => line.revision.nth === 4)!.restored, null);
  assert.equal(lines.find((line) => line.revision.nth === 2)!.restored, null, 'the one just before is no restore');
  // The latest of those that said so is the one brought back.
  const again = historyOf([old, revision(2, { body: 'B' }), revision(3), revision(4, { body: 'B' }), revision(5)]);
  assert.equal(again[0]!.restored?.nth, 3);
});

test('two revisions compared say what changed part by part, only the parts that changed', () => {
  const changes = compared(revision(1), revision(2, { body: 'Roll back by the release tag.', summary: 'Known.' }));
  assert.deepEqual(
    changes.map((change) => change.part),
    ['summary', 'article'],
  );
  assert.deepEqual(changes[1]!.pieces, [
    { kind: 'same', text: 'Roll back by the ' },
    { kind: 'added', text: 'release ' },
    { kind: 'same', text: 'tag.' },
  ]);
  assert.deepEqual(compared(revision(1), revision(2)), []);
});

test('two are compared, the older first; a third pressed lets the earliest go; one pressed again is let go', () => {
  assert.deepEqual(pair([5, 2]), [2, 5]);
  assert.equal(pair([2]), null);
  assert.equal(pair([2, 2]), null);
  assert.deepEqual(choosing([], 4), [4]);
  assert.deepEqual(choosing([4], 2), [4, 2]);
  assert.deepEqual(choosing([4, 2], 7), [2, 7]);
  assert.deepEqual(choosing([4, 2], 4), [2]);
});

test('restoring a revision sets its title, article and summary back, all three', () => {
  assert.deepEqual(restoring(revision(1, { summary: '' })), { title: 'Rolling back', body: 'Roll back by the tag.', summary: '' });
});

test('the door is said only where a blueprint made the change', () => {
  assert.equal(doorSaid({ through: null }), null);
  assert.equal(doorSaid({ through: 'inst-1' }), 'through a blueprint');
});

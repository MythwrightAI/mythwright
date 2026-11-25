import assert from 'node:assert/strict';
import { test } from 'node:test';
import { lineOf } from './topicpage.ts';
import type { TopicLine } from './topicpage.ts';
import { addressNames, crumbsOf, movedHereOrAway, namedIn, pathSaid, underOf } from './wikipage.ts';

const page = (number: number, title: string, parent: number | null = null, position = number): TopicLine =>
  lineOf({
    id: `@-topics-${number}`,
    number,
    position,
    data: { title, parent: parent === null ? null : `@-topics-${parent}` },
  });

// Two wikis of the same title (two workspaces'), a third, and pages under them.
const LINES = [
  page(1, 'Handbook'),
  page(2, 'Deploying', 1),
  page(3, 'Rolling back', 2),
  page(4, 'Getting started', 1),
  page(5, 'Handbook'),
  page(6, 'Rolling back', 5),
  page(7, 'Decisions'),
  page(8, 'Overview', 7),
  page(9, 'Overview', 7),
  page(10, 'Linker'),
  page(11, 'Gold', 3, 2),
  page(12, 'Mold', 3, 1),
];

test("a page's address says its wiki's title and its own, each only where it says which it is", () => {
  // A wiki whose title no other page has: its title alone.
  assert.deepEqual(addressNames(LINES, '@-topics-7'), { title: 'Decisions' });
  // A wiki whose title another wiki has: by its id.
  assert.deepEqual(addressNames(LINES, '@-topics-1'), {});
  // A page three deep, the one of its wiki called so: its title, and its wiki's id since two wikis share a title.
  assert.deepEqual(addressNames(LINES, '@-topics-3'), {
    title: 'Rolling back',
    wiki: '@-topics-1',
  });
  // One whose wiki's title is its alone.
  assert.deepEqual(addressNames(LINES, '@-topics-11'), {
    title: 'Gold',
    wiki: '@-topics-1',
  });
  // Two of one wiki called the same: by the id.
  assert.deepEqual(addressNames(LINES, '@-topics-8'), {});
  assert.deepEqual(addressNames(LINES, '@-topics-99'), {}, 'one not read');
  const lone = [page(1, 'Handbook'), page(2, 'Deploying', 1), page(3, 'Rolling back', 2)];
  assert.deepEqual(addressNames(lone, '@-topics-3'), {
    title: 'Rolling back',
    wiki: 'Handbook',
  });
  // Moved within its wiki, it keeps its address.
  const moved = [page(1, 'Handbook'), page(2, 'Deploying', 1), page(3, 'Rolling back', 1)];
  assert.deepEqual(addressNames(moved, '@-topics-3'), addressNames(lone, '@-topics-3'));
});

test('which page an address names: of its wiki first, then the one page of any wiki called so, and which they are where several are', () => {
  const found = (wiki: string | null, name: string) => {
    const it = namedIn(LINES, wiki, name);
    return it === null ? null : 'found' in it ? it.found.id : it.several.map((one) => one.id);
  };
  assert.equal(found('@-topics-1', 'rolling BACK'), '@-topics-3', 'whatever its capitals');
  assert.equal(found('@-topics-5', 'Rolling back'), '@-topics-6');
  // By the wiki's title, which two wikis share: the page called so of either.
  assert.deepEqual(found('Handbook', 'Rolling back'), ['@-topics-3', '@-topics-6']);
  assert.equal(found('Handbook', 'Gold'), '@-topics-11');
  // A wiki that has no page called so: the one page of any wiki that is (moved to another wiki, the old address opens it).
  assert.equal(found('Decisions', 'Gold'), '@-topics-11');
  assert.equal(found('No such wiki', 'Linker'), '@-topics-10');
  // An old address, a title alone.
  assert.equal(found(null, 'Getting started'), '@-topics-4');
  assert.deepEqual(found(null, 'Overview'), ['@-topics-8', '@-topics-9']);
  assert.equal(found(null, 'Nothing'), null);
  // An id is its page, wherever it is.
  assert.equal(found(null, '@-topics-12'), '@-topics-12');
  assert.equal(found('Decisions', '@-topics-12'), '@-topics-12');
  assert.equal(found(null, '@-topics-99'), null);
});

test('a long path keeps its wiki and the last two and folds the rest; a page is said by its path', () => {
  assert.deepEqual(crumbsOf(['a']), { shown: ['a'], folded: [] });
  assert.deepEqual(crumbsOf(['a', 'b', 'c']), {
    shown: ['a', 'b', 'c'],
    folded: [],
  });
  assert.deepEqual(crumbsOf(['a', 'b', 'c', 'd', 'e']), {
    shown: ['a', 'fold', 'd', 'e'],
    folded: ['b', 'c'],
  });
  assert.equal(pathSaid(['Handbook', 'Deploying'], 'Rolling back'), 'Handbook › Deploying › Rolling back');
  assert.equal(pathSaid([], 'Handbook'), 'Handbook');
});

test('the pages under a page in the order they stand, and whether a move is of a page here', () => {
  assert.deepEqual(underOf(LINES, '@-topics-3'), ['@-topics-12', '@-topics-11']);
  assert.deepEqual(underOf(LINES, '@-topics-4'), []);
  assert.ok(
    movedHereOrAway(
      {
        topic: 'topic.moved',
        data: {
          record: '@-topics-11',
          parent: '@-topics-3',
          was: { parent: '@-topics-2' },
        },
      },
      '@-topics-3',
    ),
  );
  assert.ok(
    movedHereOrAway(
      {
        topic: 'topic.moved',
        data: {
          record: '@-topics-11',
          parent: null,
          was: { parent: '@-topics-3' },
        },
      },
      '@-topics-3',
    ),
  );
  assert.ok(
    !movedHereOrAway(
      {
        topic: 'topic.moved',
        data: {
          record: '@-topics-11',
          parent: '@-topics-2',
          was: { parent: '@-topics-1' },
        },
      },
      '@-topics-3',
    ),
  );
  assert.ok(!movedHereOrAway({ topic: 'topic.revised', data: { record: '@-topics-3' } }, '@-topics-3'));
});

import assert from 'node:assert/strict';
import { test } from 'node:test';
import { DOC_TABS, detailsOf, tabOf, toolSaid } from './doctools.ts';
import type { Known } from './doctools.ts';

test('the panel beside a document has Comments, Details and History, in that order', () => {
  assert.deepEqual(
    DOC_TABS.map(({ name }) => name),
    ['Comments', 'Details', 'History'],
  );
});

test('the tab that shows is the one kept, and Comments where nothing that is a tab was kept', () => {
  assert.equal(tabOf('details'), 'details');
  assert.equal(tabOf('history'), 'history');
  assert.equal(tabOf('comments'), 'comments');
  assert.equal(tabOf(undefined), 'comments');
  assert.equal(tabOf('media'), 'comments');
  assert.equal(tabOf(3), 'comments');
});

test('the button in the head is called what it opens, with how many comments there are where there are any', () => {
  assert.equal(toolSaid(0), 'Comments and details');
  assert.equal(toolSaid(1), 'Comments and details, 1 comment');
  assert.equal(toolSaid(7), 'Comments and details, 7 comments');
});

const known: Known = {
  path: 'docs/guides/install.md',
  library: 'Shop docs',
  size: '1.2 kB',
  lines: 40,
  edited: 'Last edited by Ada · 2 min ago',
  public: false,
  marks: true,
  comments: 3,
};

test('Details says where the document is, its library, how big it is, when it was changed, whether it is public and how many comments it has', () => {
  assert.deepEqual(detailsOf(known), [
    ['Path', 'docs/guides/install.md'],
    ['Library', 'Shop docs'],
    ['Size', '1.2 kB · 40 lines'],
    ['Changed', 'Last edited by Ada · 2 min ago'],
    ['Public', 'No'],
    ['Comments', '3'],
  ]);
  assert.deepEqual(detailsOf({ ...known, public: true }).find(([what]) => what === 'Public'), ['Public', 'Yes: written out when its library is']);
  assert.deepEqual(detailsOf({ ...known, public: null }).find(([what]) => what === 'Public'), ['Public', 'Reading']);
});

test('Details says nothing of what is not known: no time, no mark where the library has none, no count before the history is read', () => {
  const said = detailsOf({ ...known, size: null, lines: 1, edited: null, marks: false, public: null, comments: null });
  assert.deepEqual(said, [
    ['Path', 'docs/guides/install.md'],
    ['Library', 'Shop docs'],
    ['Size', '1 line'],
  ]);
  assert.deepEqual(detailsOf({ ...known, comments: 0 }).at(-1), ['Comments', 'None yet']);
});

import assert from 'node:assert/strict';
import { test } from 'node:test';
import { byKind, goesSaid, keptSaid, movedSaid, type Trashed, whereSaid } from './trash.ts';

const DAY = 86_400_000;
const one = (id: string, kind: Trashed['kind'], at: number): Trashed => ({ id, kind, thing: id, project: 'shop', called: id, by: 'user:ada', who: 'Ada', at, goes: null, may: true });

test('what is in the Trash is drawn by kind, rows first, each kind the latest first, and no kind with nothing', () => {
  const all = [one('a', 'file', 1), one('b', 'record', 1), one('c', 'record', 3), one('d', 'file', 2)];
  const drawn = byKind(all);
  assert.deepEqual(drawn.map((group) => group.kind), ['record', 'file']);
  assert.deepEqual(drawn[0]!.things.map((thing) => thing.id), ['c', 'b']);
  assert.deepEqual(drawn[1]!.things.map((thing) => thing.id), ['d', 'a']);
  assert.deepEqual(byKind([]), []);
});

test('when a thing goes for good is said in days, today, now, or never', () => {
  assert.equal(goesSaid(null, 0), 'kept until it is deleted for good');
  assert.equal(goesSaid(30 * DAY, 0), 'goes for good in 30 days');
  assert.equal(goesSaid(DAY + 1, 0), 'goes for good in 2 days');
  assert.equal(goesSaid(DAY, 0), 'goes for good in a day');
  assert.equal(goesSaid(DAY - 1, 0), 'goes for good today');
  assert.equal(goesSaid(5, 10), 'goes for good now');
});

test('where a thing was is said by its kind', () => {
  const name = (id: string) => (id === 'shop' ? 'Shop' : id);
  assert.equal(whereSaid({ kind: 'record', project: 'shop', table: 'board', thing: 'shop-2' }, name), "a card of Shop's board");
  assert.equal(whereSaid({ kind: 'record', project: 'shop', table: 'notes', thing: 'shop-notes-2' }, name), "a row of Shop's notes");
  assert.equal(whereSaid({ kind: 'file', project: 'shop', thing: 'docs/plan.md' }, name), 'docs/plan.md in Shop');
  assert.equal(whereSaid({ kind: 'project', project: 'shop', thing: 'shop' }, name), 'a project, its files where they were');
  assert.equal(whereSaid({ kind: 'attachment', project: 'shop', table: 'board', thing: 'shop-2' }, name), 'attached to shop-2 of Shop');
  assert.equal(whereSaid({ kind: 'record', project: '@', table: 'topics', thing: '@-topics-2' }, name), 'a page of a wiki');
  assert.equal(whereSaid({ kind: 'record', project: '@', table: 'topics', thing: '@-topics-2', with: 1 }, name), 'a page of a wiki, with the page under it');
  assert.equal(whereSaid({ kind: 'record', project: '@', table: 'topics', thing: '@-topics-2', with: 4 }, name), 'a page of a wiki, with the 4 pages under it');
});

test('what is said as something is deleted says how long it is kept, and nought days is deleted', () => {
  assert.equal(movedSaid(30), 'Moved to Trash, kept 30 days.');
  assert.equal(movedSaid(1), 'Moved to Trash, kept a day.');
  assert.equal(movedSaid(null), 'Moved to Trash, kept until it is deleted for good.');
  assert.equal(movedSaid(0), 'Deleted.');
  assert.equal(movedSaid(30, 4), 'Moved to Trash, with the 4 pages under it, kept 30 days.');
  assert.equal(movedSaid(0, 1), 'Deleted, with the page under it.');
  assert.ok(keptSaid(0).includes('cannot be given back'));
  assert.ok(keptSaid(7).includes('7 days'));
  assert.ok(keptSaid(null).includes('until somebody deletes it'));
});

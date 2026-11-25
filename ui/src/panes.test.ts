import assert from 'node:assert/strict';
import { test } from 'node:test';
import { aim, close, dock, fresh, group, groups, holding, landing, mend, move, places, resize, show } from './panes.ts';
import type { Layout } from './panes.ts';

const one = group('g1', ['home', 'board', 'activity'], 'board');
const shape = (layout: Layout): unknown =>
  layout.kind === 'group' ? `${layout.id}[${layout.tabs.join(',')}]*${layout.active}` : { [layout.dir]: [shape(layout.first), shape(layout.second)] };

test('a new tab joins the group it is opened in and shows; one already open just shows where it is', () => {
  assert.equal(shape(show(one, 'notes', 'g1')), 'g1[home,board,activity,notes]*notes');
  assert.equal(shape(show(one, 'home', 'g1')), 'g1[home,board,activity]*home');
  const two = dock(one, 'activity', 'g1', 'right', 'g2');
  assert.deepEqual(shape(show(two, 'activity', 'g1')), { row: ['g1[home,board]*board', 'g2[activity]*activity'] }, 'not opened twice');
});

test('a tab dropped on an edge splits the group and takes the new half', () => {
  assert.deepEqual(shape(dock(one, 'activity', 'g1', 'right', 'g2')), { row: ['g1[home,board]*board', 'g2[activity]*activity'] });
  assert.deepEqual(shape(dock(one, 'board', 'g1', 'top', 'g2')), { column: ['g2[board]*board', 'g1[home,activity]*activity'] });
});

test('a group\'s only tab dropped on its own edge stays where it is', () => {
  const alone = group('g1', ['home']);
  assert.equal(dock(alone, 'home', 'g1', 'left', 'g2'), alone);
});

test('a tab dropped in the middle of another group joins its tabs', () => {
  const two = dock(one, 'activity', 'g1', 'right', 'g2');
  assert.deepEqual(shape(dock(two, 'home', 'g2', 'center', 'g3')), { row: ['g1[board]*board', 'g2[activity,home]*home'] });
});

test('a tab is put down before another, in its own group or someone else\'s', () => {
  assert.equal(shape(move(one, 'activity', 'g1', 'home')), 'g1[activity,home,board]*activity');
  const two = dock(one, 'activity', 'g1', 'right', 'g2');
  assert.deepEqual(shape(move(two, 'board', 'g2', 'activity')), { row: ['g1[home]*home', 'g2[board,activity]*board'] });
});

test('a group that loses its last tab goes, and what was beside it has the space', () => {
  const two = dock(one, 'activity', 'g1', 'right', 'g2');
  assert.equal(shape(close(two, 'activity')), 'g1[home,board]*board');
  assert.equal(shape(move(two, 'activity', 'g1', null)), 'g1[home,board,activity]*activity');
  const three = dock(two, 'home', 'g2', 'bottom', 'g3');
  assert.deepEqual(shape(close(three, 'board')), { column: ['g2[activity]*activity', 'g3[home]*home'] });
});

test('closing the tab that is showing shows the one beside it', () => {
  assert.equal(shape(close(one, 'board')), 'g1[home,activity]*activity');
  assert.equal(shape(close(group('g1', ['home', 'board'], 'board'), 'board')), 'g1[home]*home');
  assert.equal(shape(close(group('g1', ['home']), 'home')), 'g1[]*null', 'the last group stays, empty');
});

test('a divider is moved by naming its split, and never so far that a side disappears', () => {
  const three = dock(dock(one, 'activity', 'g1', 'right', 'g2'), 'home', 'g2', 'bottom', 'g3');
  const moved = resize(resize(three, '', 0.3), 's', 0.99);
  assert.equal(moved.kind === 'split' && moved.share, 0.3);
  assert.equal(moved.kind === 'split' && moved.second.kind === 'split' && moved.second.share, 0.85);
  assert.equal(holding(moved, 'home')?.id, 'g3');
  assert.equal(groups(moved).length, 3);
});

test('a remembered layout is made to fit the tabs that are open now', () => {
  const saved = JSON.parse(JSON.stringify(dock(one, 'activity', 'g1', 'right', 'g2')));
  assert.deepEqual(shape(mend(saved, ['home', 'board', 'activity'])), { row: ['g1[home,board]*board', 'g2[activity]*activity'] });
  // A tab that is gone leaves it; one it never knew joins the first group.
  assert.equal(shape(mend(saved, ['home', 'board', 'notes'])), 'g1[home,board,notes]*board');
  // And what is not a layout at all is one group of everything.
  assert.equal(shape(mend({ nonsense: true }, ['home', 'board'])), 'g1[home,board]*home');
  assert.equal(shape(mend(null, [])), 'g1[]*null');
});

test('each group and divider has its place, as a share of the whole', () => {
  const three = resize(dock(dock(one, 'activity', 'g1', 'right', 'g2'), 'home', 'g2', 'bottom', 'g3'), '', 0.25);
  const { groups: boxes, dividers } = places(three);
  assert.deepEqual(boxes, [
    { id: 'g1', x: 0, y: 0, w: 0.25, h: 1 },
    { id: 'g2', x: 0.25, y: 0, w: 0.75, h: 0.5 },
    { id: 'g3', x: 0.25, y: 0.5, w: 0.75, h: 0.5 },
  ]);
  assert.deepEqual(
    dividers.map((divider) => [divider.path, divider.dir, divider.within.x, divider.share]),
    [
      ['', 'row', 0, 0.25],
      ['s', 'column', 0.25, 0.5],
    ],
  );
});

test('where a tab is dropped says what happens to it', () => {
  const two = dock(one, 'activity', 'g1', 'right', 'g2');
  const strip = 0.05;
  assert.deepEqual(aim(two, 0.2, 0.02, strip), { group: 'g1', over: 'tabs' });
  assert.deepEqual(aim(two, 0.25, 0.5, strip), { group: 'g1', over: 'center' });
  assert.deepEqual(aim(two, 0.02, 0.5, strip), { group: 'g1', over: 'left' });
  assert.deepEqual(aim(two, 0.48, 0.5, strip), { group: 'g1', over: 'right' });
  assert.deepEqual(aim(two, 0.75, 0.1, strip), { group: 'g2', over: 'top' });
  assert.deepEqual(aim(two, 0.75, 0.97, strip), { group: 'g2', over: 'bottom' });
  assert.equal(aim(two, 1.2, 0.5, strip), null, 'outside the area');
  assert.deepEqual(landing({ x: 0.5, y: 0, w: 0.5, h: 1 }, 'bottom'), { x: 0.5, y: 0.5, w: 0.5, h: 0.5 });
  assert.deepEqual(landing({ x: 0.5, y: 0, w: 0.5, h: 1 }, 'center'), { x: 0.5, y: 0, w: 0.5, h: 1 });
});

test('a new group is given a name no group has', () => {
  assert.equal(fresh(one), 'g2');
  assert.equal(fresh(dock(dock(one, 'activity', 'g1', 'right', 'g7'), 'home', 'g7', 'top', 'g3')), 'g8');
});

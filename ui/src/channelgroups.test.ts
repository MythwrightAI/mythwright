import assert from 'node:assert/strict';
import { test } from 'node:test';
import { arranged, arrangedOf, groupingOf, madeSaid, NO_GROUPS, pinned, placed, renamed, withGroup, withoutGroup } from './channelgroups.ts';
import { arrangementOf } from './arranged.ts';

const channels = [{ id: 'c1' }, { id: 'c2' }, { id: 'c3' }, { id: 'c4' }];
const ids = (list: { id: string }[]) => list.map((one) => one.id);
/** What is listed, as each group's name and the channels under it. */
const listed = (groups: { group: { name: string }; channels: { id: string }[] }[]) => groups.map((one) => [one.group.name, ids(one.channels)]);

function work() {
  const made = withGroup(NO_GROUPS, ' Work ');
  assert.ok(made);
  return made;
}

test('with no groups the channels are listed as they always were', () => {
  const { loose, groups } = arranged(channels, NO_GROUPS);
  assert.deepEqual(ids(loose), ['c1', 'c2', 'c3', 'c4']);
  assert.deepEqual(groups, []);
});

test('a channel put in a group is listed under it, and the rest where they were, each in the order the channels come in', () => {
  const { grouping, id } = work();
  const put = placed(placed(grouping, 'c3', id), 'c1', id);
  const { loose, groups } = arranged(channels, put);
  assert.deepEqual(ids(loose), ['c2', 'c4']);
  assert.deepEqual(listed(groups), [['Work', ['c1', 'c3']]]);
});

test('a channel is in one group at a time, and taken out of its group is in none', () => {
  const first = work();
  const second = withGroup(first.grouping, 'Play');
  assert.ok(second);
  let put = placed(second.grouping, 'c2', first.id);
  put = placed(put, 'c2', second.id);
  assert.deepEqual(listed(arranged(channels, put).groups), [['Work', []], ['Play', ['c2']]]);
  put = placed(put, 'c2', null);
  assert.deepEqual(ids(arranged(channels, put).loose), ['c1', 'c2', 'c3', 'c4']);
  assert.equal(placed(put, 'c2', 'no-such-group').of.c2, undefined, 'a group that is not there is none');
});

test('a group has a name of its own: one of the same name is the same group, and a name of nothing makes none', () => {
  const { grouping, id } = work();
  assert.deepEqual(withGroup(grouping, 'work'), { grouping, id });
  assert.equal(withGroup(grouping, '   '), null);
  const second = withGroup(grouping, 'Play');
  assert.ok(second);
  assert.notEqual(second.id, id);
  assert.equal(renamed(second.grouping, second.id, 'Work'), second.grouping, 'not renamed to what another is called');
  assert.equal(renamed(second.grouping, second.id, ' '), second.grouping);
  assert.deepEqual(renamed(second.grouping, second.id, 'Later').groups.map((group) => group.name), ['Work', 'Later']);
  assert.deepEqual(renamed(second.grouping, id, 'WORK').groups.map((group) => group.name), ['WORK', 'Play'], 'its own name written another way');
});

test('a group taken away leaves its channels in none, and the other groups as they were', () => {
  const first = work();
  const second = withGroup(first.grouping, 'Play');
  assert.ok(second);
  const put = placed(placed(second.grouping, 'c1', first.id), 'c2', second.id);
  const left = withoutGroup(put, first.id);
  const { loose, groups } = arranged(channels, left);
  assert.deepEqual(ids(loose), ['c1', 'c3', 'c4']);
  assert.deepEqual(listed(groups), [['Play', ['c2']]]);
  // And one made after another was taken away is not given the name of one that is still there.
  const third = withGroup(left, 'Other');
  assert.ok(third);
  assert.notEqual(third.id, second.id);
});

test('a channel that is gone is not listed, and its group is still there to put others in', () => {
  const { grouping, id } = work();
  const put = placed(grouping, 'gone', id);
  const { loose, groups } = arranged(channels, put);
  assert.deepEqual(ids(loose), ['c1', 'c2', 'c3', 'c4']);
  assert.deepEqual(listed(groups), [['Work', []]]);
});

test('what was kept comes back as it was, and what is not a grouping comes back as none', () => {
  const { grouping, id } = work();
  const put = placed(grouping, 'c2', id);
  assert.deepEqual(groupingOf(JSON.stringify(put)), put);
  for (const not of [null, undefined, '', 'groups', '[]', '{"groups":"many"}', '42']) {
    assert.deepEqual(groupingOf(not), NO_GROUPS, String(not));
  }
  // What of it makes no sense is left out: a group with no name, the same group twice, a channel in a group that is not there.
  const odd = { groups: [{ id: 'g1', name: 'Work' }, { id: 'g1', name: 'Again' }, { id: 'g2', name: ' ' }, 7], of: { c1: 'g1', c2: 'g9', c3: 5 } };
  assert.deepEqual(groupingOf(JSON.stringify({ ...odd, pinned: ['c1', 'c1', 4] })), { groups: [{ id: 'g1', name: 'Work' }], of: { c1: 'g1' }, pinned: ['c1'] });
});

test('a pinned channel is listed at the top and nowhere else, and is where it was when it is no longer pinned', () => {
  const { grouping, id } = work();
  let put = placed(grouping, 'c3', id);
  put = pinned(pinned(put, 'c3', true), 'c2', true);
  let listing = arranged(channels, put);
  assert.deepEqual(ids(listing.pinned), ['c2', 'c3'], 'as the channels come, not as they were pinned');
  assert.deepEqual(ids(listing.loose), ['c1', 'c4']);
  assert.deepEqual(listed(listing.groups), [['Work', []]]);
  assert.deepEqual(pinned(put, 'c2', true).pinned, ['c3', 'c2'], 'pinned twice is pinned once');
  put = pinned(put, 'c3', false);
  listing = arranged(channels, put);
  assert.deepEqual(ids(listing.pinned), ['c2']);
  assert.deepEqual(listed(listing.groups), [['Work', ['c3']]], 'back in its group');
});

test('a pinned channel that is put in a group, or in none, is listed there and is no longer pinned', () => {
  const { grouping, id } = work();
  let put = pinned(pinned(grouping, 'c1', true), 'c2', true);
  put = placed(put, 'c1', id);
  put = placed(put, 'c2', null);
  const listing = arranged(channels, put);
  assert.deepEqual(ids(listing.pinned), []);
  assert.deepEqual(ids(listing.loose), ['c2', 'c3', 'c4']);
  assert.deepEqual(listed(listing.groups), [['Work', ['c1']]]);
  // One that is gone is not listed at the top either.
  assert.deepEqual(ids(arranged(channels, pinned(grouping, 'gone', true)).pinned), []);
});

test('where only some channels are listed, a group with none of them is not there, an empty one is, and nothing is forgotten', () => {
  let put = work().grouping;
  put = withGroup(put, 'Play')!.grouping;
  put = withGroup(put, 'Later')!.grouping;
  put = placed(placed(placed(put, 'c1', 'g1'), 'c2', 'g1'), 'c3', 'g2');
  put = pinned(put, 'c4', true);
  // Everything listed is what it always was.
  assert.deepEqual(arrangedOf(channels, put, () => true), arranged(channels, put));
  // Some of a group's: those of it that are listed.
  const some = arrangedOf(channels, put, (channel) => channel.id === 'c2' || channel.id === 'c4');
  assert.deepEqual(listed(some.groups), [['Work', ['c2']], ['Later', []]]);
  assert.deepEqual(ids(some.pinned), ['c4']);
  // None of a group's: the group is not drawn. One that never held anything is, to put something in.
  const none = arrangedOf(channels, put, (channel) => channel.id === 'c3');
  assert.deepEqual(listed(none.groups), [['Play', ['c3']], ['Later', []]]);
  assert.deepEqual(ids(none.pinned), []);
  assert.deepEqual(ids(none.loose), []);
  // And the grouping is what it was.
  assert.deepEqual(listed(arranged(channels, put).groups), [['Work', ['c1', 'c2']], ['Play', ['c3']], ['Later', []]]);
});

test('a channel made from a group\'s menu is kept at the end of that group, as one dragged there is', () => {
  const { grouping, id } = work();
  const put = placed(placed(grouping, 'c3', id), 'c1', id);
  const made = placed(put, 'made', id);
  assert.deepEqual(arrangementOf(made, []).groups[0]!.channels, ['c3', 'c1', 'made']);
  assert.equal(placed(put, 'made', 'gone').of.made, undefined, 'a group taken away meanwhile puts it in none');
});

test('what is said of a channel made from a group\'s menu: nothing where the group draws it, else where it went', () => {
  assert.equal(madeSaid('#news', 'Work', null), null);
  assert.equal(madeSaid('#news', 'Work', 'Shop front'), '#news is in Work, listed there while Shop front is showing.');
  assert.equal(madeSaid('Plans', null, null), 'Plans is in no group: the group it was made from is no longer there.');
  assert.equal(madeSaid('#news', null, 'Shop front'), '#news is in no group: the group it was made from is no longer there.');
});

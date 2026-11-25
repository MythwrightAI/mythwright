import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import { arrangementOf, copyOf, copyRead, groupingFrom, isLater, NOTHING_ARRANGED, takenUp, thingsFrom, withGrouping, withThings } from './arranged.ts';
import type { Arrangement } from './arranged.ts';
import { LONGEST, NO_GROUPS, pinned, placed, withGroup } from './channelgroups.ts';
import type { Tab } from './tab.ts';

const table = JSON.parse(readFileSync(new URL('./arrangedcases.json', import.meta.url), 'utf8')) as { longest: number; cases: [unknown, string | null][] };

/** Work with two channels in it, one pinned, and a board pinned in Resources. */
function mine() {
  const made = withGroup(NO_GROUPS, 'Work')!;
  const grouping = pinned(placed(placed(made.grouping, 'c1', made.id), 'c2', made.id), 'c3', true);
  const things: Tab[] = [{ kind: 'board', project: 'shop' }, { kind: 'file', project: 'shop', path: 'docs/a b.md' }];
  return { grouping, things, arrangement: arrangementOf(grouping, things) };
}

test('a name is held to as many letters as the daemon keeps, by the same table it is held to', () => {
  assert.equal(LONGEST, table.longest);
  assert.equal(withGroup(NO_GROUPS, 'a'.repeat(LONGEST + 1)), null);
  assert.ok(withGroup(NO_GROUPS, 'a'.repeat(LONGEST)));
});

test('an arrangement the daemon keeps comes to the groups and things it says and back to itself', () => {
  for (const [arrangement, refused] of table.cases) {
    if (refused !== null) continue;
    const said = { ...NOTHING_ARRANGED, ...(arrangement as Partial<Arrangement>) };
    said.groups = said.groups.map((group) => ({ ...group, channels: group.channels ?? [] }));
    const back = arrangementOf(groupingFrom(said), thingsFrom(said));
    assert.deepEqual(back.groups, said.groups, JSON.stringify(arrangement));
    assert.deepEqual(back.pinned, said.pinned);
  }
});

test('groups, pins and things are kept as text and drawn as they were', () => {
  const { grouping, things, arrangement } = mine();
  assert.deepEqual(arrangement, {
    groups: [{ id: 'g1', name: 'Work', channels: ['c1', 'c2'] }],
    pinned: ['c3'],
    things: ['/board/shop', '/file/shop/docs/a%20b.md'],
  });
  assert.deepEqual(groupingFrom(arrangement), grouping);
  assert.deepEqual(thingsFrom(arrangement), things);
  assert.equal(groupingFrom(NOTHING_ARRANGED), NO_GROUPS);
});

test('an address that opens nothing kept to hand is passed over, and one said twice is drawn once', () => {
  const said = { ...NOTHING_ARRANGED, things: ['/board/shop', 'nonsense', '/reference', '/board/shop'] };
  assert.deepEqual(thingsFrom(said), [{ kind: 'board', project: 'shop' }]);
});

test('changing the groups leaves the things, and changing the things leaves the groups', () => {
  const { arrangement } = mine();
  const ungrouped = withGrouping(arrangement, NO_GROUPS);
  assert.deepEqual(ungrouped, { groups: [], pinned: [], things: arrangement.things });
  const unpinned = withThings(arrangement, []);
  assert.deepEqual([unpinned.groups, unpinned.pinned, unpinned.things], [arrangement.groups, arrangement.pinned, []]);
});

test('what a browser kept before is sent only where the account keeps nothing and the browser has something', () => {
  const { grouping, things, arrangement } = mine();
  assert.deepEqual(takenUp(null, { grouping, pinned: things }), { send: arrangement });
  assert.deepEqual(takenUp(NOTHING_ARRANGED, { grouping, pinned: things }), { send: null }, 'an account that keeps one, even empty, is not merged into');
  assert.deepEqual(takenUp(null, { grouping: NO_GROUPS, pinned: [] }), { send: null }, 'nothing is sent of nothing');
});

test('of two things heard, the later is the one written later', () => {
  assert.equal(isLater(2, 3), true);
  assert.equal(isLater(3, 3), false);
  assert.equal(isLater(3, 2), false);
});

test('the browser copies nothing of a channel it does not list or a thing it knows to be gone, and all while nothing was read', () => {
  const { arrangement } = mine();
  const gone = (thing: Tab) => thing.kind === 'file';
  assert.deepEqual(copyOf(arrangement, new Set(['c1']), gone), {
    groups: [{ id: 'g1', name: 'Work', channels: ['c1'] }],
    pinned: [],
    things: ['/board/shop'],
  });
  assert.deepEqual(copyOf(arrangement, null, () => false), arrangement);
});

test('a copy is read back as it was, and what is not one is nothing', () => {
  const { arrangement } = mine();
  assert.deepEqual(copyRead(JSON.stringify(arrangement)), arrangement);
  for (const not of [null, '', '{', '7', '[]']) assert.equal(copyRead(not), null, String(not));
  assert.deepEqual(copyRead(JSON.stringify({ groups: [{ id: 'g1' }, { id: 'g2', name: 'Play', channels: ['c1', 4] }], pinned: 'c1' })), {
    groups: [{ id: 'g2', name: 'Play', channels: ['c1'] }],
    pinned: [],
    things: [],
  });
});

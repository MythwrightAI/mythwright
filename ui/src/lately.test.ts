import assert from 'node:assert/strict';
import { test } from 'node:test';
import { cleaned, forgot, heldStill, isGone, isPinned, isThing, KEPT, keptOf, MOST, NOTHING, opened, pin, pinnedNow, projectOf, recentAll, recentNow, said, unlisted } from './lately.ts';
import type { Kept, Known } from './lately.ts';
import type { Tab } from './tab.ts';
import { tabKey } from './tab.ts';
import { ALL } from './workspaces.ts';
import type { Held, Workspace } from './workspaces.ts';

const board = (project: string): Tab => ({ kind: 'board', project });
const page = (id: string): Tab => ({ kind: 'project', id });
const file = (project: string, path: string): Tab => ({ kind: 'file', project, path });
const table = (project: string, name: string): Tab => ({ kind: 'table', project, name });
const running = (id: string): Tab => ({ kind: 'instance', id });
// Something of the factory's own, which is of no project: a blueprint that ships.
const pool: Tab = { kind: 'blueprint', name: 'std/pool' };
const keys = (things: Tab[]) => things.map(tabKey);
const all = (things: Tab[]) => things.reduce(opened, NOTHING);

const space = (id: string, projects: string[]): Workspace => ({ id, name: id, created: 0, projects, channels: [] });
const spaces = [space('back', ['depot']), space('front', ['shop'])];
const front: Held = { kind: 'space', id: 'front' };
const back: Held = { kind: 'space', id: 'back' };
const known: Known = {
  projects: ['shop', 'depot', 'shed'],
  declared: { shop: ['board', 'stages'], depot: ['board'] },
  running: { in_1: { project: 'shop' }, in_2: { project: null }, in_3: 'gone' },
};

test('what was lately opened is listed the latest first, and no more than eight', () => {
  const ten = Array.from({ length: 10 }, (_none, nth) => file('shop', `notes/${nth}.md`));
  const kept = all(ten);
  assert.deepEqual(keys(kept.recent), keys([...ten].reverse()));
  const listed = recentNow(kept, known, ALL, []);
  assert.equal(listed.length, MOST);
  assert.deepEqual(keys(listed), keys([...ten].reverse().slice(0, 8)));
});

test('one opened again moves up, and is listed once', () => {
  const kept = opened(all([board('shop'), pool, page('shop')]), board('shop'));
  assert.deepEqual(keys(kept.recent), ['board:shop', 'project:shop', 'blueprint::std/pool']);
  // The one that is the latest already changes nothing.
  assert.equal(opened(kept, board('shop')), kept);
});

test('what is kept has an end: the oldest goes when there are more than are kept', () => {
  const many = Array.from({ length: KEPT + 5 }, (_none, nth) => file('shop', `${nth}.md`));
  const kept = all(many);
  assert.equal(kept.recent.length, KEPT);
  assert.equal(tabKey(kept.recent[0]!), `file:shop:${KEPT + 4}.md`);
});

test('a channel, the start page, settings, a package, the factory\'s pages that are always listed, a line of the inbox and one run are not things kept to hand', () => {
  // A line of the inbox has a page and an address, and is a message to its owner, no thing of the factory's work.
  const not: Tab[] = [{ kind: 'channel', id: 'c1' }, { kind: 'home' }, { kind: 'settings' }, { kind: 'secrets' }, { kind: 'profile' }, { kind: 'extension', name: 'x' }, { kind: 'trash' }, { kind: 'activity' }, { kind: 'metrics' }, { kind: 'members' }, { kind: 'inbox', id: 'inb_1' }, { kind: 'run', id: 'run_1' }];
  for (const tab of not) {
    assert.equal(isThing(tab), false, tab.kind);
    assert.equal(opened(NOTHING, tab), NOTHING);
    assert.equal(pin(NOTHING, tab, true), NOTHING);
  }
  const things: Tab[] = [page('shop'), board('shop'), table('shop', 'stages'), { kind: 'library', project: 'shop', name: 'docs' }, { kind: 'mockups', project: 'shop', name: 'mockups' }, { kind: 'repo', project: 'shop' }, { kind: 'git', project: 'shop' }, file('shop', 'a.md'), { kind: 'blueprint', name: 'std/board' }, { kind: 'blueprint', name: 'blueprint::std/pool', project: 'shop' }, running('in_1'), pool, { kind: 'pipelines', project: 'shop' }];
  for (const tab of things) assert.equal(isThing(tab), true, tab.kind);
  assert.equal(projectOf({ kind: 'pipelines', project: 'shop' }, known), 'shop');
  assert.equal(all(things).recent.length, things.length);
});

test('one that is pinned is listed where it was pinned and not among what was lately opened', () => {
  let kept = all([board('shop'), pool, page('shop')]);
  kept = pin(kept, pool, true);
  kept = pin(kept, board('shop'), true);
  assert.deepEqual(keys(pinnedNow(kept, known, ALL, [])), ['blueprint::std/pool', 'board:shop']);
  assert.deepEqual(keys(recentNow(kept, known, ALL, [])), ['project:shop']);
  assert.ok(isPinned(kept, pool) && !isPinned(kept, page('shop')));
  // Pinned again it is where it was; opened again it is still not among what was lately opened.
  assert.equal(pin(kept, pool, true), kept);
  assert.deepEqual(keys(recentNow(opened(kept, pool), known, ALL, [])), ['project:shop']);
});

test('one that is unpinned is back among what was lately opened, where it belongs by when it was opened', () => {
  let kept = pin(all([board('shop'), pool, page('shop')]), pool, true);
  kept = pin(kept, pool, false);
  assert.deepEqual(keys(pinnedNow(kept, known, ALL, [])), []);
  assert.deepEqual(keys(recentNow(kept, known, ALL, [])), ['project:shop', 'blueprint::std/pool', 'board:shop']);
  // One pinned that was never opened was looked at as it was pinned.
  const never = pin(pin(all([pool]), board('depot'), true), board('depot'), false);
  assert.deepEqual(keys(recentNow(never, known, ALL, [])), ['board:depot', 'blueprint::std/pool']);
});

test('one that is gone is not listed, and is taken out of what was kept', () => {
  const kept = pin(all([board('gone'), page('gone'), file('gone', 'a.md'), table('shop', 'old'), table('shop', 'stages'), board('shop'), running('in_3'), running('in_1'), pool]), page('gone'), true);
  assert.deepEqual(keys(recentNow(kept, known, ALL, [])), ['blueprint::std/pool', 'instance:in_1', 'board:shop', 'table:shop:stages']);
  assert.deepEqual(keys(pinnedNow(kept, known, ALL, [])), []);
  const left = cleaned(kept, known);
  assert.deepEqual(keys(left.recent), ['blueprint::std/pool', 'instance:in_1', 'board:shop', 'table:shop:stages']);
  assert.deepEqual(left.pinned, []);
  // Where nothing is gone, what is kept is the very thing it was.
  assert.equal(cleaned(left, known), left);
});

test('what was not read says nothing of what is gone', () => {
  const unread: Known = { projects: null, declared: {}, running: {} };
  for (const thing of [board('shop'), page('anything'), table('shop', 'x'), running('in_9'), file('shop', 'a.md')]) assert.equal(isGone(thing, unread), false);
  // A project that was read, whose tables were not: its board is not known to be gone.
  assert.equal(isGone(board('shed'), known), false);
  // A file goes with its project and not before.
  assert.equal(isGone(file('shop', 'never/seen.md'), known), false);
  assert.equal(isGone(file('gone', 'a.md'), known), true);
  // Something running that was not read is listed nowhere until it is, and is not taken out.
  const kept = all([running('in_9')]);
  assert.deepEqual(recentNow(kept, known, ALL, []), []);
  assert.equal(cleaned(kept, known), kept);
});

test('whose a thing is: its project, nobody for the factory\'s own, and not known for something running that was not read', () => {
  assert.equal(projectOf(board('shop'), known), 'shop');
  assert.equal(projectOf(page('shop'), known), 'shop');
  assert.equal(projectOf({ kind: 'blueprint', name: 'blueprint::std/pool', project: 'shop' }, known), 'shop');
  assert.equal(projectOf({ kind: 'blueprint', name: 'std/board' }, known), null);
  assert.equal(projectOf(pool, known), null);
  assert.equal(projectOf(running('in_1'), known), 'shop');
  assert.equal(projectOf(running('in_2'), known), null);
  assert.equal(projectOf(running('in_9'), known), undefined);
});

test('with a workspace showing, only what is of its projects is listed, and what is the factory\'s own', () => {
  const kept = pin(pin(all([board('shop'), board('depot'), page('shed'), pool, running('in_1'), running('in_2')]), board('depot'), true), page('shop'), true);
  assert.deepEqual(keys(recentNow(kept, known, front, spaces)), ['instance:in_2', 'instance:in_1', 'blueprint::std/pool', 'board:shop']);
  assert.deepEqual(keys(pinnedNow(kept, known, front, spaces)), ['project:shop']);
  // What is of a project whose place was not read is with the usual workspace's, which is where the daemon puts it.
  assert.deepEqual(keys(recentNow(kept, known, back, spaces)), ['instance:in_2', 'blueprint::std/pool', 'project:shed']);
  assert.deepEqual(keys(pinnedNow(kept, known, back, spaces)), ['board:depot']);
  // Until the workspaces are read everything is listed: the two that are pinned, and the five that are not.
  assert.equal(pinnedNow(kept, known, ALL, []).length, 2);
  assert.equal(recentNow(kept, known, ALL, []).length, 5);
  // Eight of the workspace showing, however many of another were opened since.
  const busy = [...Array.from({ length: 8 }, (_none, nth) => file('shop', `${nth}.md`)), ...Array.from({ length: 20 }, (_none, nth) => file('depot', `${nth}.md`))].reduce(opened, NOTHING);
  assert.equal(recentNow(busy, known, front, spaces).length, 8);
});

test('the list holds still while the panel is pointed at: one pressed stays where it was, and nothing comes into it', () => {
  const first = all([board('shop'), pool, page('shop')]);
  const was = keys(recentNow(first, known, ALL, []));
  assert.deepEqual(was, ['project:shop', 'blueprint::std/pool', 'board:shop']);
  // One of them pressed: it is the latest, and is drawn where it was.
  const pressed = opened(first, board('shop'));
  assert.deepEqual(keys(recentNow(pressed, known, ALL, [])), ['board:shop', 'project:shop', 'blueprint::std/pool']);
  assert.deepEqual(keys(heldStill(was, recentAll(pressed, known, ALL, []))), was);
  // Something else opened meanwhile is not in it until the pointer has left: it would push the rest down.
  const came = opened(pressed, page('depot'));
  assert.deepEqual(keys(heldStill(was, recentAll(came, known, ALL, []))), was);
  // What can no longer be listed at all is gone from it: one that was pinned, one whose project was forgotten.
  const went = pin(came, pool, true);
  assert.deepEqual(keys(heldStill(was, recentAll(went, known, ALL, []))), ['project:shop', 'board:shop']);
  assert.deepEqual(keys(heldStill(was, recentAll(came, { ...known, projects: ['depot'] }, ALL, []))), ['blueprint::std/pool']);
});

test('a full list held still keeps its eight, though a ninth thing is the latest', () => {
  const eight = Array.from({ length: 8 }, (_none, nth) => file('shop', `${nth}.md`));
  const first = all(eight);
  const was = keys(recentNow(first, known, ALL, []));
  const next = opened(first, pool);
  // Let go of, the oldest has given way to the latest; held, it is still there, and the latest is not.
  assert.deepEqual(keys(recentNow(next, known, ALL, [])), ['blueprint::std/pool', ...was.slice(0, 7)]);
  assert.deepEqual(keys(heldStill(was, recentAll(next, known, ALL, []))), was);
});

test('what a browser from before kept, which has neither, reads as none; and what makes no sense is left out', () => {
  for (const saved of [null, undefined, '', '{', 'null', '[]', '7', '{"groups":[],"of":{},"pinned":["c1"]}', '{"recent":"x","pinned":7}']) {
    assert.deepEqual(keptOf(saved), NOTHING);
  }
  const kept: Kept = pin(all([board('shop'), pool, file('shop', 'a.md')]), pool, true);
  assert.deepEqual(keptOf(JSON.stringify(kept)), kept);
  const odd = JSON.stringify({
    recent: [{ kind: 'board' }, { kind: 'board', project: 'shop' }, { kind: 'board', project: 'shop' }, { kind: 'channel', id: 'c1' }, { kind: 'home' }, { kind: 'fleet' }, 'std/pool', null, { kind: 'file', project: 'shop' }, { kind: 'blueprint', name: 'x', project: 7 }],
    pinned: [{ kind: 'blueprint', name: 'std/pool', extra: true }, { kind: 'nothing' }, { kind: 'metrics' }],
  });
  assert.deepEqual(keptOf(odd), { recent: [board('shop'), { kind: 'blueprint', name: 'x' }], pinned: [pool] });
});

test('where two lines read alike each says whose it is, and one that reads like no other says nothing more', () => {
  const things = [file('shop', 'README.md'), file('depot', 'docs/README.md'), file('shop', 'notes.md'), { kind: 'blueprint', name: 'std/board' } as Tab, { kind: 'blueprint', name: 'std/board', project: 'shop' } as Tab];
  const title = (thing: Tab) => (thing.kind === 'file' ? thing.path.split('/').pop()! : thing.kind === 'blueprint' ? thing.name : thing.kind);
  const whose = (thing: Tab) => projectOf(thing, known) ?? null;
  assert.deepEqual(
    said(things, title, whose).map((line) => [line.title, line.whose]),
    [
      ['README.md', 'shop'],
      ['README.md', 'depot'],
      ['notes.md', null],
      ['std/board', null],
      ['std/board', 'shop'],
    ],
  );
});

test('a thing somebody took away themselves leaves both lists at once, and nothing else does', () => {
  const kept: Kept = { recent: [file('shop', 'a.md'), board('shop'), file('shop', 'b.md')], pinned: [file('shop', 'a.md'), page('shop')] };
  assert.deepEqual(forgot(kept, file('shop', 'a.md')), { recent: [board('shop'), file('shop', 'b.md')], pinned: [page('shop')] });
  // What is in neither changes nothing: the same thing is handed back.
  assert.equal(forgot(kept, file('depot', 'a.md')), kept);
});

test('one removed from Recents is listed no longer, is the latest again once it is opened, and one pinned is left as it is', () => {
  const kept = pin(all([board('shop'), pool, page('shop')]), page('shop'), true);
  const removed = unlisted(kept, pool);
  assert.deepEqual(keys(removed.recent), keys([page('shop'), board('shop')]));
  assert.deepEqual(removed.pinned, kept.pinned);
  assert.deepEqual(keys(recentNow(removed, known, ALL, [])), keys([board('shop')]));
  assert.deepEqual(keys(opened(removed, pool).recent), keys([pool, page('shop'), board('shop')]));
  // Pinned, it is not in Recents, and removing it from them changes nothing.
  assert.equal(unlisted(kept, page('shop')), kept);
  // What was never there changes nothing.
  assert.equal(unlisted(kept, file('shop', 'a.md')), kept);
});

test('one removed from Recents leaves the list at once while it is held still', () => {
  const kept = all([board('shop'), pool, page('shop')]);
  const was = keys(recentNow(kept, known, ALL, []));
  const removed = unlisted(kept, pool);
  assert.deepEqual(keys(heldStill(was, recentAll(removed, known, ALL, []))), keys([page('shop'), board('shop')]));
});

// ---- a forum's topic

test("a topic is a thing kept to hand, kept by its id alone, and listed in its place's workspace once the topics are read", () => {
  const topic: Tab = { kind: 'topic', id: '@-topics-3', title: 'Builds' };
  const own: Tab = { kind: 'topic', id: '@-topics-4' };
  assert.ok(isThing(topic));
  const kept = all([topic, own]);
  assert.deepEqual(kept.recent[1], { kind: 'topic', id: '@-topics-3' }, 'what it was called is no part of what is kept');
  const read: Known = { ...known, topics: { '@-topics-3': { workspace: null, project: 'shop' }, '@-topics-4': { workspace: 'back', project: null } } };
  assert.deepEqual(keys(recentNow(kept, read, front, spaces)), [tabKey(topic)]);
  assert.deepEqual(keys(recentNow(kept, read, back, spaces)), [tabKey(own)]);
  // Not read yet: listed nowhere, and gone from nothing.
  assert.deepEqual(recentNow(kept, known, front, spaces), []);
  assert.equal(isGone(topic, known), false);
  // Read, and not there: gone.
  assert.equal(isGone(topic, { ...known, topics: {} }), true);
  assert.deepEqual(cleaned(kept, { ...known, topics: { '@-topics-4': { workspace: 'back', project: null } } }).recent, [own]);
  assert.deepEqual(keptOf(JSON.stringify({ recent: [{ kind: 'topic', id: '@-topics-3' }, { kind: 'topic' }], pinned: [] })).recent, [{ kind: 'topic', id: '@-topics-3' }]);
});

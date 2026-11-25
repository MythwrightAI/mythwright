import assert from 'node:assert/strict';
import { test } from 'node:test';
import { dock, group, groups, show } from './panes.ts';
import type { Layout } from './panes.ts';
import { tabKey } from './tab.ts';
import type { Tab } from './tab.ts';
import { among, before, first, forgot, isGone, keptOf, setOf, showingIn, standingSet, start, switched } from './tabskept.ts';
import type { Known, Sets, TabSet } from './tabskept.ts';
import { ALL, spaceNear, standing } from './workspaces.ts';
import type { Workspace } from './workspaces.ts';

const HOME: Tab = { kind: 'home' };
const board = (project: string): Tab => ({ kind: 'board', project });
const page = (id: string): Tab => ({ kind: 'project', id });
const channel = (id: string): Tab => ({ kind: 'channel', id });
const shape = (layout: Layout): unknown =>
  layout.kind === 'group' ? `${layout.id}[${layout.tabs.join(',')}]*${layout.active}` : { [layout.dir]: [shape(layout.first), shape(layout.second)] };
/** A set as it is said here: its tabs by key, how they are arranged, and the group last used. */
const said = (set: TabSet) => ({ tabs: set.tabs.map(tabKey), layout: shape(set.layout), focused: set.focused });
/** One group of tabs, the last of them showing. */
const row = (...tabs: Tab[]): TabSet => {
  const keys = tabs.map(tabKey);
  return { tabs, layout: group('g1', keys, keys.at(-1)), focused: 'g1' };
};
/** Two groups side by side: the first tabs in one, the last in a second, which is the one last used. */
const split = (...tabs: Tab[]): TabSet => {
  const keys = tabs.map(tabKey);
  const one = group('g1', keys, keys.at(-2));
  return { tabs, layout: dock(one, keys.at(-1)!, 'g1', 'right', 'g2'), focused: 'g2' };
};
const everything: Known = { projects: ['shop', 'depot'], channels: ['chat', 'plans'] };

// ---- what a browser kept, read

test('a set that was kept is read as it was: its tabs, its groups, the tab showing in each and the group last used', () => {
  const kept = split(HOME, board('shop'), channel('chat'));
  assert.deepEqual(said(setOf(JSON.parse(JSON.stringify(kept)))!), {
    tabs: ['home', 'board:shop', 'channel:chat'],
    layout: { row: ['g1[home,board:shop]*board:shop', 'g2[channel:chat]*channel:chat'] },
    focused: 'g2',
  });
});

test('each tab is read once, a group last used that is not there is the first, and tabs the arrangement left out join the first group', () => {
  const read = setOf({ tabs: [HOME, board('shop'), board('shop'), channel('chat')], layout: group('g1', ['home', 'board:shop'], 'board:shop'), focused: 'g7' })!;
  assert.deepEqual(said(read), { tabs: ['home', 'board:shop', 'channel:chat'], layout: 'g1[home,board:shop,channel:chat]*board:shop', focused: 'g1' });
});

test('what was kept before there were groups is one row with the tab that was showing', () => {
  assert.deepEqual(said(setOf({ tabs: [HOME, board('shop')], active: 'board:shop' })!), { tabs: ['home', 'board:shop'], layout: 'g1[home,board:shop]*board:shop', focused: 'g1' });
});

test('a kind of tab there is no longer is left out as a set is read, and a set of only such tabs is nothing kept', () => {
  const kept = { tabs: [HOME, { kind: 'fleet' }, board('shop')], layout: group('g1', ['home', 'fleet', 'board:shop'], 'fleet'), focused: 'g1' };
  assert.deepEqual(said(setOf(JSON.parse(JSON.stringify(kept)))!), { tabs: ['home', 'board:shop'], layout: 'g1[home,board:shop]*home', focused: 'g1' });
  assert.equal(setOf({ tabs: [{ kind: 'fleet' }] }), null);
});

test('the Agents page kept from before it was a tab of Members is the Members page at its Agents tab, where it was and showing as it was', () => {
  const kept = { tabs: [HOME, { kind: 'agents' }, board('shop')], layout: dock(group('g1', ['home', 'agents'], 'agents'), 'board:shop', 'g1', 'right', 'g2'), focused: 'g1' };
  const read = setOf(JSON.parse(JSON.stringify(kept)))!;
  assert.deepEqual(said(read), { tabs: ['home', 'members', 'board:shop'], layout: { row: ['g1[home,members]*members', 'g2[board:shop]*board:shop'] }, focused: 'g1' });
  assert.deepEqual(read.tabs[1], { kind: 'members', at: 'agents' });
  // Kept before there were groups, it is the one showing as it was.
  assert.deepEqual(said(setOf({ tabs: [{ kind: 'agents' }, HOME], active: 'agents' })!), { tabs: ['members', 'home'], layout: 'g1[members,home]*members', focused: 'g1' });
  // Where the Members page was kept open too, it is the one tab, as that page was kept.
  const both = setOf({ tabs: [{ kind: 'members' }, { kind: 'agents' }], layout: group('g1', ['members', 'agents'], 'agents'), focused: 'g1' })!;
  assert.deepEqual(said(both), { tabs: ['members'], layout: 'g1[members]*members', focused: 'g1' });
  assert.deepEqual(both.tabs, [{ kind: 'members' }]);
});

test('what is no set is nothing kept', () => {
  for (const kept of [null, undefined, 4, 'tabs', {}, { tabs: 'home' }, { tabs: [] }, { tabs: [null, 'home', {}] }]) assert.equal(setOf(kept), null);
  for (const kept of [null, undefined, '', 'not json', '4', 'null']) assert.equal(before(kept), null);
  assert.deepEqual(said(before(JSON.stringify(row(HOME, board('shop'))))!), said(row(HOME, board('shop'))));
});

test('what a browser kept for each workspace is read by workspace, and what of it is no set is left out', () => {
  const kept = keptOf(JSON.stringify({ front: row(HOME, board('shop')), back: { tabs: [] }, yard: 'nothing' }));
  assert.deepEqual(Object.keys(kept), ['front']);
  assert.deepEqual(said(kept.front!), said(row(HOME, board('shop'))));
  for (const nothing of [null, undefined, '', 'not json', '[]', '4']) assert.deepEqual(keptOf(nothing), {});
});

test('the start page alone is what a workspace with nothing kept shows', () => {
  assert.deepEqual(said(start()), { tabs: ['home'], layout: 'g1[home]*home', focused: 'g1' });
});

test('the tab showing in the group last used', () => {
  assert.deepEqual(showingIn(split(HOME, board('shop'), channel('chat'))), channel('chat'));
  assert.deepEqual(showingIn({ ...split(HOME, board('shop'), channel('chat')), focused: 'g1' }), board('shop'));
  assert.deepEqual(showingIn(start()), HOME);
});

// ---- what a kept set comes to as things stand

test('what is of a project that was forgotten is gone, and a channel that is listed no longer; nothing else is', () => {
  const known: Known = { projects: ['shop'], channels: ['chat'] };
  const gone: Tab[] = [page('depot'), board('depot'), { kind: 'table', project: 'depot', name: 'stock' }, { kind: 'file', project: 'depot', path: 'a.md' }, { kind: 'repo', project: 'depot' }, { kind: 'blueprint', name: 'std/board', project: 'depot' }, channel('plans')];
  const there: Tab[] = [HOME, page('shop'), board('shop'), channel('chat'), { kind: 'blueprint', name: 'std/board' }, { kind: 'instance', id: 'in_1' }, { kind: 'settings' }, { kind: 'extension', name: 'tools' }, { kind: 'activity' }];
  for (const tab of gone) assert.equal(isGone(tab, known), true, tabKey(tab));
  for (const tab of there) assert.equal(isGone(tab, known), false, tabKey(tab));
});

test('what was not read says nothing of what is gone', () => {
  const unread: Known = { projects: null, channels: null };
  for (const tab of [board('depot'), channel('plans')]) assert.equal(isGone(tab, unread), false);
  const set = split(HOME, board('depot'), channel('plans'));
  assert.equal(standingSet(set, unread).set, set);
});

test('a set with nothing gone in it is the same set', () => {
  const set = split(HOME, board('shop'), channel('chat'));
  const now = standingSet(set, everything);
  assert.equal(now.set, set);
  assert.deepEqual(now.lost, []);
});

test('the tabs of a project forgotten while its set was not showing are taken out as closing each would have, and said', () => {
  const set: TabSet = { tabs: [HOME, board('depot'), board('shop'), page('depot')], layout: group('g1', ['home', 'board:depot', 'board:shop', 'project:depot'], 'board:depot'), focused: 'g1' };
  const now = standingSet(set, { projects: ['shop'], channels: [] });
  assert.deepEqual(said(now.set), { tabs: ['home', 'board:shop'], layout: 'g1[home,board:shop]*board:shop', focused: 'g1' });
  assert.deepEqual(now.lost, ['board:depot', 'project:depot']);
});

test('a group whose every tab is gone goes, and the group last used is one that is left', () => {
  const now = standingSet(split(HOME, board('shop'), channel('plans')), { projects: ['shop'], channels: ['chat'] });
  assert.deepEqual(said(now.set), { tabs: ['home', 'board:shop'], layout: 'g1[home,board:shop]*board:shop', focused: 'g1' });
  assert.deepEqual(now.lost, ['channel:plans']);
});

test('a set all of whose tabs are gone is the start page alone', () => {
  const now = standingSet(row(board('depot'), channel('plans')), { projects: [], channels: [] });
  assert.deepEqual(said(now.set), said(start()));
  assert.deepEqual(now.lost, ['board:depot', 'channel:plans']);
});

test('a tab whose thing was taken away is taken out of every kept set, and sets without it are the same sets', () => {
  const sets: Sets = { front: split(HOME, board('shop'), channel('chat')), back: row(HOME, channel('chat')), yard: row(HOME, board('shop')) };
  const next = forgot(sets, 'channel:chat');
  assert.deepEqual(said(next.front!), { tabs: ['home', 'board:shop'], layout: 'g1[home,board:shop]*board:shop', focused: 'g1' });
  assert.deepEqual(said(next.back!), said(start()));
  assert.equal(next.yard, sets.yard);
  assert.equal(forgot(sets, 'channel:nowhere'), sets);
});

// ---- which set is shown as the panel comes to be of a workspace

const there = ['front', 'back'];

test('another workspace chosen keeps what was showing under the one it was of, and shows the start page alone where nothing was kept', () => {
  const showing = split(HOME, board('shop'), channel('chat'));
  const next = switched({}, showing, 'front', 'back', there);
  assert.equal(next.sets.front, showing);
  assert.deepEqual(said(next.show!), said(start()));
  assert.deepEqual(said(next.sets.back!), said(start()));
});

test('going back shows what was open: the tabs, their groups, the tab showing in each and the group last used', () => {
  const front = split(HOME, board('shop'), channel('chat'));
  const away = switched({}, front, 'front', 'back', there);
  const back = show(away.show!.layout, 'board:depot', 'g1');
  const home = switched(away.sets, { tabs: [HOME, board('depot')], layout: back, focused: 'g1' }, 'back', 'front', there);
  assert.equal(home.show, front);
  assert.deepEqual(said(home.sets.back!), { tabs: ['home', 'board:depot'], layout: 'g1[home,board:depot]*board:depot', focused: 'g1' });
  // And back again: what was opened there.
  const again = switched(home.sets, home.show!, 'front', 'back', there);
  assert.deepEqual(said(again.show!), { tabs: ['home', 'board:depot'], layout: 'g1[home,board:depot]*board:depot', focused: 'g1' });
  assert.equal(again.sets.front, front);
});

test('a set is of the workspace it was opened in, whatever its tabs are of', () => {
  // A board of a project of the other workspace, opened here from a link, is a tab here and not there.
  const showing = row(HOME, board('depot'));
  const next = switched({ back: row(HOME) }, showing, 'front', 'back', there);
  assert.deepEqual(next.show!.tabs.map(tabKey), ['home']);
  assert.deepEqual(next.sets.front!.tabs.map(tabKey), ['home', 'board:depot']);
});

test('the panel being of the workspace it was of puts nothing on the page, and keeps what is showing', () => {
  const showing = row(HOME, board('shop'));
  const next = switched({ front: row(HOME), back: row(HOME, board('depot')) }, showing, 'front', 'front', there);
  assert.equal(next.show, null);
  assert.equal(next.sets.front, showing);
  assert.deepEqual(Object.keys(next.sets).sort(), ['back', 'front']);
});

test('where the panel was of no workspace so far, what is showing stays and is that workspace\'s from then on', () => {
  const showing = row(HOME, board('shop'));
  const next = switched({}, showing, null, 'front', there);
  assert.equal(next.show, null);
  assert.equal(next.sets.front, showing);
  // And it is never the other's.
  assert.deepEqual(said(switched(next.sets, showing, 'front', 'back', there).show!), said(start()));
});

test('the set of a workspace that was taken away is forgotten: one that was kept, and the one that was showing', () => {
  const sets: Sets = { front: row(HOME, board('shop')), back: row(HOME, board('depot')), yard: row(HOME) };
  // `yard` was taken away while another was showing.
  assert.deepEqual(Object.keys(switched(sets, sets.front!, 'front', 'front', there).sets).sort(), ['back', 'front']);
  // `front` was taken away while it was showing: the panel is of `back`, whose set is shown, and nothing is kept of `front`.
  const next = switched(sets, row(HOME, board('shop'), channel('chat')), 'front', 'back', ['back']);
  assert.equal(next.show, sets.back);
  assert.deepEqual(Object.keys(next.sets), ['back']);
});

test('a workspace called something else keeps its set: it is kept by the id it always had', () => {
  const sets: Sets = { front: row(HOME, board('shop')), back: row(HOME) };
  // Nothing here knows a name: the ids there are say which sets stay.
  assert.equal(switched(sets, sets.back!, 'back', 'front', there).show, sets.front);
});

// ---- as the page is read

const space = (id: string, projects: string[], more: Partial<Workspace> = {}): Workspace => ({ id, name: id, created: 0, projects, channels: [], ...more });
const all = [space('back', ['depot'], { usual: true }), space('front', ['shop'])];
const projects = [{ id: 'shop' }, { id: 'depot' }];
/** The workspace the panel is first of where nothing was chosen (`beginAt` in `chosen.ts`): that of the tab that was showing in what a browser kept from before, else the usual one. */
const begins = (was: TabSet | null): string => {
  const tab = was ? showingIn(was) : undefined;
  const project = tab && 'project' in tab ? tab.project : undefined;
  const held = standing(ALL, all, spaceNear({ project }, all, projects));
  assert.equal(held.kind, 'space');
  return held.kind === 'space' ? held.id : '';
};

test('read again, the page shows the set kept for the workspace the panel is of, and the others are kept', () => {
  const sets: Sets = { front: split(HOME, board('shop'), channel('chat')), back: row(HOME, board('depot')) };
  const put = first(sets, null, 'front', there, everything);
  assert.equal(put.show, sets.front);
  assert.equal(put.sets.back, sets.back);
  assert.equal(first(sets, null, 'back', there, everything).show, sets.back);
});

test('a workspace with nothing kept shows the start page alone as the page is read', () => {
  assert.deepEqual(said(first({ front: row(HOME, board('shop')) }, null, 'back', there, everything).show), said(start()));
});

test('where nothing was ever chosen, what a browser kept from before is the set of the workspace of the tab that was showing in it, and of no other', () => {
  // Kept before each workspace had its own tabs: a board of `shop`, which is in `front`, was showing.
  const was = row(HOME, board('depot'), board('shop'));
  const to = begins(was);
  assert.equal(to, 'front');
  const put = first({}, was, to, there, everything);
  assert.equal(put.show, was);
  assert.equal(put.sets.front, was);
  // The other workspace, chosen afterwards, is not shown it: and going back finds it.
  const away = switched(put.sets, put.show, 'front', 'back', there);
  assert.deepEqual(said(away.show!), said(start()));
  assert.equal(switched(away.sets, away.show!, 'back', 'front', there).show, was);
});

test('where nothing was chosen and what was showing is of no workspace, what a browser kept from before is the usual workspace\'s', () => {
  const was: TabSet = { ...row(HOME, board('shop')), layout: group('g1', ['home', 'board:shop'], 'home') };
  assert.equal(begins(was), 'back');
  assert.equal(begins(null), 'back');
  assert.equal(first({}, was, 'back', there, everything).sets.back, was);
});

test('a workspace that has a set of its own is shown that, and not what a browser kept from before', () => {
  const sets: Sets = { front: row(HOME, board('shop')) };
  const put = first(sets, row(HOME, board('depot')), 'front', there, everything);
  assert.equal(put.show, sets.front);
  assert.deepEqual(Object.keys(put.sets), ['front']);
});

test('where the panel is of no workspace there is one set, as there always was: what was kept as that, else the start page', () => {
  const was = row(HOME, board('shop'));
  const sets: Sets = { front: row(HOME) };
  const put = first(sets, was, null, [], { projects: ['shop'], channels: [] });
  assert.equal(put.show, was);
  assert.equal(put.sets, sets);
  assert.deepEqual(said(first({}, null, null, [], everything).show), said(start()));
});

test('what is first shown is shown as things stand, and the sets of workspaces that are gone are forgotten', () => {
  const sets: Sets = { front: row(HOME, board('shop'), board('depot')), yard: row(HOME) };
  const put = first(sets, null, 'front', there, { projects: ['shop'], channels: [] });
  assert.deepEqual(said(put.show), { tabs: ['home', 'board:shop'], layout: 'g1[home,board:shop]*board:shop', focused: 'g1' });
  assert.deepEqual(Object.keys(put.sets), ['front']);
  assert.equal(put.sets.front, put.show);
  // What a browser kept from before, likewise.
  assert.deepEqual(first({}, row(HOME, board('depot')), 'back', there, { projects: ['shop'], channels: [] }).show.tabs.map(tabKey), ['home']);
});

test('every group of a set that is put back has a tab showing', () => {
  const put = first({ front: split(HOME, board('shop'), channel('chat')) }, null, 'front', there, everything);
  for (const entry of groups(put.show.layout)) assert.ok(entry.active && entry.tabs.includes(entry.active));
});

// ---- what was opened before the tabs were put back

const settings: Tab = { kind: 'settings' };

test('what was opened on the page before the tabs kept were put back is opened among them, and is the tab showing', () => {
  // Settings pressed on the rail as soon as the frame was there, beside the start page it began with.
  const meanwhile = row(HOME, settings);
  const kept = split(HOME, board('shop'), channel('chat'));
  assert.deepEqual(said(among(kept, meanwhile)), {
    tabs: ['home', 'board:shop', 'channel:chat', 'settings'],
    layout: { row: ['g1[home,board:shop]*board:shop', 'g2[channel:chat,settings]*settings'] },
    focused: 'g2',
  });
  // Put back with nothing kept, it is beside the start page, as it was opened.
  assert.deepEqual(said(among(start(), meanwhile)), said(meanwhile));
});

test('nothing opened before the tabs were put back is the set put back, as it was', () => {
  const kept = split(HOME, board('shop'), channel('chat'));
  assert.equal(among(kept, start()), kept);
  // The start page shown again, closed or pressed meanwhile, is nothing opened.
  assert.equal(among(kept, { ...start(), focused: 'g9' }), kept);
});

test('what was opened before and is open in the set put back already is shown where it is, in no second place', () => {
  const kept = split(HOME, settings, channel('chat'));
  const put = among(kept, row(HOME, settings));
  assert.deepEqual(said(put), { tabs: ['home', 'settings', 'channel:chat'], layout: { row: ['g1[home,settings]*settings', 'g2[channel:chat]*channel:chat'] }, focused: 'g1' });
});

test('several opened before are all opened, the one that was showing showing; and where the start page was showing again, the group shows what it was put back showing', () => {
  const kept = row(HOME, board('shop'));
  const meanwhile: TabSet = { ...row(HOME, settings, page('depot')), layout: group('g1', ['home', 'settings', 'project:depot'], 'settings') };
  assert.deepEqual(said(among(kept, meanwhile)), { tabs: ['home', 'board:shop', 'settings', 'project:depot'], layout: 'g1[home,board:shop,project:depot,settings]*settings', focused: 'g1' });
  const back: TabSet = { ...meanwhile, layout: group('g1', ['home', 'settings', 'project:depot'], 'home') };
  assert.deepEqual(said(among(kept, back)), { tabs: ['home', 'board:shop', 'settings', 'project:depot'], layout: 'g1[home,board:shop,settings,project:depot]*board:shop', focused: 'g1' });
});

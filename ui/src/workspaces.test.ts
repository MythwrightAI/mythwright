import assert from 'node:assert/strict';
import { test } from 'node:test';
import { ALL, belongsTo, choiceOf, gathered, heldOf, holdsChannel, holdsTopic, leftOut, listed, mostWaiting, removeClosed, sameHeld, spaceNear, spaceOf, spaceOfChannel, spaceOfTopic, standing, unseen, usualOf, waitingIn } from './workspaces.ts';
import type { Held, Workspace } from './workspaces.ts';

const space = (id: string, projects: string[] = [], channels: string[] = [], more: Partial<Workspace> = {}): Workspace => ({ id, name: id, created: 0, projects, channels, ...more });
const project = (id: string) => ({ id });
const channel = (id: string, of: string | null = null) => ({ id, project: of });
const ids = (things: { id: string }[]) => things.map((thing) => thing.id);

// Every project is in a workspace; `shed` is one the daemon has not been read to have placed, which the page lists under the usual one.
const projects = [project('shop'), project('depot'), project('shed')];
const all = [space('back', ['depot'], [], { usual: true }), space('front', ['shop'], ['plans'])];
const front: Held = { kind: 'space', id: 'front' };
const back: Held = { kind: 'space', id: 'back' };
// Every channel is a project's or a workspace's own. `chat` is one the daemon has not been read to have placed, and `old` says a project
// that is not listed: the page lists both under the usual workspace. Then a workspace's own; and a project's whose project is in that
// workspace, in another, and in the usual one unread.
const talk = [channel('chat'), channel('plans'), channel('shop-talk', 'shop'), channel('depot-talk', 'depot'), channel('yard', 'shed'), channel('old', 'gone')];
const full = (id: string, of: string | null) => ({ id, name: id, project: of, topic: '', created: 0, members: [] });
const shown = (found: ReturnType<typeof gathered<(typeof talk)[number], (typeof projects)[number]>>) => ids([...found.loose, ...found.projects.flatMap((one) => one.channels)]);

// ---- until the workspaces are read: everything, held to nothing

test('until the workspaces are read every project is listed, and the panel is of nothing', () => {
  assert.deepEqual(ids(listed([project('shop'), project('depot')], [])), ['shop', 'depot']);
  assert.deepEqual(standing(ALL, []), ALL);
  assert.deepEqual(standing(front, []), ALL);
});

test('until the workspaces are read every channel of no project is listed under no heading', () => {
  const found = gathered([channel('chat'), channel('general')], [], projects);
  assert.deepEqual(ids(found.loose), ['chat', 'general']);
  assert.deepEqual(found.projects, []);
});

test('until the workspaces are read a project\'s channels are under the project, after those of no project that is listed', () => {
  const found = gathered([channel('yard', 'shed'), channel('chat'), channel('old', 'gone')], [], projects);
  assert.deepEqual(ids(found.loose), ['chat', 'old']);
  assert.deepEqual(
    found.projects.map((one) => [one.project.id, ids(one.channels)]),
    [['shed', ['yard']]],
  );
});

test('which workspace a project is in, and the first of two that name it', () => {
  assert.equal(spaceOf('shop', all)?.id, 'front');
  assert.equal(spaceOf('shop', [space('a', ['shop']), space('b', ['shop'])])?.id, 'a');
});

test('a project no workspace names is listed under the usual one, and under nothing only where there is no workspace', () => {
  // A daemon from before answers a project in none, and one made a moment ago is not yet read to be placed: it is where the daemon puts it.
  assert.equal(spaceOf('shed', all)?.id, 'back');
  assert.equal(spaceOf('shed', [space('front', ['shop']), space('back', ['depot'], [], { usual: true })])?.id, 'back');
  assert.equal(spaceOf('shed', []), null);
  // Placed, it is where it was placed, whichever is usual.
  assert.equal(spaceOf('shed', [space('back', ['depot'], [], { usual: true }), space('front', ['shop', 'shed'])])?.id, 'front');
});

test('the usual workspace is the one the daemon says, and from a daemon that says none the one made first', () => {
  assert.equal(usualOf(all)?.id, 'back');
  assert.equal(usualOf([space('a', [], [], { created: 5 }), space('b', [], [], { created: 5, usual: true })])?.id, 'b');
  // A daemon of an earlier program says nothing of it: the earliest made, and of two made in one moment the first given.
  assert.equal(usualOf([space('a', [], [], { created: 9 }), space('b', [], [], { created: 4 }), space('c', [], [], { created: 4 })])?.id, 'b');
  assert.equal(usualOf([space('only')])?.id, 'only');
  assert.equal(usualOf([]), null);
});

test('what a channel belongs to, as it is asked: its project or a workspace, and never nothing', () => {
  assert.deepEqual(belongsTo(full('shop-talk', 'shop'), all, projects), { project: 'shop' });
  assert.deepEqual(belongsTo(full('plans', null), all, projects), { workspace: 'front' });
  // One whose place was not read, and one of a project that is gone, are the usual workspace's.
  assert.deepEqual(belongsTo(full('chat', null), all, projects), { workspace: 'back' });
  assert.deepEqual(belongsTo(full('old', 'gone'), all, projects), { workspace: 'back' });
  // Nothing is said only before the workspaces are read.
  assert.deepEqual(belongsTo(full('chat', null), [], projects), {});
});

test('a channel no workspace names is listed under the usual one, and under nothing only where there is no workspace', () => {
  // A daemon from before answers a channel that belongs to nothing, and one made a moment ago is not yet read to be placed.
  assert.equal(spaceOfChannel(channel('chat'), all, projects)?.id, 'back');
  // One that says a project that is gone belongs to nothing that is there: the usual workspace too.
  assert.equal(spaceOfChannel(channel('old', 'gone'), all, projects)?.id, 'back');
  // A project's is its project's workspace's, and its project's where the project's place was not read.
  assert.equal(spaceOfChannel(channel('shop-talk', 'shop'), all, projects)?.id, 'front');
  assert.equal(spaceOfChannel(channel('yard', 'shed'), all, projects)?.id, 'back');
  // Placed, it is where it was placed, whichever is usual.
  assert.equal(spaceOfChannel(channel('plans'), all, projects)?.id, 'front');
  assert.equal(spaceOfChannel(channel('chat'), [], projects), null);
});

// ---- the panel is of one workspace

test('held to a workspace, its projects are listed and no others, each as a project always was', () => {
  assert.deepEqual(ids(listed(projects, all, front)), ['shop']);
  // With the usual one, a project whose place was not read.
  assert.deepEqual(ids(listed(projects, all, back)), ['depot', 'shed']);
  // One with nothing in it lists nothing, which is said where it is drawn; and a project it names that is gone is passed over.
  assert.deepEqual(listed(projects, [...all, space('attic', ['gone'])], { kind: 'space', id: 'attic' }), []);
  // A project two workspaces name is the first one's only.
  const twice = [space('a', ['shop']), space('b', ['shop'])];
  assert.deepEqual([ids(listed([project('shop')], twice, { kind: 'space', id: 'a' })), ids(listed([project('shop')], twice, { kind: 'space', id: 'b' }))], [['shop'], []]);
});

test('every project is listed under one workspace and no other, one that no workspace names among them', () => {
  for (const one of projects) {
    const lists: Held[] = [front, back].filter((held) => ids(listed(projects, all, held)).includes(one.id));
    assert.equal(lists.length, 1, one.id);
  }
  // And with a daemon that says no usual one, and a workspace with nothing in it.
  const before = [space('attic'), space('back', ['depot'], [], { created: 1 }), space('front', ['shop'], [], { created: 2 })];
  const every: Held[] = before.map((one) => ({ kind: 'space', id: one.id }));
  for (const one of projects) assert.equal(every.filter((held) => ids(listed(projects, before, held)).includes(one.id)).length, 1, one.id);
  assert.deepEqual(ids(listed(projects, before, { kind: 'space', id: 'attic' })), ['shed']);
});

test('held to a workspace, its own channels are under no heading, its projects are the headings, and nothing else is listed', () => {
  const found = gathered(talk, all, projects, front);
  assert.deepEqual(ids(found.loose), ['plans']);
  assert.deepEqual(
    found.projects.map((one) => [one.project.id, ids(one.channels)]),
    [['shop', ['shop-talk']]],
  );
  // No channel is the factory's own, listed whatever is chosen: what is of another workspace is left out.
  assert.deepEqual(ids(leftOut(talk, front, all, projects)), ['chat', 'depot-talk', 'yard', 'old']);
  // With the usual workspace: a channel whose place was not read, one of a project that is gone, and those of a project whose place was not read.
  const usual = gathered(talk, all, projects, back);
  assert.deepEqual(ids(usual.loose), ['chat', 'old']);
  assert.deepEqual(
    usual.projects.map((one) => [one.project.id, ids(one.channels)]),
    [
      ['depot', ['depot-talk']],
      ['shed', ['yard']],
    ],
  );
  assert.deepEqual(ids(leftOut(talk, back, all, projects)), ['plans', 'shop-talk']);
});

test('a project of the workspace with no channel is no heading', () => {
  assert.deepEqual(gathered([channel('plans')], all, projects, front), { loose: [channel('plans')], projects: [], archived: [] });
});

test('which workspace a channel is of: its project\'s, the one it is the own of, or the usual one, and never none', () => {
  const [chat, plans, shopTalk, depotTalk, yard, old] = talk.map((one) => choiceOf(one, all, projects));
  assert.deepEqual([chat, plans, shopTalk, depotTalk, yard, old], [back, front, front, back, back, back]);
  // Until the workspaces are read a channel is of none that can be chosen.
  assert.equal(choiceOf(channel('shop-talk', 'shop'), [], projects), null);
  assert.equal(choiceOf(channel('chat'), [], projects), null);
});

test('a channel is listed only where the workspace it is of is the one chosen', () => {
  const lists = (held: Held) => talk.map((one) => holdsChannel(held, one, all, projects));
  assert.deepEqual(lists(ALL), [true, true, true, true, true, true]);
  assert.deepEqual(lists(front), [false, true, true, false, false, false]);
  assert.deepEqual(lists(back), [true, false, false, true, true, true]);
});

test('every channel is listed or left out, and never both; and each is listed under one workspace and no other', () => {
  for (const held of [ALL, front, back]) {
    const here = shown(gathered(talk, all, projects, held));
    const away = ids(leftOut(talk, held, all, projects));
    assert.deepEqual([...here, ...away].sort(), ids(talk).sort());
  }
  for (const one of talk) {
    assert.equal([front, back].filter((held) => holdsChannel(held, one, all, projects)).length, 1, one.id);
  }
});

// ---- what the panel is of as things stand

test('what was chosen stands while it is there: a workspace by its id, whatever it is called', () => {
  assert.deepEqual(standing(front, all), front);
  assert.deepEqual(standing(front, [space('back', ['depot']), { ...space('front', ['shop']), name: 'Shop front' }]), front);
});

test('once they are read the panel is of a workspace: nothing chosen, or one that is gone, is the usual one', () => {
  assert.deepEqual(standing(ALL, all), back);
  assert.deepEqual(standing({ kind: 'space', id: 'gone' }, all), back);
  // The usual one whatever each is called: here it is the last by name.
  const named = [space('attic'), space('front', ['shop']), space('zed', [], [], { usual: true })];
  assert.deepEqual(standing(ALL, named), { kind: 'space', id: 'zed' });
  // And never everything: there is no such choice where there is a workspace.
  for (const held of [ALL, front, { kind: 'space', id: 'gone' } as Held]) assert.notEqual(standing(held, all).kind, 'all');
});

test('where nothing was chosen the panel begins at the workspace of what is showing beside it', () => {
  assert.deepEqual(standing(ALL, all, 'front'), front);
  assert.deepEqual(standing({ kind: 'space', id: 'gone' }, all, 'front'), front);
  // What is showing is of no workspace, or of one that is not there: the usual one.
  assert.deepEqual(standing(ALL, all, null), back);
  assert.deepEqual(standing(ALL, all, 'gone'), back);
  // What was chosen is not moved by what is showing.
  assert.deepEqual(standing(back, all, 'front'), back);
});

test('the workspace of what is showing: a project\'s, or a channel\'s, which is its project\'s or the one it is the own of', () => {
  assert.equal(spaceNear({ project: 'shop' }, all, projects), 'front');
  assert.equal(spaceNear({ channel: channel('shop-talk', 'shop') }, all, projects), 'front');
  assert.equal(spaceNear({ channel: channel('plans') }, all, projects), 'front');
  // A project whose place was not read is of the usual one, and so is a channel's.
  assert.equal(spaceNear({ project: 'shed' }, all, projects), 'back');
  assert.equal(spaceNear({ channel: channel('chat') }, all, projects), 'back');
  // What is of neither is the factory's own, and of no workspace; a channel that is not read yet is of none either.
  assert.equal(spaceNear({}, all, projects), null);
  assert.equal(spaceNear({ channel: null }, all, projects), null);
  assert.equal(spaceNear({ project: 'shop' }, [], projects), null);
});

test('a browser that kept the choice of what is in no workspace comes to the workspace of what is showing, else the usual one', () => {
  // What a browser kept when a project could be in none is nothing chosen now.
  const kept = heldOf('{"kind":"loose"}');
  assert.deepEqual(kept, ALL);
  assert.deepEqual(standing(kept, all, 'front'), front);
  assert.deepEqual(standing(kept, all), back);
  // Until the workspaces are read it is of nothing, and is not put right from an empty list.
  assert.deepEqual(standing(kept, []), ALL);
});

// ---- what waits unread out of sight

test('what waits unread in channels the panel leaves out is counted, and nothing is where everything is listed', () => {
  const unread = { chat: 1, plans: 2, 'shop-talk': 4, 'depot-talk': 8, yard: 16, old: 32 };
  assert.equal(unseen(talk, unread, ALL, all, projects), 0);
  assert.equal(unseen(talk, unread, front, all, projects), 57);
  assert.equal(unseen(talk, unread, back, all, projects), 6);
  // No channel is listed whatever is chosen: what waits in one of another workspace waits out of sight.
  assert.equal(unseen(talk, { chat: 3, plans: 1 }, front, all, projects), 3);
  assert.equal(unseen(talk, { plans: 1, 'shop-talk': 2 }, front, all, projects), 0);
  assert.equal(unseen(talk, {}, front, all, projects), 0);
});

test('what waits in each workspace is its channels\', and every channel is some workspace\'s', () => {
  const unread = { chat: 1, plans: 2, 'shop-talk': 4, 'depot-talk': 8, yard: 16, old: 32 };
  assert.equal(waitingIn(talk, unread, front, all, projects), 6);
  assert.equal(waitingIn(talk, unread, back, all, projects), 57);
  // What is out of sight is what waits in the other workspaces.
  assert.equal(unseen(talk, unread, front, all, projects), waitingIn(talk, unread, back, all, projects));
  assert.equal(unseen(talk, unread, back, all, projects), waitingIn(talk, unread, front, all, projects));
  assert.equal(waitingIn(talk, {}, front, all, projects), 0);
});

test('where one press at the foot goes: the workspace where the most wait out of sight', () => {
  const three = [space('attic', [], ['notes']), ...all];
  const every = [...talk, channel('notes')];
  // Looking at the front: more wait in the back than in the attic.
  assert.equal(mostWaiting(every, { notes: 2, chat: 1, 'depot-talk': 4 }, front, three, projects), 'back');
  assert.equal(mostWaiting(every, { notes: 9, chat: 1 }, front, three, projects), 'attic');
  // As many in each: the first as they are listed.
  assert.equal(mostWaiting(every, { notes: 1, chat: 1 }, front, three, projects), 'attic');
  // What waits in the workspace showing is in sight, and nothing waits out of it.
  assert.equal(mostWaiting(every, { plans: 3, 'shop-talk': 1 }, front, three, projects), null);
  assert.equal(mostWaiting(every, {}, front, three, projects), null);
  assert.equal(mostWaiting(every, { chat: 1 }, ALL, [], projects), null);
});

test('what a browser kept of the choice is read, and anything else is nothing chosen', () => {
  assert.deepEqual(heldOf(JSON.stringify(front)), front);
  for (const kept of [null, undefined, '', 'front', '{', 'null', '[]', '{"kind":"space"}', '{"kind":"space","id":7}', '{"kind":"space","id":""}', '{"kind":"other"}', '{"kind":"all"}', '{"kind":"loose"}']) {
    assert.deepEqual(heldOf(kept), ALL);
  }
  assert.ok(sameHeld(front, { kind: 'space', id: 'front' }));
  assert.ok(!sameHeld(front, back));
  assert.ok(!sameHeld(front, ALL) && !sameHeld(ALL, back) && sameHeld(ALL, { kind: 'all' }));
});

// ---- taking a workspace away: its name typed first

test('a workspace is removed only once its name is typed exactly, and the button says why it is closed until then', () => {
  assert.equal(removeClosed('Shop front', 'Shop front'), null);
  assert.equal(removeClosed('', 'Shop front'), 'Type Shop front to remove it.');
  for (const near of ['Shop', 'shop front', 'Shop front ', ' Shop front', 'Shop  front', 'Shop fronts']) {
    assert.equal(removeClosed(near, 'Shop front'), 'What is typed is not Shop front: type its name as it is written.', `"${near}"`);
  }
  // A name that is itself spaces or letters of another kind is typed as it is.
  assert.equal(removeClosed('Café ', 'Café '), null);
  assert.notEqual(removeClosed('Cafe ', 'Café '), null);
});

// ---- topics: a workspace's or a project's, as a channel is

test("a topic is listed in its project's workspace, in the one it is the own of, and else in the usual one", () => {
  const topic = (workspace: string | null, project: string | null) => ({ workspace, project });
  assert.equal(spaceOfTopic(topic(null, 'shop'), all, projects)?.id, 'front');
  assert.equal(spaceOfTopic(topic('front', null), all, projects)?.id, 'front');
  // A project that is gone, a workspace that is gone, and nothing said: the usual one, where the daemon settles it.
  assert.equal(spaceOfTopic(topic(null, 'gone'), all, projects)?.id, 'back');
  assert.equal(spaceOfTopic(topic('gone', null), all, projects)?.id, 'back');
  assert.equal(spaceOfTopic(topic(null, null), all, projects)?.id, 'back');
  assert.equal(spaceOfTopic(topic('front', null), [], projects), null);
});

test('the panel lists the topics of the workspace it is of, and every topic before the workspaces are read', () => {
  const topic = (workspace: string | null, project: string | null) => ({ workspace, project });
  assert.equal(holdsTopic(front, topic(null, 'shop'), all, projects), true);
  assert.equal(holdsTopic(back, topic(null, 'shop'), all, projects), false);
  assert.equal(holdsTopic(back, topic('back', null), all, projects), true);
  assert.equal(holdsTopic(ALL, topic('front', null), [], projects), true);
});

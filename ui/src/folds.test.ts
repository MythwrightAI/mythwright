import assert from 'node:assert/strict';
import { test } from 'node:test';
import { CHANNELS, channelsUnfolded, FACTORY_FOLDS, foldedAll, groupFoldsGone, ofChannels, ofTopicsPanel, openedByName, ofResources, projectFold, resourcesFolds, SECTIONS, startsUnfolded, STARTS_UNFOLDED, TOPICS, topicFoldsGone, topicsUnfolded, turnedNow, unfoldedAll } from './folds.ts';

test('the fold a project\'s things were gathered under is not kept, open or shut, and the rest is as it was', () => {
  const kept = ['project:shop:app', 'project:shop:repo', 'project:shop:blueprints', 'channel:ch_1', 'blueprints:std'];
  assert.deepEqual(turnedNow(kept), ['project:shop:repo', 'project:shop:blueprints', 'channel:ch_1', 'blueprints:std']);
  // One that never opened it kept nothing of it, and is read the same.
  assert.deepEqual(turnedNow(['project:shop:repo']), ['project:shop:repo']);
});

test('a folder of a project\'s files that is called as that fold was is still a folder', () => {
  assert.deepEqual(turnedNow(['project:shop:folder:app', 'project:shop:folder:src/app', 'project:shop:folder:a:app']), [
    'project:shop:folder:app',
    'project:shop:folder:src/app',
    'project:shop:folder:a:app',
  ]);
});

test('what was kept that is no list of names is nothing turned', () => {
  assert.deepEqual(turnedNow(null), []);
  assert.deepEqual(turnedNow({ 'project:shop:app': true }), []);
  assert.deepEqual(turnedNow(['project:shop:repo', 7, null]), ['project:shop:repo']);
});

test('a project folded when projects started unfolded is folded as one starts now, and kept as nothing; one left unfolded starts folded', () => {
  assert.deepEqual(turnedNow(['project:shop', 'project:depot:repo']), ['project:depot:repo']);
  // What says a project is unfolded now is another name, which is kept.
  assert.deepEqual(turnedNow([projectFold('shop')]), ['project:shop:tree']);
});

test('the Resources panel\'s folds are told from the other panels\'', () => {
  for (const one of [SECTIONS.factory, projectFold('shop'), 'project:shop:repo', 'project:shop:folder:docs', 'project:shop:blueprints', 'blueprints:std', 'parts:std/fleet']) assert.ok(ofResources(one), one);
  for (const one of ['channel:ch_1', 'group:mine', 'archived', 'inbox:archived']) assert.ok(!ofResources(one), one);
});

test('collapse all forgets every fold of the panel, keeps the others and keeps the parts that start unfolded as folded; expand all unfolds every fold that was read, each once', () => {
  const kept = [SECTIONS.recents, projectFold('shop'), 'parts:std/fleet', 'channel:ch_1'];
  assert.deepEqual(foldedAll(kept), ['channel:ch_1', ...STARTS_UNFOLDED]);
  assert.deepEqual(foldedAll([]), [SECTIONS.projects, SECTIONS.factory, SECTIONS.blueprints]);
  assert.deepEqual(foldedAll([SECTIONS.factory, 'channel:ch_1']), ['channel:ch_1', ...STARTS_UNFOLDED], 'each once');
  // A part that starts unfolded is unfolded by being kept no longer; anything else by being kept.
  assert.deepEqual(unfoldedAll([SECTIONS.factory, projectFold('shop')], [SECTIONS.factory, SECTIONS.pinned]), [projectFold('shop'), SECTIONS.pinned]);
  assert.deepEqual(unfoldedAll(kept, [SECTIONS.recents, SECTIONS.projects]), kept);
  // The other panels' folds are no business of the Resources panel's buttons.
  assert.deepEqual(foldedAll([SECTIONS.factory, CHANNELS.archived], ofChannels), [SECTIONS.factory]);
});

test('projects, the factory and the blueprints start unfolded, and what is kept of each is that it was folded; what was pinned and lately opened start folded', () => {
  assert.deepEqual(STARTS_UNFOLDED, ['section:projects:folded', 'section:factory:folded', 'section:blueprints:folded']);
  for (const one of STARTS_UNFOLDED) assert.ok(startsUnfolded(one), one);
  for (const one of [SECTIONS.pinned, SECTIONS.recents, projectFold('shop'), 'blueprints:std', 'section:projects']) assert.ok(!startsUnfolded(one), one);
});

test('a browser that kept those parts unfolded when every part started folded keeps them unfolded, by keeping nothing of them', () => {
  assert.deepEqual(turnedNow(['section:projects', 'section:factory', 'section:blueprints', 'section:pinned', projectFold('shop')]), ['section:pinned', projectFold('shop')]);
  // One kept folded now is kept as it is.
  assert.deepEqual(turnedNow([SECTIONS.factory]), [SECTIONS.factory]);
});

test('every fold the panel has, of what was read: its parts, each project, its blueprints, and its files and folders only where its folder was looked at', () => {
  const folds = resourcesFolds(
    [
      { id: 'shop', blueprints: true, files: [{ path: 'docs', dir: true }, { path: 'docs/a.md', dir: false }, { path: 'docs/old', dir: true }] },
      { id: 'depot', blueprints: false, files: null },
    ],
    ['blueprints:std', 'parts:std/fleet'],
  );
  assert.deepEqual(folds, [
    ...Object.values(SECTIONS),
    FACTORY_FOLDS.members,
    'project:shop:tree',
    'project:shop:blueprints',
    'project:shop:repo',
    'project:shop:folder:docs',
    'project:shop:folder:docs/old',
    'project:depot:tree',
    'blueprints:std',
    'parts:std/fleet',
  ]);
  // Nothing read but the panel's own parts, and the Members line under Factory.
  assert.deepEqual(resourcesFolds([], []), [...Object.values(SECTIONS), FACTORY_FOLDS.members]);
});

test("the Resources panel's parts are in their order: what is kept to hand, the projects, the factory's, the blueprints", () => {
  assert.deepEqual(Object.values(SECTIONS), ['section:pinned', 'section:recents', 'section:projects:folded', 'section:factory:folded', 'section:blueprints:folded']);
});

test('a branch of the channels folded when branches started unfolded is folded as one starts now, and kept as nothing', () => {
  // A group, Pinned and a project's channels said they were folded; that is how each starts now, under another name.
  assert.deepEqual(turnedNow(['group:g1', 'group:pinned', 'project-channels:shop', 'channels:archived', 'channel:ch_1']), ['channels:archived', 'channel:ch_1']);
  // What says one is unfolded now is kept.
  assert.deepEqual(turnedNow([CHANNELS.group('g1'), CHANNELS.pinned, CHANNELS.project('shop')]), ['channels:group:g1', 'channels:pinned', 'channels:project:shop']);
  // A project of the Resources panel that is called so is still a project's fold there.
  assert.deepEqual(turnedNow(['project:project-channels:tree', 'project:group:repo']), ['project:project-channels:tree', 'project:group:repo']);
});

test('the Channels panel\'s folds are told from the other panels\', and neither panel takes the other\'s for its own', () => {
  const channels = [CHANNELS.pinned, CHANNELS.archived, CHANNELS.group('g1'), CHANNELS.project('shop'), CHANNELS.channel('ch_1'), CHANNELS.members('ch_1', 'agent')];
  for (const one of channels) assert.ok(ofChannels(one) && !ofResources(one), one);
  for (const one of [SECTIONS.projects, projectFold('shop'), 'project:shop:repo', 'blueprints:std', 'parts:std/fleet', 'inbox:archived', 'channelsx:1']) assert.ok(!ofChannels(one), one);
});

test('collapse all in the channels forgets every fold of that panel, who is in each channel among them, and keeps the Resources panel\'s', () => {
  const kept = [CHANNELS.group('g1'), CHANNELS.archived, CHANNELS.channel('ch_1'), CHANNELS.members('ch_2', 'user'), SECTIONS.projects, projectFold('shop'), 'inbox:archived'];
  assert.deepEqual(foldedAll(kept, ofChannels), [SECTIONS.projects, projectFold('shop'), 'inbox:archived']);
  // And the Resources panel's Collapse all leaves the channels' as they were.
  assert.deepEqual(foldedAll(kept), [CHANNELS.group('g1'), CHANNELS.archived, CHANNELS.channel('ch_1'), CHANNELS.members('ch_2', 'user'), 'inbox:archived', ...STARTS_UNFOLDED]);
});

test('expand all in the channels unfolds every branch it draws and every channel it lists, with who is in each, each once', () => {
  const kept = [CHANNELS.group('g1'), CHANNELS.channel('ch_1'), CHANNELS.members('ch_2', 'user'), CHANNELS.members('ch_9', 'agent'), SECTIONS.factory];
  assert.deepEqual(channelsUnfolded(kept, { pinned: true, groups: ['g1', 'g2'], projects: ['shop'], archived: true, channels: ['ch_1', 'ch_2'] }), [
    CHANNELS.group('g1'),
    CHANNELS.channel('ch_1'),
    // Who is in a channel listed starts unfolded, so it is unfolded by no longer being kept as folded;
    // a channel the panel does not list keeps its own as it was.
    CHANNELS.members('ch_9', 'agent'),
    SECTIONS.factory,
    CHANNELS.pinned,
    CHANNELS.group('g2'),
    CHANNELS.project('shop'),
    CHANNELS.archived,
    CHANNELS.channel('ch_2'),
  ]);
  // Nothing pinned and nothing archived: neither is a branch, and neither is kept as unfolded.
  assert.deepEqual(channelsUnfolded([], { pinned: false, groups: [], projects: ['depot'], archived: false, channels: [] }), [CHANNELS.project('depot')]);
  assert.deepEqual(channelsUnfolded([], { pinned: false, groups: [], projects: [], archived: false, channels: ['ch_3'] }), [CHANNELS.channel('ch_3')]);
  assert.deepEqual(channelsUnfolded([], { pinned: false, groups: [], projects: [], archived: false, channels: [] }), []);
});

test('collapse all after expand all in the channels folds every branch and every channel, and a channel unfolded by hand then shows who is in it', () => {
  const drawn = { pinned: true, groups: ['g1'], projects: ['shop'], archived: true, channels: ['ch_1', 'ch_2'] };
  const expanded = channelsUnfolded([CHANNELS.members('ch_1', 'bot'), SECTIONS.factory], drawn);
  const collapsed = foldedAll(expanded, ofChannels);
  // A channel starts folded, so nothing kept of it is folded; who is in it starts unfolded, so nothing kept of that is unfolded.
  for (const channel of drawn.channels) {
    assert.ok(!collapsed.includes(CHANNELS.channel(channel)), channel);
    for (const kind of ['user', 'agent', 'bot']) assert.ok(!collapsed.includes(CHANNELS.members(channel, kind)), `${channel} ${kind}`);
  }
  assert.deepEqual(collapsed, [SECTIONS.factory]);
});

test('the fold of a group that is not there goes, so that the next group given its id starts folded; every other fold stays', () => {
  const kept = [CHANNELS.group('g1'), CHANNELS.group('g3'), CHANNELS.pinned, CHANNELS.project('g3'), 'group:g3', SECTIONS.factory];
  assert.deepEqual(groupFoldsGone(kept, ['g1', 'g2']), [CHANNELS.group('g3')]);
  assert.deepEqual(groupFoldsGone(kept, ['g1', 'g3']), []);
  assert.deepEqual(groupFoldsGone(kept, []), [CHANNELS.group('g1'), CHANNELS.group('g3')]);
});

test('the Members line is a fold of the Factory part of the Resources panel: folded as it starts, unfolded by Expand all, folded by Collapse all, and what the Channels panel kept of it dropped', () => {
  assert.ok(ofResources(FACTORY_FOLDS.members));
  assert.ok(!ofChannels(FACTORY_FOLDS.members));
  assert.ok(!startsUnfolded(FACTORY_FOLDS.members));
  assert.ok(resourcesFolds([], []).includes(FACTORY_FOLDS.members));
  assert.deepEqual(foldedAll([FACTORY_FOLDS.members, 'channel:ch_1']), ['channel:ch_1', ...STARTS_UNFOLDED]);
  // A channel's own folds are never taken for it, whatever the channel is called, and the Channels panel's Expand all leaves it.
  assert.notEqual(FACTORY_FOLDS.members, CHANNELS.channel('members'));
  assert.deepEqual(channelsUnfolded([FACTORY_FOLDS.members], { pinned: false, groups: [], projects: [], archived: false, channels: [] }), [FACTORY_FOLDS.members]);
  // Where the Channels panel had it, it was kept as unfolded: that name says nothing now, and is dropped as it is read.
  assert.deepEqual(turnedNow(['channels:members', 'channel:ch_1']), ['channel:ch_1']);
});

test('a project pressed by its name is unfolded where it was folded, and nothing else of what is kept changes', () => {
  const kept = new Set([SECTIONS.recents, projectFold('depot')]);
  assert.deepEqual(openedByName(kept, projectFold('shop'), false), new Set([SECTIONS.recents, projectFold('depot'), projectFold('shop')]));
  // What was handed is not changed: the page keeps it by what it is handed back.
  assert.deepEqual(kept, new Set([SECTIONS.recents, projectFold('depot')]));
});

test('a project pressed by its name that is unfolded already stays so, with nothing kept anew', () => {
  assert.equal(openedByName(new Set([projectFold('shop')]), projectFold('shop'), false), null);
});

test('a project held open by what was typed changes nothing that is kept, folded or not', () => {
  assert.equal(openedByName(new Set(), projectFold('shop'), true), null);
  assert.equal(openedByName(new Set([projectFold('shop')]), projectFold('shop'), true), null);
});

test("the Topics panel's one fold, its closed topics, starts folded, is told from the Channels panel's, and keeps the name it had under the Messaging panel's Topics node", () => {
  assert.equal(TOPICS.closed, 'channels:topics:closed');
  assert.ok(ofTopicsPanel(TOPICS.closed) && !ofChannels(TOPICS.closed) && !ofResources(TOPICS.closed));
  for (const one of [CHANNELS.pinned, CHANNELS.archived, CHANNELS.group('g1'), CHANNELS.channel('topics'), CHANNELS.members('ch_1', 'user')]) assert.ok(!ofTopicsPanel(one), one);
  assert.ok(!startsUnfolded(TOPICS.closed));
  // Its Collapse all forgets it alone, and the Channels panel's leaves it as it was.
  const kept = [TOPICS.closed, CHANNELS.archived, SECTIONS.factory];
  assert.deepEqual(foldedAll(kept, ofTopicsPanel), [CHANNELS.archived, SECTIONS.factory]);
  assert.deepEqual(foldedAll(kept, ofChannels), [TOPICS.closed, SECTIONS.factory]);
  // Its Expand all unfolds it where any topic showing is closed, and touches nothing else.
  assert.deepEqual(topicsUnfolded([CHANNELS.archived], true), [CHANNELS.archived, TOPICS.closed]);
  assert.deepEqual(topicsUnfolded([CHANNELS.archived], false), [CHANNELS.archived]);
  assert.deepEqual(topicsUnfolded([TOPICS.closed], true), [TOPICS.closed], 'each once');
  // A page of a wiki that holds pages is a fold of the panel's, unfolded by Expand all and folded by Collapse all.
  assert.equal(TOPICS.page('@-topics-4'), 'channels:topics:page:@-topics-4');
  assert.ok(ofTopicsPanel(TOPICS.page('@-topics-4')) && !ofChannels(TOPICS.page('@-topics-4')));
  assert.deepEqual(topicsUnfolded([CHANNELS.archived], false, ['@-topics-4', '@-topics-9']), [CHANNELS.archived, TOPICS.page('@-topics-4'), TOPICS.page('@-topics-9')]);
  assert.deepEqual(foldedAll([TOPICS.page('@-topics-4'), CHANNELS.archived], ofTopicsPanel), [CHANNELS.archived]);
  // The fold of a page that is not there is dropped once the topics are read.
  assert.deepEqual(topicFoldsGone([TOPICS.page('@-topics-4'), TOPICS.page('@-topics-5'), TOPICS.closed], ['@-topics-4']), [TOPICS.page('@-topics-5')]);
});

test('what a browser kept of the Messaging panel\'s two nodes is dropped as it is read, since neither folds anything now, and its closed topics are kept', () => {
  assert.deepEqual(turnedNow(['channels:node:folded', 'channels:topics', TOPICS.closed, CHANNELS.archived]), [TOPICS.closed, CHANNELS.archived]);
});

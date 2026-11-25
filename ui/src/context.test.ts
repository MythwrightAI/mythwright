import assert from 'node:assert/strict';
import { test } from 'node:test';
import { addable, boardsShown, DEFAULT_TABS, DEFAULT_WIDTH, firstOf, fitWidth, frontOf, keptOf, LEAST_WIDTH, mediaKind, shown, tabsOf, threadsOf, widthOf, writtenOf } from './context.ts';

test('the panel has its usual tabs until the reader chooses their own', () => {
  assert.deepEqual(tabsOf(null), DEFAULT_TABS);
  assert.deepEqual(tabsOf('not json'), DEFAULT_TABS);
});

test('the tabs the reader left are the tabs it comes back with, even none', () => {
  assert.deepEqual(tabsOf(keptOf(['board', 'media'])), ['board', 'media']);
  assert.deepEqual(tabsOf(keptOf([])), []);
});

test('the usual tabs begin with Docs beside the others', () => {
  assert.deepEqual(DEFAULT_TABS, ['media', 'channel', 'board', 'docs']);
});

test('a reader who kept tabs before Docs was usual is given it once, at the end, and not again once it is taken away', () => {
  // Kept as a list, before what was given was kept with it.
  assert.deepEqual(tabsOf('["board","media"]'), ['board', 'media', 'docs']);
  assert.deepEqual(tabsOf('[]'), ['docs']);
  // Already there, it is where the reader put it.
  assert.deepEqual(tabsOf('["docs","board"]'), ['docs', 'board']);
  // Given, then taken away, and kept so: it stays away.
  assert.deepEqual(tabsOf(keptOf(['board', 'media'])), ['board', 'media']);
  // Kept with what was given, though by a page that gave nothing yet.
  assert.deepEqual(tabsOf(JSON.stringify({ tabs: ['board'], offered: [] })), ['board', 'docs']);
  // What makes no sense is the usual tabs.
  assert.deepEqual(tabsOf(JSON.stringify({ tabs: 'board', offered: ['docs'] })), DEFAULT_TABS);
});

test('a tab that is no longer something a tab can be is left out, and none is there twice', () => {
  assert.deepEqual(tabsOf(keptOf(['board', 'weather', 'board'])), ['board']);
  // The fleet's drawing was a tab until it was taken out of the application.
  assert.deepEqual(tabsOf(keptOf(['media', 'fleet', 'board'])), ['media', 'board']);
  assert.deepEqual(tabsOf('["board","weather","board"]'), ['board', 'docs']);
});

test('what can be added is what is not a tab already', () => {
  assert.deepEqual(addable(['media', 'board']).map((panel) => panel.id), ['channel', 'docs']);
});

test('a browser that kept the tab of the channel as Channel members has it as Channel, where it was, once', () => {
  assert.deepEqual(tabsOf(keptOf(['media', 'members', 'board', 'docs'])), ['media', 'channel', 'board', 'docs']);
  assert.deepEqual(tabsOf(keptOf(['members', 'channel'])), ['channel']);
  assert.deepEqual(tabsOf('["agents","members"]'), ['channel', 'docs']);
});

test('the tab shown is the one asked for while it is there, and the first otherwise', () => {
  assert.equal(shown(['media', 'board'], 'board'), 'board');
  assert.equal(shown(['media', 'board'], 'docs'), 'media');
  assert.equal(shown([], 'docs'), null);
});

test('an attachment is a picture, a page or a file', () => {
  assert.equal(mediaKind('image/png'), 'picture');
  assert.equal(mediaKind('text/html'), 'page');
  assert.equal(mediaKind('application/pdf'), 'file');
  assert.equal(mediaKind('image/svg+xml'), 'file', 'not drawn in the client, so not shown as one');
});

test('the panel is as wide as it was dragged to, within what can be read and what leaves the channel room', () => {
  assert.equal(fitWidth(500, 1200), 500);
  assert.equal(fitWidth(100, 1200), LEAST_WIDTH, 'no narrower than it can be read at');
  assert.equal(fitWidth(1100, 1200), 840, 'the channel keeps its share');
  assert.equal(fitWidth(500, 400), LEAST_WIDTH, 'in a window with no room it is as narrow as it goes');
});

test('the width that was kept is the width it comes back with', () => {
  assert.equal(widthOf('480'), 480);
  assert.equal(widthOf(null), DEFAULT_WIDTH);
  assert.equal(widthOf('wide'), DEFAULT_WIDTH);
  assert.equal(widthOf('12'), DEFAULT_WIDTH);
});

test('a thread that was open, and a message being written at length, come back in front when the page is read again', () => {
  assert.equal(frontOf('thread'), 'thread');
  assert.equal(frontOf('write'), 'write');
  assert.equal(frontOf('agent'), null, 'what was kept before somebody was read about under Channel is nothing');
  assert.equal(frontOf(null), null);
});

test('the thread that was open beside each channel is the one that comes back, and what was not one is nothing', () => {
  assert.deepEqual(threadsOf('{"c1":"m7","c2":"m9"}'), { c1: 'm7', c2: 'm9' });
  assert.deepEqual(threadsOf('{"c1":7,"c2":"","c3":"m1"}'), { c3: 'm1' });
  assert.deepEqual(threadsOf('["m7"]'), {});
  assert.deepEqual(threadsOf('not kept'), {});
  assert.deepEqual(threadsOf(null), {});
});

test('what was being written comes back as it was left, and what was not that is nothing', () => {
  const kept = JSON.stringify([['c1', { text: 'half a thought', attached: [{ id: 'a1' }] }]]);
  assert.deepEqual(writtenOf(kept), [['c1', { text: 'half a thought', attached: [{ id: 'a1' }] }]]);
  assert.deepEqual(writtenOf('[["c1","text"],["c2",{"text":1,"attached":[]}],7]'), []);
  assert.deepEqual(writtenOf('{}'), []);
  assert.deepEqual(writtenOf(null), []);
});

test('the Board tab offers the boards of the workspace the side panel is of, beginning on the channel\'s own', () => {
  const all = [
    { id: 'shop', name: 'Shop', created: 1, usual: true, projects: ['till', 'stock'], channels: [] },
    { id: 'home', name: 'Home', created: 2, projects: ['garden', 'roof'], channels: [] },
  ];
  const boards = ['till', 'garden', 'stock', 'roof'];
  const shop = { kind: 'space' as const, id: 'shop' };
  const home = { kind: 'space' as const, id: 'home' };
  assert.deepEqual(boardsShown(boards, 'stock', shop, all), { offered: ['stock', 'till'], begins: 'stock' }, 'its own first, then the rest of its workspace in their order');
  assert.deepEqual(boardsShown(boards, 'roof', home, all), { offered: ['roof', 'garden'], begins: 'roof' });
  assert.deepEqual(boardsShown(boards, null, home, all), { offered: ['garden', 'roof'], begins: 'garden' }, 'a channel of no project begins on the first');
  assert.deepEqual(boardsShown(boards, 'notes', shop, all), { offered: ['till', 'stock'], begins: 'till' }, 'a channel whose project has no board begins on the first');
  assert.deepEqual(boardsShown(boards, 'garden', shop, all), { offered: ['garden', 'till', 'stock'], begins: 'garden' }, 'a channel of another workspace still has its own board, and the panel\'s workspace after it');
  assert.deepEqual(boardsShown(['till', 'garden', 'new'], null, shop, all), { offered: ['till', 'new'], begins: 'till' }, 'one no workspace names yet is the usual one\'s, as the side panel lists it');
  assert.deepEqual(boardsShown(boards, null, { kind: 'all' }, []), { offered: boards, begins: 'till' }, 'before the workspaces are read, every board');
  assert.deepEqual(boardsShown([], 'till', shop, all), { offered: [], begins: null });
});

test('the Board tab lists as many of a column as the panel holds, and says the rest', () => {
  assert.deepEqual(firstOf(137, 900, 28), { shown: 33, more: 104 }, 'a long column');
  assert.deepEqual(firstOf(10, 900, 28), { shown: 10, more: 0 }, 'a short one whole');
  assert.deepEqual(firstOf(35, 900, 28), { shown: 35, more: 0 }, 'all of it where only a couple more would be said');
  assert.deepEqual(firstOf(36, 900, 28), { shown: 33, more: 3 });
  assert.deepEqual(firstOf(20, 0, 28), { shown: 5, more: 15 }, 'a few at least, before the panel is measured');
  assert.deepEqual(firstOf(0, 900, 28), { shown: 0, more: 0 });
});

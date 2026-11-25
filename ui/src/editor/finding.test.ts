import assert from 'node:assert/strict';
import { test } from 'node:test';
import { answers, asks, counted, oneLine, placed, whichOf } from './finding.ts';
import type { Pressed } from './finding.ts';

const key = (name: string, held: Partial<{ ctrlKey: boolean; metaKey: boolean; shiftKey: boolean; altKey: boolean }> = {}) => ({ key: name, ctrlKey: false, metaKey: false, shiftKey: false, altKey: false, ...held });
// One editor showing, in the pane that has the keyboard, with nothing over the page or beside it.
const page = (over: Partial<Pressed> = {}): Pressed => ({ keyboard: 'nowhere', focused: true, over: false, open: false, drawer: false, ...over });

test('the key that finds is Ctrl and F, and on a Mac the one a Mac finds by', () => {
  assert.equal(asks(key('f', { ctrlKey: true }), false), 'find');
  assert.equal(asks(key('F', { ctrlKey: true }), false), 'find', 'however the letter is said');
  assert.equal(asks(key('f', { metaKey: true }), true), 'find');
  assert.equal(asks(key('f', { ctrlKey: true }), true), null, 'on a Mac Ctrl and F moves the caret');
  assert.equal(asks(key('f', { metaKey: true }), false), null);
  assert.equal(asks(key('f'), false), null, 'a letter typed is a letter');
  assert.equal(asks(key('f', { ctrlKey: true, shiftKey: true }), false), null);
  assert.equal(asks(key('f', { ctrlKey: true, altKey: true }), false), null, 'with Alt it asks to replace, which only the text itself answers');
});

test('the keys that go on to a match, and Escape', () => {
  assert.equal(asks(key('F3'), false), 'next');
  assert.equal(asks(key('F3', { shiftKey: true }), false), 'before');
  assert.equal(asks(key('g', { ctrlKey: true }), false), 'next');
  assert.equal(asks(key('G', { ctrlKey: true, shiftKey: true }), false), 'before');
  assert.equal(asks(key('g', { metaKey: true }), true), 'next');
  assert.equal(asks(key('Escape'), false), 'away');
  assert.equal(asks(key('Escape', { shiftKey: true }), false), null);
  assert.equal(asks(key('Enter'), false), null);
});

test('the key that finds is the editor\'s wherever the keyboard is on its page, and never in a field somebody types in', () => {
  assert.ok(answers('find', page()), 'nothing has the keyboard: somebody pressed on a text that is only read');
  assert.ok(answers('find', page({ keyboard: 'page' })), 'on a button of the page, or the line of the side panel that opened the text');
  assert.ok(answers('find', page({ keyboard: 'page', drawer: true })), 'or of a drawer beside the text');
  assert.ok(!answers('find', page({ keyboard: 'field' })), 'a field of the page, of a drawer or of a dialog is the browser\'s to find in');
  assert.ok(!answers('find', page({ over: true })), 'under a dialog the page is not what is being used');
  assert.ok(!answers('find', page({ keyboard: 'here' })), 'in the text or the bar it is answered there, and not twice');
  assert.ok(!answers('find', page({ keyboard: 'another' })), 'in another editor it is that one\'s');
});

test('the page a key is for is the pane somebody is in', () => {
  assert.ok(answers('find', page({ focused: true })));
  assert.ok(!answers('find', page({ focused: false })), 'another pane was the last used: of two texts side by side, one answers');
  assert.ok(!answers('find', page({ keyboard: 'pane' })), 'the keyboard is on something of another pane, which may be a page with no text');
  assert.ok(!answers('next', page({ keyboard: 'pane', open: true })));
});

test('the keys that go on to a match are answered only while the bar is open', () => {
  assert.ok(!answers('next', page()));
  assert.ok(answers('next', page({ open: true })));
  assert.ok(answers('before', page({ open: true, keyboard: 'page' })));
  assert.ok(!answers('next', page({ open: true, keyboard: 'field' })));
});

test('Escape on the page puts the bar away only where nothing else can be meant by it', () => {
  assert.ok(answers('away', page({ open: true })), 'nothing has the keyboard and nothing else is open');
  assert.ok(!answers('away', page()), 'no bar is open: the key is nobody\'s here');
  assert.ok(!answers('away', page({ open: true, drawer: true })), 'a drawer is put away first, and the bar by the next press');
  assert.ok(!answers('away', page({ open: true, keyboard: 'page' })), 'on a button it may be that button\'s menu that is meant');
  assert.ok(!answers('away', page({ open: true, keyboard: 'field' })));
  assert.ok(!answers('away', page({ open: true, over: true })), 'a dialog is what Escape closes');
  assert.ok(!answers('away', page({ open: true, keyboard: 'here' })), 'in the text and in the bar the editor answers it itself');
});

test('the bar says which match this is of how many, or why there is none', () => {
  const found = { asked: true, reads: true, matches: 17, more: false, at: 2 };
  assert.equal(counted(found), '3 of 17');
  assert.equal(counted({ ...found, at: -1 }), '17 matches', 'the caret is at none of them');
  assert.equal(counted({ ...found, matches: 1, at: -1 }), '1 match');
  assert.equal(counted({ ...found, matches: 0, at: -1 }), 'No matches');
  assert.equal(counted({ ...found, reads: false, matches: 0, at: -1 }), 'Not an expression');
  assert.equal(counted({ ...found, asked: false, reads: false, matches: 0, at: -1 }), '', 'nothing is searched for');
  assert.equal(counted({ ...found, matches: 10000, more: true, at: 0 }), '1 of 10000+', 'counted as far as it is counted, and said to be more');
  assert.equal(counted({ ...found, matches: 10000, more: true, at: -1 }), '10000+ matches');
});

test('what is selected is searched for where it is one line', () => {
  assert.equal(oneLine('deploy'), 'deploy');
  assert.equal(oneLine('do: noop'), 'do: noop');
  assert.equal(oneLine(''), '');
  assert.equal(oneLine('steps:\n  build'), '');
  assert.equal(oneLine('a\r\nb'), '');
});

test('the match the caret is at is found among them by where it is', () => {
  const matches = [
    { from: 2, to: 5 },
    { from: 9, to: 12 },
    { from: 40, to: 43 },
  ];
  assert.equal(whichOf(matches, 2, 5), 0);
  assert.equal(whichOf(matches, 9, 12), 1);
  assert.equal(whichOf(matches, 40, 43), 2);
  assert.equal(whichOf(matches, 9, 11), -1, 'part of a match selected is not the match');
  assert.equal(whichOf(matches, 6, 6), -1);
  assert.equal(whichOf([], 0, 0), -1);
});

test('the bar stands clear of the line numbers and of a drawer, and stays where it was under a drawer that leaves it no room', () => {
  assert.deepEqual(placed({ width: 980, gutters: 44, covered: 0 }), { right: 16, widest: 912 });
  // A drawer over the right of the text: the bar is beside it.
  assert.deepEqual(placed({ width: 980, gutters: 44, covered: 410 }), { right: 418, widest: 510 });
  // A phone: the drawer has the screen, and the bar is where it was, under it.
  assert.deepEqual(placed({ width: 390, gutters: 40, covered: 390 }), { right: 16, widest: 326 });
  assert.deepEqual(placed({ width: 700, gutters: 44, covered: 400 }), { right: 16, widest: 632 }, 'too little is left beside it');
  // A drawer that lies wholly to the right of this editor, in another pane, covers nothing of it.
  assert.deepEqual(placed({ width: 480, gutters: 44, covered: -120 }), { right: 16, widest: 412 });
  assert.equal(placed({ width: 30, gutters: 44, covered: 0 }).widest, 0);
});

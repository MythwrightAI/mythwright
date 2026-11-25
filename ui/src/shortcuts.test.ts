import assert from 'node:assert/strict';
import { test } from 'node:test';
import { asksForComposer, asksForSearch, asksForShortcuts, foldsSide, railView, shown, SHORTCUTS, tabAfter, turnsTabs } from './shortcuts.ts';

const press = (key: string, held: Partial<{ ctrlKey: boolean; metaKey: boolean; shiftKey: boolean }> = {}) => ({ key, ctrlKey: false, metaKey: false, shiftKey: false, ...held });

test('Ctrl, Shift and the slash key ask for the shortcuts, however the keyboard names the key', () => {
  assert.ok(asksForShortcuts(press('?', { ctrlKey: true, shiftKey: true })), 'with Shift the key is a question mark');
  assert.ok(asksForShortcuts(press('/', { ctrlKey: true, shiftKey: true })));
  assert.ok(asksForShortcuts(press('?', { metaKey: true, shiftKey: true })), '⌘ on a Mac');
  assert.ok(!asksForShortcuts(press('?', { shiftKey: true })), 'a question mark typed is a question mark');
  assert.ok(!asksForShortcuts(press('/', { ctrlKey: true })), 'without Shift it is an editor\'s to comment a line with');
});

test('Ctrl and K open the search of everything and close it, and Ctrl, Shift and F open it', () => {
  assert.equal(asksForSearch(press('k', { ctrlKey: true })), 'toggle');
  assert.equal(asksForSearch(press('K', { metaKey: true })), 'toggle', '⌘ on a Mac, with the key named as it is written');
  assert.equal(asksForSearch(press('F', { ctrlKey: true, shiftKey: true })), 'open', 'with Shift the key is named in capitals');
  assert.equal(asksForSearch(press('f', { metaKey: true, shiftKey: true })), 'open');
  assert.equal(asksForSearch(press('f', { ctrlKey: true })), null, 'Ctrl and F alone is a find of its own');
  assert.equal(asksForSearch(press('F', { shiftKey: true })), null, 'a capital F typed is a capital F');
  assert.equal(asksForSearch(press('k', { ctrlKey: true, shiftKey: true })), null);
  assert.equal(asksForSearch({ ...press('f', { ctrlKey: true, shiftKey: true }), altKey: true }), null);
});

test('Ctrl and the backslash fold the side panel', () => {
  assert.ok(foldsSide(press('\\', { ctrlKey: true })));
  assert.ok(foldsSide(press('\\', { metaKey: true })));
  assert.ok(!foldsSide(press('\\')) && !foldsSide(press('\\', { ctrlKey: true, shiftKey: true })));
});

test('a key is written as it is on the keyboard in front of the reader', () => {
  assert.equal(shown('Mod', false), 'Ctrl');
  assert.equal(shown('Mod', true), '⌘');
  assert.equal(shown('Alt', true), '⌥');
  assert.equal(shown('Enter', true), 'Enter');
});

test('the panel lists itself, and nothing twice in one place', () => {
  const anywhere = SHORTCUTS[0]!.rows;
  assert.deepEqual(anywhere.find((row) => row.does === 'Keyboard shortcuts')?.keys, ['Mod', 'Shift', '/']);
  for (const section of SHORTCUTS) {
    const said = section.rows.map((row) => row.keys.join('+'));
    assert.equal(new Set(said).size, said.length, `${section.title} gives one key two things to do`);
  }
});

test('Alt and a number show that view of the rail in the side panel', () => {
  const alt = (code: string, held = {}) => ({ key: '', code, ctrlKey: false, metaKey: false, shiftKey: false, altKey: true, ...held });
  assert.equal(railView(alt('Digit1')), 'channels');
  assert.equal(railView(alt('Digit2')), 'resources');
  assert.equal(railView(alt('Digit3')), 'inbox');
  assert.equal(railView(alt('Digit4')), 'extensions');
  // Topics was given the next number when it came to the rail: the others kept theirs.
  assert.equal(railView(alt('Digit5')), 'topics');
  assert.equal(railView(alt('Digit6')), null, 'there are five');
  assert.equal(railView(alt('Digit1', { altKey: false })), null, 'a number typed is a number');
  assert.equal(railView(alt('Digit1', { ctrlKey: true })), null);
});

test('a slash asks for the box a message is written in, unless something is being typed', () => {
  assert.ok(asksForComposer(press('/'), false));
  assert.ok(!asksForComposer(press('/'), true), 'in a box, a slash is a slash');
  assert.ok(!asksForComposer(press('/', { ctrlKey: true }), false), 'with Ctrl it is somebody else\'s');
});

test('Ctrl and Tab turn to the next tab, and with Shift to the one before', () => {
  assert.equal(turnsTabs(press('Tab', { ctrlKey: true })), 1);
  assert.equal(turnsTabs(press('Tab', { ctrlKey: true, shiftKey: true })), -1);
  assert.equal(turnsTabs(press('Tab')), 0, 'Tab by itself moves among what is on the page');
  assert.equal(turnsTabs(press('Tab', { ctrlKey: true, metaKey: true })), 0);
});

test('Ctrl and the full stop turn to the next tab, and Ctrl and the comma to the one before, in a browser too', () => {
  assert.equal(turnsTabs(press('.', { ctrlKey: true })), 1);
  assert.equal(turnsTabs(press(',', { ctrlKey: true })), -1);
  assert.equal(turnsTabs(press('>', { ctrlKey: true, shiftKey: true })), 1, 'with Shift the key is named by what it types');
  assert.equal(turnsTabs(press('<', { ctrlKey: true, shiftKey: true })), -1);
  assert.equal(turnsTabs(press('.')), 0, 'a full stop typed is a full stop');
  assert.equal(turnsTabs(press(',', { metaKey: true })), 0, '⌘ and the comma is the settings of whatever a Mac is running');
});

test('the tabs go round: after the last is the first, and before the first is the last', () => {
  const tabs = ['start', 'channel', 'board'];
  assert.equal(tabAfter(tabs, 'start', 1), 'channel');
  assert.equal(tabAfter(tabs, 'board', 1), 'start');
  assert.equal(tabAfter(tabs, 'start', -1), 'board');
  assert.equal(tabAfter(['start'], 'start', 1), null, 'with one tab there is nowhere to turn to');
  assert.equal(tabAfter(tabs, 'gone', 1), 'start');
});

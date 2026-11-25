import assert from 'node:assert/strict';
import { test } from 'node:test';
import { format, newline, toolFor } from './format.ts';
import type { Tool, Written } from './format.ts';

// What is written with the selection drawn in it: `[` and `]` round what
// is selected, `|` for a caret.
function written(drawn: string): Written {
  const caret = drawn.indexOf('|');
  if (caret !== -1) return { text: drawn.replace('|', ''), from: caret, to: caret };
  const from = drawn.indexOf('[');
  const to = drawn.indexOf(']') - 1;
  return { text: drawn.replace('[', '').replace(']', ''), from, to };
}
function drawn(now: Written): string {
  if (now.from === now.to) return `${now.text.slice(0, now.from)}|${now.text.slice(now.from)}`;
  return `${now.text.slice(0, now.from)}[${now.text.slice(now.from, now.to)}]${now.text.slice(now.to)}`;
}
const after = (tool: Tool, before: string) => drawn(format(tool, written(before)));

test('a tool puts its marks round what is selected, and keeps it selected', () => {
  assert.equal(after('bold', 'say [this] now'), 'say **[this]** now');
  assert.equal(after('italic', 'say [this] now'), 'say _[this]_ now');
  assert.equal(after('underline', 'say [this] now'), 'say ++[this]++ now');
  assert.equal(after('strike', 'say [this] now'), 'say ~~[this]~~ now');
  assert.equal(after('code', 'say [this] now'), 'say `[this]` now');
});

test('used again, a tool takes its marks away', () => {
  assert.equal(after('bold', 'say **[this]** now'), 'say [this] now');
  assert.equal(after('bold', 'say [**this**] now'), 'say [this] now', 'with the marks selected too');
});

test('the marks go against the words, not against the spaces selected with them', () => {
  assert.equal(after('bold', 'say[ this ]now'), 'say **[this]** now');
});

test('with nothing selected a tool opens a pair to write in, and used again closes it', () => {
  assert.equal(after('bold', 'say |'), 'say **|**');
  assert.equal(after('bold', 'say **|**'), 'say |');
});

test('over several lines each line is given the marks, and all of them lose them together', () => {
  assert.equal(after('italic', '[one\n\ntwo]'), '_[one_\n\n_two]_');
  assert.equal(after('italic', '_[one_\n\n_two]_'), '[one\n\ntwo]');
  assert.equal(after('italic', '[_one_\ntwo]'), '[_one_\n_two]_', 'the one that had them keeps them');
});

test('a list is made of the lines selected, numbered in their order', () => {
  assert.equal(after('bullets', '[one\ntwo]'), '[- one\n- two]');
  assert.equal(after('numbers', '[one\n\ntwo]'), '[1. one\n\n2. two]');
});

test('a list made of a list is those lines as they were', () => {
  assert.equal(after('bullets', '[- one\n- two]'), '[one\ntwo]');
});

test('one kind of line becomes another without keeping what it began with', () => {
  assert.equal(after('numbers', '[- one\n- two]'), '[1. one\n2. two]');
  assert.equal(after('quote', '# a hea|ding'), '> a hea|ding');
  assert.equal(after('h2', '> said| so'), '## said| so');
  assert.equal(after('text', '### plain| again'), 'plain| again');
});

test('a caret keeps its place among the words when its line is begun differently', () => {
  assert.equal(after('bullets', 'before\nthe li|ne\nafter'), 'before\n- the li|ne\nafter');
  assert.equal(after('bullets', '|'), '- |');
  assert.equal(after('bullets', '  further| in'), '  - further| in');
});

test('a block of code is put round the lines selected, and taken away again', () => {
  assert.equal(after('block', 'see:\n[let x = 1;\nlet y = 2;]\nok'), 'see:\n```\n[let x = 1;\nlet y = 2;]\n```\nok');
  assert.equal(after('block', 'see:\n[```\nlet x = 1;\n```]\nok'), 'see:\n[let x = 1;]\nok');
  assert.equal(after('block', '|'), '```\n|\n```');
  assert.equal(after('block', '```\n|\n```'), '|');
});

test('a link is made of the words selected, and where it leads is next to be written', () => {
  // Not drawn: the link's own brackets would be taken for the selection's.
  assert.deepEqual(format('link', written('read [the docs] first')), { text: 'read [the docs](https://) first', from: 16, to: 24 });
  assert.deepEqual(format('link', written('[https://example.com]')), { text: '[words](https://example.com)', from: 1, to: 6 });
  assert.deepEqual(format('link', written('see |')), { text: 'see [words](https://)', from: 5, to: 10 });
});

test('a new line in a list begins the next item, with the next number', () => {
  assert.equal(drawn(newline(written('- one|'))!), '- one\n- |');
  assert.equal(drawn(newline(written('  3. three|'))!), '  3. three\n  4. |');
  assert.equal(drawn(newline(written('> said|'))!), '> said\n> |');
});

test('a new line on an item with nothing in it ends the list', () => {
  assert.equal(drawn(newline(written('- one\n- |'))!), '- one\n|');
});

test('a new line anywhere else is only a new line', () => {
  assert.equal(newline(written('plain|')), null);
  assert.equal(newline(written('[- one]')), null);
});

test('the keys that ask for a tool', () => {
  const press = (key: string, held: Partial<{ ctrlKey: boolean; metaKey: boolean; shiftKey: boolean; altKey: boolean; code: string }> = {}) => ({ key, ctrlKey: false, metaKey: false, shiftKey: false, altKey: false, ...held });
  assert.equal(toolFor(press('b', { ctrlKey: true })), 'bold');
  assert.equal(toolFor(press('i', { metaKey: true })), 'italic', '⌘ on a Mac');
  assert.equal(toolFor(press('u', { ctrlKey: true })), 'underline');
  assert.equal(toolFor(press('X', { ctrlKey: true, shiftKey: true })), 'strike');
  assert.equal(toolFor(press('C', { ctrlKey: true, shiftKey: true })), 'code');
  assert.equal(toolFor(press('C', { ctrlKey: true, shiftKey: true, altKey: true })), 'block');
  assert.equal(toolFor(press('&', { ctrlKey: true, shiftKey: true, code: 'Digit7' })), 'numbers', 'whatever the key types with Shift held');
  assert.equal(toolFor(press('*', { ctrlKey: true, shiftKey: true, code: 'Digit8' })), 'bullets');
  assert.equal(toolFor(press('(', { ctrlKey: true, shiftKey: true, code: 'Digit9' })), 'quote');
  assert.equal(toolFor(press('b')), null, 'a letter typed is a letter');
  assert.equal(toolFor(press('k', { ctrlKey: true })), null, 'a key that is somebody else\'s');
});

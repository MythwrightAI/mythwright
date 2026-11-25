import assert from 'node:assert/strict';
import { test } from 'node:test';
import { backMarks, notesOf } from './cited.ts';
import { read } from './prose.ts';

const numbered = (text: string) => notesOf(read(text)).list.map((note) => [note.n, note.label, note.text, note.uses]);

test('sources are numbered in the order first used, whatever their labels, then those used nowhere', () => {
  const text = 'Frogs died.[^b] Fungus.[^a] Again.[^b]\n\n[^a]: Ada, *Notes*, 1843.\n[^z]: Unused.\n[^b]: Grace, 1952. https://example.org/b (checked 2026-11-25)';
  assert.deepEqual(numbered(text), [
    [1, 'b', 'Grace, 1952. https://example.org/b (checked 2026-11-25)', 2],
    [2, 'a', 'Ada, *Notes*, 1843.', 1],
    [3, 'z', 'Unused.', 0],
  ]);
});

test('a marker whose source is not written is numbered, and says nothing', () => {
  assert.deepEqual(numbered('A claim.[^1]'), [[1, '1', null, 1]]);
});

test('markers are read in headings, lists, quotes and marks, and a source line is no paragraph', () => {
  const blocks = read('# A[^h]\n\n- item **bold[^i]**\n\n> quoted[^q]\n\n[^q]: Q.');
  assert.deepEqual(notesOf(blocks).list.map((note) => note.label), ['h', 'i', 'q']);
  assert.equal(blocks.at(-1)?.is, 'note');
  assert.deepEqual(read('[^1]: Ada, *Notes*.')[0], { is: 'note', label: '1', text: 'Ada, *Notes*.', spans: [{ is: 'text', text: 'Ada, ' }, { is: 'italic', inside: [{ is: 'text', text: 'Notes' }] }, { is: 'text', text: '.' }] });
});

test('a source cites nothing, and its address is a link that is never fetched', () => {
  const [note] = read('[^1]: see [^1] and https://example.org/x');
  assert.deepEqual(note, {
    is: 'note',
    label: '1',
    text: 'see [^1] and https://example.org/x',
    spans: [{ is: 'text', text: 'see ' }, { is: 'text', text: '[^1]' }, { is: 'text', text: ' and ' }, { is: 'link', to: 'https://example.org/x', inside: [{ is: 'text', text: 'https://example.org/x' }] }],
  });
});

test('text with no marker and no source has no sources', () => {
  assert.deepEqual(numbered('Plain words, [a link](https://example.org) and `[^1]` as code.'), []);
});

test('the way back from a source is an arrow, or a letter for each use', () => {
  assert.deepEqual(backMarks(1), ['↑']);
  assert.deepEqual(backMarks(3), ['a', 'b', 'c']);
});

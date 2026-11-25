import assert from 'node:assert/strict';
import { test } from 'node:test';
import { parsed, written } from './markdown.ts';

const again = (markdown: string) => written(parsed(markdown));

test('a link to a page goes through the editor as it was written', () => {
  const text = 'Many died of [Chytrid fungus](topic:@-topics-49), and [what Ada said](topic:@-topics-49#817039).\n';
  assert.equal(again(text), text);
});

test('footnotes go through the editor as they were written, sources one under another', () => {
  const text = 'Frogs died.[^1] Fungus did it.[^ada]\n\n[^1]: Grace, *Frogs*, Press, 1952. https://example.org/frogs (checked 2026-11-25)\n[^ada]: Ada, *Notes*, 1843.\n';
  assert.equal(again(text), text);
  const doc = parsed(text);
  assert.deepEqual(doc.content.content.map((block) => block.type.name), ['paragraph', 'footnote_def', 'footnote_def']);
  assert.equal(doc.firstChild?.child(1).type.name, 'footnote_ref');
});

test('a source with nothing but an address is still a source, not a link defined for elsewhere', () => {
  const text = 'A claim.[^1]\n\n[^1]: https://example.org/only\n';
  assert.equal(again(text), text);
});

test('what was written before footnotes were known goes through as it did', () => {
  const text = '# Title\n\nWords with **bold** and [a site](https://example.org).\n\n- one\n- two\n';
  assert.equal(again(text), text);
});

test('a table goes through the editor byte for byte, as the lines it was written as', () => {
  const text = [
    '# Practices',
    '',
    'What the studies found, by practice.[^1]',
    '',
    '| Practice | Minutes a day | Studies |',
    '|:---------|:-------------:|--------:|',
    '| [Breath](topic:@-topics-4) | 10 | 12 |',
    '| **Body scan** \\| lying | 20–45 | 7 |',
    '|  Walking  |  | `3` |',
    '',
    'After the table, words.',
    '',
    '[^1]: Ada, *Notes*, 1843.',
    '',
  ].join('\n');
  assert.equal(again(text), text);
  const doc = parsed(text);
  assert.deepEqual(
    doc.content.content.map((block) => block.type.name),
    ['heading', 'paragraph', 'pipe_table', 'paragraph', 'footnote_def'],
  );
  assert.equal(doc.child(2).textContent, text.split('\n').slice(4, 9).join('\n'));
});

test('a table written straight after words is kept whole, an empty line put between them as between any two blocks', () => {
  assert.equal(again('Straight after words:\na | b\n--|--:\n1 | 2\n'), 'Straight after words:\n\na | b\n--|--:\n1 | 2\n');
});

test('what only looks like a table is written back as it was read', () => {
  for (const text of ['A line with a | in it.\n', '| one | two |\n\nno rule under it\n', '```\n| a | b |\n|---|---|\n```\n']) assert.equal(again(text), text);
});

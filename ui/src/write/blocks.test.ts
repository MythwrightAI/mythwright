import assert from 'node:assert/strict';
import { test } from 'node:test';
import { BLOCKS, cannotKeep, matching, splitHead } from './blocks.ts';

test('with nothing typed after the slash, every block is offered', () => {
  assert.deepEqual(matching(''), BLOCKS);
});

test('blocks are found by their name first, then by another word for them', () => {
  assert.deepEqual(matching('h').map((block) => block.id), ['h1', 'h2', 'h3', 'divider']);
  assert.deepEqual(matching('heading 2').map((block) => block.id), ['h2']);
  assert.deepEqual(matching('ord').map((block) => block.id), ['numbers']);
  assert.deepEqual(matching('LINE').map((block) => block.id), ['divider']);
  assert.deepEqual(matching('zzz'), []);
});

test("an article's boxes are found by their names", () => {
  assert.deepEqual(matching('info').map((block) => block.id), ['infobox']);
  assert.deepEqual(matching('series').map((block) => block.id), ['series']);
  assert.deepEqual(matching('note').map((block) => block.id), ['note']);
  assert.deepEqual(matching('facts').map((block) => block.id), ['infobox']);
});

test('the settings at the head of a file are set aside and nothing of them is lost', () => {
  const file = '---\npublish: true\ntitle: Shop\n---\n# Shop\n\nText.\n';
  const { head, body } = splitHead(file);
  assert.equal(head, '---\npublish: true\ntitle: Shop\n---\n');
  assert.equal(body, '# Shop\n\nText.\n');
  assert.equal(head + body, file);
});

test('a rule further down is not the head of the file', () => {
  const file = '# Shop\n\n---\n\nMore.\n';
  assert.deepEqual(splitHead(file), { head: '', body: file });
});

test('a document with something the block editor cannot keep says what it is', () => {
  assert.equal(cannotKeep('# Title\n\nJust *writing* and a [link](x).\n\n- a\n- b\n'), null);
  assert.equal(cannotKeep('Text.\n\n<div class="x">raw</div>\n'), 'raw HTML');
});

test('footnotes are kept by the block editor', () => {
  assert.equal(cannotKeep('Text.[^1]\n\n[^1]: A note.\n'), null);
});

test('what only looks like a table inside a code block is left alone', () => {
  assert.equal(cannotKeep('```\n| a | b |\n|---|---|\n<div>\n```\n'), null);
});

test('a table is kept by the block editor, as its text', () => {
  assert.equal(cannotKeep('| Item | Price |\n|---|--:|\n| Mug | 9 |\n'), null);
  assert.equal(cannotKeep('| Item | Price |\n|---|---|\n\n<b>raw</b>\n'), 'raw HTML');
});

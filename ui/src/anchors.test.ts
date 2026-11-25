import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import { anchorOf, blocksOfMarkdown, found, spanOf, within } from './anchors.ts';
import type { Anchor } from './anchors.ts';

interface Cases {
  documents: Record<string, string>;
  blocks: [string, [string, string[]][]][];
  found: { said: string; document: string; anchor: Anchor; found: { how: string; block: number; before: string } | null; source?: string }[];
  anchored: { said: string; document: string; block: number; from: number; to: number; anchor: Anchor | null }[];
}
const cases = JSON.parse(readFileSync(new URL('./anchorcases.json', import.meta.url), 'utf8')) as Cases;

test('a Markdown file is read as the blocks it shows, each with the headings over it', () => {
  for (const [name, blocks] of cases.blocks) {
    const read = blocksOfMarkdown(cases.documents[name]!);
    assert.deepEqual(
      read.map((block) => [block.text, block.under]),
      blocks,
      name,
    );
    // Each character is where the file has it.
    for (const block of read) {
      assert.equal(block.at.length, block.text.length, `${name}: ${block.text}`);
      [...block.text].forEach((char, at) => char !== ' ' && assert.equal(cases.documents[name]![block.at[at]!], char, `${name}: ${block.text}[${at}]`));
    }
  }
});

for (const one of cases.found) {
  test(`an anchor is found again: ${one.said}`, () => {
    const source = cases.documents[one.document]!;
    const blocks = blocksOfMarkdown(source);
    const at = found(one.anchor, blocks);
    if (!one.found) {
      assert.equal(at, null);
      return;
    }
    assert.ok(at, 'found');
    const block = blocks[at.block]!;
    assert.deepEqual({ how: at.how, block: at.block, before: block.text.slice(0, at.from) }, one.found);
    const span = spanOf(block, at.from, at.to);
    assert.equal(source.slice(span.from, span.to), one.source);
    // And the characters between those places are the words found.
    assert.deepEqual(within(block, span.from, span.to), { from: at.from, to: at.to });
  });
}

for (const one of cases.anchored) {
  test(`what a comment on a selection is about: ${one.said}`, () => {
    const block = blocksOfMarkdown(cases.documents[one.document]!)[one.block]!;
    const anchor = anchorOf(block, one.from, one.to);
    assert.deepEqual(anchor, one.anchor);
    // What is made is found where it was made.
    if (anchor) {
      const at = found(anchor, blocksOfMarkdown(cases.documents[one.document]!));
      assert.equal(at?.block, one.block);
      assert.equal(at?.how, 'exact');
    }
  });
}

test('every way an anchor is found is a case', () => {
  const hows = new Set(cases.found.map((one) => one.found?.how ?? null));
  assert.deepEqual([...hows].sort(), ['anywhere', 'exact', 'heading', null].sort());
});

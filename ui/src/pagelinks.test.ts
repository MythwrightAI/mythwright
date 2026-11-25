import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import { completePage, linkWords, pageLinkOf, pagesOffered, pageTyping, writtenLink } from './pagelinks.ts';
import { isBoxLanguage } from './boxes.ts';
import { read, spans } from './prose.ts';
import type { Span } from './prose.ts';
import type { TopicLine } from './topicpage.ts';
import { treeOfLines } from './wikipage.ts';

const table = JSON.parse(readFileSync(new URL('./linkcases.json', import.meta.url), 'utf8')) as { cases: [string, [string, number | null][]][] };

/** The links of some spans, in the order written. */
function pagesIn(spans: Span[], found: [string, number | null][]): void {
  for (const span of spans) {
    if (span.is === 'page') found.push([span.topic, span.reply]);
    if ('inside' in span) pagesIn(span.inside, found);
  }
}

test('the pages a text links to are read case for case as the daemon reads them', () => {
  for (const [text, wanted] of table.cases) {
    const found: [string, number | null][] = [];
    for (const block of read(text)) {
      if (block.is === 'list') block.items.forEach((item) => pagesIn(item.spans, found));
      else if (block.is === 'table') [...block.head, ...block.rows.flat()].forEach((cell) => pagesIn(cell, found));
      else if ('spans' in block) pagesIn(block.spans, found);
      // A box's lines are drawn as lines of words are (`views/boxes.tsx`).
      else if (block.is === 'code' && isBoxLanguage(block.lang)) block.text.split('\n').forEach((line) => pagesIn(spans(line), found));
    }
    const once = found.filter((link, at) => found.findIndex((other) => other[0] === link[0] && other[1] === link[1]) === at);
    assert.deepEqual(once, wanted, text);
  }
});

test('a link is to a page by its id, and to a reply by its number', () => {
  assert.deepEqual(pageLinkOf('topic:@-topics-49'), { topic: '@-topics-49', reply: null });
  assert.deepEqual(pageLinkOf('topic:@-topics-49#817039'), { topic: '@-topics-49', reply: 817039 });
  assert.equal(pageLinkOf('topic:a/b'), null);
  assert.equal(pageLinkOf('https://example.org'), null);
  assert.equal(writtenLink('Chytrid fungus', '@-topics-49'), '[Chytrid fungus](topic:@-topics-49)');
  assert.equal(writtenLink('Ada', '@-topics-1', 12), '[Ada](topic:@-topics-1#12)');
});

test('a title is written as a link with no brackets to end its words early', () => {
  assert.equal(linkWords('Arrays [and] slices'), 'Arrays (and) slices');
  assert.equal(linkWords('  two\nlines '), 'two lines');
});

test('an address that begins as a page and names none is left as it was written', () => {
  const [block] = read('see [this](topic:a/b) now');
  assert.deepEqual(block, { is: 'text', spans: [{ is: 'text', text: 'see [this](topic:a/b) now' }] });
});

test('typing two brackets offers pages, and the one chosen is written as a link', () => {
  assert.deepEqual(pageTyping('see [[chy', 9), { from: 4, typed: 'chy' });
  assert.equal(pageTyping('see [chy', 8), null);
  assert.equal(pageTyping('see [[a]] b', 11), null);
  const done = completePage('see [[chy and', 9, 4, { id: '@-topics-49', title: 'Chytrid fungus' });
  assert.deepEqual(done, { text: 'see [Chytrid fungus](topic:@-topics-49) and', caret: 39 });
  assert.equal(completePage('[[chy]] x', 5, 0, { id: '@-topics-49', title: 'Chytrid fungus' }).text, '[Chytrid fungus](topic:@-topics-49) x', 'what closed it goes');
});

const line = (id: string, title: string, parent: string | null = null): TopicLine => ({ id, number: 0, title, status: null, summary: null, workspace: 'w', project: null, parent, position: 0, replies: 0, created: 0, updated: 0 });

test('the pages offered are those whose title starts with what was typed, then those holding it, each with its wiki', () => {
  const lines = [line('a', 'Amphibians'), line('f', 'Chytrid fungus', 'a'), line('d', 'Frog decline', 'a'), line('x', 'Fungi')];
  const offered = (typed: string, except: string | null = null) => pagesOffered(lines, treeOfLines(lines), typed, except).map((one) => [one.page.title, one.wiki]);
  assert.deepEqual(offered('fung'), [['Fungi', null], ['Chytrid fungus', 'Amphibians']]);
  assert.deepEqual(offered('FROG'), [['Frog decline', 'Amphibians']]);
  assert.deepEqual(offered('', 'a').map(([title]) => title), ['Chytrid fungus', 'Frog decline', 'Fungi'], 'not the page being written');
});

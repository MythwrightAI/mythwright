import assert from 'node:assert/strict';
import { test } from 'node:test';
import { contentsOf, headingsOf, isSlug, sectionAt, slugOf, slugsOf } from './contents.ts';
import type { Entry } from './contents.ts';
import { read } from './prose.ts';

test("a heading's slug is its words in small letters, hyphens for the rest, in any script", () => {
  const cases: [string, string][] = [
    ['History', 'history'],
    ['How it began', 'how-it-began'],
    ['  Rolling back, safely!  ', 'rolling-back-safely'],
    ['C++ & Rust: a comparison', 'c-rust-a-comparison'],
    ['1.2 Numbers stay', '1-2-numbers-stay'],
    ['Über Straße', 'über-straße'],
    ['Café', 'café'],
    ['Ｆｕｌｌ width', 'full-width'],
    ['日本語の見出し', '日本語の見出し'],
    ['Прыжок в длину', 'прыжок-в-длину'],
    ['हिन्दी शीर्षक', 'हिन्दी-शीर्षक'],
    ["it's — quoted “so”", 'it-s-quoted-so'],
    ['snake_case and kebab-case', 'snake-case-and-kebab-case'],
    ['???', 'section'],
    ['', 'section'],
  ];
  for (const [words, slug] of cases) assert.equal(slugOf(words), slug, words);
});

test('slugs are unique within a page: one called the same as an earlier one is numbered', () => {
  assert.deepEqual(slugsOf(['Notes', 'Notes', 'Notes']), ['notes', 'notes-2', 'notes-3']);
  // A heading whose own words are what a numbered one would be called keeps it apart from that one.
  assert.deepEqual(slugsOf(['Notes', 'Notes 2', 'Notes']), ['notes', 'notes-2', 'notes-3']);
  assert.deepEqual(slugsOf(['Notes', 'Notes', 'Notes 2']), ['notes', 'notes-2', 'notes-2-2']);
  assert.deepEqual(slugsOf(['!', '?']), ['section', 'section-2']);
  for (const slug of slugsOf(['A', 'a', 'A!', 'Über', 'über'])) assert.ok(isSlug(slug), slug);
});

test('what follows `#s/` is a slug only where it could be one', () => {
  for (const one of ['history', 'how-it-began', 'über-straße', '日本語', 'notes-2']) assert.ok(isSlug(one), one);
  for (const one of ['', '-history', 'history-', 'a--b', 'a b', 'a/b', 'A.b']) assert.ok(!isSlug(one), one);
});

test("an article's headings are read from what it is drawn as, its marks taken off", () => {
  const text = [
    'A lead paragraph.',
    '',
    '# **Bold** start',
    'Words.',
    '## A [link](https://example.com) and `code`',
    '### Deep',
    '#### Not a heading here',
    '```',
    '# Not a heading in code',
    '```',
    '## Deep',
  ].join('\n');
  assert.deepEqual(headingsOf(read(text)), [
    { level: 1, title: 'Bold start', slug: 'bold-start' },
    { level: 2, title: 'A link and code', slug: 'a-link-and-code' },
    { level: 3, title: 'Deep', slug: 'deep' },
    { level: 2, title: 'Deep', slug: 'deep-2' },
  ]);
});

test("a heading's footnote marker and a link to a page are read as the editor reads them: no words, and the words of the link", () => {
  const text = ['# Rolling back[^1]', '', '## See [Handbook](topic:t-1) first', '', '[^1]: The runbook.'].join('\n');
  assert.deepEqual(headingsOf(read(text)), [
    { level: 1, title: 'Rolling back', slug: 'rolling-back' },
    { level: 2, title: 'See Handbook first', slug: 'see-handbook-first' },
  ]);
});

/** The contents, a line for each entry: its number, its slug, how far in by its indent. */
const lines = (entries: Entry[], depth = 0): string[] => entries.flatMap((entry) => [`${'  '.repeat(depth)}${entry.number} ${entry.heading.slug}`, ...lines(entry.under, depth + 1)]);

test('the contents nest each heading under the last that stands less far in, numbered in place', () => {
  const heading = (level: 1 | 2 | 3, slug: string) => ({ level, title: slug, slug });
  assert.deepEqual(lines(contentsOf([heading(2, 'a'), heading(3, 'a1'), heading(3, 'a2'), heading(2, 'b'), heading(3, 'b1'), heading(1, 'c'), heading(2, 'c1')])), [
    '1 a',
    '  1.1 a1',
    '  1.2 a2',
    '2 b',
    '  2.1 b1',
    '3 c',
    '  3.1 c1',
  ]);
  // One with nothing above it standing less far in is at the top, however far in it is; a level skipped nests all the same.
  assert.deepEqual(lines(contentsOf([heading(3, 'x'), heading(2, 'y'), heading(1, 'z'), heading(3, 'z1')])), ['1 x', '2 y', '3 z', '  3.1 z1']);
  assert.deepEqual(contentsOf([]), []);
});

test('the section being read is the last heading at or above the line, and at the end the last that shows', () => {
  const tops = [40, 300, 900, 1000];
  assert.equal(sectionAt(tops, 20), -1, 'above the first heading');
  assert.equal(sectionAt(tops, 40), 0);
  assert.equal(sectionAt(tops, 299), 0);
  assert.equal(sectionAt(tops, 500), 1);
  assert.equal(sectionAt(tops, 2000), 3);
  // Scrolled to the end, a short last section that never reaches the line is the one read.
  assert.equal(sectionAt(tops, 500, 1100), 3);
  assert.equal(sectionAt(tops, 500, 950), 2);
  assert.equal(sectionAt([], 500), -1);
});

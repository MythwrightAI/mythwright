import assert from 'node:assert/strict';
import { test } from 'node:test';
import { asked, inTree, MOST_ASKED, pathPieces, projectOfId } from './pathlinked.ts';
import type { File, Known } from './pathlinked.ts';
import { read } from './prose.ts';

// Shop has these files; the services' project these; nothing else is a file anywhere, and of `elsewhere` nothing is known yet.
const FILES: Record<string, string[]> = {
  shop: ['docs/plans/x.md', 'README.md', 'src/main.rs', '@blueprints/fleet.bp', 'notes/a b.md'],
  'services': ['docs/plans/x.md'],
};
const known = (own: string | null): Known => ({
  own,
  projects: ['shop', 'services', 'elsewhere'],
  knows: (file: File) => (file.project === 'elsewhere' ? 'unknown' : (FILES[file.project] ?? []).includes(file.path) ? 'file' : 'not'),
});

/** The pieces as `text` for words and `[project:path]` for a file, to read a case at a glance. */
const linked = (words: string, own: string | null = 'shop') => pathPieces(words, known(own)).map((piece) => (piece.file ? `[${piece.file.project}:${piece.file.path}]` : piece.text));

test('a path of a file of the project the words are of is a piece that names it, and the rest is words', () => {
  assert.deepEqual(linked('The plan is docs/plans/x.md now'), ['The plan is ', '[shop:docs/plans/x.md]', ' now']);
  assert.deepEqual(linked('See README.md and src/main.rs.'), ['See ', '[shop:README.md]', ' and ', '[shop:src/main.rs]', '.']);
  assert.deepEqual(linked('./docs/plans/x.md'), ['[shop:docs/plans/x.md]']);
  assert.deepEqual(linked('The fleet is @blueprints/fleet.bp'), ['The fleet is ', '[shop:@blueprints/fleet.bp]']);
  // Put together the pieces are the words, letter for letter.
  const words = 'Read docs/plans/x.md, (src/main.rs) and "README.md"; then go.';
  assert.equal(
    pathPieces(words, known('shop'))
      .map((piece) => piece.text)
      .join(''),
    words,
  );
});

test('a project named before a path with a colon is the one it is looked for in', () => {
  assert.deepEqual(linked('services:docs/plans/x.md'), ['[services:docs/plans/x.md]']);
  assert.deepEqual(linked('in services:docs/plans/x.md', null), ['in ', '[services:docs/plans/x.md]']);
  // A word before a colon that is no project names none: the whole is words.
  assert.deepEqual(linked('nowhere:docs/plans/x.md'), ['nowhere:docs/plans/x.md']);
});

test('what a sentence or a line number puts after a path is not part of it', () => {
  assert.deepEqual(linked('at src/main.rs:12 it fails'), ['at ', '[shop:src/main.rs]', ':12 it fails']);
  assert.deepEqual(linked('(docs/plans/x.md)!'), ['(', '[shop:docs/plans/x.md]', ')!']);
});

test('a path that names no file the page knows is words, never a guess', () => {
  assert.deepEqual(linked('docs/plans/missing.md'), ['docs/plans/missing.md']);
  // Words of no project, with none named, name nothing.
  assert.deepEqual(linked('docs/plans/x.md', null), ['docs/plans/x.md']);
  // Nothing known yet is words until it is.
  assert.deepEqual(linked('elsewhere:docs/plans/x.md'), ['elsewhere:docs/plans/x.md']);
});

test('what is no path is never taken for one', () => {
  for (const words of ['e.g. this', 'i.e., that', 'v1.2.3', 'ada@example.com', 'https://a.invalid/docs/plans/x.md', '/srv/shop/docs/plans/x.md', 'docs/', '../docs/plans/x.md', '@ada said so']) {
    assert.deepEqual(
      pathPieces(words, known('shop')).filter((piece) => piece.file),
      [],
      words,
    );
    assert.deepEqual(asked(words, { ...known('shop'), knows: () => 'unknown' }), [], words);
  }
});

test('a path in backticks and in a block of code is found as it is written there', () => {
  // What prose reads as code is handed over as its text, and found there.
  const [block] = read('it is `docs/plans/x.md` here');
  assert.equal(block?.is, 'text');
  const code = block?.is === 'text' ? block.spans.find((span) => span.is === 'code') : undefined;
  assert.deepEqual(code && code.is === 'code' ? linked(code.text) : null, ['[shop:docs/plans/x.md]']);
  const [, fenced] = read('Look:\n```\ncat docs/plans/x.md\nls src/\n```');
  assert.deepEqual(fenced?.is === 'code' ? linked(fenced.text) : null, ['cat ', '[shop:docs/plans/x.md]', '\nls src/']);
});

test('the paths the page knows nothing of are asked, each once, a few at most', () => {
  const unknown = { ...known('shop'), knows: () => 'unknown' as const };
  assert.deepEqual(asked('README.md and README.md and docs/a.md', unknown), [
    { project: 'shop', path: 'README.md' },
    { project: 'shop', path: 'docs/a.md' },
  ]);
  assert.equal(asked('a/1.md b/2.md c/3.md d/4.md e/5.md', unknown).length, MOST_ASKED);
  // What is known, either way, is not asked again.
  assert.deepEqual(asked('README.md docs/plans/missing.md elsewhere:x/y.md', known('shop')), [{ project: 'elsewhere', path: 'x/y.md' }]);
});

test('the files a project page has read say whether a path is one, and nothing where they were not read', () => {
  const entries = [
    { path: 'docs', dir: true },
    { path: 'docs/x.md', dir: false },
  ];
  assert.equal(inTree(entries, 'docs/x.md'), 'file');
  assert.equal(inTree(entries, 'docs'), 'not');
  assert.equal(inTree(entries, 'docs/y.md'), 'not');
  assert.equal(inTree(undefined, 'docs/x.md'), 'unknown');
  // What the factory keeps of a project's own blueprints is in no list of its folder.
  assert.equal(inTree(entries, '@blueprints/fleet.bp'), 'unknown');
});

test('what a record or a channel is of is the project its id begins with, the longest that fits', () => {
  const projects = ['shop', 'shop-front'];
  assert.equal(projectOfId('shop-12', projects), 'shop');
  assert.equal(projectOfId('shop-front-3', projects), 'shop-front');
  assert.equal(projectOfId('shop-docs:guides:install', projects), 'shop');
  assert.equal(projectOfId('in_0123456789ab', projects), null);
});

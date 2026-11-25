import assert from 'node:assert/strict';
import { test } from 'node:test';
import type { Entry } from './api.ts';
import { byName } from './find.ts';

const file = (path: string): Entry => ({ path, name: path.split('/').pop()!, dir: false, size: 1, modified: 0 }) as Entry;
const folder = (path: string): Entry => ({ ...file(path), dir: true }) as Entry;
const tree = [
  folder('crates'),
  folder('crates/blueprint'),
  file('crates/blueprint/src/vocabulary.rs'),
  file('crates/blueprint/tests/suite/vocabulary.rs'),
  file('crates/blueprint/src/lower.rs'),
  file('docs/blueprints.md'),
  file('README.md'),
  file('ui/src/blueprint/scan.ts'),
  file('blueprints/std/board.bp'),
];
const found = (typed: string, limit?: number) => byName(tree, typed, limit).map((entry) => entry.path);

test('part of a name finds the file, while it is still being typed', () => {
  assert.deepEqual(found('vocab'), ['crates/blueprint/src/vocabulary.rs', 'crates/blueprint/tests/suite/vocabulary.rs']);
  assert.deepEqual(found('READ'), ['README.md'], 'however it is cased');
});

test('a name that is what was typed comes before one that starts with it, and that before one that holds it', () => {
  const names = [file('a/board.bp.old'), file('b/my-board.bp'), file('c/board.bp')];
  assert.deepEqual(byName(names, 'board.bp').map((entry) => entry.path), ['c/board.bp', 'a/board.bp.old', 'b/my-board.bp']);
});

test('names come before paths, and of two alike the one nearer the top is first', () => {
  assert.deepEqual(found('blueprint'), [
    'docs/blueprints.md',
    'blueprints/std/board.bp',
    'ui/src/blueprint/scan.ts',
    'crates/blueprint/src/lower.rs',
    'crates/blueprint/src/vocabulary.rs',
    'crates/blueprint/tests/suite/vocabulary.rs',
  ]);
});

test('a path can be typed, or several words that are each somewhere in it', () => {
  assert.deepEqual(found('std/board'), ['blueprints/std/board.bp']);
  assert.deepEqual(found('tests vocabulary'), ['crates/blueprint/tests/suite/vocabulary.rs']);
});

test('folders are not files, nothing typed finds nothing, and no more are given than asked for', () => {
  assert.deepEqual(found('crates'), ['crates/blueprint/src/lower.rs', 'crates/blueprint/src/vocabulary.rs', 'crates/blueprint/tests/suite/vocabulary.rs']);
  assert.deepEqual(found('   '), []);
  assert.deepEqual(found('zzz'), []);
  assert.equal(found('r', 2).length, 2);
});

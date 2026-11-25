import assert from 'node:assert/strict';
import { test } from 'node:test';
import { counted, diff, linesOf, sideOf, steps, wordsOf } from './worddiff.ts';
import type { Piece } from './worddiff.ts';

/** A difference written short: `[+put in]`, `[-taken out]`, and what is the same as it is. */
const said = (pieces: Piece[]) => pieces.map((piece) => (piece.kind === 'same' ? piece.text : `[${piece.kind === 'added' ? '+' : '-'}${piece.text}]`)).join('');

/** Every difference makes both texts again, each from its own side. */
function both(older: string, newer: string): Piece[] {
  const pieces = diff(older, newer);
  assert.equal(sideOf(pieces, 'older'), older, 'the older text, again');
  assert.equal(sideOf(pieces, 'newer'), newer, 'the newer text, again');
  for (const [at, piece] of pieces.entries()) {
    assert.ok(piece.text !== '', 'no piece says nothing');
    assert.ok(at === 0 || pieces[at - 1]!.kind !== piece.kind, 'pieces of a kind side by side are one');
  }
  return pieces;
}

test('the fewest steps make one list into another', () => {
  assert.deepEqual(steps(['a', 'b', 'c'], ['a', 'b', 'c'], 10), ['=', '=', '=']);
  assert.deepEqual(steps([], ['a'], 10), ['+']);
  assert.deepEqual(steps(['a'], [], 10), ['-']);
  assert.deepEqual(steps([], [], 10), []);
  assert.deepEqual(steps(['a', 'b', 'c'], ['a', 'x', 'c'], 10), ['=', '-', '+', '=']);
  const many = steps([...'abcabba'], [...'cbabac'], 10)!;
  assert.equal(many.filter((one) => one !== '=').length, 5, 'Myers’ own case: five steps, no more');
  assert.equal(steps([...'abcdef'], [...'uvwxyz'], 3), null, 'too far apart to say step by step');
});

test('a word put in is marked as added, and nothing else is', () => {
  assert.equal(said(both('Roll back by the tag.', 'Roll back by the release tag.')), 'Roll back by the [+release ]tag.');
});

test('a word taken out is marked as removed, where it stood', () => {
  assert.equal(said(both('Roll back by the old tag.', 'Roll back by the tag.')), 'Roll back by the [-old ]tag.');
});

test('a word changed is taken out and the new one put in after it', () => {
  assert.equal(said(both('It is the linker.', 'It is the compiler.')), 'It is the [-linker][+compiler].');
});

test('whole lines put in and taken out are marked whole, and the lines between are the same', () => {
  const older = 'First line.\nSecond line.\nThird line.\n';
  const newer = 'First line.\nThird line.\nA fourth.\n';
  assert.equal(said(both(older, newer)), 'First line.\n[-Second line.\n]Third line.\n[+A fourth.\n]');
  assert.deepEqual(linesOf('a\nb'), ['a\n', 'b']);
  assert.deepEqual(linesOf('a\n\n'), ['a\n', '\n']);
});

test('a line rewritten in every word is taken out whole, then put in whole', () => {
  assert.equal(said(both('Alpha beta.\n', 'Gamma delta.\n')), '[-Alpha beta.\n][+Gamma delta.\n]');
});

test('nothing becoming something is all added, and something becoming nothing all removed', () => {
  assert.equal(said(both('', 'New words.')), '[+New words.]');
  assert.equal(said(both('Old words.', '')), '[-Old words.]');
  assert.deepEqual(both('', ''), []);
  assert.deepEqual(both('Same.', 'Same.'), [{ kind: 'same', text: 'Same.' }]);
});

test('words of any script, and marks made of several characters, are told apart whole', () => {
  assert.equal(said(both('Café crème', 'Café noir')), 'Café [-crème][+noir]');
  assert.equal(said(both('Привет мир', 'Привет друг')), 'Привет [-мир][+друг]');
  assert.equal(said(both('Done 👍🏽 today', 'Done 👎🏽 today')), 'Done [-👍🏽][+👎🏽] today');
  assert.ok(wordsOf('naïve façade').includes('naïve'), 'a word with its accents is one word');
  both('日本語のテキスト', '日本語の文章');
});

test('a long document with one change is said quickly, and only that change is marked', () => {
  const lines = Array.from({ length: 4000 }, (_, at) => `Line ${at} of the runbook says what to do.\n`);
  const changed = [...lines];
  changed[2500] = 'Line 2500 of the runbook says what not to do.\n';
  const pieces = both(lines.join(''), changed.join(''));
  assert.equal(pieces.filter((piece) => piece.kind !== 'same').map((piece) => piece.text).join('|'), 'not ');
});

test('how many words were put in and taken out', () => {
  assert.deepEqual(counted(diff('It is the linker.', 'It is the new compiler, really.')), { added: 4, removed: 1 });
  assert.deepEqual(counted(diff('', '')), { added: 0, removed: 0 });
});

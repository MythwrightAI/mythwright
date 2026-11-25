import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import { ACROSS, ACROSS_SMALL, fitted, says, tile, TILE_H, TILE_H_SAID, tileHeight, wrapped } from './tile.ts';
import type { Room } from './tile.ts';

const node = (name: string, target: string, description: string | null = null, kind: 'action' | 'call' | 'spawn' | 'trigger' = 'action') => ({ name, target, description, kind });

test('a step that says nothing of itself is drawn as it always was: its key, and what it runs under it', () => {
  assert.deepEqual(tile(node('build', 'shell')), { words: ['build'], under: 'shell', said: false });
  assert.deepEqual(tile(node('fleet', 'std/fleet', null, 'call')), { words: ['fleet'], under: 'call std/fleet', said: false });
  assert.deepEqual(tile(node('lanes', 'std/working', '', 'spawn')), { words: ['lanes'], under: 'spawn std/working', said: false });
  assert.deepEqual(tile(node('moved', 'board.item.moved', null, 'trigger')), { words: ['when'], under: 'board.item.moved', said: false });
  assert.equal(tile(node('a-step-with-a-very-long-key-indeed', 'noop')).words[0], 'a-step-with-a-very-long…');
});

test('a step that says what it is for has that as its words, and its key second with what it runs', () => {
  const said = tile(node('fleet', 'std/fleet', 'The fleet itself. Called and not spawned, so that applying this is done when the fleet is as its tables say.', 'call'));
  assert.deepEqual(said, { words: ['The fleet itself.'], under: 'fleet · call std/fleet', said: true });
  const long = tile(node('room', 'device.room', 'Look before leaping; say so if this device is short on room.'));
  assert.deepEqual(long.words, ['Look before leaping; say so', 'if this device is short on…']);
  assert.equal(long.under, 'room · device.room');
  assert.equal(tile(node('heard', 'board.item.moved', 'A card came to this column.', 'trigger')).under, 'when · board.item.moved');
});

test('nothing a tile says is wider than the tile', () => {
  const descriptions = ['Supercalifragilisticexpialidocious-and-then-some is one word', 'a b c d e f g h i j k l m n o p q r s t u v w x y z a b c d e f g h i j k l m n o p q r s t u v w x y z', 'Short.'];
  for (const description of descriptions) {
    for (const taken of [0, 9, 30]) {
      const said = tile(node('a-key-as-long-as-a-key-can-reasonably-be', 'std/a-blueprint-with-a-long-name', description, 'call'), taken);
      assert.ok(said.words.length <= 2, 'two lines of words at most');
      assert.ok(said.words.every((line) => line.length <= ACROSS), `${JSON.stringify(said.words)} fits`);
      assert.ok(said.under.length <= ACROSS_SMALL);
    }
  }
  // What stands at the corner of the first line takes room from it, and the words go round it.
  assert.deepEqual(tile(node('room', 'device.room', 'Look before leaping; say so if this device is short on room.'), 10).words, ['Look before', 'leaping; say so if this de…']);
});

test('where letters are drawn, a line is as wide as its letters are and no wider than its tile', () => {
  // Wide letters and narrow ones: counted alike, a line of the first runs past the edge.
  const drawn: Room = { words: 100, under: 100, wide: (text, small) => [...text].reduce((sum, letter) => sum + (small ? 4 : 'mwMW'.includes(letter) ? 9 : 'il.,; '.includes(letter) ? 2 : 5), 0) };
  const wide = tile(node('warm', 'std/warm', 'Warm welcome committee members swimming somewhere warmer.', 'call'), 0, drawn);
  const narrow = tile(node('list', 'std/list', 'It is still. Little will tilt it.', 'call'), 0, drawn);
  for (const one of [wide, narrow]) assert.ok(one.words.every((line) => drawn.wide(line, false) <= 100), JSON.stringify(one.words));
  assert.deepEqual(wide.words, ['Warm welcome', 'committee members…']);
  assert.deepEqual(narrow.words, ['It is still.']);
  assert.ok(drawn.wide(wide.under, true) <= 100);
  assert.equal(fitted('abcdefghij', 5), 'abcd…');
  assert.equal(fitted('abc', 5), 'abc');
  assert.equal(fitted('abc def', 5), 'abc…', 'no room is left before what says it was cut');
});

test('what a step says of itself is said as far as its first full stop', () => {
  assert.equal(says('Who is in the fleet. A row is so many agents of one role.'), 'Who is in the fleet.');
  assert.equal(says('Said once.'), 'Said once.');
  assert.equal(says('Keeps v1.2 of the thing running'), 'Keeps v1.2 of the thing running');
  assert.equal(says('  Laid out\n  over two lines. And more.'), 'Laid out over two lines.');
  assert.equal(says(null), '');
  assert.equal(says('   '), '');
});

test('words are broken between words, and the last line says there is more', () => {
  assert.deepEqual(wrapped('one two three', [20, 20]), ['one two three']);
  assert.deepEqual(wrapped('one two three four five six', [8, 9]), ['one two', 'three fo…']);
  assert.deepEqual(wrapped('', [10, 10]), []);
});

test('the tiles of a graph are all as tall as each other, and taller only where a step says something of itself', () => {
  assert.equal(tileHeight([node('a', 'noop'), node('b', 'noop', '')]), TILE_H);
  assert.equal(tileHeight([node('a', 'noop'), node('b', 'noop', 'Does nothing, on purpose.')]), TILE_H_SAID);
  assert.equal(tileHeight([]), TILE_H);
});

test('what a step says of itself is cut by one rule, here and where the daemon says a blueprint in words', () => {
  // The table the daemon's own rule is held to (`blueprint::summary::says`): a case added there is a case here.
  const cases = JSON.parse(readFileSync(new URL('./says.json', import.meta.url), 'utf8')) as [string, string][];
  assert.ok(cases.length >= 12, 'the table was found');
  for (const [description, said] of cases) assert.equal(says(description), said, JSON.stringify(description));
});

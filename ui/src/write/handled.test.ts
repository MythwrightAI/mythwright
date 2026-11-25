import assert from 'node:assert/strict';
import { test } from 'node:test';
import { blockAt, commentOn, kindOf, landing, lineAt, linesOf, moved, turnable, turning } from './handled.ts';

// Three blocks down the page, with room between them as a page has.
const spans = [
  { top: 100, bottom: 140 },
  { top: 160, bottom: 180 },
  { top: 200, bottom: 260 },
];

test('the block the pointer is at is the one it is over', () => {
  assert.equal(blockAt(spans, 100), 0);
  assert.equal(blockAt(spans, 170), 1);
  assert.equal(blockAt(spans, 260), 2);
});

test('between two blocks the pointer is at the nearer, so that the handle stays as it crosses', () => {
  assert.equal(blockAt(spans, 145), 0);
  assert.equal(blockAt(spans, 150), 0);
  assert.equal(blockAt(spans, 155), 1);
  assert.equal(blockAt(spans, 195), 2);
});

test('well above the first block and below the last the pointer is at none', () => {
  assert.equal(blockAt(spans, 95), 0);
  assert.equal(blockAt(spans, 80), null);
  assert.equal(blockAt(spans, 266), 2);
  assert.equal(blockAt(spans, 300), null);
  assert.equal(blockAt([], 100), null);
});

test('a carried block lands before the block whose middle is under the pointer, or after the last', () => {
  assert.equal(landing(spans, 90), 0);
  assert.equal(landing(spans, 119), 0);
  assert.equal(landing(spans, 121), 1);
  assert.equal(landing(spans, 190), 2);
  assert.equal(landing(spans, 231), 3);
  assert.equal(landing([], 10), 0);
});

test('a block let go just before or just after itself stays where it is', () => {
  assert.equal(moved(1, 1), null);
  assert.equal(moved(1, 2), null);
});

test('a block let go further down is counted without itself, and one further up as it is', () => {
  assert.equal(moved(0, 3), 2);
  assert.equal(moved(0, 2), 1);
  assert.equal(moved(2, 0), 0);
  assert.equal(moved(2, 1), 1);
});

test('the line that says where a block lands is between the two it falls between', () => {
  assert.equal(lineAt(spans, 0), 98);
  assert.equal(lineAt(spans, 1), 150);
  assert.equal(lineAt(spans, 2), 190);
  assert.equal(lineAt(spans, 3), 262);
});

test('a block is named as the menu names it, and what it does not name is nothing', () => {
  assert.equal(kindOf({ type: 'paragraph' }), 'text');
  assert.equal(kindOf({ type: 'heading', level: 2 }), 'h2');
  assert.equal(kindOf({ type: 'heading', level: 4 }), null);
  assert.equal(kindOf({ type: 'bullet_list' }), 'bullets');
  assert.equal(kindOf({ type: 'ordered_list' }), 'numbers');
  assert.equal(kindOf({ type: 'blockquote' }), 'quote');
  assert.equal(kindOf({ type: 'code_block' }), 'code');
  assert.equal(kindOf({ type: 'horizontal_rule' }), 'divider');
  assert.equal(kindOf({ type: 'image' }), null);
});

test('turned into what holds one line, each line is a block of its own and none is lost', () => {
  assert.deepEqual(turning('h1', 3), [
    { kind: 'h1', lines: [0] },
    { kind: 'h1', lines: [1] },
    { kind: 'h1', lines: [2] },
  ]);
  assert.deepEqual(turning('text', 1), [{ kind: 'text', lines: [0] }]);
});

test('turned into what holds several lines, it holds them all', () => {
  assert.deepEqual(turning('bullets', 2), [{ kind: 'bullets', lines: [0, 1] }]);
  assert.deepEqual(turning('code', 3), [{ kind: 'code', lines: [0, 1, 2] }]);
  assert.deepEqual(turning('quote', 1), [{ kind: 'quote', lines: [0] }]);
});

test('a block with no line yet is turned as one empty line', () => {
  assert.deepEqual(turning('h2', 0), [{ kind: 'h2', lines: [0] }]);
});

test('what holds words may be turned into any block that holds words, and a divider into nothing', () => {
  assert.deepEqual(
    turnable('text').map((block) => block.id),
    ['text', 'h1', 'h2', 'h3', 'bullets', 'numbers', 'quote', 'code'],
  );
  assert.deepEqual(turnable('divider'), []);
  assert.equal(turnable(null).length, 8);
});

test('the menu of a block marks what it is, and offers only what does something', () => {
  const lines = linesOf({ kind: 'h2', at: 0, of: 3, comments: false });
  assert.deepEqual(lines[0], { said: 'Turn into' });
  assert.deepEqual(
    lines.filter((line) => 'turn' in line && line.chosen).map((line) => ('turn' in line ? line.turn : '')),
    ['h2'],
  );
  const acts = lines.flatMap((line) => ('act' in line ? [line.label] : []));
  assert.deepEqual(acts, ['Duplicate', 'Move down', 'Delete']);
  assert.equal(lines.find((line) => 'act' in line && line.act === 'duplicate' && line.apart) !== undefined, true);
});

test('the last block cannot be moved down and the only one moved at all', () => {
  const last = linesOf({ kind: 'text', at: 2, of: 3, comments: true }).flatMap((line) => ('act' in line ? [line.label] : []));
  assert.deepEqual(last, ['Duplicate', 'Move up', 'Comment', 'Delete']);
  const only = linesOf({ kind: 'text', at: 0, of: 1, comments: false }).flatMap((line) => ('act' in line ? [line.label] : []));
  assert.deepEqual(only, ['Duplicate', 'Delete']);
});

test('a divider is offered nothing to turn into, and its first line is set apart by nothing', () => {
  const lines = linesOf({ kind: 'divider', at: 1, of: 3, comments: false });
  assert.equal(lines.some((line) => 'said' in line || 'turn' in line), false);
  assert.equal(lines.some((line) => 'apart' in line && line.apart), false);
});

test('a comment on a block begins by naming it by its first words, after what was being written', () => {
  assert.equal(commentOn('Riverside  Shop', ''), 'On “Riverside Shop”: ');
  assert.equal(commentOn('a'.repeat(80), ''), `On “${'a'.repeat(57)}…”: `);
  assert.equal(commentOn('Plan', 'Earlier words.  '), 'Earlier words.\nOn “Plan”: ');
  assert.equal(commentOn('   ', 'Kept'), 'Kept\n');
  assert.equal(commentOn('', ''), '');
});

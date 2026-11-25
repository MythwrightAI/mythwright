import assert from 'node:assert/strict';
import { test } from 'node:test';
import { cardsIn, read, spans } from './prose.ts';
import type { Span } from './prose.ts';

/** Spans written short, to be read: `text`, or [kind, inside]. */
const short = (all: Span[]): unknown[] =>
  all.map((one) => (one.is === 'text' ? one.text : one.is === 'code' ? ['code', one.text] : one.is === 'link' ? ['link', one.to, ...short(one.inside)] : one.is === 'image' ? ['image', one.to] : one.is === 'cite' ? ['cite', one.label] : one.is === 'page' ? ['page', one.topic, one.reply, ...short(one.inside)] : [one.is, ...short(one.inside)]));

test('the marks the tools write are read as what they mean', () => {
  assert.deepEqual(short(spans('a **b** _c_ ++d++ ~~e~~ `f` g')), ['a ', ['bold', 'b'], ' ', ['italic', 'c'], ' ', ['underline', 'd'], ' ', ['strike', 'e'], ' ', ['code', 'f'], ' g']);
});

test('marks are read inside marks, but not inside code', () => {
  assert.deepEqual(short(spans('**all _of_ it**')), [['bold', 'all ', ['italic', 'of'], ' it']]);
  assert.deepEqual(short(spans('`**as written**`')), [['code', '**as written**']]);
});

test('an agent writing italics with stars is read too', () => {
  assert.deepEqual(short(spans('this is *important* now')), ['this is ', ['italic', 'important'], ' now']);
});

test('what only looks like a mark is left as it was written', () => {
  for (const plain of ['2 * 3 * 4', 'snake_case_name and _private_field', 'the __init__ method', 'c++ and g++', 'about ~5 or ~~ 6', '** spaced **']) {
    assert.deepEqual(short(spans(plain)), [plain]);
  }
});

test('a link is its words and where they lead, and only to where a link may lead', () => {
  assert.deepEqual(short(spans('see [the docs](https://example.com/a) now')), ['see ', ['link', 'https://example.com/a', 'the docs'], ' now']);
  assert.deepEqual(short(spans('[write](mailto:ada@example.com)')), [['link', 'mailto:ada@example.com', 'write']]);
  assert.deepEqual(short(spans('[run](javascript:alert(1))')), ['[run](javascript:alert(1))']);
});

test('an address written out is a link, without the full stop after it', () => {
  assert.deepEqual(short(spans('it is at https://example.com/x?y=1. Go.')), ['it is at ', ['link', 'https://example.com/x?y=1', 'https://example.com/x?y=1'], '. Go.']);
  assert.deepEqual(short(spans('(https://example.com)')), ['(', ['link', 'https://example.com', 'https://example.com'], ')']);
});

test('a message of plain lines is one block, with its lines as they were', () => {
  const blocks = read('one\ntwo\n\nthree');
  assert.equal(blocks.length, 1);
  assert.deepEqual(blocks[0], { is: 'text', spans: [{ is: 'text', text: 'one\ntwo\n\nthree' }] });
});

test('lines that begin with a dash or a number are a list, further in when written further in', () => {
  const [before, list, after] = read('Three things:\n- one\n- two\n  1. inside\n\nand then');
  assert.equal(before!.is, 'text');
  assert.ok(list && list.is === 'list');
  assert.deepEqual(
    list.items.map((item) => [item.depth, item.mark, short(item.spans)]),
    [
      [0, '•', ['one']],
      [0, '•', ['two']],
      [1, '1.', ['inside']],
    ],
  );
  assert.deepEqual(after, { is: 'text', spans: [{ is: 'text', text: 'and then' }] });
});

test('a paragraph written under an item is that item going on, drawn in it, and the list goes on after it', () => {
  const [list, after] = read('- ask the worker\n\n    it works the app board,\n    so check with it first.\n\n- or wait\n\nand then');
  assert.ok(list && list.is === 'list');
  assert.deepEqual(
    list.items.map((item) => short(item.spans)),
    [['ask the worker\n\nit works the app board,\nso check with it first.'], ['or wait']],
  );
  assert.deepEqual(after, { is: 'text', spans: [{ is: 'text', text: 'and then' }] });
  const [numbered] = read('1. first\n\n   its next paragraph');
  assert.ok(numbered && numbered.is === 'list');
  assert.deepEqual(short(numbered.items[0]!.spans), ['first\n\nits next paragraph']);
});

test('a numbered item keeps the number it was given', () => {
  const [list] = read('3. third\n4) fourth');
  assert.ok(list && list.is === 'list');
  assert.deepEqual(
    list.items.map((item) => item.mark),
    ['3.', '4.'],
  );
});

test('lines that begin with > are one quotation', () => {
  assert.deepEqual(read('> first\n> **second**\nand the answer'), [
    { is: 'quote', spans: [{ is: 'text', text: 'first\n' }, { is: 'bold', inside: [{ is: 'text', text: 'second' }] }] },
    { is: 'text', spans: [{ is: 'text', text: 'and the answer' }] },
  ]);
});

test('one to three # and a space make a heading; a channel named in passing does not', () => {
  assert.deepEqual(read('## Plan'), [{ is: 'heading', level: 2, spans: [{ is: 'text', text: 'Plan' }] }]);
  assert.equal(read('#general is where')[0]!.is, 'text');
  assert.equal(read('#### too deep')[0]!.is, 'text');
});

test('a block of code is kept as written, whatever is in it', () => {
  assert.deepEqual(read('run this:\n```sh\n- not a list\n**not bold**\n```\nthen say so'), [
    { is: 'text', spans: [{ is: 'text', text: 'run this:' }] },
    { is: 'code', text: '- not a list\n**not bold**', lang: 'sh' },
    { is: 'text', spans: [{ is: 'text', text: 'then say so' }] },
  ]);
  assert.deepEqual(read('```one line```'), [{ is: 'code', text: 'one line' }]);
});

test('lines further in after an empty line are a block of code, and an item further in is still an item', () => {
  assert.deepEqual(read('Run:\n\n    cargo build\n    cargo test\n\nthen say so'), [
    { is: 'text', spans: [{ is: 'text', text: 'Run:' }] },
    { is: 'code', text: 'cargo build\ncargo test' },
    { is: 'text', spans: [{ is: 'text', text: 'then say so' }] },
  ]);
  const [list] = read('- one\n    - inside');
  assert.ok(list && list.is === 'list' && list.items[1]!.depth === 2);
});

test('a card is named by its id, which is its project and a number', () => {
  assert.deepEqual(cardsIn('see forge-1, then shop-12.', ['shop', 'forge']), [
    'see ',
    { card: 'forge-1', project: 'forge' },
    ', then ',
    { card: 'shop-12', project: 'shop' },
    '.',
  ]);
});

test('what only looks like a card is left as it was written', () => {
  for (const plain of ['utf-8 and glen-4', 'on 2026-11-10', 'workshop-3', 'docs/shop-3.md', 'shop-3-final', 'shop-']) {
    assert.deepEqual(cardsIn(plain, ['shop']), [plain]);
  }
  assert.deepEqual(cardsIn('shop-3', []), ['shop-3'], 'with no projects there are no cards');
});

test('a card of a project whose name begins like another is that project\'s', () => {
  assert.deepEqual(cardsIn('shop-front-3', ['shop', 'shop-front']), [{ card: 'shop-front-3', project: 'shop-front' }]);
});

test('a table is its headings, where each column stands and its rows, each cell read as a line is', () => {
  const [before, table, after] = read('Found:\n| Practice | Minutes | Studies |\n|:--|:-:|--:|\n| [Breath](topic:@-topics-4) | 10 | 12[^1] |\n| **Scan** \\| lying | 20 |\n\nAfter.');
  assert.deepEqual(before, { is: 'text', spans: [{ is: 'text', text: 'Found:' }] });
  assert.ok(table && table.is === 'table');
  assert.deepEqual(table.align, ['left', 'center', 'right']);
  assert.deepEqual(table.head.map(short), [['Practice'], ['Minutes'], ['Studies']]);
  assert.deepEqual(
    table.rows.map((row) => row.map(short)),
    [
      [[['page', '@-topics-4', null, 'Breath']], ['10'], ['12', ['cite', '1']]],
      [[['bold', 'Scan'], ' | lying'], ['20'], []],
    ],
  );
  assert.deepEqual(after, { is: 'text', spans: [{ is: 'text', text: 'After.' }] });
});

test('a table needs its line of dashes, as many cells as its headings, and ends at a line that is no row', () => {
  const cases: [string, string[]][] = [
    ['| a | b |\n|---|---|\n| 1 | 2 |', ['table']],
    ['a | b\n--|--\n1 | 2\nwords after', ['table', 'text']],
    ['| a | b |\n| 1 | 2 |', ['text']],
    ['| a | b |\n|---|\n| 1 | 2 |', ['text']],
    ['a line with a | in it', ['text']],
    ['---\n| a |\n', ['text']],
    ['| single |\n|---|\n| one |', ['table']],
    ['| a | b |\n|-x-|---|', ['text']],
    ['```\n| a | b |\n|---|---|\n```', ['code']],
    ['- item\n  | a | b |\n  |---|---|', ['list']],
  ];
  for (const [text, kinds] of cases) assert.deepEqual(read(text).map((block) => block.is), kinds, text);
});

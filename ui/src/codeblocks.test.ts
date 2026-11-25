import assert from 'node:assert/strict';
import { test } from 'node:test';
import { copySaid, copyWord, drawerOf, partsOf, undrawnSaid } from './codeblocks.ts';

const code = (text: string, lang?: string) => ({ is: 'code', text, ...(lang ? { lang } : {}) });
const words = (text: string) => ({ is: 'words', text });

test('words with no block of code in them are one part, letter for letter, and no words are none', () => {
  for (const plain of ['one line', 'two\nlines\n\nand more  ', '  begins with two spaces', 'an `inline` piece of code', '']) {
    assert.deepEqual(partsOf(plain), plain === '' ? [] : [words(plain)]);
  }
});

test('a fenced block holds what is between its marks, and not what it is written in or the line break before its end', () => {
  assert.deepEqual(partsOf('run this:\n```sh\ncargo build\ncargo test\n```\nthen say so'), [words('run this:'), code('cargo build\ncargo test', 'sh'), words('then say so')]);
  assert.deepEqual(partsOf('```one line```'), [code('one line')]);
  assert.deepEqual(partsOf('```\n```'), [code('')]);
});

test('what a block holds is as it was written: its tabs, its spaces, its empty lines, and no line break added', () => {
  const text = '\tindented by a tab\n    by four\n\n  trailing  ';
  assert.deepEqual(partsOf('```\n' + text + '\n```'), [code(text)]);
  assert.deepEqual(partsOf('```\n' + text + '\n\n```'), [code(text + '\n')]);
});

test('a fenced block that is never closed holds the rest', () => {
  assert.deepEqual(partsOf('look:\n```\nleft open\nto the end'), [words('look:'), code('left open\nto the end')]);
});

test('lines four spaces or a tab further in, after an empty line, are a block of what is past that', () => {
  assert.deepEqual(partsOf('Steps:\n\n    cargo build\n    \tcargo test\n\nand then'), [words('Steps:'), code('cargo build\n\tcargo test'), words('and then')]);
  assert.deepEqual(partsOf('\tone tab in\n\ttwo'), [code('one tab in\ntwo')]);
  assert.deepEqual(partsOf('    at the start'), [code('at the start')]);
});

test('an indented block goes on through empty lines between its lines, and ends at its last line further in', () => {
  assert.deepEqual(partsOf('so:\n\n    first\n\n      second, further\n\n\nafter'), [words('so:'), code('first\n\n  second, further'), words('after')]);
});

test('a line further in that goes on from the line above, or is an item of a list, is no block of code', () => {
  for (const plain of ['a sentence that\n    goes on further in', 'Things:\n- one\n\n    - inside', 'Numbered:\n\n    1. first', '  two spaces in\n\n  are not four']) {
    assert.deepEqual(partsOf(plain), [words(plain)]);
  }
});

test('a line under an item of a list, after an empty line, is the item going on and no block of code', () => {
  for (const plain of [
    'Two ways:\n\n- ask the worker\n\n    the helper works the app board, so check with it first.\n\n- or wait',
    '1. first\n\n   its next paragraph\n\n2. second',
    '- outer\n  - inner\n\n    more of the inner\n\n  more of the outer',
    '10. tenth\n\n    four in is where its words begin',
  ]) {
    assert.deepEqual(partsOf(plain), [words(plain)]);
  }
});

test('a block of code inside an item of a list is four further in than where the words of the item begin', () => {
  assert.deepEqual(partsOf('- build it:\n\n      cargo build\n      \tcargo test\n\n  and then it is done'), [
    words('- build it:'),
    code('cargo build\n\tcargo test'),
    words('  and then it is done'),
  ]);
  assert.deepEqual(partsOf('1. run:\n\n       make check\n\n2. ship'), [words('1. run:'), code('make check'), words('2. ship')]);
});

test('a block of code after a list that has ended is four in, as anywhere', () => {
  assert.deepEqual(partsOf('- one\n- two\n\nThen run:\n\n    cargo build\n\nand say so'), [words('- one\n- two\n\nThen run:'), code('cargo build'), words('and say so')]);
  assert.deepEqual(partsOf('- one\n\nnot in the list\n\n    code'), [words('- one\n\nnot in the list'), code('code')]);
});

test('blocks of both kinds in one text, each its own part, in order', () => {
  assert.deepEqual(partsOf('a\n```\nb\n```\nc\n\n    d\ne'), [words('a'), code('b'), words('c'), code('d'), words('e')]);
  assert.deepEqual(partsOf('```\none\n```\n```\ntwo\n```'), [code('one'), code('two')]);
});

test('the button says Copy, then what it did, and for whoever hears it a sentence only once it has done something', () => {
  assert.equal(copyWord('idle'), 'Copy');
  assert.equal(copyWord('copied'), 'Copied');
  assert.equal(copyWord('failed'), 'Failed');
  assert.equal(copySaid('idle'), '');
  assert.match(copySaid('copied'), /copied/);
  assert.match(copySaid('failed'), /select it and copy it/);
});

test('what a fenced block is written in is kept beside what it holds, and a block that says nothing has none', () => {
  assert.deepEqual(partsOf('```mermaid\ngraph TD; A-->B\n```'), [code('graph TD; A-->B', 'mermaid')]);
  assert.deepEqual(partsOf('```Mermaid\nA\n```\nthen\n```\nB\n```'), [code('A', 'mermaid'), words('then'), code('B')]);
  assert.deepEqual(partsOf('one\n```\nB\n```\n```ts\nC\n```'), [words('one'), code('B'), code('C', 'ts')]);
});

test('a block is drawn by what is set up for its language, and by nothing for one that says none or that nothing draws', () => {
  const drawers = { mermaid: 'the diagram' };
  assert.equal(drawerOf('mermaid', drawers), 'the diagram');
  for (const lang of [undefined, '', 'sh', 'constructor', '__proto__', 'tostring', 'hasownproperty']) assert.equal(drawerOf(lang, drawers), null, String(lang));
});

test('a diagram nothing draws says so under its code, and one something draws, or a block that is no diagram, says nothing', () => {
  assert.equal(undrawnSaid('mermaid', []), 'Diagram drawing is not set up in this version.');
  assert.equal(undrawnSaid('mermaid', ['mermaid']), null);
  for (const lang of [undefined, 'sh', 'constructor']) assert.equal(undrawnSaid(lang, []), null);
});

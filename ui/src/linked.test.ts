import assert from 'node:assert/strict';
import { test } from 'node:test';
import { supportLink } from './help.ts';
import { addressesIn, piecesOf, shortOf } from './linked.ts';

/** The pieces as `text` for words and `[address]` for an address, to read a case at a glance. */
const read = (words: string) => piecesOf(words).map((piece) => (piece.href ? `[${piece.href}]` : piece.text));

test('an address on the web in words is a piece that leads there, and the rest is words', () => {
  assert.deepEqual(read('The preview of origin/dev is ready to try at http://127.0.0.1:7760/'), ['The preview of origin/dev is ready to try at ', '[http://127.0.0.1:7760/]']);
  assert.deepEqual(read('https://example.invalid/a?b=c#d is up'), ['[https://example.invalid/a?b=c#d]', ' is up']);
  assert.deepEqual(read('two: http://a.invalid and https://b.invalid/x'), ['two: ', '[http://a.invalid]', ' and ', '[https://b.invalid/x]']);
  assert.deepEqual(read('Written LOUD: HTTPS://EXAMPLE.INVALID/X'), ['Written LOUD: ', '[HTTPS://EXAMPLE.INVALID/X]']);
});

test('what a sentence puts after an address is not part of it', () => {
  assert.deepEqual(read('Try it at http://127.0.0.1:7760/.'), ['Try it at ', '[http://127.0.0.1:7760/]', '.']);
  assert.deepEqual(read('See http://a.invalid/x, then http://b.invalid/y; done!'), ['See ', '[http://a.invalid/x]', ', then ', '[http://b.invalid/y]', '; done!']);
  assert.deepEqual(read('(as said at http://a.invalid/x)'), ['(as said at ', '[http://a.invalid/x]', ')']);
  assert.deepEqual(read('Is it at http://a.invalid/x?'), ['Is it at ', '[http://a.invalid/x]', '?']);
  assert.deepEqual(read('[http://a.invalid/x].'), ['[', '[http://a.invalid/x]', '].']);
  assert.deepEqual(read('"http://a.invalid/x" and <http://b.invalid/y>'), ['"', '[http://a.invalid/x]', '" and <', '[http://b.invalid/y]', '>']);
  // A bracket the address opened is the address's.
  assert.deepEqual(read('http://a.invalid/wiki/Foo_(bar).'), ['[http://a.invalid/wiki/Foo_(bar)]', '.']);
  assert.deepEqual(read('(http://a.invalid/wiki/Foo_(bar))'), ['(', '[http://a.invalid/wiki/Foo_(bar)]', ')']);
  // Over several lines, an address ends with its line.
  assert.deepEqual(read('at http://a.invalid/x\nnext'), ['at ', '[http://a.invalid/x]', '\nnext']);
});

test('nothing but an address on the web leads anywhere', () => {
  for (const words of [
    'javascript:alert(1)',
    'javascript://example.invalid/%0aalert(1)',
    'data:text/html,<b>x</b>',
    'file:///etc/passwd',
    'mailto:help@example.invalid',
    'ftp://example.invalid/x',
    'example.invalid/x',
    'www.example.invalid',
    'https://',
    'http:// spaced',
    'https:///nothing-after',
    'xhttp://example.invalid/x',
    'ask in the channel',
  ])
    assert.deepEqual(read(words), [words], words);
  // A script written right after an address does not ride along with it.
  assert.deepEqual(read('http://a.invalid/x"onclick="y'), ['[http://a.invalid/x]', '"onclick="y']);
  assert.deepEqual(read('http://a.invalid/<script>'), ['[http://a.invalid/]', '<script>']);
});

test('every address that leads somewhere is one the support line would open', () => {
  for (const words of ['at http://127.0.0.1:7760/.', 'https://a.invalid/x?y=(z)', 'see https://b.invalid, and', 'HTTP://C.INVALID'])
    for (const address of addressesIn(words)) assert.equal(supportLink(address), address, address);
});

test('the pieces put together are the words, letter for letter', () => {
  for (const words of ['', 'nothing here', 'at http://a.invalid/x. And (http://b.invalid/y)!', 'http://a.invalid http://a.invalid', ' \n http://a.invalid/x\n '])
    assert.equal(
      piecesOf(words)
        .map((piece) => piece.text)
        .join(''),
      words,
      words,
    );
  assert.deepEqual(piecesOf(''), []);
});

test('the addresses are each said once, in the order they are first written', () => {
  assert.deepEqual(addressesIn('http://b.invalid then http://a.invalid then http://b.invalid.'), ['http://b.invalid', 'http://a.invalid']);
  assert.deepEqual(addressesIn('no address'), []);
});

test('a link standing by itself is called where it leads', () => {
  assert.equal(shortOf('http://127.0.0.1:7760/'), '127.0.0.1:7760');
  assert.equal(shortOf('https://example.invalid'), 'example.invalid');
  assert.equal(shortOf('https://example.invalid/a/b?c=d'), 'example.invalid/a/b?c=d');
  assert.equal(shortOf('HTTPS://EXAMPLE.INVALID/'), 'EXAMPLE.INVALID');
});

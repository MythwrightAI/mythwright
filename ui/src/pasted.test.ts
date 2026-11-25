import assert from 'node:assert/strict';
import { test } from 'node:test';
import { dataOf, drawnAgain, filesOf, pasteOf, pasteSaid, pastedName, picturesIn } from './pasted.ts';

const png = (name = 'image.png', size = 120) => ({ name, type: 'image/png', size });
const nothing = { files: [], items: [], html: '', text: '' };

test('a picture listed both as a file and as an item is taken once', () => {
  const file = png();
  const again = png();
  assert.deepEqual(filesOf({ files: [file], items: [{ kind: 'file', file: again }] }), [file]);
});

test('a picture listed only as an item is taken, and an item that is text is not', () => {
  const file = png();
  assert.deepEqual(filesOf({ files: [], items: [{ kind: 'string', file: null }, { kind: 'file', file }] }), [file]);
});

test('an item of kind file that gives no file is passed over', () => {
  assert.deepEqual(filesOf({ files: [], items: [{ kind: 'file', file: null }] }), []);
});

test('two files alike in both lists are two, and a third only an item has is taken too', () => {
  const [a, b, c] = [png('a.png'), png('a.png'), png('a.png')];
  const other = { name: 'notes.txt', type: 'text/plain', size: 18 };
  assert.deepEqual(filesOf({ files: [a, b], items: [{ kind: 'file', file: c }, { kind: 'file', file: png('a.png') }, { kind: 'file', file: other }] }), [a, b, other]);
});

test('the pictures of some HTML are read in order, however the address is quoted', () => {
  assert.deepEqual(picturesIn(`<p>x</p><img alt="a" src="data:image/png;base64,AAAA"><IMG src='https://example.test/a.png?x=1&amp;y=2'><img src=b.gif><img alt="none">`), [
    'data:image/png;base64,AAAA',
    'https://example.test/a.png?x=1&y=2',
    'b.gif',
  ]);
  assert.deepEqual(picturesIn('<p>no pictures</p>'), []);
});

test('a file taken lets no text in, even with the HTML and the address of where it was copied from', () => {
  const file = png();
  const said = pasteOf({ files: [file], items: [], html: '<img src="https://example.test/a.png">', text: 'https://example.test/a.png' });
  assert.deepEqual(said, { take: 'files', files: [file] });
  assert.equal(pasteSaid(said), null);
});

test('HTML that holds its picture in itself is decoded, and lets no text in', () => {
  const said = pasteOf({ ...nothing, html: '<meta charset="utf-8"><img src="data:image/png;base64,iVBORw0KGgo=">', text: '' });
  assert.deepEqual(said, { take: 'inline', sources: ['data:image/png;base64,iVBORw0KGgo='] });
});

test('a data address that is no picture is not one to attach', () => {
  assert.deepEqual(pasteOf({ ...nothing, html: '<img src="data:text/html;base64,PGI+">', text: 'x' }), { take: 'text', remote: true });
});

test('a picture somewhere else lets the text in and says the picture itself was not there', () => {
  const said = pasteOf({ ...nothing, html: '<img src="https://example.test/a.png">', text: '' });
  assert.deepEqual(said, { take: 'text', remote: true });
  assert.match(pasteSaid(said)!, /not on the clipboard.*Copy image/);
});

test('text alone is the browser\'s to put in, saying nothing', () => {
  const said = pasteOf({ ...nothing, html: '<b>hi</b>', text: 'hi' });
  assert.deepEqual(said, { take: 'text', remote: false });
  assert.equal(pasteSaid(said), null);
});

test('a paste that brings nothing at all says so', () => {
  const said = pasteOf(nothing);
  assert.deepEqual(said, { take: 'nothing' });
  assert.match(pasteSaid(said)!, /^Nothing came with that paste/);
});

test('a data address is read as its kind and its bytes, written either way', () => {
  assert.deepEqual(dataOf('data:image/png;base64,AQID'), { type: 'image/png', bytes: new Uint8Array([1, 2, 3]) });
  assert.deepEqual(dataOf('data:image/svg+xml;charset=utf-8,%3Csvg%2F%3E'), { type: 'image/svg+xml', bytes: new TextEncoder().encode('<svg/>') });
  assert.equal(dataOf('data:text/plain;base64,AQID'), null);
  assert.equal(dataOf('data:image/png;base64,!!!'), null);
  assert.equal(dataOf('https://example.test/a.png'), null);
});

test('a picture the client does not show is drawn again, and one it shows is not', () => {
  for (const type of ['image/bmp', 'image/tiff', 'image/x-ms-bmp', 'image/svg+xml', 'IMAGE/BMP']) assert.equal(drawnAgain(type), true, type);
  for (const type of ['image/png', 'image/jpeg', 'image/gif', 'image/webp', 'image/avif', 'text/plain', '']) assert.equal(drawnAgain(type), false, type);
});

test('a pasted picture with no name of its own is called after when it was pasted', () => {
  const at = new Date(2026, 10, 21, 9, 5, 7);
  assert.equal(pastedName('image.png', 'image/png', at), 'pasted-20261121-090507.png');
  assert.equal(pastedName('', 'image/jpeg', at), 'pasted-20261121-090507.jpg');
  assert.equal(pastedName('blob', 'image/png', at), 'pasted-20261121-090507.png');
  assert.equal(pastedName('Image.JPG', 'image/jpeg', at), 'pasted-20261121-090507.JPG');
});

test('a name of its own is kept, ending as what it now is', () => {
  const at = new Date(2026, 10, 21, 9, 5, 7);
  assert.equal(pastedName('board.png', 'image/png', at), 'board.png');
  assert.equal(pastedName('photo.jpeg', 'image/jpeg', at), 'photo.jpeg');
  assert.equal(pastedName('scan.tif', 'image/tiff', at), 'scan.tif');
  // Drawn again as a PNG.
  assert.equal(pastedName('board.bmp', 'image/png', at), 'board.png');
  assert.equal(pastedName('image.bmp', 'image/png', at), 'pasted-20261121-090507.png');
  assert.equal(pastedName('notes.txt', 'text/plain', at), 'notes.txt');
  assert.equal(pastedName('Makefile', '', at), 'Makefile');
});

test('two of one paste that would be called the same are numbered', () => {
  const at = new Date(2026, 10, 21, 9, 5, 7);
  assert.equal(pastedName('image.png', 'image/png', at, ['pasted-20261121-090507.png']), 'pasted-20261121-090507-2.png');
  assert.equal(pastedName('a.png', 'image/png', at, ['a.png', 'a-2.png']), 'a-3.png');
});

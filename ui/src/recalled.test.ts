import assert from 'node:assert/strict';
import { test } from 'node:test';
import { going, recall, sentHere } from './recalled.ts';
import type { Box, Recalling } from './recalled.ts';

const ME = 'user:acc_1';
const said = [
  { author: ME, body: 'first' },
  { author: 'agent:builder', body: 'an answer' },
  { author: ME, body: 'second', thread: null },
  { author: ME, body: 'under it', thread: 'm1' },
  { author: ME, body: '' },
  { author: ME, body: 'second' },
  { author: 'user:acc_2', body: 'somebody else' },
  { author: ME, body: 'third\nof two lines' },
];
const sent = ['first', 'second', 'third\nof two lines'];
const plain = { offered: false, phone: false };
const empty: Box = { text: '', from: 0, to: 0 };
const at = (text: string, caret = text.length): Box => ({ text, from: caret, to: caret });

test('what the reader sent here is theirs alone, oldest first, in the channel or the thread it was said in', () => {
  assert.deepEqual(sentHere(said, ME), sent, 'nothing said under a message, nothing empty, nobody else, and one line for words sent twice running');
  assert.deepEqual(sentHere(said, ME, 'm1'), ['under it']);
  assert.deepEqual(sentHere(said, 'user:acc_2'), ['somebody else']);
  assert.deepEqual(sentHere(undefined, ME), [], 'nothing read yet');
  assert.deepEqual(sentHere(said, undefined), [], 'nobody signed in');
  assert.deepEqual(sentHere([{ author: ME, body: 'a' }, { author: ME, body: 'b' }, { author: ME, body: 'a' }], ME), ['a', 'b', 'a'], 'the same words again later are another line');
});

test('Up in an empty box puts back the last thing sent, and again goes further back, staying at the first', () => {
  const one = recall('ArrowUp', empty, sent, null, plain);
  assert.deepEqual(one, { text: 'third\nof two lines', recalling: { back: 1, draft: '', shown: 'third\nof two lines' } });
  // The caret is at its end, on its last line: Up is the box's until the caret is on the first.
  assert.equal(recall('ArrowUp', at(one!.text), sent, one!.recalling, plain), null);
  const two = recall('ArrowUp', at(one!.text, 3), sent, one!.recalling, plain);
  assert.deepEqual(two, { text: 'second', recalling: { back: 2, draft: '', shown: 'second' } });
  const three = recall('ArrowUp', at('second'), sent, two!.recalling, plain);
  assert.deepEqual(three, { text: 'first', recalling: { back: 3, draft: '', shown: 'first' } });
  const still = recall('ArrowUp', at('first'), sent, three!.recalling, plain);
  assert.deepEqual(still, three, 'there is nothing before the first');
});

test('Down goes forward, and past the newest gives back what was being written, or an empty box', () => {
  const back: Recalling = { back: 3, draft: 'half wri', shown: 'first' };
  const two = recall('ArrowDown', at('first'), sent, back, plain);
  assert.deepEqual(two, { text: 'second', recalling: { back: 2, draft: 'half wri', shown: 'second' } });
  const one = recall('ArrowDown', at('second'), sent, two!.recalling, plain);
  assert.equal(one?.text, 'third\nof two lines');
  // On the first of its two lines Down is the box's: the caret goes to the last.
  assert.equal(recall('ArrowDown', at(one!.text, 2), sent, one!.recalling, plain), null);
  assert.deepEqual(recall('ArrowDown', at(one!.text), sent, one!.recalling, plain), { text: 'half wri', recalling: null });
  assert.deepEqual(recall('ArrowDown', at('third\nof two lines'), sent, { back: 1, draft: '', shown: 'third\nof two lines' }, plain), { text: '', recalling: null });
});

test('what was half written is kept, and Up begins only with nothing written before the caret', () => {
  assert.equal(recall('ArrowUp', at('half wri'), sent, null, plain), null, 'the caret at its end: Up is the box\'s, which takes it to the start');
  assert.deepEqual(recall('ArrowUp', at('half wri', 0), sent, null, plain), { text: 'third\nof two lines', recalling: { back: 1, draft: 'half wri', shown: 'third\nof two lines' } });
});

test('a message of several lines being written keeps its arrows, but on the first line at its start', () => {
  const written = 'one line\nanother';
  assert.equal(recall('ArrowUp', at(written), sent, null, plain), null, 'the caret on the second line moves up');
  assert.equal(recall('ArrowUp', at(written, 4), sent, null, plain), null, 'on the first line with words before it');
  assert.equal(recall('ArrowUp', at(written, 0), sent, null, plain)?.text, 'third\nof two lines');
  assert.equal(recall('ArrowDown', at(written, 0), sent, null, plain), null, 'Down is nothing until one goes back');
});

test('a recalled text that was changed is the reader\'s: they are going back no more, and Up begins again keeping it', () => {
  const was: Recalling = { back: 2, draft: '', shown: 'second' };
  assert.equal(going(was, 'second'), was);
  assert.equal(going(was, 'second!'), null);
  assert.equal(recall('ArrowDown', at('second!'), sent, was, plain), null, 'Down is the box\'s again');
  assert.equal(recall('ArrowUp', at('second!'), sent, was, plain), null, 'Up from its end is the box\'s');
  assert.deepEqual(recall('ArrowUp', at('second!', 0), sent, was, plain), { text: 'third\nof two lines', recalling: { back: 1, draft: 'second!', shown: 'third\nof two lines' } });
});

test('nothing is recalled while a list is offered, on a phone, with words selected, or where nothing was sent', () => {
  assert.equal(recall('ArrowUp', empty, sent, null, { offered: true, phone: false }), null, 'the arrows are the list\'s');
  assert.equal(recall('ArrowUp', empty, sent, null, { offered: false, phone: true }), null);
  assert.equal(recall('ArrowDown', at('second'), sent, { back: 2, draft: '', shown: 'second' }, { offered: true, phone: false }), null);
  assert.equal(recall('ArrowUp', { text: 'some words', from: 0, to: 4 }, sent, null, plain), null);
  assert.equal(recall('ArrowUp', empty, [], null, plain), null);
  assert.equal(recall('ArrowLeft', empty, sent, null, plain), null, 'only Up and Down');
});

test('a history that grew shorter while one went back through it is held to what is there', () => {
  assert.deepEqual(recall('ArrowUp', at('first', 0), ['only'], { back: 3, draft: '', shown: 'first' }, plain), { text: 'only', recalling: { back: 1, draft: '', shown: 'only' } });
  assert.deepEqual(recall('ArrowDown', at('first'), ['only', 'two'], { back: 3, draft: 'x', shown: 'first' }, plain), { text: 'only', recalling: { back: 2, draft: 'x', shown: 'only' } });
});

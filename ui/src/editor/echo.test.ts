import assert from 'node:assert/strict';
import { test } from 'node:test';
import { isEcho, say } from './echo.ts';

test('what the editor said itself is not put back into it, however late it comes back', () => {
  const said: string[] = [];
  say(said, 'a');
  say(said, 'ab');
  say(said, 'abc');
  // The first two come back after the third was typed: neither may undo it.
  assert.equal(isEcho(said, 'a'), true);
  assert.equal(isEcho(said, 'ab'), true);
  assert.equal(isEcho(said, 'abc'), true);
  assert.deepEqual(said, []);
});

test('what it never said is someone else\'s, and is put in', () => {
  const said: string[] = [];
  say(said, 'draft');
  assert.equal(isEcho(said, 'the file as it now is on disk'), false);
  assert.deepEqual(said, [], 'and what it said before no longer counts');
  assert.equal(isEcho(said, 'draft'), false);
});

test('one that comes back skips the ones before it, which will not', () => {
  const said: string[] = [];
  for (const text of ['a', 'ab', 'abc']) say(said, text);
  assert.equal(isEcho(said, 'abc'), true);
  assert.equal(isEcho(said, 'ab'), false, 'by then it is somebody putting old text back on purpose');
});

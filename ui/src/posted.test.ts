import assert from 'node:assert/strict';
import { test } from 'node:test';
import type { Attachment, Message } from './api.ts';
import { chipOf, foldersOf, mayPlace, pathFor, placedIn } from './posted.ts';

const file = (state?: 'there' | 'gone' | 'kept'): Attachment => ({
  id: 'att_1',
  name: 'distribution.md',
  media: 'text/markdown',
  size: state === 'there' ? 2048 : 0,
  file: { project: 'shop', path: 'docs/distribution.md', ...(state ? { state } : {}) },
});

test('a file of a project is a chip that opens while it is there, says where it went, and says nothing of one kept from its reader', () => {
  assert.equal(chipOf({ id: 'att_2', name: 'copy.md', media: 'text/markdown', size: 3 }), null, 'a copy is no such chip');
  assert.deepEqual(chipOf(file('there')), { said: 'distribution.md', note: 'shop/docs/distribution.md · 2.0 kB', opens: true });
  assert.deepEqual(chipOf(file('gone')), { said: 'distribution.md', note: 'shop/docs/distribution.md · no longer there', opens: false });
  assert.deepEqual(chipOf(file('kept')), { said: 'distribution.md', note: 'a file of a project that is not yours to read', opens: false });
  assert.deepEqual(chipOf(file()), { said: 'distribution.md', note: 'shop/docs/distribution.md', opens: false }, 'not yet read for this reader: named, and opened once it is');
});

test('a copy is offered to be added to a project by whoever may write in the channel and write a project\'s files', () => {
  const open = { members: [] };
  const writes = { actor: 'user:mira', scopes: ['channels', 'projects'], owner: false };
  assert.equal(mayPlace({ channel: open, by: writes }), true);
  assert.equal(mayPlace({ channel: open, by: { ...writes, scopes: ['channels', 'projects:read'] } }), false, 'reading a project is not writing its files');
  assert.equal(mayPlace({ channel: open, by: { ...writes, scopes: ['channels:read', 'projects'] } }), false, 'nor is reading the channel writing in it');
  assert.equal(mayPlace({ channel: { ...open, archived: { at: 1 } }, by: writes }), false, 'nothing is changed in an archived channel');
  assert.equal(mayPlace({ channel: open, by: null }), false);
});

test('the folders offered are the libraries\', the top and every folder, a library\'s folder once', () => {
  const entry = (path: string, dir = true) => ({ path, name: path.split('/').pop() ?? path, dir, size: 0, modified: 0 });
  const tree = [entry('src'), entry('docs'), entry('docs/guides'), entry('README.md', false)];
  assert.deepEqual(
    foldersOf(tree, [{ name: 'docs', at: 'docs/' }]).map((one) => [one.path, one.said]),
    [
      ['docs', 'docs (library)'],
      ['', 'The top of the project'],
      ['docs/guides', 'docs/guides/'],
      ['src', 'src/'],
    ],
  );
  assert.deepEqual(foldersOf([], []).map((one) => one.path), [''], 'a project with nothing in it has its top');
});

test('a copy is written under its name in the folder chosen, and a name is one file\'s', () => {
  assert.equal(pathFor('docs', 'brief.md'), 'docs/brief.md');
  assert.equal(pathFor('', ' brief.md '), 'brief.md');
  for (const name of ['', '  ', 'a/b.md', 'a\\b.md', '.', '..']) assert.equal(pathFor('docs', name), null, name);
});

test('a message whose copy was added to a project names that file, and nothing else of it changes', () => {
  const copy: Attachment = { id: 'att_9', name: 'brief.md', media: 'text/markdown', size: 8 };
  const other: Attachment = { id: 'att_8', name: 'other.png', media: 'image/png', size: 9 };
  const message = { id: 'msg_1', channel: 'ch', author: 'user:ada', author_name: 'Ada', body: 'The brief.', reply_to: null, thread: null, attachments: [other, copy], created: 1 } as unknown as Message;
  const placed = placedIn(message, 'att_9', 'shop', 'docs/brief-2.md');
  assert.deepEqual(placed.attachments?.[1], { id: 'att_9', name: 'brief-2.md', media: 'text/markdown', size: 0, file: { project: 'shop', path: 'docs/brief-2.md' } });
  assert.deepEqual(placed.attachments?.[0], other);
  assert.equal(placed.body, 'The brief.');
  assert.equal(placedIn(message, 'att_7', 'shop', 'x.md'), message, 'a message that does not carry it is as it was');
});

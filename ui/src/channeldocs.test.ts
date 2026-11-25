import assert from 'node:assert/strict';
import { test } from 'node:test';
import { lineOf, mayChange, offered } from './channeldocs.ts';
import type { ChannelDoc } from './channeldocs.ts';

const entry = (path: string, dir = false) => ({ path, name: path.split('/').pop() ?? path, dir, size: 1, modified: 0 });

test('a line says what an entry is called and where it is, or what has become of it, and never leaves one out', () => {
  const base = { by: 'user:ada', added: 1 };
  const there: ChannelDoc = { ...base, doc: 'shop-docs:guides:install', kind: 'record', project: 'shop', path: 'docs/guides/install.md', title: 'Installing', state: 'there' };
  assert.deepEqual(lineOf(there), { said: 'Installing', where: 'shop-docs:guides:install' });
  const file: ChannelDoc = { ...base, doc: 'shop/README.md', kind: 'file', project: 'shop', path: 'README.md', title: 'README.md', state: 'there' };
  assert.deepEqual(lineOf(file), { said: 'README.md', where: 'shop/README.md' });
  assert.deepEqual(lineOf({ ...file, state: 'gone', title: undefined }), { said: 'Gone', where: 'shop/README.md' });
  assert.deepEqual(lineOf({ ...there, state: 'kept', title: undefined, path: undefined }), { said: 'Not yours to read', where: 'shop-docs:guides:install' }, 'nothing of what is kept from the reader but what the entry says');
});

test('the list is changed by whoever may write in the channel', () => {
  const ada = { actor: 'user:ada', scopes: ['*'], owner: true };
  const mira = { actor: 'user:mira', scopes: ['channels', 'records'], owner: false };
  const reads = { actor: 'user:mira', scopes: ['channels:read'], owner: false };
  const open = { members: [] };
  const kept = { kept: true, members: [{ actor: 'user:otto' }] };
  assert.equal(mayChange(open, mira), true);
  assert.equal(mayChange(open, reads), false, 'leave to read is not leave to write');
  assert.equal(mayChange(kept, mira), false, 'a private channel is its members\'');
  assert.equal(mayChange({ ...kept, members: [{ actor: 'user:mira' }] }, mira), true);
  assert.equal(mayChange(kept, ada), true, 'and the owner\'s, as the daemon has it');
  assert.equal(mayChange({ ...open, archived: { at: 1 } }, ada), false, 'nothing is added to an archived channel');
});

test('what may be added is each project\'s library documents and its other files, less what is kept already, narrowed by what is typed', () => {
  const projects = [
    { id: 'shop', name: 'Shop' },
    { id: 'mill', name: 'Mill' },
  ];
  const trees = {
    shop: [entry('docs', true), entry('docs/overview.md'), entry('docs/guides/install.md'), entry('README.md'), entry('src/till.rs')],
    mill: [entry('notes.md')],
  };
  const libraries = { shop: [{ name: 'docs', store: 'documents' }, { name: 'board', store: 'local' }] };
  const all = offered(projects, trees, libraries, ['shop/README.md']);
  assert.deepEqual(
    all.map((one) => [one.project, one.documents.map((offer) => offer.doc), one.files.map((offer) => offer.doc)]),
    [
      ['shop', ['shop-docs:guides:install', 'shop-docs:overview'], ['shop/src/till.rs']],
      ['mill', [], ['mill/notes.md']],
    ],
  );
  assert.equal(all[0]?.documents[1]?.title, 'overview');
  const typed = offered(projects, trees, libraries, [], 'mill');
  assert.deepEqual(typed.map((one) => one.project), ['mill'], 'every word typed, in what an entry is called, where it is or its project');
  assert.deepEqual(offered(projects, trees, libraries, [], 'install').flatMap((one) => one.documents.map((offer) => offer.doc)), ['shop-docs:guides:install']);
  assert.deepEqual(offered(projects, {}, libraries, []), [], 'a project whose files were not read offers nothing');
});

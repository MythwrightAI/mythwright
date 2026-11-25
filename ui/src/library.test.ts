import assert from 'node:assert/strict';
import { test } from 'node:test';
import { changes, documentAt, fileOf, folderOf, isLibrary, pagesOf, pathOf, placeFor, shelves, titleOf } from './library.ts';
import type { Row } from './table.ts';

const row = (id: string, data: Record<string, unknown> = {}): Row => ({ id, number: 0, data, position: 0, created: 0, updated: 0 });
const docs = { name: 'docs', store: 'documents', at: 'docs' };

test('a library is a collection whose records are documents, and no other is', () => {
  assert.equal(isLibrary(docs), true);
  assert.equal(isLibrary({ name: 'board', store: 'local' }), false);
  assert.equal(isLibrary({ name: 'notes', store: 'files' }), false);
});

test('a document is a file in the folder its library is of', () => {
  const install = row('shop-docs:guides:install', { path: 'guides/install.md' });
  assert.equal(fileOf(docs, install), 'docs/guides/install.md');
  assert.equal(fileOf({ name: 'docs', store: 'documents', at: 'notes/design' }, install), 'notes/design/guides/install.md');
  // The folder is the one called what the library is, where none is said; `.` is the project itself.
  assert.equal(folderOf({ name: 'handbook', store: 'documents' }), 'handbook');
  assert.equal(fileOf({ name: 'docs', store: 'documents', at: '.' }, install), 'guides/install.md');
});

test('where a document is can be read off what it is known by', () => {
  assert.equal(pathOf(row('shop-docs:guides:deep:tagged')), 'guides/deep/tagged.md');
  assert.equal(pathOf(row('riverside-shop-docs:README')), 'README.md');
  assert.equal(pathOf(row('shop-docs:a', { path: 'said/by/the/record.md' })), 'said/by/the/record.md');
});

test('a document is called its title, or its file where it has none', () => {
  assert.equal(titleOf(row('shop-docs:notes', { title: 'Rough notes', path: 'notes.md' })), 'Rough notes');
  assert.equal(titleOf(row('shop-docs:guides:notes', { title: ' ', path: 'guides/notes.md' })), 'notes');
});

test('the documents are set out by folder, the library first and each folder by name', () => {
  const rows = [
    row('shop-docs:guides:install', { title: 'Installing it', path: 'guides/install.md' }),
    row('shop-docs:welcome', { title: 'Welcome', path: 'welcome.md' }),
    row('shop-docs:api:events', { title: 'Events', path: 'api/events.md' }),
    row('shop-docs:about', { title: 'About', path: 'about.md' }),
    row('shop-docs:guides:first', { title: 'First steps', path: 'guides/first.md' }),
  ];
  const set = shelves(rows).map((shelf) => [shelf.folder, shelf.rows.map(titleOf)]);
  assert.deepEqual(set, [
    ['', ['About', 'Welcome']],
    ['api', ['Events']],
    ['guides', ['First steps', 'Installing it']],
  ]);
  assert.deepEqual(shelves([]), []);
});

test('a library is read as pages at an address of its own', () => {
  assert.equal(pagesOf('shop', 'docs'), '/library/shop/docs/');
  assert.equal(pagesOf('riverside shop', 'docs'), '/library/riverside%20shop/docs/');
});

test('a library is read again when one of its documents or a file under its folder changed, and not for anything else', () => {
  const said = (topic: string, data: Record<string, unknown>) => changes({ topic, data }, 'shop', docs);
  assert.equal(said('record.created', { project: 'shop', collection: 'docs', id: 'shop-docs:a' }), true);
  assert.equal(said('record.updated', { project: 'shop', collection: 'sprints' }), false);
  assert.equal(said('record.removed', { project: 'depot', collection: 'docs' }), false);
  assert.equal(said('project.file.changed', { project: 'shop', path: 'docs/guides/new.md' }), true);
  assert.equal(said('project.file.changed', { project: 'shop', path: 'docs/picture.png' }), false);
  assert.equal(said('project.file.changed', { project: 'shop', path: 'src/docs/readme.md' }), false);
  assert.equal(said('project.file.changed', { project: 'depot', path: 'docs/new.md' }), false);
  assert.equal(said('project.rescanned', { project: 'shop', changed: 400 }), true);
  assert.equal(said('collection.ensured', { project: 'shop', name: 'docs' }), true);
  assert.equal(said('collection.ensured', { project: 'shop', name: 'board' }), false);
  // A library of the project itself hears of any document in it.
  assert.equal(changes({ topic: 'project.file.changed', data: { project: 'shop', path: 'README.md' } }, 'shop', { ...docs, at: '.' }), true);
});

test('a new document goes where its title and the folder said for it say', () => {
  assert.equal(placeFor('Getting Started', ''), 'getting-started');
  assert.equal(placeFor('  Install it!  ', 'Guides/First Steps'), 'guides/first-steps/install-it');
  assert.equal(placeFor('Install', '/guides/'), 'guides/install');
  assert.equal(placeFor('  ', 'guides'), null);
});

test('a file is the document of the library whose folder it is in, known by where it is there', () => {
  const board = { name: 'board', store: 'local' };
  // In a folder of the library, and at its top.
  assert.deepEqual(documentAt([board, docs], 'shop', 'docs/guides/install.md'), { library: docs, record: 'shop-docs:guides:install' });
  assert.deepEqual(documentAt([board, docs], 'shop', 'docs/welcome.md'), { library: docs, record: 'shop-docs:welcome' });
  // And it is the record its row says: the file of that record is this file.
  const found = documentAt([docs], 'shop', 'docs/guides/deep/tagged.md');
  assert.equal(fileOf(docs, row(found!.record)), 'docs/guides/deep/tagged.md');
});

test('a file beside a library, or one that is not Markdown, is no document', () => {
  // Beside the library and not of it: at the top of the project, and in a folder that only begins as the library's does.
  assert.equal(documentAt([docs], 'shop', 'README.md'), null);
  assert.equal(documentAt([docs], 'shop', 'docs-old/welcome.md'), null);
  assert.equal(documentAt([docs], 'shop', 'src/docs/welcome.md'), null);
  // In the library's folder and no document: not Markdown, a name that could not be written in an id, no name at all.
  assert.equal(documentAt([docs], 'shop', 'docs/logo.png'), null);
  assert.equal(documentAt([docs], 'shop', 'docs/a:b.md'), null);
  assert.equal(documentAt([docs], 'shop', 'docs/.md'), null);
  assert.equal(documentAt([docs], 'shop', 'docs/guides//install.md'), null);
  // A table that is no library holds no documents, whatever it is called; and a project with no library has none.
  assert.equal(documentAt([{ name: 'docs', store: 'local' }], 'shop', 'docs/welcome.md'), null);
  assert.equal(documentAt([], 'shop', 'docs/welcome.md'), null);
});

test('of two libraries of one project a file is the document of the one whose folder holds it', () => {
  const notes = { name: 'notes', store: 'documents', at: 'notes/design' };
  assert.deepEqual(documentAt([docs, notes], 'shop', 'notes/design/login.md'), { library: notes, record: 'shop-notes:login' });
  assert.deepEqual(documentAt([docs, notes], 'shop', 'docs/login.md'), { library: docs, record: 'shop-docs:login' });
  assert.equal(documentAt([docs, notes], 'shop', 'notes/login.md'), null, 'above the folder the second is of, and in neither');
  // One library's folder inside another's: the file is the first one's that holds it, in the order they are listed, as the daemon says whose it is.
  const all = { name: 'all', store: 'documents', at: '.' };
  assert.deepEqual(documentAt([all, docs], 'shop', 'docs/welcome.md'), { library: all, record: 'shop-all:docs:welcome' });
  assert.deepEqual(documentAt([docs, all], 'shop', 'docs/welcome.md'), { library: docs, record: 'shop-docs:welcome' });
});

test('a library that is the project itself holds every Markdown file of the project', () => {
  const whole = { name: 'pages', store: 'documents', at: '.' };
  assert.deepEqual(documentAt([whole], 'riverside-shop', 'README.md'), { library: whole, record: 'riverside-shop-pages:README' });
  assert.deepEqual(documentAt([whole], 'riverside-shop', 'src/notes/todo.md'), { library: whole, record: 'riverside-shop-pages:src:notes:todo' });
  assert.equal(documentAt([whole], 'riverside-shop', 'src/main.rs'), null);
  // One whose folder is not said is of the folder called what it is.
  assert.deepEqual(documentAt([{ name: 'handbook', store: 'documents' }], 'shop', 'handbook/day-one.md'), { library: { name: 'handbook', store: 'documents' }, record: 'shop-handbook:day-one' });
});

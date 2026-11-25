import assert from 'node:assert/strict';
import { test } from 'node:test';
import { folderAsked, keptIn, keptLines, keptSaid, mayKeep } from './blueprinted.ts';

test('a project the daemon says nothing of keeps its own blueprints where the factory keeps them', () => {
  assert.deepEqual(keptIn(undefined), { said: 'usual', folder: null });
  assert.deepEqual(keptIn(null), { said: 'usual', folder: null });
  assert.deepEqual(keptIn({}), { said: 'usual', folder: null });
  assert.deepEqual(keptIn({ blueprints: { said: 'none', folder: null } }), { said: 'none', folder: null });
  // A daemon from before said the usual folder of the repository by its name: it is the usual, and no folder.
  assert.deepEqual(keptIn({ blueprints: { said: 'usual', folder: 'blueprints' } }), { said: 'usual', folder: null });
  assert.deepEqual(keptIn({ blueprints: { said: 'folder', folder: 'bp' } }), { said: 'folder', folder: 'bp' });
});

test('where a project keeps them is said in a few words', () => {
  assert.equal(keptSaid({ said: 'usual', folder: null }), 'Its own blueprints are kept by the factory');
  assert.equal(keptSaid({ said: 'folder', folder: 'tools/blueprints' }), 'Its own blueprints are under tools/blueprints/ of its repository, and kept by the factory');
  assert.equal(keptSaid({ said: 'none', folder: null }), 'It keeps no blueprints of its own');
});

test('changing it is for whoever may take a folder up', () => {
  assert.equal(mayKeep(['*']), true);
  assert.equal(mayKeep(['admin', 'projects']), true);
  assert.equal(mayKeep(['projects', 'blueprints']), false);
  assert.equal(mayKeep(['admin', 'projects:read']), false);
});

test('the gear offers the factory, a folder of the repository besides and none, with the one that is so chosen', () => {
  const lines = (kept: Parameters<typeof keptLines>[0]) => keptLines(kept).map((line) => `${line.chosen ? '* ' : ''}${line.label}`);
  assert.deepEqual(lines({ said: 'usual', folder: null }), ['* Kept by the factory', 'Also a repo folder…', 'None of its own']);
  // A folder said is a line of its own, chosen, beside the line that asks for another, and what it holds may be moved in.
  assert.deepEqual(lines({ said: 'folder', folder: 'bp' }), [
    'Kept by the factory',
    '* Also under bp/',
    'Also a repo folder…',
    'None of its own',
    'Move them in',
  ]);
  assert.deepEqual(lines({ said: 'none', folder: null }), ['Kept by the factory', 'Also a repo folder…', '* None of its own']);
  assert.deepEqual(
    keptLines({ said: 'folder', folder: 'bp' }).map((line) => line.asks),
    [{ usual: true }, { folder: 'bp' }, 'folder', { none: true }, { move_in: true }],
  );
});

test('a folder written in the dialog is sent without a slash at its end, the one that was usual among them', () => {
  assert.deepEqual(folderAsked(' tools/blueprints/ '), { folder: 'tools/blueprints' });
  assert.deepEqual(folderAsked('blueprints/'), { folder: 'blueprints' });
  // Not within the project: the daemon says why.
  assert.deepEqual(folderAsked('/etc'), { folder: '/etc' });
  assert.equal(folderAsked('  '), null);
  assert.equal(folderAsked('/'), null);
});

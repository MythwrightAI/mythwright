import assert from 'node:assert/strict';
import { test } from 'node:test';
import type { Entry } from './api.ts';
import { firstText, folderMeant, lineOf, named, offered, saidWhere } from './repomade.ts';

const at = (path: string, dir = false): Entry => ({ path, name: path.split('/').pop()!, dir, size: 0, modified: 0 });
const shop = [at('docs', true), at('docs/plan.md'), at('docs/old', true), at('README.md')];

test('a file and a folder are offered to whoever may write a project\'s files, and nothing to anybody else', () => {
  assert.deepEqual(offered(['projects:write']), ['file', 'folder']);
  assert.deepEqual(offered(['projects']), ['file', 'folder']);
  assert.deepEqual(offered(['*']), ['file', 'folder']);
  assert.deepEqual(offered(['projects:read', 'records']), []);
  assert.deepEqual(offered([]), []);
  assert.deepEqual(['file', 'folder'].map((made) => lineOf(made as 'file' | 'folder')), ['New file…', 'New folder…']);
});

test('a press means the folder of its line, or the top where none is said or what was said is no folder there', () => {
  assert.equal(folderMeant('docs/old', shop), 'docs/old');
  assert.equal(folderMeant('', shop), '');
  assert.equal(folderMeant(null, shop), '');
  assert.equal(folderMeant(undefined, shop), '');
  assert.equal(folderMeant('docs/plan.md', shop), '', 'a file is no folder');
  assert.equal(folderMeant('gone', shop), '', 'a folder taken away meanwhile');
});

test('a name comes to a path in the folder it was asked for in', () => {
  assert.deepEqual(named('notes.md', '', shop), { path: 'notes.md' });
  assert.deepEqual(named('  ideas.md ', 'docs', shop), { path: 'docs/ideas.md' }, 'what is about it is not part of it');
  assert.deepEqual(named('2026', 'docs/old', shop), { path: 'docs/old/2026' });
  assert.deepEqual(named('.env', '', shop), { path: '.env' });
});

test('a name that cannot be one is said why, before anything is asked', () => {
  const wrong = (name: string, folder = '') => {
    const came = named(name, folder, shop);
    assert.ok('wrong' in came, `${name} to be refused`);
    return came.wrong;
  };
  assert.equal(wrong(''), 'Give it a name.');
  assert.equal(wrong('   '), 'Give it a name.');
  assert.match(wrong('docs/ideas.md'), /^One name, without a slash/);
  assert.match(wrong('a\\b'), /^One name, without a slash/);
  assert.match(wrong('..'), /no name a file or a folder can have/);
  assert.match(wrong('.'), /no name a file or a folder can have/);
  assert.equal(wrong('draft.'), 'A name may not end in a dot.');
  assert.equal(wrong('.GIT'), "That is the repository's own folder.");
});

test('a name that is there already is refused, a file or a folder and whatever its case', () => {
  assert.deepEqual(named('plan.md', 'docs', shop), { wrong: 'There is a file called plan.md here already.' });
  assert.deepEqual(named('docs', '', shop), { wrong: 'There is a folder called docs here already.' });
  assert.deepEqual(named('readme.md', '', shop), { wrong: 'There is a file called README.md here already.' });
  // The same name in another folder is another file.
  assert.deepEqual(named('plan.md', '', shop), { path: 'plan.md' });
});

test('a Markdown file begins with a heading of its name, and any other file empty', () => {
  assert.equal(firstText('notes.md'), '# notes\n');
  assert.equal(firstText('docs/Opening hours.markdown'), '# Opening hours\n');
  assert.equal(firstText('docs/plan.MD'), '# plan\n');
  assert.equal(firstText('src/main.rs'), '');
  assert.equal(firstText('Makefile'), '');
});

test('where something made goes is said by its folder', () => {
  assert.equal(saidWhere(''), 'at the top of the repo');
  assert.equal(saidWhere('docs/old'), 'in docs/old/');
});

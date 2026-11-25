import assert from 'node:assert/strict';
import { test } from 'node:test';
import { isPicture, languageOfPath } from './language.ts';

test('a file is in the language its name says', () => {
  const told = (path: string) => languageOfPath(path);
  assert.equal(told('src/main.rs'), 'rust');
  assert.equal(told('ui/src/app.tsx'), 'tsx');
  assert.equal(told('scripts/build.mjs'), 'javascript');
  assert.equal(told('Cargo.toml'), 'toml');
  assert.equal(told('Cargo.lock'), 'toml');
  assert.equal(told('package.json'), 'json');
  assert.equal(told('docs/README.MD'), 'markdown');
  assert.equal(told('deploy/Dockerfile'), 'dockerfile');
  assert.equal(told('run.sh'), 'shell');
});

test('a file whose name says nothing is plain text', () => {
  assert.equal(languageOfPath('LICENSE'), 'text');
  assert.equal(languageOfPath('notes.unknown'), 'text');
  assert.equal(languageOfPath('.gitignore'), 'text');
});

test('pictures are known by their endings', () => {
  assert.equal(isPicture('art/logo.PNG'), true);
  assert.equal(isPicture('art/drawing.svg'), true);
  assert.equal(isPicture('art/notes.md'), false);
  assert.equal(isPicture('png'), false);
});

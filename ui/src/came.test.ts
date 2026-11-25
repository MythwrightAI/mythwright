import assert from 'node:assert/strict';
import { test } from 'node:test';
import { against, asked, DIFFERS, isBehind, mayUpdate, said } from './came.ts';

const sample = { name: 'sample', title: 'Sample' };

test('a package is behind only where the daemon says so', () => {
  assert.equal(isBehind(undefined), false);
  // A daemon from before says nothing of it.
  assert.equal(isBehind(sample), false);
  assert.equal(isBehind({ ...sample, behind: false }), false);
  assert.equal(isBehind({ ...sample, behind: true }), true);
});

test('taking the one that came is offered to whoever may install a package', () => {
  assert.equal(mayUpdate(undefined), false);
  assert.equal(mayUpdate([]), false);
  assert.equal(mayUpdate(['agents', 'extensions', 'blueprints']), false);
  assert.equal(mayUpdate(['agents:read', 'admin']), true);
  assert.equal(mayUpdate(['*']), true);
});

test('where the two differ is said only where the daemon said it', () => {
  assert.equal(against({ ...sample, behind: true }), '');
  assert.equal(against({ ...sample, behind: true, differs: { words: '' } }), '');
  assert.equal(
    against({ ...sample, behind: true, differs: { words: 'says `agent.wall.signin` besides' } }),
    'Against the installed copy, the one that came says `agent.wall.signin` besides.',
  );
});

test('nothing said of a package that differs calls either copy the newer', () => {
  const differing = { ...sample, behind: true, differs: { words: 'does not say `install`' } };
  const lines = [DIFFERS, ...said(differing, 'Product'), ...asked(differing, 'Product')];
  for (const line of lines) assert.ok(!/\b(newer|older|out of date|latest)\b/i.test(line), line);
  assert.equal(said(differing, 'Product').length, 3);
  assert.equal(said({ ...sample, behind: true }, 'Product').length, 2);
});

test('what is asked says what is replaced, what is lost and what is kept', () => {
  const lines = asked({ ...sample, behind: true, differs: { words: 'says `x` besides' } }, 'Product');
  assert.equal(lines.length, 3);
  const [replaced = '', where = '', lost = ''] = lines;
  assert.ok(replaced.includes('replaced by the one that came with this version of Product'));
  assert.ok(where.includes('says `x` besides'));
  assert.ok(lost.includes('Changes made to the installed copy are lost') && lost.includes('extension.was.yaml'));
});

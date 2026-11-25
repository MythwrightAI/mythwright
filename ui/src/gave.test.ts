import assert from 'node:assert/strict';
import { test } from 'node:test';
import { anyResult, brief, keptFrom, letGo, resultOf } from './gave.ts';

test('a step that gave nothing says nothing', () => {
  for (const nothing of [null, undefined, '', {}, []]) assert.equal(brief(nothing), '');
});

test('what a step gave is said in a line: each thing by its name, and what is inside another by how much there is', () => {
  assert.equal(brief({ instance: 'in_1m7qf', outputs: { address: '127.0.0.1:8080' } }), 'instance: in_1m7qf, outputs: address: 127.0.0.1:8080');
  assert.equal(brief({ code: 0, stdout: 'built\nlinked\n' }), 'code: 0, stdout: built …');
  assert.equal(brief({ rows: [{ id: 1 }, { id: 2 }], count: 2, wrong: null }), 'rows: 2 things, count: 2, wrong: nothing');
  assert.equal(brief({ made: { a: 1, b: 2 }, those: [], one: [{}] }), 'made: 2 things, those: none, one: 1 thing');
  assert.equal(brief('It is done.'), 'It is done.');
  assert.equal(brief(42), '42');
  assert.equal(brief(false), 'false');
  assert.equal(brief(['backlog', 'ready', 3]), 'backlog, ready, 3');
  assert.equal(brief([{ a: 1 }, { a: 2 }]), '2 things');
});

test('a long one is cut, and the whole is for the press', () => {
  const said = brief({ words: 'x'.repeat(400) });
  assert.equal(said.length, 140);
  assert.ok(said.endsWith('…'));
});

test('a step\'s line says why it failed before anything it gave, and nothing where it went well and gave nothing', () => {
  assert.deepEqual(resultOf({ error: 'the build failed\nat step 3', tolerated: false, output: { code: 1 } }), { says: 'the build failed', failed: true, whole: 'error' });
  assert.deepEqual(resultOf({ error: null, tolerated: false, output: { code: 0 } }), { says: 'code: 0', failed: false, whole: 'output' });
  assert.deepEqual(resultOf({ error: null, tolerated: true, output: null }), { says: 'failed, tolerated', failed: false, whole: null });
  assert.deepEqual(resultOf({ error: null, tolerated: true, output: { code: 2 } }), { says: 'failed, tolerated · code: 2', failed: false, whole: 'output' });
  assert.equal(resultOf({ error: null, tolerated: false, output: null }), null);
  assert.equal(resultOf({ error: null, tolerated: false, output: {} }), null);
});

test('a run none of whose steps has anything to say has no place for it', () => {
  assert.equal(anyResult([{ error: null, tolerated: false, output: null }, { error: null, tolerated: false, output: {} }]), false);
  assert.equal(anyResult([]), false);
  assert.equal(anyResult([{ error: null, tolerated: false, output: null }, { error: null, tolerated: false, output: { code: 0 } }]), true);
  assert.equal(anyResult([{ error: 'no', tolerated: false, output: null }]), true);
});

test('a run whose reader may not be told what set it off says so, and one read whole says nothing', () => {
  assert.equal(keptFrom({}), null);
  assert.equal(keptFrom({ kept_from: false }), null);
  const said = keptFrom({ kept_from: true });
  assert.ok(said?.startsWith('What set this run off is kept from you'));
  assert.ok(said?.includes('what they wrote'));
});

test('a run whose steps\' detail was let go says so in the daemon\'s words, and one whose detail is kept says nothing', () => {
  assert.equal(letGo({}), null);
  assert.equal(letGo({ let_go: 'what its steps printed was let go after a day' }), 'What its steps printed was let go after a day.');
});

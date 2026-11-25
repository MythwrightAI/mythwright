import assert from 'node:assert/strict';
import { test } from 'node:test';
import type { GraphParam } from './api.ts';
import { answered, applyingKey, applyingOf, forgetApplying, keepApplying, startOf, valueOf, written } from './applyasked.ts';

const param = (name: string, type: string, more: Partial<GraphParam> = {}): GraphParam => ({ name, type, default: null, required: false, description: null, choices: [], ...more });

test('what was written is there again for the same blueprint of the same project, and for no other', () => {
  const key = applyingKey('deploy', 'shop');
  assert.equal(applyingOf(key), null, 'nothing where nothing was written');
  keepApplying(key, { where: '', values: { version: 'v2' } });
  assert.equal(applyingOf(key)?.values.version, 'v2');
  assert.equal(applyingOf(applyingKey('deploy', 'blog')), null);
  assert.equal(applyingOf(applyingKey('deploy', null)), null);
  assert.equal(applyingOf(applyingKey('release', 'shop')), null);
  // Applied, it is let go of.
  keepApplying(key, null);
  assert.equal(applyingOf(key), null);
});

test('everything written is let go of when whoever was signed in no longer is', () => {
  keepApplying(applyingKey('deploy', 'shop'), { where: '', values: { version: 'v2' } });
  keepApplying(applyingKey('std/board', null), { where: 'shop', values: {} });
  forgetApplying();
  assert.equal(applyingOf(applyingKey('deploy', 'shop')), null);
  assert.equal(applyingOf(applyingKey('std/board', null)), null);
});

test('a box starts with what the param was last applied with, else what its blueprint says, else nothing', () => {
  const port = param('port', 'int', { default: 8080 });
  assert.equal(written(startOf(port, { port: 9090 })), '9090');
  assert.equal(written(startOf(port, {})), '8080');
  assert.equal(written(startOf(param('version', 'string'), {})), '');
  assert.equal(written(startOf(param('columns', 'list', { default: ['backlog', 'done'] }), {})), '["backlog","done"]');
  assert.equal(written(startOf(param('push', 'bool', { default: false }), {})), 'false');
});

test('what is written in a box is read by the param\'s type', () => {
  assert.equal(valueOf(param('version', 'string'), 'v2'), 'v2');
  assert.equal(valueOf(param('port', 'int'), '8080'), 8080);
  assert.throws(() => valueOf(param('port', 'int'), 'eighty'));
  assert.throws(() => valueOf(param('port', 'number'), ' '));
  assert.equal(valueOf(param('push', 'bool'), 'true'), true);
  assert.equal(valueOf(param('push', 'bool'), 'false'), false);
  assert.deepEqual(valueOf(param('columns', 'list'), '["a","b"]'), ['a', 'b']);
  assert.deepEqual(valueOf(param('anything', 'any'), '{"a":1}'), { a: 1 });
  assert.equal(valueOf(param('anything', 'any'), 'plain words'), 'plain words');
});

test('a blueprint is applied with what was written, a box left empty saying nothing unless it is needed', () => {
  const params = [param('version', 'string', { required: true }), param('port', 'int'), param('vars', 'map')];
  assert.deepEqual(answered(params, { version: 'v2', port: '', vars: '' }), { values: { version: 'v2' } });
  assert.deepEqual(answered(params, { version: '', port: '81' }), { values: { version: '', port: 81 } }, 'one that is needed is said as it was written');
  assert.deepEqual(answered(params, { version: 'v2', port: 'eighty' }), { wrong: '`port` should be a number.' });
  assert.deepEqual(answered(params, { version: 'v2', vars: '{a' }), { wrong: '`vars` should be a map written as JSON.' });
});

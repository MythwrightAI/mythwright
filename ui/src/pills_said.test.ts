import assert from 'node:assert/strict';
import { test } from 'node:test';
import { checkSays, instanceSays, instanceWord } from './pills_said.ts';

test('a text that checks says it can be applied', () => {
  assert.match(checkSays(0), /can be applied/);
});

test('a text with problems says how many and that it cannot be applied', () => {
  assert.match(checkSays(1), /^One thing is wrong/);
  assert.match(checkSays(3), /^3 things are wrong/);
  for (const many of [1, 3]) assert.match(checkSays(many), /cannot be applied until it checks/);
});

test('an active instance is called running, and every other status by its own word', () => {
  assert.equal(instanceWord('active'), 'running');
  for (const status of ['starting', 'done', 'failed', 'stopped']) assert.equal(instanceWord(status), status);
});

test('every status an instance can have says something of its own', () => {
  const said = ['starting', 'active', 'done', 'failed', 'stopped'].map(instanceSays);
  assert.equal(new Set(said).size, said.length);
  for (const one of said) assert.ok(one.endsWith('.') && !one.includes('undefined'), one);
});

test('a status nothing knows is said as it is', () => {
  assert.equal(instanceSays('paused'), 'It is paused.');
});

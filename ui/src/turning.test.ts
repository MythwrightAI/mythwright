import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import { NAMES, beginsAt } from './turning.ts';

test('the names that turn over are each said once, and short enough for the narrowest dialog', () => {
  assert.ok(NAMES.length >= 6);
  assert.equal(new Set(NAMES).size, NAMES.length);
  for (const name of NAMES) assert.ok(name.trim() === name && name.length >= 4 && name.length <= 30, name);
});

test('a showing begins at one of the names, whatever is thrown', () => {
  assert.equal(beginsAt(0), 0);
  assert.equal(beginsAt(0.5, 12), 6);
  assert.equal(beginsAt(0.999999, 12), 11);
  // A throw of one, which a browser never gives, is still a name.
  assert.equal(beginsAt(1, 12), 11);
  assert.equal(beginsAt(-0.1, 12), 0);
});

test('the names stand still where less movement is asked for', () => {
  const sheet = readFileSync(new URL('./turning.css', import.meta.url), 'utf8');
  const still = sheet.slice(sheet.indexOf('prefers-reduced-motion'));
  assert.ok(sheet.includes('prefers-reduced-motion') && /animation:\s*none/.test(still), 'the sheet says nothing of less movement');
});

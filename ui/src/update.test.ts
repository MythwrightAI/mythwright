import assert from 'node:assert/strict';
import { test } from 'node:test';
import { loadedFrom, verdict } from './update.ts';

const page = { build: 'one', live: false };
const dev = { build: 'one', live: true };

test('a page offers to reload when a different build is being served', () => {
  assert.equal(verdict(page, 'two', false), 'offer');
});

test('a page on the build being served has nothing to do', () => {
  assert.equal(verdict(page, 'one', false), 'current');
});

test('a development page reloads by itself, but not over unsaved edits', () => {
  assert.equal(verdict(dev, 'two', false), 'reload');
  assert.equal(verdict(dev, 'two', true), 'offer');
});

test('a page that does not know its own build never reloads on hearsay', () => {
  assert.equal(verdict({ build: null, live: true }, 'two', false), 'current');
});

test('an announcement that names no build changes nothing', () => {
  for (const served of [undefined, null, '', 7]) assert.equal(verdict(dev, served, false), 'current');
});

function entryPage(attributes: Record<string, string> | null) {
  return { querySelector: () => (attributes ? { getAttribute: (name: string) => attributes[name] ?? null } : null) };
}

test('a page reads its build from what the build wrote into it', () => {
  assert.deepEqual(loadedFrom(entryPage({ content: 'abc123', 'data-live': 'no' })), { build: 'abc123', live: false });
  assert.deepEqual(loadedFrom(entryPage({ content: 'abc123', 'data-live': 'yes' })), { build: 'abc123', live: true });
});

test('a page that did not come from a build has no build', () => {
  assert.equal(loadedFrom(entryPage(null)).build, null);
  assert.equal(loadedFrom(entryPage({ content: '%BUILD%', 'data-live': '%LIVE%' })).build, null);
});

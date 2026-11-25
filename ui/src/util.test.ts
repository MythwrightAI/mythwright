import assert from 'node:assert/strict';
import { test } from 'node:test';
import { agentState, amount, copyOf, found, isNamedBlueprint, reorder } from './util.ts';

const tabs = ['home', 'channel', 'board', 'activity'];

test('a tab is put down before the one it is dropped on', () => {
  assert.deepEqual(reorder(tabs, 'activity', 'channel'), ['home', 'activity', 'channel', 'board']);
  assert.deepEqual(reorder(tabs, 'home', 'activity'), ['channel', 'board', 'home', 'activity']);
});

test('dropped past the last tab, it goes to the end', () => {
  assert.deepEqual(reorder(tabs, 'home', null), ['channel', 'board', 'activity', 'home']);
});

test('dropped on itself, or not a tab at all, nothing moves', () => {
  assert.equal(reorder(tabs, 'board', 'board'), tabs);
  assert.equal(reorder(tabs, 'nonesuch', 'home'), tabs);
});

test('an agent is working, ready to be asked, or not running', () => {
  assert.equal(agentState(false, { active: true }), 'idle');
  assert.equal(agentState(true, { active: true }), 'working');
  assert.equal(agentState(false, { active: false }), 'stopped');
  assert.equal(agentState(true, undefined), 'stopped', 'one with no slot is not running, whatever was last heard of it');
});

test('a search says how much it found, and that there may be more where it found as many as it asked for', () => {
  assert.equal(found(0, 30), 'No results');
  assert.equal(found(1, 30), '1 result');
  assert.equal(found(12, 30), '12 results');
  assert.equal(found(30, 30), '30 or more results');
});

test('a file is a blueprint by any of the endings one may have, and never by yml alone', () => {
  const endings = ['bp', 'bp.yml', 'bp.yaml'];
  for (const path of ['activity.bp', 'activity.bp.yml', 'blueprints/std/slot.bp.yaml', 'a.b/release.bp']) {
    assert.equal(isNamedBlueprint(path, endings), true, path);
  }
  for (const path of ['compose.yml', '.bp', 'blueprints/.bp.yml', 'activity.bp.old', 'activity.yaml', 'bp']) {
    assert.equal(isNamedBlueprint(path, endings), false, path);
  }
  assert.equal(isNamedBlueprint('activity.bp', []), false, 'before the daemon has said what they are called, nothing is one');
});

test('an amount is written as what the daemon says it is of, whole once it is large', () => {
  assert.equal(amount(4.123, 'usd'), '$4.12');
  assert.equal(amount(0.5, 'usd'), '$0.50');
  assert.equal(amount(0.0421, 'usd'), '$0.0421');
  assert.equal(amount(0, 'usd'), '$0.00');
  assert.equal(amount(1_204_000, 'tokens'), '1,204,000');
  assert.equal(amount(100_333.3, 'tokens'), '100,333');
  assert.equal(amount(12.5, 'mb'), '12.50');
  assert.equal(amount(Number.NaN, 'usd'), '-');
});

test('a copy of a file goes beside it, called what it is called and that it is a copy, and ends as it did', () => {
  assert.equal(copyOf('docs/welcome.md', ['docs/welcome.md']), 'docs/welcome-copy.md');
  assert.equal(copyOf('notes.txt', []), 'notes-copy.txt');
  assert.equal(copyOf('docs/guides/how.to.md', []), 'docs/guides/how.to-copy.md');
});

test('a copy where there is one already is numbered, and a name with nothing after a dot is copied whole', () => {
  assert.equal(copyOf('docs/welcome.md', ['docs/welcome.md', 'docs/welcome-copy.md']), 'docs/welcome-copy-2.md');
  assert.equal(copyOf('docs/welcome.md', ['docs/welcome-copy.md', 'docs/welcome-copy-2.md']), 'docs/welcome-copy-3.md');
  assert.equal(copyOf('Makefile', []), 'Makefile-copy');
  assert.equal(copyOf('.env', []), '.env-copy');
});

import assert from 'node:assert/strict';
import { test } from 'node:test';
import { changesUse, listed } from './uses.ts';

test('a project lists its own blueprints first, each with the files it links, then what else is applied there, each by name', () => {
  const lines = listed({
    applied: ['std/working', 'fleet', 'std/board'],
    own: [
      { name: 'release', file: '@blueprints/release.bp', links: ['@blueprints/prompts/ask.md'] },
      { name: 'fleet', file: 'fleet.bp' },
    ],
  });
  assert.deepEqual(lines, [
    { name: 'fleet', file: 'fleet.bp', links: [] },
    { name: 'release', file: '@blueprints/release.bp', links: ['@blueprints/prompts/ask.md'] },
    { name: 'std/board', file: null, links: [] },
    { name: 'std/working', file: null, links: [] },
  ]);
});

test('one that is both its own and applied is listed once, as the file it is', () => {
  const lines = listed({ applied: ['fleet', 'fleet'], own: [{ name: 'fleet', file: 'fleet.bp' }] });
  assert.deepEqual(lines, [{ name: 'fleet', file: 'fleet.bp', links: [] }]);
});

test('a project that uses nothing, or is not known yet, lists nothing', () => {
  assert.deepEqual(listed({ applied: [], own: [] }), []);
  assert.deepEqual(listed(undefined), []);
});

test('an instance coming or going may change what a project uses, and nothing else an instance says does', () => {
  for (const topic of ['instance.started', 'instance.active', 'instance.finished', 'instance.stopped', 'instance.failed']) {
    assert.equal(changesUse(topic), true, topic);
  }
  for (const topic of ['instance.state', 'instance.updated', 'run.started', 'step.finished']) {
    assert.equal(changesUse(topic), false, topic);
  }
});

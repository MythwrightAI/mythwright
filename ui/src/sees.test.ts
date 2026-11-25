import assert from 'node:assert/strict';
import { test } from 'node:test';

import { sees } from './sees.ts';

const behind = { what: 'its work', how: 'behind a wall', walled: true };

test('what an agent sees is said in the daemon\'s words, with what it may do with the repository where it is behind a wall', () => {
  assert.deepEqual(sees({ sees: behind }), ['its work', 'behind a wall']);
  assert.deepEqual(sees({ sees: behind, git: 'read' }), ['its work', 'behind a wall'], 'reading is the usual');
  assert.deepEqual(sees({ sees: behind, git: 'commit' }), ['its work', 'behind a wall; git commit']);
  assert.deepEqual(sees({ sees: { ...behind, how: 'behind a wall, and shown 2 more' } }), ['its work', 'behind a wall, and shown 2 more']);
});

test('a slot that asks for a wall in a factory that holds none is said to have none, and nothing of the repository', () => {
  const open = { what: 'this device', how: 'no wall, since this factory holds none', walled: false };
  assert.deepEqual(sees({ sees: open, git: 'commit' }), ['this device', 'no wall, since this factory holds none']);
  const none = { what: 'this device', how: 'no wall, as whoever runs the daemon', walled: false };
  assert.deepEqual(sees({ sees: none }), ['this device', 'no wall, as whoever runs the daemon']);
});

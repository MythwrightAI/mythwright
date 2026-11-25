import assert from 'node:assert/strict';
import { test } from 'node:test';
import type { GraphNode } from './api.ts';
import { runOf, stepSays } from './runsaid.ts';

const node = (group: string, name: string, description: string | null, kind: GraphNode['kind'] = 'action') => ({ id: `${group}/${name}`, group, name, kind, description }) as GraphNode;

const drawn = {
  revision: 'aaaa',
  graph: {
    nodes: [
      node('main', 'room', 'Look before leaping; say so if this device is short on room.'),
      node('main', 'kept', null),
      node('on.looked', '@', 'What sets it off.', 'trigger'),
      node('on.looked', 'wrong', "Check this stage's row and say where the fleet reports what would leave a card stuck. Nothing is said where nothing would."),
      node('on.looked', 'each', 'One of them.'),
      node('on.looked', 'task.inner', 'Inside a task.'),
      node('on.other', 'wrong', 'Another handler has a step called the same.'),
    ],
  },
};

test('a step of a run is said by what it says of itself, as far as its first full stop', () => {
  assert.equal(stepSays(drawn, { flow: 'on.looked', revision: 'aaaa' }, 'wrong'), "Check this stage's row and say where the fleet reports what would leave a card stuck.");
  assert.equal(stepSays(drawn, { flow: 'main', revision: 'aaaa' }, 'room'), 'Look before leaping; say so if this device is short on room.');
  assert.equal(stepSays(drawn, { flow: 'on.looked', revision: 'aaaa' }, 'task.inner'), 'Inside a task.');
});

test('a step that says nothing of itself says nothing, and neither does one the text has not got', () => {
  assert.equal(stepSays(drawn, { flow: 'main', revision: 'aaaa' }, 'kept'), '');
  assert.equal(stepSays(drawn, { flow: 'main', revision: 'aaaa' }, 'gone'), '');
  assert.equal(stepSays(drawn, { flow: 'on_update', revision: 'aaaa' }, 'wrong'), '');
});

test('a step is its own flow’s: one called the same in another says what that one says', () => {
  assert.equal(stepSays(drawn, { flow: 'on.other', revision: 'aaaa' }, 'wrong'), 'Another handler has a step called the same.');
  assert.equal(stepSays(drawn, { flow: 'main', revision: 'aaaa' }, 'wrong'), '');
});

test('each of a list is said as its step is', () => {
  assert.equal(stepSays(drawn, { flow: 'on.looked', revision: 'aaaa' }, 'each[0]'), 'One of them.');
  assert.equal(stepSays(drawn, { flow: 'on.looked', revision: 'aaaa' }, 'each[12]'), 'One of them.');
});

test('what sets a handler off is no step of its run', () => {
  assert.equal(stepSays(drawn, { flow: 'on.looked', revision: 'aaaa' }, '@'), '');
});

test('a run of another text than the one drawn is shown by its keys: a key may mean something else there', () => {
  assert.equal(stepSays(drawn, { flow: 'on.looked', revision: 'bbbb' }, 'wrong'), '');
  assert.equal(stepSays({ ...drawn, revision: 'bbbb' }, { flow: 'on.looked', revision: 'aaaa' }, 'wrong'), '');
});

test('with no text drawn, or one that does not read, a run is shown by its keys', () => {
  assert.equal(stepSays(null, { flow: 'on.looked', revision: 'aaaa' }, 'wrong'), '');
  assert.equal(stepSays(undefined, { flow: 'on.looked', revision: 'aaaa' }, 'wrong'), '');
  assert.equal(stepSays({ revision: 'aaaa', graph: null }, { flow: 'on.looked', revision: 'aaaa' }, 'wrong'), '');
});

test('a run of a text’s own steps is said in words, with what the text calls them; a handler’s by its handler', () => {
  assert.deepEqual(runOf('main'), { words: 'Its steps', key: 'main' });
  assert.deepEqual(runOf('on.looked'), { words: 'looked', key: null });
  assert.deepEqual(runOf('on_update'), { words: 'on_update', key: null });
});

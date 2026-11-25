import assert from 'node:assert/strict';
import { test } from 'node:test';
import { COME, standing } from './textsetting.ts';

test('what is so of how texts are come by is said in a sentence, by what was said and by where the factory runs', () => {
  assert.match(standing({ come: 'follow', usual: 'held' }), /taken as it changes/);
  assert.match(standing({ come: 'held', usual: 'read' }), /until somebody applies it saying to take them/);
  assert.match(standing({ come: 'usual', usual: 'held' }), /until somebody applies it saying to take them/);
  assert.match(standing({ come: 'usual', usual: 'read' }), /checkout's blueprints are followed/);
  assert.deepEqual(Object.keys(COME), ['usual', 'held', 'follow']);
});

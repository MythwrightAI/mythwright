import assert from 'node:assert/strict';
import { test } from 'node:test';
import { forgetRunning, idsIn, known, named, parts, tell, titled } from './running.ts';

test('the ids of things running are found in a text, with what is around them', () => {
  assert.deepEqual(parts('{\n  "instance": "in_1m7qfzbgtp98vz92"\n}'), ['{\n  "instance": "', { id: 'in_1m7qfzbgtp98vz92' }, '"\n}']);
  assert.deepEqual(parts('in_1m7qfzbgtp98vz92 started in_1m7qg52khgwyb62w'), [{ id: 'in_1m7qfzbgtp98vz92' }, ' started ', { id: 'in_1m7qg52khgwyb62w' }]);
  assert.deepEqual(parts('nothing running is named here'), ['nothing running is named here']);
  assert.deepEqual(parts(''), []);
});

test('a word that only begins as an id does is not taken for one', () => {
  for (const word of ['in_place', 'in_flight: finish', 'in_ms: 250', 'in_1', 'in_progressively', 'login_1m7qfzbgtp98vz92', 'in_1m7qfzbgtp98vz92_more', 'run_1m7qg52kfgtf555x']) {
    assert.deepEqual(idsIn(word), [], word);
  }
});

test('each id in a text is said once', () => {
  assert.deepEqual(idsIn('in_1m7qfzbgtp98vz92 and in_1m7qfzbgtp98vz92 and in_1m7qg52khgwyb62w'), ['in_1m7qfzbgtp98vz92', 'in_1m7qg52khgwyb62w']);
});

test('an id is said as the blueprint it runs and its key, as gone where it names nothing, and as itself until it is read', () => {
  assert.equal(named({ blueprint: 'std/fleet', key: 'fleet-shop' }), 'std/fleet · fleet-shop');
  assert.equal(titled('in_1m7qfzbgtp98vz92', { blueprint: 'std/board', key: 'board' }), 'std/board · board');
  assert.equal(titled('in_1m7qfzbgtp98vz92', 'gone'), 'in_1m7qfzbgtp98vz92 (gone)');
  assert.equal(titled('in_1m7qfzbgtp98vz92', undefined), 'in_1m7qfzbgtp98vz92');
});

test('what was read of an id is kept, and let go of when whoever was signed in no longer is', () => {
  forgetRunning();
  tell('in_1m7qfzbgtp98vz92', { blueprint: 'std/board', key: 'board' });
  const kept = known.value;
  tell('in_1m7qfzbgtp98vz92', { blueprint: 'std/board', key: 'board' });
  assert.equal(known.value, kept, 'told the same again, nothing is drawn again for it');
  tell('in_1m7qg52khgwyb62w', 'gone');
  assert.deepEqual(known.value, { in_1m7qfzbgtp98vz92: { blueprint: 'std/board', key: 'board' }, in_1m7qg52khgwyb62w: 'gone' });
  tell('in_1m7qfzbgtp98vz92', 'gone');
  assert.equal(known.value.in_1m7qfzbgtp98vz92, 'gone', 'one that was there and is no longer is said to be gone');
  forgetRunning();
  assert.deepEqual(known.value, {});
});

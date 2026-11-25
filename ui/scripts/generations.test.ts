import assert from 'node:assert/strict';
import { test } from 'node:test';
import { DAY, KEPT_FOR, KEPT_FOR_DAYS, KEPT_MOST, generations, leftovers } from './generations.ts';

const NOW = Date.UTC(2026, 10, 21);
const one = { id: 'one', at: NOW - 3 * DAY, files: ['assets/app-1.js', 'assets/chunk-editor.js'] };
const two = { id: 'two', at: NOW - 2 * DAY, files: ['assets/app-2.js', 'assets/chunk-editor.js'] };
const three = { id: 'three', at: NOW, files: ['assets/app-3.js', 'assets/chunk-editor-new.js'] };

/** `n` builds, the newest first, each an hour after the one before it, the newest an hour before `NOW`. */
const hourly = (n: number) =>
  Array.from({ length: n }, (_, at) => ({ id: `b${at}`, at: NOW - (at + 1) * 60 * 60 * 1000, files: [`assets/app-b${at}.js`] }));

test('the figures are a week and twenty builds', () => {
  // A test of the tooling holds the installed copy to the same two figures
  // (`xtask/tests/suite/install.rs`), read from this file by their names.
  assert.equal(KEPT_FOR_DAYS, 7);
  assert.equal(KEPT_FOR, 7 * DAY);
  assert.equal(KEPT_MOST, 20);
});

test('the build before the new one is kept, so a page still on it can load the rest of itself', () => {
  assert.deepEqual(generations([one], two), [two, one]);
  assert.deepEqual(leftovers([two, one], ['assets/app-1.js', 'assets/app-2.js', 'assets/chunk-editor.js']), []);
});

test('a page two and three builds behind still has every part of its build', () => {
  const kept = generations([two, one], three);
  assert.deepEqual(kept, [three, two, one]);
  const present = [...one.files, ...two.files, ...three.files];
  assert.deepEqual(leftovers(kept, present), []);
});

test('a build whose place was taken a week or more before the new one is removed, and every build before it', () => {
  const old = { id: 'old', at: NOW - 10 * DAY, files: ['assets/app-old.js'] };
  const replaced = { id: 'replaced', at: NOW - KEPT_FOR, files: ['assets/app-replaced.js'] };
  const young = { id: 'young', at: NOW - KEPT_FOR + 1, files: ['assets/app-young.js'] };
  // `young` took the place of `replaced` a moment less than a week ago, and
  // `replaced` took that of `old` more than a week ago.
  assert.deepEqual(
    generations([young, replaced, old], three).map((build) => build.id),
    ['three', 'young', 'replaced'],
  );
  assert.deepEqual(
    generations([replaced, old], three).map((build) => build.id),
    ['three', 'replaced'],
    'the build the new one replaces is kept however long it was current',
  );
});

test('twenty builds before the current one are kept at most, the newest of them', () => {
  const before = hourly(KEPT_MOST + 5);
  const kept = generations(before, three);
  assert.equal(kept.length, 1 + KEPT_MOST);
  assert.deepEqual(kept.slice(1), before.slice(0, KEPT_MOST));
  assert.deepEqual(
    leftovers(kept, before.flatMap((build) => build.files)),
    before.slice(KEPT_MOST).flatMap((build) => build.files),
  );
});

test('builds are kept by when they were made, whatever order they were listed in', () => {
  assert.deepEqual(generations([one, two], three), [three, two, one]);
});

test('a build listed before builds said when they were made is kept as one made now', () => {
  const unsaid = { id: 'unsaid', files: ['assets/app-unsaid.js'] };
  const kept = generations([unsaid], three);
  assert.deepEqual(kept, [three, { ...unsaid, at: NOW }]);
  // Written so, it is a week from then and not from each build after.
  const later = { id: 'later', at: NOW + KEPT_FOR + DAY, files: ['assets/app-later.js'] };
  const after = { id: 'after', at: NOW + 2 * DAY, files: [] };
  assert.deepEqual(
    generations([after, ...kept], later).map((build) => build.id),
    ['later', 'after', 'three'],
  );
});

test('a file two kept builds share is not removed with the older of them', () => {
  const kept = generations([two, one], three);
  assert.ok(!leftovers(kept.slice(0, 2), ['assets/chunk-editor.js']).includes('assets/chunk-editor.js'));
});

test('building the same thing again does not push the previous one out', () => {
  assert.deepEqual(generations([two, one], { ...two, at: NOW }), [{ ...two, at: NOW }, one]);
});

test('going back to an earlier build keeps the ones it replaces', () => {
  assert.deepEqual(generations([two, one], { ...one, at: NOW }), [{ ...one, at: NOW }, two]);
});

test('files nothing is known about are leftovers', () => {
  assert.deepEqual(leftovers([one], ['assets/app-1.js', 'assets/stray.js']), ['assets/stray.js']);
});

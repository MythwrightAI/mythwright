import assert from 'node:assert/strict';
import { test } from 'node:test';
import { begun, keyed, lineAt, playedFrom, pressedPlay, startOf, stepTo, trackAt } from './player.ts';

const frames = [{ lasts: 100 }, { lasts: 200 }, { lasts: 50 }];

test('a part read begins at its first frame, playing; where nothing is to move, or nothing to play, still with no frame shown', () => {
  assert.deepEqual(begun(false, 3), { at: 0, playing: true });
  assert.deepEqual(begun(true, 3), { at: null, playing: false });
  assert.deepEqual(begun(false, 0), { at: null, playing: false });
});

test('each frame begins once those before it have been shown for their own time', () => {
  assert.deepEqual([0, 1, 2, 3].map((at) => startOf(frames, at)), [0, 100, 300, 350]);
});

test('played once from a frame, each is shown for its own time, and at the end the last is held and nothing plays', () => {
  assert.deepEqual([0, 99, 100, 299, 300, 349].map((elapsed) => playedFrom(frames, 0, elapsed)), [
    { at: 0, playing: true },
    { at: 0, playing: true },
    { at: 1, playing: true },
    { at: 1, playing: true },
    { at: 2, playing: true },
    { at: 2, playing: true },
  ]);
  assert.deepEqual(playedFrom(frames, 0, 350), { at: 2, playing: false });
  assert.deepEqual(playedFrom(frames, 0, 10_000), { at: 2, playing: false });
  // Gone on from a frame in the middle, from its beginning.
  assert.deepEqual(playedFrom(frames, 1, 0), { at: 1, playing: true });
  assert.deepEqual(playedFrom(frames, 1, 200), { at: 2, playing: true });
  assert.deepEqual(playedFrom([], 0, 0), { at: null, playing: false });
});

test('Play pauses what plays, goes on from where it was paused, and plays again from the first at the end or from still', () => {
  assert.deepEqual(pressedPlay(3, { at: 1, playing: true }), { at: 1, playing: false });
  assert.deepEqual(pressedPlay(3, { at: 1, playing: false }), { at: 1, playing: true });
  assert.deepEqual(pressedPlay(3, { at: 2, playing: false }), { at: 0, playing: true });
  assert.deepEqual(pressedPlay(3, { at: null, playing: false }), { at: 0, playing: true });
  assert.deepEqual(pressedPlay(0, { at: null, playing: false }), { at: null, playing: false });
});

test('the next and the one before are paused there and go past neither end; from still the first and the last', () => {
  assert.deepEqual(stepTo(3, { at: 1, playing: true }, 1), { at: 2, playing: false });
  assert.deepEqual(stepTo(3, { at: 2, playing: false }, 1), { at: 2, playing: false });
  assert.deepEqual(stepTo(3, { at: 1, playing: false }, -1), { at: 0, playing: false });
  assert.deepEqual(stepTo(3, { at: 0, playing: false }, -1), { at: 0, playing: false });
  assert.deepEqual(stepTo(3, { at: null, playing: false }, 1), { at: 0, playing: false });
  assert.deepEqual(stepTo(3, { at: null, playing: false }, -1), { at: 2, playing: false });
  assert.deepEqual(stepTo(0, { at: null, playing: false }, 1), { at: null, playing: false });
});

test('the track is the frames side by side, and a point of it is the frame under it, paused', () => {
  assert.deepEqual([0, 33, 34, 66, 67, 99, 100, 140, -5].map((x) => trackAt(3, x, 100).at), [0, 0, 1, 1, 2, 2, 2, 2, 0]);
  assert.equal(trackAt(3, 10, 100).playing, false);
  assert.deepEqual(trackAt(0, 10, 100), { at: null, playing: false });
});

test('a line is pressed to the first frame of it or of a line under it, and a line no frame is of has none', () => {
  const lines = [{ line: null }, { line: [0] }, { line: [1, 0] }, { line: [1, 1] }, { line: [0] }];
  assert.equal(lineAt(lines, [0]), 1);
  assert.equal(lineAt(lines, [1]), 2);
  assert.equal(lineAt(lines, [1, 1]), 3);
  assert.equal(lineAt(lines, [2]), null);
});

test('Space plays and pauses, the left arrow and , go back a frame, the right arrow and . on, and no other key does anything', () => {
  const head = { at: 1, playing: false };
  assert.deepEqual(keyed(' ', 3, head), { at: 1, playing: true });
  assert.deepEqual(keyed(' ', 3, { at: 1, playing: true }), { at: 1, playing: false });
  assert.deepEqual(keyed('ArrowLeft', 3, head), { at: 0, playing: false });
  assert.deepEqual(keyed(',', 3, head), { at: 0, playing: false });
  assert.deepEqual(keyed('ArrowRight', 3, head), { at: 2, playing: false });
  assert.deepEqual(keyed('.', 3, head), { at: 2, playing: false });
  assert.equal(keyed('Enter', 3, head), null);
  assert.equal(keyed('a', 3, head), null);
});

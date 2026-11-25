import assert from 'node:assert/strict';
import { test } from 'node:test';
import { acrossOf, acrossSaid, mayQuiet } from './across.ts';

const denver = { zone: 'America/Denver', hours: '12h' as const };
// 2026-11-23 21:05 UTC, a Monday: 2:05 PM in Denver.
const made = Date.UTC(2026, 10, 23, 21, 5);

test('a copy says what it is a copy of and when it was taken, in the reader’s zone, and what is done there stays there', () => {
  const copy = acrossOf({ kind: 'copy', of: 'prod', made, profile: 'preview', words: 'A copy of prod taken 2026-11-23 21:05 UTC: what is done here stays here.' })!;
  assert.equal(acrossSaid(copy, denver, made + 60_000), 'Profile: preview. A copy of prod taken 2:05 PM MST: what is done here stays here.');
  assert.match(acrossSaid(copy, denver, made + 3 * 86_400_000), /^Profile: preview\. A copy of prod taken Mon 2:05 PM MST:/);
  // One of a factory with no profile is said by the device it was on.
  const nameless = acrossOf({ kind: 'copy', device: 'studio-laptop', made, words: '' })!;
  assert.equal(acrossSaid(nameless, denver, made), 'A copy of the factory on studio-laptop taken 2:05 PM MST: what is done here stays here.');
});

test('what it was told to say is said as the daemon words it, copy or not', () => {
  assert.equal(acrossSaid(acrossOf({ kind: 'said', says: 'Trying the release.', profile: 'preview', words: 'Trying the release.' })!, denver), 'Trying the release.');
});

test('nothing is said where the daemon says nothing or something this page does not read', () => {
  assert.equal(acrossOf(null), null);
  assert.equal(acrossOf(undefined), null);
  assert.equal(acrossOf({ kind: 'later', words: 'x' }), null);
  // A factory with a profile and nothing else to say says nothing there: its profile is said where it always was.
  assert.equal(acrossOf({ kind: 'profile', profile: 'prod', words: 'Profile: prod' }), null);
});

test('whoever manages the factory, and nobody else, may say it may stop', () => {
  assert.equal(mayQuiet(['*']), true);
  assert.equal(mayQuiet(['admin', 'board']), true);
  assert.equal(mayQuiet(['board', 'channels']), false);
});

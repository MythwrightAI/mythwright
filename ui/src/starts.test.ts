import assert from 'node:assert/strict';
import { test } from 'node:test';
import type { Start } from './api.ts';
import { ADDING, chosenAfter, had, hasOf, lacks, offeredAt, usual, wantedAfter, whyNot, whyNotYet } from './starts.ts';

const start = (name: string, more: Partial<Start> = {}): Start => ({
  name,
  title: `A ${name}`,
  says: '',
  blueprint: `std/${name}`,
  on: true,
  needs: [],
  why: '',
  ...more,
});
// A list that is nobody's: what holds is about the list, not about what is in it.
const STARTS = [start('ledger'), start('clerk', { needs: ['ledger'], why: 'a clerk keeps a ledger' }), start('shelf'), start('lamp', { on: false })];

test('a project starts with whatever is on unless said, in the order listed', () => {
  assert.deepEqual(usual(STARTS), ['ledger', 'clerk', 'shelf']);
});

test('turning something off turns off what cannot be had without it, and nothing else', () => {
  assert.deepEqual(chosenAfter(STARTS, usual(STARTS), 'ledger', false), ['shelf']);
  assert.deepEqual(chosenAfter(STARTS, usual(STARTS), 'shelf', false), ['ledger', 'clerk']);
  assert.deepEqual(chosenAfter(STARTS, usual(STARTS), 'clerk', false), ['ledger', 'shelf']);
});

test('what is turned off because of another is turned off through however many', () => {
  const chain = [start('a'), start('b', { needs: ['a'] }), start('c', { needs: ['b'] })];
  assert.deepEqual(chosenAfter(chain, ['a', 'b', 'c'], 'a', false), []);
});

test('turning something on turns nothing else on, and it does not stay on without what it needs', () => {
  assert.deepEqual(chosenAfter(STARTS, ['shelf'], 'clerk', true), ['shelf']);
  assert.deepEqual(chosenAfter(STARTS, ['shelf'], 'ledger', true), ['ledger', 'shelf']);
  assert.deepEqual(chosenAfter(STARTS, ['ledger', 'shelf'], 'clerk', true), ['ledger', 'clerk', 'shelf']);
  assert.deepEqual(chosenAfter(STARTS, ['ledger'], 'lamp', true), ['ledger', 'lamp']);
});

test('why something cannot be turned on is said by what it lacks, and nothing is said where it can', () => {
  const clerk = STARTS[1]!;
  assert.deepEqual(lacks(clerk, ['shelf']), ['ledger']);
  assert.equal(whyNot(STARTS, clerk, ['shelf']), 'A clerk keeps a ledger: turn on a ledger to have one.');
  assert.equal(whyNot(STARTS, clerk, ['ledger']), null);
  assert.equal(whyNot(STARTS, STARTS[0]!, []), null);
});

test('what went off for the lack of another comes back with it, and what somebody turned off themselves stays off', () => {
  // The ledger is turned off: the clerk goes with it, though nobody turned the clerk off.
  let wanted = wantedAfter(STARTS, usual(STARTS), 'ledger', false);
  assert.deepEqual(had(STARTS, wanted), ['shelf']);
  // Turned on again, the clerk is back.
  wanted = wantedAfter(STARTS, wanted, 'ledger', true);
  assert.deepEqual(had(STARTS, wanted), ['ledger', 'clerk', 'shelf']);
  // Somebody who turned the clerk off keeps it off through the ledger going and coming.
  wanted = wantedAfter(STARTS, wanted, 'clerk', false);
  wanted = wantedAfter(STARTS, wanted, 'ledger', false);
  wanted = wantedAfter(STARTS, wanted, 'ledger', true);
  assert.deepEqual(had(STARTS, wanted), ['ledger', 'shelf']);
  // Through however many: the last of a chain comes back when the first does.
  const chain = [start('a'), start('b', { needs: ['a'] }), start('c', { needs: ['b'] })];
  const off = wantedAfter(chain, ['a', 'b', 'c'], 'a', false);
  assert.deepEqual(had(chain, off), []);
  assert.deepEqual(had(chain, wantedAfter(chain, off, 'a', true)), ['a', 'b', 'c']);
  // What is had is in the order listed, whatever order it was pressed in.
  assert.deepEqual(had(STARTS, ['shelf', 'clerk', 'ledger']), ['ledger', 'clerk', 'shelf']);
});

test('each thing is offered where it says, and one that says nothing as a project is added', () => {
  const list = [start('ledger', { offered: ADDING }), start('clerk'), start('lamp', { offered: 'ledger' }), start('bell', { offered: 'ledger' }), start('rug', { offered: 'shelf' })];
  assert.deepEqual(offeredAt(list, ADDING).map((one) => one.name), ['ledger', 'clerk']);
  assert.deepEqual(offeredAt(list, 'ledger').map((one) => one.name), ['lamp', 'bell']);
  assert.deepEqual(offeredAt(list, 'clerk'), []);
  // What is offered as a project is added is chosen among by itself: nothing offered elsewhere is on unless said.
  assert.deepEqual(usual(offeredAt([...STARTS, start('rug', { offered: 'shelf' })], ADDING)), ['ledger', 'clerk', 'shelf']);
});

test('a project has what of the list is alive in it, by the blueprint each is', () => {
  assert.deepEqual(hasOf(STARTS, ['std/shelf', 'std/ledger', 'std/else']), ['ledger', 'shelf']);
  assert.deepEqual(hasOf(STARTS, []), []);
  // A blueprint two of the list would be is both of them, and one nobody lists is none.
  assert.deepEqual(hasOf([start('a', { blueprint: 'one' }), start('b', { blueprint: 'one' })], ['one']), ['a', 'b']);
});

test('what a project cannot be given yet says why, by what is not running for it', () => {
  const clerk = STARTS[1]!;
  assert.equal(whyNotYet(STARTS, clerk, ['shelf']), 'A clerk keeps a ledger: a ledger is not running for this project.');
  assert.equal(whyNotYet(STARTS, clerk, ['ledger']), null);
  assert.equal(whyNotYet(STARTS, STARTS[0]!, []), null);
  const both = start('desk', { needs: ['ledger', 'shelf'] });
  assert.equal(whyNotYet(STARTS, both, []), 'A desk needs it: a ledger and a shelf are not running for this project.');
  assert.equal(whyNotYet(STARTS, both, ['ledger']), 'A desk needs it: a shelf is not running for this project.');
});

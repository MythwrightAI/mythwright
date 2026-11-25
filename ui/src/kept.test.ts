import assert from 'node:assert/strict';
import { test } from 'node:test';
import { calledOf, gone, hashed, lastPerson, mayAdd, mayKeep, mayLeave, mayOpen } from './kept.ts';
import type { Asks, Called } from './kept.ts';

const member = (actor: string) => ({ actor, name: actor });
const channel = (kept: boolean, ...members: string[]): Called => ({
  id: 'ch_1',
  name: kept ? 'ch_1' : 'general',
  kept: kept || undefined,
  title: kept ? 'Pay review' : undefined,
  members: members.map(member),
});
const who = (actor: string, scopes: string[], owner = false): Asks => ({ actor, scopes, owner });
const mira = who('user:mira', ['channels', 'agents']);
const reads = who('user:mira', ['channels:read']);
const owner = who('user:ada', ['channels', 'admin', 'private'], true);

test('a channel is called its name, and one kept to its members what it was made as', () => {
  assert.equal(calledOf(channel(false)), 'general');
  assert.equal(hashed(channel(false)), '#general');
  assert.equal(calledOf(channel(true)), 'Pay review');
  assert.equal(hashed(channel(true)), 'Pay review');
  // One that says no title is shown by what it has.
  assert.equal(calledOf({ name: 'ch_1', kept: true }), 'ch_1');
});

test('an open channel is made private by somebody who is in it and may change channels', () => {
  assert.equal(mayKeep(channel(false, 'user:mira'), mira), true);
  assert.equal(mayKeep(channel(false, 'user:nell'), mira), false);
  assert.equal(mayKeep(channel(false, 'user:mira'), reads), false);
  assert.equal(mayKeep(channel(true, 'user:mira'), mira), false);
});

test('a private channel is opened again by the owner and by nobody else', () => {
  assert.equal(mayOpen(channel(true, 'user:mira'), owner), true);
  assert.equal(mayOpen(channel(true, 'user:mira'), mira), false);
  assert.equal(mayOpen(channel(true, 'user:mira'), who('user:mira', ['channels', 'admin'])), false);
  assert.equal(mayOpen(channel(false, 'user:mira'), owner), false);
  // A key of the owner's that was given less is not offered it either.
  assert.equal(mayOpen(channel(true), who('user:ada', ['channels'], true)), false);
});

test('somebody who is in a channel leaves it, but for the last person in a private one', () => {
  assert.equal(mayLeave(channel(false, 'user:mira'), mira), true);
  assert.equal(mayLeave(channel(false, 'user:nell'), mira), false);
  assert.equal(mayLeave(channel(true, 'user:mira', 'user:nell'), mira), true);
  assert.equal(mayLeave(channel(true, 'user:mira', 'agent:scribe', 'bot:conductor'), mira), false);
  assert.equal(lastPerson(channel(true, 'user:mira', 'agent:scribe'), mira), true);
  assert.equal(lastPerson(channel(false, 'user:mira'), mira), false);
  assert.equal(mayLeave(channel(true, 'user:mira', 'user:nell'), reads), false);
});

test('somebody is added to a private channel by one who is in it', () => {
  assert.equal(mayAdd(channel(false), mira), true);
  assert.equal(mayAdd(channel(true, 'user:mira'), mira), true);
  assert.equal(mayAdd(channel(true, 'user:nell'), mira), false);
  assert.equal(mayAdd(channel(true, 'user:nell'), owner), true);
  assert.equal(mayAdd(channel(false), reads), false);
});

test('a channel that was listed and is listed no longer is gone', () => {
  const all = [{ id: 'a' }, { id: 'b' }, { id: 'c' }];
  assert.deepEqual(gone(all, [{ id: 'c' }, { id: 'a' }, { id: 'd' }]), ['b']);
  assert.deepEqual(gone(all, all), []);
  assert.deepEqual(gone([], all), []);
});

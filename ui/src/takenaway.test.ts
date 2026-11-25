import assert from 'node:assert/strict';
import { test } from 'node:test';
import { type Removal, goes, mayTakeAway, stays } from './takenaway.ts';

const found = (more: Partial<Removal>): Removal => ({
  name: 'helper',
  project: 'shop',
  removed: false,
  refused: null,
  active: true,
  channels: [],
  turns: 0,
  cards: [],
  folder_left: null,
  ...more,
});

test('taking an agent away is offered to a person who may change agents and manages the factory, and to nobody else', () => {
  const cases: [string, string[], boolean][] = [
    ['user', ['*'], true],
    ['user', ['agents', 'admin'], true],
    ['user', ['agents:write', 'admin'], true],
    // A member changes agents and does not manage the factory.
    ['user', ['projects', 'channels', 'board', 'agents', 'shell'], false],
    ['user', ['admin', 'agents:read'], false],
    ['user', [], false],
    // An agent is never offered it, whatever it holds.
    ['agent', ['*'], false],
    ['agent', ['agents', 'admin'], false],
  ];
  for (const [kind, scopes, may] of cases) assert.equal(mayTakeAway({ kind, scopes }), may, `${kind} ${scopes}`);
});

test('what goes with an agent is its slot, its credential, its place in each channel, and its folder', () => {
  assert.deepEqual(goes(found({})), [
    'Its slot: what it was told, what it may do and what it holds.',
    'Its credential, which is nobody’s from then on.',
    'The folder that was its own on this device: the home its program was shown.',
  ]);
  assert.equal(goes(found({ channels: ['shop'] }))[2], 'Its place in #shop.');
  assert.equal(goes(found({ channels: ['shop', 'yard'] }))[2], 'Its place in #shop and #yard.');
  assert.equal(goes(found({ channels: ['a', 'b', 'c'] }))[2], 'Its place in #a, #b and #c.');
  // One that works in no project wrote what it wrote in its own folder.
  assert.equal(
    goes(found({ project: null })).at(-1),
    'The folder that was its own on this device: the home its program was shown and, since it works in no project, whatever it wrote there.',
  );
});

test('what stays is what it said, what it was asked, and the cards given to it', () => {
  assert.deepEqual(stays(found({})), ['What it said in channels, and the history of what it changed.']);
  assert.equal(stays(found({ turns: 1 }))[1], 'The one piece of work it was asked, with what it answered.');
  assert.equal(stays(found({ turns: 14 }))[1], 'The 14 pieces of work it was asked, with what it answered.');
  assert.equal(stays(found({ cards: ['shop-1'] }))[1], 'shop-1 is given to it, and stays so until somebody gives it to another.');
  assert.deepEqual(stays(found({ turns: 2, cards: ['shop-1', 'shop-4'] })), [
    'What it said in channels, and the history of what it changed.',
    'The 2 pieces of work it was asked, with what it answered.',
    '2 cards are given to it (shop-1, shop-4), and stay so until somebody gives them to another.',
  ]);
});

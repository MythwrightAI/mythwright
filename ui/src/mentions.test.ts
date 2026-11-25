import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import { command, commanding, complete, fixedWords, handleOf, matching, mention, mentionable, namedBy, offeredAt, offeredUnder, saidIn, taggable, taking, topical, typing, under, whoIs } from './mentions.ts';

const members = [
  { actor: 'user:acc_1', name: 'Ada Lovelace' },
  { actor: 'agent:builder', name: 'builder' },
  { actor: 'bot:parrot', name: 'Parrot' },
  { actor: 'agent:reviewer', name: 'reviewer' },
];

test('an @ at the start of a word begins a mention, and what follows it is what was typed', () => {
  assert.deepEqual(typing('@', 1), { from: 0, typed: '' });
  assert.deepEqual(typing('hello @Bu', 9), { from: 6, typed: 'bu' });
  assert.deepEqual(typing('(@re', 4), { from: 1, typed: 're' });
  assert.deepEqual(typing('one\n@', 5), { from: 4, typed: '' });
});

test('an address, a finished mention and a caret elsewhere are not mentions being typed', () => {
  assert.equal(typing('ada@example', 11), null);
  assert.equal(typing('@builder please', 15), null, 'the caret has moved on');
  assert.equal(typing('@builder please', 0), null);
  assert.deepEqual(typing('@builder please', 4), { from: 0, typed: 'bui' }, 'back inside it, it is being typed again');
});

test('everyone in the channel can be mentioned, agents and bots first, each by the name it answers to', () => {
  assert.deepEqual(
    mentionable(members).map((one) => `${one.kind} @${one.handle}`),
    ['agent @builder', 'agent @reviewer', 'bot @parrot', 'user @ada-lovelace'],
  );
  assert.equal(handleOf('  Ada  Lovelace! '), 'ada-lovelace');
});

test('a person is named by the word the daemon names them by, case for case', () => {
  // One table for both: the daemon's rule is held to it by its own tests (`people::handle`).
  const cases = JSON.parse(readFileSync(new URL('./handles.json', import.meta.url), 'utf8')) as [string, string][];
  assert.ok(cases.length > 20, 'the table is there');
  for (const [name, handle] of cases) assert.equal(handleOf(name), handle, JSON.stringify(name));
});

test('what was typed narrows the choice: names that start with it, then names that contain it', () => {
  const all = mentionable(members);
  assert.deepEqual(matching(all, '').length, 4);
  assert.deepEqual(matching(all, 're').map((one) => one.handle), ['reviewer']);
  assert.deepEqual(matching(all, 'r').map((one) => one.handle), ['reviewer', 'builder', 'parrot']);
  assert.deepEqual(matching(all, 'love').map((one) => one.handle), ['ada-lovelace']);
  assert.deepEqual(matching(all, 'zz'), []);
});

test('choosing finishes the mention and leaves the caret after it', () => {
  assert.deepEqual(complete('hello @bu', 9, 6, 'builder'), { text: 'hello @builder ', caret: 15 });
  assert.deepEqual(complete('@re can you look', 3, 0, 'reviewer'), { text: '@reviewer can you look', caret: 10 }, 'no second space');
});

test('someone chosen without typing their name is mentioned where the caret is, apart from the word before', () => {
  assert.deepEqual(mention('', 0, 'builder'), { text: '@builder ', caret: 9 });
  assert.deepEqual(mention('look at this', 12, 'builder'), { text: 'look at this @builder ', caret: 22 });
  assert.deepEqual(mention('look  at this', 5, 'builder'), { text: 'look @builder at this', caret: 14 });
});

test('choosing someone finishes the mention being typed', () => {
  assert.deepEqual(mention('ask @bu', 7, 'builder'), { text: 'ask @builder ', caret: 13 });
});

const takes = [
  { written: 'status', does: 'what is running here' },
  { written: 'board <project>', does: 'a board' },
  { written: 'board move <card> <column> [--note TEXT]', does: 'move a card' },
  { written: 'agent stop <name> [--idle]', does: 'stop an agent' },
];

test('the words of a command that are always written are those before the first place it leaves open', () => {
  assert.equal(fixedWords('status'), 'status');
  assert.equal(fixedWords('board <project>'), 'board');
  assert.equal(fixedWords('board move <card> <column> [--note TEXT]'), 'board move');
});

test('what follows the bot\'s name on its line is what is being written to it, and nothing is before it is named', () => {
  assert.deepEqual(commanding('@conductor ', 11, 'conductor'), { from: 11, typed: '' });
  assert.deepEqual(commanding('please @Conductor Board  mo', 27, 'conductor'), { from: 18, typed: 'board mo' });
  assert.equal(commanding('@conductor', 10, 'conductor'), null, 'its name is still being typed');
  assert.equal(commanding('@conductor board\nmove', 21, 'conductor'), null, 'the next line is not said to it');
  assert.equal(commanding('me@conductor board', 18, 'conductor'), null, 'an address is not a mention');
  assert.equal(commanding('@conductor board', 16, null), null);
  assert.equal(commanding('@other board', 12, 'conductor'), null);
});

test('the commands offered are those what was typed could still become, and none once it is past them', () => {
  const names = (typed: string) => taking(takes, typed).map((one) => fixedWords(one.written));
  assert.deepEqual(names(''), ['status', 'board', 'board move', 'agent stop']);
  assert.deepEqual(names('b'), ['board', 'board move']);
  assert.deepEqual(names('board '), ['board move'], 'the one that was finished is not offered again');
  assert.deepEqual(names('board m'), ['board move']);
  assert.deepEqual(names('board move shop-1'), []);
  assert.deepEqual(names('weather'), []);
});

test('a command chosen is put where what was typed to the bot was, with room after it to go on', () => {
  assert.deepEqual(command('@conductor bo', 13, 11, 'board move <card> <column>'), { text: '@conductor board move ', caret: 22 });
  assert.deepEqual(command('@conductor st please', 13, 11, 'status'), { text: '@conductor status please', caret: 18 });
});

test('what is said under a name is whole and says how an agent stands, what it runs on, and what a bot is', () => {
  const [builder, , parrot, ada] = mentionable(members);
  assert.equal(under(builder!, { state: 'working', on: 'a model with a very long name indeed' }), 'working · a model with a very long name indeed');
  assert.equal(under(builder!, { state: 'idle', on: null }), 'idle');
  assert.equal(under(builder!, { state: 'stopped', on: 'slow' }), 'not running · slow');
  assert.equal(under(builder!), 'agent', 'one nothing is known of is still said to be an agent');
  assert.equal(under(parrot!, { takes: true }), 'bot · takes commands');
  assert.equal(under(parrot!), 'bot · answers by itself');
  assert.equal(under(ada!), 'Ada Lovelace');
});

test('on a record, whoever being named tells is offered: each person by their handle and each agent the factory does not start, never the writer', () => {
  const people = [
    { actor: 'user:acc_2', name: 'Nell Gwyn' },
    { actor: 'user:acc_1', name: 'Ada Lovelace' },
    { actor: 'user:acc_3', name: 'Mira' },
  ];
  const agents = [
    { name: 'builder', adapter: 'assistant' },
    { name: 'scout', adapter: 'external' },
  ];
  const offered = taggable(people, agents, 'user:acc_3');
  assert.deepEqual(
    offered.map((one) => [one.handle, one.kind]),
    [
      ['ada-lovelace', 'user'],
      ['nell-gwyn', 'user'],
      ['scout', 'agent'],
    ],
  );
  // An agent from outside writing is not offered itself.
  assert.deepEqual(taggable([], agents, 'agent:scout'), []);
});

test('what is offered as somebody writes stands over the box where it fits, and is never cut off by the window', () => {
  const window = { width: 1000, height: 800 };
  const size = { width: 300, height: 200 };
  // Room over the box: it stands there, its foot where it may come.
  assert.deepEqual(offeredAt({ left: 100, above: 500, below: 560, first: 'over' }, size, window), { left: 100, top: 300, most: null });
  // Too little over the box (a thread near the panel's top): under it, where there is room.
  assert.deepEqual(offeredAt({ left: 100, above: 150, below: 210, first: 'over' }, size, window), { left: 100, top: 210, most: null });
  // Written at length it is under the tools first, and over the box only where there is no room under them.
  assert.deepEqual(offeredAt({ left: 100, above: 500, below: 560, first: 'under' }, size, window), { left: 100, top: 560, most: null });
  assert.deepEqual(offeredAt({ left: 100, above: 700, below: 760, first: 'under' }, size, window), { left: 100, top: 500, most: null });
  // Room on neither side: the side with more, as tall as that room, scrolling in itself.
  assert.deepEqual(offeredAt({ left: 100, above: 150, below: 700, first: 'over' }, { width: 300, height: 400 }, { width: 1000, height: 800 }), { left: 100, top: 8, most: 142 });
  assert.deepEqual(offeredAt({ left: 100, above: 100, below: 300, first: 'over' }, { width: 300, height: 600 }, { width: 1000, height: 800 }), { left: 100, top: 300, most: 492 });
  // Never under what is fixed at the window's top, and never past its right edge.
  assert.deepEqual(offeredAt({ left: 100, above: 230, below: 300, first: 'over' }, size, { ...window, top: 32 }), { left: 100, top: 300, most: null });
  assert.deepEqual(offeredAt({ left: 900, above: 500, below: 560, first: 'over' }, size, window), { left: 692, top: 300, most: null });
  assert.deepEqual(offeredAt({ left: 4, above: 500, below: 560, first: 'over' }, size, window), { left: 8, top: 300, most: null });
});

test('in a topic every person but whoever writes is offered, then every agent, whether the factory starts it or not', () => {
  const people = [
    { actor: 'user:acc_2', name: 'Nell Gwyn' },
    { actor: 'user:acc_1', name: 'Ada Lovelace' },
    { actor: 'user:acc_3', name: 'Me' },
  ];
  const agents = [
    { name: 'builder', adapter: 'assistant' },
    { name: 'scout', adapter: 'external' },
  ];
  assert.deepEqual(
    topical(people, agents, 'user:acc_3').map((one) => `${one.kind} @${one.handle}`),
    ['user @ada-lovelace', 'user @nell-gwyn', 'agent @builder', 'agent @scout'],
  );
  assert.deepEqual(
    topical([], agents, 'agent:builder').map((one) => one.handle),
    ['scout'],
    'an agent writing is not offered itself',
  );
});

test('an agent the factory starts and runs is said to answer when named, and nobody else is', () => {
  const builder = { handle: 'builder', name: 'builder', kind: 'agent' } as const;
  assert.equal(offeredUnder(builder, { state: 'idle', on: 'Echo', answers: true }), 'idle · Echo · answers when named');
  assert.equal(offeredUnder(builder, { state: 'stopped', on: 'Echo', answers: false }), 'not running · Echo');
  assert.equal(offeredUnder({ handle: 'ada', name: 'Ada', kind: 'user' }, { answers: true }), 'Ada');
});

test('words are split where an @ names somebody, as the daemon reads a name, and an address names nobody', () => {
  assert.deepEqual(saidIn('@Ada, ask @scout-2 about ada@example.com.'), [
    { handle: 'ada', written: '@Ada' },
    ', ask ',
    { handle: 'scout-2', written: '@scout-2' },
    ' about ada@example.com.',
  ]);
  assert.deepEqual(saidIn('no names here, nor @ alone'), ['no names here, nor @ alone']);
  assert.deepEqual(saidIn('(@nell)'), ['(', { handle: 'nell', written: '@nell' }, ')']);
  assert.deepEqual(saidIn('café@nell'), ['café@nell'], 'after a letter of any alphabet it is an address');
});

test('a name is an agent, the bot or a person known by it, and a name of nobody is nobody', () => {
  const known = {
    people: [
      { actor: 'user:acc_1', name: 'Ada Lovelace' },
      { actor: 'agent:scout', name: 'scout' },
    ],
    agents: [{ name: 'scout' }],
    bot: 'mw',
  };
  assert.deepEqual(namedBy('ada-lovelace', known), { handle: 'ada-lovelace', name: 'Ada Lovelace', kind: 'user', actor: 'user:acc_1' });
  assert.deepEqual(namedBy('scout', known), { handle: 'scout', name: 'scout', kind: 'agent', actor: 'agent:scout' });
  assert.deepEqual(namedBy('mw', known), { handle: 'mw', name: 'mw', kind: 'bot', actor: 'bot:mw' });
  assert.equal(namedBy('nobody', known), null);
  assert.equal(namedBy('ada', known), null, 'a person is named by the whole of their name');
});

test('who somebody named is says their name, what they are, and of an agent how it stands and what it runs on', () => {
  assert.equal(whoIs({ handle: 'ada', name: 'Ada Lovelace', kind: 'user' }), 'Ada Lovelace, person');
  assert.equal(whoIs({ handle: 'scout', name: 'scout', kind: 'agent' }, { state: 'working', on: 'Echo' }), 'scout, agent · working · Echo');
  assert.equal(whoIs({ handle: 'scout', name: 'scout', kind: 'agent' }, { on: 'external · reads what is said for itself' }), 'scout, agent · external · reads what is said for itself');
  assert.equal(whoIs({ handle: 'mw', name: 'mw', kind: 'bot' }, { takes: true }), 'mw, bot · takes commands');
});

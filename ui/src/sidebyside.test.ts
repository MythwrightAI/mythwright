import assert from 'node:assert/strict';
import { test } from 'node:test';
import type { Beside } from './api.ts';
import { allTheSame, asWays, asking, cannotPlay, chosenFirst, countsOf, forksOf, offered, played, playedSays, ranOf, standing, toggled } from './sidebyside.ts';
import type { Of } from './sidebyside.ts';

const of = (more: Partial<Of> = {}): Of => ({ source: 'here', saved: null, ships: false, shipped: null, isDefault: false, forks: [], ...more });
const shop = [{ project: 'shop', called: 'Shop', source: 'forked' }];
const words = (more: Partial<Of>) => offered(of(more)).map((text) => text.word);

test('the texts offered are those the page has of its blueprint, each once', () => {
  // A project's own text, saved as it stands: one text, and nothing to play beside it.
  assert.deepEqual(words({ saved: 'here' }), ['here']);
  // Changed and not saved: the page's and the file's.
  assert.deepEqual(words({ saved: 'saved' }), ['here', 'saved']);
  // The default's own page: the default as it ships, and each fork of it there is.
  assert.deepEqual(words({ ships: true, isDefault: true, forks: shop }), ['default', 'fork-shop']);
  assert.equal(offered(of({ ships: true, isDefault: true, forks: shop }))[1]!.called, 'The fork in Shop');
  // A fork that says what ships to the letter is one text with the default.
  assert.deepEqual(words({ ships: true, isDefault: true, source: 'same', forks: [{ project: 'shop', called: 'Shop', source: 'same' }] }), ['default']);
  assert.deepEqual(offered(of({ ships: true, isDefault: true, source: 'same', forks: [{ project: 'shop', called: 'Shop', source: 'same' }] }))[0]!.same, ['The fork in Shop']);
  // A fork's page: as it stands, as saved where that is another, and the default, which the daemon reads where the page has not.
  assert.deepEqual(words({ ships: true, saved: 'saved' }), ['here', 'saved', 'default']);
  assert.equal(offered(of({ ships: true })).at(-1)!.source, null);
  assert.deepEqual(words({ ships: true, shipped: 'here' }), ['here']);
});

test('two are chosen at first, and pressing one takes it in or out in the order offered, four at most', () => {
  const texts = offered(of({ ships: true, saved: 'saved', shipped: 'shipped', forks: [...shop, { project: 'cafe', called: 'Café', source: 'theirs' }] }));
  assert.deepEqual(chosenFirst(texts), ['here', 'saved']);
  assert.deepEqual(toggled(['here', 'saved'], 'fork-shop', texts), ['here', 'saved', 'fork-shop']);
  assert.deepEqual(toggled(['fork-shop', 'here'], 'default', texts), ['here', 'default', 'fork-shop']);
  assert.deepEqual(toggled(['here', 'saved', 'default', 'fork-shop'], 'fork-cafe', texts), ['here', 'saved', 'default', 'fork-shop']);
  assert.deepEqual(toggled(['here', 'saved'], 'here', texts), ['saved']);
  assert.equal(cannotPlay(['saved']), 'Choose two texts, or more: each is played in a sandbox of its own, against the same things done.');
  assert.equal(cannotPlay(['here', 'saved']), null);
});

test('what is played is each text as its source or as what ships, with the acts as each sandbox takes them', () => {
  const texts = offered(of({ ships: true, saved: 'saved' }));
  const asked = asking({
    name: 'std/board',
    project: 'shop',
    texts,
    first: [],
    params: { limit: 2 },
    told: {},
    answers: 'asked',
    acts: [
      { kind: 'answer', asked: 1, how: 'usual' },
      { kind: 'answer', asked: 2, how: 'fails', why: '' },
      { kind: 'answer', asked: 3, how: 'gives', gives: { reply: 'yes' } },
      { kind: 'say', topic: 'bell.rung', data: { by: 'ada' } },
      { kind: 'advance', by: '5m' },
    ],
  });
  assert.deepEqual(asked.texts, [
    { called: 'As it stands here', source: 'here' },
    { called: 'As it was saved', source: 'saved' },
    { called: 'The default', ships: true },
  ]);
  assert.deepEqual(asked.acts, [
    // As usual is each sandbox's to answer: only it knows what its stand-in usually answers.
    { answer: { asked: 1, usual: true } },
    { answer: { asked: 2, fails: 'It was told to fail.' } },
    { answer: { asked: 3, gives: { reply: 'yes' } } },
    { say: { topic: 'bell.rung', data: { by: 'ada' } } },
    { advance: { by: '5m' } },
  ]);
  assert.deepEqual(played({ kind: 'answers', answers: 'default' }), { answers: { answers: 'default' } });
  assert.equal(asked.project, 'shop');
  assert.equal(asking({ name: null, project: null, texts, first: [], params: {}, told: {}, answers: 'asked', acts: [] }).name, undefined);
});

const BESIDE: Beside = {
  columns: [
    { way: 'As it stands here', text: 'aaaaaaaa', version: '2', now: false, given: 1, reads: [] },
    { way: 'The default', text: 'bbbbbbbb', version: null, now: false, given: 1, reads: [] },
  ],
  rows: [
    { kind: 'came', what: 'bell.rung', by: [{ count: 1, errors: 0, avg: 0, sum: 0 }, null] },
    { kind: 'came', what: 'card.asked', by: [{ count: 2, errors: 0, avg: 60_000, sum: 120_000 }, { count: 1, errors: 0, avg: 0, sum: 0 }] },
    { kind: 'took', what: 'agent.task stood in', by: [{ count: 2, errors: 1, avg: 0, sum: 0 }, { count: 1, errors: 0, avg: 0, sum: 0 }] },
  ],
};

test('a column reads every row of the daemon, in its order, headed by kind once, with nothing where it has none', () => {
  const second = countsOf(BESIDE, 1);
  assert.deepEqual(
    second.map((row) => [row.head, row.what, row.figures]),
    [
      ['Came of them', 'bell.rung', null],
      [null, 'card.asked', ['1', '0ms']],
      ['Measured', 'agent.task stood in', ['1', '0ms']],
    ],
  );
  assert.deepEqual(countsOf(BESIDE, 0)[2]!.figures, ['2 (1 failed)', '0ms']);
  assert.equal(ranOf(BESIDE, 0), 'version 2 · aaaaaaaa');
  assert.equal(ranOf(BESIDE, 1), 'bbbbbbbb');
  assert.equal(allTheSame(BESIDE), false);
  assert.equal(allTheSame({ ...BESIDE, columns: BESIDE.columns.map((column) => ({ ...column, text: 'same' })) }), true);
});

test('the texts played are the ways of a comparison on real work, each by its word and as it was played', () => {
  const texts = offered(of({ ships: true, saved: 'saved' }));
  assert.deepEqual(asWays(texts, 'shipped'), [
    { word: 'here', called: 'As it stands here', source: 'here' },
    { word: 'saved', called: 'As it was saved', source: 'saved' },
    { word: 'default', called: 'The default', source: 'shipped' },
  ]);
  // What ships that the page has not read is no way.
  assert.equal(asWays(texts, null).length, 2);
});

test('what is played against each is said: what was done in the sandbox test here, and how stand-ins answer', () => {
  assert.match(playedSays([], 'default'), /nothing else is done.*answers at once/);
  assert.match(playedSays([{ kind: 'advance', by: '5m' }], 'asked'), /the one thing done.*waits to be answered/);
  assert.match(playedSays([{ kind: 'advance', by: '5m' }, { kind: 'advance', by: '5m' }], 'asked'), /the 2 things done/);
});

test("a blueprint's forks are every project's own file under its name, but the page's own", () => {
  const own = [
    { project: 'shop', name: 'std/board', file: 'blueprints/std/board.bp' },
    { project: 'cafe', name: 'std/board', file: 'blueprints/std/board.bp' },
    { project: 'cafe', name: 'std/cards', file: 'blueprints/std/cards.bp' },
  ];
  assert.deepEqual(forksOf(own, 'std/board', { project: 'cafe', path: null }), [
    { project: 'shop', file: 'blueprints/std/board.bp' },
    { project: 'cafe', file: 'blueprints/std/board.bp' },
  ]);
  assert.deepEqual(forksOf(own, 'std/board', { project: 'shop', path: 'blueprints/std/board.bp' }), [{ project: 'cafe', file: 'blueprints/std/board.bp' }]);
  assert.deepEqual(forksOf(own, null, { project: null, path: null }), []);
});

test('with one text, once everything is read, it says so and what would make a second, each a press only where the page can do it', () => {
  const own = offered(of({ saved: 'here' }));
  // Nothing is said of how many there are before the saved file, the default and the forks are read.
  assert.deepEqual(standing({ read: false, texts: own, file: true, changes: true, forks: false }), { reading: true });
  // A project's own blueprint, saved as it stands: changing it is what makes a second.
  const mine = standing({ read: true, texts: own, file: true, changes: true, forks: false });
  assert.ok('one' in mine);
  assert.equal(mine.says, 'There is one text of this blueprint to play: as it stands here, which is also as it was saved. Side by side plays the same things against two texts or more, each in a sandbox of its own.');
  assert.deepEqual(
    mine.seconds.map((second) => [second.word, second.called, second.then]),
    [['change', 'Change the text here', 'it is then played as it stands here beside as it was saved']],
  );
  assert.equal(mine.otherwise, null);
  // Somebody who may not write its file is offered nothing to press, and told what would.
  const read = standing({ read: true, texts: own, file: true, changes: false, forks: false });
  assert.ok('one' in read);
  assert.deepEqual(read.seconds, []);
  assert.equal(read.otherwise, "A second would be its text changed here, which whoever may write the project's files can do.");
  // What ships, with no fork anywhere: a fork, where the page offers Fork.
  const shipped = offered(of({ ships: true, isDefault: true }));
  const fresh = standing({ read: true, texts: shipped, file: false, changes: false, forks: true });
  assert.ok('one' in fresh);
  assert.equal(fresh.says, 'There is one text of this blueprint to play: the default. Side by side plays the same things against two texts or more, each in a sandbox of its own.');
  assert.deepEqual(
    fresh.seconds.map((second) => [second.word, second.called, second.then]),
    [['fork', 'Fork', 'the fork is then played beside the default']],
  );
  const unforkable = standing({ read: true, texts: shipped, file: false, changes: false, forks: false });
  assert.ok('one' in unforkable);
  assert.equal(unforkable.otherwise, "A second would be a fork of it, which whoever may write a project's files can make.");
  // A fork the same to the letter as the default, saved: both presses where the page has both.
  const same = offered(of({ ships: true, source: 'x', saved: 'x', shipped: 'x' }));
  const both = standing({ read: true, texts: same, file: true, changes: true, forks: true });
  assert.ok('one' in both);
  assert.deepEqual(both.seconds.map((second) => second.word), ['change', 'fork']);
  // Two texts or more: they are offered to be chosen, and nothing is said of a second.
  const two = offered(of({ saved: 'saved' }));
  assert.deepEqual(standing({ read: true, texts: two, file: true, changes: true, forks: false }), { several: two });
});

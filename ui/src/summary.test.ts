import assert from 'node:assert/strict';
import { test } from 'node:test';
import type { Series, Stats } from './api.ts';
import { costLabels, costOf, costSaid, everyLine, firstView, lineOf, pieces, startsAt } from './summary.ts';
import type { Summary, SummaryLine } from './summary.ts';

const stats = (count: number, errors = 0, p50 = 0, sum = 0): Stats => ({ count, errors, error_rate: count ? errors / count : 0, avg: count ? sum / count : 0, sum, min: 0, max: 0, p50, p95: p50 });
const series = (name: string, total: Stats, unit?: string): Series => ({ name, unit, labels: {}, total, points: [] });
const line = (text: string, more: Partial<SummaryLine> = {}): SummaryLine => ({ text, at: null, key: null, mark: null, under: [], ...more });

test('a blueprint that ships, and one somebody cannot change, opens on what it says in plain words', () => {
  assert.equal(startsAt({ running: false, changes: false }), 'summary');
  // Whoever can change it came to write: the text, as it always was.
  assert.equal(startsAt({ running: false, changes: true }), 'text');
  // Something running is shown by its graph, which says how its steps went, whoever looks.
  assert.equal(startsAt({ running: true, changes: false }), 'node');
  assert.equal(startsAt({ running: true, changes: true }), 'node');
  // What was last looked at there is what is shown, whichever it was.
  assert.equal(firstView('runs', { running: false, changes: false }), 'runs');
  assert.equal(firstView('text', { running: false, changes: false }), 'text');
  assert.equal(firstView(null, { running: false, changes: false }), 'summary');
  // Opened at a line of its text, it shows that line, whoever looks and whatever was last looked at.
  assert.equal(firstView(null, { running: false, changes: false, at: true }), 'text');
  assert.equal(firstView('runs', { running: false, changes: true, at: true }), 'text');
});

test('what a line sets apart as written is told from its own words', () => {
  assert.deepEqual(pieces('A table called `board`.'), [
    { text: 'A table called ', code: false },
    { text: 'board', code: true },
    { text: '.', code: false },
  ]);
  assert.deepEqual(pieces('`seen` and `last`'), [
    { text: 'seen', code: true },
    { text: ' and ', code: false },
    { text: 'last', code: true },
  ]);
  assert.deepEqual(pieces('Nothing set apart.'), [{ text: 'Nothing set apart.', code: false }]);
  // A mark with no partner is a mark, and nothing after it is taken for a name.
  assert.deepEqual(pieces('It costs a ` or so.'), [{ text: 'It costs a ` or so.', code: false }]);
  assert.deepEqual(pieces(''), []);
});

test('a line leads to the line of the text its step or handler is written on, and to nowhere where it is about the whole', () => {
  const graph = { nodes: [{ id: 'main/board', line: 12 }, { id: 'on.changed/@', line: 40 }] } as never;
  assert.equal(lineOf(graph, 'main/board'), 12);
  assert.equal(lineOf(graph, 'on.changed/@'), 40);
  assert.equal(lineOf(graph, 'main/gone'), null);
  assert.equal(lineOf(graph, null), null);
  assert.equal(lineOf(null, 'main/board'), null);
});

test('every line of a summary is found, those under a handler or a task among them', () => {
  const summary: Summary = {
    name: 'shop/hears',
    parts: [
      { kind: 'is', title: 'What it is', ordered: false, lines: [line('One.')] },
      { kind: 'hears', title: 'What it hears', ordered: false, lines: [line('When.', { under: [line('First.', { under: [line('Inner.')] }), line('Second.')] })] },
    ],
  };
  assert.deepEqual(everyLine(summary).map((one) => one.text), ['One.', 'When.', 'First.', 'Inner.', 'Second.']);
});

test('what a blueprint\'s runs have cost is counted from what was measured, and is nothing where nothing was', () => {
  assert.equal(costOf([]), null);
  // Times that are not a run's (its steps, what its handlers waited) are no part of it.
  assert.equal(costOf([series('step', stats(40, 2, 12)), series('handler.event', stats(9))]), null);
  assert.deepEqual(costOf([series('run', stats(12, 1, 1400)), series('step', stats(40, 2, 12))]), { runs: 12, failed: 1, typical: 1400, spent: [] });
  // Several series of runs (one a flow) are added up, and the one with the most says what is typical.
  assert.deepEqual(costOf([series('run', stats(2, 0, 9000)), series('run', stats(10, 3, 800))]), { runs: 12, failed: 3, typical: 800, spent: [] });
  // What says it is of something else than time is added up under its own name, whatever it is called.
  const spent = costOf([series('agent.output_tokens', stats(3, 0, 0, 5400), 'tokens'), series('run', stats(1, 0, 20)), series('agent.cost_usd', stats(3, 0, 0, 0.42), 'usd'), series('checkout.total_usd', stats(0, 0, 0, 0), 'usd')]);
  assert.deepEqual(spent, {
    runs: 1,
    failed: 0,
    typical: 20,
    spent: [
      { name: 'agent.cost_usd', unit: 'usd', total: 0.42, count: 3 },
      { name: 'agent.output_tokens', unit: 'tokens', total: 5400, count: 3 },
    ],
  });
  // And something measured with no run at all is still said.
  assert.equal(costOf([series('agent.cost_usd', stats(1, 0, 0, 1), 'usd')])?.runs, 0);
});

test('what its runs have cost is said as counts and amounts, with no word on whether they are good', () => {
  assert.deepEqual(costSaid({ runs: 12, failed: 1, typical: 1400, spent: [] }, 'the last 7 days'), ['It ran 12 times in the last 7 days; 1 failed.', 'A run typically took 1.4s.']);
  assert.deepEqual(costSaid({ runs: 1, failed: 0, typical: 20, spent: [] }, 'the last 7 days'), ['It ran once in the last 7 days; none failed.', 'The run took 20ms.']);
  assert.deepEqual(costSaid({ runs: 2400, failed: 1200, typical: 61_000, spent: [{ name: 'agent.output_tokens', unit: 'tokens', total: 5400, count: 3 }] }, 'the last 7 days'), [
    'It ran 2,400 times in the last 7 days; 1,200 failed.',
    'A run typically took 1m 01s.',
    'agent.output_tokens: 5,400 tokens over 3 measurements.',
  ]);
  const said = costSaid({ runs: 0, failed: 0, typical: 0, spent: [{ name: 'agent.cost_usd', unit: 'usd', total: 0.42, count: 1 }] }, 'the last 7 days');
  assert.equal(said.length, 1);
  assert.match(said[0]!, /^agent\.cost_usd: \$0\.42 over 1 measurement\.$/);
  for (const word of ['good', 'bad', 'well', 'poor', 'better', 'worse', 'score']) assert.ok(!costSaid({ runs: 9, failed: 9, typical: 5, spent: [] }, 'a week').join(' ').includes(word), word);
});

test('a blueprint\'s measurements are asked for by its name, and by its project where it is one\'s', () => {
  assert.equal(costLabels('std/board', 'shop'), 'blueprint:std/board,project:shop');
  assert.equal(costLabels('std/board', null), 'blueprint:std/board');
});

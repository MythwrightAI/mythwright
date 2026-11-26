import assert from 'node:assert/strict';
import { test } from 'node:test';
import type { Graph, GraphNode } from '../api.ts';
import { chainsOf, foldedGraph, foldName, foldTile, opensItself, ranOf, sharedStart, together } from './folds.ts';
import type { Ran } from './folds.ts';
import { layout } from './layout.ts';
import { tookSaid } from './tile.ts';

function node(id: string, layer: number, description: string | null = null): GraphNode {
  const [group, name] = id.split('/') as [string, string];
  return {
    id,
    group,
    name,
    kind: name === '@' ? 'trigger' : 'action',
    target: 'shell',
    layer,
    line: 1,
    end_line: 1,
    description,
    when: null,
    for_each: null,
    timeout: null,
    retry: null,
    device: null,
    measure: null,
    allow_failure: false,
    with: null,
    hooks: [],
    watch: [],
  };
}

/** A graph of steps in `main`, each `[key, layer]`, and its arrows `[from, to]` by key. */
function graph(steps: [string, number][], edges: [string, string][]): Graph {
  return {
    name: 'test',
    description: null,
    params: [],
    state: {},
    permissions: null,
    groups: [{ id: 'main', label: 'main', kind: 'main' }],
    nodes: steps.map(([key, layer]) => node(`main/${key}`, layer)),
    edges: edges.map(([from, to]) => ({ from: `main/${from}`, to: `main/${to}`, kind: 'needs' as const })),
  };
}

/**
 * The preview's run as its text has it: seventeen steps, the first five
 * one after another, the build leading both to its copy and to the
 * commits of its train, and the line it tells waiting on three.
 */
const PREVIEW = graph(
  [
    ['table', 0],
    ['began', 1],
    ['before', 2],
    ['cleared', 3],
    ['built', 4],
    ['copy', 5],
    ['serve', 6],
    ['up', 7],
    ['staged', 8],
    ['held', 9],
    ['given', 10],
    ['train', 5],
    ['ran', 6],
    ['simmed', 6],
    ['checks', 7],
    ['told', 11],
    ['kept', 12],
  ],
  [
    ['table', 'began'],
    ['began', 'before'],
    ['before', 'cleared'],
    ['cleared', 'built'],
    ['built', 'copy'],
    ['copy', 'serve'],
    ['serve', 'up'],
    ['up', 'staged'],
    ['up', 'held'],
    ['staged', 'held'],
    ['held', 'given'],
    ['built', 'train'],
    ['train', 'ran'],
    ['train', 'simmed'],
    ['ran', 'checks'],
    ['simmed', 'checks'],
    ['up', 'told'],
    ['given', 'told'],
    ['checks', 'told'],
    ['told', 'kept'],
  ],
);

test("the preview's run folds into its two long chains, and is drawn in eleven tiles over seven columns", () => {
  const folds = chainsOf(PREVIEW);
  assert.deepEqual(
    folds.map((fold) => fold.keys),
    [
      ['table', 'began', 'before', 'cleared', 'built'],
      ['copy', 'serve', 'up'],
    ],
  );
  const folded = foldedGraph(PREVIEW, folds, () => false);
  assert.equal(folded.graph.nodes.length, 11);
  const layers = Object.fromEntries(folded.graph.nodes.map((one) => [one.name, one.layer]));
  assert.deepEqual(layers, { table: 0, copy: 1, staged: 2, held: 3, given: 4, train: 1, ran: 2, simmed: 2, checks: 3, told: 5, kept: 6 });
  assert.equal(Math.max(...Object.values(layers)) + 1, 7);
  // Arrows into a fold go to it, those out of it come from it, and none is drawn twice.
  const arrows = folded.graph.edges.map((edge) => `${edge.from}>${edge.to}`);
  assert.ok(arrows.includes('fold:main/table>fold:main/copy') && arrows.includes('fold:main/table>main/train') && arrows.includes('fold:main/copy>main/told'));
  assert.equal(new Set(arrows).size, arrows.length);
  assert.ok(!arrows.some((arrow) => arrow.includes('main/began') || arrow.includes('main/serve')), 'no arrow to a step inside a shut fold');
  const placed = layout(folded.graph);
  assert.equal(Object.keys(placed.nodes).length, 11);
});

test('a fold unfolded is drawn as the graph always was', () => {
  const folds = chainsOf(PREVIEW);
  assert.equal(foldedGraph(PREVIEW, folds, () => true).graph, PREVIEW);
  // One open and one shut: the open one's steps are back in their own columns, and what follows the shut one moves left.
  const half = foldedGraph(PREVIEW, folds, (fold) => fold.keys[0] === 'table');
  const layers = Object.fromEntries(half.graph.nodes.map((one) => [one.name, one.layer]));
  assert.deepEqual([layers.table, layers.began, layers.built, layers.copy, layers.staged, layers.kept], [0, 1, 4, 5, 6, 10]);
  assert.deepEqual(Object.keys(half.shut), ['fold:main/copy']);
  assert.deepEqual(
    half.open.map((fold) => fold.id),
    ['fold:main/table'],
  );
});

test('which chains fold', () => {
  const line = (n: number) =>
    graph(
      Array.from({ length: n }, (_, at): [string, number] => [`s${at}`, at]),
      Array.from({ length: n - 1 }, (_, at): [string, string] => [`s${at}`, `s${at + 1}`]),
    );
  const cases: [string, Graph, string[][]][] = [
    ['a graph of a few steps, as it always was', line(9), []],
    ['a straight line of ten, as one fold', line(10), [['s0', 's1', 's2', 's3', 's4', 's5', 's6', 's7', 's8', 's9']]],
    [
      'two branches of a fork each a chain, and the steps where they part and meet in none',
      graph(
        [
          ['a', 0],
          ['b1', 1],
          ['b2', 2],
          ['b3', 3],
          ['c1', 1],
          ['c2', 2],
          ['c3', 3],
          ['c4', 4],
          ['d', 5],
          ['e', 6],
        ],
        [
          ['a', 'b1'],
          ['b1', 'b2'],
          ['b2', 'b3'],
          ['a', 'c1'],
          ['c1', 'c2'],
          ['c2', 'c3'],
          ['c3', 'c4'],
          ['b3', 'd'],
          ['c4', 'd'],
          ['d', 'e'],
        ],
      ),
      [
        ['b1', 'b2', 'b3'],
        ['c1', 'c2', 'c3', 'c4'],
      ],
    ],
    [
      'a chain of two, left as it is',
      graph(
        [
          ['a', 0],
          ['b', 1],
          ['c', 1],
          ['d', 1],
          ['e', 1],
          ['f', 1],
          ['g', 1],
          ['h', 1],
          ['i', 1],
          ['j', 2],
        ],
        [
          ['a', 'b'],
          ['a', 'c'],
          ['a', 'd'],
          ['a', 'e'],
          ['a', 'f'],
          ['a', 'g'],
          ['a', 'h'],
          ['i', 'j'],
        ],
      ),
      [],
    ],
  ];
  for (const [said, shape, folded] of cases)
    assert.deepEqual(
      chainsOf(shape).map((fold) => fold.keys),
      folded,
      said,
    );
});

test('what starts a handler stands in no fold', () => {
  const shape = graph(
    Array.from({ length: 10 }, (_, at): [string, number] => [at === 0 ? '@' : `s${at}`, at]),
    Array.from({ length: 9 }, (_, at): [string, string] => [at === 0 ? '@' : `s${at}`, `s${at + 1}`]),
  );
  assert.deepEqual(
    chainsOf(shape).map((fold) => fold.keys[0]),
    ['s1'],
  );
});

test('what a fold is called', () => {
  const said = (key: string, description: string | null = null) => ({ kind: 'action' as const, name: key, description });
  const cases: [string, string[], string][] = [
    ['build.compile build.link', ['build.compile', 'build.link'], 'build'],
    ['a task and its steps', ['build', 'build.compile', 'build.link'], 'build'],
    ['keys of words with dashes', ['deploy-push', 'deploy-wait', 'deploy-check'], 'deploy'],
    ['two words alike', ['set_up_a', 'set_up_b'], 'set_up'],
    ['nothing alike but a letter', ['table', 'tally'], ''],
    ['one key', ['table'], ''],
  ];
  for (const [what, keys, start] of cases) assert.equal(sharedStart(keys), start, what);
  assert.equal(foldName([said('table', 'The table each deploy is kept in. Made the first time.'), said('began')]), 'The table each deploy is kept in.');
  assert.equal(foldName([said('table'), said('began')]), 'table');
  assert.equal(foldName([said('build.compile', 'Compile it.'), said('build.link')]), 'build');
  const tile = foldTile({ name: 'Set up', keys: ['table', 'began', 'before', 'cleared', 'built'] });
  assert.deepEqual(tile.words, ['Set up']);
  assert.equal(tile.under, '5 steps · table → built');
});

test('how steps stand together', () => {
  const ok: Ran = { status: 'ok', started: 10, finished: 20 };
  const failed: Ran = { status: 'failed', started: 20, finished: 25 };
  const running: Ran = { status: 'running', started: 25, finished: null };
  const skipped: Ran = { status: 'skipped', started: 30, finished: 30 };
  const cancelled: Ran = { status: 'cancelled', started: 30, finished: 31 };
  const cases: [string, (Ran | undefined)[], boolean, Ran | undefined][] = [
    ['none begun', [undefined, undefined], true, undefined],
    ['all done', [ok, { ...ok, started: 20, finished: 40 }], false, { status: 'ok', started: 10, finished: 40 }],
    ['one failed, whatever else', [ok, failed, running], true, { status: 'failed', started: 10, finished: null }],
    ['one under way', [ok, running], true, { status: 'running', started: 10, finished: null }],
    ['between two steps while the run goes on', [ok, undefined], true, { status: 'running', started: 10, finished: null }],
    ['some never come to once the run is over', [ok, undefined], false, { status: 'cancelled', started: 10, finished: 20 }],
    ['one cancelled', [ok, cancelled], false, { status: 'cancelled', started: 10, finished: 31 }],
    ['all skipped', [skipped, skipped], false, { status: 'skipped', started: 30, finished: 30 }],
    ['some skipped, the rest done', [ok, skipped], false, { status: 'ok', started: 10, finished: 30 }],
  ];
  for (const [said, steps, going, stood] of cases) assert.deepEqual(together(steps, going), stood, said);
});

test("a run's steps by the node each is drawn on, a step of a list as its runs together", () => {
  const run = {
    flow: 'main',
    status: 'running',
    steps: [
      { step: 'build', status: 'ok', started: 1, finished: 5 },
      { step: 'build.compile', status: 'ok', started: 1, finished: 3 },
      { step: 'staged[0]', status: 'ok', started: 6, finished: 7 },
      { step: 'staged[1]', status: 'running', started: 7, finished: null },
    ],
  };
  assert.deepEqual(ranOf(run), {
    'main/build': { status: 'ok', started: 1, finished: 5 },
    'main/build.compile': { status: 'ok', started: 1, finished: 3 },
    'main/staged': { status: 'running', started: 6, finished: null },
  });
});

test('a fold opens itself where a step in it failed', () => {
  const fold = { members: ['main/a', 'main/b', 'main/c'] };
  const cases: [string, Record<string, Ran>, boolean][] = [
    ['nothing run', {}, false],
    ['all done', { 'main/a': { status: 'ok', started: 1, finished: 2 } }, false],
    ['one under way', { 'main/b': { status: 'running', started: 1, finished: null } }, false],
    ['one failed', { 'main/a': { status: 'ok', started: 1, finished: 2 }, 'main/b': { status: 'failed', started: 2, finished: 3 } }, true],
    ['a step outside it failed', { 'main/d': { status: 'failed', started: 2, finished: 3 } }, false],
  ];
  for (const [said, ran, opens] of cases) assert.equal(opensItself(fold, ran), opens, said);
});

test("what a run's tile says at its foot", () => {
  const took = (ms: number) => `${ms}ms`;
  const cases: [string, Ran | undefined, boolean, number, string][] = [
    ['not begun while the run goes on', undefined, true, 100, 'waiting'],
    ['never begun, the run over', undefined, false, 100, 'not run'],
    ['done', { status: 'ok', started: 10, finished: 40 }, false, 100, '30ms'],
    ['failed', { status: 'failed', started: 10, finished: 15 }, false, 100, '5ms'],
    ['under way, until now', { status: 'running', started: 10, finished: null }, true, 100, '90ms'],
    ['under way, a clock behind the daemon', { status: 'running', started: 10, finished: null }, true, 5, '0ms'],
    ['skipped', { status: 'skipped', started: 10, finished: 10 }, false, 100, 'skipped'],
    ['cancelled', { status: 'cancelled', started: 10, finished: 17 }, false, 100, 'cancelled'],
  ];
  for (const [said, ran, going, now, foot] of cases) assert.equal(tookSaid(ran, going, now, took), foot, said);
});

test("what follows a task stays past its steps' columns once a chain before it is folded", () => {
  const shape = graph(
    [
      ['s0', 0],
      ['s1', 1],
      ['s2', 2],
      ['build', 3],
      ['build.x', 4],
      ['build.y', 5],
      ['after', 6],
      ['t1', 7],
      ['t2', 8],
      ['t3', 9],
    ],
    [
      ['s0', 's1'],
      ['s1', 's2'],
      ['s2', 'build'],
      ['build.x', 'build.y'],
      ['build', 'after'],
      ['after', 't1'],
      ['t1', 't2'],
      ['t2', 't3'],
    ],
  );
  shape.nodes[3]!.kind = 'task';
  shape.edges.push({ from: 'main/build', to: 'main/build.x', kind: 'starts' });
  const folds = chainsOf(shape);
  assert.deepEqual(
    folds.map((fold) => fold.keys),
    [
      ['s0', 's1', 's2'],
      ['after', 't1', 't2', 't3'],
    ],
  );
  const folded = foldedGraph(shape, folds, (fold) => fold.keys[0] !== 's0');
  const layers = Object.fromEntries(folded.graph.nodes.map((one) => [one.name, one.layer]));
  assert.deepEqual(layers, { s0: 0, build: 1, 'build.x': 2, 'build.y': 3, after: 4, t1: 5, t2: 6, t3: 7 });
});

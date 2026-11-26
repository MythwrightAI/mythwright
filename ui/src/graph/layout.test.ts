import assert from 'node:assert/strict';
import { test } from 'node:test';
import type { Graph, GraphNode } from '../api.ts';
import { around, fit, fitBox, fitLeast, layout, NODE_H, NODE_W } from './layout.ts';

function node(id: string, layer: number): GraphNode {
  const [group, name] = id.split('/') as [string, string];
  return {
    id,
    group,
    name,
    kind: name === '@' ? 'trigger' : 'action',
    target: 'noop',
    layer,
    line: 1,
    end_line: 1,
    description: null,
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

function graph(nodes: GraphNode[], edges: [string, string][]): Graph {
  const groups = [...new Set(nodes.map((n) => n.group))].map((id) => ({ id, label: id, kind: id === 'main' ? 'main' : 'handler' }));
  return {
    name: 'test',
    description: null,
    params: [],
    state: {},
    permissions: null,
    groups,
    nodes,
    edges: edges.map(([from, to]) => ({ from, to, kind: 'needs' as const })),
  };
}

function overlaps(a: { x: number; y: number; w: number; h: number }, b: typeof a): boolean {
  return a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h;
}

test('a task and the steps inside it are drawn as a chain, each in a place of its own', () => {
  // As the daemon describes `build: { task: { compile, test } }` followed by `ship`.
  const task: Graph = graph(
    [node('main/build', 0), node('main/build.compile', 1), node('main/build.test', 2), node('main/ship', 3)],
    [
      ['main/build.compile', 'main/build.test'],
      ['main/build', 'main/ship'],
    ],
  );
  task.nodes[0]!.kind = 'task';
  task.edges.unshift({ from: 'main/build', to: 'main/build.compile', kind: 'starts' });
  const placed = layout(task);
  const boxes = task.nodes.map((one) => placed.nodes[one.id]!);
  assert.equal(boxes.filter(Boolean).length, 4, 'every node has a place');
  assert.ok(boxes.every((box, index) => index === 0 || boxes[index - 1]!.x < box.x), 'in the order they happen');
  assert.ok(boxes.every((box) => box.y === boxes[0]!.y), 'on one line');
  assert.equal(placed.edges.length, 3);
  assert.equal(placed.groups.length, 1, 'in the group the task is in');
});

test('a chain is drawn left to right on one line', () => {
  const placed = layout(
    graph(
      [node('main/a', 0), node('main/b', 1), node('main/c', 2)],
      [
        ['main/a', 'main/b'],
        ['main/b', 'main/c'],
      ],
    ),
  );
  const [a, b, c] = ['main/a', 'main/b', 'main/c'].map((id) => placed.nodes[id]!);
  assert.ok(a!.x < b!.x && b!.x < c!.x);
  assert.equal(a!.y, b!.y);
  assert.equal(b!.y, c!.y);
  assert.equal(placed.edges.length, 2);
  assert.ok(placed.edges[0]!.d.startsWith(`M${a!.x + NODE_W},${a!.y + NODE_H / 2} `));
});

test('nodes drawn taller are that tall, as far apart as ever, and their arrows leave from their middles', () => {
  const fan = graph(
    [node('main/start', 0), node('main/x', 1), node('main/y', 1)],
    [
      ['main/start', 'main/x'],
      ['main/start', 'main/y'],
    ],
  );
  const [usual, tall] = [layout(fan), layout(fan, NODE_H + 16)];
  const [x, y] = [tall.nodes['main/x']!, tall.nodes['main/y']!];
  assert.equal(usual.nodes['main/x']!.h, NODE_H, 'as tall as ever unless it is said');
  assert.equal(x.h, NODE_H + 16);
  assert.equal(y.y - (x.y + x.h), usual.nodes['main/y']!.y - (usual.nodes['main/x']!.y + NODE_H), 'the room between two is what it was');
  assert.equal(tall.height - usual.height, 2 * 16, 'and the group is taller by what its two rows grew');
  const start = tall.nodes['main/start']!;
  assert.ok(tall.edges[0]!.d.startsWith(`M${start.x + NODE_W},${start.y + (NODE_H + 16) / 2} `));
});

test('nodes that fan out share a column and never overlap', () => {
  const placed = layout(
    graph(
      [node('main/start', 0), node('main/x', 1), node('main/y', 1), node('main/z', 1), node('main/end', 2)],
      [
        ['main/start', 'main/x'],
        ['main/start', 'main/y'],
        ['main/start', 'main/z'],
        ['main/x', 'main/end'],
        ['main/y', 'main/end'],
        ['main/z', 'main/end'],
      ],
    ),
  );
  const boxes = Object.values(placed.nodes);
  for (let i = 0; i < boxes.length; i++) {
    for (let j = i + 1; j < boxes.length; j++) assert.ok(!overlaps(boxes[i]!, boxes[j]!), 'two nodes overlap');
  }
  assert.equal(placed.nodes['main/x']!.x, placed.nodes['main/y']!.x);
  assert.equal(new Set(['main/x', 'main/y', 'main/z'].map((id) => placed.nodes[id]!.y)).size, 3);
});

test('a node sits level with what leads to it', () => {
  const placed = layout(
    graph(
      [node('main/a', 0), node('main/b', 0), node('main/after_b', 1)],
      [['main/b', 'main/after_b']],
    ),
  );
  assert.equal(placed.nodes['main/after_b']!.y, placed.nodes['main/b']!.y);
});

test('groups are stacked, share a width, and contain their nodes', () => {
  const placed = layout(
    graph(
      [node('main/a', 0), node('main/b', 1), node('on.ready/@', 0), node('on.ready/go', 1), node('on.ready/done', 2)],
      [
        ['main/a', 'main/b'],
        ['on.ready/@', 'on.ready/go'],
        ['on.ready/go', 'on.ready/done'],
      ],
    ),
  );
  assert.equal(placed.groups.length, 2);
  const [main, handler] = placed.groups;
  assert.ok(main!.y + main!.h < handler!.y, 'the handler is below the main flow');
  assert.equal(main!.w, handler!.w);
  for (const [id, box] of Object.entries(placed.nodes)) {
    const group = placed.groups.find((g) => id.startsWith(`${g.id}/`))!;
    assert.ok(box.x >= group.x && box.x + box.w <= group.x + group.w, `${id} is inside its group across`);
    assert.ok(box.y >= group.y && box.y + box.h <= group.y + group.h, `${id} is inside its group down`);
  }
  assert.equal(placed.height, handler!.y + handler!.h);
});

test('an empty graph has no size and fitting it does not divide by zero', () => {
  const placed = layout(graph([], []));
  assert.deepEqual([placed.width, placed.height, placed.groups.length], [0, 0, 0]);
  assert.deepEqual(fit(placed, { width: 800, height: 600 }), { k: 1, x: 32, y: 32 });
});

test('fitting shrinks a large graph into view and never enlarges a small one much', () => {
  const large = fit({ width: 4000, height: 1000 }, { width: 1000, height: 600 });
  assert.ok(large.k < 0.25 && large.k >= 0.2);
  const small = fit({ width: 200, height: 100 }, { width: 1000, height: 600 });
  assert.equal(small.k, 1.25);
  assert.ok(small.x > 32, 'a small graph is centred');
});

test('what is followed is looked at by a box about it, never smaller than a tile and a half across and two high', () => {
  const placed = layout(graph([node('main/a', 0), node('main/b', 1), node('main/c', 2)], [['main/a', 'main/b'], ['main/b', 'main/c']]));
  const one = placed.nodes['main/b']!;
  const box = around(placed, ['main/b'])!;
  assert.deepEqual([box.w, box.h], [NODE_W * 1.5, NODE_H * 2]);
  // About its middle.
  assert.equal(box.x + box.w / 2, one.x + one.w / 2);
  assert.equal(box.y + box.h / 2, one.y + one.h / 2);
  // Several are held whole, and what the graph has not got is passed over.
  const all = around(placed, ['main/a', 'main/c', 'main/gone'])!;
  assert.ok(all.x <= placed.nodes['main/a']!.x && all.x + all.w >= placed.nodes['main/c']!.x + NODE_W);
  assert.equal(around(placed, ['main/gone']), null);
});

test('a box is fitted in the middle of what it is drawn in, as large as there is room for and never larger than the whole is drawn', () => {
  const view = fitBox({ x: 100, y: 50, w: 200, h: 100 }, { width: 400, height: 300 }, 0);
  assert.equal(view.k, 1.25);
  assert.equal(view.x + (100 + 100) * view.k, 200);
  assert.equal(view.y + (50 + 50) * view.k, 150);
  const small = fitBox({ x: 0, y: 0, w: 2000, h: 100 }, { width: 400, height: 300 }, 0);
  assert.equal(small.k, 0.2);
});

test('a graph that fits at the least it is drawn at is fitted whole, and one that does not is drawn at that from its start', () => {
  const viewport = { width: 1000, height: 600 };
  const cases: [string, { width: number; height: number }, { k: number; x: number; y: number }][] = [
    ['small enough to fit', { width: 600, height: 200 }, fit({ width: 600, height: 200 }, viewport)],
    ['too wide: from its left, at the least', { width: 3000, height: 200 }, { k: 0.75, x: 32, y: 64 }],
    ['too wide and too tall: from its top left', { width: 3000, height: 2000 }, { k: 0.75, x: 32, y: 32 }],
  ];
  for (const [said, size, fitted] of cases) assert.deepEqual(fitLeast(size, viewport, 0.75), fitted, said);
});

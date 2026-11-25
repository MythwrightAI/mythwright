import assert from 'node:assert/strict';
import { test } from 'node:test';
import type { Graph, GraphNode } from './api.ts';
import { frameAt, isOf, ONCE, stillOf } from './player.ts';
import type { Frame } from './player.ts';
import { endedAs, inTurn, LASTS, lineAbout, sections } from './preview.ts';
import type { Summary, SummaryLine, SummaryPart } from './summary.ts';

function node(id: string, layer: number, more: Partial<GraphNode> = {}): GraphNode {
  const [group, name] = [id.slice(0, id.lastIndexOf('/')), id.slice(id.lastIndexOf('/') + 1)];
  return {
    id,
    group,
    name: name === '@' ? group.replace(/^on\./, '') : name,
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
    ...more,
  };
}

function graph(nodes: GraphNode[]): Graph {
  const groups = [...new Set(nodes.map((one) => one.group))].map((id) => ({ id, label: id, kind: id === 'main' ? 'main' : 'handler' }));
  return { name: 'test', description: null, params: [], state: {}, permissions: null, groups, nodes, edges: [] };
}

const line = (text: string, at: string | null = null, under: SummaryLine[] = [], ending: string | null = null): SummaryLine => ({ text, at, key: null, mark: null, under, ending });
const part = (kind: string, lines: SummaryLine[]): SummaryPart => ({ kind, title: kind, ordered: kind === 'does', lines });
const summary = (...parts: SummaryPart[]): Summary => ({ name: 'test', parts });
/** What each frame lights, as `id:how` in order, for a case to read at a glance. */
const lit = (frames: Frame[]) => frames.map((one) => Object.entries(one.tiles).map(([id, how]) => `${id}:${how}`).sort().join(' '));

test('a straight chain is lit a step at a time, what was done kept lit as done, and then all of it done', () => {
  const chain = graph([node('main/a', 0), node('main/b', 1), node('main/c', 2)]);
  const [does] = sections(chain, summary(part('does', [line('A.', 'main/a'), line('B.', 'main/b'), line('C.', 'main/c')])));
  assert.equal(does!.kind, 'does');
  assert.deepEqual(lit(does!.frames), ['main/a:runs', 'main/a:done main/b:runs', 'main/a:done main/b:done main/c:runs', 'main/a:done main/b:done main/c:done']);
  // Each frame is of the line that says what is being done, and the last of none.
  assert.deepEqual(does!.frames.map((one) => one.line), [[0], [1], [2], null]);
  assert.deepEqual(does!.frames.map((one) => one.lasts), [LASTS.runs, LASTS.runs, LASTS.runs, LASTS.over]);
  // Where nothing is to move it is all of it done, of no line.
  assert.deepEqual(does!.still, { tiles: { 'main/a': 'done', 'main/b': 'done', 'main/c': 'done' }, line: null, says: null, lasts: 0 });
});

test('a diamond of waits lights the two that wait on the same step together, and the one that waits on both after them', () => {
  const diamond = graph([node('main/a', 0), node('main/b', 1), node('main/c', 1), node('main/d', 2)]);
  const frames = sections(diamond, summary(part('does', [line('A.', 'main/a'), line('B.', 'main/b'), line('C.', 'main/c'), line('D.', 'main/d')])))[0]!.frames;
  assert.deepEqual(lit(frames).slice(0, 3), ['main/a:runs', 'main/a:done main/b:runs main/c:runs', 'main/a:done main/b:done main/c:done main/d:runs']);
  // A frame of two steps is of the first line that says one of them.
  assert.deepEqual(frames[1]!.line, [1]);
});

test('a task is under way while the steps inside it are done, and the step after it waits for all of them', () => {
  const task = graph([node('main/build', 0, { kind: 'task' }), node('main/build.compile', 1), node('main/build.link', 2), node('main/ship', 3)]);
  const turns = inTurn(task, 'main');
  assert.deepEqual(
    turns.map((turn) => turn.running),
    [['main/build'], ['main/build', 'main/build.compile'], ['main/build', 'main/build.link'], ['main/ship']],
  );
  // The line under the task's is the one of the step inside it that is being done.
  const lines = [line('Build.', 'main/build', [line('Compile.', 'main/build.compile'), line('Link.', 'main/build.link')]), line('Ship.', 'main/ship')];
  assert.deepEqual(lineAbout(lines, turns[2]!.running), [0, 1]);
});

test('a handler with a condition has its event arrive, its condition asked as written, then its steps in turn', () => {
  const heard = graph([node('main/a', 0), node('on.moved/@', 0, { when: 'event.data.to == "done"' }), node('on.moved/tidy', 1), node('on.moved/say', 2)]);
  const hears = part('hears', [line('Its handlers take turns.'), line('When a card is moved.', 'on.moved/@', [line('Tidy.', 'on.moved/tidy'), line('Say.', 'on.moved/say')])]);
  const [, section] = sections(heard, summary(part('does', [line('A.', 'main/a')]), hears));
  assert.equal(section!.kind, 'hears');
  assert.deepEqual(lit(section!.frames), [
    'on.moved/@:arrives',
    'on.moved/@:asks',
    'on.moved/@:done on.moved/tidy:runs',
    'on.moved/@:done on.moved/say:runs on.moved/tidy:done',
    'on.moved/@:done on.moved/say:done on.moved/tidy:done',
  ]);
  assert.deepEqual(section!.frames[1]!.says, 'event.data.to == "done"');
  assert.deepEqual(section!.frames.map((one) => one.line), [[1], [1], [1, 0], [1, 1], [1]]);
  // A line that is no handler plays nothing; the section lights the handler's tiles alone.
  assert.deepEqual(Object.keys(section!.still.tiles).sort(), ['on.moved/@', 'on.moved/say', 'on.moved/tidy']);
});

test('a handler with no condition is not asked anything', () => {
  const heard = graph([node('on.tick/@', 0), node('on.tick/log', 1)]);
  const frames = sections(heard, summary(part('hears', [line('Every minute.', 'on.tick/@', [line('Log.', 'on.tick/log')])])))[0]!.frames;
  assert.deepEqual(lit(frames), ['on.tick/@:arrives', 'on.tick/@:done on.tick/log:runs', 'on.tick/@:done on.tick/log:done']);
  assert.ok(frames.every((one) => one.says === null));
});

test('what it does as it is stopped has everything it was doing lit as ended, and each thing it then does lit among its lines in turn', () => {
  const stopped = graph([node('main/a', 0), node('main/b', 1), node('on.x/@', 0)]);
  const ends = part('ends', [line('When it is stopped:', null, [line('It says it stopped.'), line('It writes it down.')], 'stop')]);
  const section = sections(stopped, summary(part('does', [line('A.', 'main/a'), line('B.', 'main/b')]), ends))[1]!;
  assert.equal(section.kind, 'ends');
  const all = 'main/a:ended main/b:ended on.x/@:ended';
  assert.deepEqual(lit(section.frames), [all, all, all]);
  assert.deepEqual(section.frames.map((one) => one.line), [[0], [0, 0], [0, 1]]);
});

test('a text with only handlers ends with them, and does nothing as it is applied', () => {
  const only = graph([node('on.a/@', 0), node('on.a/one', 1), node('on.b/@', 0), node('on.b/two', 1)]);
  const said = summary(
    part('hears', [line('When a.', 'on.a/@', [line('One.', 'on.a/one')]), line('When b.', 'on.b/@', [line('Two.', 'on.b/two')])]),
    part('ends', [line('When it is stopped:', null, [line('It says so.')], 'stop')]),
  );
  const [hears, ends] = sections(only, said);
  assert.deepEqual(lit(hears!.frames), ['on.a/@:arrives', 'on.a/@:done on.a/one:runs', 'on.a/@:done on.a/one:done', 'on.b/@:arrives', 'on.b/@:done on.b/two:runs', 'on.b/@:done on.b/two:done']);
  // Each handler is played by itself: the first is not lit while the second plays.
  assert.ok(!('on.a/@' in hears!.frames[3]!.tiles));
  assert.deepEqual(lit(ends!.frames)[0], 'on.a/@:ended on.a/one:ended on.b/@:ended on.b/two:ended');
  // Everything ends with it.
  assert.equal(Object.keys(ends!.still.tiles).length, 4);
});

test('what it is lights the whole of it, and another part each line about a tile in turn', () => {
  const made = graph([node('main/table', 0), node('main/look', 1)]);
  const said = summary(part('is', [line('Keeps a table.')]), part('makes', [line('A table called seen.', 'main/table')]), part('may', [line('It runs as whoever applies it.'), line('It runs a command.', 'main/look')]));
  const [is, makes, may] = sections(made, said);
  assert.deepEqual(lit(is!.frames), ['main/look:shown main/table:shown']);
  assert.deepEqual(lit(makes!.frames), ['main/table:shown']);
  assert.deepEqual(may!.frames.map((one) => one.line), [[1]]);
});

test('the sections are in the order the Summary is read, whatever order the daemon hands them in', () => {
  const said = summary(part('is', [line('It.')]), part('makes', [line('A table.')]), part('does', [line('A.', 'main/a')]), part('may', [line('It runs.')]));
  assert.deepEqual(sections(graph([node('main/a', 0)]), said).map((one) => one.kind), ['is', 'does', 'makes', 'may']);
});

test('an empty text plays nothing, and a text with no summary has no preview', () => {
  const empty = graph([]);
  const parts = sections(empty, summary(part('is', [line('It does nothing.')])));
  assert.equal(parts.length, 1);
  assert.deepEqual(parts[0]!.frames, []);
  assert.deepEqual(parts[0]!.still.tiles, {});
  assert.deepEqual(sections(empty, summary()), []);
  assert.deepEqual(sections(empty, null), []);
  assert.deepEqual(sections(null, summary(part('is', [line('It.')]))), []);
});

test('the player shows each frame for its own time, and round again from the first after a rest', () => {
  const frames = [{ lasts: 100 }, { lasts: 200 }, { lasts: 50 }];
  const round = { again: true, rest: 150 };
  assert.deepEqual([0, 99, 100, 299, 300, 349, 350, 499, 500, 600].map((at) => frameAt(frames, at, round)), [0, 0, 1, 1, 2, 2, 2, 2, 0, 1]);
  // Played once, as a replay of a run is, the last is kept.
  assert.equal(frameAt(frames, 10_000, ONCE), 2);
  assert.equal(frameAt([], 0, round), null);
  assert.equal(frameAt(frames, -5, round), 0);
});

test('what frames come to where nothing is to move, and whether a frame is of a line', () => {
  const frames: Frame[] = [
    { tiles: { a: 'arrives' }, line: [0], says: null, lasts: 1 },
    { tiles: { a: 'done', b: 'runs' }, line: [0, 1], says: 'x', lasts: 1 },
    { tiles: { b: 'done' }, line: null, says: null, lasts: 1 },
  ];
  assert.deepEqual(stillOf(frames), { tiles: { a: 'done', b: 'done' }, line: null, says: null, lasts: 0 });
  assert.ok(isOf(frames[1]!, [0]));
  assert.ok(isOf(frames[1]!, [0, 1]));
  assert.ok(!isOf(frames[1]!, [0, 0]));
  assert.ok(!isOf(frames[0]!, [0, 1]));
  assert.ok(!isOf(frames[2]!, [0]));
});

test('a step that says when it is done has that asked, as written, before it is done', () => {
  const asked = graph([node('main/a', 0), node('main/b', 1, { when: 'steps.a.out.ok' }), node('main/c', 1), node('main/d', 2)]);
  const frames = sections(asked, summary(part('does', [line('A.', 'main/a'), line('B.', 'main/b'), line('C.', 'main/c'), line('D.', 'main/d')])))[0]!.frames;
  assert.deepEqual(lit(frames), [
    'main/a:runs',
    'main/a:done main/b:asks',
    'main/a:done main/b:runs main/c:runs',
    'main/a:done main/b:done main/c:done main/d:runs',
    'main/a:done main/b:done main/c:done main/d:done',
  ]);
  assert.equal(frames[1]!.says, 'steps.a.out.ok');
  assert.deepEqual(frames[1]!.line, [1]);
  assert.equal(frames[1]!.lasts, LASTS.asks);
  assert.ok(frames.filter((_, at) => at !== 1).every((one) => one.says === null));
  // Inside a handler the same: its event, then the step's own condition, then the step.
  const heard = graph([node('on.x/@', 0), node('on.x/y', 1, { when: 'event.data.ok' })]);
  const hears = sections(heard, summary(part('hears', [line('When x.', 'on.x/@', [line('Y.', 'on.x/y')])])))[0]!.frames;
  assert.deepEqual(lit(hears), ['on.x/@:arrives', 'on.x/@:done on.x/y:asks', 'on.x/@:done on.x/y:runs', 'on.x/@:done on.x/y:done']);
  assert.deepEqual(hears[1]!.line, [0, 0]);
  assert.equal(hears[1]!.says, 'event.data.ok');
});

test('each ending is played as what it is', () => {
  const ending = graph([node('main/a', 0), node('main/b', 1), node('main/c', 1), node('on.x/@', 0), node('on.x/y', 1)]);
  const as = (kind: string | null) => Object.entries(endedAs(ending, kind)).map(([id, how]) => `${id}:${how}`).sort().join(' ');
  assert.equal(as('success'), 'main/a:done main/b:done main/c:done');
  // The step that failed is the last done, what it waited on done before it, and nothing after it.
  assert.equal(as('fail'), 'main/a:done main/b:failed');
  assert.equal(as('finish'), 'main/a:ended main/b:ended main/c:ended');
  assert.equal(as('update'), 'main/a:taken main/b:taken main/c:taken on.x/@:taken on.x/y:taken');
  assert.equal(as('stop'), 'main/a:ended main/b:ended main/c:ended on.x/@:ended on.x/y:ended');
  // Said nothing of, by a daemon from before: what it was doing, ended.
  assert.equal(as(null), 'main/a:ended main/b:ended main/c:ended');
  // A task fails with the step inside it that failed; one with no steps has nothing to fail.
  const task = graph([node('main/build', 0, { kind: 'task' }), node('main/build.compile', 1), node('main/build.link', 2)]);
  assert.deepEqual(endedAs(task, 'fail'), { 'main/build': 'failed', 'main/build.compile': 'done', 'main/build.link': 'failed' });
  assert.deepEqual(endedAs(graph([node('on.x/@', 0)]), 'fail'), {});
  // Each ending's frames light the graph as it leaves it, first of its line and then of each thing it does.
  const ends = part('ends', [line('When its steps fail:', null, [line('Say so.')], 'fail'), line('When it is stopped:', null, [line('Leave.')], 'stop')]);
  const frames = sections(ending, summary(ends))[0]!.frames;
  assert.deepEqual(frames.map((one) => one.line), [[0], [0, 0], [1], [1, 0]]);
  assert.equal(frames[1]!.tiles['main/b'], 'failed');
  assert.equal(frames[3]!.tiles['on.x/y'], 'ended');
});

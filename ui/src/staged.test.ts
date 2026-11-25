import assert from 'node:assert/strict';
import { test } from 'node:test';
import type { GraphNode, Step } from './api.ts';
import { childAt, childOf, childRun, failedAt, drawnOf, earlierAsked, earlierSaid, earlierThan, stageCalled, stageOf, stagesOf, stageTook, WINDOW } from './staged.ts';

const step = (name: string, status: Step['status'], started: number, finished: number | null, more: Partial<Step> = {}): Step => ({
  run: 'run_1',
  step: name,
  status,
  attempt: 1,
  tolerated: false,
  output: null,
  error: null,
  started,
  finished,
  ...more,
});

const node = (name: string, kind: GraphNode['kind'], layer: number, description: string | null = null, group = 'main') =>
  ({ id: `${group}/${name}`, group, name, kind, layer, description }) as GraphNode;

// What the check a run page is drawn for ran: a task of two, a lone step beside a command that failed and was let fail, and a call.
const steps = [
  step('build', 'ok', 100, 160),
  step('build.compile', 'ok', 100, 130),
  step('build.link', 'ok', 130, 160),
  step('tests', 'failed', 160, 200, { tolerated: true, error: 'exited with code 2: broken' }),
  step('lint', 'ok', 161, 170),
  step('deploy', 'ok', 200, 300, { output: { instance: 'in_child' } }),
];
const graph = {
  nodes: [
    node('build', 'task', 0, 'Build it. Every part.'),
    node('build.compile', 'action', 1),
    node('build.link', 'action', 2),
    node('lint', 'action', 3),
    node('tests', 'action', 3, 'Run the tests.'),
    node('deploy', 'call', 4),
    node('@', 'trigger', 0, null, 'on.pushed'),
  ],
};
const run = { flow: 'main', revision: 'r1', started: 100, finished: 300, status: 'ok', steps };

test('a step is in the stage of the step at the top of its flow, without which of a list it is', () => {
  assert.equal(stageOf('build'), 'build');
  assert.equal(stageOf('build.compile'), 'build');
  assert.equal(stageOf('each[2]'), 'each');
  assert.equal(stageOf('build[1].compile'), 'build');
});

test('a run of a text that is drawn is its stages in the order of the graph’s layers, each with what it says it is for', () => {
  const stages = stagesOf(run, graph, 1000);
  assert.deepEqual(
    stages.map((one) => [one.key, one.kind, one.status, one.layer]),
    [
      ['build', 'task', 'ok', 0],
      ['lint', 'step', 'ok', 3],
      ['tests', 'step', 'failed', 3],
      ['deploy', 'call', 'ok', 4],
    ],
  );
  assert.equal(stages[0]!.says, 'Build it.');
  assert.deepEqual(stages[0]!.inside.map((one) => one.step), ['build.compile', 'build.link']);
  assert.equal(stages[0]!.head?.step, 'build');
  // Stages of one layer run side by side: the second of them is said to be beside the one before.
  assert.deepEqual(stages.map((one) => one.beside), [false, false, true, false]);
  assert.equal(stages[2]!.tolerated, true);
  assert.equal(stages[3]!.child, 'in_child');
  assert.equal(stageCalled(stages[2]!), 'Run the tests.');
  assert.equal(stageCalled(stages[1]!), 'lint');
  // Of one layer in the order the text says them, however they happened to begin.
  const swapped = stagesOf({ ...run, steps: [...steps].reverse() }, graph, 1000);
  assert.deepEqual(swapped.map((one) => one.key), ['build', 'lint', 'tests', 'deploy']);
});

test('each stage has its share of the run’s time, and a stage under way is measured to now', () => {
  const stages = stagesOf(run, graph, 1000);
  assert.equal(stages[0]!.share, 60 / 200);
  assert.equal(stages[3]!.share, 100 / 200);
  const going = stagesOf({ ...run, finished: null, status: 'running', steps: [step('build', 'running', 100, null)] }, graph, 150);
  assert.equal(going[0]!.share, 1);
  assert.equal(stageTook(going[0]!, 150, (ms) => `${ms}ms`), '50ms so far');
  assert.equal(stageTook(stages[0]!, 1000, (ms) => `${ms}ms`), '60ms');
});

test('the stages a run has not come to yet are waiting while it goes on, and not run once it is over', () => {
  const begun = [step('build', 'running', 100, null), step('build.compile', 'running', 100, null)];
  const going = stagesOf({ ...run, status: 'running', finished: null, steps: begun }, graph, 150);
  assert.deepEqual(
    going.map((one) => [one.key, one.status, one.started]),
    [
      ['build', 'running', 100],
      ['lint', 'waiting', null],
      ['tests', 'waiting', null],
      ['deploy', 'waiting', null],
    ],
  );
  const over = stagesOf({ ...run, status: 'failed', steps: [step('build', 'failed', 100, 120)] }, graph, 150);
  assert.deepEqual(over.map((one) => one.status), ['failed', 'not run', 'not run', 'not run']);
});

test('a run of a text that cannot be had is shown in its stages by the names of its steps, in the order they began', () => {
  const stages = stagesOf(run, null, 1000);
  assert.deepEqual(
    stages.map((one) => [one.key, one.kind, one.says, one.layer]),
    [
      ['build', 'task', '', null],
      ['tests', 'step', '', null],
      ['lint', 'step', '', null],
      ['deploy', 'call', '', null],
    ],
  );
  assert.ok(stages.every((one) => !one.beside));
});

test('a list of steps with no row of its own stands as its steps do together', () => {
  const each = [step('each[0]', 'ok', 1, 2), step('each[1]', 'failed', 2, 3), step('each[2]', 'ok', 3, 4)];
  const [stage] = stagesOf({ ...run, steps: each, started: 1, finished: 4 }, null, 10);
  assert.deepEqual([stage!.key, stage!.status, stage!.tolerated, stage!.started, stage!.finished, stage!.inside.length], ['each', 'failed', false, 1, 4, 3]);
  const let_be = [step('each[0]', 'ok', 1, 2), step('each[1]', 'failed', 2, 3, { tolerated: true })];
  assert.deepEqual([stagesOf({ ...run, steps: let_be }, null, 10)[0]!.status, stagesOf({ ...run, steps: let_be }, null, 10)[0]!.tolerated], ['failed', true]);
  const going = [step('each[0]', 'ok', 1, 2), step('each[1]', 'running', 2, null)];
  const [under] = stagesOf({ ...run, steps: going }, null, 10);
  assert.deepEqual([under!.status, under!.finished], ['running', null]);
});

test('a handler’s run is staged by its own flow’s steps, and none of another flow is a stage of it', () => {
  const handled = stagesOf({ ...run, flow: 'on.pushed', steps: [step('say', 'ok', 1, 2)] }, graph, 10);
  assert.deepEqual(handled.map((one) => one.key), ['say']);
});

test('the child a step started is what its output says, and nothing that is no instance', () => {
  assert.equal(childOf({ output: { instance: 'in_4' } }), 'in_4');
  assert.equal(childOf({ output: { instance: 'run_4' } }), null);
  assert.equal(childOf({ output: 'in_4' }), null);
  assert.equal(childOf(null), null);
});

test('a call unfolds into its child’s own steps’ latest run since the stage began, and a child not said yet is the first made since', () => {
  const runs = [
    { id: 'a', flow: 'main', started: 50 },
    { id: 'b', flow: 'on.x', started: 300 },
    { id: 'c', flow: 'main', started: 210 },
  ];
  assert.equal(childRun(runs, 200)?.id, 'c');
  assert.equal(childRun(runs, null)?.id, 'c');
  assert.equal(childRun(runs, 5000), null);
  const children = [
    { id: 'old', created: 10 },
    { id: 'later', created: 250 },
    { id: 'first', created: 205 },
  ];
  assert.equal(childAt(children, 200), 'first');
  assert.equal(childAt(children, 300), null);
  assert.equal(childAt(children, null), null);
});

test('a step’s output is drawn from its end, no more than so many lines, and what came before is asked for by where it begins', () => {
  const lines = (from: number, to: number) => Array.from({ length: to - from + 1 }, (_, at) => ({ line: from + at }));
  assert.equal(drawnOf(lines(1, 300), WINDOW).length, 300);
  const long = drawnOf(lines(1, 20_000), WINDOW);
  assert.deepEqual([long.length, long[0]!.line, long.at(-1)!.line], [WINDOW, 20_000 - WINDOW + 1, 20_000]);
  assert.equal(earlierThan(lines(19_501, 20_000)), 19_500);
  assert.equal(earlierThan(lines(1, 300)), 0);
  assert.equal(earlierThan([]), 0);
  assert.deepEqual(earlierAsked(19_501, 500), { after: 19_000, limit: 500 });
  assert.deepEqual(earlierAsked(301, 500), { after: 0, limit: 300 });
  assert.equal(earlierAsked(1, 500), null);
  assert.equal(earlierSaid(19_500, 500), 'Show 500 lines earlier (19,000 lines before them)');
  assert.equal(earlierSaid(300, 500), 'Show the 300 lines before these');
  assert.equal(earlierSaid(1, 500), 'Show the line before these');
});

test('the stage that failed a run is the last that failed and was not let fail, and none of a run that did not fail', () => {
  const failing = [...steps.slice(0, 5), step('deploy', 'failed', 200, 300, { output: { instance: 'in_child' }, error: 'step `onward` failed: no' })];
  // A case met in use: a stage that failed, opened from the channel that said so, beside one let fail.
  assert.equal(failedAt({ status: 'failed' }, stagesOf({ ...run, status: 'failed', steps: failing }, graph, 1000)), 'deploy');
  assert.equal(failedAt({ status: 'ok' }, stagesOf(run, graph, 1000)), null);
  // Failed with only a stage that was let fail, or with none: nothing to mark.
  assert.equal(failedAt({ status: 'failed' }, stagesOf({ ...run, status: 'failed' }, graph, 1000)), null);
  assert.equal(failedAt({ status: 'failed' }, []), null);
});

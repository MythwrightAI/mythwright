import assert from 'node:assert/strict';
import { test } from 'node:test';
import { askedOf, asksFirst, asksTriggers, firstPipeline, isPipeline, LATEST, latestOf, othersOf, pipelineNamed, pipelinesOf, refusedOf, shortName } from './pipelines.ts';

const uses = {
  applied: ['std/fleet', 'pipelines/check', 'std/run', 'std/fleet'],
  own: [
    { name: 'pipelines/deploy', file: '@blueprints/pipelines/deploy.bp' },
    { name: 'release', file: '@blueprints/release.bp' },
    { name: 'pipelines/check', file: '@blueprints/pipelines/check.bp' },
    { name: 'pipelines/', file: '@blueprints/pipelines/.bp' },
  ],
};

test('a project’s pipelines are its own blueprints under pipelines/, by name, and nothing else it has', () => {
  assert.deepEqual(pipelinesOf(uses), [
    { name: 'pipelines/check', file: '@blueprints/pipelines/check.bp' },
    { name: 'pipelines/deploy', file: '@blueprints/pipelines/deploy.bp' },
  ]);
  assert.deepEqual(pipelinesOf(undefined), []);
  assert.equal(isPipeline('pipelines/check'), true);
  assert.equal(isPipeline('std/pipelines/check'), false);
  assert.equal(isPipeline('pipelines/'), false);
});

test('everything else that runs for a project is what is applied there that is no pipeline, each once', () => {
  assert.deepEqual(othersOf(uses), ['std/fleet', 'std/run']);
  assert.deepEqual(othersOf(undefined), []);
});

test('a pipeline’s row says what comes after the folder', () => {
  assert.equal(shortName('pipelines/check'), 'check');
  assert.equal(shortName('pipelines/ci/nightly'), 'ci/nightly');
  assert.equal(shortName('std/run'), 'std/run');
});

test('a new pipeline is named under pipelines/ whether or not that was written, by the rules every blueprint of a project’s own is', () => {
  assert.deepEqual(pipelineNamed('gates', '@blueprints', [], []), { name: 'pipelines/gates', path: '@blueprints/pipelines/gates.bp' });
  assert.deepEqual(pipelineNamed('pipelines/gates', '@blueprints', [], []), { name: 'pipelines/gates', path: '@blueprints/pipelines/gates.bp' });
  assert.deepEqual(pipelineNamed('ci/nightly.bp', '@blueprints', [], []), { name: 'pipelines/ci/nightly', path: '@blueprints/pipelines/ci/nightly.bp' });
  assert.deepEqual(pipelineNamed('  ', '@blueprints', [], []), { wrong: 'Give it a name.' });
  assert.ok('wrong' in pipelineNamed('gates', '@blueprints', ['pipelines/gates'], []));
  assert.ok('wrong' in pipelineNamed('two words', '@blueprints', [], []));
});

test('a new pipeline begins as one stage that checks and runs as it is', () => {
  const text = firstPipeline('pipelines/gates');
  assert.match(text, /^blueprint: pipelines\/gates\n/);
  assert.match(text, /\n {4}task:\n/);
  assert.doesNotMatch(text, /shell|run:/);
});

test('Run asks first where the pipeline is handed something or the project runs under an environment, and only then', () => {
  assert.equal(asksFirst(0, []), false);
  assert.equal(asksFirst(1, []), true);
  assert.equal(asksFirst(0, ['dev']), true);
});

test('a pipeline’s latest runs are its instances’ own steps’ runs, the newest first, so many at most', () => {
  const run = (id: string, started: number, flow = 'main') => ({ id, flow, status: 'ok', started });
  const marks = latestOf([[run('a', 1), run('h', 9, 'on.x')], [run('b', 5)], [], [run('c', 3)]]);
  assert.deepEqual(marks.map((one) => one.run), ['b', 'c', 'a']);
  const many = latestOf([Array.from({ length: LATEST + 3 }, (_, at) => run(`r${at}`, at))]);
  assert.equal(many.length, LATEST);
  assert.equal(many[0]!.run, `r${LATEST + 2}`);
});

test('Run asks the project’s triggers for a run where they are kept for it, and applies the pipeline itself only where they are not', () => {
  assert.equal(asksTriggers(uses), false);
  assert.equal(asksTriggers(undefined), false);
  assert.equal(asksTriggers({ ...uses, applied: [...uses.applied, 'std/pipeline'] }), true);
  assert.equal(asksTriggers({ ...uses, applied: ['std/pipeline-at'], own: [] }), false);
  assert.deepEqual(askedOf('shop', 'pipelines/check', { times: 2 }, 'a1'), { project: 'shop', pipeline: 'pipelines/check', with: { times: 2 }, asked: 'a1' });
});

test('a refusal the triggers say is of one asking alone, by what the press named it, and never of another press of the same person', () => {
  assert.equal(refusedOf({ asked: 'a1' }, 'a1'), true);
  assert.equal(refusedOf({ asked: 'a2' }, 'a1'), false);
  assert.equal(refusedOf({}, 'a1'), false);
  assert.equal(refusedOf(undefined, 'a1'), false);
  assert.equal(refusedOf({ asked: '' }, ''), false);
});

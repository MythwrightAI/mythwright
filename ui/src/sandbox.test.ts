import assert from 'node:assert/strict';
import { test } from 'node:test';
import type { Instance } from './api.ts';
import { actSays, answerRead, asksOf, called, differs, grouped, itself, lit, making, needsFirst, nth, outcome, paramsOf, sent, valuesOf } from './sandbox.ts';
import type { Asking, Line, Shown } from './sandbox.ts';
import type { Start } from './starts.ts';

const instance = (id: string, blueprint: string, more: Partial<Instance> = {}): Instance => ({
  id,
  blueprint,
  revision: 'r',
  params: {},
  state: {},
  status: 'active',
  parent: null,
  key: blueprint,
  target: null,
  project: 'sandbox',
  labels: {},
  holds: [],
  error: null,
  created: 0,
  updated: 0,
  ...more,
});

const asking = (id: number, more: Partial<Asking> = {}): Asking => ({
  id,
  action: 'agent.task',
  summary: 'Ask an agent.',
  instance: 'in_1',
  blueprint: 'asks',
  run: 'run_1',
  step: 'ask',
  asked: { prompt: 'Which way?' },
  usual: { available: false },
  waiting: true,
  ...more,
});

const shown = (more: Partial<Shown> = {}): Shown => ({
  sandbox: 'sandbox_1',
  said: 'This was a sandbox test.',
  blueprint: 'asks',
  project: 'sandbox',
  clock: 0,
  applying: false,
  waiting: [],
  over: null,
  stood: [],
  stood_said: 'Nothing was stood in.',
  does: [],
  instances: [instance('in_1', 'asks')],
  timeline: [],
  tables: [],
  channels: [],
  inbox: [],
  asked: [],
  somebody: 'user:acct_1',
  ...more,
});

const step = (run: string, key: string, status: string, more: Partial<Line> = {}): Line => ({ what: 'step', instance: 'in_1', run, flow: 'main', step: key, status, ...more });

test('what goes first is what the daemon says a start cannot be had without', () => {
  const start = (name: string, blueprint: string, needs: string[]): Start => ({ name, title: name, says: '', blueprint, on: true, needs, why: '' });
  // A fleet's start is a project's own text, `fleet`, which calls the one that ships.
  const starts = [start('board', 'std/board', []), start('fleet', 'fleet', ['board']), start('features', 'std/features', ['board', 'fleet'])];
  assert.deepEqual(needsFirst('fleet', starts), ['std/board']);
  assert.deepEqual(needsFirst('std/fleet', starts), ['std/board']);
  assert.deepEqual(needsFirst('std/features', starts), ['std/board', 'fleet']);
  // One of a folder of somebody's own is no start by being called the same.
  assert.deepEqual(needsFirst('shop/board', starts), []);
  assert.deepEqual(needsFirst('std/board', starts), []);
  // A text that is nobody's start needs nothing the daemon knows of.
  assert.deepEqual(needsFirst('release', starts), []);
  assert.deepEqual(needsFirst(null, starts), []);
});

test('each act is sent as the daemon takes it', () => {
  assert.deepEqual(sent({ kind: 'say', topic: 'bell.rung', data: { by: 'ada' } }, []), { body: { say: { topic: 'bell.rung', data: { by: 'ada' } } } });
  assert.deepEqual(sent({ kind: 'do', action: 'board.add', with: { title: 'Try it' } }, []), { body: { do: { action: 'board.add', with: { title: 'Try it' } } } });
  assert.deepEqual(sent({ kind: 'advance', by: '10m' }, []), { body: { advance: { by: '10m' } } });
  assert.deepEqual(sent({ kind: 'answers', answers: 'default' }, []), { body: { answers: { answers: 'default' } } });
  assert.deepEqual(making('blueprint: x', 'shop', ['std/board'], { a: 1 }, 'asked'), { source: 'blueprint: x', project: 'shop', first: ['std/board'], params: { a: 1 }, answers: 'asked' });
  assert.deepEqual(making('blueprint: x', null, [], {}, 'default'), { source: 'blueprint: x', first: [], params: {}, answers: 'default' });
});

test('a stand-in is answered three ways: as usual, that it fails, and by what was written', () => {
  const waiting = [asking(2, { usual: { code: 0 } })];
  assert.deepEqual(sent({ kind: 'answer', asked: 2, how: 'usual' }, waiting), { body: { answer: { asked: 2, gives: { code: 0 } } } });
  assert.deepEqual(sent({ kind: 'answer', asked: 2, how: 'fails', why: ' no room ' }, waiting), { body: { answer: { asked: 2, fails: 'no room' } } });
  assert.deepEqual(sent({ kind: 'answer', asked: 2, how: 'fails' }, waiting), { body: { answer: { asked: 2, fails: 'It was told to fail.' } } });
  assert.deepEqual(sent({ kind: 'answer', asked: 2, how: 'gives', gives: { reply: 'left' } }, waiting), { body: { answer: { asked: 2, gives: { reply: 'left' } } } });
});

test('an answer played against a run in which its stand-in does not wait is not sent, and says so', () => {
  const answer = sent({ kind: 'answer', asked: 3, how: 'usual' }, [asking(1)]);
  assert.ok('wrong' in answer && answer.wrong.includes('3rd thing asked'), JSON.stringify(answer));
  assert.deepEqual([1, 2, 3, 4, 11, 12, 13, 21, 22, 103].map(nth), ['1st', '2nd', '3rd', '4th', '11th', '12th', '13th', '21st', '22nd', '103rd']);
});

test('an act is said in a line', () => {
  assert.equal(actSays({ kind: 'say', topic: 'bell.rung', data: {} }), 'Said bell.rung');
  assert.equal(actSays({ kind: 'do', action: 'board.add', with: {} }), 'Did board.add');
  assert.equal(actSays({ kind: 'advance', by: '5m' }), 'Moved the clock on 5m');
  assert.equal(actSays({ kind: 'answers', answers: 'default' }), 'Had every stand-in answer as usual from here on');
  assert.equal(actSays({ kind: 'answer', asked: 1, how: 'fails' }), 'Answered the 1st stand-in: it fails');
});

test('an answer written out is read as JSON before it is sent, and one that does not read says why', () => {
  assert.deepEqual(answerRead('{ "reply": "left" }'), { value: { reply: 'left' } });
  assert.deepEqual(answerRead('  '), { value: {} });
  const wrong = answerRead('{ reply: left }');
  assert.ok('wrong' in wrong && wrong.wrong.startsWith('That does not read as JSON, so it was not sent'), JSON.stringify(wrong));
});

test('what is done by hand is asked from what its action says it takes, and no project is asked', () => {
  const action = {
    args: [
      { name: 'project', type: 'string', required: false, help: 'which project' },
      { name: 'title', type: 'string', required: true, help: 'what the card is called' },
      { name: 'priority', type: 'int', required: false, help: 'how soon' },
      { name: 'fields', type: 'object', required: false, help: 'what else it holds' },
      { name: 'quiet', type: 'bool', required: false, help: 'say nothing' },
    ],
  };
  const asks = asksOf(action);
  assert.deepEqual(
    asks.map((ask) => [ask.name, ask.box, ask.needed]),
    [
      ['title', 'text', true],
      ['priority', 'number', false],
      ['fields', 'json', false],
      ['quiet', 'bool', false],
    ],
  );
  // A box left empty says nothing; a number is one; a mapping is read, and what does not read is the words it is.
  assert.deepEqual(valuesOf(asks, { title: ' Try it ', priority: '2', fields: '{"labels":["a"]}', quiet: 'true' }), {
    values: { title: 'Try it', priority: 2, fields: { labels: ['a'] }, quiet: true },
  });
  assert.deepEqual(valuesOf(asks, { title: 'Try it', fields: 'plain words' }), { values: { title: 'Try it', fields: 'plain words' } });
  assert.deepEqual(valuesOf(asks, { priority: '2' }), { wrong: '`title` is needed.' });
  assert.deepEqual(valuesOf(asks, { title: 'x', priority: 'soon' }), { wrong: '`priority` is a number, and `soon` is not one.' });
});

test('an instance is called by its blueprint, and by its key where that is something else', () => {
  const all = [instance('in_1', 'std/board'), instance('in_2', 'std/cards', { key: 'cards' })];
  assert.equal(called(all, 'in_1'), 'std/board');
  assert.equal(called(all, 'in_2'), 'std/cards (cards)');
  assert.equal(called(all, 'in_9'), '');
});

test('the timeline is put under the run each line is of, a run named again when it comes after another', () => {
  const lines: Line[] = [
    { what: 'instance', did: 'started', instance: 'in_1' },
    { what: 'run', did: 'started', instance: 'in_1', run: 'run_1', flow: 'main' },
    { what: 'said', topic: 'collection.ensured', by: 'user:a', instance: 'in_1', run: 'run_1', flow: 'main' },
    step('run_1', 'board', 'ok'),
    { what: 'instance', did: 'started', instance: 'in_2' },
    { ...step('run_2', 'open', 'ok'), instance: 'in_2' },
    { what: 'said', topic: 'collection.history', by: 'system' },
    step('run_1', 'cards', 'ok'),
    // Said by hand: nobody's run.
    { what: 'said', topic: 'bell.rung', by: 'user:a' },
    { ...step('run_3', 'note', 'ok'), flow: 'on.rung' },
  ];
  const groups = grouped(lines);
  assert.deepEqual(
    groups.map((group) => [group.run, group.flow, group.lines.length]),
    [
      [null, null, 1],
      ['run_1', 'main', 4],
      ['run_2', 'main', 2],
      ['run_1', 'main', 1],
      [null, null, 1],
      ['run_3', 'on.rung', 1],
    ],
  );
});

test('the tiles of the text are lit by what its own instance did, each way told apart', () => {
  const lines: Line[] = [
    step('run_1', 'check', 'ok', { stood: [1] }),
    step('run_1', 'board', 'ok'),
    step('run_1', 'warn', 'skipped'),
    step('run_1', 'look', 'failed', { tolerated: true }),
    step('run_1', 'each[2]', 'failed'),
    step('run_1', 'last', 'running'),
    { what: 'run', did: 'started', instance: 'in_1', run: 'run_2', flow: 'on.rung', heard: 'bell.rung' },
    { ...step('run_2', 'note', 'ok'), flow: 'on.rung' },
    // Another instance's steps light nothing of this text.
    { ...step('run_9', 'board', 'failed'), instance: 'in_2' },
  ];
  assert.deepEqual(lit(lines, 'in_1'), {
    'main/check': 'stood',
    'main/board': 'ok',
    'main/warn': 'skipped',
    'main/look': 'let',
    'main/each': 'failed',
    'main/last': 'running',
    'on.rung/@': 'ok',
    'on.rung/note': 'ok',
  });
  assert.deepEqual(lit(lines, null), {});
});

test('the text itself is the last applied of its name with nothing above it', () => {
  const all = [instance('in_1', 'std/board'), instance('in_2', 'asks'), instance('in_3', 'asks', { parent: 'in_2' }), instance('in_4', 'asks')];
  assert.equal(itself({ instances: all, blueprint: 'asks' })?.id, 'in_4');
  assert.equal(itself({ instances: all, blueprint: 'gone' }), null);
});

test('how a test stands is said in a sentence: stopped, failed, waiting, up, or run to its end', () => {
  assert.deepEqual(outcome(shown({ over: 'this sandbox test was stopped' })), { how: 'bad', says: 'this sandbox test was stopped' });
  assert.equal(outcome(shown({ instances: [instance('in_1', 'asks', { status: 'failed', error: 'no board' })] })).says, 'asks failed: no board');
  assert.equal(outcome(shown({ waiting: [asking(1)] })).how, 'waits');
  assert.ok(outcome(shown({ waiting: [asking(1), asking(2)] })).says.startsWith('It waits: 2 stand-ins'));
  assert.deepEqual(outcome(shown()), { how: 'good', says: 'Its steps went well and it is up, listening for what it hears.' });
  assert.equal(outcome(shown({ instances: [instance('in_1', 'asks', { status: 'done' })] })).says, 'It ran to its end.');
  assert.equal(outcome(shown({ instances: [instance('in_1', 'asks', { status: 'starting' })] })).how, 'waits');
  assert.equal(outcome(shown({ instances: [] })).how, 'waits');
});

test('what differs between one run and the next is said step by step, table by table', () => {
  const before = shown({
    timeline: [step('run_1', 'build', 'ok', { stood: [1] }), step('run_1', 'note', 'ok'), step('run_1', 'old', 'ok')],
    tables: [{ name: 'board', records: [{ id: 'sandbox-1', data: {} }] }],
    stood: [{ action: 'shell', times: 1 }],
  });
  const now = shown({
    timeline: [step('run_1', 'build', 'failed'), step('run_1', 'note', 'ok'), step('run_1', 'tell', 'ok')],
    tables: [
      { name: 'board', records: [] },
      { name: 'notes', records: [{ id: 'sandbox-notes-1', data: {} }] },
    ],
    stood: [{ action: 'shell', times: 2 }],
    instances: [instance('in_1', 'asks', { status: 'failed', error: 'the build failed' })],
  });
  assert.deepEqual(differs(before, now), [
    '`build` of asks was stood in the time before, and is failed now.',
    '`tell` of asks ran this time (ok), and did not the time before.',
    '`old` of asks ran the time before, and did not this time.',
    'The table `board` had 1 row before, and has 0 now.',
    'The table `notes` is there now, with 1 row, and was not before.',
    '`shell` was stood in 1 time before, and 2 times now.',
    'Before: Its steps went well and it is up, listening for what it hears. Now: asks failed: the build failed',
  ]);
  assert.deepEqual(differs(before, before), ['Nothing differs from the run before: every step came out as it did, and the tables hold as many rows.']);
});

test('what a text is told is read from its boxes by the kind of each, with which project left to the daemon', () => {
  const params = [
    { name: 'project', type: 'string', required: true },
    { name: 'limit', type: 'int', required: false },
    { name: 'tone', type: 'string', required: false },
    { name: 'loud', type: 'bool', required: false },
    { name: 'shape', type: 'any', required: false },
  ];
  assert.deepEqual(paramsOf(params, { limit: '2', tone: 'calm', loud: 'true', shape: 'round' }), { params: { limit: 2, tone: 'calm', loud: true, shape: 'round' } });
  assert.deepEqual(paramsOf(params, {}), { params: {} });
  assert.deepEqual(paramsOf(params, { limit: 'many' }), { wrong: '`limit` should be a int.' });
  assert.deepEqual(paramsOf([{ name: 'need', type: 'string', required: true }], {}), { wrong: '`need` is needed: say it under What it is told.' });
});

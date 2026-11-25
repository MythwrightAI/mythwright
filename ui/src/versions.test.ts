import assert from 'node:assert/strict';
import { test } from 'node:test';
import type { Behind, Fields, Version } from './api.ts';
import { applied, called, changes, lag, moved, nth, running, saidVersion } from './versions.ts';

const version = (n: number, params: Fields, revision = 'r1'): Version => ({ version: n, revision, source: '', params, by: 'user:a', by_name: 'Ada', applied: n, outcome: 'applied' });
const COLUMNS = ['backlog', 'ready', 'doing', 'review', 'done'];

test('the first version is what there was to start with', () => {
  assert.deepEqual(changes(undefined, version(1, { columns: COLUMNS })), ['what it was started with']);
});

test('a list of words says which came and which went', () => {
  const before = version(1, { columns: COLUMNS, store: 'local' });
  assert.deepEqual(changes(before, version(2, { columns: [...COLUMNS, 'blocked'], store: 'local' })), ['columns: added blocked']);
  assert.deepEqual(changes(before, version(2, { columns: ['backlog', 'ready', 'doing', 'done', 'shipped'], store: 'local' })), ['columns: added shipped; took out review']);
  assert.deepEqual(changes(before, version(2, { columns: ['ready', 'backlog', 'doing', 'review', 'done'], store: 'local' })), ['columns: put in another order']);
});

test('anything else says what it was and what it is', () => {
  const before = version(1, { port: 8080, store: 'local' });
  assert.deepEqual(changes(before, version(2, { port: 9090, store: 'files' })), ['port: 8080 → 9090', 'store: local → files']);
  assert.deepEqual(changes(before, version(2, { store: 'local', owner: 'ada' })), ['owner: ada', 'port: no longer given']);
});

test('a change to the text is said, with whatever else changed', () => {
  assert.deepEqual(changes(version(1, { port: 1 }), version(2, { port: 2 }, 'r2')), ['its text changed', 'port: 1 → 2']);
});

test('applied again with nothing changed says so', () => {
  assert.deepEqual(changes(version(1, { port: 1 }), version(2, { port: 1 })), ['the same again']);
});

test('a long value is cut short', () => {
  const [said] = changes(version(1, { note: 'a' }), version(2, { note: 'x'.repeat(80) }));
  assert.ok(said!.length < 70 && said!.endsWith('…'), said);
});

test('the text an instance runs is said by the start of its revision and the version the text gives itself', () => {
  assert.equal(running({ revision: '0ac2eb479e5347be', version: '3' }), '0ac2eb47 · version 3');
  assert.equal(running({ revision: '0ac2eb479e5347be', version: null }), '0ac2eb47');
  assert.equal(running({ revision: '0ac2eb479e5347be' }), '0ac2eb47', 'a daemon from before it said the version');
});

const behind = (takes: Behind['takes'], why = 'Because.'): Behind => ({ revision: 'd8608bab080ae477', version: '4', takes, why });

test('an instance that runs the text now there is not marked', () => {
  assert.equal(lag({ behind: null }), null);
  assert.equal(lag({}), null);
});

test('an instance that is behind says so, what it is behind, and why', () => {
  assert.deepEqual(lag({ behind: behind('in_place', 'Only what it does later has changed.') }), {
    mark: 'behind',
    there: 'd8608bab · version 4',
    why: 'Only what it does later has changed.',
    moved: null,
    leave: 'Or leave it: it goes on with the text it has, whatever is installed, until somebody applies it.',
    again: 'Apply it again (it takes the text where it is: nothing runs again)',
  });
});

test('what moved between the text it runs and the one now there is said by version first', () => {
  const one = { blueprint: 'std/lane-round', version: '3', behind: { ...behind('again'), changed: ['the param `asking` is new', 'the step `work` is changed'] } };
  assert.equal(lag(one)!.moved, 'std/lane-round 3 to 4: the param `asking` is new; the step `work` is changed');
  assert.equal(moved({ name: 'worker', was: null, now: '1', changed: ['the step `claim` is changed'] }), 'worker, now version 1: the step `claim` is changed');
  assert.equal(moved({ name: 'worker', was: null, now: '1', changed: [] }), 'worker, now version 1');
  assert.equal(moved({ name: 'worker', was: '2', now: '3' }), 'worker 2 to 3');
});

test('applying it again is offered only where that is what brings it up to date, and says whether its steps run again', () => {
  assert.equal(lag({ behind: behind('again') })!.again, 'Apply it again (its steps run again)');
  const carried = lag({ behind: behind('carried', 'The text now there says how one that is running takes it.') })!;
  assert.equal(carried.again, 'Apply it again (it is carried over: its steps do not run again)', 'a text that says `on_update` is taken that way');
  assert.equal(carried.mark, 'behind');
  assert.equal(carried.why, 'The text now there says how one that is running takes it.');
  assert.equal(lag({ behind: behind('in_place') })!.again, 'Apply it again (it takes the text where it is: nothing runs again)', 'one that only does otherwise later can be brought up to date from here too');
  const unfit = lag({ behind: behind('unfit') })!;
  assert.equal(unfit.again, null, 'a text that does not check cannot be applied');
  assert.equal(unfit.mark, 'behind: the text there does not check');
});

test('once it has been applied again it is said how it came to the text now there', () => {
  assert.equal(applied('std/working', 'working-ready', 'in_place', true), 'Applied std/working again: working-ready runs the text now there (it took the text where it is, and nothing ran again).');
  assert.equal(applied('std/working', 'working-ready', 'carried', true), 'Applied std/working again: working-ready runs the text now there (it was carried over, and its steps did not run again).');
  assert.equal(applied('std/working', 'working-ready', 'again', true), 'Applied std/working again: working-ready runs the text now there (its steps ran again).');
  assert.equal(applied('std/working', 'working-ready', undefined, true), 'Applied std/working again: working-ready runs the text now there.', 'a daemon from before it said how');
  assert.equal(applied('std/working', 'working-ready', undefined, false), 'working-ready is already running that.');
});

test('the version a blueprint says of itself is read from its text, and only where the text itself says one', () => {
  assert.equal(saidVersion('blueprint: std/board\nversion: 3\ndescription: A board.\n'), '3');
  assert.equal(saidVersion('blueprint: deploy\nversion: "2.1"  # the second\n'), '2.1');
  assert.equal(saidVersion("blueprint: deploy\nversion: 'spring'\n"), 'spring');
  assert.equal(saidVersion('blueprint: deploy\nversion: 4 # bumped\n'), '4');
  assert.equal(saidVersion('blueprint: deploy\nparams:\n  version: { type: string }\nsteps:\n  go:\n    with:\n      version: 9\n'), null, 'a param or an argument called version is not the text\'s own');
  assert.equal(saidVersion('blueprint: deploy\nversion:\n'), null);
  assert.equal(saidVersion(''), null);
});

test('which time something was applied is said in a word while there is one', () => {
  assert.deepEqual([1, 2, 3, 12].map(nth), ['first', 'second', 'third', 'twelfth']);
  assert.deepEqual([13, 21, 22, 23, 101, 111].map(nth), ['13th', '21st', '22nd', '23rd', '101st', '111th']);
});

test('one of the definitions an instance was given is called by its blueprint\'s version first, and by which time it was applied, and never by the one word for both', () => {
  const second = called({ version: 2, source: 'blueprint: std/board\nversion: 3\n' });
  assert.deepEqual([second.head, second.under], ['Blueprint version 3', 'second time applied']);
  assert.equal(second.whole, 'blueprint version 3, the second time it was applied');
  // The same text applied again with something else is the same version of the blueprint, another time.
  assert.equal(called({ version: 3, source: 'blueprint: std/board\nversion: 3\n' }).head, 'Blueprint version 3');
  assert.equal(called({ version: 3, source: 'blueprint: std/board\nversion: 3\n' }).under, 'third time applied');
  // A text that says no version of itself is called by which time it was applied, and no version is made up for it.
  const plain = called({ version: 1, source: 'blueprint: greeter\n' });
  assert.deepEqual([plain.version, plain.head, plain.under, plain.whole], [null, 'First time applied', null, 'the first time it was applied']);
  for (const one of [second, plain]) assert.ok(!/version \d+ of|^Version \d/.test(`${one.head} ${one.under ?? ''}`), 'the count is never called a version');
});

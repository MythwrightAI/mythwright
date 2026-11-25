import assert from 'node:assert/strict';
import { test } from 'node:test';
import type { SummaryLine, SummaryPart } from './summary.ts';
import { MOST_ROWS, dataModel, fieldsSaid, foldsRows, madeOf, modelFold, namedBy, rows, startedFrom, wordy, foldKey, folds, fromOf, handlers, headOf, isFact, isHandler, ownLines, readOrder, sharedFrom } from './summarylaid.ts';

const line = (text: string, more: Partial<SummaryLine> = {}): SummaryLine => ({ text, at: null, key: null, mark: null, under: [], ...more });
const part = (kind: string, lines: SummaryLine[]): SummaryPart => ({ kind, title: kind, ordered: kind === 'does', lines });
const under = (count: number) => Array.from({ length: count }, (_, at) => line(`Thing ${at}.`));
const FACTORY = 'Heard from its own project, and from the factory itself.';
const OWN = 'Heard only from itself and what it started.';
const heard = (name: string, from: string | null) => line(`When ${name}.`, { at: `on.${name}/@`, key: name, title: `\`${name}\``, from, under: under(2) });

test("a summary's head is where its text stands first, what it says it is, and its facts from every part", () => {
  const from = line('Forked from `std/board`.', { opens: { what: 'default', name: 'std/board' } });
  const said = line('A board of cards.');
  const version = line('This is version 3 of it.', { label: 'Version', title: '3' });
  const told = line('Whoever applies it may tell it 2 things:', { under: under(2) });
  const partOf = line('A part of `std/fleet`.', { opens: { what: 'default', name: 'std/fleet' } });
  const permissions = line('Of what they can do, it uses only what it lists: `board`.', { label: 'Permissions', items: ['`board`'] });
  const command = line('It runs a command: version.', { mark: 'command', at: 'main/look', key: 'look' });
  const secret = line('It hands the secret `KEY` to what it runs.', { mark: 'secret' });
  const runs = line('It runs as whoever applies it.');
  const [is, may] = [part('is', [from, said, version, partOf, told]), part('may', [runs, permissions, command, secret, command])];
  const parts = [is, may];
  const head = headOf(parts);
  assert.deepEqual(head.standing, [from, partOf]);
  assert.equal(head.lead, said);
  assert.deepEqual(head.facts, [{ label: 'Version', lines: [version] }, { label: 'Permissions', lines: [permissions] }]);
  assert.deepEqual(head.rest, [told]);
  // Each kind of mark once, in the order first met.
  assert.deepEqual(head.marks, ['command', 'secret']);
  // A fact is drawn in the head and not where its part is; the head's part draws nothing of its own.
  assert.deepEqual(ownLines(may), [runs, command, secret, command]);
  assert.deepEqual(ownLines(is), []);
  // A step is no fact; a thing it makes is a row of its own part, and never a fact of the head.
  assert.equal(isFact(line('Do it.', { at: 'main/do', title: 'Do it.' })), false);
  const [shelf, bin] = [line('A thing called `shelf`.', { at: 'main/shelf', key: 'shelf', label: 'Makes', title: 'a thing called `shelf`' }), line('A thing called `bin`.', { at: 'main/bin', key: 'bin', label: 'Makes', title: 'a thing called `bin`' })];
  const made = part('makes', [shelf, line('Another blueprint, `shop/helper`.', { at: 'main/helper' }), bin]);
  assert.deepEqual(headOf([made]).facts, []);
  assert.deepEqual(ownLines(made), made.lines);
  assert.deepEqual(headOf([made, ...parts]).facts.map((one) => one.label), ['Version', 'Permissions']);
  // Said in words of their own they are a list a line each; a version, or names alone, are not.
  assert.equal(wordy({ label: 'Makes', lines: [shelf] }), true);
  assert.equal(wordy({ label: 'Version', lines: [version] }), false);
  assert.equal(wordy({ label: 'Permissions', lines: [permissions] }), false);
  assert.equal(wordy({ label: 'After a restart', lines: [line('x', { label: 'After a restart', title: 'Taken up where it was' })] }), true);
});

test('a text that says nothing of what it is has no lead, and one with no part `is` a head of its facts alone', () => {
  const params = line('Whoever applies it may tell it 1 thing:', { under: under(1) });
  const named = line('Its text does not say.', { key: 'project' });
  assert.equal(headOf([part('is', [params, named])]).lead, null);
  assert.deepEqual(headOf([part('is', [params, named])]).rest, [params, named]);
  const only = headOf([part('may', [line('Read things.', { key: 'things:read' })])]);
  assert.deepEqual([only.standing, only.lead, only.facts, only.rest, only.marks], [[], null, [], [], []]);
});

test('the parts are read in the order a reader needs them, and a kind not named comes after them, before how to change it', () => {
  const kinds = ['is', 'makes', 'does', 'hears', 'may', 'ends', 'change'];
  assert.deepEqual(readOrder(kinds.map((kind) => part(kind, []))).map((one) => one.kind), ['is', 'does', 'hears', 'ends', 'makes', 'may', 'change']);
  const fork = ['is', 'differs', 'moved', 'makes', 'does', 'may', 'back'];
  assert.deepEqual(readOrder(fork.map((kind) => part(kind, []))).map((one) => one.kind), ['is', 'differs', 'moved', 'does', 'makes', 'may', 'back']);
  const unknown = ['is', 'later', 'does', 'other', 'change'];
  assert.deepEqual(readOrder(unknown.map((kind) => part(kind, []))).map((one) => one.kind), ['is', 'does', 'later', 'other', 'change']);
});

test('where most handlers hear from is said once, and a handler that hears from elsewhere says so itself', () => {
  const [a, b, c, d] = [heard('a', FACTORY), heard('b', OWN), heard('c', FACTORY), heard('d', null)];
  const lines = [line('Its handlers take turns.'), a, b, c, d];
  assert.equal(sharedFrom(lines), FACTORY);
  assert.equal(fromOf(a, FACTORY), null);
  assert.equal(fromOf(b, FACTORY), OWN);
  assert.equal(fromOf(d, FACTORY), null);
  assert.equal(fromOf(c, FACTORY), null);
  // One handler alone, or none saying the same twice: each says its own.
  assert.equal(sharedFrom([heard('a', FACTORY)]), null);
  assert.equal(sharedFrom([heard('a', FACTORY), heard('b', OWN)]), null);
  assert.equal(fromOf(heard('a', FACTORY), null), FACTORY);
  // Of two said as often, the first met.
  assert.equal(sharedFrom([heard('a', OWN), heard('b', FACTORY), heard('c', FACTORY), heard('d', OWN)]), OWN);
  // A line that is no handler is not counted, whatever it says.
  assert.equal(sharedFrom([line('x', { from: OWN }), line('y', { from: OWN }), heard('a', FACTORY)]), null);
  assert.equal(handlers(part('hears', lines)), 4);
  assert.equal(isHandler(line('x', { at: 'main/x' })), false);
});

test("a handler's steps are folded away, and so is a long list under a line about the whole; a task's steps are not", () => {
  assert.equal(folds(line('When this happens.', { at: 'on.changed/@', under: under(1) })), true);
  assert.equal(folds(line('When this happens.', { at: 'on.changed/@' })), false);
  assert.equal(folds(line('It may be told 12 things:', { under: under(12) })), true);
  assert.equal(folds(line('It may be told 5 things:', { under: under(5) })), true);
  // A few are read where they are.
  assert.equal(folds(line('It may be told 4 things:', { under: under(4) })), false);
  assert.equal(folds(line('Nothing under it.')), false);
  assert.equal(folds(line('Do these as one.', { at: 'main/build', under: under(9) })), false);
  // A fold is known by what the graph calls its line, and otherwise by its part and place.
  assert.equal(foldKey('hears', heard('a', FACTORY), 3), 'on.a/@');
  assert.equal(foldKey('is', line('It may be told 12 things:', { under: under(12) }), 2), 'is:2');
});

test('what a blueprint starts the same way from several places is one row, and anything else a row alone', () => {
  const due = (by: string, at: string) => line(`Another blueprint, \`std/due\`, run under it as \`look\` and waited for. Each time its handler \`${by}\` comes to it.`, { at, key: 'look', title: '`std/due`', named: '`look`', tags: ['waited for'], by, opens: { what: 'blueprint', name: 'std/due' } });
  const [first, staffed, listening] = [due('first', 'on.first/look'), due('staffed', 'on.staffed/look'), due('listening', 'on.listening/look')];
  const looked = line('Another blueprint, `std/looked`.', { at: 'on.looked/wrong', key: 'wrong', title: '`std/looked`', named: '`looked`', tags: ['waited for'], by: 'looked', opens: { what: 'blueprint', name: 'std/looked' } });
  const left = { ...first, tags: ['left running'], at: 'main/look', by: null };
  const plain = line('Another blueprint, `std/cards`.', { at: 'main/cards' });
  const laid = rows([looked, first, plain, staffed, left, listening]);
  assert.deepEqual(laid.map((row) => row.lines.map((one) => one.at)), [['on.looked/wrong'], ['on.first/look', 'on.staffed/look', 'on.listening/look'], ['main/cards'], ['main/look']]);
  assert.equal(laid[1]!.line, first);
  // Under another name, or for each of something, it is another row.
  assert.equal(rows([first, { ...staffed, named: '`other`' }]).length, 2);
  assert.equal(rows([first, { ...staffed, only: ['One for each of the param `kinds`.'] }]).length, 2);
  // Two lines with no title are two rows, whatever they say.
  assert.equal(rows([plain, plain]).length, 2);
  // Started from its handler, else from its step; an attached one from nowhere.
  assert.deepEqual([first, left, { ...looked, at: 'uses/watcher', key: 'watcher', by: null }].map(startedFrom), ['first', 'look', '']);
  // Folded to the first five only where that puts away more than a couple.
  assert.equal(MOST_ROWS, 5);
  assert.deepEqual([5, 6, 7, 8, 20].map(foldsRows), [false, false, false, true, true]);
});

test('a way to change it keeps its label where its part is, and is no fact of the head', () => {
  const fork = line('Fork it: its text is copied.', { label: 'Fork it', title: 'its text is copied.', opens: { what: 'forking' } });
  const change = part('change', [fork]);
  assert.deepEqual(ownLines(change), [fork]);
  assert.deepEqual(headOf([part('is', [line('A board.')]), change]).facts, []);
});

test('what it makes is a row each: what is made and what it is called, as the daemon says them', () => {
  const table = line('A table called `agents`.', { label: 'Makes', title: 'a table called `agents`', made: { what: 'a table', name: '`agents`', called: 'agents' } });
  assert.deepEqual(madeOf(table), { what: 'a table', name: '`agents`' });
  const param = line('A table called what the param `name` says.', { made: { what: 'a table', name: 'the param `name`' } });
  assert.deepEqual(madeOf(param), { what: 'a table', name: 'the param `name`' });
  // A daemon from before says nothing apart: the row is called by its own words.
  assert.deepEqual(madeOf(line('A thing called `shelf`.', { title: 'a thing called `shelf`' })), { what: '', name: 'a thing called `shelf`' });
  assert.deepEqual(madeOf(line('Another thing.')), { what: '', name: 'Another thing.' });
});

test('a table is counted by its fields, and its model folds under its row by what it is about', () => {
  const field = (name: string) => ({ name, kind: 'text', text: `\`${name}\`: text.` });
  assert.equal(fieldsSaid({ fields: [field('a'), field('b')] }), '2 fields');
  assert.equal(fieldsSaid({ fields: [field('a')] }), '1 field');
  assert.equal(fieldsSaid({ as_said: 'What it is made of is what the param `shape` says.' }), 'as said');
  assert.equal(fieldsSaid({}), '');
  assert.equal(fieldsSaid(null), '');
  assert.equal(modelFold('main/agents', 3), 'makes:model:main/agents');
  assert.equal(modelFold(null, 3), 'makes:model:3');
});

test('the Data model lists every table it makes with fields, once, where there are two or more, and a field leads to the table it names', () => {
  const field = (name: string, names: string | null = null) => ({ name, kind: names ? `record of \`${names}\`` : 'text', names, text: name });
  const people = line('A table called `people`.', { made: { what: 'a table', name: '`people`', called: 'people' }, model: { fields: [field('title'), field('lead', 'people')] } });
  const jobs = line('A table called `jobs`.', { made: { what: 'a table', name: '`jobs`', called: 'jobs' }, model: { fields: [field('worker', 'people')] } });
  const again = line('A table called `jobs`.', { at: 'on.x/again', made: { what: 'a table', name: '`jobs`', called: 'jobs' }, model: { fields: [field('worker', 'people')] } });
  const shaped = line('A table called what the param `x` says.', { made: { what: 'a table', name: 'the param `x`' }, model: { as_said: 'What it is made of is what the param `y` says.' } });
  const blueprint = line('Another blueprint.', { made: { what: 'a blueprint', name: '`shop/x`', called: 'shop/x' } });
  const tables = dataModel([people, blueprint, jobs, again, shaped]);
  assert.deepEqual(
    tables.map((one) => [one.name, one.called]),
    [
      ['`people`', 'people'],
      ['`jobs`', 'jobs'],
    ],
  );
  // One is read under its own row: no part of its own.
  assert.deepEqual(dataModel([people, blueprint, shaped]), []);
  assert.equal(namedBy('people', tables)?.called, 'people');
  assert.equal(namedBy('nowhere', tables), null);
  assert.equal(namedBy(null, tables), null);
});

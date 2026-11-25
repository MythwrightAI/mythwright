import assert from 'node:assert/strict';
import { test } from 'node:test';
import { backAsked, defaultNamed, follows, forkNamed, forkedSaid, mayFork, offersFork } from './forked.ts';
import type { Owned, Where } from './forked.ts';
import type { Summary, SummaryLine } from './summary.ts';

const line = (text: string, opens?: SummaryLine['opens'], under: SummaryLine[] = []): SummaryLine => ({ text, at: null, key: null, mark: null, under, opens });
const member = ['projects', 'blueprints', 'board'];
const reader = ['projects:read', 'blueprints:read'];
const shop: Where = { project: 'shop', scopes: member, tables: { board: 'board', stages: 'table', docs: 'library' } };
const factory: Where = { project: null, scopes: member, tables: {} };

test('a line that leads to the default opens what ships, for the project the page is of or for none', () => {
  assert.deepEqual(follows({ what: 'default', name: 'std/board' }, shop), { does: 'open', tab: { kind: 'blueprint', name: 'std/board', project: 'shop' } });
  assert.deepEqual(follows({ what: 'default', name: 'std/board' }, factory), { does: 'open', tab: { kind: 'blueprint', name: 'std/board' } });
  assert.equal(follows({ what: 'default' }, shop), null);
  // Another blueprint it starts opens the same way, by its name, for the project it is looked at for.
  assert.deepEqual(follows({ what: 'blueprint', name: 'std/due' }, shop), { does: 'open', tab: { kind: 'blueprint', name: 'std/due', project: 'shop' } });
  assert.equal(follows({ what: 'blueprint' }, shop), null);
});

test('a line that leads to a fork opens the file it is, and is words where the page is of no project', () => {
  assert.deepEqual(follows({ what: 'fork', name: 'blueprints/std/board.bp' }, shop), { does: 'open', tab: { kind: 'file', project: 'shop', path: 'blueprints/std/board.bp' } });
  assert.equal(follows({ what: 'fork', name: 'blueprints/std/board.bp' }, factory), null);
});

test('a line that leads to a table opens the page its kind has, once the project has the table', () => {
  assert.deepEqual(follows({ what: 'table', name: 'stages' }, shop), { does: 'open', tab: { kind: 'table', project: 'shop', name: 'stages' } });
  assert.deepEqual(follows({ what: 'table', name: 'board' }, shop), { does: 'open', tab: { kind: 'board', project: 'shop' } });
  assert.deepEqual(follows({ what: 'table', name: 'docs' }, shop), { does: 'open', tab: { kind: 'library', project: 'shop', name: 'docs' } });
  // Not applied there yet, or looked at for no project: there is nothing to open.
  assert.equal(follows({ what: 'table', name: 'segments' }, shop), null);
  assert.equal(follows({ what: 'table', name: 'stages' }, factory), null);
});

test('what a page does by a press is offered only to whoever it would be done for', () => {
  assert.deepEqual(follows({ what: 'apply' }, shop), { does: 'apply' });
  assert.deepEqual(follows({ what: 'forking' }, shop), { does: 'fork' });
  // One opened from the factory's list asks which project.
  assert.deepEqual(follows({ what: 'forking' }, factory), { does: 'fork' });
  assert.deepEqual(follows({ what: 'back', name: 'std/board' }, shop), { does: 'back', name: 'std/board' });
  // Somebody who may not write a project's files is offered neither: the lines are words.
  const reading = { ...shop, scopes: reader };
  assert.equal(follows({ what: 'forking' }, reading), null);
  assert.equal(follows({ what: 'back', name: 'std/board' }, reading), null);
  assert.equal(mayFork(reader), false);
  assert.equal(mayFork(['projects:write']), true);
  assert.equal(mayFork(['*']), true);
  // A line that leads nowhere, and a word of a daemon after this one, are words.
  assert.equal(follows(undefined, shop), null);
  assert.equal(follows(null, shop), null);
  assert.equal(follows({ what: 'merging' }, shop), null);
});

const ships: Summary = {
  name: 'std/board',
  parts: [
    { kind: 'is', title: 'What it is', ordered: false, lines: [line('Give a project a board.')] },
    { kind: 'change', title: 'How to change what it does', ordered: true, lines: [line('Tell it something else.', { what: 'apply' }), line('Change the rows.', undefined, [line('`board`', { what: 'table', name: 'board' })]), line('Fork it.', { what: 'forking' })] },
  ],
};
const forked: Summary = {
  name: 'std/board',
  parts: [
    { kind: 'is', title: 'What it is', ordered: false, lines: [line('This project runs its own fork of it.', { what: 'fork', name: 'blueprints/std/board.bp' }), line('Give a project a board.')] },
    { kind: 'change', title: 'How to change what it does', ordered: true, lines: [line('Change this project\'s fork of it.', { what: 'fork', name: 'blueprints/std/board.bp' })] },
  ],
};

test('fork is offered in the head of one that ships, to whoever may, while the project has none of it', () => {
  assert.equal(forkNamed(ships), null);
  assert.equal(forkNamed(forked), 'blueprints/std/board.bp');
  assert.equal(forkNamed(null), null);
  assert.equal(offersFork({ ships: true, scopes: member, summary: ships }), true);
  // The project has one already: the line that opens it is what is offered.
  assert.equal(offersFork({ ships: true, scopes: member, summary: forked }), false);
  // Somebody who may not; a file of a project's, which is no default; and a text that has no summary yet.
  assert.equal(offersFork({ ships: true, scopes: reader, summary: ships }), false);
  assert.equal(offersFork({ ships: false, scopes: member, summary: ships }), false);
  assert.equal(offersFork({ ships: true, scopes: member, summary: null }), false);
  // A project that keeps no blueprints of its own: its summary says so, and offers no fork.
  const none: Summary = {
    name: 'std/board',
    parts: [{ kind: 'change', title: 'How to change what it does', ordered: true, lines: [line('This project keeps no blueprints of its own.')] }],
  };
  assert.equal(offersFork({ ships: true, scopes: member, summary: none }), false);
});

test("a fork's summary says the default it was forked from, by the line that opens it", () => {
  const fork: Summary = { name: 'std/board', parts: [{ kind: 'is', title: 'What it is', ordered: false, lines: [line('Forked from std/board.', { what: 'default', name: 'std/board' })] }] };
  assert.equal(defaultNamed(fork), 'std/board');
  assert.equal(defaultNamed(ships), null);
  assert.equal(defaultNamed(forked), null);
  assert.equal(defaultNamed(undefined), null);
});

const same: Owned = { blueprint: 'std/board', path: 'blueprints/std/board.bp', differs: { name: 'std/board', was: '4', now: '4', changed: [], same: true } };
const changed: Owned = { ...same, differs: { name: 'std/board', was: '5', now: '4', changed: ['the step `board` is changed', 'the param `store` is gone'], same: false } };

test('the question before a fork is taken away says what goes, what then runs and what is lost', () => {
  const nothing = backAsked(same, 'Shop');
  assert.deepEqual(nothing.says, [
    'blueprints/std/board.bp is taken away, and std/board as it ships is what runs in Shop under the name again, once what is running there is applied.',
    'Nothing is lost by it: the fork says what ships now.',
  ]);
  assert.deepEqual([nothing.lost, nothing.confirm, nothing.force], [[], 'Go back to the default', false]);
  // One that says something else: it is said to be lost, with what moved, and only then is the daemon told to lose it.
  const lost = backAsked(changed, 'Shop');
  assert.match(lost.says[1]!, /what it says is then gone, unless the project keeps it somewhere else\. Where it differs from the default as it ships now:$/);
  assert.deepEqual(lost.lost, ['the step `board` is changed', 'the param `store` is gone']);
  assert.deepEqual([lost.confirm, lost.force], ['Remove the fork', true]);
  // Where the daemon says where the fork differs from what it was forked from, that is what is said to go:
  // what the project did to it, and not what the default did since.
  const said = [
    { kind: 'differs', title: 'Where it differs from the default it was forked from', ordered: false, lines: [line('The handler `moved` is changed.', undefined, [line('The step `tell` is new.')])] },
    { kind: 'moved', title: 'The default has moved since', ordered: false, lines: [line('The param `store` is new.')] },
  ];
  const known = backAsked({ ...changed, said }, 'Shop');
  assert.match(known.says[1]!, /Where it differs from the default it was forked from:$/);
  assert.deepEqual(known.lost, ['The handler `moved` is changed.', 'The step `tell` is new.']);
  assert.equal(known.force, true);
});

test('what is said once a fork is made names the file that now runs there', () => {
  assert.equal(
    forkedSaid({ ...same, version: '4', project: 'shop' }, 'Shop'),
    "Forked std/board version 4 for Shop: blueprints/std/board.bp is what runs there under the name, and is the project's to change.",
  );
  assert.match(forkedSaid({ ...same, version: null }, 'Shop'), /^Forked std\/board for Shop: /);
});

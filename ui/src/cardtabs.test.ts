import assert from 'node:assert/strict';
import { test } from 'node:test';
import { aboutCards, calledOf, cellsOf, linkOf, LONG, namedIn, namingFields, rowsAbout, tabOf } from './cardtabs.ts';
import type { Tabbed } from './cardtabs.ts';
import type { Field, Row } from './table.ts';

const field = (name: string, kind = 'text', of: string[] = [], more: Partial<Field> = {}): Field => ({ name, kind, of, required: false, ...more });
const table = (name: string, more: Partial<Tabbed> = {}): Tabbed => ({ name, fields: [field('title')], ...more });
const row = (id: string, number: number, data: Record<string, unknown>): Row => ({ id, number, data, position: number, created: 0, updated: 0 });

const board = table('board');
const contests = table('contests', { beside: 'board', about: 'card', fields: [field('card', 'record', ['board']), field('state', 'choice', ['waiting', 'came'])], views: [{ name: 'A/B testing' }] });
const stages = table('stages', { beside: 'board', about: 'column', fields: [field('title'), field('column')] });
const notes = table('notes', { about: 'card', fields: [field('card', 'record', ['board'])] });
const ofAnother = table('others', { beside: 'board', about: 'card', fields: [field('card', 'record', ['features'])] });

test('a table has a tab in a card where it sits beside the board and each of its rows is about a card', () => {
  assert.deepEqual(
    aboutCards('board', [board, contests, stages, notes, ofAnother]).map((one) => one.name),
    ['contests'],
  );
  // Beside another board, or about a column, or beside nothing: none.
  assert.deepEqual(aboutCards('features', [board, contests, stages]), []);
});

test('a tab is called what its blueprint called its one way of being laid out, and else by its name', () => {
  assert.equal(tabOf(contests), 'A/B testing');
  assert.equal(tabOf(table('contests', { views: [{ name: 'contests' }] })), 'Contests');
  assert.equal(tabOf(table('ab_tests')), 'Ab tests');
  // A way somebody saved is not what the blueprint called it, and two say no one thing.
  assert.equal(tabOf(table('contests', { views: [{ name: 'contests' }, { name: 'mine', saved: true }] })), 'Contests');
  assert.equal(tabOf(table('contests', { views: [{ name: 'one' }, { name: 'two' }] })), 'Contests');
});

test('the rows about a card are those whose field names it, in the order they were made', () => {
  const rows = [row('c3', 3, { card: 'shop-1' }), row('c1', 1, { card: 'shop-1' }), row('c2', 2, { card: 'shop-2' }), row('c4', 4, {})];
  assert.deepEqual(rowsAbout(contests, rows, 'shop-1').map((one) => one.id), ['c1', 'c3']);
  assert.deepEqual(rowsAbout(contests, rows, 'shop-9'), []);
  assert.deepEqual(rowsAbout(table('x'), rows, 'shop-1'), []);
});

test('what a row says is each field that says something, a flag that is so by its name and a list a line each', () => {
  const fields = [
    field('title'),
    field('contest', 'record', ['contests']),
    field('state', 'choice', ['working', 'kept']),
    field('not_held_apart', 'flag'),
    field('held', 'flag'),
    field('scores', 'list'),
    field('said'),
    field('patch', 'text', [], { hidden: true }),
    field('branch', 'text', [], { label_chosen: 'Its branch' }),
    field('empty'),
  ];
  const entry = row('e1', 1, {
    title: 'shop-1-otter',
    contest: 'c1',
    state: 'kept',
    not_held_apart: true,
    held: false,
    scores: ['seat-1: A', '', 'seat-2: A'],
    said: 'x'.repeat(LONG + 1),
    patch: 'diff',
    branch: 'lane/shop-1--a',
    empty: '',
  });
  assert.deepEqual(cellsOf(entry, fields, ['contest']), [
    { name: 'state', says: 'kept', long: false },
    { name: 'not held apart', says: 'not held apart', flag: true },
    { name: 'scores', says: 'seat-1: A, seat-2: A', lines: ['seat-1: A', 'seat-2: A'] },
    { name: 'said', says: 'x'.repeat(LONG + 1), long: true },
    { name: 'Its branch', says: 'lane/shop-1--a', long: false },
  ]);
  // Words of several lines are long however short.
  assert.equal(cellsOf(row('e2', 2, { said: 'a\nb' }), [field('said')])[0]?.long, true);
  assert.equal(calledOf(field('known_by')), 'known by');
});

test('the fields of a table that name a row of another are what it is shown under', () => {
  assert.deepEqual(namingFields([field('contest', 'record', ['contests']), field('card', 'record', ['board']), field('also', 'records', ['contests'])], 'contests'), ['contest', 'also']);
});

test('what a row says is drawn as a link only where it is an address a browser opens as a page', () => {
  assert.equal(linkOf('https://example.org/acme/shop/pull/1'), 'https://example.org/acme/shop/pull/1');
  assert.equal(linkOf(' http://example.org '), 'http://example.org');
  assert.equal(linkOf('javascript:alert(1)'), undefined);
  assert.equal(linkOf('lane/shop-4'), undefined);
  assert.equal(linkOf('https://example.org/a b'), undefined, 'words with an address in them are words');
  assert.equal(linkOf(''), undefined);
});

test('what is asked on a card names a row under the row that asked it by what the row is last called by, as a word of it', () => {
  const fields = [field('title'), field('card', 'text', [], { calls: 1 }), field('way', 'text', [], { calls: 2 })];
  const otter = row('shop-entries-1', 1, { title: 'shop-contests-1-otter', card: 'shop-1', way: 'otter' });
  const slate = row('shop-entries-2', 2, { title: 'shop-contests-1-slate', card: 'shop-1', way: 'Slate' });
  const asked = "Only otter's work could be kept: B is otter's work; slate's was not kept (it failed).";
  assert.equal(namedIn(asked, otter, fields), true);
  assert.equal(namedIn(asked, slate, fields), true, 'whatever its capitals');
  assert.equal(namedIn('The otters are all gone.', otter, fields), false, 'a word that only begins with it names nothing');
  assert.equal(namedIn('Take quick-otter?', otter, fields), false, 'nor one it is only a part of');
  assert.equal(namedIn("Take shop-1's work?", otter, [field('title'), field('card', 'text', [], { calls: 1 }), field('way')]), true, 'by the last it is called by');
  assert.equal(namedIn("Take shop-contests-1-otter?", otter, [field('title')]), false, 'a row called by nothing is named by nothing');
  assert.equal(namedIn("Take otter's work?", row('shop-entries-3', 3, { way: '' }), fields), false);
});

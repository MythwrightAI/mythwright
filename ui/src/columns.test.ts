import assert from 'node:assert/strict';
import { test } from 'node:test';
import { carried, carries, frozen, landing, lefts, lines, listed, sortedBy, sortKept, staying, stepped, steps } from './columns.ts';
import { banded, headings } from './groups.ts';
import { changed, columns } from './table.ts';
import type { Field, Group, View } from './table.ts';

const field = (name: string, kind = 'text'): Field => ({ name, kind, of: [], required: name === 'title' });
// A map, as the daemon hands a table's fields over: in the order of their names.
const raw = [field('due', 'date'), field('labels', 'list'), field('note'), field('points', 'number'), field('status', 'choice'), field('title')];
const fields = columns(raw);
const names = (all: Field[]) => all.map((one) => one.name).join(' ');
const plain = { fields };
const groups: Group[] = [
  { name: 'When', fields: ['due', 'status'] },
  { name: 'What', fields: ['points', 'labels', 'note'] },
];
const grouped = { fields, groups };
const said = (bands: ReturnType<typeof banded>) => bands.map((band) => `${band.name ?? ''}: ${names(band.fields)}`);

test('a view that says no order leaves the columns as they always stood, and so does one kept before a view could say', () => {
  assert.equal(names(fields), 'title status labels due points note');
  for (const none of [undefined, null, []]) assert.equal(names(banded(fields, null, none).flatMap((band) => band.fields)), names(fields));
});

test('the columns a view names come first as it names them, the rest after as they stood, and what a record is known by first whatever is said', () => {
  const order = ['points', 'title', 'due'];
  assert.equal(names(banded(fields, null, order).flatMap((band) => band.fields)), 'title points due status labels note');
  // A column the table came to have later is no part of an order written before it, and stands at the end.
  const later = columns([...raw, field('zone')]);
  assert.equal(names(banded(later, null, ['note', 'points', 'due', 'labels', 'status']).flatMap((band) => band.fields)), 'title note points due labels status zone');
  // A name that is no column is passed over.
  assert.equal(names(banded(fields, null, ['gone', 'note']).flatMap((band) => band.fields)), 'title note status labels due points');
});

test('in a table that says groups a view orders the columns inside each and never scatters one', () => {
  assert.deepEqual(said(banded(fields, groups)), [': title', 'When: due status', 'What: points labels note']);
  assert.deepEqual(said(banded(fields, groups, ['note', 'status', 'points'])), [': title', 'When: status due', 'What: note points labels']);
});

test('a column carried stands before the one it was let go before, or at the end, and the order kept names every column', () => {
  assert.deepEqual(carried(plain, 'points', 'status'), ['title', 'points', 'status', 'labels', 'due', 'note']);
  assert.deepEqual(carried(plain, 'status', null), ['title', 'labels', 'due', 'points', 'note', 'status']);
  // From the order the view already says.
  assert.deepEqual(carried({ fields, order: ['title', 'points', 'status', 'labels', 'due', 'note'] }, 'note', 'points'), ['title', 'note', 'points', 'status', 'labels', 'due']);
});

test('a column let go where it stands changes nothing, what a record is known by is not carried, and nothing is put before it', () => {
  assert.equal(carried(plain, 'status', 'labels'), null);
  assert.equal(carried(plain, 'note', null), null);
  assert.equal(carried(plain, 'status', 'status'), null);
  assert.equal(carried(plain, 'title', 'note'), null);
  assert.equal(carried(plain, 'note', 'title'), null);
  assert.equal(carried(plain, 'nothing', 'note'), null);
});

test('a column is carried within its group and not out of it', () => {
  assert.deepEqual(carried(grouped, 'note', 'points'), ['title', 'due', 'status', 'note', 'points', 'labels']);
  assert.deepEqual(carried(grouped, 'due', null), ['title', 'status', 'due', 'points', 'labels', 'note']);
  assert.equal(carried(grouped, 'note', 'due'), null);
  assert.equal(carried(grouped, 'due', 'points'), null);
});

test('a column is moved a place past the column beside it that is shown, and has no place to go at an end of its band or beside what a record is known by', () => {
  const shown = ['title', 'status', 'due', 'points', 'note'];
  // Labels is left out of the view: status moves past it and past due in one step.
  assert.equal(stepped(plain, shown, 'status', 1), 'points');
  assert.equal(stepped(plain, shown, 'points', -1), 'due');
  assert.equal(stepped(plain, shown, 'points', 1), null);
  assert.equal(stepped(plain, shown, 'note', 1), undefined);
  assert.equal(stepped(plain, shown, 'status', -1), undefined);
  assert.equal(stepped(plain, shown, 'title', 1), undefined);
  // In groups a column has the ends of its own group.
  const every = fields.map((one) => one.name);
  assert.equal(stepped(grouped, every, 'status', 1), undefined);
  assert.equal(stepped(grouped, every, 'points', -1), undefined);
  assert.equal(stepped(grouped, every, 'points', 1), 'note');
  assert.equal(stepped(grouped, every, 'note', -1), 'labels');
});

test('carrying and moving are changes worked out from the view as it is when they are made, so two made one after the other both hold', () => {
  const view: View = { name: 'rows', kind: 'table', by: null };
  const first = changed(raw, [view], 'rows', carries(null, 'points', 'status'));
  assert.deepEqual(first.columns, ['title', 'points', 'status', 'labels', 'due', 'note']);
  const second = changed(raw, [{ ...view, columns: first.columns }], 'rows', steps(null, 'note', -1));
  assert.deepEqual(second.columns, ['title', 'points', 'status', 'labels', 'note', 'due']);
  // One that goes nowhere says nothing of the columns, and what the view said is carried over as it was.
  const none = changed(raw, [{ ...view, columns: second.columns }], 'rows', steps(null, 'due', 1));
  assert.deepEqual(none.columns, second.columns);
  // Moved in the list of every column, one that is left out is a place too.
  const hidden = changed(raw, [{ ...view, hides: ['labels'] }], 'rows', steps(null, 'status', 1, true));
  assert.deepEqual(hidden.columns, ['title', 'labels', 'status', 'due', 'points', 'note']);
});

test('the columns a layout lists are in the order they stand, each with whether it is shown and where it can be moved', () => {
  const all = listed({ fields, groups, order: ['status'] }, ['note']);
  assert.deepEqual(all.map((one) => `${one.field.name}${one.shown ? '' : ' (out)'}${one.up ? ' up' : ''}${one.down ? ' down' : ''}`), ['status down', 'due up', 'points down', 'labels up down', 'note (out) up']);
});

test('a carried column lands before the head whose middle the pointer has not passed, and after the last past them all', () => {
  const heads = [
    { name: 'status', left: 300, right: 430 },
    { name: 'labels', left: 430, right: 560 },
    { name: 'due', left: 560, right: 690 },
  ];
  assert.deepEqual(landing(heads, 310), { before: 'status', at: 300 });
  assert.deepEqual(landing(heads, 366), { before: 'labels', at: 430 });
  assert.deepEqual(landing(heads, 624), { before: 'due', at: 560 });
  assert.deepEqual(landing(heads, 626), { before: null, at: 690 });
  // Before the first of them, never before what a record is known by, which is not among them.
  assert.deepEqual(landing(heads, 20), { before: 'status', at: 300 });
  assert.equal(landing([], 20), null);
});

test('a view is frozen up to a column: that one and those before it, and none where it names none that is drawn', () => {
  const drawn = ['title', 'status', 'due', 'points'];
  assert.equal(frozen(drawn, 'title'), 1);
  assert.equal(frozen(drawn, 'due'), 3);
  assert.equal(frozen(drawn, null), 0);
  assert.equal(frozen(drawn, undefined), 0);
  assert.equal(frozen(drawn, 'labels'), 0);
});

test('only as many columns stay as leave room for the rest to be scrolled in, and each is told where it begins', () => {
  const widths = [260, 130, 130, 130];
  assert.equal(staying(widths, 3, 1200), 3);
  assert.equal(staying(widths, 3, 760), 3);
  assert.equal(staying(widths, 3, 759), 2);
  assert.equal(staying(widths, 3, 520), 1);
  assert.equal(staying(widths, 3, 499), 0);
  // At a phone's width what a record is known by leaves less than that of the screen: nothing stays.
  assert.equal(staying([210, 112, 112], 2, 366), 0);
  assert.equal(staying(widths, 0, 1200), 0);
  assert.deepEqual(lefts(widths, 3), [0, 260, 390]);
  assert.deepEqual(lefts(widths, 0), []);
});

test('the band of headings is two cells where the frozen edge falls inside a band, each over its own columns', () => {
  const bands = banded(fields, groups);
  const cells = (stay: number) => headings(bands, stay).map((one) => `${one.name ?? ''} ${one.from}+${one.span}${one.begins ? ' begins' : ''}`);
  assert.deepEqual(cells(0), [' 0+1', 'When 1+2 begins', 'What 3+3 begins']);
  assert.deepEqual(cells(1), [' 0+1', 'When 1+2 begins', 'What 3+3 begins']);
  assert.deepEqual(cells(2), [' 0+1', 'When 1+1 begins', 'When 2+1', 'What 3+3 begins']);
  assert.deepEqual(cells(5), [' 0+1', 'When 1+2 begins', 'What 3+2 begins', 'What 5+1']);
});

test('a line of the menu sorts up or down, and the one in force pressed again leaves the rows as they were added', () => {
  assert.deepEqual(sortedBy(null, 'points', false), { field: 'points', down: false });
  assert.deepEqual(sortedBy({ field: 'points', down: false }, 'points', true), { field: 'points', down: true });
  assert.deepEqual(sortedBy({ field: 'title', down: true }, 'points', true), { field: 'points', down: true });
  assert.equal(sortedBy({ field: 'points', down: true }, 'points', true), null);
});

test('what a browser kept of a sort is read back, and what is no sort is none', () => {
  assert.deepEqual(sortKept('{"field":"points","down":true}'), { field: 'points', down: true });
  assert.deepEqual(sortKept('{"field":"points"}'), { field: 'points', down: false });
  for (const none of [null, '', 'null', '{', '"points"', '{"down":true}']) assert.equal(sortKept(none), null);
});

test('everybody sorts, and only whoever may save a view is offered what changes it', () => {
  const all = { may: true, known: false, holds: 'things' as const, gathers: true, left: true, right: true };
  assert.deepEqual(lines(all), ['filter', 'up', 'down', 'group', 'figure', 'freeze', 'hide', 'left', 'right']);
  assert.deepEqual(lines({ ...all, may: false }), ['up', 'down']);
  // What a record is known by is never left out; a column no two rows share a thing in says so; one that cannot be gathered by or moved is not offered it.
  assert.deepEqual(lines({ ...all, known: true, left: false, right: false }), ['filter', 'up', 'down', 'group', 'figure', 'freeze']);
  assert.deepEqual(lines({ ...all, holds: 'unshared', gathers: false, right: false }), ['unshared', 'up', 'down', 'figure', 'freeze', 'hide', 'left']);
  // A view is never held by a number or a day: there is no line to say so.
  assert.deepEqual(lines({ ...all, holds: 'never', gathers: false }), ['up', 'down', 'figure', 'freeze', 'hide', 'left', 'right']);
});

import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import { called, figure, figuring, fits, FIGURES, kept, meaning, offered, written } from './figures.ts';
import type { Field, Row } from './table.ts';

const field = (name: string, kind = 'text'): Field => ({ name, kind, of: [], required: false });
const row = (number: number, data: Record<string, unknown>): Row => ({ id: `r-${number}`, number, data, position: number, created: 0, updated: 0 });
const rows = [
  row(1, { title: 'Shelving', points: 3, done: true, status: 'doing', labels: ['urgent', 'soon'], note: '' }),
  row(2, { title: 'Crates', points: 0, done: false, status: 'doing', labels: ['soon'], note: '  ' }),
  row(3, { title: 'Baskets', points: null, status: 'todo', labels: [] }),
  row(4, { title: 'Bins', points: 4.5, done: true, status: '', note: 'by the door' }),
];
const [title, points, done, status, labels, note] = [field('title'), field('points', 'number'), field('done', 'flag'), field('status', 'choice'), field('labels', 'list'), field('note')];

test('how many rows say something in a column, and how many nothing: nothing, an empty text and an empty list are all nothing said', () => {
  assert.equal(figure(rows, title, 'filled'), 4);
  assert.equal(figure(rows, title, 'empty'), 0);
  // A cell somebody emptied is kept as empty text, and means what one never filled means.
  assert.equal(figure(rows, note, 'filled'), 1);
  assert.equal(figure(rows, note, 'empty'), 3);
  assert.equal(figure(rows, labels, 'filled'), 2);
  assert.equal(figure(rows, labels, 'empty'), 2);
  // Nought is a number that was said.
  assert.equal(figure(rows, points, 'filled'), 3);
  assert.equal(figure(rows, points, 'empty'), 1);
  assert.equal(figure(rows, status, 'empty'), 1);
});

test('a number is added up, and has a mean, a least and a greatest of the rows that say one', () => {
  assert.equal(figure(rows, points, 'sum'), 7.5);
  assert.equal(figure(rows, points, 'mean'), 2.5);
  assert.equal(figure(rows, points, 'least'), 0);
  assert.equal(figure(rows, points, 'greatest'), 4.5);
});

test('a figure with nothing to go by is no figure: the mean of no numbers is not nought, though they add up to it', () => {
  const none = [row(1, { title: 'Shelving' }), row(2, { title: 'Crates', points: '' })];
  assert.equal(figure(none, points, 'mean'), null);
  assert.equal(figure(none, points, 'least'), null);
  assert.equal(figure(none, points, 'greatest'), null);
  assert.equal(figure(none, points, 'sum'), 0);
  assert.equal(figure([], points, 'mean'), null);
  assert.equal(figure([], title, 'filled'), 0);
  assert.equal(written(null), '–');
});

test('a tick is counted where it is ticked, and how many different things are said counts each thing of a list once', () => {
  assert.equal(figure(rows, done, 'ticked'), 2);
  assert.equal(figure(rows, status, 'different'), 2);
  assert.equal(figure(rows, labels, 'different'), 2);
  assert.equal(figure(rows, title, 'different'), 4);
});

test('a column says only the figures its kind can: a text is not added up and a number is not ticked', () => {
  assert.deepEqual(offered(points), ['filled', 'empty', 'sum', 'mean', 'least', 'greatest']);
  assert.deepEqual(offered(done), ['filled', 'empty', 'ticked']);
  assert.deepEqual(offered(status), ['filled', 'empty', 'different']);
  assert.deepEqual(offered(title), ['filled', 'empty', 'different']);
  assert.equal(figure(rows, title, 'sum'), null);
  assert.equal(figure(rows, points, 'ticked'), null);
  assert.equal(figure(rows, points, 'median'), null);
  assert.equal(fits('number', 'median'), false);
  // Every figure is called something and says what it is.
  for (const one of FIGURES) assert.ok(called(one) !== one && meaning(one) !== '');
});

test('a figure is written as a plain number, to two places where it has more', () => {
  assert.equal(written(7.5), '7.5');
  assert.equal(written(12), '12');
  assert.equal(written(10 / 3), '3.33');
  assert.equal(written(0), '0');
});

test('the figures a view says are told one at a time, and less what names a column that is gone or cannot say it', () => {
  assert.deepEqual(figuring(null, 'points', 'sum'), { points: 'sum' });
  assert.deepEqual(figuring({ points: 'sum' }, 'title', 'filled'), { points: 'sum', title: 'filled' });
  assert.deepEqual(figuring({ points: 'sum', title: 'filled' }, 'points', null), { title: 'filled' });
  assert.equal(figuring({ points: 'sum' }, 'points', null), null);
  assert.deepEqual(kept({ points: 'sum', gone: 'filled', title: 'sum' }, [title, points]), { points: 'sum' });
  assert.equal(kept({ gone: 'filled' }, [title]), null);
  assert.equal(kept(null, [title]), null);
});

test('every figure comes to what the table of cases the daemon is held to says', () => {
  const table = JSON.parse(readFileSync(new URL('./figurecases.json', import.meta.url), 'utf8')) as {
    rows: Record<string, unknown>[];
    cases: [string, string, string, number | null][];
  };
  const held = table.rows.map((data, at) => row(at + 1, data));
  assert.ok(table.cases.length > 20, 'the table is there');
  for (const [name, kind, which, meant] of table.cases) assert.equal(figure(held, field(name, kind), which), meant, `${name} ${which}`);
});

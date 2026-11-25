import assert from 'node:assert/strict';
import { test } from 'node:test';
import { cellOf, parse, planOf, records, summed, toCsv, valueOf, written } from './csv.ts';
import type { Field, Row } from './table.ts';

const field = (name: string, kind: string, more: Partial<Field> = {}): Field => ({ name, kind, of: [], required: false, ...more });
const FIELDS: Field[] = [
  field('points', 'number'),
  field('title', 'text', { required: true }),
  field('urgent', 'flag'),
  field('tags', 'list'),
  field('state', 'choice', { of: ['open', 'shut'] }),
  field('notes', 'text'),
  field('owner', 'record', { of: ['people'] }),
  field('after', 'records', { of: ['tickets'] }),
  field('moved', 'text', { hidden: true, quiet: true }),
];
const row = (id: string, data: Record<string, unknown>): Row => ({ id, number: 1, data, position: 1, created: 1, updated: 1 });
const ROWS: Row[] = [
  row('shop-tickets-1', { title: 'Shelving', points: 8, urgent: true, tags: ['wood', 'a; b'], state: 'open', notes: 'He said "no", then\nleft', owner: 'shop-people-2', after: ['shop-tickets-2', 'shop-tickets-3'], moved: 'kept' }),
  row('shop-tickets-2', { title: 'Crates, large', urgent: false }),
  row('shop-tickets-3', { title: ' Labels ' }),
];

test('a file has a column for each field somebody fills in, what a record is known by first', () => {
  assert.deepEqual(written(FIELDS).map((one) => one.name), ['title', 'state', 'owner', 'after', 'tags', 'points', 'urgent', 'notes']);
});

test('a table is written out a line a record, each cell as its field holds it', () => {
  assert.equal(
    toCsv(FIELDS, ROWS),
    [
      'id,title,state,owner,after,tags,points,urgent,notes',
      'shop-tickets-1,Shelving,open,shop-people-2,shop-tickets-2; shop-tickets-3,wood; a\\; b,8,true,"He said ""no"", then\nleft"',
      'shop-tickets-2,"Crates, large",,,,,,false,',
      'shop-tickets-3," Labels ",,,,,,,',
      '',
    ].join('\r\n'),
  );
});

test('a cell is read as its field takes it, and one that cannot be says why', () => {
  assert.deepEqual(valueOf(FIELDS[0]!, ' 8.5 '), { value: 8.5 });
  assert.deepEqual(valueOf(FIELDS[0]!, 'eight'), { wrong: '`points` is a number, and "eight" is not one' });
  assert.deepEqual(valueOf(FIELDS[2]!, 'TRUE'), { value: true });
  assert.deepEqual(valueOf(FIELDS[2]!, 'no'), { value: false });
  assert.deepEqual(valueOf(FIELDS[2]!, 'maybe'), { wrong: '`urgent` is true or false, and "maybe" is neither' });
  assert.deepEqual(valueOf(FIELDS[3]!, 'wood;a\\; b ; '), { value: ['wood', 'a; b'] });
  assert.deepEqual(valueOf(FIELDS[1]!, ' Labels '), { value: ' Labels ' });
  assert.deepEqual(valueOf(FIELDS[4]!, ' open '), { value: 'open' });
  assert.deepEqual(valueOf(FIELDS[1]!, '  '), { value: null });
  assert.equal(cellOf(FIELDS[3]!, valueOf(FIELDS[3]!, 'wood; a\\; b')['value' as never]), 'wood; a\\; b');
});

test('a file is read by its quotation marks, whatever ends its lines, and a line with nothing on it is left out', () => {
  assert.deepEqual(parse('﻿a,b\r\n1,"x, ""y""\nz"\n\n,\r3,'), [['a', 'b'], ['1', 'x, "y"\nz'], ['', ''], ['3', '']]);
  assert.deepEqual(parse(''), []);
  assert.deepEqual(parse('a'), [['a']]);
});

test('a table written out and read straight back changes nothing', () => {
  const plan = planOf(FIELDS, ROWS, toCsv(FIELDS, ROWS));
  assert.deepEqual(plan, { add: [], change: [], same: 3, wrong: [], left: [] });
});

test('a line with a record\'s id changes that record in the cells that say something else, and one with none is added', () => {
  const text = ['id,title,points,urgent,tags,colour', 'shop-tickets-1,Shelving,13,TRUE,wood;a\\; b,red', 'shop-tickets-2,"Crates, large",,false,,', ',Paint,2,,"tin; brush",', 'shop-tickets-3,Labels,,,,'].join('\n');
  assert.deepEqual(planOf(FIELDS, ROWS, text), {
    add: [{ line: 4, fields: { title: 'Paint', points: 2, tags: ['tin', 'brush'] } }],
    change: [
      { line: 2, id: 'shop-tickets-1', fields: { points: 13 } },
      { line: 5, id: 'shop-tickets-3', fields: { title: 'Labels' } },
    ],
    same: 1,
    wrong: [],
    left: ['colour'],
  });
});

test('a cell emptied takes what the record said away', () => {
  const plan = planOf(FIELDS, ROWS, 'id,title,points\nshop-tickets-1,Shelving,\n');
  assert.deepEqual(plan.change, [{ line: 2, id: 'shop-tickets-1', fields: { points: null } }]);
});

test('a line that cannot be read in says why, and the rest are read', () => {
  const text = ['id,title,points', 'shop-tickets-9,Gone,1', ',,4', ',Paint,many', ',Brushes,3', ',,'].join('\n');
  const plan = planOf(FIELDS, ROWS, text);
  assert.deepEqual(plan.add, [{ line: 5, fields: { title: 'Brushes', points: 3 } }]);
  assert.deepEqual(plan.wrong, [
    { line: 2, why: 'no record of this table is called shop-tickets-9; with its id left empty the line is a record to add' },
    { line: 3, why: 'it says nothing of `title`, which a record has to say' },
    { line: 4, why: '`points` is a number, and "many" is not one' },
    { line: 6, why: 'it says nothing of `title`, which a record has to say' },
  ]);
});

test('a file that names none of the table\'s fields, or is empty, is read as nothing', () => {
  assert.deepEqual(planOf(FIELDS, ROWS, 'name,colour\nPaint,red\n').wrong, [{ line: 1, why: 'its first line names no field of this table (title, state, owner, after, tags, points, urgent, notes)' }]);
  assert.deepEqual(planOf(FIELDS, ROWS, '').wrong, [{ line: 1, why: 'the file is empty' }]);
});

test('a file with no column of ids only adds, and a column a table keeps for itself is left out', () => {
  const plan = planOf(FIELDS, ROWS, 'title,moved\nPaint,x\n');
  assert.deepEqual(plan.add, [{ line: 2, fields: { title: 'Paint' } }]);
  assert.deepEqual(plan.left, ['moved']);
});

test('what a file would do is said in a sentence', () => {
  assert.equal(summed({ add: [{ line: 2, fields: {} }], change: [], same: 0, wrong: [], left: [] }, 'ticket'), '1 ticket to add, 0 to change');
  assert.equal(summed({ add: [], change: [{ line: 2, id: 'a', fields: {} }], same: 2, wrong: [], left: [] }, 'ticket'), '0 tickets to add, 1 to change, 2 that say what is there already');
});

test('a record that is added says what its table has every record say, whether or not the file has a column for it, but for what is given it', () => {
  const fields = [field('title', 'text', { required: true }), field('status', 'choice', { of: ['todo', 'done'], required: true })];
  assert.deepEqual(planOf(fields, [], 'title\nPaint\n').wrong, [{ line: 2, why: 'it says nothing of `status`, which a record has to say' }]);
  assert.deepEqual(planOf(fields, [], 'title\nPaint\n', ['status']).add, [{ line: 2, fields: { title: 'Paint' } }]);
});

test('a line is said by where it begins in the file, though a cell before it holds a line\'s end', () => {
  const text = 'title,notes\r\n"Paint","two\r\nlines"\r\n\r\nBrushes,\n';
  assert.deepEqual(records(text), [
    { line: 1, cells: ['title', 'notes'] },
    { line: 2, cells: ['Paint', 'two\r\nlines'] },
    { line: 5, cells: ['Brushes', ''] },
  ]);
  const plan = planOf(FIELDS, ROWS, 'id,title,points\n,"A\nB",1\n,Paint,many\n');
  assert.deepEqual(plan.wrong, [{ line: 4, why: '`points` is a number, and "many" is not one' }]);
});

import assert from 'node:assert/strict';
import { test } from 'node:test';
import { banded, headed } from './groups.ts';
import { columns } from './table.ts';
import type { Field, Group } from './table.ts';

const field = (name: string, kind = 'text'): Field => ({ name, kind, of: [], required: name === 'title' });
// As a fleet's agents are declared, less a few: a map, which the daemon hands over in the order of their names.
const fields = columns([field('adapter'), field('count', 'number'), field('holds', 'list'), field('permission', 'choice'), field('role'), field('title'), field('told'), field('wall', 'choice')]);
const groups: Group[] = [
  { name: 'Who they are', fields: ['role', 'count'] },
  { name: 'What they may ask', fields: ['holds'] },
  { name: 'How their program works', fields: ['permission', 'adapter'] },
];
const said = (bands: ReturnType<typeof banded>) => bands.map((band) => `${band.name ?? ''}: ${band.fields.map((one) => one.name).join(' ')}`);

test('columns are kept together by group, in the order the groups are said and the order each says its fields', () => {
  assert.deepEqual(said(banded(fields, groups)), [
    // What a record is known by first, then what is in no group, as they stood: by kind and then by name.
    ': title wall told',
    'Who they are: role count',
    'What they may ask: holds',
    'How their program works: permission adapter',
  ]);
  assert.equal(headed(banded(fields, groups)), true);
});

test('a table that says no groups is as it always was: its columns in their own order, under no heading', () => {
  for (const none of [undefined, null, []]) {
    const bands = banded(fields, none);
    assert.deepEqual(said(bands), [`: ${fields.map((one) => one.name).join(' ')}`]);
    assert.equal(headed(bands), false);
  }
  assert.deepEqual(banded([], groups), []);
});

test('what a record is known by is first whatever a group says of it', () => {
  const bands = banded(fields, [{ name: 'Who they are', fields: ['role', 'title', 'count'] }]);
  assert.deepEqual(said(bands), [': title permission wall holds adapter told', 'Who they are: role count']);
});

test('a group none of whose fields is a column is no band, and a field that is not one is passed over', () => {
  // The view leaves `holds` and `role` out: they are not among the columns handed over.
  const shown = fields.filter((one) => one.name !== 'holds' && one.name !== 'role');
  assert.deepEqual(said(banded(shown, groups)), [': title wall told', 'Who they are: count', 'How their program works: permission adapter']);
  // A field the table no longer has, and one named twice by a table from somewhere that did not refuse it.
  const odd: Group[] = [{ name: 'One', fields: ['gone', 'role'] }, { name: 'Two', fields: ['role', 'count'] }];
  assert.deepEqual(said(banded(fields, odd)), [': title permission wall holds adapter told', 'One: role', 'Two: count']);
  // Every column in a group but the one a record is known by: that one stands alone before them.
  const all: Group[] = [{ name: 'All', fields: fields.map((one) => one.name) }];
  assert.deepEqual(said(banded(fields, all)), [': title', `All: ${fields.filter((one) => one.name !== 'title').map((one) => one.name).join(' ')}`]);
});

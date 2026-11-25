import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import { applied, LAYOUTS, slugOf, tableNamed } from './tablemade.ts';

test('a name comes to what the daemon makes of it', () => {
  assert.equal(slugOf('Reading list'), 'reading-list');
  assert.equal(slugOf('  Q3 -- plans! '), 'q3-plans');
  assert.equal(slugOf('Café notes'), 'caf-notes');
  assert.equal(slugOf('!!!'), '');
});

test('a table is made as its name, with what people call it where that is other, under its own key', () => {
  assert.deepEqual(tableNamed('Reading list', ['board'], 'Shop'), { name: 'reading-list', title: 'Reading list', key: 'table-reading-list' });
  assert.deepEqual(tableNamed('notes', ['board'], 'Shop'), { name: 'notes', title: '', key: 'table-notes' });
});

test('a name the project has a table of, or one with nothing to call it by, is refused before anything is asked', () => {
  assert.deepEqual(tableNamed('Board', ['board'], 'Shop'), { wrong: 'Shop has a table called board already: call this one something else.' });
  assert.deepEqual(tableNamed('Agents', ['board', 'agents'], 'Shop'), { wrong: 'Shop has a table called agents already: call this one something else.' });
  assert.deepEqual(tableNamed('   ', [], 'Shop'), { wrong: 'A table is called something with a letter or a digit in it.' });
  assert.ok('wrong' in tableNamed('x'.repeat(200), [], 'Shop'));
});

test('it is applied as std/table under its key, with the layout and only an environment that was chosen', () => {
  const named = { name: 'reading', title: 'Reading', key: 'table-reading' };
  assert.deepEqual(applied('shop', named, 'list', null), {
    blueprint: 'std/table',
    project: 'shop',
    key: 'table-reading',
    params: { name: 'reading', title: 'Reading', layout: 'list' },
    wait: true,
  });
  assert.equal(applied('shop', named, 'table', 'prod').environment, 'prod');
  assert.deepEqual(
    LAYOUTS.map((one) => one.kind),
    ['table', 'list', 'gallery'],
  );
});

test('what may be chosen is what std/table takes, in its order', () => {
  const text = readFileSync(new URL('../../blueprints/std/table.bp', import.meta.url), 'utf8');
  const taken = /layout:[\s\S]*?enum: \[([^\]]*)\]/.exec(text)?.[1]?.split(',').map((one) => one.trim());
  assert.deepEqual(
    LAYOUTS.map((one) => one.kind),
    taken,
  );
});

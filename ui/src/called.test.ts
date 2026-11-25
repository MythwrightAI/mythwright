import assert from 'node:assert/strict';
import { test } from 'node:test';
import { projectNamed, tableCalled, tableTitled } from './called.ts';

test('a table is called what somebody called it, else what its blueprint says, else its name as it reads', () => {
  assert.equal(tableCalled({ name: 'test-runs' }), 'Test-runs');
  assert.equal(tableCalled({ name: 'test-runs', title: 'Test runs' }), 'Test runs');
  assert.equal(tableCalled({ name: 'test-runs', title: 'Test runs', title_chosen: 'Runs' }), 'Runs');
  assert.equal(tableCalled({ name: 'test-runs', title: null, title_chosen: null }), 'Test-runs');
});

test('a project is called something of its own, refused before anything is asked otherwise', () => {
  const all = [
    { id: 'shop', name: 'Shop' },
    { id: 'depot', name: 'Depot' },
  ];
  assert.deepEqual(projectNamed('  Shop front ', 'shop', all), { name: 'Shop front' });
  assert.deepEqual(projectNamed('shop', 'shop', all), { name: 'shop' }, 'its own name in other capitals is its own');
  assert.deepEqual(projectNamed('DEPOT', 'shop', all), { wrong: 'Another project is called DEPOT: call this one something else.' });
  assert.deepEqual(projectNamed('   ', 'shop', all), { wrong: 'A project is called something: say what it is to be called.' });
  assert.deepEqual(projectNamed('a\nb', 'shop', all), { wrong: "A project's name is one line of words." });
  assert.ok('wrong' in projectNamed('x'.repeat(81), 'shop', all));
});

test('a table is called something, or by its name again where nothing is typed', () => {
  assert.deepEqual(tableTitled(' Test runs '), { title: 'Test runs' });
  assert.deepEqual(tableTitled('  '), { title: null });
  assert.deepEqual(tableTitled('a\tb'), { wrong: 'What a table is called is one line of words.' });
  assert.ok('wrong' in tableTitled('x'.repeat(81)));
});

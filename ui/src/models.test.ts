import assert from 'node:assert/strict';
import { test } from 'node:test';
import { choices, offered } from './models.ts';

const listed = [
  { id: 'default', label: 'Default (recommended)', note: 'The big one', default: true },
  { id: 'big', label: 'Big', note: 'For hard work', default: false },
  { id: 'small', label: 'Small', note: 'For quick answers', default: false },
];

test('the provider\'s usual model comes first and means naming none', () => {
  assert.deepEqual(choices(listed), [
    { value: '', label: 'Default (recommended)', note: 'The big one' },
    { value: 'big', label: 'Big', note: 'For hard work' },
    { value: 'small', label: 'Small', note: 'For quick answers' },
  ]);
});

test('a provider that marks no usual model still offers leaving it to the tool', () => {
  const options = choices(listed.map((model) => ({ ...model, default: false })));
  assert.deepEqual(options[0], { value: '', label: 'Default', note: 'Whatever it is set to use' });
  assert.equal(options.length, 4);
});

test('a provider that lists nothing offers only its own choice', () => {
  assert.deepEqual(choices([]), [{ value: '', label: 'Default', note: 'Whatever it is set to use' }]);
});

test('a model that is not in the list is one that was typed', () => {
  assert.equal(offered(listed, 'big'), true);
  assert.equal(offered(listed, ''), true);
  assert.equal(offered(listed, 'something-newer'), false);
});

test('a usual one listed by a word of its own that names a model of the list is one line, by that model\'s name', () => {
  const named = [
    { id: 'default', label: 'Default (recommended)', note: 'Big · the most capable, for hard work', default: true },
    { id: 'big', label: 'Big', note: 'For hard work', default: false },
    { id: 'small', label: 'Small', note: 'For quick answers', default: false },
  ];
  assert.deepEqual(choices(named), [
    { value: '', label: 'Big (recommended)', note: 'For hard work' },
    { value: 'small', label: 'Small', note: 'For quick answers' },
  ]);
  // Said by its label, and marked only as the usual one where nothing says recommended.
  const plain = [
    { id: 'usual', label: 'Default (Big 2)', note: '', default: true },
    { id: 'big', label: 'Big', note: '', default: false },
    { id: 'big2', label: 'Big 2', note: 'The newer', default: false },
  ];
  assert.deepEqual(choices(plain).map((one) => [one.value, one.label]), [
    ['', 'Big 2 (default)'],
    ['big', 'Big'],
  ]);
  // A name with a dot in it is that name, and a near one is not it.
  const dotted = [
    { id: 'default', label: 'Default (recommended)', note: 'Model 5.5 · for most things', default: true },
    { id: 'm55', label: 'Model 5.5', note: '', default: false },
    { id: 'm515', label: 'Model 5x5', note: '', default: false },
  ];
  assert.deepEqual(choices(dotted).map((one) => [one.value, one.label]), [
    ['', 'Model 5.5 (recommended)'],
    ['m515', 'Model 5x5'],
  ]);
  // A slot that names that very model keeps its own line, so that what it was given is shown.
  assert.deepEqual(choices(named, 'big').map((one) => [one.value, one.label]), [
    ['', 'Default (recommended)'],
    ['big', 'Big'],
    ['small', 'Small'],
  ]);
  assert.equal(offered(named, 'big'), true);
  assert.equal(offered(named, 'default'), false);
});

test('a usual one that names no model of the list is a line of its own, as it is listed', () => {
  assert.deepEqual(choices(listed)[0], { value: '', label: 'Default (recommended)', note: 'The big one' });
  // Its own name is the usual one's: nothing to put together.
  const own = [
    { id: 'big', label: 'Big', note: 'For hard work', default: true },
    { id: 'small', label: 'Small', note: 'For quick answers', default: false },
  ];
  assert.deepEqual(choices(own).map((one) => [one.value, one.label]), [
    ['', 'Big'],
    ['small', 'Small'],
  ]);
});

import assert from 'node:assert/strict';
import { test } from 'node:test';
import { askingKey, askingOf, forgetAsking, keepAsking, nextName, unasked, way } from './waysasked.ts';

test('what is asked where nothing was written is two ways, each as the blueprint is written', () => {
  const asking = askingOf(askingKey('work', 'shop'));
  assert.deepEqual(asking, unasked());
  assert.deepEqual(
    asking.ways.map((one) => [one.name, one.blueprint, one.share]),
    [
      ['a', '', '1'],
      ['b', '', '1'],
    ],
  );
});

test('what was written is there again for the same blueprint of the same project, and for no other', () => {
  const key = askingKey('work', 'shop');
  keepAsking(key, { ...unasked(), name: 'tones', ways: [way('theirs'), { ...way('mine'), hands: 'tone=kind' }] });
  assert.equal(askingOf(key).name, 'tones');
  assert.equal(askingOf(key).ways[1]?.hands, 'tone=kind');
  assert.equal(askingOf(askingKey('work', 'blog')).name, '');
  assert.equal(askingOf(askingKey('work', null)).name, '');
  assert.equal(askingOf(askingKey('other', 'shop')).name, '');
  // Started, it is let go of.
  keepAsking(key, null);
  assert.deepEqual(askingOf(key), unasked());
});

test('everything written is let go of when whoever was signed in no longer is', () => {
  keepAsking(askingKey('work', 'shop'), { ...unasked(), name: 'tones' });
  keepAsking(askingKey('work', null), { ...unasked(), name: 'models' });
  forgetAsking();
  assert.equal(askingOf(askingKey('work', 'shop')).name, '');
  assert.equal(askingOf(askingKey('work', null)).name, '');
});

test('a way that is added is called by the first letter no way has', () => {
  assert.equal(nextName([way('a'), way('b')]), 'c');
  assert.equal(nextName([way('theirs'), way('mine')]), 'a');
  assert.equal(nextName([way('a'), way('c')]), 'b');
  // One taken out and another added does not give two ways one name.
  assert.equal(nextName([way('a'), way('c'), way('b')]), 'd');
});

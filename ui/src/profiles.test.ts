import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import { alone, places, plain, titleOf, titleShowing } from './profiles.ts';
import type { Profile } from './profiles.ts';

const here = (file: string) => readFileSync(new URL(file, import.meta.url), 'utf8');

const one = (name: string, more: Partial<Profile> = {}): Profile => ({ name, this: false, url: null, why: null, ...more });

test('the tab says which factory it is, and a factory with no profile is called what it always was', () => {
  assert.equal(titleOf('Factory', 'dev1'), 'dev1 · Factory');
  assert.equal(titleOf('Factory', null), 'Factory');
  assert.equal(titleOf('Factory', undefined), 'Factory', 'a daemon from before profiles says nothing of one');
  assert.equal(titleOf('Factory', ''), 'Factory');
});

test('the factory the page is in is listed first, and the others by name', () => {
  const listed = places([
    one('staging', { url: 'http://127.0.0.1:7802' }),
    one('prod', { this: true }),
    one('dev1', { url: 'http://127.0.0.1:7801/' }),
  ]);
  assert.deepEqual(
    listed.map((place) => [place.name, place.here, place.to, place.says]),
    [
      ['prod', true, null, 'You are here.'],
      ['dev1', false, 'http://127.0.0.1:7801/', '127.0.0.1:7801'],
      ['staging', false, 'http://127.0.0.1:7802', '127.0.0.1:7802'],
    ],
  );
});

test('one that cannot be opened from where the page is says why, and goes nowhere', () => {
  const why = 'This page came through factory.example:8443, which leads to this factory and to no other.';
  const [, other] = places([one('prod', { this: true }), one('dev1', { why })]);
  assert.deepEqual([other!.to, other!.says], [null, why]);
  // The one the page is in is never somewhere to go, whatever is said of it.
  assert.equal(places([one('prod', { this: true, url: 'http://127.0.0.1:7700' })])[0]!.to, null);
});

test('two factories of one name are two lines, told apart by where they are', () => {
  const listed = places([one('dev1', { url: 'http://127.0.0.1:7802' }), one('dev1', { this: true }), one('dev1', { url: 'http://127.0.0.1:7801' })]);
  assert.deepEqual(
    listed.map((place) => place.says),
    ['You are here.', '127.0.0.1:7801', '127.0.0.1:7802'],
  );
  assert.equal(new Set(listed.map((place) => place.key)).size, 3, 'a name is not what tells them apart');
});

test('a factory with no other beside it has nowhere else to go', () => {
  assert.equal(alone([one('dev1', { this: true })]), true);
  assert.equal(alone([]), true);
  assert.equal(alone([one('dev1', { this: true }), one('prod', { why: 'not from here' })]), false);
});

test('an address is read without what every address starts with', () => {
  assert.equal(plain('http://127.0.0.1:7801'), '127.0.0.1:7801');
  assert.equal(plain('https://box.example.ts.net:8444/'), 'box.example.ts.net:8444');
});

test('no profile is drawn differently for what it is called', () => {
  // `prod` is a word somebody chose. Nothing may give it a colour, or a look, of its own.
  for (const file of ['./where.css', './phone.css', './views/where.tsx', './profiles.ts']) {
    const text = here(file).replace(/\/\/.*|\/\*[\s\S]*?\*\//g, '');
    assert.ok(!/\b(prod|production|dev\d*|staging)\b/i.test(text), `${file} names a profile`);
  }
  // And what says where the page is uses the theme's colours and no other.
  const style = here('./where.css').replace(/\/\*[\s\S]*?\*\//g, '');
  assert.ok(!/#[0-9a-f]{3,8}\b|rgba?\(|hsla?\(/i.test(style), 'where.css names a colour of its own');
});

test('in a browser a tab says the thing it shows after the profile and before the product, and the start page the product alone', () => {
  assert.equal(titleShowing('Factory', 'dev1', 'Board of Shop'), 'dev1 · Board of Shop · Factory');
  assert.equal(titleShowing('Factory', null, 'Board of Shop'), 'Board of Shop · Factory');
  assert.equal(titleShowing('Factory', '', 'Settings'), 'Settings · Factory');
  assert.equal(titleShowing('Factory', 'dev1', null), 'dev1 · Factory');
  assert.equal(titleShowing('Factory', null, null), 'Factory');
});

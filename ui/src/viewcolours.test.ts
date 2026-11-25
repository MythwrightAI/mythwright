import assert from 'node:assert/strict';
import { test } from 'node:test';
import { viewColours } from './viewcolours.ts';

const theme: Record<string, string> = { text: '#eee', dim: '#999', faint: '#666', line: '#333', accent: '#d73', ok: '#3a7', bad: '#d44', card: '#fc3', info: '#3ac', agent: '#a6f', bg0: '#000', bg1: '#111', bg2: '#222' };
const told = viewColours((name) => theme[name] ?? '');

test('a page is told a card is the ground of a box, not the yellow the client marks a piece of work with', () => {
  assert.equal(told.card, '#222');
  assert.equal(told.panel, '#111');
  assert.equal(told.bg, '#000');
});

test('the yellow is still told, under a name nobody would paint a box with', () => {
  assert.equal(told.warn, '#fc3');
});

test('every colour a page is told is one the theme has', () => {
  for (const [name, value] of Object.entries(told)) assert.notEqual(value, '', `${name} is told as nothing`);
});

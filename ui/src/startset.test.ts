import assert from 'node:assert/strict';
import { test } from 'node:test';
import type { GraphParam } from './api.ts';
import { askedOf, askedWith, changedOf, nameOf, ownRow, savedRows, writtenAfter } from './startset.ts';
import type { Start } from './starts.ts';

const param = (name: string, type: string, more: Partial<GraphParam> = {}): GraphParam => ({ name, type, default: null, required: false, description: null, choices: [], ...more });
// Things nobody ships: what holds is about the asking, not about what is set.
const SETTABLE = {
  ledger: [param('pages', 'int', { default: 10 }), param('ruled', 'bool', { default: false }), param('cover', 'string')],
  shelf: [param('rows', 'list', { default: ['top', 'bottom'] })],
  lamp: [],
};
const titled = (name: string) => name[0]!.toUpperCase() + name.slice(1);

test('what nobody set up is asked for by its name alone', () => {
  assert.deepEqual(askedOf(['ledger', 'shelf', 'lamp'], SETTABLE, {}, titled), { asked: ['ledger', 'shelf', 'lamp'] });
});

test('what somebody wrote is handed on read by its type, and nothing they did not write', () => {
  let written = writtenAfter({}, 'ledger', 'pages', '40');
  written = writtenAfter(written, 'shelf', 'rows', '["one"]');
  assert.deepEqual(askedOf(['ledger', 'shelf'], SETTABLE, written, titled), {
    asked: [
      { name: 'ledger', with: { pages: 40 } },
      { name: 'shelf', with: { rows: ['one'] } },
    ],
  });
});

test('a box written in and emptied again says nothing', () => {
  const written = writtenAfter({}, 'ledger', 'cover', '');
  assert.deepEqual(askedOf(['ledger'], SETTABLE, written, titled), { asked: ['ledger'] });
});

test('what is set up of something that is off is not asked for', () => {
  const written = writtenAfter({}, 'ledger', 'pages', '40');
  assert.deepEqual(askedOf(['shelf'], SETTABLE, written, titled), { asked: ['shelf'] });
});

test('what cannot be read is said with whose setting it is, and nothing is asked', () => {
  const written = writtenAfter({}, 'ledger', 'pages', 'many');
  assert.deepEqual(askedOf(['ledger'], SETTABLE, written, titled), { wrong: 'Ledger: `pages` should be a number.', at: 'ledger' });
});

test('how many settings are changed counts only what reads otherwise than it starts', () => {
  const startsAs = (one: GraphParam) => (one.default === null ? '' : typeof one.default === 'string' ? one.default : JSON.stringify(one.default));
  assert.equal(changedOf(SETTABLE.ledger, undefined, startsAs), 0);
  assert.equal(changedOf(SETTABLE.ledger, { pages: '10', ruled: 'true' }, startsAs), 1);
});

const row = (name: string, more: Partial<Start> = {}): Start => ({ name, title: titled(name), says: '', blueprint: `shop/${name}`, on: true, needs: [], why: '', ...more });

test('a row of one’s own is called by the last part of its blueprint, numbered where that is taken', () => {
  assert.deepEqual(ownRow('std/docs', ['board']), { name: 'docs', title: 'docs', says: 'std/docs', blueprint: 'std/docs', on: true, needs: [], why: '', offered: 'adding' });
  assert.equal(ownRow('std/docs', ['docs']).name, 'docs-2');
  assert.equal(ownRow('std/docs', ['docs', 'docs-2']).name, 'docs-3');
  assert.equal(ownRow('research/Ask Things', []).name, 'ask-things');
  assert.equal(ownRow('@@', []).name, 'row');
});

test('a row of one’s own is asked for whole, with what was written of it, and the rest by name', () => {
  const rows = [row('ledger'), row('notes', { blueprint: 'std/docs', says: 'std/docs' })];
  const settable = { ...SETTABLE, notes: [param('folder', 'string', { default: 'docs' })] };
  const written = writtenAfter({}, 'notes', 'folder', 'kept');
  assert.deepEqual(askedWith(['ledger', 'notes'], rows, ['notes'], settable, written, titled), {
    asked: ['ledger', { name: 'notes', title: 'Notes', says: 'std/docs', blueprint: 'std/docs', with: { folder: 'kept' } }],
  });
  assert.deepEqual(askedWith(['notes'], rows, ['notes'], settable, {}, titled), {
    asked: [{ name: 'notes', title: 'Notes', says: 'std/docs', blueprint: 'std/docs', with: {} }],
  });
});

test('a setup is saved as every row of the table, on as its switch is, handed what was written over what it said', () => {
  const rows = [row('ledger', { with: { cover: 'red' } }), row('shelf', { on: false }), row('lamp', { on: false, offered: 'ledger' })];
  const written = writtenAfter({}, 'ledger', 'pages', '40');
  assert.deepEqual(savedRows(rows, ['shelf'], SETTABLE, written, titled), {
    rows: [
      { ...rows[0], on: false, with: { cover: 'red', pages: 40 } },
      { ...rows[1], on: true, with: {} },
      // Offered elsewhere, it keeps what it said.
      { ...rows[2], on: false, with: {} },
    ],
  });
  assert.deepEqual(savedRows(rows, [], SETTABLE, writtenAfter({}, 'ledger', 'pages', 'many'), titled), { wrong: 'Ledger: `pages` should be a number.', at: 'ledger' });
});

test('a name written by a person comes to small letters, digits and dashes', () => {
  assert.equal(nameOf('A research project'), 'a-research-project');
  assert.equal(nameOf('  --Lab 2!  '), 'lab-2');
  assert.equal(nameOf('???'), '');
});

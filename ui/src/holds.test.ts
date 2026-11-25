import { strict as assert } from 'node:assert';
import { test } from 'node:test';
import { type Area, choices, commandFor, covers, mayChange, mayGive, reads, rowOf, saidApart, saidOf, scopesAfter, tableOf } from './holds.ts';

// As the daemon lists them: areas with parts, and areas that are one thing.
const AREAS: Area[] = [
  { area: 'admin', does: 'manage accounts', verbs: [] },
  { area: 'board', verbs: [{ verb: 'read', does: 'read work items' }, { verb: 'write', does: 'create, change and move work items' }] },
  { area: 'blueprints', verbs: [{ verb: 'read', does: 'read blueprints' }, { verb: 'run', does: 'start, apply and stop' }] },
  { area: 'secrets', verbs: [{ verb: 'use', does: 'use what is kept secret' }] },
  { area: 'emit', does: 'say an event from outside', verbs: [] },
  { area: 'shell', does: 'run commands on a device', verbs: [] },
];
const row = (scopes: string[], area: string) => tableOf(scopes, AREAS).find((one) => one.area === area)!;
const says = (scopes: string[], area: string) => {
  const held = row(scopes, area);
  return reads(AREAS.find((one) => one.area === area)!, held.verbs, held.only);
};

test('an area alone is everything in it, and an area with what is done is only that', () => {
  assert.equal(says(['board'], 'board'), 'read and change');
  assert.equal(says(['board:read'], 'board'), 'read');
  assert.equal(says(['board:write'], 'board'), 'change');
  assert.equal(says(['board:read', 'board:write'], 'board'), 'read and change');
  assert.equal(row(['board:read', 'board:write'], 'board').whole, true);
  assert.equal(says(['blueprints:read'], 'blueprints'), 'read');
  assert.equal(says(['blueprints'], 'blueprints'), 'read and run');
  assert.equal(says(['secrets:use'], 'secrets'), 'use');
});

test('what is not held is said as plainly as what is', () => {
  const table = tableOf(['board:read'], AREAS);
  assert.deepEqual(table.map((one) => one.area), AREAS.map((area) => area.area), 'a row for every area there is');
  assert.equal(says(['board:read'], 'blueprints'), 'nothing');
  assert.equal(says(['board:read'], 'shell'), 'nothing');
  assert.equal(says(['shell'], 'shell'), 'yes');
});

test('everything is every area, whole', () => {
  const table = tableOf(['*'], AREAS);
  assert.ok(table.every((one) => one.whole), JSON.stringify(table));
  assert.equal(says(['*'], 'board'), 'read and change');
  assert.equal(says(['*'], 'admin'), 'yes');
});

test('a scope under a word of its own, or of an area nobody listed, is shown as it is written', () => {
  assert.equal(says(['emit:shop.sold'], 'emit'), 'only shop.sold');
  assert.equal(row(['emit:shop.sold'], 'emit').whole, false);
  const table = tableOf(['board', 'weather:read', 'tides'], AREAS);
  const weather = table.find((one) => one.area === 'weather')!;
  assert.deepEqual(weather.only, ['read']);
  assert.equal(table.find((one) => one.area === 'tides')!.whole, true);
  assert.equal(table.length, AREAS.length + 2);
});

test('a row offers nothing and then each thing with all before it, and what is held some other way', () => {
  const board = AREAS[1]!;
  assert.deepEqual(choices(board, ['read']).map((one) => one.reads), ['nothing', 'read', 'read and change']);
  assert.deepEqual(choices(board, ['write']).map((one) => one.reads), ['nothing', 'read', 'read and change', 'change']);
  assert.deepEqual(choices(AREAS[2]!, []).map((one) => one.reads), ['nothing', 'read', 'read and run']);
  assert.deepEqual(choices(AREAS[5]!, ['all']).map((one) => one.reads), ['nothing', 'yes']);
});

test('a changed row is written back as the whole list, the short way', () => {
  const usual = ['board', 'blueprints:read'];
  assert.deepEqual(scopesAfter(usual, AREAS, 'board', ['read']), ['board:read', 'blueprints:read']);
  assert.deepEqual(scopesAfter(usual, AREAS, 'board', []), ['blueprints:read']);
  assert.deepEqual(scopesAfter(usual, AREAS, 'blueprints', ['read', 'run']), ['board', 'blueprints']);
  assert.deepEqual(scopesAfter(usual, AREAS, 'shell', ['all']), ['board', 'blueprints:read', 'shell']);
  // Changed to what it was, it is the list it was, however that was written.
  assert.deepEqual(scopesAfter(['board:read', 'board:write'], AREAS, 'blueprints', []), ['board']);
  // What the list does not know, and what is held under a word of its own, is kept.
  assert.deepEqual(scopesAfter(['emit:shop.sold', 'weather:read', 'board'], AREAS, 'board', ['read']), ['board:read', 'emit:shop.sold', 'weather:read']);
  assert.deepEqual(scopesAfter(['emit:shop.sold'], AREAS, 'emit', []), []);
  assert.deepEqual(scopesAfter(['emit:shop.sold'], AREAS, 'emit', ['all']), ['emit']);
  // Everything, less one thing, is every area written out.
  assert.deepEqual(scopesAfter(['*'], AREAS, 'board', ['read']), ['admin', 'board:read', 'blueprints', 'secrets', 'emit', 'shell']);
});

test('nobody is offered to give what they do not hold', () => {
  const board = AREAS[1]!;
  assert.equal(mayGive(['board:read'], board, ['read']), true);
  assert.equal(mayGive(['board:read'], board, ['read', 'write']), false);
  assert.equal(mayGive(['board'], board, ['read', 'write']), true);
  assert.equal(mayGive(['*'], AREAS[0]!, ['all']), true);
  assert.equal(mayGive(['board'], AREAS[5]!, ['all']), false);
  assert.equal(mayGive(['board'], AREAS[5]!, []), true, 'taking away takes holding nothing');
});

test('an agent is one of the row its fleet called it after', () => {
  const rows = [{ data: { title: 'worker', holds: [] } }, { data: { title: 'night-worker', holds: ['board:read'] } }, { data: { title: 'architect' } }];
  assert.equal(rowOf('shop-worker-2', 'shop', rows), rows[0]);
  assert.equal(rowOf('shop-night-worker-1', 'shop', rows), rows[1]);
  assert.equal(rowOf('shop-builder', 'shop', rows), null, 'one added by hand has no number after a row');
  assert.equal(rowOf('depot-worker-1', 'shop', rows), null);
  assert.equal(rowOf('shop-worker-1', null, rows), null);
  assert.equal(saidOf(rows[0]!), null, 'a row that says nothing leaves it to the usual');
  assert.deepEqual(saidOf(rows[1]!), ['board:read']);
  assert.equal(saidOf(rows[2]!), null);
});

test('what an agent holds is changed by whoever may change agents, and by its row where it is one of a fleet\'s', () => {
  assert.equal(mayChange(['agents'], false), true, 'one in no row is changed by itself');
  assert.equal(mayChange(['agents:write'], false), true);
  assert.equal(mayChange(['agents:read', 'records'], false), false);
  assert.equal(mayChange(['agents'], true), false, 'a row is a record, and changing one takes that too');
  assert.equal(mayChange(['agents:write', 'records:write'], true), true);
  assert.equal(mayChange(['*'], true), true);
  assert.equal(covers(['board'], 'board:write'), true);
  assert.equal(covers(['board:read'], 'board:write'), false);
});

test('the command that says what an agent holds says the whole of it', () => {
  assert.equal(commandFor('mw', 'helper', ['board', 'blueprints']), 'mw agent add helper --scope board --scope blueprints');
  assert.equal(commandFor('mw', 'helper', []), 'mw agent add helper');
  const mine = saidApart(true, 'mw', 'helper', ['board']);
  assert.ok(mine.includes('change a line here') && mine.endsWith('mw agent add helper --scope board'), mine);
  const theirs = saidApart(false, 'mw', 'helper', ['board']);
  assert.ok(theirs.includes('Whoever may change agents') && theirs.endsWith('mw agent add helper --scope board'), theirs);
  assert.ok(!theirs.includes('--scope …'), 'no flag is pointed at without saying what goes after it');
});

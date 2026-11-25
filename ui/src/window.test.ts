import assert from 'node:assert/strict';
import { test } from 'node:test';
import { ALL, anchorAt, CAP, countOf, fromOf, keyOf, markOf, readOf, shifted, keptRead, drawn, endOf, heightOf, held, joined, kept, LIMIT, measured, missing, paged, rankAt, roomFor, rowAt, topOf, totalOf, yOf } from './window.ts';
import type { Run, Segment } from './window.ts';
import type { Row } from './table.ts';

const row = (number: number): Row => ({ id: `t-${number}`, number, data: {}, position: number, created: 0, updated: 0 });
const rows = (from: number, to: number): Row[] => Array.from({ length: to - from }, (_, nth) => row(from + nth + 1));
const run = (start: number, end: number, before?: string | null, after?: string | null): Run => ({ start, rows: rows(start, end), before, after });

test('a read that touches a run is joined to it, and reading on and back begins where whichever reaches furthest says', () => {
  const runs = joined([run(0, 100, null, 'a100')], run(100, 200, 'b100', 'a200'));
  assert.equal(runs.length, 1);
  assert.equal(runs[0]!.start, 0);
  assert.equal(runs[0]!.rows.length, 200);
  assert.equal(runs[0]!.before, null);
  assert.equal(runs[0]!.after, 'a200');
  assert.equal(rowAt(runs, 150)!.id, 't-151');
});

test('a read that lies over a run stands over its rows, and one apart from every run is a run of its own, in order', () => {
  const over = joined([run(0, 100, null, 'a100')], { start: 50, rows: rows(50, 60).map((one) => ({ ...one, data: { new: true } })), before: 'b50', after: 'a60' });
  assert.equal(over.length, 1);
  assert.deepEqual(rowAt(over, 55)!.data, { new: true });
  assert.equal(over[0]!.after, 'a100');
  const apart = joined([run(500, 600, 'b500', 'a600')], run(0, 100, null, 'a100'));
  assert.deepEqual(apart.map((one) => one.start), [0, 500]);
  // Three joined by one that touches both.
  const bridged = joined(apart, run(100, 500, 'b100', 'a500'));
  assert.equal(bridged.length, 1);
  assert.equal(endOf(bridged[0]!), 600);
  assert.equal(held(bridged), 600);
});

test('a read of no rows changes no run', () => {
  const runs = [run(0, 10, null, 'a10')];
  assert.equal(joined(runs, { start: 10, rows: [], after: null }), runs);
});

test('what is asked next: from the start, on from a run, back from a run a window away, or from a rank', () => {
  assert.deepEqual(missing([], 0, 50, 1000), { start: true, rank: 0 });
  assert.deepEqual(missing([run(0, 100, null, 'a100')], 80, 150, 1000), { after: 'a100', rank: 100 });
  // A gap a window fills before a run that knows where reading back begins.
  assert.deepEqual(missing([run(500, 600, 'b500', 'a600')], 450, 520, 1000), { before: 'b500', rank: 500 });
  // Too far before it: from the rank.
  assert.deepEqual(missing([run(500, 600, 'b500', 'a600')], 300, 520, 1000), { at: 300, rank: 300 });
  // A run cut short knows no place that way.
  assert.deepEqual(missing([run(0, 100, null, undefined)], 90, 120, 1000), { at: 100, rank: 100 });
  // A run of a read that went through to the end of the order says nothing more is there.
  assert.deepEqual(missing([run(0, 100, null, null)], 90, 120, 1000), { at: 100, rank: 100 });
});

test('nothing is asked where every row is there, past the count, or past where the rows are known to end', () => {
  assert.equal(missing([run(0, 100, null, 'a100')], 10, 90, 1000), null);
  assert.equal(missing([run(0, 100, null, null)], 90, 200, 100), null);
  assert.equal(missing([run(0, 100, null, 'a100')], 90, 200, 1000, 100), null);
  assert.equal(missing([], 0, 10, 0), null);
});

test('past what is kept the furthest rows are let go of, and a run cut knows no place at the cut', () => {
  const runs = [run(0, 400, null, 'a400'), run(1000, 1400, 'b1000', 'a1400')];
  const left = kept(runs, 1100, 1150, 600);
  assert.ok(held(left) <= 600);
  assert.equal(rowAt(left, 0), undefined);
  assert.ok(rowAt(left, 1125));
  const cut = left.find((one) => one.start < 1100)!;
  assert.equal(cut.before, cut.start === 1000 ? 'b1000' : undefined);
  assert.deepEqual(kept(runs, 0, 10, 1000), runs);
});

test('where a row begins and which row is at a height, with rows measured otherwise than the rest', () => {
  assert.equal(topOf(30, [], 10), 300);
  assert.equal(rankAt(30, [], 300, 100), 10);
  assert.equal(rankAt(30, [], 299, 100), 9);
  assert.equal(rankAt(30, [], 99999, 100), 99);
  assert.equal(rankAt(30, [], -5, 100), 0);
  // The row of rank 2 is opened into what names it, and is 130 tall.
  const odd: [number, number][] = [[2, 130]];
  assert.equal(topOf(30, odd, 2), 60);
  assert.equal(topOf(30, odd, 3), 190);
  assert.equal(heightOf(30, odd, 2), 130);
  assert.equal(heightOf(30, odd, 3), 30);
  assert.equal(rankAt(30, odd, 100, 100), 2);
  assert.equal(rankAt(30, odd, 189, 100), 2);
  assert.equal(rankAt(30, odd, 190, 100), 3);
  assert.equal(rankAt(30, odd, 220, 100), 4);
});

test('a row measured as tall as the rest is kept as nothing, and past so many the furthest are let go of', () => {
  assert.deepEqual(measured(30, [], 4, 30), []);
  assert.deepEqual(measured(30, [], 4, 60), [[4, 60]]);
  assert.deepEqual(measured(30, [[4, 60]], 4, 30), []);
  assert.deepEqual(measured(30, [[9, 40]], 4, 60), [[4, 60], [9, 40]]);
  assert.deepEqual(measured(30, [[0, 40], [500, 40]], 499, 50, 450, 520, 2), [[499, 50], [500, 40]]);
});

const segment = (key: string, count: number, head = 0, open = true): Segment => ({ key, head, count, open, odd: [] });

test('what is drawn is what is on the screen and so many rows either side, and where it begins and ends', () => {
  const one = [segment('', 10_000)];
  assert.equal(totalOf(one, 30), 300_000);
  const shown = drawn(one, 30, 30_000, 30_600, 40);
  assert.deepEqual(shown.pieces, [{ key: '', head: false, from: 960, to: 1061 }]);
  assert.equal(shown.top, 960 * 30);
  assert.equal(shown.bottom, 1061 * 30);
  // At the very top, nothing before the first row.
  assert.deepEqual(drawn(one, 30, 0, 600, 40).pieces, [{ key: '', head: false, from: 0, to: 61 }]);
  assert.equal(drawn([], 30, 0, 600).bottom, 0);
});

test('bands: a head each, rows under it only where it is open, drawn in order and following on', () => {
  const bands = [segment('ready', 3, 40), segment('doing', 1000, 40, false), segment('', 50, 40)];
  assert.equal(totalOf(bands, 30), 40 + 90 + 40 + 40 + 1500);
  const shown = drawn(bands, 30, 0, 400, 2);
  assert.deepEqual(shown.pieces, [
    { key: 'ready', head: true },
    { key: 'ready', head: false, from: 0, to: 3 },
    { key: 'doing', head: true },
    { key: '', head: true },
    { key: '', head: false, from: 0, to: 9 },
  ]);
  assert.equal(shown.top, 0);
  assert.equal(yOf(bands, 30, '', 0), 210);
  assert.equal(yOf(bands, 30, 'doing', 0), null);
  assert.equal(yOf(bands, 30, 'doing', -1), 130);
  assert.deepEqual(anchorAt(bands, 30, 135), { key: 'doing', rank: -1, above: 5 });
  assert.deepEqual(anchorAt(bands, 30, 245), { key: '', rank: 1, above: 5 });
  assert.equal(anchorAt(bands, 30, 99_999), null);
});

test('the room is as tall as the rows, and no taller than what a browser draws', () => {
  assert.equal(roomFor(1000), 1000);
  assert.equal(roomFor(32_000_000), CAP);
});

test('under the cap the room is the rows, and the page is never moved under the reader', () => {
  assert.deepEqual(paged({ top: 0, base: 0 }, 5000, 100_000, 800), { base: 0 });
});

test('past the cap, a small move is a move by its pixels, and a jump goes to the same part of all the rows', () => {
  const total = 32_000_000;
  const cap = 8_000_000;
  // Moved a little from where the room's top stood a long way in.
  assert.deepEqual(paged({ top: 3_000_000, base: 9_000_000 }, 3_000_120, total, 800, cap), { base: 9_000_000 });
  // The scrollbar dragged to 73% of the room is 73% of the rows.
  const jumped = paged({ top: 0, base: 0 }, Math.round(0.73 * (cap - 800)), total, 800, cap);
  const y = Math.round(0.73 * (cap - 800)) + jumped.base;
  assert.ok(Math.abs(y / (total - 800) - 0.73) < 0.0001);
  assert.equal(jumped.put, undefined);
  // The end of the room is the end of the rows.
  assert.deepEqual(paged({ top: 0, base: 0 }, cap - 800, total, 800, cap), { base: total - cap });
});

test('past the cap, near an edge of the room with rows beyond it, what scrolls is put back where the same part of the room is and nothing moves under the reader', () => {
  const total = 32_000_000;
  const cap = 8_000_000;
  const near = paged({ top: 1000, base: 10_000_000 }, 900, total, 800, cap);
  assert.ok(near.put !== undefined && near.put > 1000);
  assert.equal(near.put! + near.base, 10_000_900);
  // At the very top of the rows nothing is moved.
  assert.deepEqual(paged({ top: 100, base: 0 }, 0, total, 800, cap), { base: 0 });
  // At the very end, neither.
  assert.deepEqual(paged({ top: cap - 900, base: total - cap }, cap - 800, total, 800, cap), { base: total - cap });
});

test('a window is a hundred rows', () => assert.equal(LIMIT, 100));

test('what tells one asking from another: the view, the sort and what the view says', () => {
  const plain = markOf({ view: 'All', sort: null, says: { only: null } });
  assert.equal(markOf({ view: 'All', sort: null, says: { only: null } }), plain);
  assert.notEqual(markOf({ view: 'All', sort: { field: 'points', down: false }, says: { only: null } }), plain);
  assert.notEqual(markOf({ view: 'All', sort: { field: 'points', down: true }, says: { only: null } }), markOf({ view: 'All', sort: { field: 'points', down: false }, says: { only: null } }));
  assert.notEqual(markOf({ view: 'All', sort: null, says: { only: { status: ['ready'] } } }), plain);
  assert.notEqual(markOf({ view: null, sort: null, says: { only: null } }), plain);
  // What is searched for tells one reading from another, and a search for nothing is none.
  assert.notEqual(markOf({ view: 'All', sort: null, says: { only: null }, find: 'ab' }), plain);
  assert.equal(markOf({ view: 'All', sort: null, says: { only: null }, find: '  ' }), plain);
});

test('a band is kept by what it is of, and a view gathered by none by one key of its own', () => {
  assert.equal(keyOf(null), ALL);
  assert.equal(keyOf(''), '=');
  assert.notEqual(keyOf('*'), ALL);
});

const counted = { count: 12, of: 40, bands: [{ key: 'ready', count: 5 }, { key: '', count: 7 }], more_bands: 0, figures: {}, whole: true };

test('how many rows a band has: every row the view shows, a band its own, and none of a band not counted', () => {
  assert.equal(countOf(counted, null), 12);
  assert.equal(countOf(counted, 'ready'), 5);
  assert.equal(countOf(counted, ''), 7);
  assert.equal(countOf(counted, 'gone'), 0);
  assert.equal(countOf(null, null), null);
});

test('what is said to the daemon for what is asked next', () => {
  assert.deepEqual(fromOf({ start: true, rank: 0 }), { start: true });
  assert.deepEqual(fromOf({ after: 'a', rank: 100 }), { after: 'a' });
  assert.deepEqual(fromOf({ before: 'b', rank: 500 }), { before: 'b' });
  assert.deepEqual(fromOf({ at: 300, rank: 300 }), { at: 300 });
});

const answer = (count: number, before: string | null, after: string | null, rank?: number) => ({ rows: rows(0, count), before, after, looked: count, short: false, rank });

test('where what an answer read stands: on from a run after it, back from one before it, and at the rank it was asked', () => {
  assert.deepEqual(readOf({ start: true, rank: 0 }, answer(100, null, 'a')).run.start, 0);
  assert.equal(readOf({ after: 'x', rank: 100 }, answer(100, 'b', 'a')).run.start, 100);
  assert.equal(readOf({ before: 'x', rank: 500 }, answer(100, 'b', 'a')).run.start, 400);
  assert.equal(readOf({ at: 300, rank: 300 }, answer(100, 'b', 'a', 300)).run.start, 300);
  const read = readOf({ after: 'x', rank: 100 }, answer(40, 'b', 'a'));
  assert.equal(read.run.before, 'b');
  assert.equal(read.run.after, 'a');
});

test('where the rows end: a read on that went through to the end says so, a read back or one that goes on does not', () => {
  assert.equal(readOf({ after: 'x', rank: 100 }, answer(40, 'b', null)).end, 140);
  assert.equal(readOf({ start: true, rank: 0 }, answer(0, null, null)).end, 0);
  assert.equal(readOf({ after: 'x', rank: 100 }, answer(100, 'b', 'a')).end, null);
  assert.equal(readOf({ before: 'x', rank: 500 }, answer(100, null, 'a')).end, null);
});

test('what stands at the top of the screen stays there as what is above it is measured, read in or counted again', () => {
  const was = { segments: [segment('', 1000)], each: 30 };
  // The rows above were measured taller: the same row, as far into itself.
  assert.equal(shifted(was, { segments: [segment('', 1000)], each: 34 }, 3015), 3415);
  // A row above opened into what names it.
  assert.equal(shifted(was, { segments: [{ ...segment('', 1000), odd: [[50, 130]] }], each: 30 }, 3015), 3115);
  // Nothing above it changed.
  assert.equal(shifted(was, { segments: [segment('', 2000)], each: 30 }, 3015), 3015);
  // What stood there is folded away now: where it was.
  const banded = { segments: [segment('a', 10, 40), segment('b', 10, 40)], each: 30 };
  assert.equal(shifted(banded, { segments: [segment('a', 10, 40, false), segment('b', 10, 40)], each: 30 }, 50), 50);
  assert.equal(shifted(banded, { segments: [segment('a', 10, 40, false), segment('b', 10, 40)], each: 30 }, 400), 100);
});

test('an answer that found no row before it had looked as far as one answer looks moves where reading goes on from, and nothing else', () => {
  const runs = [run(0, 100, null, 'a100'), run(500, 600, 'b500', 'a600')];
  const short = { rows: [], before: null, after: 'a20100', looked: 20_000, short: true };
  const on = keptRead(runs, { after: 'a100', rank: 100 }, short);
  assert.equal(on[0]!.after, 'a20100');
  assert.deepEqual(missing(on, 100, 150, 1000), { after: 'a20100', rank: 100 });
  const back = keptRead(runs, { before: 'b500', rank: 500 }, { rows: [], before: 'b-far', after: null, looked: 20_000, short: true });
  assert.equal(back[1]!.before, 'b-far');
  // One that read rows joins them; one that found nothing at the end changes no run.
  assert.equal(held(keptRead(runs, { after: 'a100', rank: 100 }, { rows: rows(100, 110), before: 'b', after: 'a110', looked: 10, short: false })), 210);
  assert.equal(keptRead(runs, { after: 'a600', rank: 600 }, { rows: [], before: null, after: null, looked: 0, short: false }), runs);
  // From the start, a run of no rows that knows where to go on from.
  const first = keptRead([], { start: true, rank: 0 }, short);
  assert.deepEqual(missing(first, 0, 50, 1000), { after: 'a20100', rank: 0 });
  const then = keptRead(first, { after: 'a20100', rank: 0 }, { rows: rows(0, 3), before: 'b0', after: 'a3', looked: 900, short: false });
  assert.equal(held(then), 3);
  assert.equal(then.length, 1);
  assert.equal(rowAt(then, 0)!.id, 't-1');
});

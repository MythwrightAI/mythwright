import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import { changed, type Field, type View } from './table.ts';
import { dragged, drawnWhileHeld, fitted, heldRule, NARROWEST, sizedStyle, STEP, stepWidth, WIDEST, widens, widthOf, widthsWith } from './widths.ts';

const field = (name: string, kind = 'text'): Field => ({ name, kind, of: [], required: false });
const fields = [field('title'), field('points', 'number'), field('due', 'date')];
const view = (more: Partial<View> = {}): View => ({ name: 'Table', kind: 'table', by: null, saved: true, ...more });

test('the narrowest and the widest a column may be are what the daemon holds a view to', () => {
  const kernel = readFileSync(new URL('../../crates/kernel/src/columns.rs', import.meta.url), 'utf8');
  assert.match(kernel, new RegExp(`pub const NARROWEST: u32 = ${NARROWEST};`));
  assert.match(kernel, new RegExp(`pub const WIDEST: u32 = ${WIDEST};`));
});

test('a width is kept whole and from the narrowest to the widest, however it is come by', () => {
  assert.equal(fitted(240.4), 240);
  assert.equal(fitted(240.6), 241);
  assert.equal(fitted(3), NARROWEST);
  assert.equal(fitted(-40), NARROWEST);
  assert.equal(fitted(99_999), WIDEST);
  assert.equal(fitted(Number.NaN), NARROWEST);
  assert.equal(fitted(NARROWEST), NARROWEST);
  assert.equal(fitted(WIDEST), WIDEST);
});

test('what a drag comes to is the width it was let go at, at the usual size of text, whatever size the reader has', () => {
  // Taken hold of at 200 and moved 60 to the right, at the usual size.
  assert.equal(dragged(200, 60, 1), 260);
  // To the left.
  assert.equal(dragged(200, -80, 1), 120);
  // With text half as large again, what is drawn 300 wide is kept as 200: another reader sees as much of it.
  assert.equal(dragged(240, 60, 1.5), 200);
  // Past either end it is the end.
  assert.equal(dragged(200, -500, 1), NARROWEST);
  assert.equal(dragged(1900, 900, 1), WIDEST);
  // A size that says nothing is the usual one.
  assert.equal(dragged(200, 10, 0), 210);
  // While it is held it is drawn no narrower or wider than it may be kept at, by the size.
  assert.equal(drawnWhileHeld(10, 1), NARROWEST);
  assert.equal(drawnWhileHeld(10, 2), NARROWEST * 2);
  assert.equal(drawnWhileHeld(5000, 1.25), WIDEST * 1.25);
  assert.equal(drawnWhileHeld(333.5, 1), 333.5);
});

test('what a view says of a width is read only where it is one', () => {
  assert.equal(widthOf(view({ widths: { title: 320 } }), 'title'), 320);
  assert.equal(widthOf(view({ widths: { title: 320 } }), 'points'), null);
  assert.equal(widthOf(view(), 'title'), null);
  assert.equal(widthOf(null, 'title'), null);
  assert.equal(widthOf(view({ widths: { title: 0 } }), 'title'), null);
});

test('a column made a width, or as wide as what is in it, leaves the widths of the others as they were', () => {
  assert.deepEqual(widthsWith(null, 'title', 300), { title: 300 });
  assert.deepEqual(widthsWith({ title: 300, due: 90 }, 'title', 180.7), { title: 181, due: 90 });
  assert.deepEqual(widthsWith({ title: 300, due: 90 }, 'title', null), { due: 90 });
  // None left is nothing said, which a view kept before widths could be said says too.
  assert.equal(widthsWith({ title: 300 }, 'title', null), null);
  assert.equal(widthsWith(undefined, 'title', null), null);
});

test('a width let go of is saved from the view as it is then, carrying everything else it says', () => {
  const now = view({ columns: ['due', 'points'], freezes: 'due', widths: { due: 120 } });
  const sent = changed(fields, [now], 'Table', widens('points', 200));
  assert.deepEqual([sent.columns, sent.freezes, sent.widths], [['due', 'points'], 'due', { due: 120, points: 200 }]);
  // Two let go of in the time one save takes: the second is worked out from what the first came to, and both stand.
  const first = { ...now, widths: sent.widths };
  assert.deepEqual(changed(fields, [first], 'Table', widens('title', 400)).widths, { due: 120, points: 200, title: 400 });
  // A double press: as wide as what is in it, saved as no width for that column.
  assert.deepEqual(changed(fields, [first], 'Table', widens('due', null)).widths, { points: 200 });
  assert.equal(changed(fields, [view({ widths: { due: 120 } })], 'Table', widens('due', null)).widths, null);
});

test('an arrow on a column edge makes it a step wider or narrower than it is, or than it was drawn where the view says nothing', () => {
  // Nothing said: from what it was drawn at.
  assert.deepEqual(stepWidth('title', 1, 241.6)(view(), fields, []).widths, { title: 242 + STEP });
  assert.deepEqual(stepWidth('title', -1, 241.6)(view(), fields, []).widths, { title: 242 - STEP });
  // Said: from what it says, whatever it was drawn at, so that presses made before the last was answered each count.
  assert.deepEqual(stepWidth('title', 1, 999)(view({ widths: { title: 300 } }), fields, []).widths, { title: 300 + STEP });
  // At an end a step changes nothing.
  assert.equal(stepWidth('title', -1, 0)(view({ widths: { title: NARROWEST } }), fields, []).widths, undefined);
  assert.equal(stepWidth('title', 1, 0)(view({ widths: { title: WIDEST } }), fields, []).widths, undefined);
  // From nothing said at the narrowest, a step narrower keeps it there and says so.
  assert.deepEqual(stepWidth('title', -1, 10)(view(), fields, []).widths, { title: NARROWEST });
});

test('a cell says its width by the size of its reader\'s text, and a column held is drawn by one rule over its own cells', () => {
  assert.equal(sizedStyle(null), null);
  assert.equal(sizedStyle(240), '--cw:calc(240px * var(--size))');
  const rule = heldRule('7', 3, 251.333);
  assert.ok(rule.startsWith('.grid[data-width-held="7"] thead tr:last-child > :nth-child(3), '), rule);
  assert.ok(rule.includes('tbody tr[data-row] > :nth-child(3)') && rule.includes('tfoot tr.grid-figures > :nth-child(3)'), rule);
  assert.ok(rule.endsWith('{ width: 251.33px; min-width: 251.33px; max-width: 251.33px; }'), rule);
});

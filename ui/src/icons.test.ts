import assert from 'node:assert/strict';
import { test } from 'node:test';
import { calledIcon, drawingOf, found, iconOf, mayChoose, moved, placed, wantsIcons } from './icons.ts';
import type { IconNode, IconSet } from './icons.ts';

const line = (d: string): IconNode[] => [['path', { d }]];
const set: IconSet = {
  calendar: [line('M1'), 'date,day,month,events'],
  'calendar-days': [line('M2'), 'date,month,year,events'],
  hash: [line('M3'), 'number,pound,hashtag'],
  heart: [line('M4'), 'like,love,favourite'],
  star: [line('M5'), 'favourite,bookmark,rating'],
  'star-off': [line('M6'), 'dislike,unfavourite'],
  'user-star': [line('M7'), 'person,favourite'],
};

/** The same, with some whose names only have what is typed inside them. */
const more: IconSet = { 'align-start': [line('M8'), 'left,flex'], 'book-heart': [line('M9'), 'diary,love'], megastar: [line('M10'), 'big'], ...set };

test('what somebody chose stands over what the table declared, and a field that says neither names none', () => {
  assert.equal(iconOf({}), null);
  assert.equal(iconOf({ icon: 'hash' }), 'hash');
  assert.equal(iconOf({ icon_chosen: 'star' }), 'star');
  assert.equal(iconOf({ icon: 'hash', icon_chosen: 'star' }), 'star');
  // A daemon says nothing, or null, for one that is not there.
  assert.equal(iconOf({ icon: null, icon_chosen: null }), null);
  assert.equal(iconOf({ icon: 'hash', icon_chosen: null }), 'hash');
  assert.equal(wantsIcons([{}, { icon: null }]), false);
  assert.equal(wantsIcons([{}, { icon_chosen: 'star' }]), true);
  assert.equal(wantsIcons([{ icon: 'hash' }]), true);
});

test('a field is drawn as its icon where the set has one of that name, and as the mark of its kind otherwise', () => {
  assert.deepEqual(drawingOf({ icon: 'hash' }, set), line('M3'));
  assert.deepEqual(drawingOf({ icon: 'hash', icon_chosen: 'star' }, set), line('M5'));
  // No icon named: the mark of its kind.
  assert.equal(drawingOf({}, set), null);
  // A name the set has no drawing for: the mark of its kind, and nothing fails.
  assert.equal(drawingOf({ icon_chosen: 'no-such-drawing' }, set), null);
  // What was chosen stands over what was declared though it cannot be drawn: the declared one is not drawn in its place.
  assert.equal(drawingOf({ icon: 'hash', icon_chosen: 'no-such-drawing' }, set), null);
  // A name that is something every object has is no drawing.
  assert.equal(drawingOf({ icon_chosen: 'constructor' }, set), null);
  assert.equal(drawingOf({ icon_chosen: 'toString' }, set), null);
  // Until the set has been read, the mark of its kind.
  assert.equal(drawingOf({ icon_chosen: 'star' }, null), null);
});

test('icons are found by name and by the words they are found by, the nearest name first', () => {
  // Nothing typed: all of them, in the set's order.
  assert.deepEqual(found(set, ''), Object.keys(set));
  assert.deepEqual(found(set, '   '), Object.keys(set));
  // By name: the one called just that, then those it is a part of, in the set's order.
  assert.deepEqual(found(set, 'star'), ['star', 'star-off', 'user-star']);
  assert.deepEqual(found(set, 'OFF'), ['star-off']);
  // Then those a part of whose name it begins, those it is anywhere in, and last those found by their words.
  assert.deepEqual(found(more, 'star'), ['star', 'star-off', 'user-star', 'align-start', 'megastar']);
  assert.deepEqual(found(more, 'heart'), ['heart', 'book-heart']);
  assert.deepEqual(found(more, 'love'), ['book-heart', 'heart']);
  assert.deepEqual(found(more, 'art'), ['align-start', 'book-heart', 'heart']);
  // By a word it is found by, which what is typed begins: it is not looked for inside one.
  assert.deepEqual(found(set, 'date'), ['calendar', 'calendar-days']);
  assert.deepEqual(found(set, 'favourite'), ['heart', 'star', 'user-star']);
  assert.deepEqual(found(set, 'fav'), ['heart', 'star', 'user-star']);
  assert.deepEqual(found(set, 'ourite'), []);
  assert.deepEqual(found(set, 'ha'), ['hash']);
  assert.deepEqual(found(set, 'book'), ['star']);
  // Every word typed has to be found, in the name or among the words; those found by name alone come first.
  assert.deepEqual(found(set, 'calendar year'), ['calendar-days']);
  assert.deepEqual(found(set, 'star rating'), ['star']);
  assert.deepEqual(found(set, 'star fav'), ['star', 'user-star']);
  // A name with a dash is typed with a space, with the dash or with neither.
  assert.deepEqual(found(set, 'calendar days'), ['calendar-days']);
  assert.deepEqual(found(set, 'calendar-days'), ['calendar-days']);
  assert.deepEqual(found(set, 'calendardays'), ['calendar-days']);
  assert.deepEqual(found(set, 'days calendar'), ['calendar-days']);
  // Nothing found.
  assert.deepEqual(found(set, 'zebra'), []);
  assert.deepEqual(found(set, 'star zebra'), []);
  assert.deepEqual(found({}, 'star'), []);
});

test('an icon is called by its name as it is read', () => {
  assert.equal(calledIcon('star'), 'Star');
  assert.equal(calledIcon('calendar-days'), 'Calendar days');
  assert.equal(calledIcon('arrow-down-0-1'), 'Arrow down 0 1');
});

test('choosing is offered to whoever may save a view of the table', () => {
  assert.equal(mayChoose(['*'], { name: 'stock' }), true);
  assert.equal(mayChoose(['records'], { name: 'stock' }), true);
  assert.equal(mayChoose(['records:write'], { name: 'stock' }), true);
  assert.equal(mayChoose(['records:read', 'board'], { name: 'stock' }), false);
  // A board's is its cards'.
  assert.equal(mayChoose(['board:write'], { name: 'board' }), true);
  assert.equal(mayChoose(['records'], { name: 'board' }), false);
  // And what a table says changing its records takes beside that.
  assert.equal(mayChoose(['records'], { name: 'agents', takes: 'shell' }), false);
  assert.equal(mayChoose(['records', 'shell'], { name: 'agents', takes: 'shell' }), true);
  assert.equal(mayChoose(['records'], { name: 'stock', takes: null }), true);
  assert.equal(mayChoose([], { name: 'stock' }), false);
});

test('the arrows move the mark an icon or a row at a time and stop at the edges', () => {
  // Twenty icons, eight to a row: rows of 8, 8 and 4.
  assert.equal(moved(0, 'ArrowRight', 8, 20), 1);
  assert.equal(moved(1, 'ArrowLeft', 8, 20), 0);
  assert.equal(moved(0, 'ArrowLeft', 8, 20), 0);
  assert.equal(moved(19, 'ArrowRight', 8, 20), 19);
  assert.equal(moved(3, 'ArrowDown', 8, 20), 11);
  assert.equal(moved(11, 'ArrowUp', 8, 20), 3);
  assert.equal(moved(3, 'ArrowUp', 8, 20), 3);
  // Down to a last row that is short goes to the last there is; down from the last row stays.
  assert.equal(moved(9, 'ArrowDown', 8, 20), 17);
  assert.equal(moved(14, 'ArrowDown', 8, 20), 19);
  assert.equal(moved(17, 'ArrowDown', 8, 20), 17);
  // The end of one row leads to the start of the next.
  assert.equal(moved(7, 'ArrowRight', 8, 20), 8);
  // Any other key leaves it, and a mark past what is found comes back to the last.
  assert.equal(moved(5, 'a', 8, 20), 5);
  assert.equal(moved(30, 'a', 8, 20), 19);
  assert.equal(moved(4, 'ArrowDown', 8, 0), 0);
  assert.equal(moved(0, 'ArrowDown', 0, 3), 1);
});

test('the panel is put under what opened it and kept wholly in the window', () => {
  const size = { width: 320, height: 364 };
  const wide = { width: 1280, height: 860 };
  // Under it, its left edge where the opener's is.
  assert.deepEqual(placed({ left: 300, top: 100, bottom: 133 }, size, wide), { left: 300, top: 137 });
  // Moved in from the right edge, and from the left.
  assert.deepEqual(placed({ left: 1200, top: 100, bottom: 133 }, size, wide), { left: 952, top: 137 });
  assert.deepEqual(placed({ left: -40, top: 100, bottom: 133 }, size, wide), { left: 8, top: 137 });
  // Over it where there is no room under.
  assert.deepEqual(placed({ left: 300, top: 700, bottom: 733 }, size, wide), { left: 300, top: 332 });
  // And where there is room neither way, as low as is still in the window.
  assert.deepEqual(placed({ left: 300, top: 200, bottom: 233 }, size, { width: 1280, height: 420 }), { left: 300, top: 48 });
  // At a phone's width, as wide as the window lets it be.
  const phone = { width: 390, height: 844 };
  const there = placed({ left: 250, top: 120, bottom: 159 }, { width: 320, height: 364 }, phone);
  assert.deepEqual(there, { left: 62, top: 163 });
  assert.ok(there.left >= 8 && there.left + 320 <= phone.width - 8);
});

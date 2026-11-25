import assert from 'node:assert/strict';
import { test } from 'node:test';
import { drawerScope, forgetAllReturned, forgetReturned, keepsDrawer, putBack, recall, remember, scopes, signOf } from './returned.ts';

const kept = { sign: 'div.messages', nth: 0, top: 600, left: 0 };
const part = (top: number, most: number) => ({ top, left: 0, most, mostLeft: 0 });

test('a part is put back where it was scrolled once it is long enough to be scrolled that far', () => {
  assert.deepEqual(putBack(kept, part(0, 900)), { top: 600, left: 0 });
  assert.deepEqual(putBack(kept, part(0, 600)), { top: 600, left: 0 }, 'just long enough');
  assert.equal(putBack(kept, part(0, 300)), 'wait', 'what it holds is not all drawn yet');
  assert.equal(putBack(kept, null), 'wait', 'not drawn at all yet');
});

test('a part put back is held there while what is above it is laid out, and is there once it is', () => {
  assert.deepEqual(putBack(kept, part(740, 1400), true), { top: 600, left: 0 }, 'moved by the browser as what is above it grew');
  assert.equal(putBack(kept, part(600, 1400), true), 'there');
  assert.equal(putBack(kept, part(601, 1400), true), 'there', 'to the pixel');
  assert.equal(putBack(kept, part(300, 300), true), 'wait', 'shorter for a moment as it is drawn again');
});

test('a part the view scrolled itself is left where the view put it', () => {
  assert.equal(putBack(kept, part(1200, 1200)), 'leave', 'a channel at its newest message');
  assert.equal(putBack({ ...kept, left: 40 }, { top: 0, left: 10, most: 900, mostLeft: 100 }), 'leave', 'and sideways');
});

test('a part is found again by its element and its classes, in whatever order they are written', () => {
  assert.equal(signOf('DIV', ['messages', 'split']), signOf('div', ['split', 'messages']));
  assert.notEqual(signOf('div', ['messages']), signOf('section', ['messages']));
  assert.equal(signOf('DIV', []), 'div.');
});

test('what is kept for a tab is its own, and goes with it, its drawer\'s with it', () => {
  forgetAllReturned();
  remember('board:shop', 'board', 1);
  remember(drawerScope('board:shop'), 'card.title:shop-2', 'half changed');
  remember('table:shop:notes', 'rows', 2);
  assert.equal(recall('board:shop', 'board'), 1);
  assert.equal(recall('board:shop', 'rows'), undefined, 'by name');
  assert.equal(recall('table:shop:notes', 'board'), undefined, 'and by tab');
  assert.equal(recall(null, 'board'), undefined, 'outside a tab nothing is kept');
  remember(null, 'board', 3);
  assert.deepEqual(scopes().sort(), ['board:shop', drawerScope('board:shop'), 'table:shop:notes'].sort());
  forgetReturned('board:shop');
  assert.deepEqual(scopes(), ['table:shop:notes'], 'one scope a tab that is open, and none for one that is not');
  forgetAllReturned();
  assert.deepEqual(scopes(), [], 'signing out lets go of everything');
});

test('a drawer is kept for its tab where another tab comes to show, and not where its own is pressed again', () => {
  assert.equal(keepsDrawer('board:shop', 'home'), true);
  assert.equal(keepsDrawer('board:shop', ''), true, 'another workspace\'s set shown');
  assert.equal(keepsDrawer('board:shop', 'board:shop'), false, 'its own tab pressed again puts it away');
  assert.equal(keepsDrawer(null, 'home'), false, 'nobody\'s (its tab was closed under it)');
  assert.equal(keepsDrawer('', 'home'), false);
});

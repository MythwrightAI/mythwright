// Where a menu that a line of another menu opens is put: beside that
// line, on the side that has room for it, its first line level with the
// line that opened it, and on the screen. Where neither side has room (a
// phone's width) it lies over the lower part of the menu that opened it,
// under the line, so that the line and what is above it stay in view.
//
// Holds nothing of the page, so that its tests can read it.

export interface Box {
  left: number;
  top: number;
  right: number;
  bottom: number;
}

/** How far from the window's edges anything floating is kept. */
const EDGE = 4;
/** The room between the menu and the one beside it. */
const GAP = 2;
/** How far a menu's first line is below its own top: its border and its padding. */
const INSET = 5;

export interface Beside {
  left: number;
  top: number;
  /** As tall as it may be drawn: taller than that, it scrolls in itself. */
  most: number;
  /** Which way it was put: beside on the right or the left, or over the menu that opened it. */
  side: 'right' | 'left' | 'over';
}

/**
 * Where a menu `size` big is put that `line`, a line of the menu `menu`,
 * opened, in a window `window` big.
 */
export function besideAt(menu: Box, line: Box, size: { width: number; height: number }, window: { width: number; height: number }): Beside {
  const most = window.height - 2 * EDGE;
  const tall = Math.min(size.height, most);
  const downFrom = (top: number) => Math.max(EDGE, Math.min(top, window.height - EDGE - tall));
  if (menu.right + GAP + size.width <= window.width - EDGE) return { left: menu.right + GAP, top: downFrom(line.top - INSET), most, side: 'right' };
  if (menu.left - GAP - size.width >= EDGE) return { left: menu.left - GAP - size.width, top: downFrom(line.top - INSET), most, side: 'left' };
  // Over the menu, under its line and a little in from its left, so that what it lies over is seen to be beneath it.
  const left = Math.max(EDGE, Math.min(menu.left + 16, window.width - EDGE - size.width));
  const below = line.bottom + GAP;
  const room = window.height - EDGE - below;
  // Where there is not room enough under the line to read it, it is moved up as far as it must be, over the line too.
  if (room >= Math.min(tall, 120)) return { left, top: below, most: room, side: 'over' };
  return { left, top: downFrom(below), most, side: 'over' };
}

export interface Point {
  x: number;
  y: number;
}

/** How far past the corners of the menu beside it the way to it is taken to reach. */
const SLACK = 8;

/**
 * Whether the pointer, moved from `from` to `to` over the menu a line of
 * it opened another beside, is on its way to that one (`box`, put on
 * `side`): inside the narrowing way from where it was to the near edge
 * of the menu beside. Crossing the other lines on that way leaves what
 * was opened where it is, however slowly the pointer goes; resting on
 * another line, or moving off the way, puts it away a moment later. One
 * put over the menu it came from is reached directly, and has no way.
 */
export function towards(from: Point, to: Point, box: Box, side: Beside['side']): boolean {
  if (side === 'over') return false;
  const edge = side === 'right' ? box.left : box.right;
  const ahead = side === 'right' ? to.x >= from.x : to.x <= from.x;
  if (!ahead) return false;
  const a = from;
  const b = { x: edge, y: box.top - SLACK };
  const c = { x: edge, y: box.bottom + SLACK };
  const cross = (p: Point, q: Point, r: Point) => (q.x - p.x) * (r.y - p.y) - (q.y - p.y) * (r.x - p.x);
  const [one, two, three] = [cross(a, b, to), cross(b, c, to), cross(c, a, to)];
  const someBelow = one < 0 || two < 0 || three < 0;
  const someAbove = one > 0 || two > 0 || three > 0;
  return !(someBelow && someAbove);
}

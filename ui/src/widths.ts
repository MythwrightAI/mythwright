// How wide each column of a table is, where its view says.
//
// A view says how wide some of its columns are (`widths`): for each, whole
// pixels at the usual size of text, which the page draws as that many times
// the size its reader's look says (`--size`), so that a column holds as
// much of what is in it for everybody whatever size their text is. One it
// says nothing of is as wide as what is in it, as columns always were. The
// width is the view's, for everybody, like the order of its columns: it is
// dragged by the edge at the right of a column's head by whoever may save a
// view, and saved when the edge is let go, through the one door every
// change of a view goes through (`Change`, made in its turn: `inturn.ts`).
// Nothing here knows the page.

import type { Change, View } from './table.ts';

/** The narrowest a column may be, as the daemon holds it (`kernel::columns::NARROWEST`): its mark and a few letters. */
export const NARROWEST = 60;

/** The widest a column may be, as the daemon holds it (`kernel::columns::WIDEST`). */
export const WIDEST = 2000;

/** How much an arrow on a column's edge makes it wider or narrower, in pixels at the usual size of text. */
export const STEP = 16;

/** A width as it is kept: whole pixels, and from the narrowest to the widest a column may be. */
export function fitted(width: number): number {
  if (!Number.isFinite(width)) return NARROWEST;
  return Math.min(WIDEST, Math.max(NARROWEST, Math.round(width)));
}

/**
 * What a column comes to that was `from` pixels wide as it is drawn when
 * its edge was taken hold of, and was dragged `moved` pixels to the right
 * (less than nothing to the left), on a page whose text is `size` times
 * the usual: the width that is kept, at the usual size.
 */
export function dragged(from: number, moved: number, size: number): number {
  return fitted((from + moved) / (size > 0 ? size : 1));
}

/** How wide a column is drawn while its edge is held and dragged to `asked` pixels, on a page whose text is `size` times the usual: never past what it may be kept at. */
export function drawnWhileHeld(asked: number, size: number): number {
  const by = size > 0 ? size : 1;
  return Math.min(WIDEST * by, Math.max(NARROWEST * by, asked));
}

/** How wide the view says the column `name` is, or nothing where it says nothing of it or what it says is no width. */
export function widthOf(view: View | null | undefined, name: string): number | null {
  const said = view?.widths?.[name];
  return typeof said === 'number' && Number.isFinite(said) && said > 0 ? said : null;
}

/** What the view's widths are once the column `name` is `width` wide, or as wide as what is in it where that is nothing: nothing where none is left. */
export function widthsWith(widths: Record<string, number> | null | undefined, name: string, width: number | null): Record<string, number> | null {
  const next = Object.fromEntries(Object.entries(widths ?? {}).filter(([one]) => one !== name));
  if (width !== null) next[name] = fitted(width);
  return Object.keys(next).length > 0 ? next : null;
}

/**
 * The change to a view that makes the column `name` this wide, or as wide
 * as what is in it again where `width` is nothing: worked out from the
 * view as it is when the change is made, so that two columns let go of in
 * the time one save takes both stand.
 */
export function widens(name: string, width: number | null): Change {
  return (view) => ({ name: view.name, kind: view.kind, widths: widthsWith(view.widths, name, width) });
}

/**
 * The change to a view that makes the column `name` a step wider (`by`
 * 1) or narrower (-1) than the view says it is when the change is made,
 * or than it was drawn (`natural`, at the usual size) where the view says
 * nothing of it. A step past the narrowest or the widest changes nothing.
 */
export function stepWidth(name: string, by: -1 | 1, natural: number): Change {
  return (view) => {
    const now = widthOf(view, name) ?? fitted(natural);
    const next = fitted(now + by * STEP);
    return next === now && widthOf(view, name) !== null ? { name: view.name, kind: view.kind } : widens(name, next)(view, [], []);
  };
}

/** What a cell of a column that is kept `width` wide says of itself in its style: the width it is drawn at, by the size of the reader's text. */
export function sizedStyle(width: number | null): string | null {
  return width === null ? null : `--cw:calc(${width}px * var(--size))`;
}

/**
 * The rule that has the `nth` column (from 1) of the table marked `held`
 * drawn `width` pixels wide while its edge is held: its head, every row's
 * cell and the figure at its foot, and nothing that spans several columns.
 */
export function heldRule(held: string, nth: number, width: number): string {
  const at = `.grid[data-width-held="${held}"]`;
  const cells = [`thead tr:last-child > :nth-child(${nth})`, `tbody tr[data-row] > :nth-child(${nth})`, `tfoot tr.grid-figures > :nth-child(${nth})`];
  const px = `${Math.round(width * 100) / 100}px`;
  return `${cells.map((one) => `${at} ${one}`).join(', ')} { width: ${px}; min-width: ${px}; max-width: ${px}; }`;
}

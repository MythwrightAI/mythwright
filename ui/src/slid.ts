// A card that moves to another column is seen to slide there.
//
// The cards' boxes are noted before a drawing that changes where they
// are, and read again after it; each card that is somewhere else is then
// moved by `transform` alone from where it was to where it is, in the
// drawer's time and easing (`--slide`), by the style sheet
// (`views/cardslide.ts`). The cards it leaves and those it lands among
// close up and open the same way, being cards that are somewhere else.
// What moved, and by how much, is a function of the two sets of boxes,
// with its cases (`slid.test.ts`): nothing that is not on the page both
// times (a column folded, a view held to fewer), nothing out of sight
// either time (a column scrolled far), and nothing under the pointer,
// which goes where it goes at once.

/** A card's box on the page, as the browser says it. */
export interface Box {
  left: number;
  top: number;
  width: number;
  height: number;
}

/** A card that moved: what it is, and how far from where it is now it was. */
export interface Moved {
  id: string;
  x: number;
  y: number;
}

const meets = (one: Box, other: Box): boolean =>
  one.left < other.left + other.width && other.left < one.left + one.width && one.top < other.top + other.height && other.top < one.top + one.height;

const holds = (one: Box, at: { x: number; y: number }): boolean => at.x >= one.left && at.x < one.left + one.width && at.y >= one.top && at.y < one.top + one.height;

/**
 * The cards that moved between `before` and `after`, each by how far it
 * was from where it is now: those in both, a pixel or more away, each box
 * meeting `view` (what is in sight of the board) both times where one is
 * said, and neither holding the `pointer` where one is said.
 */
export function moved(before: ReadonlyMap<string, Box>, after: ReadonlyMap<string, Box>, view?: Box | null, pointer?: { x: number; y: number } | null): Moved[] {
  const all: Moved[] = [];
  for (const [id, now] of after) {
    const was = before.get(id);
    if (!was) continue;
    const x = was.left - now.left;
    const y = was.top - now.top;
    if (Math.abs(x) < 1 && Math.abs(y) < 1) continue;
    if (view && !(meets(was, view) && meets(now, view))) continue;
    if (pointer && (holds(was, pointer) || holds(now, pointer))) continue;
    all.push({ id, x: Math.round(x), y: Math.round(y) });
  }
  return all;
}

/** What the cards of a set of columns are, in order, as one word: a drawing that changes it may have moved some. */
export function arrangementOf(columns: readonly (readonly [string, readonly string[]])[]): string {
  return columns.map(([column, ids]) => `${column}:${ids.join(',')}`).join('\n');
}

/** Whether a card that is `now` within its column's list `within`, and was `was`, is seen on its way only out of that list: it is flown over the page. */
export function leaves(was: Box, within: Box): boolean {
  return !(was.left >= within.left - 1 && was.left + was.width <= within.left + within.width + 1 && was.top >= within.top - 1 && was.top + was.height <= within.top + within.height + 1);
}

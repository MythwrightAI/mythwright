// Whether a card was left for a person, and why: said in the card's own
// drawer, where whoever wonders why a card sits in a column looks.
//
// A round that cannot go on leaves its card where it is and writes why on
// the card's row, in the field the fleet keeps of how a card's rounds went
// (`again`: a list whose first word says what the last of them came to).
// The field is kept for whatever looks after the cards and a form has no
// place for it (`keptForItself`), so the reason was said in the project's
// channel and nowhere on the card: opened, a card the built-in assistant
// could not work read "In doing", assigned to nobody. This reads the row
// and gives the words to say; it fills nothing in.
//
// It is left only for as long as nobody has moved it since: how a card
// came to be where it is, is on its row too (`moved`: where from, the kind
// of actor, what was said, how many moves), and a round's own move is a
// blueprint's. A card somebody has moved since has been seen to.

/** A card that was left for a person. */
export interface Left {
  /** Why, in the words the round kept, as far in as they go; empty where it kept none. */
  why: string;
  /** The column its stage takes cards from, where the row says: put back there, it is taken up again. */
  from: string | null;
}

const words = (kept: unknown): string[] | null => (Array.isArray(kept) && kept.every((one) => typeof one === 'string') ? (kept as string[]) : null);

/** The reason alone: a failure names every step it came up through, and a person is told the innermost. */
export function innermost(why: string): string {
  let rest = why.trim();
  for (;;) {
    const cut = /^step `[^`]*` failed: /.exec(rest);
    if (!cut) return rest;
    rest = rest.slice(cut[0].length);
  }
}

/**
 * What a card's row says of having been left: `again` as the row keeps
 * it, `moved` as the board wrote it, and `taken`, the column its stage
 * took it from. Nothing for a card in hand, given again, taken up again,
 * asked about, never worked, or left and since moved by anybody.
 */
export function leftFor(again: unknown, moved?: unknown, taken?: unknown): Left | null {
  const kept = words(again);
  if (!kept || kept[0] !== 'left') return null;
  // Four words where the board keeps how the card was moved; a board that keeps something else under the name says nothing of it.
  const move = words(moved);
  if (move && move.length === 4 && move[1] !== 'blueprint') return null;
  return { why: innermost(kept[1] ?? ''), from: typeof taken === 'string' && taken.trim() !== '' ? taken.trim() : null };
}

// A word on a colour of its own: an option of a choice, one of the things a
// list holds. Wherever such a word is drawn (a table's cell, a card, a
// band's head, what a view is held to) it is this, so that it is the same
// colour everywhere.

import { colour } from '../pills.ts';
import type { Field } from '../table.ts';

/** A word as a tag, in the colour its name comes to; `small` where it sits in a line of other marks. */
export function Tag({ of, small }: { of: unknown; small?: boolean }) {
  return <span class={`tag tag-${colour(String(of))}${small ? ' tag-small' : ''}`}>{String(of)}</span>;
}

/** Whether what a field holds is words that are drawn as tags: not the ids of other records, nor people. */
export const worded = (field: Field | undefined): boolean => field?.kind === 'list' || field?.kind === 'choice';

// What a table's page adds to the menu of its gear (`views/pagehead.tsx`),
// as words: which lines there are and what each says, before anything is
// drawn or done. A page's head has one thing to press for what else can be
// done with the page, so the tables that sit beside a table are lines of
// its gear, under a heading, and so is what can be done to a board.
//
// This holds nothing of the page, so that its tests can read it: a line
// that is pressed says what pressing it does (`does`), and whoever draws
// the menu does that.

import { counted } from './sits.ts';
import type { Sat } from './sits.ts';

/**
 * A line of a menu that is pressed, as words: what it is called, what
 * pressing it does, and its word on that, which is a few faint words at
 * the line's end (`hint`) where they fit beside its name, and a sentence
 * under the line, read whole, where they do not (`note`).
 */
export interface Pressed<Does> {
  label: string;
  hint?: string;
  note?: string;
  does: Does;
}

/** A line of a menu as words: one that only says something, or one that is pressed. */
export type Worded<Does> = { says: string } | Pressed<Does>;

/** A table's name as a heading says it. */
export const headed = (name: string): string => name.charAt(0).toUpperCase() + name.slice(1).replace(/[-_]/g, ' ');

/**
 * The lines for the tables that sit beside a table: a heading that only
 * says, and under it each of them by what it is called, with how many
 * rows it has, which opens that table. Nothing where none sits beside it:
 * no heading over nothing.
 */
export function besideLines(sat: Sat[]): Worded<string>[] {
  if (sat.length === 0) return [];
  return [{ says: 'Its tables' }, ...sat.map(({ table, rows }) => ({ label: table.title_chosen || table.title || headed(table.name), hint: counted(table, rows) || 'being read', does: table.name }))];
}

/** What can be done to a board from its gear. */
export type Tool = 'card' | 'column' | 'arrange' | 'layout';

/**
 * The lines for what can be done to a board with these columns: a card
 * added to its first, a column added after its last, the columns put in
 * another order or one taken away, and how the view being looked at is
 * laid out, which says how it is laid out now (`laid`) where that is
 * known. Each says in a few words what it does, at the end of its line;
 * a board with no column says of none. What arranging the columns is
 * does not fit beside so long a name at the menu's width, and is said
 * under it.
 */
export function boardLines(columns: string[], laid?: string): Pressed<Tool>[] {
  const [first, last] = [columns[0], columns[columns.length - 1]];
  return [
    { label: 'Add a card', hint: first ? `to ${first}` : undefined, does: 'card' },
    { label: 'Add a column', hint: last ? `after ${last}` : undefined, does: 'column' },
    { label: 'Arrange the columns', note: 'Change their order, or take one away.', does: 'arrange' },
    { label: 'Layout', hint: laid, does: 'layout' },
  ];
}

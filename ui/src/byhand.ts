// A table whose rows are not written by hand.
//
// A table may say its rows are written only by the commands and
// blueprints that keep it (`by_hand: false`, where its blueprint declares
// it): a row for each heavy run of a script, each simulation scenario,
// each deploy. Somebody who looks at it reads those rows; adding one here,
// or changing what one says, would be making up a run that never ran. So
// the page offers nothing that writes a row or what the rows are made of,
// and goes on offering everything that is how somebody looks at a table
// (sorting, holding the view to some rows, gathering them, a figure at a
// column's foot, freezing, widths, leaving a column out, saved views),
// opening a row to read it, and taking one away to the Trash, which is
// tidying and not writing. Like `beside` (`sits.ts`) it is how the table
// is shown and nothing else: the daemon refuses nothing for it, and every
// door that writes a record (a route, `mw record`, an action) is as it
// was. This holds nothing of the page, so that its tests can read it.

/** As much of a table as this goes by. */
export interface Written {
  /** `false` where its rows are written only by what keeps it; nothing, where they are written by hand too. */
  by_hand?: boolean | null;
}

/** What of a table the page offers to be done by hand, whoever may do it: what they hold is asked beside this. */
export interface Offered {
  /** A new row: the head's button, the table's last line, and the line a layout writes one in. */
  adds: boolean;
  /** A cell changed where it is, a field of a row's form, a row carried to another column or another day. */
  changes: boolean;
  /** The columns themselves: added, retyped, what a choice offers, copied, taken away and called something else. */
  columns: boolean;
  /** A CSV file read in, which adds and changes rows. */
  readsIn: boolean;
  /** A row taken away to the Trash, from its form. */
  removes: boolean;
}

/** Whether people write the rows of `table` by hand: unless its blueprint says they do not. A daemon from before says nothing, and they do. */
export function byHand(table: Written | null | undefined): boolean {
  return table?.by_hand !== false;
}

/** What of `table` is offered to be done by hand. */
export function offered(table: Written | null | undefined): Offered {
  const hand = byHand(table);
  return { adds: hand, changes: hand, columns: hand, readsIn: hand, removes: true };
}

/** The few faint words at the head of such a table, beside how many rows it has. */
export const KEPT_WORDS = 'written by what keeps it';

/** What is said of such a table where somebody points at those words, and under a row's form: why nothing here writes. */
export const KEPT_SAID = 'Its rows are written by the commands and blueprints that keep it, and are read here: nothing here adds a row, changes one or changes its columns. Its blueprint says so.';

/** The words at a table's head that say so, and what they say where they are pointed at; nothing, for a table written by hand. */
export function keptSaid(table: Written | null | undefined): { words: string; more: string } | null {
  return byHand(table) ? null : { words: KEPT_WORDS, more: KEPT_SAID };
}

/** Why a field of a row's form is closed, for every field of such a table; nothing, for a table written by hand. */
export function closedWhy(table: Written | null | undefined): string | null {
  return byHand(table) ? null : KEPT_SAID;
}

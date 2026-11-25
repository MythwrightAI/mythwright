// What is set up for a column of a board, or of any table laid out in
// columns: the rows of the tables that sit beside it that are
// about that column, gathered under a heading for each table, each with a
// few faint words on what it is, and a line to add one of each such table
// for whoever may. The column's head says only its name and its count and
// one small mark beside the name that opens this (`views/columnset.tsx`);
// a column with nothing about it has no mark, and a row is added from it
// only to a table it lists.
//
// Nothing here knows what any such table is of. A heading is what the
// table is called for people (`tableCalled`); a row's few words are what
// one of its rows is (`oneOf`) and the first two of its other cells that
// say something; a row that holds a card until it is pressed says so, as
// the board's buttons read such a row (`cardactions.ts`, by what its
// table says and never by its name), and is what the dot on the mark says
// is there.
//
// Where a table's rows say what the factory's bot does with the cards of
// a column (a fleet's stages, and what is pressed on a card), a row's
// words are that rule in a sentence: who does what with a card
// there, how many at once, and where a card goes on to (`stageSays`);
// what holds a card until it is pressed, and whether once for good or
// each time the card comes (`actionSays`). Such a table is told by the
// fields it says, never by its name, and what a row leaves empty is said
// as the fleet fills it in (`fill` of the stages in `std/arrangement`),
// and nothing a row does not say. This holds nothing of the page, so that
// its tests can read it.

import { tableCalled } from './called.ts';
import { actionsOf, pressed as isPressed } from './cardactions.ts';
import type { Action } from './cardactions.ts';
import { covers } from './holds.ts';
import { aboutThis, oneOf } from './sits.ts';
import type { Sat, Sitting } from './sits.ts';
import { slugOf } from './tablemade.ts';
import { said } from './table.ts';
import type { Row } from './table.ts';

/** A table as much as this goes by: what it is called and what writing it takes. */
export interface SetTable extends Sitting {
  title?: string | null;
  title_chosen?: string | null;
  by_hand?: boolean | null;
  takes?: string | null;
}

/** A row about the column, as its line says it. */
export interface SetRow {
  table: string;
  row: Row;
  called: string;
  /** The few faint words beside it. */
  says: string;
  /** It holds each card in the column until it is pressed. */
  holds: boolean;
}

/** What is set up for one column. */
export interface SetUp {
  /** Each table with a row about the column, under its heading, in the order the tables are. */
  groups: { heading: string; rows: SetRow[] }[];
  /** Something about the column holds its cards until it is pressed, which a dot on the mark says. */
  holds: boolean;
  /** The tables the menu lists a row of which may be added for the column, by whoever asks, with what one of their rows is called. */
  adds: { table: string; one: string }[];
}

/** A word with its article: `an action`, `a stage`. */
export const a = (word: string): string => `${/^[aeiou]/i.test(word) ? 'an' : 'a'} ${word}`;

/** The few faint words beside a row: what one of its table's rows is, and the first two of its other cells that say something, each by its field. */
export function rowSays(table: Sitting, row: Row, holds: boolean, pressed: boolean): string {
  const one = oneOf(table.name);
  const what = one === 'row' ? '' : a(one);
  if (pressed) return [what, holds ? 'holds each card here until it is pressed' : 'is pressed on each card here'].filter(Boolean).join(' that ');
  const skip = new Set(['title', 'name', table.about ?? '']);
  const cells = table.fields
    .filter((field) => !skip.has(field.name) && !field.hidden && field.kind !== 'list')
    .map((field) => [field, said(row.data[field.name]).trim()] as const)
    .filter(([, value]) => value !== '' && value.length <= 24)
    .slice(0, 2)
    .map(([field, value]) => (field.kind === 'flag' ? field.name : `${field.name} ${value}`));
  return [what, ...cells].filter(Boolean).join(' · ');
}

/** Whether a table's rows say what is done with a card in a column: it says the column a row is about, what is done there and where a card goes on to. */
export function staged(table: Sitting): boolean {
  const says = (field: string | null | undefined) => table.fields.some((each) => each.name === field);
  return says(table.about) && says('does') && says('pass');
}

/** The three things a fleet's agents do with a card, each as a sentence begins; any other word names the blueprint begun for each card. */
const DONE: Record<string, string> = { work: 'Worked', review: 'Reviewed', plan: 'Planned' };

/** A column as the rule names it: as the board calls it, and said to be none of the board's where `columns` is known and has it not. */
const columnSaid = (name: string, columns: string[] | null): string => (columns && !columns.includes(name) ? `${name} (no such column)` : name);

/**
 * What the factory's bot does with the cards of a column, as a stage's
 * row says it: `Worked by the worker role, at most 2 at a time → review
 * on pass`, and where cards are reviewed `· back to doing on fail`. What
 * the row leaves empty is what the fleet fills it in with: what is done is
 * a review, the role is what the row is called written as a name is, as
 * many cards at once as the role's agents can take, a worked card goes on
 * to review and a sent back one to doing, and a card passed where nothing
 * is said stays where it is. `columns` is the board's, where it is known.
 */
export function stageSays(row: Row, columns: string[] | null = null): string {
  const cell = (name: string) => said(row.data[name]).trim();
  const does = cell('does') || 'review';
  const role = cell('role') || slugOf(cell('title'));
  const at = Number(row.data.at_once);
  const lead = DONE[does] ? `${DONE[does]} by the ${role} role` : `${does} is begun for each card here`;
  const many = Number.isInteger(at) && at > 0 ? `at most ${at} at a time` : 'as many at a time as its agents can take';
  const pass = cell('pass') || (does === 'work' ? 'review' : '');
  const on = pass ? ` → ${columnSaid(pass, columns)} on pass` : ' · stays here on pass';
  const back = does === 'review' ? ` · back to ${columnSaid(cell('fail') || 'doing', columns)} on fail` : '';
  return `${lead}, ${many}${on}${back}`;
}

/** What something pressed on a card in a column does there, as its row says it: whether it holds each card until it is pressed, and whether a press is for good or for each time a card comes. */
export function actionSays(action: Action): string {
  const what = action.holds ? `Holds each card here until ${action.title} is pressed` : `${action.title} may be pressed on each card here`;
  return `${what} · ${action.again ? 'each time it comes' : 'once for good'}`;
}

/**
 * What is set up for `column` of the table the tables of `sat` sit beside:
 * nothing (no group) where no row is about it. `held` is what whoever
 * reads this holds, which says which tables a row may be added to: one
 * whose rows are written by hand, by whoever may write records and holds
 * what writing the table takes. `columns` is the board's, where it is
 * known, for a stage's rule to say a column it names is none of them.
 */
export function setUpOf(sat: Sat<SetTable>[], column: string, held: string[], columns: string[] | null = null): SetUp {
  const groups: SetUp['groups'] = [];
  let holds = false;
  for (const { table, rows } of sat) {
    if (!table.about || column === '') continue;
    const pressed = isPressed(table);
    const actions = pressed ? actionsOf(table, rows ?? []) : [];
    const about = aboutThis([{ table, rows }], column);
    if (about.length === 0) continue;
    const rule = staged(table);
    const lines = about.map(({ row, called }): SetRow => {
      const action = actions.find((one) => one.id === row.id);
      const holding = action?.holds ?? false;
      holds ||= holding;
      const says = rule ? stageSays(row, columns) : action ? actionSays(action) : rowSays(table, row, holding, pressed);
      return { table: table.name, row, called, says, holds: holding };
    });
    groups.push({ heading: tableCalled(table), rows: lines });
  }
  // Each such table is one the menu lists: a table that is about something other than columns (a card) has no row
  // about one, and is offered nowhere here.
  const listed = new Set(groups.flatMap((group) => group.rows.map((one) => one.table)));
  const adds = sat
    .map(({ table }) => table)
    .filter((table) => listed.has(table.name) && table.by_hand !== false && covers(held, 'records:write') && (!table.takes || covers(held, table.takes)))
    .map((table) => ({ table: table.name, one: oneOf(table.name) }));
  return { groups, holds, adds };
}

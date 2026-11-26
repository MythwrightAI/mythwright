// Where a card goes when somebody sends it back.
//
// That is not the client's to say. What is done with the cards of a
// column is a row of a table that sits beside the board and is about its
// columns (a fleet's `stages`), and the row says where a card that did
// not pass goes (`fail`). So it is read from there, as any table is read:
// the row that is about the column the card is in says where to. This
// holds nothing of the page, so that its tests can read it.

import { aboutOf, besideOf } from './sits.ts';
import type { Sitting } from './sits.ts';
import type { Row } from './table.ts';

/** The table a fleet's stages are rows of, and the field in which one says its column, as they were before a table could say what it sits beside. */
const STAGES = 'stages';
const COLUMN = 'column';
/** The field in which a row says where a card that did not pass goes. */
const FAIL = 'fail';

/**
 * The tables of a project whose rows say where a card of a column is sent
 * back to, of all its tables: those that sit beside its board, say which
 * of its columns a row is about, and have somewhere to say where a card
 * goes back to. No table is known by its name for that. Only where none
 * says so, which is a fleet a program from before declared, is it the
 * table called `stages`, by its `column`, as it always was.
 */
export function sendingBack(all: Sitting[]): Sitting[] {
  const says = (one: Sitting, field: string | null | undefined) => one.fields.some((each) => each.name === field);
  const beside = besideOf('board', all).filter((one) => says(one, one.about) && says(one, FAIL));
  if (beside.length > 0) return beside;
  return all.filter((one) => one.name === STAGES).map((one) => ({ ...one, about: COLUMN }));
}

/** A row of such a table as `backTo` reads one: with the column it is about said in `column`, whichever field its table says that in. */
export function asStage(table: Sitting, row: Row): Row {
  return { ...row, data: { ...row.data, [COLUMN]: aboutOf(table, row) } };
}

/** What a cell says: what somebody wrote there, and nothing for one emptied or never filled. */
const said = (value: unknown): string => (typeof value === 'string' ? value.trim() : '');

/** The column a stage has: the one its row says, or the one it is called after where it says none. */
const columnOf = (stage: Row): string => said(stage.data.column) || said(stage.data.title);

/**
 * Where a card in the column `status` goes when it is sent back, on a
 * board with these columns; nothing where there is nowhere to send it,
 * and sending back is not offered.
 *
 * `stages` is the rows of the project's `stages`, in their order, or
 * nothing where the project has no such table or it may not be read. The
 * first stage that has the card's column says where a card that did not
 * pass goes, and a column it names that the board has not got is nowhere.
 * Where no stage has the column, or the one that has it names none, it
 * is as it was before a row could say: a card in `review` goes back to
 * `doing`, on a board that has one.
 */
export function backTo(status: string, columns: string[], stages: Row[] | null): string | null {
  const stage = (stages ?? []).find((one) => columnOf(one) === status);
  const fail = stage ? said(stage.data.fail) : '';
  const to = fail || (status === 'review' ? 'doing' : '');
  return to !== '' && to !== status && columns.includes(to) ? to : null;
}

/** A count a cell or a card says, and nothing for anything that is not one. */
const counted = (value: unknown): number | null => {
  const count = typeof value === 'number' ? value : typeof value === 'string' && value.trim() !== '' ? Number(value) : NaN;
  return Number.isFinite(count) ? count : null;
};

/**
 * Whether a card in the column `status` has been sent back as often as the
 * stage that has the column allows (`rounds` of its row), beside what it
 * was given (`given`): sent back as it is, it would be left for a person
 * again as it came back, so it is offered one more round with it. Nothing
 * where no stage has the column or its row says no count.
 */
export function outOfRounds(status: string, stages: Row[] | null, rounds: unknown, given: unknown): boolean {
  const stage = (stages ?? []).find((one) => columnOf(one) === status);
  const most = stage ? counted(stage.data.rounds) : null;
  return most !== null && (counted(rounds) ?? 0) >= most + (counted(given) ?? 0);
}

// The figure at the foot of a table's column.
//
// A view may say one figure for a column (`figures`): how many of the
// rows it shows say something there and how many say nothing; for a
// number what they add up to, their mean, the least and the greatest; for
// a tick how many are ticked; and for anything else how many different
// things are said. It counts and never scores: no figure is better or
// worse than another, and none is marked.
//
// A row says nothing in a column where it holds nothing, an empty text
// or an empty list there (a cell somebody emptied is kept as empty text,
// and means what one never filled means). A figure that has nothing to
// go by is no figure: the mean of no numbers is not nought.
//
// A table's number is a plain number. What the factory measures is a
// length of time unless its name says otherwise; a column of a table was
// never a measurement, so nothing is read into its name and a figure is
// written as the cells above it are. This has no page in it, and the
// daemon holds the same table of which kind can say which figure
// (`kernel::columns`).

import { holds } from './ordered.ts';
import type { Field, Row } from './table.ts';
import { said } from './table.ts';

/** Every figure there is, in the order they are offered. */
export const FIGURES = ['filled', 'empty', 'sum', 'mean', 'least', 'greatest', 'ticked', 'different'] as const;
export type Figure = (typeof FIGURES)[number];

/** Whether a column of this kind can say this figure. */
export function fits(kind: string, figure: string): boolean {
  if (figure === 'filled' || figure === 'empty') return true;
  if (figure === 'sum' || figure === 'mean' || figure === 'least' || figure === 'greatest') return kind === 'number';
  if (figure === 'ticked') return kind === 'flag';
  return figure === 'different' && kind !== 'number' && kind !== 'flag';
}

/** The figures a column can say, in the order they are offered. */
export function offered(field: Field): Figure[] {
  return FIGURES.filter((figure) => fits(field.kind, figure));
}

/** What a figure is called where it is chosen, and before the figure at the foot. */
export function called(figure: string): string {
  return (
    { filled: 'Filled', empty: 'Empty', sum: 'Sum', mean: 'Mean', least: 'Least', greatest: 'Greatest', ticked: 'Ticked', different: 'Different' }[figure] ?? figure
  );
}

/** What a figure is, in a sentence, for whoever points at it. */
export function meaning(figure: string): string {
  return (
    {
      filled: 'How many of the rows shown say something here',
      empty: 'How many of the rows shown say nothing here',
      sum: 'What the numbers of the rows shown add up to',
      mean: 'The mean of the numbers of the rows shown',
      least: 'The least of the numbers of the rows shown',
      greatest: 'The greatest of the numbers of the rows shown',
      ticked: 'How many of the rows shown are ticked',
      different: 'How many different things the rows shown say here',
    }[figure] ?? ''
  );
}

/** Whether a row says something in a column. */
function says(row: Row, field: Field): boolean {
  return said(row.data[field.name]).trim() !== '';
}

/** The numbers the rows say in a column: what is a number, or is written as one. */
function numbers(rows: Row[], field: Field): number[] {
  return rows.filter((row) => says(row, field)).flatMap((row) => {
    const value = Number(row.data[field.name]);
    return Number.isFinite(value) ? [value] : [];
  });
}

/**
 * A figure of the rows in a column; `null` where the column cannot say
 * it, or there is nothing to go by: the mean, the least and the greatest
 * of no numbers are no figure. What no numbers add up to is nought.
 */
export function figure(rows: Row[], field: Field, which: string): number | null {
  if (!fits(field.kind, which)) return null;
  if (which === 'filled') return rows.filter((row) => says(row, field)).length;
  if (which === 'empty') return rows.filter((row) => !says(row, field)).length;
  if (which === 'ticked') return rows.filter((row) => row.data[field.name] === true).length;
  if (which === 'different') return new Set(rows.flatMap((row) => holds(row, field))).size;
  const all = numbers(rows, field);
  if (which === 'sum') return all.reduce((sum, one) => sum + one, 0);
  if (all.length === 0) return null;
  if (which === 'mean') return all.reduce((sum, one) => sum + one, 0) / all.length;
  return which === 'least' ? Math.min(...all) : Math.max(...all);
}

/** A figure as it is written: a plain number, to two places where it has more; a dash where there is none. */
export function written(value: number | null): string {
  return value === null ? '–' : String(Math.round(value * 100) / 100);
}

/** The figures a view says, less what names a column that is gone or a figure its column cannot say: what is saved with the view. */
export function kept(figures: Record<string, string> | null | undefined, fields: Field[]): Record<string, string> | null {
  const still = Object.entries(figures ?? {}).filter(([name, which]) => fields.some((field) => field.name === name && fits(field.kind, which)));
  return still.length > 0 ? Object.fromEntries(still) : null;
}

/** The figures a view says once a column is told one, or told none. */
export function figuring(figures: Record<string, string> | null | undefined, name: string, which: string | null): Record<string, string> | null {
  const next = { ...figures };
  if (which === null) delete next[name];
  else next[name] = which;
  return Object.keys(next).length > 0 ? next : null;
}

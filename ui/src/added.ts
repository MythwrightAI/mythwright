// A table's columns as the people who look at it change them: the part
// of that with no page in it.
//
// What a table's columns are is said where it is declared, and whoever
// looks at the table may add a column of their own, call any column
// something else, and put what a choice offers in another order. Each is
// the table's, kept by the daemon for everybody (`studio::added`); the
// daemon is what holds every rule, and this is what the page offers and
// says: which lines a column's menu has for it, what a column somebody
// adds may hold, where one that is inserted stands, what filling a
// column takes of whoever fills it, and whose a column is.
//
// A column's name is what records, files, views and blueprints know it
// by, and never changes: renaming a column is choosing its label
// (`heading` in `table.ts`).

import { banded } from './groups.ts';
import { covers } from './holds.ts';
import { mayChoose } from './icons.ts';
import { columns, heading } from './table.ts';
import type { Field, Group, Guarded, Row } from './table.ts';

/** The scope adding, retyping and taking away a column takes, beside what saving a view of the table does. */
export const SCOPE = 'columns';

/** A kind of thing a column can hold, as it is offered. */
export interface Kind {
  kind: string;
  name: string;
  says: string;
}

/** What a column somebody adds can hold, in the order they are offered: the kinds with no authority in them. */
export const KINDS: Kind[] = [
  { kind: 'text', name: 'Text', says: 'Words.' },
  { kind: 'number', name: 'Number', says: 'A number.' },
  { kind: 'choice', name: 'Select', says: 'One of a few things you name.' },
  { kind: 'list', name: 'Multi-select', says: 'Several words, each a tag.' },
  { kind: 'date', name: 'Date', says: 'A day.' },
  { kind: 'flag', name: 'Checkbox', says: 'Ticked or not.' },
  { kind: 'record', name: 'Relation', says: 'A row of another table.' },
  { kind: 'records', name: 'Relations', says: 'Several rows of another table.' },
];

/** What a kind of field is called, one a person may not add among them. */
export function kindCalled(kind: string): string {
  return KINDS.find((one) => one.kind === kind)?.name ?? (kind === 'actor' ? 'Person' : kind);
}

/** What the page knows of a table when it asks what somebody may do with its columns. */
export interface Shaped extends Guarded {
  name: string;
  store?: string;
}

/**
 * Whether somebody who holds `held` may add a column to a table, retype
 * one and take one away: the scope of its own, with what saving a view
 * of the table takes; and never of a library or of mockups, whose
 * records are people's files. The daemon asks the same and is what
 * holds it.
 */
export function mayShape(held: string[], table: Shaped): boolean {
  return covers(held, SCOPE) && mayChoose(held, table) && table.store !== 'documents' && table.store !== 'mockups';
}

/** A line of a column's menu that changes what the table's columns are. */
export type Shape = 'type' | 'options' | 'insert-left' | 'insert-right' | 'duplicate' | 'delete';

/**
 * The lines of a column's menu that change the table's columns, in the
 * order they stand. What kind of thing a column holds is changed, and a
 * column taken away, only where somebody added it and whoever looks
 * `shapes` (holds the scope): what is declared is read by what declared
 * it. What a choice offers is opened by whoever `may` save a view, since
 * the order of it is theirs to change, of a declared choice too. A
 * column is inserted on either side and copied by whoever shapes;
 * nothing stands before what a record is `known` by, and only a kind
 * somebody may add is copied.
 */
export function shapes(of: { may: boolean; shapes: boolean; added: boolean; kind: string; known: boolean }): Shape[] {
  const all: Shape[] = [];
  if (of.shapes && of.added) all.push('type');
  if (of.kind === 'choice' && of.may) all.push('options');
  if (of.shapes && !of.known) all.push('insert-left');
  if (of.shapes) all.push('insert-right');
  if (of.shapes && KINDS.some((one) => one.kind === of.kind)) all.push('duplicate');
  if (of.shapes && of.added) all.push('delete');
  return all;
}

/** A word a choice offers, as the daemon keeps it: lowercase letters and digits, with a dash where anything else was. */
export function option(written: string): string {
  return written
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '');
}

/** What a column is to be called, where it can be called that: one line, 64 letters at most. */
export function labelled(written: string): string | null {
  const label = written.trim();
  const plain = [...label].every((letter) => letter.charCodeAt(0) > 31 && letter.charCodeAt(0) !== 127);
  return label !== '' && [...label].length <= 64 && plain ? label : null;
}

/** Why a column cannot be called `label` in a table with these fields, in a sentence; nothing, where it can. */
export function refusedLabel(fields: Field[], written: string, but?: string): string | null {
  const label = labelled(written);
  if (label === null) return written.trim() === '' ? 'A column is called something.' : 'A column is called by a word or a few, on one line and 64 letters at most.';
  const other = fields.find((field) => field.name !== but && heading(field).toLowerCase() === label.toLowerCase());
  return other ? `There is a column called ${heading(other)} already.` : null;
}

/** Where a column that is added stands in a view, as the daemon is told it. */
export interface Place {
  view: string;
  /** How that view is laid out: a table with no view yet keeps the one it is shown by as this. */
  kind: string;
  /** The view's columns as they stand now, every one by name. */
  order: string[];
  /** The column it stands before; after the last, where none is said. */
  before: string | null;
}

/**
 * Where a column added beside the column `beside` stands in the view
 * called `view`: before it (`side` -1), after it (1), or with nothing
 * said after the last column. Every column of the table is named as
 * they stand, band by band, so that the others stay where they are. A
 * column somebody adds is in no group: in a table that says its columns
 * in groups it stands among those in none, before the groups, in the
 * place this says among them.
 */
export function placing(table: { fields: Field[]; groups?: Group[] | null }, view: { name: string; kind: string; columns?: string[] | null }, beside: string | null, side: -1 | 1): Place {
  const order = banded(columns(table.fields), table.groups, view.columns).flatMap((band) => band.fields.map((field) => field.name));
  const at = beside === null ? -1 : order.indexOf(beside);
  const before = at < 0 ? null : side === -1 ? order[at]! : (order[at + 1] ?? null);
  return { view: view.name, kind: view.kind, order, before };
}

/** How many rows say something in a column: nothing, empty words and an empty list are each nothing said. */
export function saying(rows: Row[], name: string): number {
  return rows.filter((row) => {
    const value = row.data[name];
    return !(value === null || value === undefined || value === '' || (Array.isArray(value) && value.length === 0));
  }).length;
}

/** What is asked before a column is taken away: what goes, in a sentence. */
export function goes(field: Field, rows: Row[]): string {
  const said = saying(rows, field.name);
  const rest = said === 0 ? 'No row says anything in it.' : said === 1 ? 'What 1 row says in it goes with it.' : `What ${said} rows say in it goes with it.`;
  return `${heading(field)} is taken away for everybody who opens this table. ${rest} It cannot be put back.`;
}

/** The scopes a field's `takes` says: one is a word, and several, every one of which is asked, a list. */
function scopesOf(takes: string | string[] | null | undefined): string[] {
  if (!takes) return [];
  return typeof takes === 'string' ? [takes] : takes;
}

/** One thing filling a column takes, and whether whoever looks holds it. */
export interface Need {
  words: string;
  held: boolean;
}

/**
 * What filling a column takes of whoever fills it, from what its table
 * and the field say, and whether somebody who holds `held` may: what
 * changing any row of the table takes, what the table asks beside that,
 * what the field asks of any value and of one value, and what a list of
 * scopes gives. What only one value takes is said and does not stop the
 * column being filled with another.
 */
export function filling(field: Field, table: Shaped, held: string[]): { needs: Need[]; may: boolean; notes: string[] } {
  const needs: Need[] = [];
  const need = (scope: string, words: string) => needs.push({ words, held: covers(held, scope) });
  const base = table.name === 'board' ? 'board:write' : 'records:write';
  need(base, `${base}, to change a ${table.name === 'board' ? 'card' : 'row'} of this table`);
  if (table.reads) need(table.reads, `${table.reads}, which this table is kept private under`);
  if (table.takes) need(table.takes, `${table.takes}, which this table asks of whoever changes a row`);
  for (const scope of scopesOf(field.takes)) need(scope, `${scope}, which this column asks of any value`);
  const may = needs.every((one) => one.held);
  for (const [value, takes] of Object.entries(field.takes_of ?? {})) {
    for (const scope of scopesOf(takes)) need(scope, `${scope}, to give it the value ${value}`);
  }
  const notes: string[] = [];
  if (field.required) notes.push('Every row has to say something here.');
  if (field.takes_each) {
    const unsaid = field.unsaid ?? [];
    notes.push(`Each scope written here takes holding it${unsaid.length > 0 ? `; a row that leaves it empty gives ${unsaid.join(', ')}` : ''}.`);
  }
  if (field.kind === 'choice' && field.unsaid?.[0]) notes.push(`A row that leaves it empty stands for ${field.unsaid[0]}.`);
  if (table.steers) notes.push(`A row here steers the rows of ${table.steers}: it is written by whoever could write each of those.`);
  if (field.quiet) notes.push('A change of it alone is no line of a row’s history.');
  return { needs, may, notes };
}

/**
 * Whose a column is, in a sentence for the foot of its menu: somebody's
 * who looks at the table, or what declared the table's (`by`: what that
 * is called, where it is known), which reads it, so that what it holds
 * and whether it is there are said there. Either way what blueprints
 * and files call it is said: its name, whatever it is headed by.
 */
export function whose(field: Field, by?: string | null): { words: string; more: string } {
  const called = `Blueprints and files call it ${field.name}.`;
  if (field.added) return { words: 'A column somebody added to this table', more: called };
  const reads = by ? `This column is read by ${by}, which declares it` : 'This column is read by what declares the table';
  return { words: reads, more: `What it holds, and whether it is there, are changed where the table is declared. ${called}` };
}

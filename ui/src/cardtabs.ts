// The tabs of a card's drawer beside its form: a tab for each table that
// sits beside the card's board and whose rows are each about a card (its
// `about` names a record of the board), drawn with the rows about this
// card and, under each, the rows of other tables that name it. A card's
// A/B tests are such a table (`contests`, which `std/ab` declares, with
// what each contender did as rows of `entries` naming its test), and so
// is anything else a blueprint declares the same way: nothing here knows
// a test. This holds nothing of the page, so that its tests can read it.

import { sitsBeside } from './sits.ts';
import type { Sitting } from './sits.ts';
import { idsOf, knownBy, said, titled } from './table.ts';
import type { Field, Row } from './table.ts';

/** As much of a table as this goes by. */
export interface Tabbed extends Sitting {
  views?: { name: string; saved?: boolean }[];
}

/** The tables of `all` whose rows are each about one card of the board `board`, by name. */
export function aboutCards<T extends Tabbed>(board: string, all: T[]): T[] {
  return all
    .filter((one) => sitsBeside(one, all) === board)
    .filter((one) => one.fields.some((field) => field.name === one.about && field.kind === 'record' && field.of[0] === board))
    .sort((a, b) => a.name.localeCompare(b.name));
}

/**
 * What a table's tab is called: the name its blueprint gave its one way of
 * being laid out, where that says something its name does not (`A/B
 * testing`); else its name.
 */
export function tabOf(table: Tabbed): string {
  const declared = (table.views ?? []).filter((view) => !view.saved);
  const [one] = declared;
  if (declared.length === 1 && one && one.name !== table.name) return one.name;
  return titled(table.name.replaceAll('_', ' '));
}

/** The rows of `table` about the card `card`, in the order they were made. */
export function rowsAbout(table: Sitting, rows: Row[], card: string): Row[] {
  const about = table.about;
  if (!about) return [];
  return rows
    .filter((row) => idsOf(row.data[about]).includes(card))
    .sort((a, b) => a.position - b.position || a.number - b.number || a.id.localeCompare(b.id));
}

/** One thing a row says: under what, and what. */
export interface Cell {
  name: string;
  /** What it says, as words; for a flag that is so, nothing but its name. */
  says: string;
  /** A list, a line each. */
  lines?: string[];
  /** A flag that is so. */
  flag?: boolean;
  /** Long words, shown folded. */
  long?: boolean;
}

/** An address a browser opens as a page, where words are one, drawn as a link that opens it; nothing else is. */
export const linkOf = (words: string): string | undefined => (/^https?:\/\/\S+$/.test(words.trim()) ? words.trim() : undefined);

/** What a field is called where it is shown: what somebody called it, else its name in words. */
export function calledOf(field: Field): string {
  return field.label_chosen?.trim() || field.name.replaceAll('_', ' ');
}

/** Words longer than this are shown folded. */
export const LONG = 120;

/**
 * What a row says, a cell for each field that says something, in the
 * order its table declares them: leaving out what it is known by (said
 * as its name), what names `leave` (the row it is shown under), what is
 * hidden, and a flag that is not so.
 */
export function cellsOf(row: Row, fields: Field[], leave: string[] = []): Cell[] {
  const known = knownBy(row, fields);
  return fields.flatMap((field): Cell[] => {
    if (field.hidden || leave.includes(field.name)) return [];
    const value = row.data[field.name];
    const name = calledOf(field);
    if (field.kind === 'flag') return value === true ? [{ name, says: name, flag: true }] : [];
    if (Array.isArray(value)) {
      const lines = value.map(said).filter((line) => line.trim() !== '');
      return lines.length === 0 ? [] : [{ name, says: lines.join(', '), lines }];
    }
    const says = said(value).trim();
    if (says === '' || says === known) return [];
    return [{ name, says, long: says.length > LONG || says.includes('\n') }];
  });
}

/** The fields of a table that say what a row of another is, by naming it: `contest` of `entries`. */
export function namingFields(fields: Field[], of: string): string[] {
  return fields.filter((field) => (field.kind === 'record' || field.kind === 'records') && field.of[0] === of).map((field) => field.name);
}

/**
 * Whether what is asked on a card (`words`) names a row under the row that
 * asked it: by what the last of the fields its table says it is called by
 * holds (`calls`: an attempt by its contender, `otter`), as a word of what
 * was asked, whatever its capitals. A row whose table says it is called by
 * nothing is named by nothing, so that no id is looked for in words.
 */
export function namedIn(words: string, row: Row, fields: Field[]): boolean {
  const calling = fields.filter((one) => (one.calls ?? 0) > 0).sort((a, b) => a.calls! - b.calls!);
  const last = calling.at(-1);
  const name = last ? said(row.data[last.name]).trim().toLowerCase() : '';
  if (!name) return false;
  const text = words.toLowerCase();
  let from = 0;
  for (let at = text.indexOf(name, from); at >= 0; at = text.indexOf(name, from)) {
    const before = text[at - 1];
    const after = text[at + name.length];
    const edge = (letter: string | undefined) => letter === undefined || !/[\p{L}\p{N}_-]/u.test(letter);
    if (edge(before) && edge(after)) return true;
    from = at + 1;
  }
  return false;
}

// A table that sits beside another.
//
// A table may say it sits beside another of its project (`beside`, where
// its blueprint declares it), and which of its own fields says what of
// that table a row is about (`about`): a fleet's `stages` sit beside the
// `board`, each row about one of its columns. That is how the table is
// shown and nothing else: it is reached from the page of the table it
// sits beside, and is no line of its own under the project. The client
// knows nothing of what such a table is of; it knows that one sits beside
// another. This holds nothing of the page, so that its tests can read it.

import { knownBy, said } from './table.ts';
import type { Field, Row, Sits } from './table.ts';

/** As much of a table as this goes by. */
export interface Sitting extends Sits {
  name: string;
  fields: Field[];
}

/**
 * The table `one` sits beside, of the tables its project has; nothing
 * where it says none, or names one that is not there, which is to sit
 * beside nothing. Nor where following what each sits beside comes round
 * to a table already passed: tables that sit beside each other would
 * each be reached only from the other, so neither is taken at its word.
 */
export function sitsBeside(one: Sitting, all: Sitting[]): string | null {
  const beside = (table: Sitting) => all.find((other) => other.name === table.beside && other.name !== table.name);
  const first = beside(one);
  if (!first) return null;
  const passed = new Set([one.name]);
  for (let at: Sitting | undefined = first; at; at = beside(at)) {
    if (passed.has(at.name)) return null;
    passed.add(at.name);
  }
  return first.name;
}

/** The tables that sit beside the one called `name`, by name. */
export function besideOf<T extends Sitting>(name: string, all: T[]): T[] {
  return all.filter((one) => sitsBeside(one, all) === name).sort((a, b) => a.name.localeCompare(b.name));
}

/** A table that sits beside another, with its rows; `null` until they have been read. */
export interface Sat<T extends Sitting = Sitting> {
  table: T;
  rows: Row[] | null;
}

/**
 * What a row of a table that says `about` is about: what that field of
 * the row says, and where it says nothing, what the row is called. A
 * table that says no `about` has rows that are about nothing.
 */
export function aboutOf(table: Sitting, row: Row): string {
  if (!table.about || !table.fields.some((field) => field.name === table.about)) return '';
  return said(row.data[table.about]).trim() || knownBy(row, table.fields);
}

/** A row that is about something, of which table, and what it is called. */
export interface About {
  table: string;
  row: Row;
  called: string;
}

/** The rows of the tables beside one that are about `what` (a column of it), a table at a time, in the order of their rows. */
export function aboutThis(sat: Sat[], what: string): About[] {
  return sat.flatMap(({ table, rows }) =>
    (rows ?? [])
      .filter((row) => what !== '' && aboutOf(table, row) === what)
      .sort((a, b) => a.position - b.position || a.number - b.number)
      .map((row) => ({ table: table.name, row, called: knownBy(row, table.fields) })),
  );
}

/** Words that end in `s` and are no plural of anything a record could be called. */
const NO_PLURAL = ['news', 'series', 'species'];

/**
 * What one record of a table is called, by the English its name is
 * written in: `entry` of `entries`, `box` of `boxes`, `agent` of `agents`;
 * and `row` where its name says nothing of one (`staff`, `status`,
 * `news`), never a word made up by dropping a letter (`entrie`).
 */
export function oneOf(name: string): string {
  if (name.length < 3 || !name.endsWith('s') || NO_PLURAL.includes(name) || /(ss|us|is)$/.test(name)) return 'row';
  if (name.endsWith('ies')) return `${name.slice(0, -3)}y`;
  if (/(sses|tuses|ches|shes|xes|zzes)$/.test(name)) return name.slice(0, -2);
  return name.slice(0, -1);
}

/** A word made plural as English makes it: `attempts` of `attempt`, `entries` of `entry`, `boxes` of `box`. */
function pluralOf(word: string): string {
  if (/[^aeiou]y$/.test(word)) return `${word.slice(0, -1)}ies`;
  if (/(s|x|z|ch|sh)$/.test(word)) return `${word}es`;
  return `${word}s`;
}

/** As much of a table as what its rows are called goes by: its name, and what it is called for people. */
export type Called = string | { name: string; title?: string | null; title_chosen?: string | null };

/**
 * What one of a table's rows is called and what several are: by the last
 * word of what the table is called for people where it is called something
 * (`A/B attempts`: an attempt, attempts; `Test run`: a run, runs), and else
 * by its name (`entries`: an entry, entries; `staff`: a row, rows).
 */
export function rowWords(table: Called): { one: string; many: string } {
  const name = typeof table === 'string' ? table : table.name;
  const called = typeof table === 'string' ? null : table.title_chosen || table.title || null;
  const word = called?.trim().split(/\s+/).at(-1)?.toLowerCase() ?? '';
  if (/^[a-z]+$/.test(word)) {
    const one = oneOf(word);
    if (one !== 'row') return { one, many: word };
    // A title that is only the table's own name (`Git` of `git`) says nothing more of what one of its rows is.
    if (!word.endsWith('s') && word !== name.toLowerCase()) return { one: word, many: pluralOf(word) };
  }
  const one = oneOf(name);
  // More than one is the table's own name, which is what its rows are called together; where that says nothing, rows.
  return { one, many: one === 'row' ? 'rows' : name };
}

/** How many rows a table has, in its own words: `3 agents`, `1 stage`, `4 attempts`, `no segments yet`; nothing until they are read. */
export function counted(table: Called, rows: Row[] | null): string {
  if (rows === null) return '';
  const { one, many } = rowWords(table);
  if (rows.length === 0) return `no ${many} yet`;
  return `${rows.length} ${rows.length === 1 ? one : many}`;
}

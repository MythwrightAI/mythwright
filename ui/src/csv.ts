// A table written out as a CSV file, and one read back in.
//
// Nothing of the page is here, and nothing of the daemon: writing a table
// out and reading one in are the client's alone, over the records the
// page read and the routes that add and change one record.
//
// The first line names the columns: `id`, then each field by its name,
// as its table declares it (never by how a head words it). A line after
// it is a record. A cell is the field's value as text: a number as its
// figures, a flag as `true` or `false`, a list as its things with `; `
// between, a field that names a record as that record's id, and one that
// names several as their ids with `; ` between. A field the table says is
// kept on a record for whatever looks after it is not written, since
// nobody fills one in.
//
// Read back, a line whose `id` is a record of the table changes that
// record, in the cells where it says something else than the record
// does; a line with no `id` is a record to add. So a table written out
// and read straight back changes nothing. Every record of the table is
// written, whatever view is being looked at.

import { columns, filled } from './table.ts';
import type { Field, Row } from './table.ts';

/** What stands between the things of a cell that holds several. */
const BETWEEN = '; ';

/** The fields a file has a column for, in the order they are written. */
export function written(fields: Field[]): Field[] {
  return filled(columns(fields));
}

const several = (field: Field) => field.kind === 'list' || field.kind === 'records';

/** A thing among several, written so that a `;` of its own is not taken for the end of it. */
const guarded = (thing: string) => thing.replace(/\\/g, '\\\\').replace(/;/g, '\\;');

/** The things of a cell that holds several. */
function things(cell: string): string[] {
  const all: string[] = [];
  let now = '';
  for (let at = 0; at < cell.length; at += 1) {
    const here = cell[at]!;
    if (here === '\\' && at + 1 < cell.length) now += cell[(at += 1)];
    else if (here === ';') {
      all.push(now);
      now = '';
    } else now += here;
  }
  all.push(now);
  return all.map((one) => one.trim()).filter((one) => one !== '');
}

/** A field's value as the text of its cell; nothing, for a field that says nothing. */
export function cellOf(field: Field, value: unknown): string {
  if (value === null || value === undefined) return '';
  if (several(field) && Array.isArray(value)) return value.map((one) => guarded(String(one))).join(BETWEEN);
  if (typeof value === 'string') return value;
  if (typeof value === 'number' || typeof value === 'boolean') return String(value);
  return JSON.stringify(value);
}

/** What a cell says of a field, as the daemon is told it; or why it cannot be read. An empty cell says nothing. */
export function valueOf(field: Field, cell: string): { value: unknown } | { wrong: string } {
  const text = cell.trim();
  if (text === '') return { value: null };
  if (several(field)) return { value: things(cell) };
  if (field.kind === 'number') {
    const figure = Number(text);
    return Number.isFinite(figure) ? { value: figure } : { wrong: `\`${field.name}\` is a number, and "${text}" is not one` };
  }
  if (field.kind === 'flag') {
    if (/^(true|yes|1)$/i.test(text)) return { value: true };
    if (/^(false|no|0)$/i.test(text)) return { value: false };
    return { wrong: `\`${field.name}\` is true or false, and "${text}" is neither` };
  }
  // Text is kept as it was written, with the room about it; anything else is a word.
  return { value: field.kind === 'text' ? cell : text };
}

/** A cell as a file holds it: in quotation marks where it holds a comma, a quotation mark, a line's end or room at its edges. */
function quoted(cell: string): string {
  return /[",\r\n]|^\s|\s$/.test(cell) ? `"${cell.replace(/"/g, '""')}"` : cell;
}

/** A table as the text of a CSV file. */
export function toCsv(fields: Field[], rows: Row[]): string {
  const of = written(fields);
  const lines = [['id', ...of.map((field) => field.name)], ...rows.map((row) => [row.id, ...of.map((field) => cellOf(field, row.data[field.name]))])];
  return lines.map((line) => line.map(quoted).join(',')).join('\r\n') + '\r\n';
}

/** The records of a CSV file, each as its cells and the line of the file it begins on. A line with nothing on it is left out. */
export function records(text: string): { line: number; cells: string[] }[] {
  const all: { line: number; cells: string[] }[] = [];
  let cells: string[] = [];
  let cell = '';
  let within = false;
  // Which line of the file is being read, and which the record being read began on: a cell may hold a line's end.
  let [now, began] = [1, 1];
  const end = () => {
    cells.push(cell);
    if (cells.length > 1 || cells[0] !== '') all.push({ line: began, cells });
    cells = [];
    cell = '';
  };
  // A mark some programs put before everything they write is no part of the first cell.
  const read = text.replace(/^\uFEFF/, '');
  for (let at = 0; at < read.length; at += 1) {
    const here = read[at]!;
    const ends = here === '\n' || here === '\r';
    if (here === '\r' && read[at + 1] === '\n') {
      // The two that end a line together are one end.
      if (within) cell += here;
      continue;
    }
    if (ends) now += 1;
    if (within) {
      if (here !== '"') cell += here;
      else if (read[at + 1] === '"') cell += read[(at += 1)];
      else within = false;
    } else if (here === '"' && cell === '') within = true;
    else if (here === ',') {
      cells.push(cell);
      cell = '';
    } else if (ends) {
      end();
      began = now;
    } else cell += here;
  }
  if (cell !== '' || cells.length > 0) end();
  return all;
}

/** The lines of a CSV file, each as its cells. */
export function parse(text: string): string[][] {
  return records(text).map((one) => one.cells);
}

export interface Line {
  /** Which line of the file it is, counted from 1 as an editor counts them. */
  line: number;
  fields: Record<string, unknown>;
}

/** What reading a file into a table would do. */
export interface Plan {
  /** Records to add. */
  add: Line[];
  /** Records to change, each with only the cells that say something else than it does. */
  change: (Line & { id: string })[];
  /** How many lines say what their record says already. */
  same: number;
  /** Lines that cannot be read in, each with why. */
  wrong: { line: number; why: string }[];
  /** Columns of the file that are no field of the table, which are left out. */
  left: string[];
}

/**
 * What a file would do to a table that has these fields and these
 * records. A line is said by where it begins in the file, as an editor
 * counts them.
 * `given` are the fields a new record need not say though its table has
 * every record say them, since whatever adds one gives them: a card
 * that says no column is put in its board's first.
 */
export function planOf(fields: Field[], rows: Row[], text: string, given: string[] = []): Plan {
  const plan: Plan = { add: [], change: [], same: 0, wrong: [], left: [] };
  const [first, ...lines] = records(text);
  const head = first?.cells;
  if (!head) {
    plan.wrong.push({ line: 1, why: 'the file is empty' });
    return plan;
  }
  const of = written(fields);
  const names = head.map((name) => name.trim());
  const cols = names.map((name) => of.find((field) => field.name === name) ?? null);
  const ids = names.indexOf('id');
  plan.left = names.filter((name, at) => name !== '' && at !== ids && cols[at] === null);
  if (cols.every((one) => one === null)) {
    plan.wrong.push({ line: 1, why: `its first line names no field of this table (${of.map((field) => field.name).join(', ')})` });
    return plan;
  }
  const known = new Map(rows.map((row) => [row.id, row]));
  for (const { line, cells } of lines) {
    const id = ids < 0 ? '' : (cells[ids] ?? '').trim();
    const row = id ? known.get(id) : undefined;
    if (id && !row) {
      plan.wrong.push({ line, why: `no record of this table is called ${id}; with its id left empty the line is a record to add` });
      continue;
    }
    const said: Record<string, unknown> = {};
    let why = '';
    for (const [at, field] of cols.entries()) {
      if (!field || why) continue;
      const read = valueOf(field, cells[at] ?? '');
      if ('wrong' in read) why = read.wrong;
      // What the record says already is not said again: a cell that comes to what the record holds changes nothing,
      // however it is written (`TRUE`, a list with no room after its `;`).
      else if (row ? cellOf(field, read.value) !== cellOf(field, row.data[field.name]) : read.value !== null) said[field.name] = read.value;
    }
    const lacking = row ? undefined : of.find((field) => field.required && !given.includes(field.name) && said[field.name] === undefined);
    if (why) plan.wrong.push({ line, why });
    else if (lacking) plan.wrong.push({ line, why: `it says nothing of \`${lacking.name}\`, which a record has to say` });
    else if (row && Object.keys(said).length === 0) plan.same += 1;
    else if (row) plan.change.push({ line, id, fields: said });
    else if (Object.keys(said).length === 0) plan.wrong.push({ line, why: 'it says nothing of any field' });
    else plan.add.push({ line, fields: said });
  }
  return plan;
}

/** What a plan comes to, in a sentence: how many are added and changed, and what is left as it is. */
export function summed(plan: Plan, what: string): string {
  const count = (many: number, one: string) => `${many} ${many === 1 ? one : `${one}s`}`;
  const parts = [`${count(plan.add.length, what)} to add`, `${plan.change.length} to change`];
  if (plan.same > 0) parts.push(`${plan.same} that ${plan.same === 1 ? 'says' : 'say'} what is there already`);
  return parts.join(', ');
}

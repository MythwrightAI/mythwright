// What a table read a window at a time does with the
// rows it is told changed, so that nothing on the screen jumps.
//
// The page is told only which record of its table was added, changed or
// taken away. It gathers the ids it hears for a moment and asks the
// daemon once how the view shows each now (`ids=`), and then:
//
//   - a row it holds that the view still shows is drawn where it is with
//     what it now says, even where it would now stand elsewhere: then it
//     is marked as moved, and counted;
//   - a row it holds that the view no longer shows (it was taken away, or
//     no longer holds what the view is held to) is drawn where it is,
//     struck, saying what became of it, until the view is read again;
//   - a row it does not hold that was added and that the view shows is
//     counted as new, but where nobody sorts the view: a row added is
//     given the highest number, so it comes after every row of its band
//     and nothing drawn moves for it, and it is read in.
//
// What is counted is said in one line at the top of the table, with one
// press that reads the view again where the reader is (`said`). Past
// `MANY` ids heard before they are asked about, nothing is asked by id:
// the line says that many rows changed. A row the reader changed
// themselves is drawn as the daemon answers it and is none of these: one
// they took out of the view is taken out.
//
// Everything here is a function of plain values with its cases
// (`rowslive.test.ts`); what asks and keeps is `rowsread.ts`.

import { said as cellSaid, heading } from './table.ts';
import type { Field, Row, Sort } from './table.ts';
import type { Counted, Read } from './rowsapi.ts';
import type { Run } from './window.ts';

/** What the page was told of a record. */
export type Heard = 'created' | 'updated' | 'removed';

/** Past this many ids heard before an answer, nothing is asked by id. */
export const MANY = 200;

/** How long ids heard are gathered before they are asked about. */
export const GATHER = 250;

/** How the view shows one row now, as `ids=` answers it. */
export interface Seen {
  id: string;
  in: boolean;
  row: Read | null;
}

/** What a row drawn in place says of itself: that it would stand elsewhere, or that the view no longer shows it, and why. */
export type Mark = 'moved' | 'removed' | 'out';

/** What is counted of the changes since the view was last read. */
export interface Live {
  marks: Record<string, Mark>;
  /** The rows added that the view shows and the page does not hold, by id. */
  fresh: string[];
  /** More changed than were asked about: the view is to be read again. */
  many: boolean;
}

export const NONE: Live = { marks: {}, fresh: [], many: false };

/** Which of two kinds of being told stands: being added or taken away stands over being changed. */
export function heardBoth(was: Heard | undefined, now: Heard): Heard {
  if (!was) return now;
  if (now === 'removed') return 'removed';
  if (was === 'created') return 'created';
  return now;
}

/**
 * Whether a row would stand elsewhere now: where the view is sorted by a
 * column or gathered by one and what the row says there changed. A view
 * nobody sorts is read by the number each row was given, which never
 * changes.
 */
export function moves(was: Row, now: Row, sort: Sort | null, by: string | null): boolean {
  return [sort?.field, by].some((field) => field && cellSaid(was.data[field]) !== cellSaid(now.data[field]));
}

/** The row of `id` wherever the runs of any band hold it, with the band's key. */
export function holding(bands: Record<string, Run[]>, id: string): { key: string; row: Row }[] {
  const found: { key: string; row: Row }[] = [];
  for (const [key, runs] of Object.entries(bands)) {
    for (const run of runs) {
      const row = run.rows.find((one) => one?.id === id);
      if (row) found.push({ key, row });
    }
  }
  return found;
}

/** The runs with the row of `id` standing where it was, as `row` now says. */
export function drawnAgain(runs: Run[], id: string, row: Row): Run[] {
  return runs.map((run) => (run.rows.some((one) => one?.id === id) ? { ...run, rows: run.rows.map((one) => (one?.id === id ? row : one)) } : run));
}

/**
 * What an answer for the ids heard comes to: each band's runs with every
 * row held drawn as it now is, and what is counted. `mine` are rows the
 * reader changed themselves, which are drawn and never counted; one of
 * theirs the view no longer shows is to be taken out (`dropped`), as the
 * daemon answers. Where the view is read by the number each row was
 * given (`appends`: nobody sorts it), a row added comes after every row
 * there is in its band, so nothing drawn moves for it, and it is to be
 * read in (`appended`) and not counted.
 */
export function seenOf(
  bands: Record<string, Run[]>,
  live: Live,
  seen: Seen[],
  heard: Record<string, Heard>,
  mine: Set<string>,
  sort: Sort | null,
  by: string | null,
): { bands: Record<string, Run[]>; live: Live; appended: string[]; dropped: string[] } {
  const next = { ...bands };
  const marks = { ...live.marks };
  const fresh = [...live.fresh];
  const appended: string[] = [];
  const dropped: string[] = [];
  for (const one of seen) {
    const held = holding(next, one.id);
    if (held.length === 0) {
      const added = one.in && heard[one.id] === 'created';
      if (added && (mine.has(one.id) || !sort)) appended.push(one.id);
      else if (added && !fresh.includes(one.id)) fresh.push(one.id);
      if (!one.in) {
        const at = fresh.indexOf(one.id);
        if (at >= 0) fresh.splice(at, 1);
      }
      continue;
    }
    for (const { key, row } of held) {
      if (one.row) next[key] = drawnAgain(next[key]!, one.id, one.row);
      if (mine.has(one.id)) {
        // The reader's own change is no move under them, and what they took out of the view is taken out.
        if (!one.in && !dropped.includes(one.id)) dropped.push(one.id);
        continue;
      }
      if (!one.in) marks[one.id] = one.row ? 'out' : 'removed';
      else if (one.row && (marks[one.id] === 'moved' || moves(row, one.row, sort, by))) marks[one.id] = 'moved';
      else delete marks[one.id];
    }
  }
  return { bands: next, live: { marks, fresh, many: live.many }, appended, dropped };
}

/** The line at the top of the table, or nothing where nothing is counted. */
export function said(live: Live): string | null {
  if (live.many) return 'Many rows changed';
  const moved = Object.values(live.marks).filter((mark) => mark === 'moved').length;
  const gone = Object.values(live.marks).filter((mark) => mark !== 'moved').length;
  const parts = [
    live.fresh.length > 0 ? `${live.fresh.length} new ${live.fresh.length === 1 ? 'row' : 'rows'}` : null,
    moved > 0 ? `${moved} moved` : null,
    gone > 0 ? `${gone} no longer here` : null,
  ].filter((one): one is string => one !== null);
  return parts.length === 0 ? null : parts.join(' · ');
}

/** What a row drawn in place says of what became of it. */
export function markSaid(mark: Mark): string {
  return mark === 'moved' ? 'moved' : mark === 'removed' ? 'removed' : 'no longer in this view';
}

/** What is said under the boxes while a search goes on: how far it has looked. */
export function lookedSaid(counted: Counted | null, find: string): string | null {
  if (!find.trim() || !counted || counted.whole || counted.looked === undefined) return null;
  return `Looked through ${counted.looked.toLocaleString()} of ${counted.of.toLocaleString()}`;
}

/** What keeps a row out of a view, in words: each column it is held to that the row holds none of, and the search. */
export function outSaid(out: string[], fields: Field[]): string {
  const named = out.map((name) => (name === 'find' ? 'what is searched for' : heading(fields.find((field) => field.name === name) ?? { name })));
  const joined = named.length <= 1 ? (named[0] ?? '') : `${named.slice(0, -1).join(', ')} and ${named.at(-1)}`;
  return `It is not in this view: ${joined} ${named.length === 1 ? 'keeps' : 'keep'} it out.`;
}

/** The band a row is drawn under, of the column a view is gathered by: the first thing it holds there, `''` where it holds nothing. */
export function bandOfRow(value: unknown): string {
  const first = Array.isArray(value) ? value.find((one) => String(one ?? '').trim() !== '') : value;
  return first === undefined || first === null ? '' : typeof first === 'boolean' ? (first ? 'yes' : '') : String(first).trim();
}

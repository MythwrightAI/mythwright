// What the daemon is asked of a table read a window at a time:
// one window of a view's rows (`GET …/rows`), and what is counted of the
// view (`GET …/rows/counted`). The daemon sorts, holds the view to what it
// is held to, gathers it into bands and works out its figures; a page asks
// for the rows it is to draw and nothing more (`rowsread.ts`). `api.ts` is
// at its ceiling, so this is a file of its own.

import { call } from './api.ts';
import type { Row, Sort } from './table.ts';

/** Where a window is read from: the start, on from a place or back from one (each handed out by an answer before), a rank, or about a row by its id or its number. */
export type From = { start: true } | { after: string } | { before: string } | { at: number } | { around: string };

/** What a window is asked. */
export interface Asked {
  /** The view it is of, where it is a view the table keeps; every row where none is said. */
  view?: string | null;
  sort?: Sort | null;
  /** One band of a view gathered by a column: what that band is of, `''` for those that say nothing there. */
  band?: string | null;
  /** Words searched for among the columns the view shows. */
  find?: string | null;
  from?: From;
  limit?: number;
}

/** A row as a window hands it: a record, and how many things were said on it. */
export type Read = Row & { commented?: number; created_by?: string | null };

/** One window, as it is answered. */
export interface Answer {
  rows: Read[];
  /** Where reading on from its last row begins; nothing at the end. */
  after: string | null;
  /** Where reading back from its first row begins; nothing at the start. */
  before: string | null;
  looked: number;
  /** Fewer rows than asked and not at the end: how far one answer looks was reached, and `after` (or `before`) says where it got to. */
  short: boolean;
  /** Where the first row stands, where it was asked by a rank. */
  rank?: number;
  /** How many rows were read whole where the table is kept, past what a window is read from cheaply. */
  whole_read?: number;
  /** Of a window about a row: the row, whether the view shows it and where it stands among those it shows, or what keeps it out (a column it is held to, or `find`). */
  around?: { id: string; in: boolean; rank?: number; kept_out?: string[]; row?: Read };
}

/** What is counted of a view. */
export interface Counted {
  /** How many rows the view shows, and how many the table has. */
  count: number;
  of: number;
  /** The view's bands with how many rows each, where it is gathered by a column; with how many more there are beyond them. */
  bands: { key: string; count: number }[] | null;
  more_bands: number;
  /** The figure at the foot of each column that says one, by the column: nothing where there is nothing to go by. */
  figures: Record<string, number | null>;
  /** Every row was gone through: where not, each count is at least so many, and asking again goes on. */
  whole: boolean;
  /** How many of the table's rows were gone through to count what is so far. */
  looked?: number;
}

/** The words of an address that asks for `asked`, in the order they are always said. */
export function addressed(asked: Asked, figures?: Record<string, string>): string {
  const parts: [string, string][] = [];
  if (asked.view) parts.push(['view', asked.view]);
  if (asked.sort) {
    parts.push(['sort', asked.sort.field]);
    if (asked.sort.down) parts.push(['down', '1']);
  }
  // A band of the rows that say nothing is said as nothing, which is a band all the same.
  if (asked.band !== undefined && asked.band !== null) parts.push(['band', asked.band]);
  if (asked.find) parts.push(['find', asked.find]);
  const from = asked.from;
  if (from && 'after' in from) parts.push(['after', from.after]);
  else if (from && 'before' in from) parts.push(['before', from.before]);
  else if (from && 'at' in from) parts.push(['at', String(from.at)]);
  else if (from && 'around' in from) parts.push(['around', from.around]);
  if (asked.limit !== undefined) parts.push(['limit', String(asked.limit)]);
  if (figures) parts.push(['figures', Object.entries(figures).map(([column, which]) => `${column}:${which}`).join(',')]);
  return parts.length === 0 ? '' : `?${parts.map(([key, value]) => `${key}=${encodeURIComponent(value)}`).join('&')}`;
}

const part = encodeURIComponent;

/** The words of an address that asks how the view `asked` shows each of `ids`. */
export function withIds(asked: Asked, ids: string[]): string {
  const said = addressed(asked);
  return `${said}${said ? '&' : '?'}ids=${ids.map(part).join(',')}`;
}

export const rowsApi = {
  /** One window of `name`'s rows. */
  window: (project: string, name: string, asked: Asked) => call<Answer>('GET', `/collections/${part(project)}/${part(name)}/rows${addressed(asked)}`),
  /** Each of `ids` as the view shows it now: whether it does, and the row; none for one that is not a row of it. */
  seen: (project: string, name: string, asked: Asked, ids: string[]) =>
    call<{ rows: { id: string; in: boolean; row: Read | null }[] }>('GET', `/collections/${part(project)}/${part(name)}/rows${withIds({ view: asked.view, sort: asked.sort, find: asked.find }, ids)}`),
  /** What is counted of a view of `name`: the figures asked, or the view's own. */
  counted: (project: string, name: string, asked: Asked, figures?: Record<string, string>) =>
    call<Counted>('GET', `/collections/${part(project)}/${part(name)}/rows/counted${addressed({ ...asked, from: undefined, limit: undefined }, figures)}`),
};

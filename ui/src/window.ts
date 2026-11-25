// A table drawn a window at a time: what of it is on the screen,
// what has to be asked for to draw that, and where everything stands.
//
// A table may hold a million rows. The page never has them all: it keeps
// the rows it has read as runs of rows that follow one another (`Run`),
// each with where reading on and reading back from it begin, at most so
// many of them (`KEPT`), the far ones let go of; it draws only those on
// the screen and so many either side of it (`OVER`), with what is above
// and below them stood in for by room as tall as they would be; and it
// asks for what is to be drawn that it has not got (`missing`). The room
// is as tall as the rows the view shows (each as tall as the first that
// was drawn, where none was measured otherwise: `Sizes`), so the
// scrollbar says where the reader is among all of them.
//
// A browser draws nothing taller than some millions of pixels, so past
// `CAP` the room is that tall and the page is read in pages of it: where
// the reader is among all the rows is where the scrollbar is among the
// room, wheel and keys move by real pixels inside it, and the page moved
// under the reader as they near its edge (`paged`).
//
// Everything here is a function of plain values with its cases
// (`window.test.ts`); what reads is `rowsread.ts` and what draws is
// `views/windowed.tsx`.

import type { Answer, Counted, From } from './rowsapi.ts';
import type { Row, Sort } from './table.ts';

/** Past this many rows a view's bands start folded, and each is read as it is opened: the daemon's `SMALL`. */
export const SMALL = 10_000;
/** How many rows one window is asked for. */
export const LIMIT = 100;
/** How many rows are drawn past each edge of the screen, so that a quick scroll meets rows already drawn. */
export const OVER = 40;
/** How many rows of one band are kept at most: the furthest from what is drawn are let go of. */
export const KEPT = 600;
/** How tall the room a table scrolls in is made at most, in pixels: under what every browser draws. */
export const CAP = 8_000_000;

/**
 * Rows that follow one another in the view's order: the rank of the
 * first among every row the view shows, the rows, and where reading back
 * from the first and on from the last begin. A place is `null` where
 * there is nothing that way (the start, the end), and not said where it
 * is not known (a run cut short of what was read).
 */
export interface Run {
  start: number;
  rows: Row[];
  before?: string | null;
  after?: string | null;
}

/** Where one past the last row of a run stands. */
export const endOf = (run: Run): number => run.start + run.rows.length;

/**
 * The runs once `read` is among them: joined with every run it lies over
 * or touches, its rows standing over theirs where they lie over each
 * other, and reading on and back from the joined run begins where it does
 * from whichever of them reaches furthest that way (`read`, where they
 * reach as far). A read of no rows changes nothing: what it says of the
 * end is the reader's to keep.
 */
export function joined(runs: Run[], read: Run): Run[] {
  if (read.rows.length === 0) return runs;
  const touching = (run: Run) => run.start <= endOf(read) && endOf(run) >= read.start;
  const near = runs.filter(touching);
  const apart = runs.filter((run) => !touching(run));
  const start = Math.min(read.start, ...near.map((run) => run.start));
  const end = Math.max(endOf(read), ...near.map(endOf));
  const rows: Row[] = new Array(end - start);
  for (const run of near) run.rows.forEach((row, nth) => (rows[run.start - start + nth] = row));
  read.rows.forEach((row, nth) => (rows[read.start - start + nth] = row));
  const first = read.start === start ? read : near.find((run) => run.start === start)!;
  const last = endOf(read) === end ? read : near.find((run) => endOf(run) === end)!;
  const one: Run = { start, rows, before: first.before, after: last.after };
  return [...apart, one].sort((a, b) => a.start - b.start);
}

/** The row at `rank`, where it was read. */
export function rowAt(runs: Run[], rank: number): Row | undefined {
  const run = runs.find((one) => one.start <= rank && rank < endOf(one));
  return run?.rows[rank - run.start];
}

/** How many rows the runs hold. */
export const held = (runs: Run[]): number => runs.reduce((all, run) => all + run.rows.length, 0);

/** What is asked next to have a row of a band: where it is read from, and the rank the first row it answers will have (of a read back, the rank one past its last). */
export type Next = { start: true; rank: 0 } | { after: string; rank: number } | { before: string; rank: number } | { at: number; rank: number };

/**
 * What to ask for next so as to have every row of `[from, to)`, where
 * the band has `count` rows and none is known to be at or past `end`:
 * on from a run that ends where the first row missing is, back from one
 * that begins just past a gap a window fills, or from the rank of the
 * first row missing (the start, at the very first). Nothing where all of
 * them are there.
 */
export function missing(runs: Run[], from: number, to: number, count: number, end: number | null = null): Next | null {
  const last = Math.min(to, count, end ?? Number.POSITIVE_INFINITY);
  let rank = Math.max(0, from);
  while (rank < last) {
    const run = runs.find((one) => one.start <= rank && rank < endOf(one));
    if (!run) break;
    rank = endOf(run);
  }
  if (rank >= last) return null;
  const on = runs.find((run) => endOf(run) === rank);
  if (on && typeof on.after === 'string') return { after: on.after, rank };
  const ahead = runs.filter((run) => run.start > rank).sort((a, b) => a.start - b.start)[0];
  if (ahead && typeof ahead.before === 'string' && ahead.start - rank <= LIMIT) return { before: ahead.before, rank: ahead.start };
  return rank === 0 ? { start: true, rank: 0 } : { at: rank, rank };
}

/**
 * The runs with no more than `most` rows, those nearest `[from, to)`
 * kept: a run cut there no longer knows where reading on or back from
 * the cut begins.
 */
export function kept(runs: Run[], from: number, to: number, most = KEPT): Run[] {
  if (held(runs) <= most) return runs;
  const middle = Math.floor((from + to) / 2);
  const low = Math.max(0, middle - Math.floor(most / 2));
  const high = low + most;
  return runs.flatMap((run) => {
    const a = Math.max(run.start, low);
    const b = Math.min(endOf(run), high);
    if (a >= b) return [];
    return [{ start: a, rows: run.rows.slice(a - run.start, b - run.start), before: a === run.start ? run.before : undefined, after: b === endOf(run) ? run.after : undefined }];
  });
}

/**
 * How tall the rows of a band are: each as tall as `each` (the first row
 * that was drawn), but those measured otherwise (one opened into what
 * names it, a wrapped one), which are kept by their rank in order.
 */
export type Odd = [number, number][];

/** Where the row at `rank` begins among its band's rows. */
export function topOf(each: number, odd: Odd, rank: number): number {
  let top = rank * each;
  for (const [at, height] of odd) {
    if (at >= rank) break;
    top += height - each;
  }
  return top;
}

/** How tall the row at `rank` is. */
export function heightOf(each: number, odd: Odd, rank: number): number {
  return odd.find(([at]) => at === rank)?.[1] ?? each;
}

/** The rank of the row whose span holds `y`, among `count` rows: the last where `y` is past them all, the first before them. */
export function rankAt(each: number, odd: Odd, y: number, count: number): number {
  if (count <= 0 || y <= 0) return 0;
  let top = 0;
  let rank = 0;
  for (const [at, height] of odd) {
    if (at >= count) break;
    const plain = (at - rank) * each;
    if (y < top + plain) return Math.min(count - 1, rank + Math.floor((y - top) / each));
    top += plain;
    if (y < top + height) return at;
    top += height;
    rank = at + 1;
  }
  return Math.min(count - 1, rank + Math.floor((y - top) / Math.max(1, each)));
}

/** The heights kept once the row at `rank` measured `height`: kept only where it is not `each`, and the furthest from `[from, to)` let go of past `most`. */
export function measured(each: number, odd: Odd, rank: number, height: number, from = 0, to = Number.POSITIVE_INFINITY, most = KEPT): Odd {
  const other = odd.filter(([at]) => at !== rank);
  const next = Math.abs(height - each) < 0.5 ? other : [...other, [rank, height] as [number, number]].sort((a, b) => a[0] - b[0]);
  if (next.length <= most) return next;
  const near = (at: number) => (at < from ? from - at : at >= to ? at - to : 0);
  return [...next].sort((a, b) => near(a[0]) - near(b[0])).slice(0, most).sort((a, b) => a[0] - b[0]);
}

/**
 * A band of what a table draws, in order: a view gathered by a column
 * has one for each thing it says, each with a line at its head as tall as
 * `head` and its rows under it where it is open; a view gathered by none
 * is one band with no head.
 */
export interface Segment {
  key: string;
  head: number;
  count: number;
  open: boolean;
  odd: Odd;
}

/** How tall the rows of a segment are, drawn. */
const rowsOf = (segment: Segment, each: number): number => (segment.open ? topOf(each, segment.odd, segment.count) : 0);

/** How tall everything drawn is. */
export function totalOf(segments: Segment[], each: number): number {
  return segments.reduce((all, segment) => all + segment.head + rowsOf(segment, each), 0);
}

/** A part of what is drawn: a band's head, or its rows from `from` up to `to`. */
export type Piece = { key: string; head: true } | { key: string; head: false; from: number; to: number };

/**
 * What is drawn for `[y0, y1)` and `over` rows either side of it: the
 * pieces in order, where the first begins and where the last ends. What
 * is drawn follows on, so that the room above it and below it is all
 * that stands in for the rest.
 */
export function drawn(segments: Segment[], each: number, y0: number, y1: number, over = OVER): { pieces: Piece[]; top: number; bottom: number } {
  const a = y0 - over * each;
  const b = y1 + over * each;
  const pieces: Piece[] = [];
  let top: number | null = null;
  let bottom = 0;
  let y = 0;
  for (const segment of segments) {
    if (segment.head > 0) {
      if (y + segment.head > a && y < b) {
        top ??= y;
        pieces.push({ key: segment.key, head: true });
        bottom = y + segment.head;
      }
      y += segment.head;
    }
    const tall = rowsOf(segment, each);
    if (segment.open && segment.count > 0 && y + tall > a && y < b) {
      const from = rankAt(each, segment.odd, a - y, segment.count);
      const to = rankAt(each, segment.odd, b - y, segment.count) + 1;
      top ??= y + topOf(each, segment.odd, from);
      pieces.push({ key: segment.key, head: false, from, to });
      bottom = y + topOf(each, segment.odd, to);
    }
    y += tall;
  }
  return { pieces, top: top ?? 0, bottom: top === null ? 0 : bottom };
}

/** What stands at the top of the screen: a band's head (`rank` -1) or a row, and how far above the screen's top it begins. */
export interface Anchor {
  key: string;
  rank: number;
  above: number;
}

/** Where a band's head (`rank` -1) or a row begins, where it is drawn at all. */
export function yOf(segments: Segment[], each: number, key: string, rank: number): number | null {
  let y = 0;
  for (const segment of segments) {
    if (segment.key === key) {
      if (rank < 0) return y;
      if (!segment.open || rank >= segment.count) return null;
      return y + segment.head + topOf(each, segment.odd, rank);
    }
    y += segment.head + rowsOf(segment, each);
  }
  return null;
}

/** What stands at `y`. */
export function anchorAt(segments: Segment[], each: number, y: number): Anchor | null {
  let at = 0;
  for (const segment of segments) {
    if (y < at + segment.head) return { key: segment.key, rank: -1, above: y - at };
    at += segment.head;
    const tall = rowsOf(segment, each);
    if (segment.open && segment.count > 0 && y < at + tall) {
      const rank = rankAt(each, segment.odd, y - at, segment.count);
      return { key: segment.key, rank, above: y - at - topOf(each, segment.odd, rank) };
    }
    at += tall;
  }
  return null;
}

/** How tall the room is made for `total` pixels of rows. */
export const roomFor = (total: number, cap = CAP): number => Math.min(total, cap);

/**
 * Where the room's top stands among all the rows (`base`), once what
 * scrolls is at `top` having been at `was.top` with the room's top at
 * `was.base`, with `view` pixels of it on the screen. Under `cap` it is
 * always nought. Past it, a small move is a move by its pixels; a jump
 * (the scrollbar dragged, Home, End, a place put back) is to the same
 * part of all the rows as of the room; and where that leaves the reader
 * near an edge of the room with more rows past it, what scrolls is put
 * (`put`) where the same part of the room is, and the room's top moved so
 * that nothing under the reader moves.
 */
export function paged(was: { top: number; base: number }, top: number, total: number, view: number, cap = CAP): { base: number; put?: number } {
  if (total <= cap) return { base: 0 };
  const most = cap - view;
  const mostY = total - view;
  if (most <= 0) return { base: 0 };
  const jump = Math.abs(top - was.top) > view * 2;
  const y = Math.max(0, Math.min(mostY, jump ? (top / most) * mostY : was.base + top));
  const margin = Math.min(view * 2, most / 4);
  const nearEdge = (top < margin && y > top) || (top > most - margin && y < mostY - (most - top));
  if (!jump && nearEdge) {
    const put = Math.round((y / mostY) * most);
    return { base: Math.max(0, Math.min(total - cap, y - put)), put };
  }
  return { base: Math.max(0, Math.min(total - cap, y - top)) };
}

/** What a page reads of a table (`rowsread.ts`): the key a band's runs are kept by: of a view gathered by no column, the one band there is. */
export const ALL = '*';
export const keyOf = (band: string | null): string => (band === null ? ALL : `=${band}`);

/** What a view asks of the daemon, which reading anew when it changes: the view's name where the table keeps it, the reader's sort, and what the view says that changes which rows it shows and how they are counted. */
export interface Asking {
  view: string | null;
  sort: Sort | null;
  /** What the view says that changes what it shows: what it is held to, gathered by, and its figures. */
  says: unknown;
  /** Words searched for among its rows, where any are. */
  find?: string | null;
}

/** What tells one asking from another, so that what was read for one is never drawn for the next. */
export function markOf(asking: Asking): string {
  const find = asking.find?.trim() || null;
  return JSON.stringify([asking.view, asking.sort?.field ?? null, asking.sort?.down ?? false, asking.says ?? null, ...(find ? [find] : [])]);
}

/** How many rows a band has, where that was counted: of the view gathered by no column, every row it shows. */
export function countOf(counted: Counted | null, band: string | null): number | null {
  if (!counted) return null;
  if (band === null) return counted.count;
  return counted.bands?.find((one) => one.key === band)?.count ?? 0;
}

/** What is said to the daemon for what is asked next. */
export function fromOf(next: Next): From {
  if ('start' in next) return { start: true };
  if ('after' in next) return { after: next.after };
  if ('before' in next) return { before: next.before };
  return { at: next.at };
}

/**
 * What an answer to `next` comes to: the run it read, standing where it
 * does among every row of the band, and where the band's rows are known
 * to end where the answer says there are no more (a read on that went
 * through to the end, or a rank asked for that is past it).
 */
export function readOf(next: Next, answer: Answer): { run: Run; end: number | null } {
  const back = 'before' in next;
  const start = back ? Math.max(0, next.rank - answer.rows.length) : (answer.rank ?? next.rank);
  const run: Run = { start, rows: answer.rows, before: answer.before, after: answer.after };
  const end = !back && answer.after === null ? start + answer.rows.length : null;
  return { run, end };
}


/**
 * Where `y` comes to once what is drawn is laid out as `now` and no
 * longer as `was` (a row measured, a window read in, the count moved):
 * what stood at `y` stands as far into itself as it did, so that nothing
 * on the screen moves. Where it is not drawn now, `y` as it was.
 */
export function shifted(was: { segments: Segment[]; each: number }, now: { segments: Segment[]; each: number }, y: number): number {
  const anchor = anchorAt(was.segments, was.each, y);
  if (!anchor) return y;
  const at = yOf(now.segments, now.each, anchor.key, anchor.rank);
  return at === null ? y : at + anchor.above;
}

/**
 * The runs once an answer to `next` is kept: its rows joined to them, or,
 * where it found no row the view shows before it had looked as far as one
 * answer looks (a view held to something few rows hold), the run it was
 * read on or back from told where it got to, so that the next asking goes
 * on from there and does not look at the same rows again.
 */
export function keptRead(runs: Run[], next: Next, answer: Answer): Run[] {
  if (answer.rows.length > 0) return joined(runs, readOf(next, answer).run);
  // From the start: no row yet, and where reading goes on from kept as a run of none.
  if ('start' in next && typeof answer.after === 'string') return [{ start: 0, rows: [], before: null, after: answer.after }, ...runs.filter((run) => run.rows.length > 0)];
  if ('after' in next && typeof answer.after === 'string') return runs.map((run) => (endOf(run) === next.rank && run.after === next.after ? { ...run, after: answer.after } : run));
  if ('before' in next && typeof answer.before === 'string') return runs.map((run) => (run.start === next.rank && run.before === next.before ? { ...run, before: answer.before } : run));
  return runs;
}

// A table's rows as a page reads them a window at a time: what
// is counted of the view (how many rows it shows and the table has, its
// bands, the figures at its columns' feet), and for each band the runs of
// rows read so far (`window.ts`), filled as the page says which it is to
// draw (`want`) and nothing more: nothing is asked while nothing is
// wanted, and a reading that fell short (the daemon looked as far as one
// answer looks) is read on from where it got to only while what it fills
// is still wanted.
//
// It follows the table as it is told it changed. A record added, changed
// or taken away by anyone is asked about by its id, the ids heard in a
// moment once (`GATHER`), and drawn where it is, marked or struck, with
// what is new counted (`rowslive.ts`): nothing on the screen moves until
// the reader asks for it to be read again (`show`), which reads it about
// the row at the top of the screen and has that row kept where it was
// (`placed`). What arrives while a window is on its way is asked about
// again once the window is in, since the window may have been read
// before it (`lately`). The table declared again, or its columns changed,
// is read again whole, a little after (`AGAIN`). What was read is kept
// by the tab, so that a tab shown again is drawn at once from it and read
// again behind (`useKeptRef`).
//
// It also goes to a row, by its id or its number (`goTo`): read about
// that row, which the drawing brings into view and marks.
//
// What a reading of one band comes to (where its rows stand, where they
// end) is worked out by functions with their cases (`rowsread.test.ts`).

import { useEffect, useMemo, useState } from 'preact/hooks';
import { ApiError } from './api.ts';
import { coalesce } from './events.ts';
import { rowsApi } from './rowsapi.ts';
import type { Answer, Asked, Counted, Read } from './rowsapi.ts';
import { useEvents } from './ui.tsx';
import { countOf, fromOf, keptRead, keyOf, kept, LIMIT, markOf, missing, readOf } from './window.ts';
import type { Asking, Next, Run } from './window.ts';
import { bandOfRow, GATHER, heardBoth, MANY, NONE, seenOf } from './rowslive.ts';
import type { Heard, Live } from './rowslive.ts';
import { useKeptRef } from './views/returned.tsx';

/**
 * How many rows a table has at most for a page to read every one of them
 * as well as its windows, for what still goes by all of them (the other
 * layouts, what a view can be held to, a CSV written out, a row naming
 * another of its table) until each is read another way. Past it nothing
 * reads the table whole.
 */
export const WHOLE = 5_000;

/** How long after the table was said to change it is read again: what changes together is read once. */
export const AGAIN = 120;

/** What is read of one band. */
export interface Band {
  runs: Run[];
  /** Where its rows are known to end, where an answer said so. */
  end: number | null;
}

/** A row the drawing is to keep where it was, or to bring into view: its band's key, its id, and its rank as it was (`from`) and is (`to`). */
export interface Placed {
  key: string;
  id: string;
  from: number | null;
  to: number;
}

/** What going to a row came to. */
export type Gone = { in: true; id: string } | { in: false; id: string; out: string[]; row: Read | null } | { missing: string };

/** What a view's rows are read as. */
export interface Reading {
  counted: Counted | null;
  bands: Record<string, Band>;
  /** Asks for the rows of `[from, to)` of a band (`null`, of a view gathered by no column) as they are wanted, and keeps what is read. */
  want: (band: string | null, from: number, to: number) => void;
  /** Reads again what is drawn and what is counted: about the row at the top of the screen, where one is said. */
  again: (anchor?: { key: string; id: string; rank: number }) => void;
  /** Why it could not be read, where it could not. */
  error: string | null;
  /** What changed since it was read: rows marked where they are, and what is counted (`rowslive.ts`). */
  live: Live;
  /** A row the reader changed themselves: drawn as the daemon answers, and counted as nothing. */
  mine: (id: string) => void;
  /** Reads the view about a row, by its id or its number, for the drawing to bring into view. */
  goTo: (said: string) => Promise<Gone>;
  /** A row the drawing is to keep where it was or bring into view, once; taken by `took`. */
  placed: Placed | null;
  took: () => void;
  /** The row last gone to, marked. */
  marked: string | null;
}

interface Held {
  mark: string;
  counted: Counted | null;
  bands: Record<string, Band>;
  error: string | null;
  /** What is wanted of each band, and whether it is being read, or is to be read again before what was read is drawn. */
  wanted: Record<string, { band: string | null; from: number; to: number }>;
  asking: Set<string>;
  stale: Set<string>;
  counting: boolean;
  recount: boolean;
  /** Each reading again moves this on: an answer to an asking before it is not kept. */
  turn: number;
  /** The ids heard and not yet asked about, an id asked about now, and what was counted of the changes. */
  heard: Record<string, Heard>;
  seeing: boolean;
  live: Live;
  mine: Set<string>;
  /** How many ids were heard, and when each was last: a window asked before an id was heard may hold it as it was. */
  said: number;
  lately: Map<string, number>;
  placed: Placed | null;
  marked: string | null;
  /** What is asked now, and what draws the page again: an answer is kept for the asking there is when it comes. */
  now: { project: string; name: string; asking: Asking; mark: string; on: boolean; draw: () => void };
}

/**
 * The rows of a view of `project`'s `name`, read as `asking` says, by the
 * tab (under `kept`). `on` is false where nothing is to be read (the view
 * is laid out some other way).
 */
export function useRowsRead(project: string, name: string, asking: Asking, on = true, keep = `rows.read:${project}/${name}`): Reading {
  const mark = markOf(asking);
  const [, drawAgain] = useState(0);
  const state = useMemo(() => ({ alive: true }), []);
  const draw = () => state.alive && drawAgain((n) => n + 1);
  const held = useKeptRef<Held>(keep, {
    mark: '', counted: null, bands: {}, error: null, wanted: {}, asking: new Set(), stale: new Set(), counting: false, recount: false, turn: 0,
    heard: {}, seeing: false, live: NONE, mine: new Set(), said: 0, lately: new Map(), placed: null, marked: null,
    now: { project, name, asking, mark, on, draw },
  }).current;
  held.now = { project, name, asking, mark, on, draw };
  const asked = (band: string | null): Asked => ({ view: held.now.asking.view, sort: held.now.asking.sort, find: held.now.asking.find ?? null, band });

  const count = () => {
    // Asked again while a counting is on its way: what it answers may be of before a change, so it is asked once more after.
    if (held.counting) return void (held.recount = true);
    held.counting = true;
    held.recount = false;
    const turn = held.turn;
    const { project, name, asking } = held.now;
    rowsApi.counted(project, name, { view: asking.view, sort: asking.sort, find: asking.find ?? null }).then(
      (counted) => {
        held.counting = false;
        if (turn !== held.turn) return count();
        held.counted = counted;
        if (held.recount) count();
        held.error = null;
        for (const key of Object.keys(held.wanted)) pump(key);
        held.now.draw();
        // What was counted so far goes on where it got to, while the page is there to want it.
        if (!counted.whole && state.alive) count();
      },
      (error: unknown) => {
        held.counting = false;
        if (turn !== held.turn) return count();
        held.error = error instanceof Error ? error.message : String(error);
        held.now.draw();
      },
    );
  };

  const pump = (key: string) => {
    const wanted = held.wanted[key];
    if (!wanted || held.asking.has(key) || !held.counted) return;
    const band = held.bands[key] ?? { runs: [], end: null };
    const stale = held.stale.has(key);
    const total = countOf(held.counted, wanted.band) ?? 0;
    // Read again: from where what is wanted begins, whatever was read.
    const next = stale ? missing([], wanted.from, wanted.to, total) : missing(band.runs, wanted.from, wanted.to, total, band.end);
    if (!next) {
      if (stale) {
        held.stale.delete(key);
        held.bands[key] = { runs: [], end: null };
        held.now.draw();
      }
      return;
    }
    held.asking.add(key);
    const turn = held.turn;
    const since = held.said;
    const { project, name } = held.now;
    rowsApi.window(project, name, { ...asked(wanted.band), from: fromOf(next), limit: stale ? Math.min(200, Math.max(LIMIT, wanted.to - wanted.from)) : LIMIT }).then(
      (answer) => {
        held.asking.delete(key);
        if (turn !== held.turn) return pump(key);
        crossed(answer.rows, since);
        const read = readOf(next, answer);
        const was = stale ? { runs: [], end: null } : (held.bands[key] ?? { runs: [], end: null });
        held.stale.delete(key);
        const now = held.wanted[key] ?? wanted;
        held.bands[key] = {
          runs: kept(keptRead(was.runs, next, answer), now.from, now.to),
          end: read.end !== null ? Math.min(read.end, was.end ?? read.end) : answer.rows.length === 0 && 'at' in next ? next.rank : was.end,
        };
        held.now.draw();
        pump(key);
      },
      (error: unknown) => {
        held.asking.delete(key);
        if (turn !== held.turn) return pump(key);
        // A place handed out for the view as it was: read it from where the reader is.
        if (error instanceof ApiError && error.status === 409) {
          held.bands[key] = { runs: [], end: null };
          return pump(key);
        }
        held.error = error instanceof Error ? error.message : String(error);
        held.now.draw();
      },
    );
  };

  /** A row of what was read that was heard of after the reading was asked for: it may be in it as it was, so it is asked about again. */
  const crossed = (rows: Read[], since: number) => {
    let more = false;
    for (const row of rows) {
      if ((held.lately.get(row.id) ?? 0) > since) {
        held.heard[row.id] = heardBoth(held.heard[row.id], 'updated');
        more = true;
      }
    }
    if (more) soonSee();
  };

  /** Asks how the view shows each id heard, and draws what that comes to. */
  const see = () => {
    const ids = Object.keys(held.heard);
    if (held.seeing || ids.length === 0 || !held.now.on) return;
    const heard = held.heard;
    held.heard = {};
    held.seeing = true;
    const { project, name, mark, asking } = held.now;
    rowsApi.seen(project, name, asked(null), ids).then(
      (answer) => {
        held.seeing = false;
        if (held.mark !== mark) return see();
        const runs = Object.fromEntries(Object.entries(held.bands).map(([key, band]) => [key, band.runs]));
        const by = (asking.says as { group?: string | null } | null)?.group ?? null;
        const came = seenOf(runs, held.live, answer.rows, heard, held.mine, asking.sort, by);
        for (const [key, band] of Object.entries(held.bands)) held.bands[key] = { ...band, runs: came.bands[key] ?? band.runs };
        for (const one of answer.rows) held.mine.delete(one.id);
        held.live = came.live;
        // What the reader took out of the view is taken out: what is drawn is read again.
        if (came.dropped.length > 0) return again(), see();
        // Read in where it comes after every row: a band that ended is no longer known to end there.
        if (came.appended.length > 0) for (const band of Object.values(held.bands)) band.end = null;
        count();
        held.now.draw();
        see();
      },
      () => {
        held.seeing = false;
      },
    );
  };
  const soonSee = useMemo(() => coalesce(() => see(), GATHER), [project, name]);

  /** Heard that a record of the table changed. */
  const hear = (id: string, kind: Heard) => {
    held.said += 1;
    held.lately.delete(id);
    held.lately.set(id, held.said);
    if (held.lately.size > 4 * MANY) held.lately.delete(held.lately.keys().next().value!);
    if (held.live.many) return;
    held.heard[id] = heardBoth(held.heard[id], kind);
    if (Object.keys(held.heard).length > MANY) {
      // Too many to ask about one by one: the reader is told so, and what is counted goes on.
      held.heard = {};
      held.live = { ...held.live, many: true };
      count();
      held.now.draw();
      return;
    }
    soonSee();
  };

  /** Reads the window of a band about a row, and has the drawing place that row (`placed`). */
  const about = (key: string, band: string | null, said: string, from: number | null): Promise<Answer['around'] | null> => {
    held.asking.add(key);
    const turn = held.turn;
    const { project, name } = held.now;
    return rowsApi.window(project, name, { ...asked(band), from: { around: said }, limit: LIMIT }).then(
      (answer) => {
        held.asking.delete(key);
        const around = answer.around;
        if (turn !== held.turn || !around?.in || answer.rank === undefined || around.rank === undefined) {
          pump(key);
          return around ?? null;
        }
        held.stale.delete(key);
        const next: Next = { at: answer.rank, rank: answer.rank };
        held.bands[key] = { runs: keptRead([], next, answer), end: answer.after === null ? answer.rank + answer.rows.length : null };
        held.placed = { key, id: around.id, from, to: around.rank };
        held.now.draw();
        pump(key);
        return around;
      },
      (error: unknown) => {
        held.asking.delete(key);
        pump(key);
        throw error;
      },
    );
  };

  // Another asking: what was read was of the one before.
  if (held.mark !== mark) {
    held.mark = mark;
    held.bands = {};
    held.wanted = {};
    held.stale.clear();
    held.turn += 1;
    held.error = null;
    held.live = NONE;
    held.heard = {};
    held.marked = null;
    if (on) count();
  }

  const again = (anchor?: { key: string; id: string; rank: number }) => {
    held.turn += 1;
    held.live = NONE;
    held.heard = {};
    for (const key of Object.keys(held.bands)) held.stale.add(key);
    held.counting = false;
    count();
    // About the row at the top of the screen, so that it stays where it was whatever came before it meanwhile.
    if (anchor) void about(anchor.key, held.wanted[anchor.key]?.band ?? null, anchor.id, anchor.rank).catch(() => {});
    for (const key of Object.keys(held.wanted)) if (key !== anchor?.key) pump(key);
  };
  const soon = useMemo(() => coalesce(() => held.now.on && again(), AGAIN), [project, name]);

  useEffect(() => {
    state.alive = true;
    // Shown again with what it had: drawn at once from that, and read again behind.
    if (on && held.counted) again();
    else if (on) count();
    return () => {
      state.alive = false;
    };
  }, [on]);

  useEvents(
    ['record.created', 'record.updated', 'record.removed', 'collection.ensured', 'collection.fields'],
    (event) => {
      if (!on || event.data?.project !== project || (event.data?.collection ?? event.data?.name) !== name) return;
      const id = event.data?.id;
      if (event.topic.startsWith('record.') && typeof id === 'string') hear(id, event.topic.slice('record.'.length) as Heard);
      // What the table is made of changed: read again whole.
      else soon();
    },
    [project, name, on, soon],
  );

  return {
    counted: held.mark === mark ? held.counted : null,
    bands: held.mark === mark ? held.bands : {},
    error: held.error,
    want: (band, from, to) => {
      const key = keyOf(band);
      const was = held.wanted[key];
      if (was && was.from === from && was.to === to) return pump(key);
      held.wanted[key] = { band, from, to };
      pump(key);
    },
    again,
    live: held.mark === mark ? held.live : NONE,
    mine: (id) => held.mine.add(id),
    // Read as it is when it is asked: what draws the table is drawn again by itself, with this object as it was handed.
    get placed() {
      return held.placed;
    },
    took: () => {
      held.placed = null;
    },
    marked: held.marked,
    goTo: async (said) => {
      const by = (held.now.asking.says as { group?: string | null } | null)?.group ?? null;
      try {
        // Of a view gathered by a column, the row is read about in the band it is in, which reading it first says.
        let band: string | null = null;
        if (by) {
          const first = await rowsApi.window(held.now.project, held.now.name, { ...asked(null), from: { around: said }, limit: 1 });
          const row = first.around?.row;
          if (!first.around?.in) return { in: false, id: first.around?.id ?? said, out: first.around?.kept_out ?? [], row: row ?? null };
          band = bandOfRow(row?.data[by]);
        }
        const found = await about(keyOf(band), band, said, null);
        if (!found) return { missing: said };
        if (!found.in) return { in: false, id: found.id, out: found.kept_out ?? [], row: found.row ?? null };
        held.marked = found.id;
        held.now.draw();
        return { in: true, id: found.id };
      } catch (error) {
        return { missing: error instanceof Error ? error.message : String(error) };
      }
    },
  };
}

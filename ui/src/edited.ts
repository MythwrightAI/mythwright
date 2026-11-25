// When something was last changed, and by whom, as far as the page knows.
//
// Nothing of the page is here. A table's last change is the later of two
// things the page has already read: when the table itself was last
// written (declared again, a view saved, an icon chosen) and when the
// newest of its records was. Who made it is a line of that record's
// history, where its table keeps one and a line was written as the change
// was made: a change that wrote no line (a table with no history, a field
// its table says is no news, a view saved) is nobody's to name, and
// neither is one another program made to a file with nobody looking.
//
// A line never says when: it is heard a moment after what it is about,
// and one written late, for something that happened long before (a
// history filled in when its table first asks for one), is heard now.
// So a line names whoever made the change only where it is of the same
// moment as the change the page read.
//
// What this cannot see is a record that was taken away: nothing that is
// still there says so. Whatever shows this says that of it.

import { when } from './util.ts';

export interface Edited {
  /** When, in milliseconds. */
  at: number;
  /** Who, by name; nothing where nobody is known to have. */
  by: string | null;
}

/** A line of something's history, as far as this goes by it: who it says did it, and when. */
export interface Last {
  who: string;
  ts: number;
}

/**
 * How far apart a change and the line written of it may be and still be
 * one moment: the line is written from the event, a moment after the
 * record, and a file's time is the disk's and not the journal's.
 */
export const SLACK = 5000;

/** The record changed last, where there is any. */
export function newest<R extends { updated: number }>(rows: R[]): R | null {
  return rows.reduce<R | null>((latest, row) => (latest === null || row.updated > latest.updated ? row : latest), null);
}

/** The last line of a history as it was read, oldest first. */
export function lastOf(lines: { who: string; ts: number }[]): Last | null {
  const last = lines[lines.length - 1];
  return last ? { who: last.who, ts: last.ts } : null;
}

/**
 * When something last changed and who changed it: `times` are the times
 * the page read of it (its own, and its newest record's), `lines` the
 * lines it knows of (the last one read, the last one heard). Nothing
 * where nothing says when.
 */
export function edited(times: (number | null | undefined)[], lines: (Last | null)[]): Edited | null {
  const at = Math.max(0, ...times.filter((one): one is number => typeof one === 'number'));
  if (at <= 0) return null;
  const of = lines
    .filter((line): line is Last => line !== null && line.who.trim() !== '' && Math.abs(line.ts - at) <= SLACK)
    .sort((a, b) => b.ts - a.ts)[0];
  return { at, by: of?.who ?? null };
}

/** How it is said: short for a page's head, and whole for wherever there is room. */
export function saidOf(one: Edited, now = Date.now()): { short: string; whole: string } {
  const at = when(one.at, now);
  return { short: `Edited ${at}`, whole: one.by ? `Last edited by ${one.by} · ${at}` : `Last edited ${at}` };
}

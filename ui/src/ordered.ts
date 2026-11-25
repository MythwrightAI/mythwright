// Records in the order they can be done, and gathered by what they name.
//
// A record may say which others of its table come before it (a card's
// `after`). Laid out by that, a table is columns: what waits on nothing
// that is still to do, what waits only on those, and so on, each column
// holding what can be done side by side. This is the part with no page in
// it: which column a record is in, what it waits on and what waits on it,
// which wait in a ring and never come round, and what is said of one that
// waits on something no longer there. And, apart from that, records in
// bands by what one of their fields names. Nothing here knows what a
// record is of.

import type { Field, Named, Row } from './table.ts';
import { idsOf, named, said } from './table.ts';

/** What is asked of a record to put it in order: what it waits on, and whether it is done. */
export interface Waits {
  id: string;
  after: string[];
  done: boolean;
}

/** A table's records by what comes first. */
export interface Ordered {
  /** Those that are done, in the order they were given: they hold nothing up. */
  done: string[];
  /** The rest, a column at a time: the first can be begun now, each next one waits only on those before it. */
  columns: string[][];
  /** Those that never come round: each is in a ring of things waiting on one another, or waits on one that is. */
  ring: string[];
  /** Which of `ring` are in a ring themselves, and not only behind one. */
  turning: string[];
  /** What each waits on that is there, done or not. */
  after: Record<string, string[]>;
  /** What waits on each. */
  before: Record<string, string[]>;
  /** What each waits on that is not there: taken away, or never was. It holds nothing up, and is said. */
  missing: Record<string, string[]>;
}

/**
 * Puts records in the order they can be done. One that is done holds
 * nothing up, wherever it is named; one that waits on something that is
 * not there is placed by the rest of what it waits on; and what waits in
 * a ring is set apart, since no column is its own.
 */
export function ordered(rows: Waits[]): Ordered {
  const known = new Map(rows.map((row) => [row.id, row]));
  const after: Record<string, string[]> = {};
  const before: Record<string, string[]> = {};
  const missing: Record<string, string[]> = {};
  for (const row of rows) before[row.id] = [];
  for (const row of rows) {
    const named = [...new Set(row.after)];
    after[row.id] = named.filter((id) => known.has(id));
    missing[row.id] = named.filter((id) => !known.has(id));
    for (const id of after[row.id]!) before[id]!.push(row.id);
  }
  const done = rows.filter((row) => row.done).map((row) => row.id);
  const open = rows.filter((row) => !row.done);
  // What still holds each up: what it waits on that is there and not done.
  const holds = new Map(open.map((row) => [row.id, after[row.id]!.filter((id) => !known.get(id)!.done)]));
  const column = new Map<string, number>();
  for (let left = open.filter((row) => !column.has(row.id)); left.length > 0; ) {
    const next = left.filter((row) => holds.get(row.id)!.every((id) => column.has(id)));
    if (next.length === 0) break;
    // Placed together, so that nothing lands beside what it waits on.
    const at = next.map((row) => Math.max(-1, ...holds.get(row.id)!.map((id) => column.get(id)!)) + 1);
    next.forEach((row, nth) => column.set(row.id, at[nth]!));
    left = left.filter((row) => !column.has(row.id));
  }
  const columns: string[][] = [];
  for (const row of open) {
    const at = column.get(row.id);
    if (at !== undefined) (columns[at] ??= []).push(row.id);
  }
  const ring = open.filter((row) => !column.has(row.id)).map((row) => row.id);
  const stuck = new Set(ring);
  // In a ring itself: following what it waits on comes back to it.
  const comesBack = (id: string) => {
    const seen = new Set<string>();
    for (const todo = [...holds.get(id)!]; todo.length > 0; ) {
      const at = todo.pop()!;
      if (at === id) return true;
      if (seen.has(at) || !stuck.has(at)) continue;
      seen.add(at);
      todo.push(...holds.get(at)!);
    }
    return false;
  };
  return { done, columns: Array.from(columns, (one) => one ?? []), ring, turning: ring.filter(comesBack), after, before, missing };
}

/**
 * What a record's choice says of whether it is done: done at the last of
 * what can be chosen, begun anywhere past the first. A record at the first,
 * or with nothing chosen, is one nobody has begun.
 */
export function standing(field: Field | undefined, value: unknown): { done: boolean; begun: boolean; at: string } {
  const at = said(value);
  const nth = field ? field.of.indexOf(at) : -1;
  const last = (field?.of.length ?? 0) - 1;
  return { done: nth !== -1 && last > 0 && nth === last, begun: nth > 0 && nth < last, at };
}

/** What a column of the order is headed, by its place. */
export function columnName(nth: number): string {
  return ['Can begin now', 'Then', 'After that'][nth] ?? `Step ${nth + 1}`;
}

/** Records gathered under one thing a field of theirs names. */
export interface Band {
  /** What the field holds: nothing, for the band of those that name nothing. */
  key: string;
  name: string;
  /** What it names is no longer there. */
  gone: boolean;
  rows: Row[];
}

/** The band of records that name nothing is called this. */
export const UNGROUPED = 'Ungrouped';

/** Whether a field holds several things at once: a list of words, or several records. */
const several = (field: Field) => field.kind === 'list' || field.kind === 'records';

/**
 * The things a record holds in a field, as what each is: the one thing a
 * field of one says, each of what a field of several holds, and none for
 * a record that says nothing there.
 */
export function holds(row: Row, field: Field): string[] {
  const all = several(field) ? idsOf(row.data[field.name]) : [said(row.data[field.name])];
  return [...new Set(all.map((one) => one.trim()).filter((one) => one !== ''))];
}

/**
 * A table's records in bands by one of their fields: a band for each
 * thing it names, called what that is known by, and last a band of those
 * that name nothing, where there are any. A record that names another
 * table's is called by that record (`others`); a text that is the id of a
 * record somebody looked up is called what that one is (`called`); a
 * choice comes in the order of what can be chosen, and anything else by
 * name, so that a band stays where it was. A record that holds several
 * things (its labels) is in the band of each: it is one record, shown
 * once for each thing it holds. Bands of the same name stay apart: two
 * things called alike are still two.
 */
export function bands(rows: Row[], field: Field, others: Named, called: Record<string, string> = {}): Band[] {
  const found = new Map<string, Row[]>();
  for (const row of rows) {
    const keys = holds(row, field);
    for (const key of keys.length > 0 ? keys : ['']) found.set(key, [...(found.get(key) ?? []), row]);
  }
  const names = field.kind === 'record' || field.kind === 'records';
  const of = others[field.of[0] ?? ''];
  const place = (key: string): [number, string] => {
    if (field.kind === 'choice') return [(field.of.indexOf(key) + field.of.length + 1) % (field.of.length + 1), key];
    const at = names && of ? of.rows.findIndex((row) => row.id === key) : -1;
    return [at === -1 ? Number.MAX_SAFE_INTEGER : (of!.rows[at]!.number ?? at), key];
  };
  const some = [...found.keys()]
    .filter((key) => key !== '')
    .sort((a, b) => place(a)[0] - place(b)[0] || a.localeCompare(b))
    .map((key): Band => ({ key, ...bandCalled(key, field, others, called), rows: found.get(key)! }));
  const none = found.get('');
  return none ? [...some, { key: '', name: UNGROUPED, gone: false, rows: none }] : some;
}

/**
 * What the band of the rows that hold `key` in `field` is called, and
 * whether what it names is no longer there: a record by what it is known
 * by, somebody by their name, a text that is the id of a record somebody
 * looked up by what that one is called (`called`), anything else as it
 * is; and the band of those that hold nothing `Ungrouped`. A page that
 * is told a view's bands by the daemon calls each so (`views/windowed.tsx`).
 */
export function bandCalled(key: string, field: Field, others: Named, called: Record<string, string> = {}): { name: string; gone: boolean } {
  if (key === '') return { name: UNGROUPED, gone: false };
  if (field.kind === 'record' || field.kind === 'records') {
    const what = named(field, key, others);
    if (what) return { name: what.words, gone: what.gone };
  }
  return { name: field.kind === 'actor' ? (key.split(':')[1] ?? key) : (called[key] ?? key), gone: false };
}

/** Whether two of the rows hold the same thing in a field: then it names something, and is not words of a record's own. */
function shared(field: Field, rows: Row[]): boolean {
  const seen = new Set<string>();
  for (const row of rows) {
    for (const thing of holds(row, field)) {
      if (seen.has(thing)) return true;
      seen.add(thing);
    }
  }
  return false;
}

/**
 * The fields a view of this table can gather its records by, as its
 * layout offers them: one that names a record of another table, a choice
 * other than the one its columns come from, or who; a text or a list
 * that two of the `rows` share a thing of, which is one that names
 * something and not words of a record's own; and whatever it is gathered
 * by `now`. What a table keeps for itself and leaves out of a view is
 * not offered.
 */
export function groupable(fields: Field[], table: string, columnsBy: string | null, now: string | null = null, rows: Row[] = []): Field[] {
  return fields.filter(
    (field) =>
      field.name === now ||
      (!field.hidden &&
        ((field.kind === 'record' && field.of[0] !== table) ||
          (field.kind === 'choice' && field.name !== columnsBy) ||
          field.kind === 'actor' ||
          ((field.kind === 'text' || several(field)) && shared(field, rows)))),
  );
}

/** What a view is held to: for each field, the things a record is to hold one of there. */
export type Only = Record<string, string[]>;

/**
 * The records a view shows, where it is held to some: those that, in
 * each field it names, hold one of the things said. A field the table no
 * longer has holds nothing back, and a view held to nothing shows all.
 */
export function shown(rows: Row[], only: Only | null | undefined, fields: Field[]): Row[] {
  const by = Object.entries(only ?? {}).flatMap(([name, any]) => {
    const field = fields.find((one) => one.name === name);
    return field && any.length > 0 ? [{ field, any }] : [];
  });
  if (by.length === 0) return rows;
  return rows.filter((row) => by.every(({ field, any }) => holds(row, field).some((thing) => any.includes(thing))));
}

/** A field a view can be held by, with the things there are to hold it to. */
export interface Holding {
  field: Field;
  things: string[];
}

/**
 * What a view of this table can be held to, as its layout offers it: for
 * each field that holds several things, two records sharing one of them
 * (so that it names something), the things any of the `rows` holds
 * there, for each choice other than the one its columns come from
 * what can be chosen, and whatever the view is held to `now` whether or
 * not a record still holds it. In the order of what can be chosen for a choice, and by name
 * for anything else. A field nothing is held in offers nothing.
 */
export function holdable(fields: Field[], columnsBy: string | null, rows: Row[], now: Only | null | undefined = null): Holding[] {
  return fields
    .filter((field) => !field.hidden && ((several(field) && (shared(field, rows) || (now?.[field.name]?.length ?? 0) > 0)) || (field.kind === 'choice' && field.name !== columnsBy)))
    .map((field) => {
      // A choice offers what can be chosen, held by a record yet or not; a list only what some record holds.
      const there = new Set([...(field.kind === 'choice' ? field.of : []), ...rows.flatMap((row) => holds(row, field)), ...(now?.[field.name] ?? [])]);
      const at = (thing: string) => (field.kind === 'choice' ? (field.of.indexOf(thing) + field.of.length + 1) % (field.of.length + 1) : 0);
      return { field, things: [...there].sort((a, b) => at(a) - at(b) || a.localeCompare(b)) };
    })
    .filter((one) => one.things.length > 0);
}

/**
 * What a view can be held to by one column, as the column's own menu
 * offers it: for a choice what can be chosen, and for anything else that
 * names something (a record or several, who, a text, a list) the things
 * the `rows` hold there, where two of them share one; with whatever the
 * view is held to there `now`. No things, where every row says something
 * of its own or nothing: a record's own words are not held to. `null`
 * for a column a view is never held by: a number, a day, a tick.
 */
export function holdableBy(field: Field, rows: Row[], now: Only | null | undefined = null): Holding | null {
  if (!['record', 'records', 'choice', 'actor', 'text', 'list'].includes(field.kind)) return null;
  const held = now?.[field.name] ?? [];
  const some = field.kind === 'choice' || held.length > 0 || shared(field, rows);
  const there = new Set(some ? [...(field.kind === 'choice' ? field.of : []), ...rows.flatMap((row) => holds(row, field)), ...held] : []);
  const at = (thing: string) => (field.kind === 'choice' ? (field.of.indexOf(thing) + field.of.length + 1) % (field.of.length + 1) : 0);
  return { field, things: [...there].sort((a, b) => at(a) - at(b) || a.localeCompare(b, undefined, { numeric: true })) };
}

/** What a view is held to once a thing is turned on or off in a field: nothing at all, when nothing is left. */
export function holding(only: Only | null | undefined, field: string, thing: string, on: boolean): Only | null {
  const rest = (only?.[field] ?? []).filter((one) => one !== thing);
  const next: Only = { ...only, [field]: on ? [...rest, thing] : rest };
  if (next[field]!.length === 0) delete next[field];
  return Object.keys(next).length > 0 ? next : null;
}

/** What a view is held to, in words: `labels before-preview or landed`, and between fields `and`. */
export function heldTo(only: Only | null | undefined, fields: Field[]): string {
  return heldBy(only, fields)
    .map(({ words, any }) => `${words} ${any.join(' or ')}`)
    .join(', and ');
}

/** The same a part at a time, for whoever draws it: each field a view is held by, what it is called in a sentence, and the things. */
export function heldBy(only: Only | null | undefined, fields: Field[]): { field: Field; words: string; any: string[] }[] {
  return Object.entries(only ?? {}).flatMap(([name, any]) => {
    const field = fields.find((one) => one.name === name);
    return field && any.length > 0 ? [{ field, words: name.replace(/[-_]/g, ' '), any }] : [];
  });
}

// The columns of a table as a view has them: carried to another place,
// frozen where the table is scrolled sideways, and put in order by what
// one of them says.
//
// A view says the order its columns stand in (`columns`), which of them
// it is frozen up to (`freezes`) and a figure at the foot of some
// (`figures`, in `figures.ts`). Which columns stand where is `banded` in
// `groups.ts`; this is what is worked out from that: the order a view is
// saved with once a column is carried, where a carried column would
// land, and which columns stay while the rest scroll. And one thing that
// is the reader's own and no view's: the column they sorted by, kept in
// their browser. Nothing here knows the page.

import { banded } from './groups.ts';
import type { Change, Field, Group, Sort } from './table.ts';
import { columns, shownIn, titleField } from './table.ts';

/** A table's columns as a view has them: every field of the table as columns stand when nothing says otherwise, its groups, and the order the view says. */
export interface Standing {
  fields: Field[];
  groups?: Group[] | null;
  order?: string[] | null;
}

/**
 * The order a view is saved with once the column `name` stands before
 * the column `before`, or at the end of its band where none is said:
 * every column of the table by name as they then stand, band by band,
 * those the view leaves out among them, so that one shown again is where
 * it was. Nothing, where it may not go there or is there already: what a
 * record is known by is not carried and nothing is put before it, and a
 * column stays in its band.
 */
export function carried(table: Standing, name: string, before: string | null): string[] | null {
  const known = titleField(table.fields);
  const bands = banded(table.fields, table.groups, table.order);
  const band = bands.find((one) => one.fields.some((field) => field.name === name));
  if (!band || name === known?.name || before === name || before === known?.name) return null;
  if (before !== null && !band.fields.some((field) => field.name === before)) return null;
  const names = band.fields.map((field) => field.name);
  const rest = names.filter((one) => one !== name);
  const to = before === null ? rest.length : rest.indexOf(before);
  const next = [...rest.slice(0, to), name, ...rest.slice(to)];
  if (next.every((one, at) => one === names[at])) return null;
  return bands.flatMap((one) => (one === band ? next : one.fields.map((field) => field.name)));
}

/**
 * The column `name` would stand before once it is moved one place among
 * the columns that are `shown`, to the left (`by` -1) or to the right
 * (1): the name of a column, `null` for the end of its band, and
 * `undefined` where there is no such place, at an end of its band or
 * beside what a record is known by.
 */
export function stepped(table: Standing, shown: string[], name: string, by: -1 | 1): string | null | undefined {
  const known = titleField(table.fields);
  const band = banded(table.fields, table.groups, table.order).find((one) => one.fields.some((field) => field.name === name));
  if (!band || name === known?.name) return undefined;
  const among = band.fields.map((field) => field.name).filter((one) => one !== known?.name && (one === name || shown.includes(one)));
  const at = among.indexOf(name);
  if (by === -1) return at > 0 ? among[at - 1] : undefined;
  if (at >= among.length - 1) return undefined;
  return among[at + 2] ?? null;
}

/** A head a carried column may be let go beside: its column, and where its edges are across the page. */
export interface Edge {
  name: string;
  left: number;
  right: number;
}

/**
 * Where a column carried to `x` would land among the heads it may stand
 * among (`heads`: those of its band but for what a record is known by,
 * itself among them, from left to right): the column it would stand
 * before, `null` for after the last, and where across the page the line
 * that says so is drawn. It lands before the head whose middle the
 * pointer has not passed.
 */
export function landing(heads: Edge[], x: number): { before: string | null; at: number } | null {
  if (heads.length === 0) return null;
  const next = heads.find((head) => x < (head.left + head.right) / 2);
  return next ? { before: next.name, at: next.left } : { before: null, at: heads[heads.length - 1]!.right };
}

/**
 * How many of the columns drawn, from the first, a view frozen up to
 * `freezes` keeps where they are: that column and those before it. None,
 * where it says none or names one that is not drawn.
 */
export function frozen(drawn: string[], freezes: string | null | undefined): number {
  return freezes ? drawn.indexOf(freezes) + 1 : 0;
}

/** The least room, in pixels, that the columns that stay leave for the rest to be scrolled in: about two columns at their narrowest. */
export const ROOM_LEFT = 240;

/**
 * How many of the frozen columns stay in a room this wide: as many, from
 * the first, as leave `ROOM_LEFT` for the rest to be scrolled in. Frozen
 * past that, a table would have nothing left to scroll. At a phone's
 * width that is none: what a record is known by takes more than the
 * screen less that much. `widths` are those of the columns drawn, in order.
 */
export function staying(widths: number[], count: number, room: number): number {
  let taken = 0;
  for (let nth = 0; nth < Math.min(count, widths.length); nth++) {
    taken += widths[nth]!;
    if (taken > room - ROOM_LEFT) return nth;
  }
  return Math.min(count, widths.length);
}

/** Where each of the first `count` columns begins across the table, when those before it stay: the widths before it, added up. */
export function lefts(widths: number[], count: number): number[] {
  const out: number[] = [];
  let at = 0;
  for (let nth = 0; nth < count; nth++) {
    out.push(at);
    at += widths[nth] ?? 0;
  }
  return out;
}

/**
 * What a reader's sort becomes when a line of a column's menu is pressed:
 * up or down by that column, and as the rows were added where the line
 * pressed is the one already in force.
 */
export function sortedBy(now: Sort | null, field: string, down: boolean): Sort | null {
  return now?.field === field && now.down === down ? null : { field, down };
}

/** The sort a reader left a view with, as their browser kept it; none where it kept none, or what it kept is no sort. */
export function sortKept(text: string | null): Sort | null {
  try {
    const read: unknown = JSON.parse(text ?? 'null');
    if (read && typeof read === 'object' && typeof (read as Sort).field === 'string') return { field: (read as Sort).field, down: (read as Sort).down === true };
  } catch {
    /* what was kept is not a sort */
  }
  return null;
}

/**
 * The change to a view that carries the column `name` to stand before
 * `before`, or to the end of its band, in a table that says these
 * groups: worked out from the view as it is when the change is made
 * (`inturn.ts`), and one that says nothing of its columns where the
 * column may not go there.
 */
export function carries(groups: Group[] | null | undefined, name: string, before: string | null): Change {
  return (view, fields) => {
    const next = carried({ fields: columns(fields), groups, order: view.columns }, name, before);
    return next ? { name: view.name, kind: view.kind, columns: next } : { name: view.name, kind: view.kind };
  };
}

/**
 * The change to a view that moves the column `name` one place: past the
 * column beside it that the view shows, or past the one beside it
 * whether it is shown or not (`every`: where every column is listed).
 */
export function steps(groups: Group[] | null | undefined, name: string, by: -1 | 1, every = false): Change {
  return (view, fields, views) => {
    const table = { fields: columns(fields), groups, order: view.columns };
    const among = (every ? table.fields : shownIn(view, table.fields)).map((field) => field.name);
    const before = stepped(table, among, name, by);
    return before === undefined ? { name: view.name, kind: view.kind } : carries(groups, name, before)(view, fields, views);
  };
}

/** What a column's menu can be asked: each line there may be, by a word. */
export type Line = 'filter' | 'unshared' | 'up' | 'down' | 'group' | 'figure' | 'freeze' | 'hide' | 'left' | 'right';

/**
 * The lines of a column's menu, in order. Sorting is whoever looks at
 * the table's own and is always there. The rest change the view, which
 * is the table's for everybody, and are there only for whoever `may`
 * save one: what the view is held to by this column (`holds`: there are
 * things to hold it to; or no two rows share a thing in it, and a line
 * says so and is nothing to press; or it is a column a view is `never`
 * held by, a number or a day, and there is no line), gathering by it
 * where a view can be, the figure at its foot, freezing up to it,
 * leaving it out, and moving it a place where there is one. What a
 * record is `known` by is never left out and never moved.
 */
export function lines(of: { may: boolean; known: boolean; holds: 'things' | 'unshared' | 'never'; gathers: boolean; left: boolean; right: boolean }): Line[] {
  const sort: Line[] = ['up', 'down'];
  if (!of.may) return sort;
  return [...(of.holds === 'never' ? [] : ([of.holds === 'things' ? 'filter' : 'unshared'] as Line[])), ...sort, ...(of.gathers ? (['group'] as Line[]) : []), 'figure', 'freeze', ...(of.known ? [] : (['hide'] as Line[])), ...(of.left ? (['left'] as Line[]) : []), ...(of.right ? (['right'] as Line[]) : [])];
}

/**
 * The columns a view can show or leave out, as its layout lists them: in
 * the order they stand, but for what a record is known by, each with
 * whether it is shown (`hides` are those left out) and whether it has a
 * place to be moved to among all of them, shown or not.
 */
export function listed(table: Standing, hides: string[]): { field: Field; shown: boolean; up: boolean; down: boolean }[] {
  const known = titleField(table.fields);
  const every = table.fields.map((field) => field.name);
  return banded(table.fields, table.groups, table.order)
    .flatMap((band) => band.fields)
    .filter((field) => field !== known)
    .map((field) => ({ field, shown: !hides.includes(field.name), up: stepped(table, every, field.name, -1) !== undefined, down: stepped(table, every, field.name, 1) !== undefined }));
}

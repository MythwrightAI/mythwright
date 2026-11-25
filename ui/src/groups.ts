// A table's columns in groups.
//
// A table may say its columns in groups (`groups`, where its blueprint
// declares it): headings in the order they are drawn, each with the
// fields under it in order. A table that says so is drawn with a band of
// those headings over its columns' heads and its columns kept together
// under them, and a row's form has a section under each. A field in no
// group is as it always was, before the groups, and so is what a record
// is known by, which is first whatever a group says of it. This holds
// nothing of the page, so that its tests can read it.

import { titleField } from './table.ts';
import type { Field, Group } from './table.ts';

/** Columns that are drawn together: under a heading, or under none. */
export interface Band {
  /** The heading; nothing, for the columns that are in no group. */
  name: string | null;
  fields: Field[];
}

/**
 * The columns `fields` (in the order a table has them when nothing says
 * otherwise) in their bands: first those in no group, as they stood,
 * then each group's in the order the group names them. A group none of
 * whose fields is among the columns (each is left out of the view, or
 * gone) is no band, and a field a group names that is not among them is
 * passed over. A table that says no groups is one band with no heading.
 *
 * A view may say the order its columns stand in (`order`: `columns` of
 * the view). That is the order inside each band and never across them:
 * the bands keep the table's order, and no view scatters a group. In a
 * band the columns the view names come first, as it names them, and the
 * rest after them as they stood; what a record is known by stays first
 * whatever is said, and a name that is no column is passed over.
 */
export function banded(fields: Field[], groups: Group[] | null | undefined, order?: string[] | null): Band[] {
  const known = titleField(fields);
  const named = new Set((groups ?? []).flatMap((group) => group.fields));
  const taken = new Set<string>();
  const bands: Band[] = [{ name: null, fields: fields.filter((field) => field === known || !named.has(field.name)) }];
  for (const group of groups ?? []) {
    const under = group.fields.flatMap((name) => {
      const field = fields.find((one) => one.name === name && one !== known && !taken.has(name));
      if (field) taken.add(name);
      return field ? [field] : [];
    });
    if (under.length > 0) bands.push({ name: group.name, fields: under });
  }
  return bands.filter((band) => band.fields.length > 0).map((band) => ({ ...band, fields: inOrder(band.fields, known, order) }));
}

/** Columns that stand together, in the order a view says: those it names first, as it names them, then the rest as they stood; `known` first. */
function inOrder(fields: Field[], known: Field | undefined, order: string[] | null | undefined): Field[] {
  if (!order || order.length === 0) return fields;
  const at = (field: Field) => (field === known ? -1 : order.includes(field.name) ? order.indexOf(field.name) : order.length);
  // The sort keeps those it cannot tell apart where they stood.
  return [...fields].sort((a, b) => at(a) - at(b));
}

/** Whether any of the bands has a heading: a table with none is drawn with no band over its heads. */
export function headed(bands: Band[]): boolean {
  return bands.some((band) => band.name !== null);
}

/** A cell of the band of headings: what it says, how many columns it is over, which column it begins at, and whether a group begins there. */
export interface Heading {
  name: string | null;
  span: number;
  from: number;
  begins: boolean;
}

/**
 * The band of headings over a table's heads, a cell for each band. Where
 * the first `stay` columns stay while the rest are scrolled sideways and
 * that edge falls inside a band, the band is two cells, each over its own
 * columns and each saying the heading: one stays and the other scrolls,
 * so a column is always under what it is headed by.
 */
export function headings(bands: Band[], stay = 0): Heading[] {
  const out: Heading[] = [];
  let from = 0;
  bands.forEach((band, nth) => {
    const span = band.fields.length;
    const cut = stay > from && stay < from + span ? stay - from : 0;
    if (cut > 0) out.push({ name: band.name, span: cut, from, begins: nth > 0 }, { name: band.name, span: span - cut, from: from + cut, begins: false });
    else out.push({ name: band.name, span, from, begins: nth > 0 });
    from += span;
  });
  return out;
}

// Mockups: a collection whose records are folders of a project that each
// hold a page. What their page in the client needs worked out, with
// nothing of the page in it: which collections are mockups, where one's
// files are, how variants are set beside what they vary, the widths one
// is looked at in, and which events may have changed them.

import type { Row } from './table.ts';

/** What of a collection says its records are mockups and where they are. */
export interface Kept {
  name: string;
  store: string;
  at?: string | null;
}

/** Whether a collection's records are mockups. */
export function isMockups(collection: Kept): boolean {
  return collection.store === 'mockups';
}

/** The folder of the project the mockups are in, from the top of the project; nothing for the project itself. */
export function folderOf(kept: Kept): string {
  if (kept.at === '.') return '';
  return kept.at ?? kept.name;
}

/** The folder a mockup is, within the folder they are all in. Its record says; failing that, what it is known by does. */
export function nameOf(row: Row): string {
  const said = row.data.path;
  if (typeof said === 'string' && said) return said;
  return row.id.slice(row.id.indexOf(':') + 1);
}

/** A file of a mockup, from the top of its project: its page, unless another is said. */
export function fileOf(kept: Kept, row: Row, file = 'index.html'): string {
  const folder = folderOf(kept);
  return [folder, nameOf(row), file].filter(Boolean).join('/');
}

/** What a mockup is called. */
export function titleOf(row: Row): string {
  const title = row.data.title;
  return typeof title === 'string' && title.trim() ? title : nameOf(row);
}

/** How far a mockup has got, where it says. */
export function statusOf(row: Row): string {
  return typeof row.data.status === 'string' ? row.data.status : '';
}

/** What a mockup is a variant of, by what that one is known by; nothing for one that is no variant. */
export function variantOf(row: Row): string | null {
  const of = row.data.of;
  return typeof of === 'string' && of ? of : null;
}

export interface Group {
  /** A mockup that is a variant of nothing here. */
  first: Row;
  /** The mockups that are variants of it, and variants of those, by what they are called. */
  variants: Row[];
}

/**
 * The mockups with each one's variants beside it. One that is a variant
 * of something that is not here (taken away, or never there) stands by
 * itself, so that nothing is left out of the gallery.
 */
export function groups(rows: Row[]): Group[] {
  const known = new Set(rows.map((row) => row.id));
  const by = (a: Row, b: Row) => titleOf(a).localeCompare(titleOf(b));
  // What a mockup is, in the end, a variant of: followed up as far as it
  // goes, and no further round than there are mockups.
  const root = (row: Row): string => {
    let at = row;
    for (let step = 0; step < rows.length; step += 1) {
      const of = variantOf(at);
      const next = of && of !== at.id && known.has(of) ? rows.find((one) => one.id === of) : undefined;
      if (!next) return at.id;
      at = next;
    }
    return row.id;
  };
  const firsts = rows.filter((row) => root(row) === row.id).sort(by);
  return firsts.map((first) => ({ first, variants: rows.filter((row) => row.id !== first.id && root(row) === first.id).sort(by) }));
}

/** A width a mockup is looked at in. */
export interface Width {
  name: 'phone' | 'tablet' | 'whole';
  label: string;
  /** In pixels; nothing for as wide as there is room. */
  across: number | null;
  says: string;
}

export const WIDTHS: Width[] = [
  { name: 'phone', label: 'Phone', across: 390, says: 'As wide as a phone held upright: 390' },
  { name: 'tablet', label: 'Tablet', across: 820, says: 'As wide as a tablet held upright: 820' },
  { name: 'whole', label: 'Whole', across: null, says: 'As wide as there is room for here' },
];

export function widthCalled(name: string | undefined): Width {
  return WIDTHS.find((width) => width.name === name) ?? WIDTHS[2]!;
}

/** Where a mockup is opened outside the client, by whoever is signed in. */
export function addressOf(id: string): string {
  return `/mockups/${encodeURIComponent(id)}`;
}

/** The events after which there may be other mockups than there were, or one may look otherwise. */
export const CHANGES = ['record.created', 'record.updated', 'record.removed', 'project.file.changed', 'project.rescanned', 'collection.ensured'];

/**
 * What an event may have changed: `list` when there may be other mockups
 * or other things kept of them, and the folder of the one whose files
 * changed, so that only it is drawn again.
 */
export function changes(event: { topic: string; data?: Record<string, unknown> }, project: string, kept: Kept): { list: boolean; drawn: string | null } {
  const nothing = { list: false, drawn: null };
  const data = event.data ?? {};
  if (data.project !== project) return nothing;
  if (event.topic.startsWith('record.')) return { list: data.collection === kept.name, drawn: null };
  if (event.topic === 'collection.ensured') return { list: data.name === kept.name, drawn: null };
  if (event.topic === 'project.rescanned') return { list: true, drawn: null };
  const path = typeof data.path === 'string' ? data.path : '';
  const folder = folderOf(kept);
  const within = folder === '' ? path : path.startsWith(`${folder}/`) ? path.slice(folder.length + 1) : null;
  if (within === null || !within.includes('/')) return nothing;
  return { list: true, drawn: within.slice(0, within.indexOf('/')) };
}

/** What a variant of a mockup is called until somebody calls it something. */
export function variantTitle(row: Row, taken: string[]): string {
  const base = `${titleOf(row)}, another way`;
  if (!taken.includes(base)) return base;
  for (let n = 2; n < 100; n += 1) if (!taken.includes(`${base} ${n}`)) return `${base} ${n}`;
  return base;
}

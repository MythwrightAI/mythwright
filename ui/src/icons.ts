// The icon a table's column is drawn with.
//
// A field may say an icon where its table is declared, and somebody who
// looks at the table may choose another, which stands over that. Both are
// the field's and are names: the daemon keeps the name and knows no set
// of drawings. Which drawing a name is, is said here, from the set the
// client was built with; a name that set has no drawing for is drawn as
// the mark of what kind of thing the field holds, and nothing fails.
//
// Nothing here reads or draws anything: these are functions of what was
// read, so that their tests can read them. What loads the set and asks
// the daemon is `iconset.ts`; what draws is `views/mark.tsx`, and what
// chooses, `views/iconpicker.tsx`.

import { covers } from './holds.ts';

/** One part of a drawing: what it is, and what it is drawn with. */
export type IconNode = [string, Record<string, string>];

/** The drawings there are: each by its name, with the words it is found by, in lowercase, a comma between them. */
export type IconSet = Record<string, [IconNode[], string]>;

/** What a field says of its icon: what its table declared, and what somebody chose. */
export interface Iconed {
  icon?: string | null;
  icon_chosen?: string | null;
}

/** The name of the icon a field is to be drawn with: what was chosen, else what was declared, else none. */
export function iconOf(field: Iconed): string | null {
  return field.icon_chosen || field.icon || null;
}

/**
 * The drawing a field has, or none where it is drawn as the mark of its
 * kind: where it names no icon, where the set has not been read yet, and
 * where the set has no drawing of that name. What was chosen stands over
 * what was declared whether or not it can be drawn.
 */
export function drawingOf(field: Iconed, set: IconSet | null): IconNode[] | null {
  const name = iconOf(field);
  if (!name || !set || !Object.hasOwn(set, name)) return null;
  return set[name]![0];
}

/** Whether any of a table's fields names an icon, which is what the set is read for. */
export function wantsIcons(fields: Iconed[]): boolean {
  return fields.some((field) => iconOf(field) !== null);
}

/**
 * The icons found by what was typed, by name: every word typed is in the
 * icon's name, or begins a word the icon is found by. The nearer its name
 * is to what was typed the sooner it comes: one that is called just that,
 * then those with every word typed a part of their name, those with each
 * beginning a part, those with each anywhere in the name, and last those
 * found by their words. Within each they are in the set's order. With
 * nothing typed, all of them.
 */
export function found(set: IconSet, typed: string): string[] {
  const words = typed
    .toLowerCase()
    .split(/[\s,]+/)
    .filter(Boolean);
  const names = Object.keys(set);
  if (words.length === 0) return names;
  const ranks: string[][] = [[], [], [], [], []];
  for (const name of names) {
    const parts = name.split('-');
    // A name is written with dashes, and typed with them, with spaces or with neither.
    const spelled = `${name} ${parts.join('')}`;
    // The words it is found by, each after a comma or a space: one typed begins a word and is not looked for inside one.
    const by = `,${set[name]![1]}`;
    const rank =
      words.join('-') === name || words.join('') === name
        ? 0
        : words.every((word) => parts.includes(word))
          ? 1
          : words.every((word) => parts.some((part) => part.startsWith(word)))
            ? 2
            : words.every((word) => spelled.includes(word))
              ? 3
              : words.every((word) => spelled.includes(word) || by.includes(`,${word}`) || by.includes(` ${word}`))
                ? 4
                : -1;
    if (rank >= 0) ranks[rank]!.push(name);
  }
  return ranks.flat();
}

/** What an icon is called to whoever reads it: `calendar-days` is `Calendar days`. */
export function calledIcon(name: string): string {
  return name.charAt(0).toUpperCase() + name.slice(1).replaceAll('-', ' ');
}

/**
 * Whether somebody who holds `held` may choose a column's icon in a
 * table: what saving a view of it takes, which is changing its records
 * (a board's cards), and whatever the table says changing them takes
 * beside that. The daemon asks the same and is what holds it; this is
 * only whether the choice is offered.
 */
export function mayChoose(held: string[], table: { name: string; takes?: string | null }): boolean {
  return covers(held, table.name === 'board' ? 'board:write' : 'records:write') && (!table.takes || covers(held, table.takes));
}

/**
 * Where the mark is after a key, among `count` icons laid out `columns`
 * to a row: an arrow moves it one icon or one row, and at an edge leaves
 * it, but that down from the last whole row goes to the last icon. Any
 * other key leaves it where it is.
 */
export function moved(marked: number, key: string, columns: number, count: number): number {
  if (count <= 0) return 0;
  const step = { ArrowLeft: -1, ArrowRight: 1, ArrowUp: -Math.max(1, columns), ArrowDown: Math.max(1, columns) }[key];
  if (step === undefined) return Math.min(marked, count - 1);
  const next = marked + step;
  // A row further than there is one leaves the mark where it is, but for the last row, which may be short.
  if (next < 0) return marked;
  if (next >= count) return key === 'ArrowDown' && Math.floor(marked / columns) < Math.floor((count - 1) / columns) ? count - 1 : marked;
  return next;
}

/**
 * Where a panel of a size is put so that it is under what opened it and
 * wholly in the window: under it, or over it where there is no room
 * under, and never past an edge by less than `room`.
 */
export function placed(opener: { left: number; top: number; bottom: number }, size: { width: number; height: number }, window: { width: number; height: number }, room = 8): { left: number; top: number } {
  const left = Math.max(room, Math.min(opener.left, window.width - size.width - room));
  const under = opener.bottom + 4;
  const over = opener.top - 4 - size.height;
  const top = under + size.height + room <= window.height ? under : over >= room ? over : Math.max(room, window.height - size.height - room);
  return { left, top };
}

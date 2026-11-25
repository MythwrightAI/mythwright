// A line of a tree carried to another place in it, as plain values: what a
// carry measures of the lines, where the pointer lands one, and where a
// step by the keyboard puts one. It knows no outline, project or topic: a
// line is a key, whether it can hold other lines (a "folder"), and the
// folder it is directly in, so that a project's outline
// (`views/outlinedrag.tsx`) and a wiki's pages are carried by
// the same rules. What carries is `views/treedrag.tsx`.
//
// Holds nothing of the page, so that its tests can read it.

/** Where something is put: into a folder (or the top), before a line of that folder (or at its end). */
export interface At {
  into?: string | null;
  before?: string | null;
}

/**
 * A line of a tree as a carry measures it, once, as the carry begins:
 * what it is (its key), the folder it is directly in (nothing for the
 * top), whether it can hold lines and is unfolded, and its boxes on the
 * page: its own line (`top`, `bottom`, `left`) and the end of everything
 * drawn under it (`end`). In the order the lines are drawn, top to bottom.
 */
export interface Row {
  node: string;
  into: string | null;
  folder: boolean;
  open: boolean;
  top: number;
  bottom: number;
  end: number;
  left: number;
}

/**
 * Where something carried lands: into a folder (or the top) and before a
 * line there (or at its end). `stays` where that is where it is, so that
 * nobody is told. What says so on the page: a line between two lines
 * (`line`, where across and down the page), or a folder outlined, landed
 * in at its end (`folder`).
 */
export interface Landing {
  into: string | null;
  before: string | null;
  stays: boolean;
  line: { top: number; left: number } | null;
  folder: string | null;
}

/**
 * Where the line `carried` lands with the pointer at `x`, `y`, among
 * the lines of its tree (`rows`): nowhere outside them. Over
 * the upper half of a line, before it; over the lower half, after it,
 * and, where it is the last of its folder and the pointer is further left
 * than it, after that folder, as far out as the pointer is; over the
 * middle of a folder's line (and the lower part of an unfolded one with
 * nothing drawn in it), into that folder at its end, which is not
 * unfolded under the pointer; over the lower part of an unfolded
 * folder's line, into it before what is first in it, and of a folded
 * one, after it. Never into itself
 * or what is under it: there, it stays where it is.
 */
export function landing(rows: Row[], carried: string, x: number, y: number): Landing | null {
  const first = rows[0];
  const it = rows.find((row) => row.node === carried);
  if (!first || !it) return null;
  const end = Math.max(...rows.map((row) => row.end));
  if (y < first.top || y >= end) return null;
  const byNode = new Map(rows.map((row) => [row.node, row]));
  const under = (row: Row): boolean => {
    for (let at: Row | undefined = row; at; at = at.into === null ? undefined : byNode.get(at.into)) if (at.node === carried) return true;
    return false;
  };
  const next = (row: Row): Row | null => rows.slice(rows.indexOf(row) + 1).find((one) => one.into === row.into) ?? null;
  const firstIn = (folder: Row): Row | null => rows.find((one) => one.into === folder.node) ?? null;
  const stayed = (into: string | null, before: string | null) => into === it.into && (before === it.node || before === (next(it)?.node ?? null));
  const landed = (into: string | null, before: string | null, line: Landing['line'], folder: string | null = null): Landing =>
    stayed(into, before) ? { into: it.into, before: next(it)?.node ?? null, stays: true, line: null, folder: null } : { into, before, stays: false, line, folder };
  const over = [...rows].reverse().find((row) => row.top <= y) ?? first;
  if (under(over)) return { into: it.into, before: next(it)?.node ?? null, stays: true, line: null, folder: null };
  const tall = over.bottom - over.top;
  const onLine = y < over.bottom;
  if (over.folder && onLine) {
    const inner = firstIn(over);
    const middle = y >= over.top + tall / 4 && (y < over.bottom - tall / 4 || (over.open && !inner));
    if (middle) return landed(over.node, null, null, over.node);
    if (y >= over.bottom - tall / 4 && inner) return landed(over.node, inner.node, { top: inner.top, left: inner.left });
  } else if (over.folder && over.open && !firstIn(over)) {
    // Below an unfolded folder's line with nothing drawn in it: into it.
    return landed(over.node, null, null, over.node);
  }
  if (onLine && y < over.top + tall / 2) return landed(over.into, over.node, { top: over.top, left: over.left });
  // After it, or after a folder it is the last of, as far out as the pointer is.
  let after = over;
  while (after.into !== null && !next(after) && x < after.left) {
    const out = byNode.get(after.into);
    if (!out || under(out)) break;
    after = out;
  }
  const then = next(after);
  return landed(after.into, then?.node ?? null, { top: then ? then.top : after.end, left: after.left });
}

/** A step of a line by the keyboard: up or down among its level, into the folder above it, or out of its folder. */
export type Step = 'up' | 'down' | 'in' | 'out';

/** A tree as it is laid out: a line by its key, or a folder by its id with what is drawn in it (a project's `Laid` is one). */
export type Tree = { thing: string } | { folder: string; items: Tree[] };

/**
 * Where a step by the keyboard puts `node`, from the tree as it is laid
 * out: before the line above it, after the line below it, at the end of
 * the folder directly above it, or after the folder it is in; nothing
 * where there is nowhere to go that way.
 */
export function stepped(laid: Tree[], node: string, step: Step): At | null {
  const named = (one: Tree) => ('folder' in one ? one.folder : one.thing);
  const find = (items: Tree[], into: string | null, outer: { items: Tree[]; into: string | null } | null): { items: Tree[]; at: number; into: string | null; outer: typeof outer } | null => {
    const at = items.findIndex((one) => named(one) === node);
    if (at >= 0) return { items, at, into, outer };
    for (const one of items) {
      if (!('folder' in one)) continue;
      const found = find(one.items, one.folder, { items, into });
      if (found) return found;
    }
    return null;
  };
  const found = find(laid, null, null);
  if (!found) return null;
  const { items, at, into, outer } = found;
  const above = items[at - 1];
  if (step === 'up') return above ? { into, before: named(above) } : null;
  if (step === 'down') {
    if (!items[at + 1]) return null;
    const then = items[at + 2];
    return { into, before: then ? named(then) : null };
  }
  if (step === 'in') return above && 'folder' in above ? { into: above.folder, before: null } : null;
  if (into === null || !outer) return null;
  const was = outer.items.findIndex((one) => named(one) === into);
  const then = outer.items[was + 1];
  return { into: outer.into, before: then ? named(then) : null };
}

/**
 * The same step, from the lines as a carry measures them (`rows`) where
 * whoever carries has no tree laid out to hand: what is folded away is not
 * drawn, and a step goes only among the lines of its own level and of the
 * level its folder is in, which are drawn while it is, so it comes to
 * what `stepped` comes to.
 */
export function steppedRows(rows: Row[], node: string, step: Step): At | null {
  const it = rows.find((row) => row.node === node);
  if (!it) return null;
  const level = (into: string | null) => rows.filter((row) => row.into === into);
  const items = level(it.into);
  const at = items.indexOf(it);
  const above = items[at - 1];
  if (step === 'up') return above ? { into: it.into, before: above.node } : null;
  if (step === 'down') {
    if (!items[at + 1]) return null;
    return { into: it.into, before: items[at + 2]?.node ?? null };
  }
  if (step === 'in') return above?.folder ? { into: above.node, before: null } : null;
  const outer = it.into === null ? undefined : rows.find((row) => row.node === it.into);
  if (!outer) return null;
  const around = level(outer.into);
  return { into: outer.into, before: around[around.indexOf(outer) + 1]?.node ?? null };
}

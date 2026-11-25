// How open tabs are laid out: in groups, side by side or stacked.
//
// The layout is a tree. A leaf is a group: a row of tabs, one of them
// showing. Anything else is a split: two layouts, beside each other or one
// above the other, sharing the space. Dragging a tab to the edge of a group
// splits that group; closing a group's last tab takes the group away and
// gives its space back. Everything here returns a new layout and changes
// nothing, so it can be tested without a screen.

import { reorder } from './util.ts';

export interface Group {
  kind: 'group';
  id: string;
  /** Tab keys, in the order they are shown. */
  tabs: string[];
  /** The tab that is showing. */
  active: string | null;
}

export interface Split {
  kind: 'split';
  /** `row`: side by side. `column`: one above the other. */
  dir: 'row' | 'column';
  first: Layout;
  second: Layout;
  /** How much of the space the first takes, from 0 to 1. */
  share: number;
}

export type Layout = Group | Split;

/** Where on a group a tab is dropped: among its tabs, or at one of its edges. */
export type Zone = 'center' | 'left' | 'right' | 'top' | 'bottom';

export function group(id: string, tabs: string[] = [], active: string | null = tabs[0] ?? null): Group {
  return { kind: 'group', id, tabs, active };
}

export function groups(layout: Layout): Group[] {
  return layout.kind === 'group' ? [layout] : [...groups(layout.first), ...groups(layout.second)];
}

/** The group a tab is in. */
export function holding(layout: Layout, key: string): Group | null {
  return groups(layout).find((entry) => entry.tabs.includes(key)) ?? null;
}

function each(layout: Layout, change: (entry: Group) => Group): Layout {
  if (layout.kind === 'group') return change(layout);
  return { ...layout, first: each(layout.first, change), second: each(layout.second, change) };
}

/** Takes away groups with no tabs, giving their space to what was beside them. One group always remains. */
function tidy(layout: Layout): Layout {
  if (layout.kind === 'group') return layout;
  const first = tidy(layout.first);
  const second = tidy(layout.second);
  const empty = (side: Layout) => side.kind === 'group' && side.tabs.length === 0;
  if (empty(first)) return second;
  if (empty(second)) return first;
  return { ...layout, first, second };
}

/** Shows a tab: in the group it is already in, or else added to group `into`. */
export function show(layout: Layout, key: string, into: string): Layout {
  const where = holding(layout, key)?.id ?? (groups(layout).some((entry) => entry.id === into) ? into : groups(layout)[0]!.id);
  return each(layout, (entry) => {
    if (entry.id !== where) return entry;
    return { ...entry, tabs: entry.tabs.includes(key) ? entry.tabs : [...entry.tabs, key], active: key };
  });
}

/** Closes a tab. The tab beside it shows instead, and an emptied group goes. */
export function close(layout: Layout, key: string): Layout {
  return tidy(
    each(layout, (entry) => {
      const at = entry.tabs.indexOf(key);
      if (at < 0) return entry;
      const tabs = entry.tabs.filter((tab) => tab !== key);
      const active = entry.active === key ? (tabs[Math.min(at, tabs.length - 1)] ?? null) : entry.active;
      return { ...entry, tabs, active };
    }),
  );
}

/** Moves a tab into group `to`, before tab `before` or at the end, and shows it there. */
export function move(layout: Layout, key: string, to: string, before: string | null): Layout {
  if (key === before || !groups(layout).some((entry) => entry.id === to)) return layout;
  const taken = each(layout, (entry) => {
    if (!entry.tabs.includes(key) || entry.id === to) return entry;
    const at = entry.tabs.indexOf(key);
    const tabs = entry.tabs.filter((tab) => tab !== key);
    return { ...entry, tabs, active: entry.active === key ? (tabs[Math.min(at, tabs.length - 1)] ?? null) : entry.active };
  });
  const placed = each(taken, (entry) => {
    if (entry.id !== to) return entry;
    const tabs = reorder(entry.tabs.includes(key) ? entry.tabs : [...entry.tabs, key], key, before);
    return { ...entry, tabs, active: key };
  });
  return tidy(placed);
}

/**
 * Drops a tab on a group. In the middle it joins that group's tabs; at an
 * edge the group is split and the tab gets the new half, a group called
 * `fresh`. A group's only tab dropped on its own edge has nowhere to go.
 */
export function dock(layout: Layout, key: string, onto: string, zone: Zone, fresh: string): Layout {
  if (zone === 'center') return move(layout, key, onto, null);
  const target = groups(layout).find((entry) => entry.id === onto);
  if (!target || (target.tabs.length === 1 && target.tabs[0] === key)) return layout;
  const without = each(layout, (entry) => {
    if (!entry.tabs.includes(key)) return entry;
    const at = entry.tabs.indexOf(key);
    const tabs = entry.tabs.filter((tab) => tab !== key);
    return { ...entry, tabs, active: entry.active === key ? (tabs[Math.min(at, tabs.length - 1)] ?? null) : entry.active };
  });
  const made = group(fresh, [key]);
  const split = (entry: Layout): Layout => {
    if (entry.kind === 'split') return { ...entry, first: split(entry.first), second: split(entry.second) };
    if (entry.id !== onto) return entry;
    const before = zone === 'left' || zone === 'top';
    return {
      kind: 'split',
      dir: zone === 'left' || zone === 'right' ? 'row' : 'column',
      first: before ? made : entry,
      second: before ? entry : made,
      share: 0.5,
    };
  };
  return tidy(split(without));
}

/** The splits of a layout, each with the path to it: `f` into the first half, `s` into the second. */
export function resize(layout: Layout, path: string, share: number): Layout {
  if (layout.kind === 'group') return layout;
  if (path === '') return { ...layout, share: Math.min(0.85, Math.max(0.15, share)) };
  const rest = path.slice(1);
  return path[0] === 'f' ? { ...layout, first: resize(layout.first, rest, share) } : { ...layout, second: resize(layout.second, rest, share) };
}

/**
 * A layout read back from storage, made to fit the tabs that are open now:
 * tabs that are gone are dropped from it, tabs it does not know are added to
 * its first group, and anything that is not a layout at all becomes one group.
 */
export function mend(saved: unknown, open: string[]): Layout {
  const seen = new Set<string>();
  const read = (value: unknown): Layout | null => {
    if (!value || typeof value !== 'object') return null;
    const node = value as Record<string, unknown>;
    if (node.kind === 'group' && typeof node.id === 'string' && Array.isArray(node.tabs)) {
      const tabs = node.tabs.filter((tab): tab is string => typeof tab === 'string' && open.includes(tab) && !seen.has(tab));
      tabs.forEach((tab) => seen.add(tab));
      const active = typeof node.active === 'string' && tabs.includes(node.active) ? node.active : (tabs[0] ?? null);
      return group(node.id, tabs, active);
    }
    if (node.kind === 'split' && (node.dir === 'row' || node.dir === 'column')) {
      const first = read(node.first);
      const second = read(node.second);
      if (!first || !second) return first ?? second;
      const share = typeof node.share === 'number' && node.share > 0 && node.share < 1 ? node.share : 0.5;
      return { kind: 'split', dir: node.dir, first, second, share };
    }
    return null;
  };
  const layout = tidy(read(saved) ?? group('g1'));
  const missing = open.filter((tab) => !seen.has(tab));
  const home = groups(layout)[0]!;
  return each(layout, (entry) =>
    entry.id !== home.id ? entry : { ...entry, tabs: [...entry.tabs, ...missing], active: entry.active ?? missing[0] ?? null },
  );
}

/** A name for a new group that no group in the layout has. */
export function fresh(layout: Layout): string {
  const taken = groups(layout).map((entry) => Number(entry.id.slice(1)) || 0);
  return `g${Math.max(0, ...taken) + 1}`;
}

/** A rectangle, as fractions of the whole area: 0 is its left or top, 1 its right or bottom. */
export interface Box {
  x: number;
  y: number;
  w: number;
  h: number;
}

/** A divider between the two halves of a split: `within` is the split it divides. */
export interface Divider {
  path: string;
  dir: 'row' | 'column';
  within: Box;
  share: number;
}

const WHOLE: Box = { x: 0, y: 0, w: 1, h: 1 };

/**
 * Where everything is. The layout is drawn flat, each group and each view
 * placed by these, so that a view moved to another group is moved on the
 * screen and not made again: what was typed in it stays.
 */
export function places(layout: Layout, box: Box = WHOLE, path = ''): { groups: (Box & { id: string })[]; dividers: Divider[] } {
  if (layout.kind === 'group') return { groups: [{ ...box, id: layout.id }], dividers: [] };
  const row = layout.dir === 'row';
  const first: Box = row ? { ...box, w: box.w * layout.share } : { ...box, h: box.h * layout.share };
  const second: Box = row
    ? { ...box, x: box.x + first.w, w: box.w - first.w }
    : { ...box, y: box.y + first.h, h: box.h - first.h };
  const a = places(layout.first, first, `${path}f`);
  const b = places(layout.second, second, `${path}s`);
  return {
    groups: [...a.groups, ...b.groups],
    dividers: [{ path, dir: layout.dir, within: box, share: layout.share }, ...a.dividers, ...b.dividers],
  };
}

/** How near an edge of a group a tab must be dropped to split it, as a share of the group. */
const EDGE = 0.25;

/**
 * What dropping a tab at a point would do: which group it is over, and
 * whether it is over that group's tabs or, in its body, which zone. `strip`
 * is the height of a row of tabs as a fraction of the whole area.
 */
export function aim(layout: Layout, x: number, y: number, strip: number): { group: string; over: 'tabs' | Zone } | null {
  const box = places(layout).groups.find((entry) => x >= entry.x && x < entry.x + entry.w && y >= entry.y && y < entry.y + entry.h);
  if (!box) return null;
  if (y < box.y + strip) return { group: box.id, over: 'tabs' };
  const across = (x - box.x) / box.w;
  const down = (y - box.y - strip) / Math.max(box.h - strip, Number.EPSILON);
  const nearest = Math.min(across, 1 - across, down, 1 - down);
  if (nearest > EDGE) return { group: box.id, over: 'center' };
  if (nearest === across) return { group: box.id, over: 'left' };
  if (nearest === 1 - across) return { group: box.id, over: 'right' };
  return { group: box.id, over: nearest === down ? 'top' : 'bottom' };
}

/** The part of a box a tab dropped in a zone would take. */
export function landing(box: Box, zone: Zone): Box {
  switch (zone) {
    case 'left':
      return { ...box, w: box.w / 2 };
    case 'right':
      return { ...box, x: box.x + box.w / 2, w: box.w / 2 };
    case 'top':
      return { ...box, h: box.h / 2 };
    case 'bottom':
      return { ...box, y: box.y + box.h / 2, h: box.h / 2 };
    default:
      return box;
  }
}

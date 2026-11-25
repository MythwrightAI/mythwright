// Channels put in groups of a person's own making.
//
// A group is a name somebody gave, and the channels they put under it. It
// is theirs and nobody else's: it is kept with their account, so that it
// follows them to every browser (`arranged.ts`, `arrangedkept.ts`), and
// the daemon keeps it and acts on none of it. What is folded is not part
// of it, and is each browser's. A channel is in one group or in none, and
// one that is in none is listed as it always was.
//
// A channel can also be pinned: it is then listed at the top, before the
// rest, and not where it otherwise would be. Its group is still its group,
// and is where it is again when it is no longer pinned.
//
// What is kept can outlive what it is about: a channel that is gone is
// simply not shown, and is where it was if it comes back from the Trash, and a group all of whose channels are gone is still
// there, empty, to put others in.

export interface Group {
  id: string;
  name: string;
}

export interface Grouping {
  /** The groups, in the order they were made. */
  groups: Group[];
  /** Which group a channel is in, by the channel's id. */
  of: Record<string, string>;
  /** The channels that are pinned to the top, by id. */
  pinned: string[];
}

export const NO_GROUPS: Grouping = { groups: [], of: {}, pinned: [] };

/** The most letters a group's name has, which is what the daemon keeps (held to it by `arrangedcases.json`). */
export const LONGEST = 60;

/** What was kept, as a grouping. Anything that is not one is none, and what of one makes no sense is left out. */
export function groupingOf(saved: string | null | undefined): Grouping {
  if (!saved) return NO_GROUPS;
  let read: unknown;
  try {
    read = JSON.parse(saved);
  } catch {
    return NO_GROUPS;
  }
  if (typeof read !== 'object' || read === null) return NO_GROUPS;
  const { groups, of, pinned } = read as { groups?: unknown; of?: unknown; pinned?: unknown };
  const kept: Group[] = [];
  for (const one of Array.isArray(groups) ? groups : []) {
    const { id, name } = (one ?? {}) as { id?: unknown; name?: unknown };
    if (typeof id === 'string' && typeof name === 'string' && name.trim() && !kept.some((group) => group.id === id)) {
      kept.push({ id, name: name.trim() });
    }
  }
  const places: Record<string, string> = {};
  for (const [channel, group] of Object.entries(typeof of === 'object' && of !== null ? of : {})) {
    if (typeof group === 'string' && kept.some((one) => one.id === group)) places[channel] = group;
  }
  const top = (Array.isArray(pinned) ? pinned : []).filter((one): one is string => typeof one === 'string');
  return { groups: kept, of: places, pinned: [...new Set(top)] };
}

/** The group of that name, whatever its capitals. */
function named(grouping: Grouping, name: string): Group | undefined {
  const wanted = name.trim().toLowerCase();
  return grouping.groups.find((group) => group.name.toLowerCase() === wanted);
}

/**
 * A group of this name: the grouping with it, and its id. One of that name
 * already there is that one, and a name of nothing but spaces makes none.
 */
export function withGroup(grouping: Grouping, name: string): { grouping: Grouping; id: string } | null {
  const called = name.trim();
  if (!called || [...called].length > LONGEST) return null;
  const there = named(grouping, called);
  if (there) return { grouping, id: there.id };
  const last = Math.max(0, ...grouping.groups.map((group) => Number(group.id.slice(1)) || 0));
  const id = `g${last + 1}`;
  return { grouping: { ...grouping, groups: [...grouping.groups, { id, name: called }] }, id };
}

/** The group called something else. Not where the name is empty, or is another group's. */
export function renamed(grouping: Grouping, id: string, name: string): Grouping {
  const called = name.trim();
  const other = named(grouping, called);
  if (!called || [...called].length > LONGEST || (other && other.id !== id)) return grouping;
  return { ...grouping, groups: grouping.groups.map((group) => (group.id === id ? { id, name: called } : group)) };
}

/** The group taken away. Its channels are in none again: nothing of a channel goes with its group. */
export function withoutGroup(grouping: Grouping, id: string): Grouping {
  const of = Object.fromEntries(Object.entries(grouping.of).filter(([, group]) => group !== id));
  return { ...grouping, groups: grouping.groups.filter((group) => group.id !== id), of };
}

/**
 * A channel put in a group, or with `null` in none. A group that is not
 * there is none. Somebody who puts a pinned channel somewhere means it to
 * be listed there, so it is no longer pinned.
 */
export function placed(grouping: Grouping, channel: string, group: string | null): Grouping {
  const of = { ...grouping.of };
  if (group !== null && grouping.groups.some((one) => one.id === group)) of[channel] = group;
  else delete of[channel];
  return { groups: grouping.groups, of, pinned: grouping.pinned.filter((one) => one !== channel) };
}

/**
 * What is said once a channel made from a group's menu has been put in
 * that group (`placed`, which puts it at the group's end as dragging it
 * there does), given what the channel is called in a sentence, the
 * group's name as it is then and, where it was made in another workspace
 * than the one showing, that one's name. Nothing where the group draws it
 * now. The group lists only the channels of the workspace showing, so one
 * made in another is in the group all the same and is said to be listed
 * there while that one is showing; and a group taken away meanwhile puts
 * it in none, which is said too.
 */
export function madeSaid(channel: string, group: string | null, elsewhere: string | null): string | null {
  if (group === null) return `${channel} is in no group: the group it was made from is no longer there.`;
  if (elsewhere !== null) return `${channel} is in ${group}, listed there while ${elsewhere} is showing.`;
  return null;
}

/** A channel pinned to the top, or no longer. Where it is otherwise listed is kept. */
export function pinned(grouping: Grouping, channel: string, on: boolean): Grouping {
  const rest = grouping.pinned.filter((one) => one !== channel);
  return { ...grouping, pinned: on ? [...rest, channel] : rest };
}

export interface Arranged<T> {
  /** Those pinned to the top, as they were listed. */
  pinned: T[];
  /** Those in no group, as they were listed. */
  loose: T[];
  /** Each group, as they were made, with its channels as they were listed. */
  groups: { group: Group; channels: T[] }[];
}

/** The channels there are, as they are to be listed. */
export function arranged<T extends { id: string }>(channels: T[], grouping: Grouping): Arranged<T> {
  const groups = grouping.groups.map((group) => ({ group, channels: [] as T[] }));
  const loose: T[] = [];
  const top: T[] = [];
  for (const channel of channels) {
    const into = groups.find((one) => one.group.id === grouping.of[channel.id]);
    (grouping.pinned.includes(channel.id) ? top : into ? into.channels : loose).push(channel);
  }
  return { pinned: top, loose, groups };
}

/**
 * The same where only some of the channels are listed (the side panel
 * held to one workspace): those, wherever somebody put them. A group all
 * of whose channels are left out is not there, and nothing of it is
 * forgotten: it is as it was when they are listed again. One with
 * nothing in it at all is there still, to put something in.
 */
export function arrangedOf<T extends { id: string }>(channels: T[], grouping: Grouping, listed: (channel: T) => boolean): Arranged<T> {
  const every = arranged(channels, grouping);
  const shown = arranged(channels.filter(listed), grouping);
  return { ...shown, groups: shown.groups.filter((one, at) => one.channels.length > 0 || every.groups[at]!.channels.length === 0) };
}

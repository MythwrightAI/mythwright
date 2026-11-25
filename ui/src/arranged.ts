// How somebody arranged the side panel, as the daemon keeps it with their
// account (`people::arranged`), and what it comes to on the page: their
// groups of channels and the channels pinned to the top (`channelgroups.ts`,
// a `Grouping`), and what is pinned at the top of Resources (`lately.ts`).
//
// What is kept is text and nothing more: a channel by its id, a thing by
// how it is kept (`keptAs`: what an address said after its `#` before
// addresses were paths, which every pin from before says). Here is what
// goes between the two, and what is decided of what a browser kept before
// the account kept anything, with nothing of the page in it.

import type { Grouping } from './channelgroups.ts';
import { NO_GROUPS } from './channelgroups.ts';
import { isThing } from './lately.ts';
import type { Tab } from './tab.ts';
import { fromKept, keptAs, tabKey } from './tab.ts';

export interface Arrangement {
  /** The groups, in the order they were made, each with its channels. */
  groups: { id: string; name: string; channels: string[] }[];
  /** The channels pinned to the top. */
  pinned: string[];
  /** What is pinned in Resources, each by its address. */
  things: string[];
}

export const NOTHING_ARRANGED: Arrangement = { groups: [], pinned: [], things: [] };

/** What the daemon hands of an arrangement. */
export interface KeptArrangement {
  /** Nothing where the account never kept one. */
  arrangement: Arrangement | null;
  /** How many times it was written: the later of two is told by it. */
  nth: number;
}

/** The groups and pins of channels an arrangement says. */
export function groupingFrom(arrangement: Arrangement): Grouping {
  if (!arrangement.groups.length && !arrangement.pinned.length) return NO_GROUPS;
  const of: Record<string, string> = {};
  for (const group of arrangement.groups) for (const channel of group.channels) of[channel] = group.id;
  return { groups: arrangement.groups.map(({ id, name }) => ({ id, name })), of, pinned: [...arrangement.pinned] };
}

/** The things an arrangement says are pinned, as tabs: an address that opens nothing that is kept to hand is passed over. */
export function thingsFrom(arrangement: Arrangement): Tab[] {
  const seen = new Set<string>();
  return arrangement.things.flatMap((address) => {
    const tab = fromKept(address);
    if (!tab || !isThing(tab) || seen.has(tabKey(tab))) return [];
    seen.add(tabKey(tab));
    return [tab];
  });
}

/** An arrangement of these groups and pins and these things, as it is kept. */
export function arrangementOf(grouping: Grouping, things: Tab[]): Arrangement {
  return {
    groups: grouping.groups.map((group) => ({ ...group, channels: Object.keys(grouping.of).filter((channel) => grouping.of[channel] === group.id) })),
    pinned: [...grouping.pinned],
    things: things.flatMap((thing) => keptAs(thing) ?? []),
  };
}

/** The same arrangement with its groups and pins of channels changed. */
export function withGrouping(arrangement: Arrangement, grouping: Grouping): Arrangement {
  return { ...arrangementOf(grouping, []), things: arrangement.things };
}

/** The same arrangement with what is pinned in Resources changed. */
export function withThings(arrangement: Arrangement, things: Tab[]): Arrangement {
  return { ...arrangement, things: arrangementOf(NO_GROUPS, things).things };
}

/** Whether an arrangement says nothing at all. */
export function isEmpty(arrangement: Arrangement): boolean {
  return !arrangement.groups.length && !arrangement.pinned.length && !arrangement.things.length;
}

/**
 * What is done with what a browser kept before arrangements were the
 * account's, the first time the account's is read: sent to be kept where
 * the account keeps none and the browser has anything (`send`), and taken
 * out of the browser either way, without merging, where the account keeps
 * one already.
 */
export function takenUp(account: Arrangement | null, before: { grouping: Grouping; pinned: Tab[] }): { send: Arrangement | null } {
  if (account) return { send: null };
  const kept = arrangementOf(before.grouping, before.pinned);
  return { send: isEmpty(kept) ? null : kept };
}

/** Whether what was heard is later than what the page has: an answer and what is said of it may come in either order. */
export function isLater(had: number, heard: number): boolean {
  return heard > had;
}

/**
 * The arrangement as a browser keeps a copy of it, only so that the panel
 * is drawn at once before the account's is read: without the channels the
 * page does not list (taken away, or kept from whoever reads it) and the
 * things it knows to be gone, so that the browser keeps nothing of them.
 * `listed` is nothing while the channels were not read.
 */
export function copyOf(arrangement: Arrangement, listed: Set<string> | null, gone: (thing: Tab) => boolean): Arrangement {
  const there = (channel: string) => listed === null || listed.has(channel);
  return {
    groups: arrangement.groups.map((group) => ({ ...group, channels: group.channels.filter(there) })),
    pinned: arrangement.pinned.filter(there),
    things: arrangement.things.filter((address) => {
      const tab = fromKept(address);
      return !tab || !gone(tab);
    }),
  };
}

/** A copy a browser kept, read: anything that is not one is nothing. */
export function copyRead(saved: string | null | undefined): Arrangement | null {
  if (!saved) return null;
  try {
    const read = JSON.parse(saved) as Partial<Arrangement> | null;
    if (typeof read !== 'object' || read === null || Array.isArray(read)) return null;
    const texts = (list: unknown) => (Array.isArray(list) ? list.filter((one): one is string => typeof one === 'string') : []);
    const groups = (Array.isArray(read.groups) ? read.groups : []).flatMap((group) =>
      group && typeof group.id === 'string' && typeof group.name === 'string' ? [{ id: group.id, name: group.name, channels: texts(group.channels) }] : [],
    );
    return { groups, pinned: texts(read.pinned), things: texts(read.things) };
  } catch {
    return null;
  }
}

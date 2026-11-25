// What whoever is signed in follows (`/api/following`): a page of a wiki,
// alone or with every page under it, a board, a table, a library, a
// channel or a project, each with how often they are told of what changes
// in it. Read once something asks, and again whenever the daemon says one
// of their follows changed (`inbox.follows`, which only they hear) or the
// stream says it may have missed something.
//
// What a page says of a follow, and which thing a tab is, are functions
// here of plain values, with their cases in `following.test.ts`.

import { signal } from '@preact/signals';
import { call } from './api.ts';
import { stream } from './events.ts';
import type { Tab } from './tab.ts';

/** How often somebody is told of what changes in what they follow. */
export type Often = 'once' | 'hourly' | 'daily' | 'off';

/** The ways, in the order they are offered, each as it is said. */
export const OFTEN: [Often, string][] = [
  ['once', 'At once'],
  ['hourly', 'Hourly'],
  ['daily', 'Daily'],
  ['off', 'Off'],
];

/** One thing followed, as the daemon hands it out. */
export interface Follow {
  kind: string;
  thing: string;
  title: string;
  /** For a page: whether every page under it is followed with it. */
  pages: boolean;
  often: Often;
  /** The time of day a daily digest comes at, in the follower's zone. */
  at: string;
  made: number;
}

/** What may be followed, as the page knows it: its kind and which. */
export interface Followable {
  kind: string;
  thing: string;
}

/** What one change a line of something followed told of is. */
export interface Change {
  seq: number;
  record: string;
  label: string;
  noun: string;
  verb: string;
  from: string;
  at: number;
}

/** The thing a tab shows, where it can be followed: a page of a wiki, a board, a table, a library, a channel, a project. */
export function followableOf(tab: Tab): Followable | null {
  switch (tab.kind) {
    case 'topic':
      return { kind: 'page', thing: tab.id };
    case 'board':
      return { kind: 'board', thing: `${tab.project}/board` };
    case 'table':
      return { kind: 'table', thing: `${tab.project}/${tab.name}` };
    case 'library':
      return { kind: 'library', thing: `${tab.project}/${tab.name}` };
    case 'channel':
      return { kind: 'channel', thing: tab.id };
    case 'project':
      return { kind: 'project', thing: tab.id };
    default:
      return null;
  }
}

/** The tab that shows a thing followed. */
export function tabOf(follow: Followable): Tab | null {
  const [project, name] = follow.thing.split('/');
  switch (follow.kind) {
    case 'page':
      return { kind: 'topic', id: follow.thing };
    case 'board':
      return project ? { kind: 'board', project } : null;
    case 'table':
      return project && name ? { kind: 'table', project, name } : null;
    case 'library':
      return project && name ? { kind: 'library', project, name } : null;
    case 'channel':
      return { kind: 'channel', id: follow.thing };
    case 'project':
      return { kind: 'project', id: follow.thing };
    default:
      return null;
  }
}

/** The thing a line of something followed is of, by its place (`<kind>:<thing>`, a digest's after a `#` too). */
export function placeOf(place: string): Followable | null {
  const at = place.indexOf(':');
  if (at <= 0) return null;
  const thing = place.slice(at + 1).split('#')[0] ?? '';
  return thing ? { kind: place.slice(0, at), thing } : null;
}

/** Whether two say the same thing, a table's kinds being one thing to the daemon's id. */
export function same(one: Followable, other: Followable): boolean {
  const table = (kind: string) => (kind === 'board' || kind === 'table' || kind === 'library' ? 'table' : kind);
  return one.thing === other.thing && table(one.kind) === table(other.kind);
}

/** How a follow of `thing` stands among `all`: nothing where it is not followed. */
export function followOf(all: Follow[], thing: Followable): Follow | null {
  return all.find((one) => same(one, thing)) ?? null;
}

/** What a kind is called, as Settings lists it. */
export function kindSaid(kind: string, pages = false): string {
  if (kind === 'page') return pages ? 'Wiki page and its pages' : 'Page';
  return kind.charAt(0).toUpperCase() + kind.slice(1);
}

/** How often, in words: `At once`, `Hourly`, `Daily at 09:00`, `Off`. */
export function oftenSaid(often: Often, at: string): string {
  if (often === 'daily') return `Daily at ${at}`;
  return OFTEN.find(([one]) => one === often)?.[1] ?? 'At once';
}

/** What a change a line told of says: `Ada changed a row: Basket`. */
export function changeSaid(change: Change): string {
  const who = change.from.slice(change.from.indexOf(':') + 1);
  const did = change.verb === 'said' ? `said a new ${change.noun}` : `${change.verb} ${change.noun === 'reply' ? 'a reply' : `a ${change.noun}`}`;
  return `${who} ${did}${change.label ? `: ${change.label}` : ''}`;
}

/** Everything followed, as last read: nothing until something asks. */
export const following = signal<Follow[] | null>(null);

let asked = 0;
let heard = false;

/** Reads what is followed again; of two readings on their way, the one asked for last is kept. */
export async function readFollowing(): Promise<void> {
  const mine = ++asked;
  const read = await call<{ following: Follow[] }>('GET', '/following').catch(() => null);
  if (read && mine === asked) following.value = read.following;
}

/** Reads what is followed, once, and follows it from then on. */
export function useFollowing(): Follow[] | null {
  if (!heard) {
    heard = true;
    stream.on('inbox.follows', () => void readFollowing(), () => void readFollowing());
    void readFollowing();
  }
  return following.value;
}

/** Follows a thing as `how` says, what is not said being as it was; or, with `follows` false, no longer. */
export async function follow(thing: Followable, how: { follows?: boolean; pages?: boolean; often?: Often; at?: string }): Promise<void> {
  await call('POST', '/following', { ...thing, ...how });
  await readFollowing();
}

/** The changes a line of something followed told of, oldest first. */
export function changesOf(line: string): Promise<Change[]> {
  return call<{ changes: Change[] }>('GET', `/inbox/${encodeURIComponent(line)}/changes`).then((read) => read.changes);
}

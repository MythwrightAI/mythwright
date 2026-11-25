// A topic's revisions as its History reads them (`GET /api/topics/<topic>/revisions`,
// oldest first): what the daemon keeps of each change of its title, its post
// (the article) or its summary, whatever door it came by, and what History
// works out from them: what each changed, which one a later one brought back,
// and what changed between any two, part by part. It holds nothing of the
// page, so that its cases can read it.

import { diff } from './worddiff.ts';
import type { Piece } from './worddiff.ts';

/** One revision of a topic: all it said then, whole, who made it and when. */
export interface Revision {
  /** Its number, from 1 for what it said as it was made. */
  nth: number;
  title: string;
  body: string;
  summary: string;
  /** Who made it, by their actor, and by name. */
  by: string | null;
  who: string;
  /** The blueprint's instance it was made through, where a blueprint made it. */
  through: string | null;
  at: number;
}

/** The parts of a topic a revision keeps, in the order the page draws them. */
export type Part = 'title' | 'summary' | 'article';

export const PARTS: readonly Part[] = ['title', 'summary', 'article'];

/** What each part is called where History says it, on an article. */
export const PART_NAMES: Record<Part, string> = { title: 'Title', summary: 'Summary', article: 'Article' };

/** Where a topic's History is read: its article's tab, or a thread's place, where what it says is its post. */
export type HistoryOf = 'article' | 'thread';

/** What a part is called where History says it: a thread's post (a tracked question's) is its Post, as its page calls it. */
export const partName = (part: Part, of: HistoryOf = 'article'): string => (part === 'article' && of === 'thread' ? 'Post' : PART_NAMES[part]);

const text = (value: unknown): string => (typeof value === 'string' ? value : '');

/** A revision from what the daemon handed, whatever it handed of it. */
export function revisionOf(handed: Record<string, unknown>): Revision {
  return {
    nth: Number(handed.nth) || 0,
    title: text(handed.title),
    body: text(handed.body),
    summary: text(handed.summary),
    by: typeof handed.by === 'string' ? handed.by : null,
    who: text(handed.who) || text(handed.by) || 'the factory',
    through: typeof handed.through === 'string' && handed.through ? handed.through : null,
    at: Number(handed.at) || 0,
  };
}

/** What a part says in a revision. */
export function partOf(revision: Pick<Revision, 'title' | 'body' | 'summary'>, part: Part): string {
  return part === 'title' ? revision.title : part === 'summary' ? revision.summary : revision.body;
}

/** The parts that say something else in `newer` than in `older`; every part that says something, for the first. */
export function changedParts(older: Revision | null, newer: Revision): Part[] {
  return PARTS.filter((part) => (older ? partOf(older, part) !== partOf(newer, part) : partOf(newer, part).trim() !== ''));
}

/** A line of History: a revision, what it changed, and the earlier one it brought back where it says just what that one said. */
export interface HistoryLine {
  revision: Revision;
  changed: Part[];
  /** The first: what it said as it was made. */
  first: boolean;
  /** The latest: what it says now, which is not restored. */
  latest: boolean;
  /** The earlier revision it says all of again, where one before the one before it does (as restoring one makes). */
  restored: Revision | null;
}

const same = (one: Revision, other: Revision) => PARTS.every((part) => partOf(one, part) === partOf(other, part));

/** The lines of History, newest first, from the revisions as the daemon hands them (oldest first, or in any order). */
export function historyOf(revisions: readonly Revision[]): HistoryLine[] {
  const sorted = [...revisions].sort((one, other) => one.nth - other.nth);
  const lines = sorted.map((revision, at) => {
    const before = at > 0 ? sorted[at - 1]! : null;
    // The latest that said all this before the one it changed: what a restore of it makes.
    let restored: Revision | null = null;
    for (let back = at - 2; back >= 0 && before && !same(before, revision); back -= 1) {
      if (same(sorted[back]!, revision)) {
        restored = sorted[back]!;
        break;
      }
    }
    return { revision, changed: changedParts(before, revision), first: at === 0, latest: at === sorted.length - 1, restored };
  });
  return lines.reverse();
}

/** What changed in one part from one revision to another. */
export interface PartChange {
  part: Part;
  pieces: Piece[];
}

/** What changed from `older` to `newer`, part by part, in the order the page draws them: only the parts that changed. */
export function compared(older: Revision, newer: Revision): PartChange[] {
  return PARTS.filter((part) => partOf(older, part) !== partOf(newer, part)).map((part) => ({ part, pieces: diff(partOf(older, part), partOf(newer, part)) }));
}

/** The two revisions to compare, by their numbers, the older first; nothing where there are not two. */
export function pair(chosen: readonly number[]): [number, number] | null {
  const two = [...new Set(chosen)].sort((one, other) => one - other);
  return two.length === 2 ? [two[0]!, two[1]!] : null;
}

/** What is chosen to compare once `nth` is pressed: added, or taken away where it was chosen; the earliest of three let go. */
export function choosing(chosen: readonly number[], nth: number): number[] {
  if (chosen.includes(nth)) return chosen.filter((one) => one !== nth);
  return [...chosen, nth].slice(-2);
}

/** The fields a topic is set to as a revision is restored: what it said then, all of it, which the daemon keeps as a new revision. */
export function restoring(revision: Revision): { title: string; body: string; summary: string } {
  return { title: revision.title, body: revision.body, summary: revision.summary };
}

/** How the door a revision came by is said, where it was not a person's own hand: a blueprint's. */
export function doorSaid(revision: Pick<Revision, 'through'>): string | null {
  return revision.through ? 'through a blueprint' : null;
}

/** What a line of History says it changed: its parts by name, or that it was made, or that it changed nothing a reader sees. */
export function changedSaid(line: Pick<HistoryLine, 'changed' | 'first'>, of: HistoryOf = 'article'): string {
  if (line.first) return 'Made';
  if (line.changed.length === 0) return 'No change';
  const named = line.changed.map((part) => partName(part, of));
  return named.length === 1 ? named[0]! : `${named.slice(0, -1).join(', ')} and ${named.at(-1)!}`;
}

// The Trash: what was deleted, as the daemon hands it to whoever asks
// (`/api/trash`), given back and deleted for good from there, and how
// long what is deleted is kept (`/api/system/trash`). Read and changed
// here and not in `api.ts`, which is at its ceiling; and what is said of
// it, as functions of plain values with their cases.

import { call } from './api.ts';

export type Kind = 'record' | 'file' | 'project' | 'channel' | 'attachment';

/** Something in the Trash, as the daemon hands it. */
export interface Trashed {
  id: string;
  kind: Kind;
  /** What it is known by: a record's id, a file's path in its project, a project's id; for an attachment, its record's id. */
  thing: string;
  project: string;
  /** The table, for a row of one. */
  table?: string;
  called: string;
  /** Who deleted it, as an actor (`user:<id>`), and by name. */
  by: string;
  who: string;
  at: number;
  /** When it goes for good; never, where it is kept until somebody deletes it for good. */
  goes: number | null;
  /** Whether whoever reads this may give it back and delete it for good. */
  may: boolean;
  /** How many records went in with it and come back with it: the pages under a page. Left out for none. */
  with?: number;
}

/** How long what is deleted is kept: so many days, nought for at once, or for ever. */
export interface Keeping {
  days: number | null;
  by?: string;
  at?: number;
}

export const trashed = () => call<{ trash: Trashed[]; days: number | null }>('GET', '/trash');
export const restore = (id: string, beside = false) =>
  call<{ restored: Trashed; at: string; said?: string }>('POST', `/trash/${encodeURIComponent(id)}/restore`, { beside });
export const forget = (id: string) => call<{ gone: Trashed }>('DELETE', `/trash/${encodeURIComponent(id)}`);
/** Forgets a project, into the Trash: nothing it runs is stopped, and its files stay where they are. */
export const forgetProject = (id: string) => call<{ trash?: string | null; kept?: number | null }>('DELETE', `/projects/${encodeURIComponent(id)}`);
export const trashKept = () => call<Keeping>('GET', '/system/trash');
export const keepTrash = (days: number | 'always') => call<Keeping & { gone: number }>('POST', '/system/trash', { days });

/** The kinds, in the order the page draws them, each with its heading. */
export const KINDS: { kind: Kind; heading: string }[] = [
  { kind: 'record', heading: 'Rows and cards' },
  { kind: 'file', heading: 'Files' },
  { kind: 'project', heading: 'Projects' },
  { kind: 'channel', heading: 'Channels' },
  { kind: 'attachment', heading: 'Attachments' },
];

/** What is in the Trash, by kind, each kind the latest first; a kind with nothing is left out. */
export function byKind(all: Trashed[]): { kind: Kind; heading: string; things: Trashed[] }[] {
  return KINDS.map(({ kind, heading }) => ({
    kind,
    heading,
    things: all.filter((one) => one.kind === kind).sort((a, b) => b.at - a.at || a.id.localeCompare(b.id)),
  })).filter((group) => group.things.length > 0);
}

const DAY = 86_400_000;

/** How many days, in words. */
function days(count: number): string {
  return count === 1 ? 'a day' : `${count} days`;
}

/** When it goes for good, said from `now`. */
export function goesSaid(goes: number | null, now: number): string {
  if (goes === null) return 'kept until it is deleted for good';
  const left = goes - now;
  if (left <= 0) return 'goes for good now';
  if (left < DAY) return 'goes for good today';
  return `goes for good in ${days(Math.ceil(left / DAY))}`;
}

/** The pages that went with a page, in a few words after it: nothing for none. */
export function pagesSaid(withIt: number | undefined): string {
  if (!withIt) return '';
  return withIt === 1 ? ', with the page under it' : `, with the ${withIt} pages under it`;
}

/** Whether it is a page of a wiki: a record of the factory's own table of topics. */
const isPage = (one: Pick<Trashed, 'kind' | 'project' | 'table'>) => one.kind === 'record' && one.project === '@' && one.table === 'topics';

/** Where it was, in a few words. */
export function whereSaid(one: Pick<Trashed, 'kind' | 'project' | 'table' | 'thing' | 'with'>, projectName: (id: string) => string): string {
  if (isPage(one)) return `a page of a wiki${pagesSaid(one.with)}`;
  const project = projectName(one.project);
  switch (one.kind) {
    case 'record':
      return one.table === 'board' ? `a card of ${project}'s board` : `a row of ${project}'s ${one.table ?? 'table'}`;
    case 'file':
      return `${one.thing} in ${project}`;
    case 'project':
      return 'a project, its files where they were';
    case 'channel':
      return 'a channel, with everything said and attached in it';
    case 'attachment':
      return `attached to ${one.thing} of ${project}`;
  }
}

/** What is said as something is deleted, with how long it is kept, and what went with it (the pages under a page). */
export function movedSaid(keptDays: number | null, withIt?: number): string {
  const went = pagesSaid(withIt);
  if (keptDays === null) return `Moved to Trash${went}, kept until it is deleted for good.`;
  if (keptDays === 0) return `Deleted${went}.`;
  return `Moved to Trash${went}, kept ${days(keptDays)}.`;
}

/** How long, in a sentence, for Settings. */
export function keptSaid(keptDays: number | null): string {
  if (keptDays === null) return 'What is deleted is kept in the Trash until somebody deletes it for good.';
  if (keptDays === 0) return 'What is deleted goes at once: nothing is kept in the Trash, and it cannot be given back.';
  return `What is deleted is kept in the Trash for ${days(keptDays)}, and then goes for good.`;
}

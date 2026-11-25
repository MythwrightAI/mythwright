// A project's Git page: its repository as the daemon reads it from git
// (`places::seen`), and the parts installed packages say they show of a
// repository they serve (`agents::shown`). What is asked and what it
// comes to in words, as functions of plain values; what reads and draws
// is `views/gitpage.tsx`.
//
// Nothing here knows a service. A part is a package's title, a heading,
// its columns and, for each row, where it is read.

import { call } from './api.ts';

export interface Head {
  branch?: string | null;
  commit?: string | null;
  upstream?: string | null;
  ahead?: number | null;
  behind?: number | null;
}
export interface Branch {
  name: string;
  remote?: string;
  refname: string;
  commit: string;
  at: number;
  subject: string;
}
export interface Commit {
  commit: string;
  author: string;
  email: string;
  at: number;
  parents: string[];
  subject: string;
}
export interface Changed {
  path: string;
  was?: string;
  staged: string;
  unstaged: string;
}
export interface Remote {
  name: string;
  fetch: string;
  push?: string;
}
export interface Column {
  field: string;
  title: string;
}
export interface Part {
  name: string;
  title: string;
  columns: Column[];
  link?: string;
}
export interface Offered {
  package: string;
  title: string;
  parts: Part[];
  refused?: string;
}
/** The repository as it stands, or why a project's folder is none. */
export interface Seen {
  project: string;
  repository: boolean;
  said?: string;
  head?: Head;
  branches?: Branch[];
  of?: string;
  commits?: Commit[];
  changes?: Changed[];
  more?: number;
  remotes?: Remote[];
  shows?: Offered[];
}
export type Cell = string | number;
export interface Read {
  part: Part;
  rows: Record<string, Cell>[];
  more: number;
}

const part = (text: string) => encodeURIComponent(text);

/** The repository as it stands, with the commits of `branch` (whole, as it is listed) or of what the folder is on. */
export function readSeen(project: string, branch?: string | null): Promise<Seen> {
  const asked = branch && branch !== 'HEAD' ? `?branch=${part(branch)}` : '';
  return call<Seen>('GET', `/projects/${part(project)}/git${asked}`);
}

/** One part a package shows, read now by its program. */
export function readPart(project: string, offered: string, name: string): Promise<Read> {
  return call<Read>('GET', `/projects/${part(project)}/git/${part(offered)}/${part(name)}`);
}

/** A commit as a person reads one: its first ten letters. */
export const short = (commit: string | null | undefined) => (commit ?? '').slice(0, 10);

/** The branch the folder is on, in words: where it is, and how it stands against the branch it follows. */
export function headSaid(head: Head | undefined): string {
  if (!head) return '';
  const on = head.branch ? `On ${head.branch}` : `On no branch, at ${short(head.commit)}`;
  if (!head.commit) return `${on}, with nothing committed yet`;
  if (!head.upstream) return on;
  const [ahead, behind] = [head.ahead ?? 0, head.behind ?? 0];
  const stands =
    ahead === 0 && behind === 0
      ? 'level with it'
      : [ahead > 0 ? `${ahead} ahead` : '', behind > 0 ? `${behind} behind` : ''].filter(Boolean).join(' and ');
  return `${on}, following ${head.upstream}: ${stands}`;
}

/** What is changed of a path, in words: what is staged of it, then what is not. */
export function changeSaid(change: Changed): string {
  if (change.staged === 'unmerged') return 'both sides changed it, in the middle of a merge';
  const said: string[] = [];
  if (change.staged) said.push(`${change.staged}, staged`);
  if (change.unstaged === 'untracked') said.push('new, not tracked');
  else if (change.unstaged && change.unstaged !== change.staged) said.push(`${change.unstaged}, not staged`);
  else if (change.unstaged) said.push('and changed again');
  return said.join('; ');
}

/** A repository's branches by where they are: its own first, then each remote's under the remote's name. */
export function branchesBy(branches: Branch[]): [string, Branch[]][] {
  const by = new Map<string, Branch[]>();
  for (const branch of branches) {
    const where = branch.remote ?? '';
    by.set(where, [...(by.get(where) ?? []), branch]);
  }
  return [...by.entries()].sort(([a], [b]) => (a === '' ? -1 : b === '' ? 1 : a.localeCompare(b)));
}

/** A cell as it is drawn: what the daemon handed, as text. */
export const cellSaid = (cell: Cell | undefined) => (cell === undefined || cell === null ? '' : String(cell));

/** Where a row is read, where the part says a link and the row has one. */
export const linkOf = (read: Read, row: Record<string, Cell>): string | null => {
  const link = read.part.link ? row[read.part.link] : undefined;
  return typeof link === 'string' && /^https?:\/\//.test(link) ? link : null;
};

/** What is said where a part has no rows. */
export const noneSaid = (title: string) => `No ${title.toLowerCase()}.`;

/**
 * Reads one thing at a time and, asked again while it reads, once more
 * when that is done, with the last thing asked: what a page that is told
 * the repository changed many times in a moment needs, and never two
 * readings drawn in the wrong order.
 */
export class OneAtATime {
  private reading = false;
  private again = false;
  private readonly read: () => Promise<void>;
  constructor(read: () => Promise<void>) {
    this.read = read;
  }
  ask(): void {
    if (this.reading) {
      this.again = true;
      return;
    }
    this.reading = true;
    void this.read().finally(() => {
      this.reading = false;
      if (this.again) {
        this.again = false;
        this.ask();
      }
    });
  }
}

/** What a project's Git page is marked by, wherever it is listed. */
/**
 * Whether a project's Git line is listed under it in the side panel: where its folder is a repository and is one
 * whoever added it said, as its Repo line is. A folder the factory made is made a repository only so that pieces of
 * work can branch it, and a project is not taken for software for that; its Git page is still opened from its own
 * page and by its address. A daemon from before says nothing of either, and its projects' folders are taken as given.
 */
export const gitListed = (project: { folder?: 'given' | 'own'; repository?: boolean }): boolean =>
  project.repository === true && project.folder !== 'own';

export const GIT_GLYPH = '⎇';

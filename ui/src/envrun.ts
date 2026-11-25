// An environment loaded from a branch, as the client shows it: the
// blueprint it says it is loaded by, the branches of its project's
// repository to choose from, whether Run is open, and what it last ran
// (`docs/environments.md`, *Loaded from a branch*). Worked out from what
// the daemon answers and nothing else; how a run went is the run's, read
// from it by the daemon as the environment is shown, and the run's steps
// are drawn as a run's always are (`views/run.tsx`).

import { covers } from './holds.ts';

/**
 * What an environment was last loaded with, as the daemon answers it: what
 * it keeps, or the run of its deploy that is newer than that, whose commit
 * is known only where its blueprint took one (empty otherwise).
 */
export interface Ran {
  branch: string;
  /** The commit the branch was at, whole. */
  commit: string;
  blueprint: string;
  at: number;
  by_name: string;
  instance: string;
  run?: string | null;
  /** How its run went, read from the run as the environment was shown: nothing where the run is no longer there. */
  status?: 'running' | 'ok' | 'failed' | 'cancelled';
  finished?: number | null;
  error?: string | null;
}

/** A branch of the project's repository: its own, or as a remote holds it (`origin/dev`). */
export interface Branch {
  name: string;
  remote?: string;
  commit: string;
  /** When its commit was made, in milliseconds. */
  at: number;
  subject: string;
}

/** The project's branches as the daemon answers them. */
export interface Branches {
  repository: boolean;
  /** Its remotes by name: Fetch asks them, and is offered only where there is one. */
  remotes?: string[];
  branches: Branch[];
  /** Why there are none, where the project is no repository. */
  said?: string;
}

/** What may be done here by whoever holds `scopes`, as the daemon holds it to. */
export interface MayRun {
  /** Run it: what applying takes. What the blueprint asks is asked by the apply. */
  run: boolean;
  /** Ask the remotes what they have now: writing the project, and `shell`. */
  fetch: boolean;
}

export function mayRun(scopes: string[]): MayRun {
  return {
    run: covers(scopes, 'blueprints:run') && covers(scopes, 'projects:read'),
    fetch: covers(scopes, 'projects:write') && covers(scopes, 'shell'),
  };
}

/** A commit as it is said: its first seven. */
export const short = (commit: string): string => commit.slice(0, 7);

/**
 * The branch that is chosen: what somebody picked while it is one there
 * is, else what the environment last ran while it is there, else the
 * first; nothing where there is none.
 */
export function chosen(branches: Branch[], picked: string, ran: Ran | null | undefined): string {
  const there = (name: string | undefined) => !!name && branches.some((branch) => branch.name === name);
  if (there(picked)) return picked;
  if (there(ran?.branch)) return ran!.branch;
  return branches[0]?.name ?? '';
}

/** Why Run is closed, where it is; nothing where it is open. */
export function runClosed(said: { deploy: string | null | undefined; branches: Branches | null; branch: string; may: boolean }): string | null {
  if (!said.may) return 'Running it takes what applying a blueprint takes, which you do not hold.';
  if (!said.deploy) return 'Say the blueprint it is loaded by first.';
  if (!said.branches) return 'Reading the branches…';
  if (!said.branches.repository) return said.branches.said ?? 'The project is no repository, so it has no branch to run.';
  if (said.branches.branches.length === 0) return 'The repository has no branch yet: nothing is committed.';
  if (!said.branches.branches.some((branch) => branch.name === said.branch)) return 'Choose the branch to load it from.';
  return null;
}

/**
 * What an environment last ran, as the daemon says it (`environments::last`):
 * what was loaded through Run, else the latest run of anything applied
 * under it, else that something runs under it, else that nothing ran (with
 * the command that names its deploy blueprint where it names none). The
 * daemon decides and words it; the page only says when beside it.
 */
export interface Last {
  kind: 'loaded' | 'applied' | 'running' | 'unsaid' | 'never';
  said: string;
  at?: number;
  run?: string;
  instance?: string;
}

/** What an environment last ran, in a line: the daemon's words, and when where it is a run. */
export function lastRunSaid(last: Last | null | undefined, ago: (at: number) => string): string {
  if (!last) return '';
  return last.at === undefined ? last.said : `${last.said}, ${ago(last.at)}`;
}

/** A branch as it is offered: its name, its commit and the first line of it. */
export function branchSaid(branch: Branch): string {
  return branch.subject ? `${branch.name} · ${short(branch.commit)} · ${branch.subject}` : `${branch.name} · ${short(branch.commit)}`;
}

/** One thing a run gave out: its name, its words, and where it leads where it is an address of the web. */
export interface Gave {
  name: string;
  text: string;
  href: string | null;
}

/** What a run gave out (its `outputs`), each by name in the run's order; an address of the web is a link, and nothing else is. */
export function gave(outputs: unknown): Gave[] {
  if (!outputs || typeof outputs !== 'object' || Array.isArray(outputs)) return [];
  return Object.entries(outputs as Record<string, unknown>)
    .filter(([, value]) => value !== null && value !== undefined && value !== '')
    .map(([name, value]) => {
      const text = typeof value === 'string' ? value : JSON.stringify(value);
      return { name, text, href: typeof value === 'string' && /^https?:\/\/[^\s]+$/.test(value.trim()) ? value.trim() : null };
    });
}

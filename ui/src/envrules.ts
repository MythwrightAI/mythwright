// What an environment holds to, as the client shows it in its drawer
// (`views/envdrawer.tsx`; `environments::rules` in the daemon): who approves what is started under it and which branches
// run under it, said in words to anybody and changed by whoever may change
// its secrets; and the runs that acted under it, each opening its run's
// page. Worked out from what the daemon answers and nothing else: what
// either rule comes to for something started under it is the daemon's.

import type { Member } from './api.ts';
import { covers } from './holds.ts';

/** What an environment holds to, as the daemon answers it: each list left out where it says nothing. */
export interface Rules {
  approve?: string[];
  branches?: string[];
}

/** A run that acted under an environment, as the daemon lists it with it, the newest first. */
export interface Acted {
  run: string;
  status: 'running' | 'ok' | 'failed' | 'cancelled' | string;
  at: number;
  /** What it is a run of, said as what was applied under the environment. */
  what: string;
  by_name: string;
}

/** Whether whoever holds `scopes` may say what an environment holds to: what the daemon asks of it. */
export const mayRule = (scopes: string[]): boolean => covers(scopes, 'environments:secrets');

/** A person's name by their actor, from those read; somebody not read, as somebody. */
export function nameOf(actor: string, people: Member[]): string {
  return people.find((one) => one.actor === actor)?.name ?? 'somebody no longer here';
}

/** Names said as a sentence says them: `Grace`, `Grace and Ada`, `Grace, Ada and Bob`. */
export function joined(names: string[]): string {
  if (names.length < 2) return names[0] ?? '';
  return `${names.slice(0, -1).join(', ')} and ${names[names.length - 1]}`;
}

/** Who approves, in words, for whoever reads the drawer. */
export function approveSaid(rules: Rules | undefined, people: Member[]): string {
  const approve = rules?.approve ?? [];
  if (approve.length === 0) return 'Nobody is named: whoever may apply starts anything under it.';
  const names = joined(approve.map((one) => nameOf(one, people)));
  return `${names} ${approve.length === 1 ? 'approves' : 'approve'} what goes under it: anybody else's run waits for a go-ahead from one of them.`;
}

/** Which branches run under it, in words. */
export function branchesSaid(rules: Rules | undefined): string {
  const branches = rules?.branches ?? [];
  if (branches.length === 0) return 'Any branch may be run under it.';
  return `Only ${joined(branches)} may be run under it.`;
}

/** The branches as they are written in their box: by spaces. */
export const branchesWritten = (rules: Rules | undefined): string => (rules?.branches ?? []).join(' ');

/** What is written in the box, as the branches it says: by spaces or commas, each once, in order. */
export function branchesOf(text: string): string[] {
  const said: string[] = [];
  for (const one of text.split(/[\s,]+/)) if (one && !said.includes(one)) said.push(one);
  return said;
}

/** Those who may be added as approving: the people read who are not among them already, by name. */
export function addable(people: Member[], rules: Rules | undefined): Member[] {
  const approve = rules?.approve ?? [];
  return people.filter((one) => one.actor.startsWith('user:') && !approve.includes(one.actor)).sort((a, b) => a.name.localeCompare(b.name));
}

/** How a run went, in words. */
export function wentSaid(status: string): string {
  return status === 'ok' ? 'went well' : status === 'failed' ? 'failed' : status === 'cancelled' ? 'was cancelled' : status === 'running' ? 'running' : 'not known';
}

/** A run that acted under it, in a line: what it is of, how it went, by whom and when. */
export function actedSaid(acted: Acted, ago: (at: number) => string): string {
  return `${acted.what}, ${wentSaid(acted.status)}, by ${acted.by_name}, ${ago(acted.at)}`;
}

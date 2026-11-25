// What is said before a key is revoked, and the asking for it.
//
// The daemon says what revoking a key would come to, changing nothing by
// being asked (`GET /api/credentials/<id>/withdrawing`): the keys made
// with it, which go with it, and what each applied that is still running,
// which is stopped with it unless it is left. This is those said as lines
// for somebody to read through before they say yes. It holds nothing of
// the page, so that its tests can read it.

import { call } from './api.ts';

/** An instance a key applied, as the daemon says it. */
export interface Applied {
  id: string;
  blueprint: string;
  key: string;
  project: string | null;
  /** Why it could not be stopped, where it could not. */
  why?: string;
}

/** What revoking a key would come to, or came to. */
export interface Revoking {
  name: string;
  /** The keys made with it, all the way down: revoked with it. */
  keys: { id: string; name: string }[];
  /** What they applied that is running: stopped, unless it is left. */
  running: Applied[];
  /** What was stopped, once it is revoked. */
  stopped?: Applied[];
}

/** What revoking the key `id` would come to: nothing is changed by asking. */
export const revoking = (id: string) => call<Revoking>('GET', `/credentials/${encodeURIComponent(id)}/withdrawing`);
/** Revokes the key `id`, and every key made with it; what they applied is stopped unless `leave`. */
export const revoke = (id: string, leave: boolean) =>
  call<Revoking>('DELETE', `/credentials/${encodeURIComponent(id)}${leave ? '?leave=true' : ''}`);
/** Gives the key `id` another secret: the new token, shown once. */
export const roll = (id: string) =>
  call<{ token: string; id: string; name: string; scopes: string[] }>('POST', `/credentials/${encodeURIComponent(id)}/roll`, {});

/** One instance as a line: its blueprint, where, and its id. */
export function appliedLine(one: Applied): string {
  const where = one.project ? ` in ${one.project}` : '';
  return `${one.blueprint}${where} (${one.id})`;
}

/** What goes with the key, a line for each thing, as they would read before saying yes. */
export function goesWith(found: Revoking, leave: boolean): string[] {
  const keys = found.keys.map((key) => `the key ${key.name}, which was made with it`);
  const running = found.running.map((one) => `${appliedLine(one)}, ${leave ? 'left running' : 'stopped'}`);
  return [...keys, ...running];
}

/** What is said once it is revoked, in a line. */
export function revokedSaid(done: Revoking): string {
  const keys = done.keys.length === 0 ? '' : ` with ${done.keys.length === 1 ? 'one key' : `${done.keys.length} keys`} made with it`;
  const stopped = done.stopped?.length ?? 0;
  const left = done.running.length;
  const what =
    stopped > 0 ? `; ${stopped === 1 ? 'one thing' : `${stopped} things`} it applied stopped` : '';
  const still = left > 0 ? `; ${left === 1 ? 'one thing' : `${left} things`} it applied still running` : '';
  return `${done.name} is revoked${keys}${what}${still}.`;
}

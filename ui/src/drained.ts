// A daemon draining to stop (`mw down --drain`, `api::drain`), as the
// page says it: it begins nothing new and stops once what is in hand is
// done, by a time it says, with what it waits for and what becomes of
// each where it is cut off at that time. Read with the rest of what the
// daemon says of itself (`draining` of `/api/meta`) and followed as
// `system.draining` is said, so a page that is open when it begins says
// so with nothing read again; what it waits for is read from
// `/api/system/drain` then, and again each time a turn or a run ends. As
// it stops, `system.stopping` says whether everything in hand was done,
// or what was cut off, and the page says that.

import type { Setting } from './clock.ts';
import { timeIn } from './clock.ts';

export type Draining = { since: number; until: number };

/** A turn in hand or a run under way that a drain waits for, as `/api/system/drain` says it. */
export type Waited = { slot?: string; turn?: string; of?: string; flow?: string; becomes?: string };

/** What `draining` of the daemon's meta, or of what `system.draining` carries, comes to: nothing where it does not drain. */
export function drainingOf(said: unknown): Draining | null {
  if (!said || typeof said !== 'object') return null;
  const { since, until } = said as Record<string, unknown>;
  return typeof since === 'number' && typeof until === 'number' ? { since, until } : null;
}

/** What the page says of a daemon that drains, in the reader's time. */
export function drainSaid(draining: Draining, setting: Setting): string {
  return `The daemon is draining to stop: it begins nothing new, and stops once the work in hand is done, by ${timeIn(draining.until, setting)} at the latest. What it runs is taken up by the next start.`;
}

/** What it waits for, a line each, as `/api/system/drain` hands it; one line where nothing is in hand. */
export function waitsSaid(standing: unknown): string[] {
  const read = (standing ?? {}) as { turns?: Waited[]; runs?: Waited[] };
  const turns = (read.turns ?? []).map((one) => `${one.slot ?? 'An agent'}'s turn: if it is cut off, ${one.becomes ?? 'the next start takes it up'}.`);
  const runs = (read.runs ?? []).map((one) => `A run of ${one.of ?? 'a blueprint'} (${one.flow ?? 'main'}): if it is cut off, ${one.becomes ?? 'the next start takes it up'}.`);
  const all = [...turns, ...runs];
  return all.length ? all : ['Nothing is in hand.'];
}

/** What the page says as a daemon that drained stops (what `system.stopping` carries), or nothing of one that did not drain. */
export function stoppedSaid(data: unknown): string[] | null {
  const read = (data ?? {}) as { drained?: unknown; cut?: unknown };
  if (typeof read.drained !== 'boolean') return null;
  if (read.drained) return ['The daemon drained and stopped: nothing was left in hand. What it runs is taken up when it is started again.'];
  const cut = Array.isArray(read.cut) ? read.cut.filter((one): one is string => typeof one === 'string') : [];
  return ['The daemon stopped at its bound with this still in hand:', ...cut];
}

// A project's feature flags as the client shows them
// (`docs/environments.md`, *Feature flags*), worked out from what the
// daemon answers (`/api/projects/<p>/flags`) and nothing else: a row a
// flag and a cell an environment, what pressing a cell asks for, whether
// whoever looks may, and what an answer is said as. What a value may be,
// and whether a change waits for a go-ahead, is the daemon's
// (`kernel::flags`); what is read here as a value is only so that what
// cannot be one is said before anything is asked.

import { covers } from './holds.ts';

/** What a flag says in one environment, as the daemon answers it: nothing, where it says nothing there. */
export interface Says {
  value: string | number | null;
  shown: string;
}

export type Kind = 'switch' | 'percentage' | 'word';

/** A flag, as the daemon answers it. */
export interface Flag {
  id: string;
  name: string;
  flag: string;
  variable: string;
  about: string;
  owner: string;
  until: string;
  kind: Kind;
  says: Record<string, Says>;
}

/** What the daemon answers of a project's flags. */
export interface Listed {
  flags: Flag[];
  environments: string[];
  /** A flag's variable no row names any more, where it is still set. */
  left: { environment: string; variable: string; value: unknown }[];
}

/** What the daemon answers to a change of one. */
export interface Came {
  came: 'set' | 'same' | 'asks';
  flag: string;
  environment: string;
  shown?: string;
  said?: string;
}

/** Whether whoever holds `scopes` may change what a flag says: writing the project. */
export const mayChangeFlags = (scopes: string[]): boolean => covers(scopes, 'projects:write');

/** What pressing a switch's cell asks for: off where it is on, and on otherwise. Anything else asks what to say. */
export function pressed(flag: Flag, environment: string): string | null {
  if (flag.kind !== 'switch') return null;
  return flag.says[environment]?.value === 'on' ? 'off' : 'on';
}

/** What is typed for a flag of `kind`, as it is sent; or why it cannot be sent, in a sentence. */
export function typedValue(kind: Kind, typed: string): { value: string | number } | { wrong: string } {
  const text = typed.trim();
  if (kind === 'switch') {
    const word = text.toLowerCase();
    return word === 'on' || word === 'off' ? { value: word } : { wrong: 'A switch is on or off.' };
  }
  if (kind === 'percentage') {
    const number = text.replace(/%$/, '').trim();
    const whole = /^\d{1,3}$/.test(number) ? Number(number) : NaN;
    return whole >= 0 && whole <= 100 ? { value: whole } : { wrong: 'A percentage is a whole number from 0 to 100.' };
  }
  return /^[a-z0-9._-]{0,40}$/.test(text) ? { value: text } : { wrong: 'A word is lowercase letters, digits, -, _ and . (40 at most).' };
}

/** What an answer to a change is said as, in a line. */
export function cameSaid(came: Came, name: string): string {
  if (came.came === 'same') return `${name} says that in ${came.environment} already.`;
  if (came.came === 'asks') return came.said ? `${came.said}.` : `Setting ${name} in ${came.environment} waits for a go-ahead.`;
  return `${name} is ${came.shown ?? ''} in ${came.environment}.`;
}

/** Whether a flag is past the day it was to be taken out, on `today` (`2026-11-24`). */
export const pastItsDay = (flag: Flag, today: string): boolean => /^\d{4}-\d{2}-\d{2}/.test(flag.until) && flag.until.slice(0, 10) < today;

/** The flags as they are said in one environment, for its drawer: each with what it says there. */
export const saidIn = (listed: Listed, environment: string): { name: string; variable: string; shown: string; on: boolean }[] =>
  listed.flags.map((flag) => {
    const says = flag.says[environment];
    return { name: flag.name, variable: flag.variable, shown: says?.shown ?? 'off', on: says?.value !== null && says?.value !== undefined && says.value !== 'off' && says.value !== 0 && says.value !== '' };
  });

// What every page of a factory says across its top where it is not the
// factory somebody would take it for (`folders::across`): a
// copy of another, with its profile, what it is a copy of and when; or
// what it was told to say there. A copy is easily taken for the real
// factory.
//
// The daemon says what it is made of, and the words are put together
// here so that the time is in the reader's zone, as every time the client
// draws is (`clock.ts`). Holds nothing of the page, so that its tests can
// read it.

import { whenIn } from './clock.ts';
import type { Setting } from './clock.ts';
import { covers } from './holds.ts';

/** What the daemon says of it, as `across` of its meta and of `system.across`. */
export interface Across {
  kind: 'said' | 'copy';
  says?: string;
  of?: string;
  device?: string;
  made?: number;
  profile?: string;
  words: string;
}

/** What is said, or nothing: where the daemon says nothing, or says something this page does not read. */
export function acrossOf(said: unknown): Across | null {
  if (typeof said !== 'object' || said === null) return null;
  const one = said as Partial<Across>;
  return one.kind === 'said' || one.kind === 'copy' ? (one as Across) : null;
}

/** The line in words: a copy's when in the reader's zone, and anything else as the daemon words it. */
export function acrossSaid(across: Across, setting: Setting, now = Date.now()): string {
  if (across.kind !== 'copy' || across.made === undefined) return across.words;
  const of = across.of ?? `the factory on ${across.device ?? 'another device'}`;
  const named = across.profile ? `Profile: ${across.profile}. ` : '';
  return `${named}A copy of ${of} taken ${whenIn(across.made, setting, now)}: what is done here stays here.`;
}

/** Whether whoever reads it may say it may stop: whoever manages the factory, as the daemon asks. */
export function mayQuiet(scopes: string[]): boolean {
  return covers(scopes, 'admin');
}

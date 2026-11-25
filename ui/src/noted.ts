// What an instance has to say beside its status, as Activity lists it:
// the sentence its row carries (`error`), and whether that is a failure or
// something it went on past.
//
// An instance that failed says why, and that is a failure. One that is
// still there, or ended some other way, and carries a sentence went on
// past it: an update it could not take (`std/looked` cannot be applied
// here as it is now, so `wrong` went on with it as it was applied), which
// is a warning and never drawn as an error. A row with nothing to say has
// no note.

import type { Instance } from './api.ts';

export interface Note {
  /** The sentence whole, as the instance's row says it. */
  says: string;
  /** A failure, or a warning it went on past. */
  is: 'failure' | 'warning';
}

export function noteOf(instance: Pick<Instance, 'status' | 'error'>): Note | null {
  const says = instance.error?.trim();
  if (!says) return null;
  return { says, is: instance.status === 'failed' ? 'failure' : 'warning' };
}

/** Whether any of the rows has something to say, which is where the column for it is drawn. */
export function anyNoted(instances: Pick<Instance, 'status' | 'error'>[]): boolean {
  return instances.some((one) => noteOf(one) !== null);
}

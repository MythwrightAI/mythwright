// Which of a blueprint's views is shown once an apply of it has answered.
//
// An apply answers when the blueprint's steps have run, and the graph has
// drawn them going and done meanwhile, as each was said: so the answer
// comes after the page already shows what it is the answer to, by no time
// at all on a quiet machine and by a second on a busy one. Whoever has
// pressed another of its views in that while has chosen what they are
// reading. The page used to show the graph when the answer came, whatever
// was showing by then: somebody who had gone on to the runs was taken back
// from them, with nothing said.

import type { Applied } from './api.ts';
import { named } from './running.ts';

/** A blueprint's views. */
export type Mode = 'summary' | 'text' | 'node' | 'runs' | 'history' | 'sandbox' | 'beside';

/**
 * The view to show when an apply has answered, or `null` to leave what is
 * showing: `said` is how many things the apply said beside going ahead,
 * which are listed under the text and nowhere else; `chose` is whether
 * somebody has pressed one of its views since the apply was asked for.
 * What they chose stays, and what was said is still told them where they
 * are (the page says so in passing, and that it is under the text).
 */
export function shownOnceApplied(said: number, chose: boolean): Mode | null {
  if (chose) return null;
  return said > 0 ? 'text' : 'node';
}

/**
 * What is said once an apply has answered: started, updated, or already
 * running what it was asked to, `there` saying for which project where
 * that is another than the page's. What is already running is an
 * instance, and is said as everything else says one (`named`: its
 * blueprint and its key); `name` is the blueprint the page last checked,
 * which is all a daemon from before gave to go by. The page used to say
 * that alone, which is no instance, and `null` where its check had no
 * name to give.
 */
export function appliedSaid(applied: Pick<Applied, 'created' | 'changed' | 'blueprint' | 'key'>, name: string | null, there: string): string {
  const blueprint = applied.blueprint ?? name;
  if (applied.created) return `Started ${blueprint ?? 'it'}${there}.`;
  if (applied.changed) return `Updated ${blueprint ?? 'it'}${there}.`;
  const which = blueprint && applied.key ? named({ blueprint, key: applied.key }) : blueprint;
  return which ? `${which} is already running this${there}; nothing to do.` : `What is running is already running this${there}; nothing to do.`;
}

// A word shown working, as the page has it: a scenario the daemon hands
// with the language (`readScenarios`), run in a sandbox in the drawer.
//
// Nothing of what happened is worked out here: a scenario's acts are
// sent as the daemon wrote them, and what came of them is the sandbox's
// own answer, drawn as the Sandbox view draws one. This is what each act
// is said as in the list of what was done, and what a scenario's
// sandbox is kept by. Nothing of the page is in it, so that its cases
// can read it.

import { brief } from './gave.ts';
import { nth } from './sandbox.ts';

/** One thing a scenario does, as a sandbox's acts take it: `{ "say": { topic, data } }` and the rest. */
export type Done = Record<string, unknown>;

const fieldsOf = (value: unknown): Record<string, unknown> => (value && typeof value === 'object' && !Array.isArray(value) ? (value as Record<string, unknown>) : {});
const some = (value: unknown): string => {
  const said = brief(value);
  return said ? ` ${said}` : '';
};

/** What a scenario did, in a line of words, with what is code in backticks. */
export function doneSays(act: Done): string {
  const [kind, said] = Object.entries(act)[0] ?? ['', {}];
  const with_ = fieldsOf(said);
  switch (kind) {
    case 'apply': {
      const params = fieldsOf(with_.params);
      return Object.keys(params).length === 0 ? 'Applied it' : `Applied it, told${some(params)}`;
    }
    case 'say':
      return `Said \`${String(with_.topic ?? '')}\`${some(with_.data)}`;
    case 'do':
      return `Did \`${String(with_.action ?? '')}\`${some(with_.with)}`;
    case 'advance':
      return `Moved the clock on ${String(with_.by ?? '')}`;
    case 'answer': {
      const which = `the ${nth(Number(with_.asked ?? 0))} stand-in`;
      return typeof with_.fails === 'string' ? `Answered ${which}: it fails, ${with_.fails}` : `Answered ${which}:${some(with_.gives) || ' it went well'}`;
    }
    case 'answers':
      return with_.answers === 'default' ? 'Had every stand-in answer as usual from here on' : 'Had every stand-in wait to be answered from here on';
    default:
      return `Did ${JSON.stringify(act)}`;
  }
}

let opened = 0;

/**
 * What the sandbox of one opening of a scenario is kept by, among the
 * sandboxes kept by a tab's key (`sandboxkept.ts`): its own each time, so
 * that one opened again is never taken for the one before, which is
 * being ended.
 */
export function scenarioKey(): string {
  opened += 1;
  return `scenario:${opened}`;
}

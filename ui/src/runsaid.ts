// What a run's steps, and a run in the list of runs, are shown as.
//
// A run names its steps by their keys, which is what the text, the logs
// and everything kept call them: a step called `wrong` that asks whether
// anything is wrong, and found nothing, read as bad news that went well.
// Most steps of what ships say what they are for, in a `description`,
// and the graph's tile already says a step by that (`graph/tile.ts`). A
// run's line says it the same way, by the same rule, with the key beside
// it, smaller: nothing is renamed.
//
// What a step says of itself is in its text and not in its run, and the
// page has one text drawn: the one it shows. A run is of the text its
// instance had when it began, which it names (`revision`), and only a
// run of the very text that is drawn is said by that text's words: a key
// may mean something else in another text, so a run of any other is
// shown by its keys, as every run was. Nothing is asked of the daemon
// for it.

import type { Graph } from './api.ts';
import type { Run } from './gave.ts';
import { says } from './graph/tile.ts';
import { baseStep } from './util.ts';

/** A text as the page has it drawn: its graph, and which text that is. */
export interface Drawn {
  graph: Pick<Graph, 'nodes'> | null;
  revision: string;
}

/**
 * What a step of a run says of itself, as far as its first full stop;
 * nothing where it says nothing, where the text is not drawn, or where
 * the run is of another text than the one that is.
 */
export function stepSays(drawn: Drawn | null | undefined, run: Pick<Run, 'flow' | 'revision'>, step: string): string {
  if (!drawn?.graph || drawn.revision !== run.revision) return '';
  // One of a list is called after its step with which of them it is (`each[2]`): they are all that step.
  const id = `${run.flow}/${baseStep(step)}`;
  const node = drawn.graph.nodes.find((one) => one.id === id && one.kind !== 'trigger');
  return node ? says(node.description) : '';
}

/** What a run is of, in words, and what its text calls that where the words are not it. */
export interface Of {
  words: string;
  key: string | null;
}

/**
 * What a run is of, for a line of the list of runs. A text's own steps
 * are said in words, with what they are called under them; a handler's
 * run is called by its handler, which has nowhere to say what it is for.
 */
export function runOf(flow: string): Of {
  if (flow === 'main') return { words: 'Its steps', key: 'main' };
  return { words: flow.replace('on.', ''), key: null };
}

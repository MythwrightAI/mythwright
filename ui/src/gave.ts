// What a step gave out, in a few words: for the line of a step in a run's
// list, where the whole of it would not fit and is opened on a press.
//
// It is said from what the daemon handed with the run and nothing else
// is asked for it: what a step gives out is already kept without what it
// was handed in secret. The shapes of a run and of its steps as they are
// handed are here too, and the daemon's API gives them out under its own
// name (`api.ts`, which was at its ceiling of lines).

import type { Json } from './api.ts';
import { hasContent } from './util.ts';

/** A step of a run, as the daemon hands it with the run. */
export interface Step {
  run: string;
  step: string;
  status: 'running' | 'ok' | 'failed' | 'skipped' | 'cancelled';
  attempt: number;
  tolerated: boolean;
  output: Json;
  error: string | null;
  started: number;
  finished: number | null;
}
/** A run, as the daemon hands it. */
export interface Run {
  id: string;
  instance: string;
  flow: string;
  revision: string;
  status: 'running' | 'ok' | 'failed' | 'cancelled';
  event: Json;
  outputs: Json;
  error: string | null;
  started: number;
  finished: number | null;
  steps?: Step[];
  /** What set it off is kept from whoever reads it: what that carried, what the run and its steps gave and why any failed are left out, and its steps' logs are refused. */
  kept_from?: boolean;
  /** What its steps printed, and what was said of each as it went, was let go after the days the factory keeps it, in the daemon's words. */
  let_go?: string;
}

/** No more of it than this is said in a line. */
const MOST = 140;

function cut(text: string): string {
  return text.length > MOST ? `${text.slice(0, MOST - 1)}…` : text;
}

/** One thing a step gave, as short as it can be said: words as their first line, anything inside something else by how much of it there is. */
function word(value: unknown, inside: boolean): string {
  if (value === null || value === undefined) return inside ? 'nothing' : '';
  if (typeof value === 'string') {
    const lines = value.split('\n').filter((line) => line.trim() !== '');
    const first = (lines[0] ?? '').trim();
    return lines.length > 1 ? `${first} …` : first;
  }
  if (typeof value !== 'object') return String(value);
  if (Array.isArray(value)) {
    if (value.length === 0) return inside ? 'none' : '';
    const plain = value.every((one) => one === null || typeof one !== 'object');
    if (plain && !inside) return value.map((one) => word(one, true)).join(', ');
    return value.length === 1 ? '1 thing' : `${value.length} things`;
  }
  const entries = Object.entries(value as Record<string, unknown>);
  if (entries.length === 0) return inside ? 'nothing' : '';
  if (inside) return entries.length === 1 ? `${entries[0]![0]}: ${word(entries[0]![1], true)}` : `${entries.length} things`;
  return entries.map(([name, one]) => `${name}: ${word(one, true)}`).join(', ');
}

/** What a step gave out, in a line; nothing for one that gave nothing. */
export function brief(output: unknown): string {
  return hasContent(output) ? cut(word(output, false)) : '';
}

/** What is said of a step where its result is: why it failed, or what it gave. */
export interface Result {
  /** In a line. */
  says: string;
  /** It is why the step failed. */
  failed: boolean;
  /** Whether there is more of it than the line: the whole is then opened on a press. */
  whole: 'error' | 'output' | null;
}

/** What a step's line says of how it came out; nothing for one that went well and gave nothing. */
export function resultOf(step: Pick<Step, 'error' | 'tolerated' | 'output'>): Result | null {
  if (step.error) return { says: step.error.split('\n')[0] ?? step.error, failed: true, whole: 'error' };
  const gave = brief(step.output);
  if (gave) return { says: step.tolerated ? `failed, tolerated · ${gave}` : gave, failed: false, whole: 'output' };
  return step.tolerated ? { says: 'failed, tolerated', failed: false, whole: null } : null;
}

/** Whether any step of a run has anything to say of how it came out: where none has, the list has no place for it. */
export function anyResult(steps: Pick<Step, 'error' | 'tolerated' | 'output'>[]): boolean {
  return steps.some((step) => resultOf(step) !== null);
}

/**
 * What is said over the steps of a run whose reader may not be told what set it off; nothing for a run that is read whole.
 * The daemon hands such a run with what its steps gave and why any failed left out, and refuses their logs:
 * said nowhere, the list read as steps that gave nothing and wrote nothing.
 */
export function keptFrom(run: Pick<Run, 'kept_from'>): string | null {
  if (!run.kept_from) return null;
  return 'What set this run off is kept from you, so what its steps gave, why any failed and what they wrote are not shown.';
}

/** What is said over the steps of a run whose steps' detail was let go, as the daemon says it; nothing for one whose detail is kept. */
export function letGo(run: Pick<Run, 'let_go'>): string | null {
  if (!run.let_go) return null;
  return `${run.let_go.charAt(0).toUpperCase()}${run.let_go.slice(1)}.`;
}

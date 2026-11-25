// A run as a pipeline run: its stages, worked out from the run's steps and
// the graph of the text it ran. Functions of plain values, with their
// cases in `staged.test.ts`; what is drawn is `views/pipelinerun.tsx`.
//
// A pipeline is a blueprint, and every run of every blueprint is a
// pipeline run: nothing here is of one kind of blueprint. A stage is what
// a step at the top of the run's flow is: a `task` (the steps inside it,
// which a run names `build.compile`), a `call` or a `spawn` (whose inside
// is the child's run), or a lone step, a stage of one. "Stage" is the
// client's word only: the language and the runner say steps and tasks.
//
// The structure is read from the run's steps by their names, so that a
// run of a text the page cannot read again is still shown in its stages.
// The graph, where it is of the very text the run ran (`revision`), adds
// what each stage says it is for, what kind it is, the order of its
// layers, and the stages not come to yet.

import type { Graph, Step } from './api.ts';
import { says } from './graph/tile.ts';
import { baseStep } from './util.ts';

/** How a stage stands: a step's, or `waiting` for one not come to while the run goes on, `not run` for one never come to. */
export type Standing = Step['status'] | 'waiting' | 'not run';

export interface Stage {
  /** What the text calls it: the step at the top of the flow. */
  key: string;
  /** What it says it is for, as far as its first full stop; nothing where it says nothing or the text is not drawn. */
  says: string;
  kind: 'task' | 'call' | 'spawn' | 'step';
  status: Standing;
  /** Failed and let fail (`allow_failure`): the run went on. */
  tolerated: boolean;
  started: number | null;
  finished: number | null;
  /** Its own row, where it has one: a task's, a call's, a lone step's. */
  head: Step | null;
  /** The steps inside it, in the order they began: a task's, or each of a list. A lone step has none but itself. */
  inside: Step[];
  /** The instance a `call` or `spawn` started, where it has said so. */
  child: string | null;
  /** Its share of the run's time, from 0 to 1. */
  share: number;
  /** Its column of the graph, where the text is drawn; stages of one layer run side by side. */
  layer: number | null;
  /** In the same layer as the stage before it: begun beside it, not after it. */
  beside: boolean;
}

/** What of a run the stages are worked out from. */
export interface Ran {
  flow: string;
  revision: string;
  started: number;
  finished: number | null;
  status: string;
  steps?: Step[];
}

/** The stage a step of a run is in: the step at the top of its flow, without which of a list it is. */
export function stageOf(step: string): string {
  return baseStep(step.split('.')[0] ?? step);
}

/** The child a step that starts one says it started, where it has. */
export function childOf(step: Pick<Step, 'output'> | null): string | null {
  const output = step?.output;
  if (typeof output !== 'object' || output === null || Array.isArray(output)) return null;
  const instance = (output as Record<string, unknown>).instance;
  return typeof instance === 'string' && instance.startsWith('in_') ? instance : null;
}

/** How several steps stand together: failed where one failed and was not let fail, under way where one is, else as the last. */
function together(steps: Step[]): { status: Standing; tolerated: boolean } {
  if (steps.some((one) => one.status === 'running')) return { status: 'running', tolerated: false };
  const failed = steps.filter((one) => one.status === 'failed');
  if (failed.some((one) => !one.tolerated)) return { status: 'failed', tolerated: false };
  if (failed.length > 0) return { status: 'failed', tolerated: true };
  if (steps.some((one) => one.status === 'cancelled')) return { status: 'cancelled', tolerated: false };
  if (steps.length > 0 && steps.every((one) => one.status === 'skipped')) return { status: 'skipped', tolerated: false };
  return { status: 'ok', tolerated: false };
}

/**
 * The stages of a run, in the order they run: by the graph's layers
 * where the text is drawn, those of one layer in the order the text says
 * them, else in the order their first steps began.
 * `graph` is the graph of the text the run ran, and nothing where that
 * text cannot be had; `now` is when a stage under way is measured to.
 */
export function stagesOf(run: Ran, graph: Pick<Graph, 'nodes'> | null, now: number): Stage[] {
  const steps = run.steps ?? [];
  const nodes = (graph?.nodes ?? []).filter((one) => one.group === run.flow && one.kind !== 'trigger' && !one.name.includes('.'));
  const byName = new Map(nodes.map((one) => [one.name, one]));
  const order: string[] = [];
  const of = new Map<string, Step[]>();
  for (const step of steps) {
    const key = stageOf(step.step);
    if (!of.has(key)) {
      of.set(key, []);
      order.push(key);
    }
    of.get(key)!.push(step);
  }
  // What the text has that nothing of the run has come to yet.
  for (const node of nodes) if (!of.has(node.name)) order.push(node.name);
  const over = run.status !== 'running';
  const whole = Math.max(1, (run.finished ?? now) - run.started);
  const stages = order.map((key, at) => {
    const rows = of.get(key) ?? [];
    const node = byName.get(key);
    const head = rows.find((one) => one.step === key) ?? null;
    const inside = rows.filter((one) => one !== head);
    const stood = head ? { status: head.status as Standing, tolerated: head.tolerated } : rows.length > 0 ? together(rows) : { status: (over ? 'not run' : 'waiting') as Standing, tolerated: false };
    const started = rows.length > 0 ? Math.min(...rows.map((one) => one.started)) : null;
    const ended = rows.length > 0 && rows.every((one) => one.finished !== null) ? Math.max(...rows.map((one) => one.finished!)) : null;
    const child = childOf(head);
    const kind: Stage['kind'] = node ? (node.kind === 'task' || node.kind === 'call' || node.kind === 'spawn' ? node.kind : 'step') : inside.some((one) => one.step.includes('.')) ? 'task' : child ? 'call' : 'step';
    const took = started === null ? 0 : (ended ?? now) - started;
    return {
      stage: {
        key,
        says: node ? says(node.description) : '',
        kind,
        status: stood.status,
        tolerated: stood.tolerated,
        started,
        finished: ended,
        head,
        inside,
        child,
        share: Math.min(1, Math.max(0, took / whole)),
        layer: node ? node.layer : null,
        beside: false,
      } satisfies Stage,
      at,
    };
  });
  // Of one layer, in the order the text says them, which is the same however they happened to begin.
  const written = (key: string) => {
    const at = nodes.findIndex((one) => one.name === key);
    return at < 0 ? Infinity : at;
  };
  if (graph) stages.sort((a, b) => (a.stage.layer ?? Infinity) - (b.stage.layer ?? Infinity) || written(a.stage.key) - written(b.stage.key) || a.at - b.at);
  return stages.map(({ stage }, at, all) => ({ ...stage, beside: at > 0 && stage.layer !== null && all[at - 1]!.stage.layer === stage.layer }));
}

/** What a stage is called on its band: what it says it is for, else what the text calls it. */
export function stageCalled(stage: Pick<Stage, 'says' | 'key'>): string {
  return stage.says || stage.key;
}

/** How a stage's time is said: how long it took, how long it has gone on, or nothing where it has not begun. */
export function stageTook(stage: Pick<Stage, 'started' | 'finished'>, now: number, took: (ms: number) => string): string {
  if (stage.started === null) return '';
  return stage.finished === null ? `${took(now - stage.started)} so far` : took(stage.finished - stage.started);
}

/** The run a `call` stage unfolds into: of the child's runs, its own steps' latest that began once the stage did. */
export function childRun<R extends { flow: string; started: number }>(runs: R[], since: number | null): R | null {
  const own = runs.filter((one) => one.flow === 'main' && (since === null || one.started >= since - 1000));
  return own.reduce<R | null>((latest, one) => (latest === null || one.started > latest.started ? one : latest), null);
}

/** Which child a stage that starts one started, where the step has not said so yet: the first made once it began. */
export function childAt<I extends { id: string; created: number }>(children: I[], since: number | null): string | null {
  if (since === null) return null;
  const after = children.filter((one) => one.created >= since).sort((a, b) => a.created - b.created);
  return after[0]?.id ?? null;
}

/**
 * How many lines of a step's output are drawn at first, and how many more
 * each press of Show earlier asks for: a log of sixteen megabytes is read
 * and drawn from its end, never whole.
 */
export const WINDOW = 500;

/** What of a step's output is drawn: no more than the last `most` lines read, so that lines written as it goes never make it grow without end. */
export function drawnOf<L extends { line: number }>(lines: L[], most: number): L[] {
  return lines.length > most ? lines.slice(lines.length - most) : lines;
}

/** How many lines come before the first that is drawn, which Show earlier asks for: none where the first is the first. */
export function earlierThan(lines: { line: number }[]): number {
  return Math.max(0, (lines[0]?.line ?? 1) - 1);
}

/** What Show earlier asks for of what comes before the first line drawn: the `page` lines just before it, or as many as there are. */
export function earlierAsked(first: number, page: number): { after: number; limit: number } | null {
  if (first <= 1) return null;
  const after = Math.max(0, first - 1 - page);
  return { after, limit: first - 1 - after };
}

/** What Show earlier says: how many it draws, and how many come before them still. */
export function earlierSaid(before: number, page: number): string {
  const shown = Math.min(before, page);
  const left = before - shown;
  const lines = (count: number) => `${count.toLocaleString('en')} ${count === 1 ? 'line' : 'lines'}`;
  if (left > 0) return `Show ${lines(shown)} earlier (${lines(left)} before them)`;
  return shown === 1 ? 'Show the line before these' : `Show the ${lines(shown)} before these`;
}

/**
 * The stage whose failure is the run's, where the run failed: the last
 * that failed and was not let fail. It is marked, and brought into view
 * as the run's page is first read, so that whoever opens a run that
 * failed (from a channel that said so, from Activity) reads the step
 * that failed, what it wrote and why first. Nothing, for a run that did
 * not fail or one that failed with no stage that did (it could not
 * begin, or was cut off between two).
 */
export function failedAt(run: Pick<Ran, 'status'>, stages: Pick<Stage, 'key' | 'status' | 'tolerated'>[]): string | null {
  if (run.status !== 'failed') return null;
  return stages.filter((one) => one.status === 'failed' && !one.tolerated).at(-1)?.key ?? null;
}

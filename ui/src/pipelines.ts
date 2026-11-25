// A project's pipelines: its own blueprints kept under `pipelines/` in
// the factory's place for them (`@blueprints/pipelines/<name>.bp`, called
// `pipelines/<name>`). A pipeline is a blueprint, and the folder says only
// which of a project's are listed first and offered Run; every run of
// every blueprint is still shown as a pipeline run (`staged.ts`). These
// are functions of plain values, with their cases in `pipelines.test.ts`;
// what is drawn is `views/pipelines.tsx`.

import { blueprintNamed } from './ownmade.ts';
import type { Made } from './ownmade.ts';
import type { Uses } from './uses.ts';

/** What every pipeline's name begins with. */
export const FOLDER = 'pipelines/';

/** How many of a pipeline's latest runs its row shows. */
export const LATEST = 6;

export interface Pipeline {
  /** As anything calls it: `pipelines/check`. */
  name: string;
  /** The file it is, as a path of the project's. */
  file: string;
}

/** Whether a blueprint is one of a project's pipelines by its name. */
export function isPipeline(name: string): boolean {
  return name.startsWith(FOLDER) && name.length > FOLDER.length;
}

/** A project's pipelines, by name: its own blueprints under `pipelines/`, and nothing that ships or that it only applies. */
export function pipelinesOf(uses: Uses | undefined): Pipeline[] {
  return (uses?.own ?? [])
    .filter((one) => isPipeline(one.name))
    .map((one) => ({ name: one.name, file: one.file }))
    .sort((a, b) => a.name.localeCompare(b.name));
}

/** Everything else applied for the project, by name: what runs there that is no pipeline of its own, the fleet's rounds and a deploy among it. */
export function othersOf(uses: Uses | undefined): string[] {
  return [...new Set(uses?.applied ?? [])].filter((name) => !isPipeline(name)).sort((a, b) => a.localeCompare(b));
}

/** A pipeline's name as its row says it: what comes after the folder. */
export function shortName(name: string): string {
  return isPipeline(name) ? name.slice(FOLDER.length) : name;
}

/**
 * The pipeline a name written in the dialog makes, in `written` (where
 * the project's own are written): under `pipelines/` whether or not that
 * was written, by the rules every blueprint of a project's own is named by.
 */
export function pipelineNamed(name: string, written: string, has: string[], ships: string[]): Made {
  const typed = name.trim();
  if (typed === '') return { wrong: 'Give it a name.' };
  return blueprintNamed(typed.startsWith(FOLDER) ? typed : `${FOLDER}${typed}`, written, has, ships);
}

/**
 * What a new pipeline says as it is made: a stage of one step that does
 * nothing but say so, so that it checks and runs as it is, and shows how
 * a stage is written (a `task` with its steps).
 */
export function firstPipeline(name: string): string {
  return [
    `blueprint: ${name}`,
    'description: What this pipeline does, in a sentence.',
    'steps:',
    '  build:',
    '    description: Build it.',
    '    task:',
    '      first: { do: log, with: { message: Each stage is a task with its steps under it. } }',
    '',
  ].join('\n');
}

/** Whether Run asks first, in the drawer: where the pipeline is handed something, or the project runs everything under an environment. */
export function asksFirst(params: number, environments: string[]): boolean {
  return params > 0 || environments.length > 0;
}

/** What keeps a project's triggers, which runs its pipelines as their rows say. */
export const TRIGGERS = 'std/pipeline';

/**
 * Whether Run asks the project's triggers for a run (`pipeline.asked`,
 * said as whoever presses, which the triggers answer as a row that runs
 * it by hand says, under its key and its environment, after asking what
 * whoever pressed holds) rather than applying the pipeline as they do:
 * where the triggers are kept for the project, and only there.
 */
export function asksTriggers(uses: Uses | undefined): boolean {
  return (uses?.applied ?? []).includes(TRIGGERS);
}

/**
 * What the triggers are told by Run: the project, the pipeline, what it is
 * handed, and what this press named its asking (`asked`), which the run
 * the triggers begin for it is found by (`/api/pipelines/begun`) and a
 * refusal of it says: two presses at one moment, from two pages or one,
 * each open their own run and never the other's.
 */
export function askedOf(
  project: string,
  pipeline: string,
  values: Record<string, unknown>,
  asked: string,
): { project: string; pipeline: string; with: Record<string, unknown>; asked: string } {
  return { project, pipeline, with: values, asked };
}

/** Whether what the triggers said of a refusal (`pipeline.refused`) is of the asking `asked`. */
export function refusedOf(said: { asked?: unknown } | undefined, asked: string): boolean {
  return asked !== '' && said?.asked === asked;
}

/** One of a pipeline's latest runs, as its row marks it. */
export interface Mark {
  run: string;
  status: string;
  started: number;
}

/** A pipeline's latest runs, the newest first and no more than its row shows: of each instance it ran as, that instance's own steps' run. */
export function latestOf<R extends { id: string; flow: string; status: string; started: number }>(runs: R[][]): Mark[] {
  return runs
    .flatMap((ofOne) => ofOne.filter((one) => one.flow === 'main'))
    .sort((a, b) => b.started - a.started)
    .slice(0, LATEST)
    .map((one) => ({ run: one.id, status: one.status, started: one.started }));
}

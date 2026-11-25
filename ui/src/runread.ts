// What a run's page and a project's Pipelines page read of the daemon,
// beyond what the client's first shapes of it ask for (`api.ts`, which
// is at its ceiling): a step's output from its end, and what came before
// that a page at a time; the
// instances of one blueprint; and the graph of the very text a run ran.
// Nothing here keeps or follows anything: the views do.

import { signal } from '@preact/signals';
import { api, call } from './api.ts';
import type { Graph, Instance, LogLine, Run } from './api.ts';

const id = encodeURIComponent;

/** Some lines of a step's output, and where its beginning was let go, in the daemon's words. */
export interface Lines {
  lines: LogLine[];
  let_go: string | null;
}

/** The last `most` lines a step wrote that are kept, each with where it is in the whole (`Logs::tail`, through the run's log route). */
export function tailLog(run: string, step: string, most: number): Promise<Lines> {
  return call<{ lines: LogLine[]; let_go?: string | null }>('GET', `/runs/${id(run)}/steps/${id(step)}/log?tail=${most}`).then((page) => ({ lines: page.lines, let_go: page.let_go ?? null }));
}

/** The `limit` lines a step wrote after line `after` (`Logs::read`): what came before what is drawn. */
export function logBetween(run: string, step: string, after: number, limit: number): Promise<LogLine[]> {
  return call<{ lines: LogLine[] }>('GET', `/runs/${id(run)}/steps/${id(step)}/log?after=${after}&limit=${limit}`).then((page) => page.lines);
}

/** The instances a blueprint of a project ran as, the newest first: each Run of a pipeline is one. */
export function ranAs(project: string, blueprint: string, limit: number): Promise<Instance[]> {
  return call<{ instances: Instance[] }>('GET', `/instances?project=${id(project)}&blueprint=${id(blueprint)}&limit=${limit}`).then((answer) => answer.instances);
}

/** An instance's runs, the latest first. */
export function runsOf(instance: string, limit = 20): Promise<Run[]> {
  return api.runs({ instance, limit });
}

/** The graphs of texts already read, by revision: a text of one revision is the same text wherever it is read. */
const graphs = new Map<string, Graph | null>();

/**
 * The graph of the text a run ran: its instance's where that is the text,
 * else the version the instance had at that revision, checked as any text
 * is; nothing where it can be had neither way, and the run is then shown
 * by its steps' names.
 */
export async function graphOf(run: Pick<Run, 'instance' | 'revision'>, detail: { instance: Instance; graph: Graph | null }): Promise<Graph | null> {
  if (detail.instance.revision === run.revision) return detail.graph;
  if (graphs.has(run.revision)) return graphs.get(run.revision)!;
  const version = (await api.versions(run.instance).catch(() => [])).find((one) => one.revision === run.revision);
  const graph = version ? ((await api.check(version.source, detail.instance.project).catch(() => null))?.graph ?? null) : null;
  graphs.set(run.revision, graph);
  return graph;
}

/** What each run whose page was opened is of, by its id: what its tab is called. */
export const runsKnown = signal<Record<string, string>>({});

/** Says what a run is of, for its tab. */
export function runIs(run: string, blueprint: string): void {
  if (runsKnown.value[run] !== blueprint) runsKnown.value = { ...runsKnown.value, [run]: blueprint };
}

/** What a run's tab is called: what it is a run of, where that was read, and its id until then. */
export function runTitle(run: string): string {
  const of = runsKnown.value[run];
  return of ? `${of} run` : run;
}

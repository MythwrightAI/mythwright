// The straight chains of a long graph, each drawn as one tile that unfolds
// in place, and how a run's steps stand on the tiles drawn.
//
// A run of fifteen steps drawn a tile a step is too much to read at once.
// Most of such a graph is straight chains: a step with one step after it,
// which has no other before it. Such a chain is one thing to whoever reads
// how a run went, so in a group of many nodes (`FOLD_FROM`) each chain of
// a few steps or more (`FOLD_LEAST`) is drawn as one tile, called by what
// its keys begin with alike or else by its first step, and saying how many
// steps it holds. It is a way of drawing: nothing of the blueprint or its
// run is renamed, and a graph of a few steps is drawn as it always was.
//
// Folded, the steps after a chain move left by the columns it took, so
// the graph is narrower too: each layer is worked out again as the daemon
// works it out, each node just after the last that leads to it
// (`layered`). Unfolded, every layer is the daemon's.
//
// Functions of plain values, with their cases in `folds.test.ts`; what
// draws them is `view.tsx`.

import type { Graph, GraphEdge, GraphNode } from '../api.ts';
import { fitted, keyOf, LETTERS, says, wrapped } from './tile.ts';
import type { Room, Tile } from './tile.ts';

/** How many nodes a group of a graph has before its chains are folded: fewer, and it is drawn as it always was. */
export const FOLD_FROM = 10;
/** How many steps a chain has at the least to be folded: a tile that held two would save one and cost a press. */
export const FOLD_LEAST = 3;

/** A straight chain of steps, drawn as one tile. */
export interface Fold {
  /** Unique in the graph, and never a node's: `fold:` and its first node's id. */
  id: string;
  group: string;
  /** Its nodes' ids, in the order they run. */
  members: string[];
  /** What the text calls each of them. */
  keys: string[];
  /** What its tile says it is: what its keys begin with alike, else what its first step says it is for, else that step's key. */
  name: string;
}

/** How a step of a run went, or a fold's steps together, as a tile says it. */
export interface Ran {
  status: 'running' | 'ok' | 'failed' | 'skipped' | 'cancelled';
  started: number;
  finished: number | null;
}

/** The id of a fold whose first node is `first`. */
export function foldId(first: string): string {
  return `fold:${first}`;
}

/**
 * Whether a node can stand in a chain: a step, and not what starts a
 * handler or an attachment. Nor a task, whose own steps are drawn after
 * it and would hang off a fold that held it without them.
 */
function foldable(node: GraphNode | undefined): boolean {
  return !!node && node.kind !== 'trigger' && node.kind !== 'attachment' && node.kind !== 'task';
}

/**
 * What a few keys begin with alike, by whole words of them (split before
 * a `.`, `-` or `_`): `build.compile` and `build.link` are `build`; `table`
 * and `began` nothing.
 */
export function sharedStart(keys: readonly string[]): string {
  if (keys.length < 2) return '';
  const parts = keys.map((key) => key.split(/(?=[._-])/));
  let n = 0;
  while (parts.every((one) => n < one.length && one[n] === parts[0]![n])) n += 1;
  return parts[0]!.slice(0, n).join('');
}

/** What a fold of these nodes is called. */
export function foldName(nodes: readonly Pick<GraphNode, 'kind' | 'name' | 'description'>[]): string {
  const keys = nodes.map(keyOf);
  return sharedStart(keys) || says(nodes[0]?.description) || keys[0] || '';
}

/**
 * The straight chains of a graph that are folded: in each group of at
 * least `from` nodes, each run of steps of at least `least` where every
 * one but the last has only the next after it, and every one but the
 * first only the one before it. In the order of the graph's nodes.
 */
export function chainsOf(graph: Pick<Graph, 'nodes' | 'edges'>, from = FOLD_FROM, least = FOLD_LEAST): Fold[] {
  const byId = new Map(graph.nodes.map((node) => [node.id, node]));
  const groups = [...new Set(graph.nodes.map((node) => node.group))];
  const folds: Fold[] = [];
  for (const group of groups) {
    const members = graph.nodes.filter((node) => node.group === group);
    if (members.length < from) continue;
    const here = new Set(members.map((node) => node.id));
    const outs = new Map<string, Set<string>>();
    const ins = new Map<string, Set<string>>();
    for (const edge of graph.edges) {
      if (!here.has(edge.from) || !here.has(edge.to) || edge.from === edge.to) continue;
      if (!outs.has(edge.from)) outs.set(edge.from, new Set());
      if (!ins.has(edge.to)) ins.set(edge.to, new Set());
      outs.get(edge.from)!.add(edge.to);
      ins.get(edge.to)!.add(edge.from);
    }
    // The node after this one in its chain, where it has one.
    const next = (id: string): string | null => {
      const after = outs.get(id);
      if (!after || after.size !== 1) return null;
      const [only] = [...after] as [string];
      return ins.get(only)?.size === 1 && foldable(byId.get(id)) && foldable(byId.get(only)) ? only : null;
    };
    const after = new Set(members.map((node) => next(node.id)).filter((id): id is string => id !== null));
    const seen = new Set<string>();
    for (const node of members) {
      if (after.has(node.id) || seen.has(node.id) || !foldable(node)) continue;
      const chain = [node.id];
      seen.add(node.id);
      for (let at = next(node.id); at !== null && !seen.has(at); at = next(at)) {
        chain.push(at);
        seen.add(at);
      }
      if (chain.length < least) continue;
      const nodes = chain.map((id) => byId.get(id)!);
      folds.push({ id: foldId(node.id), group, members: chain, keys: nodes.map(keyOf), name: foldName(nodes) });
    }
  }
  return folds;
}

/** A graph as it is drawn with some of its chains folded. */
export interface Folded {
  graph: Graph;
  /** The folds drawn as one tile, by their id. */
  shut: Record<string, Fold>;
  /** The folds drawn step by step, in place. */
  open: Fold[];
}

/**
 * The graph drawn with each fold `isOpen` says is shut as one node, in
 * the place of its first step: arrows into its first step go to it, those
 * out of its last come from it, and those between its steps are not drawn.
 */
export function foldedGraph(graph: Graph, folds: readonly Fold[], isOpen: (fold: Fold) => boolean): Folded {
  const shut = folds.filter((fold) => !isOpen(fold));
  const open = folds.filter((fold) => isOpen(fold));
  if (shut.length === 0) return { graph, shut: {}, open };
  const into = new Map<string, Fold>();
  for (const fold of shut) for (const member of fold.members) into.set(member, fold);
  const drawn = (id: string) => into.get(id)?.id ?? id;

  const nodes: GraphNode[] = [];
  for (const node of graph.nodes) {
    const fold = into.get(node.id);
    if (!fold) nodes.push(node);
    else if (fold.members[0] === node.id)
      nodes.push({ ...node, id: fold.id, name: fold.name, target: '', description: null, when: null, for_each: null, timeout: null, retry: null, device: null, measure: null, hooks: [], watch: [] });
  }
  // How many columns each arrow keeps between its ends: one, past all the columns of a task's own steps where it comes from one.
  const columns = (id: string): number => {
    const from = graph.nodes.find((one) => one.id === id);
    if (!from) return 1;
    const inside = graph.nodes.filter((one) => one.group === from.group && one.name.startsWith(`${from.name}.`));
    return 1 + Math.max(0, ...inside.map((one) => one.layer - from.layer));
  };
  const edges: GraphEdge[] = [];
  const spans: number[] = [];
  const known = new Map<string, number>();
  for (const edge of graph.edges) {
    const [from, to] = [drawn(edge.from), drawn(edge.to)];
    if (from === to) continue;
    const span = edge.kind === 'starts' || edge.kind === 'fires' ? 1 : columns(edge.from);
    const key = `${from}>${to}`;
    const at = known.get(key);
    if (at !== undefined) {
      spans[at] = Math.max(spans[at]!, span);
      continue;
    }
    known.set(key, edges.length);
    edges.push({ ...edge, from, to });
    spans.push(span);
  }
  const layers = layered(nodes, edges, spans);
  return { graph: { ...graph, nodes: nodes.map((node) => ({ ...node, layer: layers.get(node.id) ?? node.layer })), edges }, shut: Object.fromEntries(shut.map((fold) => [fold.id, fold])), open };
}

/**
 * Each node's layer once some columns are gone, by the daemon's rule: a
 * node nothing leads to stays where it was, and any other is just after
 * the last of those leading to it (`spans`, by edge: one column, or a
 * task's with its steps), and never further right than it was.
 */
function layered(nodes: readonly GraphNode[], edges: readonly GraphEdge[], spans: readonly number[]): Map<string, number> {
  const was = new Map(nodes.map((node) => [node.id, node.layer]));
  const led = new Set(edges.map((edge) => edge.to));
  const layer = new Map<string, number>();
  for (const node of nodes) if (!led.has(node.id)) layer.set(node.id, node.layer);
  for (let pass = 0; pass <= nodes.length; pass += 1) {
    let changed = false;
    edges.forEach((edge, at) => {
      const from = layer.get(edge.from);
      if (from === undefined || !was.has(edge.to)) return;
      const to = Math.max(0, Math.min(was.get(edge.to)!, from + spans[at]!));
      if (to > (layer.get(edge.to) ?? -1)) {
        layer.set(edge.to, to);
        changed = true;
      }
    });
    if (!changed) break;
  }
  return layer;
}

/**
 * How several steps stand together, or a step run once for each of a
 * list: failed where one failed, under way where one is or the run goes
 * on with some not come to, cancelled where the run ended before all of
 * them ran or one was, skipped where all were, else done. Nothing where
 * none has begun.
 */
export function together(steps: readonly (Ran | undefined)[], going: boolean): Ran | undefined {
  const begun = steps.filter((one): one is Ran => one !== undefined);
  if (begun.length === 0) return undefined;
  const all = begun.length === steps.length;
  const started = Math.min(...begun.map((one) => one.started));
  const ended = begun.every((one) => one.finished !== null) ? Math.max(...begun.map((one) => one.finished!)) : null;
  const status: Ran['status'] = begun.some((one) => one.status === 'failed')
    ? 'failed'
    : begun.some((one) => one.status === 'running') || (!all && going)
      ? 'running'
      : begun.some((one) => one.status === 'cancelled') || !all
        ? 'cancelled'
        : begun.every((one) => one.status === 'skipped')
          ? 'skipped'
          : 'ok';
  return { status, started, finished: status === 'running' ? null : ended };
}

/** How each node of a run's flow stands, by node id (`<flow>/<step>`): a step run once for each of a list, as they stand together. */
export function ranOf(run: { flow: string; status: string; steps?: readonly { step: string; status: string; started: number; finished: number | null }[] }): Record<string, Ran> {
  const by = new Map<string, Ran[]>();
  for (const step of run.steps ?? []) {
    const bracket = step.step.indexOf('[');
    const id = `${run.flow}/${bracket < 0 ? step.step : step.step.slice(0, bracket)}`;
    if (!by.has(id)) by.set(id, []);
    by.get(id)!.push({ status: step.status as Ran['status'], started: step.started, finished: step.finished });
  }
  const going = run.status === 'running';
  return Object.fromEntries([...by].map(([id, steps]) => [id, steps.length === 1 ? steps[0]! : together(steps, going)!]));
}

/** A fold opens itself where a step in it failed, until whoever reads it shuts it. */
export function opensItself(fold: Pick<Fold, 'members'>, ran: Readonly<Record<string, Ran | undefined>>): boolean {
  return fold.members.some((id) => ran[id]?.status === 'failed');
}

/** What a fold's tile says: what it is called, on up to two lines, and under it how many steps it holds, from the first to the last. */
export function foldTile(fold: Pick<Fold, 'name' | 'keys'>, room: Room = LETTERS): Tile {
  return {
    words: wrapped(fold.name, [room.words, room.words], (text) => room.wide(text, false)),
    under: fitted(`${fold.keys.length} steps · ${fold.keys[0]} → ${fold.keys.at(-1)}`, room.under, (text) => room.wide(text, true)),
    said: true,
  };
}

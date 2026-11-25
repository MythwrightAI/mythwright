// A blueprint previewed: each part of its Summary as a section, and what
// each plays on its graph, worked out from the text alone.
//
// Nothing of it is written for any blueprint: there are no scenes, words
// or timings of one's own. The words are the daemon's Summary of the text
// as it is written, and what a section plays is worked out from the graph
// the same check handed beside it, so a blueprint that ships, a fork, a
// project's own and a text not yet saved are previewed alike, and the
// preview follows the text as it changes. What a section plays:
//
//   - what it does when it is applied: its steps lit in the order of what
//     waits on what (by `layer`, the longest chain of `needs` before each,
//     which the daemon works out with the graph), steps that wait on the
//     same things at once, a task lit while the steps inside it are done;
//     A step that says `when` has it asked, as written, before it is done;
//   - what it hears: for each handler, its event arriving at its tile, its
//     condition asked where it says one, then its steps in turn;
//   - what it does as it ends: each ending as what it is (its steps done,
//     the last failed, ended however they went, its new text taken, or
//     everything it was doing ended as it is stopped), and each thing it
//     then does in turn among the section's lines;
//   - what it is: the whole of it;
//   - any other part: each line that is about a tile, in turn.
//
// The frames are handed to the player (`player.ts`), which is what a
// replay of a recorded run will hand frames of its own events to.

import type { Graph, GraphNode } from './api.ts';
import type { Frame, Lit } from './player.ts';
import { stillOf } from './player.ts';
import type { Summary, SummaryLine } from './summary.ts';
import { readOrder } from './summarylaid.ts';

/** How long each kind of frame is shown, in milliseconds. */
export const LASTS = { arrives: 800, asks: 1000, runs: 900, ended: 900, shown: 1000, over: 1400 } as const;

/** A part of the Summary, as it is read in the preview and what it plays. */
export interface Section {
  kind: string;
  title: string;
  /** Its lines, in the daemon's words. */
  lines: SummaryLine[];
  /** What it plays, in order. */
  frames: Frame[];
  /** What it comes to where nothing is to move: every tile it lights, as it leaves it. */
  still: Frame;
}

/** A turn of a group's steps: what is lit, the steps it is of, and the condition asked where it is one. */
export interface Turn {
  frame: Record<string, Lit>;
  running: string[];
  /** The conditions asked, as written, where the turn is of steps asking whether they are done. */
  says: string | null;
}

/**
 * The steps of one group of the graph, in the order they are done: each
 * layer lit together, what is done kept lit as done, and a step that says
 * `when` asked first, as written, in a turn of its own before it is done.
 */
export function inTurn(graph: Pick<Graph, 'nodes'>, group: string, before: Record<string, Lit> = {}): Turn[] {
  const steps = graph.nodes.filter((node) => node.group === group && node.kind !== 'trigger');
  if (steps.length === 0) return [];
  // A task is under way for as long as the steps inside it are, which are named under it (`build.compile`).
  const inside = (task: GraphNode) => steps.filter((node) => node.name.startsWith(`${task.name}.`));
  const until = new Map(steps.map((node) => [node.id, node.kind === 'task' ? Math.max(node.layer, ...inside(node).map((one) => one.layer)) : node.layer]));
  const layers = [...new Set(steps.map((node) => node.layer))].sort((a, b) => a - b);
  return layers.flatMap((layer) => {
    const done: Record<string, Lit> = { ...before };
    for (const node of steps) if (until.get(node.id)! < layer) done[node.id] = 'done';
    // What began before this layer and is still under way (a task) stays lit as being done.
    const going = steps.filter((node) => node.layer < layer && until.get(node.id)! >= layer);
    const begun = steps.filter((node) => node.layer === layer);
    const asking = begun.filter((node) => node.when);
    const turns: Turn[] = [];
    if (asking.length > 0) {
      const frame = { ...done };
      for (const node of going) frame[node.id] = 'runs';
      for (const node of asking) frame[node.id] = 'asks';
      turns.push({ frame, running: asking.map((node) => node.id), says: asking.map((node) => node.when).join(' · ') });
    }
    const frame = { ...done };
    for (const node of [...going, ...begun]) frame[node.id] = 'runs';
    turns.push({ frame, running: [...going, ...begun].map((node) => node.id), says: null });
    return turns;
  });
}

/** What a turn plays: as long as a condition is asked, or a step is done. */
const turnLasts = (turn: Turn) => (turn.says ? LASTS.asks : LASTS.runs);

/** Every step of a group lit as done. */
function allDone(graph: Pick<Graph, 'nodes'>, group: string, before: Record<string, Lit> = {}): Record<string, Lit> {
  const frame: Record<string, Lit> = { ...before };
  for (const node of graph.nodes) if (node.group === group && node.kind !== 'trigger') frame[node.id] = 'done';
  return frame;
}

/** Where among some lines (and those under them) the deepest one about any of `ids` is, as indexes down. */
export function lineAbout(lines: readonly SummaryLine[], ids: readonly string[]): number[] | null {
  let found: number[] | null = null;
  const walk = (under: readonly SummaryLine[], path: number[]) =>
    under.forEach((line, index) => {
      const here = [...path, index];
      if (line.at && ids.includes(line.at) && (!found || here.length > found.length)) found = here;
      walk(line.under, here);
    });
  walk(lines, []);
  return found;
}

const frame = (tiles: Record<string, Lit>, line: number[] | null, lasts: number, says: string | null = null): Frame => ({ tiles, line, says, lasts });

/** What it does when it is applied: its steps in turn, then all of them done. */
function does(graph: Graph, lines: SummaryLine[]): Frame[] {
  const turns = inTurn(graph, 'main');
  if (turns.length === 0) return [];
  return [
    ...turns.map((turn) => frame(turn.frame, lineAbout(lines, turn.running), turnLasts(turn), turn.says)),
    frame(allDone(graph, 'main'), null, LASTS.over),
  ];
}

/** What it hears: for each handler its event arriving, its condition asked, its steps in turn and then done. */
function hears(graph: Graph, lines: SummaryLine[]): Frame[] {
  const frames: Frame[] = [];
  lines.forEach((line, index) => {
    if (!line.at?.endsWith('/@')) return;
    const trigger = graph.nodes.find((node) => node.id === line.at);
    if (!trigger) return;
    frames.push(frame({ [trigger.id]: 'arrives' }, [index], LASTS.arrives));
    if (trigger.when) frames.push(frame({ [trigger.id]: 'asks' }, [index], LASTS.asks, trigger.when));
    const heard: Record<string, Lit> = { [trigger.id]: 'done' };
    for (const turn of inTurn(graph, trigger.group, heard)) {
      const under = lineAbout(line.under, turn.running);
      frames.push(frame(turn.frame, under ? [index, ...under] : [index], turnLasts(turn), turn.says));
    }
    frames.push(frame(allDone(graph, trigger.group, heard), [index], LASTS.over));
  });
  return frames;
}

/**
 * What an ending comes to on the graph, by which it is (`ending` of its
 * line): its steps done where they went well; the last of them failed,
 * those before it done, where they failed; ended however they went; the
 * whole of it its new text, taken, where it takes one; and everything it
 * was doing ended, handlers and all, where it is stopped. One a daemon
 * from before says nothing of is what it was doing, ended.
 */
export function endedAs(graph: Pick<Graph, 'nodes'>, ending: string | null | undefined): Record<string, Lit> {
  const main = graph.nodes.filter((node) => node.group === 'main');
  const lit = (nodes: readonly GraphNode[], how: Lit) => Object.fromEntries(nodes.map((node) => [node.id, how])) as Record<string, Lit>;
  if (ending === 'success') return lit(main, 'done');
  if (ending === 'finish') return lit(main, 'ended');
  if (ending === 'update') return lit(graph.nodes, 'taken');
  if (ending === 'stop') return lit(graph.nodes, 'ended');
  if (ending === 'fail') {
    if (main.length === 0) return {};
    // The step that fails is the last to be done (the first of the last layer): what it waited on went well.
    const last = [...main].filter((node) => node.kind !== 'task').sort((a, b) => b.layer - a.layer)[0] ?? main[0]!;
    const frame = lit(
      main.filter((node) => node.layer < last.layer),
      'done',
    );
    // A task the step that failed is inside failed with it.
    for (const node of main) if (node.kind === 'task' && last.name.startsWith(`${node.name}.`)) frame[node.id] = 'failed';
    frame[last.id] = 'failed';
    return frame;
  }
  return lit(main.length > 0 ? main : graph.nodes, 'ended');
}

/** What it does as it ends: for each ending, the graph as that ending leaves it, and each thing it then does among its lines in turn. */
function ends(graph: Graph, lines: SummaryLine[]): Frame[] {
  const frames: Frame[] = [];
  lines.forEach((line, index) => {
    const tiles = endedAs(graph, line.ending);
    frames.push(frame(tiles, [index], LASTS.ended));
    line.under.forEach((_, under) => frames.push(frame(tiles, [index, under], LASTS.ended)));
  });
  return frames;
}

/** Any other part: each line about a tile of the graph, in the order it is read. */
function pointed(graph: Graph, lines: SummaryLine[]): Frame[] {
  const there = new Set(graph.nodes.map((node) => node.id));
  const frames: Frame[] = [];
  const walk = (under: readonly SummaryLine[], path: number[]) =>
    under.forEach((line, index) => {
      const here = [...path, index];
      if (line.at && there.has(line.at)) frames.push(frame({ [line.at]: 'shown' }, here, LASTS.shown));
      walk(line.under, here);
    });
  walk(lines, []);
  return frames;
}

/** What a part plays on a graph. */
export function framesOf(graph: Graph, kind: string, lines: SummaryLine[]): Frame[] {
  if (kind === 'does') return does(graph, lines);
  if (kind === 'hears') return hears(graph, lines);
  if (kind === 'ends') return ends(graph, lines);
  if (kind === 'is') return graph.nodes.length > 0 ? [frame(Object.fromEntries(graph.nodes.map((node) => [node.id, 'shown' as Lit])), null, LASTS.over)] : [];
  return pointed(graph, lines);
}

/**
 * The sections of a preview: every part of the Summary, in the order the
 * Summary is read, each with what it plays on the graph. Nothing where
 * there is no summary: a text that does not check has none.
 */
export function sections(graph: Graph | null | undefined, summary: Summary | null | undefined): Section[] {
  if (!graph || !summary) return [];
  return readOrder(summary.parts).map((part) => {
    const frames = framesOf(graph, part.kind, part.lines);
    return { kind: part.kind, title: part.title, lines: part.lines, frames, still: stillOf(frames) };
  });
}

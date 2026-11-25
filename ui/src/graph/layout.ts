// Where each node of a blueprint's graph is drawn.
//
// The daemon has already worked out the structure: which group a node is in
// and how far it is from the start of that group (`layer`). All that is left
// is geometry: groups are stacked top to bottom, layers run left to right,
// and nodes that share a layer are stacked in the order that keeps their
// arrows shortest.

import type { Graph, GraphEdge } from '../api.ts';

export const NODE_W = 196;
export const NODE_H = 52;
const GAP_X = 64;
const GAP_Y = 16;
const PAD = 22;
const LABEL = 22;
const GROUP_GAP = 26;

export interface Box {
  x: number;
  y: number;
  w: number;
  h: number;
}
export interface PlacedGroup extends Box {
  id: string;
  label: string;
  kind: string;
}
export interface PlacedEdge {
  from: string;
  to: string;
  kind: GraphEdge['kind'];
  d: string;
}
export interface Layout {
  nodes: Record<string, Box>;
  groups: PlacedGroup[];
  edges: PlacedEdge[];
  width: number;
  height: number;
}

/** `tall` is how tall each node is drawn: all of a graph's are alike. */
export function layout(graph: Graph, tall: number = NODE_H): Layout {
  const nodes: Record<string, Box> = {};
  const groups: PlacedGroup[] = [];
  let top = 0;
  let width = 0;

  for (const group of graph.groups) {
    const members = graph.nodes.filter((node) => node.group === group.id);
    if (members.length === 0) continue;
    const layers = new Map<number, string[]>();
    for (const node of members) {
      if (!layers.has(node.layer)) layers.set(node.layer, []);
      layers.get(node.layer)!.push(node.id);
    }
    const order = [...layers.keys()].sort((a, b) => a - b);

    // Row of each node within its layer. A node is placed near the nodes
    // that lead to it, which keeps arrows short and mostly uncrossed.
    const row = new Map<string, number>();
    for (const layer of order) {
      const ids = layers.get(layer)!;
      const wanted = new Map<string, number>();
      ids.forEach((id, index) => {
        const sources = graph.edges.filter((edge) => edge.to === id && row.has(edge.from)).map((edge) => row.get(edge.from)!);
        wanted.set(id, sources.length ? sources.reduce((sum, value) => sum + value, 0) / sources.length : index);
      });
      const sorted = [...ids].sort((a, b) => wanted.get(a)! - wanted.get(b)! || ids.indexOf(a) - ids.indexOf(b));
      // Keep a node level with its source where there is room, so a straight
      // chain is drawn as a straight line.
      let next = 0;
      for (const id of sorted) {
        const at = Math.max(next, Math.round(wanted.get(id)!));
        row.set(id, at);
        next = at + 1;
      }
    }

    const rows = Math.max(...members.map((node) => row.get(node.id)!)) + 1;
    const columns = order.length;
    const box: PlacedGroup = {
      id: group.id,
      label: group.label,
      kind: group.kind,
      x: 0,
      y: top,
      w: PAD * 2 + columns * NODE_W + (columns - 1) * GAP_X,
      h: LABEL + PAD * 2 + rows * tall + (rows - 1) * GAP_Y,
    };
    for (const node of members) {
      nodes[node.id] = {
        x: PAD + order.indexOf(node.layer) * (NODE_W + GAP_X),
        y: top + LABEL + PAD + row.get(node.id)! * (tall + GAP_Y),
        w: NODE_W,
        h: tall,
      };
    }
    groups.push(box);
    width = Math.max(width, box.w);
    top += box.h + GROUP_GAP;
  }
  for (const group of groups) group.w = width;

  const edges: PlacedEdge[] = [];
  for (const edge of graph.edges) {
    const from = nodes[edge.from];
    const to = nodes[edge.to];
    if (from && to) edges.push({ ...edge, d: path(from, to) });
  }
  return { nodes, groups, edges, width, height: Math.max(0, top - GROUP_GAP) };
}

/** A curve from the right edge of one box to the left edge of another. */
function path(from: Box, to: Box): string {
  const x1 = from.x + from.w;
  const y1 = from.y + from.h / 2;
  const x2 = to.x;
  const y2 = to.y + to.h / 2;
  if (x2 > x1) {
    const bend = Math.max(24, (x2 - x1) / 2);
    return `M${x1},${y1} C${x1 + bend},${y1} ${x2 - bend},${y2} ${x2},${y2}`;
  }
  // The target is not to the right (two nodes in one column): go around.
  const out = 28;
  const mid = (Math.max(from.y + from.h, to.y + to.h) + Math.min(from.y, to.y)) / 2;
  return `M${x1},${y1} C${x1 + out},${y1} ${x1 + out},${mid} ${(x1 + x2) / 2},${mid} S${x2 - out},${y2} ${x2},${y2}`;
}

/** The scale and offset that show the whole layout in a viewport. */
/**
 * How a part of a graph is fitted to what it is drawn in: the box that
 * holds it as large as there is room for, no larger than `most` (what the
 * whole is ever drawn at, unless said), and in the middle.
 */
export function fitBox(box: Box, viewport: { width: number; height: number }, margin = 24, most = 1.25) {
  if (box.w <= 0 || box.h <= 0 || viewport.width <= 0 || viewport.height <= 0) return { k: 1, x: margin, y: margin };
  const k = Math.max(0.2, Math.min(most, (viewport.width - margin * 2) / box.w, (viewport.height - margin * 2) / box.h));
  return { k, x: (viewport.width - box.w * k) / 2 - box.x * k, y: (viewport.height - box.h * k) / 2 - box.y * k };
}

/**
 * The box a graph is looked at by while some of its nodes are followed:
 * the box that holds them, made no smaller than a tile and a half across
 * and two tiles high about its middle, so that what is followed is read
 * at the size it is drawn and not larger. None for no node it has.
 */
export function around(placed: Pick<Layout, 'nodes'>, ids: readonly string[]): Box | null {
  const boxes = ids.map((id) => placed.nodes[id]).filter((one): one is Box => !!one);
  if (boxes.length === 0) return null;
  const left = Math.min(...boxes.map((one) => one.x));
  const top = Math.min(...boxes.map((one) => one.y));
  const right = Math.max(...boxes.map((one) => one.x + one.w));
  const bottom = Math.max(...boxes.map((one) => one.y + one.h));
  const [w, h] = [Math.max(right - left, NODE_W * 1.5), Math.max(bottom - top, NODE_H * 2)];
  return { x: (left + right - w) / 2, y: (top + bottom - h) / 2, w, h };
}

export function fit(size: { width: number; height: number }, viewport: { width: number; height: number }, margin = 32) {
  if (size.width <= 0 || size.height <= 0 || viewport.width <= 0 || viewport.height <= 0) return { k: 1, x: margin, y: margin };
  const k = Math.min(1.25, (viewport.width - margin * 2) / size.width, (viewport.height - margin * 2) / size.height);
  const scale = Math.max(0.2, k);
  return {
    k: scale,
    x: Math.max(margin, (viewport.width - size.width * scale) / 2),
    y: Math.max(margin, Math.min(margin * 2, (viewport.height - size.height * scale) / 2)),
  };
}

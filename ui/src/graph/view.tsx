// A blueprint drawn as a graph: drag to move, scroll to zoom, click a node.
//
// Drawn for a run (`ran`), each tile says at its foot how its step went,
// by a mark, and how long it took or has gone on; and where it is handed
// `folding`, each straight chain of a long graph is one tile that unfolds
// in place, and is folded again by the band drawn round it (`folds.ts`).

import type { ComponentChildren } from 'preact';
import { useLayoutEffect, useMemo, useRef, useState } from 'preact/hooks';
import type { Graph, GraphNode } from '../api.ts';
import { classes, duration } from '../util.ts';
import { chainsOf, foldedGraph, foldTile, opensItself, together } from './folds.ts';
import type { Fold, Ran } from './folds.ts';
import { around, fit, fitBox, fitLeast, layout, NODE_W } from './layout.ts';
import { clip, FOOT_H, tile, TILE_H, tileHeight, tookSaid } from './tile.ts';
import type { Room } from './tile.ts';

/** From one line of a tile's words to the next. */
const LINE = 16;
/** What a tile has across for what it says, in the tile's own measure: from where its words begin to a little short of its edge. */
const ACROSS = NODE_W - 18 - 10;

/**
 * How wide a text is as a tile writes it: its words, or the smaller,
 * even-spaced line under them. Asked of the browser with the letters the
 * tile is written in, since no count of letters says it.
 */
function measure(): Room {
  const ruler = typeof document === 'undefined' ? null : document.createElement('canvas').getContext('2d');
  const family = (name: string) => (typeof document === 'undefined' ? '' : getComputedStyle(document.documentElement).getPropertyValue(name).trim());
  const [words, small] = [`600 12px ${family('--sans') || 'sans-serif'}`, `11px ${family('--mono') || 'monospace'}`];
  return {
    words: ACROSS,
    under: ACROSS,
    wide: (text, under) => {
      if (!ruler) return text.length * 6.8;
      ruler.font = under ? small : words;
      return ruler.measureText(text).width;
    },
  };
}

export interface GraphViewProps {
  graph: Graph;
  /** Node id to status (`running`, `ok`, `failed`, `skipped`, `cancelled`). */
  statuses: Record<string, string>;
  selected: string | null;
  onSelect: (node: GraphNode | null) => void;
  /**
   * A mark on a tile's corner, by node id: one or two letters that say
   * how its step came out to whoever does not tell the colours apart.
   * Nothing is drawn for a graph that is handed none.
   */
  badges?: Record<string, string>;
  /**
   * The nodes being followed, by id: the graph is brought to them, gliding,
   * each time they are others, and to the whole where there are none, until
   * whoever looks at it moves or zooms it themselves; then it is theirs
   * until `scene` is another, or Fit is pressed. A graph that is handed
   * nothing is fitted to the whole, as it always was.
   */
  focus?: readonly string[];
  /** What is being followed, as a word: another, and the graph follows again whatever was done to it. */
  scene?: string;
  /** What the legend at its foot says; where nothing is said, how it is moved, zoomed and pressed. */
  legend?: readonly string[];
  /** Something more of the band under the map, before what is said there: what plays a preview. */
  band?: ComponentChildren;
  /** How each step of a run went, by node id: each tile then says so at its foot, with how long it took. */
  ran?: Readonly<Record<string, Ran>>;
  /** Whether the run goes on: a step not begun is waiting, and not one that never ran. */
  going?: boolean;
  /** When it is now, for how long what is under way has gone on. */
  now?: number;
  /**
   * Each straight chain of a long graph folded into one tile: which the
   * reader opened or shut, by fold id, and what opens or shuts one. A fold
   * nobody chose for is shut, but where a step in it failed.
   */
  folding?: { open: Readonly<Record<string, boolean>>; choose: (fold: string, open: boolean) => void };
  /**
   * The least the whole is drawn at: a graph too wide for it is drawn at
   * that from its start, and what is followed at that. As small as it
   * takes, where unsaid, and what is followed as large as it is.
   */
  least?: number;
}

/** The mark at the foot of a run's tile: how its step went, to be told apart by its shape as well as its colour. */
function Went({ status }: { status: string | undefined }) {
  const sign =
    status === 'ok' ? (
      <>
        <circle r="6" class="fill" />
        <path d="M-2.7,0.1 L-0.8,2.1 L2.8,-2" class="sign" />
      </>
    ) : status === 'failed' ? (
      <>
        <circle r="6" class="fill" />
        <path d="M-2.2,-2.2 L2.2,2.2 M2.2,-2.2 L-2.2,2.2" class="sign" />
      </>
    ) : status === 'running' ? (
      <>
        <circle r="5" class="ring" />
        <circle r="5" class="arc" />
      </>
    ) : status === 'skipped' ? (
      <>
        <circle r="5.5" class="ring" />
        <path d="M-2.6,0 H2.6" class="line" />
      </>
    ) : status === 'cancelled' ? (
      <>
        <circle r="5.5" class="ring" />
        <path d="M-3.4,3.4 L3.4,-3.4" class="line" />
      </>
    ) : (
      <circle r="5.5" class="ring" />
    );
  return <g class={`gwent went-${status ?? 'none'}`}>{sign}</g>;
}

const LEGEND = ['drag to move', 'scroll to zoom', 'click a node for its details'];

export function GraphView({ graph: whole, statuses, selected, onSelect, badges, scene, legend = LEGEND, band, ran, going = false, now = 0, folding, least, focus: asked }: GraphViewProps) {
  // The chains folded, where it is asked: each shut unless the reader opened it, or a step in it failed.
  const folds = useMemo(() => (folding ? chainsOf(whole) : []), [whole, !!folding]);
  const isOpen = (fold: Fold) => folding?.open[fold.id] ?? (ran ? opensItself(fold, ran) : false);
  const opened = folds.map((fold) => (isOpen(fold) ? 'o' : 's')).join('');
  const shown = useMemo(() => foldedGraph(whole, folds, isOpen), [whole, folds, opened]);
  const graph = shown.graph;
  // What is followed, as it is drawn: a step in a shut fold is followed as its fold.
  const focus = useMemo(() => {
    if (!asked) return asked;
    const into = new Map(Object.values(shown.shut).flatMap((fold) => fold.members.map((member): [string, string] => [member, fold.id])));
    return [...new Set(asked.map((id) => into.get(id) ?? id))];
  }, [asked?.join(' '), shown]);
  // How a tile drawn stands: its step's, or a fold's steps' together.
  const stood = (id: string): Ran | undefined => {
    const fold = shown.shut[id];
    return fold ? together(fold.members.map((member) => ran?.[member]), going) : ran?.[id];
  };
  // Where a step says what it is for, its tile says so on two lines, and every tile of the graph is that much taller;
  // drawn for a run, each has a line more at its foot, for how its step went.
  const tall = useMemo(() => tileHeight(whole.nodes) + (ran ? FOOT_H : 0), [whole, !!ran]);
  const placed = useMemo(() => layout(graph, tall), [graph, tall]);
  const room = useMemo(measure, []);
  const box = useRef<HTMLDivElement>(null);
  const [view, setView] = useState({ k: 1, x: 32, y: 32 });
  // Whether the last change of view was the graph following what it is handed, which glides; anything else is at once.
  const [glides, setGlides] = useState(false);
  // Whether whoever looks at it has moved or zoomed it since it was last told what to follow.
  const taken = useRef(false);
  const drag = useRef<{ x: number; y: number; moved: boolean; node: string | null; fold: string | null } | null>(null);
  const [grabbing, setGrabbing] = useState(false);
  // Refit when the shape of the graph changes, not on every keystroke.
  // Before it is drawn, and not a moment after: a map that was drawn and
  // then fitted jumped under whoever had already taken hold of it.
  const shape = `${placed.width}x${placed.height}:${graph.nodes.length}`;
  const looked = (focus ?? []).join(' ');

  const fitToView = () => {
    const rect = box.current?.getBoundingClientRect();
    if (!rect) return;
    taken.current = false;
    const part = focus && focus.length > 0 ? around(placed, focus) : null;
    // What is followed is drawn no larger than it is: around it is what it is part of.
    const room = { width: rect.width, height: rect.height };
    // What is followed of a graph drawn no smaller than `least` is drawn at that, with as much round it as there is room for.
    setView(part ? fitBox(part, room, 24, least ?? 1) : least ? fitLeast(placed, room, least) : fit(placed, room));
  };
  // A fold opened or shut stays where it was and as large: what it was is held where it is on the screen, and the
  // graph is not fitted again, so that its steps unfold in place and whoever reads them goes on reading there.
  const held = useRef<{ id: string; x: number; y: number } | null>(null);
  const holdAt = (from: string, to: string) => {
    const at = placed.nodes[from];
    held.current = at ? { id: to, x: view.x + at.x * view.k, y: view.y + at.y * view.k } : null;
    // Whoever opened or shut it has the graph now: it no longer follows what it is handed until Fit.
    takeOver();
  };
  useLayoutEffect(() => {
    setGlides(false);
    const kept = held.current;
    held.current = null;
    const at = kept && placed.nodes[kept.id];
    if (kept && at) setView((was) => ({ k: was.k, x: kept.x - at.x * was.k, y: kept.y - at.y * was.k }));
    else fitToView();
  }, [shape, scene]);
  // What is followed, each time it is another: brought to, gliding, unless whoever looks has taken the graph over.
  useLayoutEffect(() => {
    if (focus === undefined || taken.current) return;
    setGlides(true);
    fitToView();
  }, [looked]);
  const takeOver = () => {
    taken.current = true;
    setGlides(false);
  };

  const zoom = (factor: number, cx?: number, cy?: number) => {
    const rect = box.current?.getBoundingClientRect();
    if (!rect) return;
    takeOver();
    const px = cx ?? rect.width / 2;
    const py = cy ?? rect.height / 2;
    setView((now) => {
      const k = Math.min(2.5, Math.max(0.15, now.k * factor));
      // Keep the point under the cursor where it is.
      return { k, x: px - ((px - now.x) / now.k) * k, y: py - ((py - now.y) / now.k) * k };
    });
  };

  return (
    // The map, and under it a band of its own for what is said of it and the buttons that move it: laid over the
    // map they lay over its tiles and its groups' names wherever the graph came to be drawn under them.
    <div class="canvas">
      <div
        ref={box}
        class={classes('canvas-map', grabbing && 'grabbing')}
        onWheel={(event) => {
          event.preventDefault();
          const rect = event.currentTarget.getBoundingClientRect();
          zoom(Math.exp(-event.deltaY * 0.0015), event.clientX - rect.left, event.clientY - rect.top);
        }}
        onPointerDown={(event) => {
          if (event.button !== 0) return;
          // Note what was under the pointer now: once the canvas has captured
          // the pointer, later events are reported against the canvas itself.
          const hit = (event.target as Element).closest?.('[data-node], [data-fold]');
          drag.current = { x: event.clientX, y: event.clientY, moved: false, node: hit?.getAttribute('data-node') ?? null, fold: hit?.getAttribute('data-fold') ?? null };
          event.currentTarget.setPointerCapture(event.pointerId);
        }}
        onPointerMove={(event) => {
          const from = drag.current;
          if (!from) return;
          const dx = event.clientX - from.x;
          const dy = event.clientY - from.y;
          if (!from.moved && Math.abs(dx) + Math.abs(dy) < 4) return;
          if (!from.moved) takeOver();
          from.moved = true;
          from.x = event.clientX;
          from.y = event.clientY;
          setGrabbing(true);
          setView((now) => ({ ...now, x: now.x + dx, y: now.y + dy }));
        }}
        onPointerUp={() => {
          const from = drag.current;
          drag.current = null;
          setGrabbing(false);
          if (!from || from.moved) return;
          // A click, not a drag: on a fold it unfolds it, on the band round an open one it folds it, on a node it selects it, elsewhere it clears.
          const open = shown.open.find((fold) => fold.id === from.fold);
          if (folding && open) {
            holdAt(open.members[0]!, open.id);
            return folding.choose(open.id, false);
          }
          const shut = from.node ? shown.shut[from.node] : undefined;
          if (folding && shut) {
            holdAt(shut.id, shut.members[0]!);
            return folding.choose(shut.id, true);
          }
          onSelect(graph.nodes.find((node) => node.id === from.node) ?? null);
        }}
      >
        <svg role="img" aria-label={`Graph of ${graph.name}`}>
          <defs>
            <marker id="arrow" viewBox="0 0 8 8" refX="7" refY="4" markerWidth="7" markerHeight="7" orient="auto">
              <path d="M0,0.5 L7,4 L0,7.5 z" fill="context-stroke" />
            </marker>
          </defs>
          {/* A graph that follows what it is handed is moved by the style sheet, so that it can glide there. */}
          <g
            transform={focus === undefined ? `translate(${view.x},${view.y}) scale(${view.k})` : undefined}
            class={focus !== undefined && glides ? 'glides' : undefined}
            style={focus === undefined ? undefined : { transform: `translate(${view.x}px, ${view.y}px) scale(${view.k})` }}
          >
            {placed.groups.map((group) => (
              <g key={group.id} class="ggroup">
                <rect x={group.x} y={group.y} width={group.w} height={group.h} />
                <text x={group.x + 14} y={group.y + 19}>
                  {group.label}
                </text>
              </g>
            ))}
            {shown.open.map((fold) => {
              // Round the steps of an open fold, with what folds it again on its top edge.
              const boxes = fold.members.map((id) => placed.nodes[id]).filter((one) => !!one);
              if (boxes.length === 0) return null;
              const [left, top] = [Math.min(...boxes.map((one) => one.x)) - 6, Math.min(...boxes.map((one) => one.y)) - 6];
              const [right, bottom] = [Math.max(...boxes.map((one) => one.x + one.w)) + 6, Math.max(...boxes.map((one) => one.y + one.h)) + 6];
              const said = `− ${fold.members.length} steps`;
              const across = room.wide(said, true) + 14;
              return (
                <g key={fold.id} class={classes('gfold-band', ran && `status-${together(fold.members.map((member) => ran[member]), going)?.status ?? 'none'}`)}>
                  <rect class="band" x={left} y={top} width={right - left} height={bottom - top} rx="11" />
                  <g class="gfold-shut" data-fold={fold.id} transform={`translate(${left + 12},${top - 7})`} style="cursor:pointer">
                    <title>{`Fold ${fold.name} into one tile`}</title>
                    <rect width={across} height="14" rx="7" />
                    <text x={across / 2} y="10.5" text-anchor="middle">
                      {said}
                    </text>
                  </g>
                </g>
              );
            })}
            {placed.edges.map((edge) => (
              <path key={`${edge.from}>${edge.to}`} class={classes('gedge', edge.kind)} d={edge.d} marker-end="url(#arrow)" />
            ))}
            {graph.nodes.map((node) => {
              const at = placed.nodes[node.id];
              if (!at) return null;
              const fold = shown.shut[node.id];
              const went = ran ? stood(node.id) : undefined;
              const status = fold ? went?.status : statuses[node.id];
              const marks = fold ? [] : [
                node.when && 'if',
                node.for_each && 'each',
                node.retry && 'retry',
                node.timeout && node.timeout,
                node.hooks.length > 0 && 'hooks',
                node.watch.length > 0 && 'watch',
                node.device && `@${node.device}`,
              ].filter(Boolean);
              const corner = clip(marks.join(' · '), 22);
              // The marks stand at the corner of the first line, in smaller letters: the words leave them their room.
              const said = fold ? foldTile(fold, room) : tile(node, marks.length > 0 ? room.wide(corner, true) + 8 : 0, room);
              // Where the first line is written: where it always was in a tile of the usual height, and in a
              // taller one so that the lines there are stand in its middle; above the foot, where it has one.
              const body = at.h - (ran ? FOOT_H : 0);
              const first = body === TILE_H ? 21 : (body - (said.words.length * LINE + 15)) / 2 + 13;
              return (
                <g
                  key={node.id}
                  data-node={node.id}
                  class={classes('gnode', fold ? 'gfold' : `kind-${node.kind}`, status && `status-${status}`, selected === node.id && 'selected')}
                  data-status={ran ? (status ?? (going ? 'waiting' : 'not run')) : undefined}
                  transform={`translate(${at.x},${at.y})`}
                  style="cursor:pointer"
                >
                  <title>{fold ? `${fold.name}: ${fold.keys.join(', ')}. Click to unfold.` : (node.description ?? `${node.name}: ${node.target}`)}</title>
                  {/* A fold is drawn as tiles stacked, the one behind just showing. */}
                  {fold && <rect class="behind" x="4" y="4" width={at.w} height={at.h} rx="8" />}
                  <rect class="box" width={at.w} height={at.h} rx="8" />
                  <rect class="stripe" width="4" height={at.h - 16} x="6" y="8" rx="2" />
                  {/* What the step says of itself where it says anything, and its key under that, smaller; else its key, as ever. */}
                  {said.words.map((line, index) => (
                    <text key={index} class={said.said ? 'says' : undefined} x="18" y={first + index * LINE} style="font-weight:600">
                      {line}
                    </text>
                  ))}
                  <text class="sub" x="18" y={first + said.words.length * LINE + 1}>
                    {said.under}
                  </text>
                  {ran && (
                    <g class="gfoot" transform={`translate(24,${body + 4})`}>
                      <Went status={status} />
                      <text class="took" x="11" y="4">
                        {tookSaid(went, going, now, duration)}
                      </text>
                    </g>
                  )}
                  {marks.length > 0 && (
                    <text class="mark" x={at.w - 8} y="14" text-anchor="end">
                      {corner}
                    </text>
                  )}
                  {badges?.[node.id] && (
                    // On the tile's corner, outside what the tile says: it lies over none of it.
                    <g class="gbadge" transform="translate(0,0)">
                      <circle r="9" />
                      <text text-anchor="middle" y="3.5">
                        {badges[node.id]}
                      </text>
                    </g>
                  )}
                </g>
              );
            })}
          </g>
        </svg>
      </div>
      <div class="canvas-band">
        {band}
        <div class="canvas-legend">
          {legend.map((said) => (
            <span key={said}>{said}</span>
          ))}
        </div>
        <div class="canvas-tools">
          <button class="icon" aria-label="Zoom out" onClick={() => zoom(1 / 1.25)}>
            −
          </button>
          <button class="icon" aria-label="Zoom in" onClick={() => zoom(1.25)}>
            +
          </button>
          <button
            onClick={() => {
              setGlides(false);
              fitToView();
            }}
          >
            Fit
          </button>
        </div>
      </div>
    </div>
  );
}

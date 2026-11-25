// A blueprint drawn as a graph: drag to move, scroll to zoom, click a node.

import type { ComponentChildren } from 'preact';
import { useLayoutEffect, useMemo, useRef, useState } from 'preact/hooks';
import type { Graph, GraphNode } from '../api.ts';
import { classes } from '../util.ts';
import { around, fit, fitBox, layout, NODE_W } from './layout.ts';
import { clip, tile, TILE_H, tileHeight } from './tile.ts';
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
}

const LEGEND = ['drag to move', 'scroll to zoom', 'click a node for its details'];

export function GraphView({ graph, statuses, selected, onSelect, badges, focus, scene, legend = LEGEND, band }: GraphViewProps) {
  // Where a step says what it is for, its tile says so on two lines, and every tile of the graph is that much taller.
  const tall = useMemo(() => tileHeight(graph.nodes), [graph]);
  const placed = useMemo(() => layout(graph, tall), [graph, tall]);
  const room = useMemo(measure, []);
  const box = useRef<HTMLDivElement>(null);
  const [view, setView] = useState({ k: 1, x: 32, y: 32 });
  // Whether the last change of view was the graph following what it is handed, which glides; anything else is at once.
  const [glides, setGlides] = useState(false);
  // Whether whoever looks at it has moved or zoomed it since it was last told what to follow.
  const taken = useRef(false);
  const drag = useRef<{ x: number; y: number; moved: boolean; node: string | null } | null>(null);
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
    setView(part ? fitBox(part, { width: rect.width, height: rect.height }, 24, 1) : fit(placed, { width: rect.width, height: rect.height }));
  };
  useLayoutEffect(() => {
    setGlides(false);
    fitToView();
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
          const hit = (event.target as Element).closest?.('[data-node]');
          drag.current = { x: event.clientX, y: event.clientY, moved: false, node: hit?.getAttribute('data-node') ?? null };
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
          // A click, not a drag: on a node it selects it, elsewhere it clears.
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
            {placed.edges.map((edge) => (
              <path key={`${edge.from}>${edge.to}`} class={classes('gedge', edge.kind)} d={edge.d} marker-end="url(#arrow)" />
            ))}
            {graph.nodes.map((node) => {
              const at = placed.nodes[node.id];
              if (!at) return null;
              const status = statuses[node.id];
              const marks = [
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
              const said = tile(node, marks.length > 0 ? room.wide(corner, true) + 8 : 0, room);
              // Where the first line is written: where it always was in a tile of the usual height, and in a
              // taller one so that the lines there are stand in its middle.
              const first = at.h === TILE_H ? 21 : (at.h - (said.words.length * LINE + 15)) / 2 + 13;
              return (
                <g
                  key={node.id}
                  data-node={node.id}
                  class={classes('gnode', `kind-${node.kind}`, status && `status-${status}`, selected === node.id && 'selected')}
                  transform={`translate(${at.x},${at.y})`}
                  style="cursor:pointer"
                >
                  <title>{node.description ?? `${node.name}: ${node.target}`}</title>
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

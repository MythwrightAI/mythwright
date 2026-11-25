// What the tile of a node in a blueprint's graph says.
//
// A step is known to its blueprint by its key, and its tile said the key
// and what it runs: `fleet`, `call std/fleet`. Neither says what the step
// is for. Most steps of what ships say that themselves, in a
// `description`, so where a step has one the tile says that, in words,
// and its key second and smaller with what it runs. A step that says
// nothing of itself is drawn as it always was. Nothing is renamed: the
// key is still what the text, the runs and the logs call the step.
//
// This is the words, with no drawing in it: which lines a tile has, and
// how tall the tiles of a graph are for that. How wide a line may be is
// handed in (`Room`): letters are not all as wide as each other, and a
// line counted in letters ran past the edge of its tile.

import type { GraphNode } from '../api.ts';
import { NODE_H } from './layout.ts';

/**
 * How much room a tile has for its words and for the smaller line under
 * them, and how wide a text is, in whatever the room is counted in. Where
 * a tile is drawn that is the width of the letters as they are drawn;
 * with nothing to draw with it is how many letters there are.
 */
export interface Room {
  words: number;
  under: number;
  wide: (text: string, small: boolean) => number;
}
/** How many letters fit across a tile in the words' size, and in the smaller, even-spaced one under them. */
export const ACROSS = 27;
export const ACROSS_SMALL = 26;
export const LETTERS: Room = { words: ACROSS, under: ACROSS_SMALL, wide: (text) => text.length };
/** How tall a tile is with a line of words and one under it, and with two lines of words. */
export const TILE_H = NODE_H;
export const TILE_H_SAID = NODE_H + 16;

type Said = Pick<GraphNode, 'kind' | 'name' | 'target' | 'description'>;

export function clip(text: string, max: number): string {
  return text.length > max ? `${text.slice(0, Math.max(0, max - 1))}…` : text;
}

/** What a node runs, as its tile says it. */
export function runs(node: Pick<GraphNode, 'kind' | 'target'>): string {
  return node.kind === 'spawn' ? `spawn ${node.target}` : node.kind === 'call' ? `call ${node.target}` : node.target;
}

/** What a node is called in its blueprint: a step's key, and `when` for what starts a handler. */
export function keyOf(node: Pick<GraphNode, 'kind' | 'name'>): string {
  return node.kind === 'trigger' ? 'when' : node.name;
}

/**
 * What a step says of itself, as far as its first full stop: the rest is
 * read where the step is opened. One that says nothing says nothing.
 */
export function says(description: string | null | undefined): string {
  const text = (description ?? '').replace(/\s+/g, ' ').trim();
  const end = /[.!?](?=\s)/.exec(text);
  return end ? text.slice(0, end.index + 1) : text;
}

/** A text cut to the room it is given, saying that it was cut; whole where it fits. */
export function fitted(text: string, room: number, wide: (text: string) => number = (one) => one.length): string {
  if (wide(text) <= room) return text;
  let cut = text;
  while (cut.length > 0 && wide(`${cut}…`) > room) cut = cut.slice(0, -1);
  return `${cut.trimEnd()}…`;
}

/**
 * Words on as many lines as there are rooms, broken between words; what
 * does not fit on the last is cut there. A word longer than its line is
 * cut too, so nothing is ever wider than it was given.
 */
export function wrapped(text: string, rooms: number[], wide: (text: string) => number = (one) => one.length): string[] {
  const words = text.split(' ').filter(Boolean);
  const lines: string[] = [];
  let at = 0;
  for (let line = 0; line < rooms.length && at < words.length; line += 1) {
    const room = rooms[line]!;
    let now = words[at]!;
    at += 1;
    while (at < words.length && wide(`${now} ${words[at]}`) <= room) {
      now = `${now} ${words[at]}`;
      at += 1;
    }
    // More to say and nowhere to say it: the line ends by saying so.
    const last = line === rooms.length - 1;
    lines.push(fitted(last && at < words.length ? `${now} ${words[at]}` : now, room, wide));
  }
  return lines;
}

/** The lines of a tile. */
export interface Tile {
  /** The words: what the step says of itself on up to two lines, or its key where it says nothing. */
  words: string[];
  /** Under them, smaller: what it runs, after its key where the words are not the key. */
  under: string;
  /** Whether the words are what the step says of itself. */
  said: boolean;
}

/**
 * What a node's tile says. `taken` is how much of the first line's room
 * something else has (the marks at its corner), counted as `room` counts.
 */
export function tile(node: Said, taken = 0, room: Room = LETTERS): Tile {
  const said = says(node.description);
  // As it always was: by its letters, whatever it is drawn with.
  if (!said) return { words: [clip(keyOf(node), 24)], under: clip(runs(node), ACROSS_SMALL), said: false };
  const first = Math.max(room.words / 3, room.words - taken);
  return {
    words: wrapped(said, [first, room.words], (text) => room.wide(text, false)),
    under: fitted(`${keyOf(node)} · ${runs(node)}`, room.under, (text) => room.wide(text, true)),
    said: true,
  };
}

/** How tall the tiles of a graph are: all alike, and taller only where some step says something of itself. */
export function tileHeight(nodes: Pick<GraphNode, 'description'>[]): number {
  return nodes.some((node) => says(node.description) !== '') ? TILE_H_SAID : TILE_H;
}

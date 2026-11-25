// What a blueprint's text says about itself: what is being written at a
// caret, what a mapping already has, which steps and params there are.
//
// Everything here reads the text as it stands, checked or not, because
// what is offered while typing cannot wait for the text to be right.

import type { BlueprintLanguage } from '../api.ts';
import { depth, keyAt, scanLine, start, writtenAfter } from './scan.ts';
import type { Frame, Place, State, Token } from './scan.ts';

export interface Reading {
  lines: string[];
  /** The line the caret is on, counted from 0, and where it starts in the text. */
  line: number;
  lineStart: number;
  place: Place;
}

/** One entry of a mapping: its key, and what is written after it on its line. */
export interface Entry {
  key: string;
  value: string;
  line: number;
  /** The column its key starts at; -1 inside `{ }`. */
  column: number;
}

/** Reads down to the line an offset is on. */
function upTo(text: string, offset: number, language: BlueprintLanguage | null) {
  const lines = text.split('\n');
  let state: State = start(language);
  let lineStart = 0;
  let line = 0;
  for (; line < lines.length - 1; line++) {
    const end = lineStart + lines[line]!.length;
    if (offset <= end) break;
    state = scanLine(lines[line]!, state, language).state;
    lineStart = end + 1;
  }
  return { lines, state, line, lineStart };
}

/** What is being written at an offset in the text. */
export function placeAt(text: string, offset: number, language: BlueprintLanguage | null): Reading {
  const { lines, state, line, lineStart } = upTo(text, offset, language);
  const before = lines[line]!.slice(0, Math.max(0, offset - lineStart));
  return { lines, line, lineStart, place: scanLine(before, state, language, true).place };
}

/** The piece of the text an offset is on, if it is on one. */
export function tokenAt(text: string, offset: number, language: BlueprintLanguage | null): { token: Token; lineStart: number } | null {
  const { lines, state, line, lineStart } = upTo(text, offset, language);
  const column = offset - lineStart;
  const token = scanLine(lines[line]!, state, language).tokens.find((one) => one.from <= column && column < one.to);
  return token ? { token, lineStart } : null;
}

/** What is written inside the `{ }` that follows a key on its own line. */
function inline(text: string, line: number): Entry[] {
  const close = text.lastIndexOf('}');
  if (close < 0) return [];
  // Read up to its last brace, the mapping is still open and holds what it was given.
  const open = scanLine(text.slice(0, close), start(null), null).state.frames;
  const outer = open.find((frame) => frame.flow === '{');
  return (outer?.entries ?? []).map(([key, value]) => ({ key, value: unquoted(value), line, column: -1 }));
}

function unquoted(value: string): string {
  const first = value[0];
  return value.length > 1 && (first === '"' || first === "'") && value.endsWith(first) ? value.slice(1, -1) : value;
}

/** The entries of a mapping that is open at a place. */
export function entriesOf(lines: string[], frame: Frame): Entry[] {
  if (frame.flow) {
    return (frame.entries ?? []).map(([key, value]) => ({ key, value: unquoted(value), line: frame.line, column: -1 }));
  }
  const found: Entry[] = [];
  // A mapping that is the value of a key starts on the line after it. The
  // file, and an item of a list, start on their own first line.
  for (let at = frame.key === '' ? frame.line : frame.line + 1; at < lines.length; at++) {
    const text = lines[at]!;
    const indent = depth(text);
    if (indent === text.length || text[indent] === '#') continue;
    // An item's first entry is on the line of its dash, further in.
    const first = at === frame.line && indent < frame.indent;
    if (indent < frame.indent && !first) break;
    if (indent > frame.indent) continue;
    const key = keyAt(text, frame.indent);
    if (key) found.push({ key: key.text, value: writtenAfter(text, key.after), line: at, column: frame.indent });
  }
  return found;
}

/** The entries of the mapping that is the value of an entry: in `{ }` on its line, or on the lines below. */
export function entriesUnder(lines: string[], entry: Entry): Entry[] {
  if (entry.value.startsWith('{')) return inline(lines[entry.line]!, entry.line);
  if (entry.value !== '' || entry.column < 0) return [];
  for (let at = entry.line + 1; at < lines.length; at++) {
    const text = lines[at]!;
    const indent = depth(text);
    if (indent === text.length || text[indent] === '#') continue;
    if (indent <= entry.column) return [];
    return entriesOf(lines, { indent, holds: { is: 'data' }, key: entry.key, line: entry.line });
  }
  return [];
}

/** What is declared under a key at the top of the file: its params, its state, its steps. */
export function declared(lines: string[], key: string): Entry[] {
  const top = entriesOf(lines, { indent: 0, holds: { is: 'data' }, key: '', line: 0 });
  const entry = top.find((one) => one.key === key);
  return entry ? entriesUnder(lines, entry) : [];
}

/**
 * The steps of the graph a place is in, and which of them it is inside.
 * A graph is whatever the language says holds steps by name: a blueprint's
 * `steps`, a handler's, and the steps of a task, which is the nearest one
 * to a place inside it. A place outside any graph, such as the file's
 * `outputs`, sees the main one.
 */
export function stepsAround(lines: string[], frames: Frame[]): { steps: Entry[]; own: string | null } {
  for (let at = frames.length - 1; at >= 0; at--) {
    const frame = frames[at]!;
    if (frame.holds.is === 'map_of' && frame.holds.shape === 'step') {
      return { steps: entriesOf(lines, frame), own: frames[at + 1]?.key ?? null };
    }
  }
  return { steps: declared(lines, 'steps'), own: null };
}

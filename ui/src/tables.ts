// A table, as it is written in text: a line of its headings with pipes
// between them, a line of dashes under it, and a line for each row.
//
//   | Practice   | Minutes | Studies |
//   |:-----------|:-------:|--------:|
//   | [Breath](topic:@-topics-4) | 10 | 12 |
//
// A colon at the left of a column's dashes sets its words at the left, at
// both ends in the middle, and at the right alone at the right, as numbers
// are. A cell holds what a line holds (`prose.ts` reads it so); a pipe that
// is a cell's own is written `\|`. The table ends at an empty line or at a
// line with no pipe in it. A line of headings and the line of dashes under
// it must have as many cells as each other; a row with fewer has empty ones
// at its end, and one with more loses those past the last heading.
//
// It is read here alone, for both the page (`prose.ts`) and the block
// editor (`write/markdown.ts`), which keeps it as the text it was written
// as, so that what one draws as a table the other keeps as one.

/** Where a column's words stand: as the language's usual, or at the left, the middle or the right. */
export type Align = 'left' | 'center' | 'right' | null;

/** A table as written: its columns' alignments and headings, its rows' cells as text, and how many lines it took. */
export interface TableText {
  align: Align[];
  head: string[];
  rows: string[][];
  lines: number;
}

/** A pipe not written as `\|`. */
const PIPE = /(?<!\\)\|/;
const DELIMITER = /^:?-+:?$/;

/** The cells of a line of a table, each trimmed and with its `\|` read as a pipe; nothing where the line has no pipe of a table. */
export function cellsOf(line: string): string[] | null {
  if (/^ {4}/.test(line) || !PIPE.test(line)) return null;
  let inner = line.trim();
  if (inner.startsWith('|')) inner = inner.slice(1);
  if (/(?<!\\)\|$/.test(inner)) inner = inner.slice(0, -1);
  return inner.split(PIPE).map((cell) => cell.trim().replace(/\\\|/g, '|'));
}

/** Where a column's words stand, by the colons of its dashes. */
function alignOf(dashes: string): Align {
  const left = dashes.startsWith(':');
  const right = dashes.endsWith(':');
  return left && right ? 'center' : right ? 'right' : left ? 'left' : null;
}

/** The table that begins at line `at` of `lines`, or nothing where none does. */
export function tableAt(lines: readonly string[], at: number): TableText | null {
  const head = cellsOf(lines[at] ?? '');
  const dashes = cellsOf(lines[at + 1] ?? '');
  if (!head || !dashes || head.length !== dashes.length || !dashes.every((cell) => DELIMITER.test(cell))) return null;
  const rows: string[][] = [];
  let next = at + 2;
  for (; next < lines.length; next++) {
    const line = lines[next]!;
    const cells = line.trim() === '' ? null : cellsOf(line);
    if (!cells) break;
    rows.push(head.map((_, column) => cells[column] ?? ''));
  }
  return { align: dashes.map(alignOf), head, rows, lines: next - at };
}

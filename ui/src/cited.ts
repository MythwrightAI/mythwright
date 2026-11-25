// Sources, as plain values: footnotes in what is written, and the numbers
// they are drawn with. Nothing of the page is in it, so that its cases can
// read it; `views/cited.tsx` draws a marker and its source, and `ui.tsx`
// and `views/articleend.tsx` the list of them.
//
// A source is written as Markdown footnotes are, and stays text:
//
//   Chytrid fungus killed whole species of frogs.[^1]
//
//   [^1]: Ada Lovelace, *Notes*, Taylor, 1843. https://example.org/notes (checked 2026-11-25)
//
// A marker is any label in `[^…]`; a source is a line of its own that
// begins with one and a colon. Each is drawn with a number, in the order
// it is first used, whatever its label says; a source that nothing uses
// comes after those that are used, in the order it was written. An address
// in a source is a link, opened only when it is pressed: nothing is fetched.

import type { Block, Span } from './prose.ts';

/** One source as it is drawn: its number, its label, what it says (nothing where it is used and not written), and how many times it is used. */
export interface Note {
  n: number;
  label: string;
  text: string | null;
  spans: Span[] | null;
  uses: number;
}

/** The sources of a text, in the order they are numbered, and each by its label. */
export interface Notes {
  list: Note[];
  byLabel: Map<string, Note>;
}

/** The labels of the markers in some spans, in the order they are written. */
function citesIn(spans: Span[], found: string[]): void {
  for (const span of spans) {
    if (span.is === 'cite') found.push(span.label);
    else if ('inside' in span) citesIn(span.inside, found);
  }
}

/** The spans of a block, wherever it keeps them. */
function spansOf(block: Block): Span[][] {
  if (block.is === 'list') return block.items.map((item) => item.spans);
  if (block.is === 'table') return [...block.head, ...block.rows.flat()];
  if ('spans' in block && block.is !== 'note') return [block.spans];
  return [];
}

/** The sources of a text read into blocks, numbered: in the order first used, then those used nowhere. */
export function notesOf(blocks: Block[]): Notes {
  const used: string[] = [];
  blocks.forEach((block) => spansOf(block).forEach((spans) => citesIn(spans, used)));
  const written = new Map<string, Extract<Block, { is: 'note' }>>();
  for (const block of blocks) if (block.is === 'note' && !written.has(block.label)) written.set(block.label, block);
  const byLabel = new Map<string, Note>();
  const add = (label: string) => {
    const one = byLabel.get(label);
    if (one) return void one.uses++;
    const source = written.get(label);
    byLabel.set(label, { n: byLabel.size + 1, label, text: source?.text ?? null, spans: source?.spans ?? null, uses: 1 });
  };
  used.forEach(add);
  for (const label of written.keys()) {
    if (byLabel.has(label)) continue;
    add(label);
    byLabel.get(label)!.uses = 0;
  }
  return { list: [...byLabel.values()], byLabel };
}

/** What is said of a marker whose source is not written. */
export const NOT_WRITTEN = 'No source is written for it yet.';

/** The marks that take a reader back to each place a source is used: one arrow, or a letter for each. */
export const backMarks = (uses: number): string[] => (uses <= 1 ? ['↑'] : Array.from({ length: uses }, (_, at) => String.fromCharCode(97 + (at % 26))));

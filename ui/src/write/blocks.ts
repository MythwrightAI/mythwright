// What writing a document knows that needs no editor to say: which blocks
// there are, what a file has that the block editor cannot keep, and the
// settings at the head of a file, which are about the document and not in it.

import { isBoxLanguage } from '../boxes.ts';
import type { BoxLanguage } from '../boxes.ts';

export interface Block {
  id: 'text' | 'h1' | 'h2' | 'h3' | 'bullets' | 'numbers' | 'quote' | 'code' | 'divider' | 'picture' | BoxLanguage;
  label: string;
  hint: string;
  /** Other words someone might type for it. */
  also: string;
}

/** The blocks the `/` menu offers, in the order it shows them. */
export const BLOCKS: Block[] = [
  { id: 'text', label: 'Text', hint: 'Plain writing', also: 'paragraph plain body' },
  { id: 'h1', label: 'Heading 1', hint: 'The title of a section', also: 'h1 title big' },
  { id: 'h2', label: 'Heading 2', hint: 'A part of a section', also: 'h2 subtitle medium' },
  { id: 'h3', label: 'Heading 3', hint: 'A smaller part', also: 'h3 small' },
  { id: 'bullets', label: 'Bulleted list', hint: 'Points in no order', also: 'unordered ul dash' },
  { id: 'numbers', label: 'Numbered list', hint: 'Steps in order', also: 'ordered ol steps' },
  { id: 'quote', label: 'Quote', hint: 'Someone else\'s words', also: 'blockquote cite' },
  { id: 'code', label: 'Code', hint: 'Kept exactly as typed', also: 'pre snippet monospace' },
  { id: 'divider', label: 'Divider', hint: 'A line between parts', also: 'rule hr line separator' },
  // Offered only where the document takes pictures (a topic's): it asks for a file and puts it in.
  { id: 'picture', label: 'Picture or page', hint: 'A picture, a diagram as SVG, or a page', also: 'image figure diagram svg view attach file' },
  // Offered only where the document links to pages (a topic's): each is a block of code to be filled in (`boxes.ts`).
  { id: 'infobox', label: 'Info box', hint: 'Key facts, at the side', also: 'infobox facts sidebar' },
  { id: 'series', label: 'Series box', hint: 'Pages that go together', also: 'navigation related group' },
  { id: 'note', label: 'Note at the top', hint: 'Where else to look', also: 'see disambiguation pointer' },
];

/** The blocks that are an article's boxes. */
export const isBox = (id: Block['id']): id is BoxLanguage => isBoxLanguage(id);

/** The blocks whose name, or another word for them, starts with what was typed after `/`. */
export function matching(typed: string): Block[] {
  const wanted = typed.trim().toLowerCase();
  if (wanted === '') return BLOCKS;
  const words = (block: Block) => `${block.label} ${block.also}`.toLowerCase().split(' ');
  const starts = BLOCKS.filter((block) => block.label.toLowerCase().startsWith(wanted));
  const rest = BLOCKS.filter((block) => !starts.includes(block) && words(block).some((word) => word.startsWith(wanted)));
  return [...starts, ...rest];
}

/**
 * A file's settings block (`---` … `---` at its very head) and the rest.
 * The block editor is given only the rest, and the head is put back as it
 * was when the document is saved.
 */
export function splitHead(text: string): { head: string; body: string } {
  const found = /^---\r?\n[\s\S]*?\r?\n---[ \t]*(?:\r?\n|$)/.exec(text);
  return found ? { head: found[0], body: text.slice(found[0].length) } : { head: '', body: text };
}

/**
 * Why a document cannot be opened in the block editor without being changed
 * by it, or `null` if it can. The editor keeps what it understands; a piece
 * of raw HTML would come out as something else. Footnotes and tables are
 * kept (`markdown.ts`), a table as the text it was written as.
 */
export function cannotKeep(text: string): string | null {
  const lines = splitHead(text).body.split(/\r?\n/);
  let fenced = false;
  for (const line of lines) {
    if (/^\s*(```|~~~)/.test(line)) fenced = !fenced;
    if (fenced) continue;
    if (/^\s{0,3}<\/?[a-zA-Z][^>]*>/.test(line)) return 'raw HTML';
  }
  return null;
}

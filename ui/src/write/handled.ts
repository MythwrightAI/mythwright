// What the handle beside a block decides, with no editor in it: which
// block the pointer is at, where a block that is carried would land, what
// a block becomes when it is turned into another, and which lines its menu
// has. The editor asks these with what it measured and does what they say
// (`handle.ts`), so that each is a function with its cases.

import { BLOCKS, isBox } from './blocks.ts';
import type { Block } from './blocks.ts';

export type Kind = Block['id'];

/** Where a block of the document is down the page, top to bottom, as the browser drew it. */
export interface Span {
  top: number;
  bottom: number;
}

/**
 * The block the pointer is at, by how far down the page it is: the one it
 * is over, or between two the nearer, so that the handle does not come and
 * go as the pointer crosses the room between two blocks. Above the first
 * and below the last by more than `slack`, none.
 */
export function blockAt(spans: Span[], y: number, slack = 8): number | null {
  if (spans.length === 0) return null;
  if (y < spans[0]!.top - slack || y > spans.at(-1)!.bottom + slack) return null;
  for (let at = 0; at < spans.length; at++) {
    const span = spans[at]!;
    if (y <= span.bottom) {
      if (y >= span.top || at === 0) return at;
      // Between the one before and this one: the nearer.
      return y - spans[at - 1]!.bottom <= span.top - y ? at - 1 : at;
    }
  }
  return spans.length - 1;
}

/** Where a carried block would be let go: before the block whose middle is under the pointer, or after the last. A number from 0 to as many blocks as there are. */
export function landing(spans: Span[], y: number): number {
  const before = spans.findIndex((span) => y < (span.top + span.bottom) / 2);
  return before < 0 ? spans.length : before;
}

/** Which block the one at `from` is, once it is let go at `gap`; nothing where it would stay where it is. */
export function moved(from: number, gap: number): number | null {
  if (gap === from || gap === from + 1) return null;
  return gap > from ? gap - 1 : gap;
}

/** Where the line that says where a block would land is drawn: midway between the two blocks it falls between, or just past the end. */
export function lineAt(spans: Span[], gap: number): number {
  if (spans.length === 0) return 0;
  if (gap <= 0) return spans[0]!.top - 2;
  if (gap >= spans.length) return spans.at(-1)!.bottom + 2;
  return (spans[gap - 1]!.bottom + spans[gap]!.top) / 2;
}

/** What a block of the document is, as the `/` menu names it; nothing for what it does not name (a fourth level of heading, a picture). */
export function kindOf(node: { type: string; level?: number }): Kind | null {
  switch (node.type) {
    case 'paragraph':
      return 'text';
    case 'heading':
      return node.level === 1 ? 'h1' : node.level === 2 ? 'h2' : node.level === 3 ? 'h3' : null;
    case 'bullet_list':
      return 'bullets';
    case 'ordered_list':
      return 'numbers';
    case 'blockquote':
      return 'quote';
    case 'code_block':
      return 'code';
    case 'horizontal_rule':
      return 'divider';
    default:
      return null;
  }
}

/**
 * What a block is turned into: the blocks it becomes, each with which of
 * its lines it holds (a line is a paragraph, a heading, an item of a list,
 * a line of code). Nothing that was written is lost: a list turned into
 * headings is a heading for each item, and the lines of code are one line
 * of text each; what holds several lines (a list, a quote, code) holds
 * them all.
 */
export function turning(to: Kind, lines: number): { kind: Kind; lines: number[] }[] {
  const all = Array.from({ length: Math.max(1, lines) }, (_, at) => at);
  switch (to) {
    case 'bullets':
    case 'numbers':
    case 'quote':
    case 'code':
      return [{ kind: to, lines: all }];
    case 'divider':
      return [];
    default:
      return all.map((line) => ({ kind: to, lines: [line] }));
  }
}

/** What a block may be turned into: every block the `/` menu offers that holds words. A divider holds none, and is turned into nothing. */
export function turnable(kind: Kind | null): Block[] {
  // A divider holds no words; a picture is put in, not turned into, and so is a box, with what it says begun.
  return kind === 'divider' ? [] : BLOCKS.filter((block) => block.id !== 'divider' && block.id !== 'picture' && !isBox(block.id));
}

/** One line of the menu a block's handle opens, before it is drawn. */
export type Line = { said: string } | { turn: Kind; label: string; chosen: boolean } | { act: 'duplicate' | 'up' | 'down' | 'comment' | 'delete'; label: string; apart?: boolean };

/**
 * The menu of a block: what it may be turned into, with what it is marked,
 * then what is done to it. A line is there only where pressing it does
 * something: no Move up for the first block, no Comment where the
 * document takes none.
 */
export function linesOf(block: { kind: Kind | null; at: number; of: number; comments: boolean }): Line[] {
  const turns = turnable(block.kind);
  const lines: Line[] = turns.length ? [{ said: 'Turn into' }, ...turns.map((one) => ({ turn: one.id, label: one.label, chosen: one.id === block.kind }))] : [];
  const acts: Line[] = [{ act: 'duplicate', label: 'Duplicate' }];
  if (block.at > 0) acts.push({ act: 'up', label: 'Move up' });
  if (block.at < block.of - 1) acts.push({ act: 'down', label: 'Move down' });
  if (block.comments) acts.push({ act: 'comment', label: 'Comment' });
  acts.push({ act: 'delete', label: 'Delete' });
  if (lines.length) acts[0] = { ...acts[0]!, apart: true } as Line;
  return [...lines, ...acts];
}

/** How a comment on a block begins: which block, by its first words, after whatever was already being written. */
export function commentOn(words: string, draft: string): string {
  const flat = words.replace(/\s+/g, ' ').trim();
  const short = flat.length > 60 ? `${flat.slice(0, 57).trimEnd()}…` : flat;
  const begun = short ? `On “${short}”: ` : '';
  const before = draft.trimEnd();
  return before ? `${before}\n${begun}` : begun;
}

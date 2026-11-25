// Comments on a few words of a document, as the page shows them: which are
// marked where their words are, and where the comments on one mark open.
//
// What a comment is about is kept with it (`anchors.ts` finds the words
// again). A comment whose words are found is marked there, in Write and
// in Source, and is opened from its mark. Every comment, on the whole
// document or on a few of its words, is listed in the Comments tab of the
// panel beside the document (`doctools.ts`), one on words saying what it
// was about and, where its words are no longer in the text, that they are
// not. The page as it is published draws no marks. Nothing is ever dropped.
//
// Opened, a mark's comments are beside the text, at the height of the
// mark's line, so that they lie over nothing. Where the page is too
// narrow for that with its column in the middle, it makes room:
// the column stands further left, and narrower where it has to, so
// that the room is to its right (`roomy`). It does so for as long as the
// document has comments on its words, so that a mark pressed has its
// room already and stays under the pointer, and the column never moves
// back as a card is put away, which a press in the text puts it; a
// comment begun on the first words commented on is the one move, which
// the press asked for. Where even that leaves too narrow a column (a
// narrow tab, a phone) they are in the Comments tab of the panel beside
// the document, which is opened for them. Everything here is a function of plain values, with
// its cases (`anchored.test.ts`).

import { aboutOf } from './anchors.ts';
import type { Anchor } from './anchors.ts';
import type { Comment } from './journey.ts';

/** How a document is shown: written in as blocks, as the file it is, or as the page it is published as. */
export type Shown = 'write' | 'source' | 'page';

/** One anchor told from another: by what it quotes and what was either side, wherever it stood. */
export function keyOf(anchor: Anchor): string {
  return JSON.stringify([anchor.quote, anchor.before ?? '', anchor.after ?? '']);
}

/** A comment on a few words, by where it is in the record's history and what it is about. */
export interface OnWords {
  seq: number;
  anchor: Anchor;
}

/** The comments that are on a few words, each once. */
export function onWords(comments: Comment[]): OnWords[] {
  return comments.flatMap(({ line, said }) => (said.on ? [{ seq: line.seq, anchor: said.on }] : []));
}

/** What is said of what a comment was about, over its words: where it is shown away from its mark, and whether its words are still there. */
export function saidOn(anchor: Anchor, found: boolean): string {
  const about = `on “${aboutOf(anchor)}”`;
  return found ? about : `${about}, which is no longer in the text`;
}

/** Which comments are on a mark, or a thread begun: those the mark covers, and any since said on the same words. */
export interface Thread {
  anchor: Anchor;
  seqs: number[];
}

export function threadOf(comments: Comment[], thread: Thread): Comment[] {
  const key = keyOf(thread.anchor);
  return comments.filter(({ line, said }) => said.on && (thread.seqs.includes(line.seq) || keyOf(said.on) === key));
}

/** The comments that marks drawn over one stretch of text stand for, by the stretch. */
export function marksOf<T extends { from: number; to: number }>(placed: (T & { seq: number })[]): { from: number; to: number; seqs: number[] }[] {
  const by = new Map<string, { from: number; to: number; seqs: number[] }>();
  for (const one of placed) {
    const key = `${one.from}:${one.to}`;
    const kept = by.get(key);
    if (kept) kept.seqs.push(one.seq);
    else by.set(key, { from: one.from, to: one.to, seqs: [one.seq] });
  }
  return [...by.values()].sort((one, other) => one.from - other.from || other.to - one.to);
}

/** The narrowest a card in the margin is, and the most; the room between it and the text, and at the page's edge. */
export const CARD = { least: 240, most: 320, gap: 24, edge: 16 };

/** The room a page makes to the right of its column for a card, gap and edge included (`--room` in `comments.css`). */
export const ROOM = CARD.gap + 280 + CARD.edge;
/** The narrowest the column is made to make that room: narrower, the panel beside the document is used and the column is as it was. */
export const LEAST_COLUMN = 440;

/** Whether a page whose words have `content` pixels across makes room for a card beside its column. */
export function roomy(content: number): boolean {
  return content - ROOM >= LEAST_COLUMN;
}

/**
 * Where a mark's comments open beside the text: from the left of the
 * scrolling page, how far in and how wide, where there is room to the
 * right of the column the text is in (`column`, where the text ends, and
 * `inside`, where the page's room for its words ends, both from the
 * page's left); nothing where there is not, and they open in the panel beside the document.
 */
export function marginOf(column: number, inside: number): { left: number; width: number } | null {
  const left = column + CARD.gap;
  const width = Math.min(CARD.most, inside - CARD.edge - left);
  return width >= CARD.least ? { left, width } : null;
}

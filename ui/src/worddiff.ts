// What changed between two texts, word by word: what is the same, what was
// taken out and what was put in, in the order they are read, so that the two
// can be drawn as one text with what changed marked (a topic's History,
// `views/topichistory.tsx`). The lines are matched first, and the words of
// each run of lines that changed after, which keeps a long document's
// difference quick and reads as a person would mark it. Written here, with no
// library, as Myers' way of finding the fewest changes.

/** A run of the text: the same in both, only in the older (removed), or only in the newer (added). */
export interface Piece {
  kind: 'same' | 'added' | 'removed';
  text: string;
}

/** Past this many changes between two runs of lines, or of words, the runs are said as taken out whole and put in whole. */
const MOST_LINES = 2000;
const MOST_WORDS = 1000;

type Step = '=' | '-' | '+';

/**
 * The fewest steps that make `older` into `newer`: kept (`=`), taken out
 * (`-`) and put in (`+`), each of one element, in order; or nothing where
 * more than `most` would be taken out and put in, which is too far apart to
 * be worth saying word for word.
 */
export function steps<T>(older: readonly T[], newer: readonly T[], most: number): Step[] | null {
  const n = older.length;
  const m = newer.length;
  const max = n + m;
  const offset = max + 1;
  // Where each diagonal has reached, by `k` = x - y; and as it was at the start of each round, kept for going back.
  const reached = new Int32Array(2 * max + 3);
  const rounds: Int32Array[] = [];
  for (let d = 0; d <= max; d += 1) {
    if (d > most) return null;
    rounds.push(reached.slice(offset - d, offset + d + 1));
    for (let k = -d; k <= d; k += 2) {
      const down = k === -d || (k !== d && reached[offset + k - 1]! < reached[offset + k + 1]!);
      let x = down ? reached[offset + k + 1]! : reached[offset + k - 1]! + 1;
      let y = x - k;
      while (x < n && y < m && older[x] === newer[y]) {
        x += 1;
        y += 1;
      }
      reached[offset + k] = x;
      if (x >= n && y >= m) return back(rounds, n, m);
    }
  }
  return back(rounds, n, m);
}

/** The steps, read back from the end through what each round reached. */
function back(rounds: Int32Array[], n: number, m: number): Step[] {
  const said: Step[] = [];
  let x = n;
  let y = m;
  for (let d = rounds.length - 1; d > 0; d -= 1) {
    const round = rounds[d]!;
    const at = (k: number) => round[k + d]!;
    const k = x - y;
    const down = k === -d || (k !== d && at(k - 1) < at(k + 1));
    const before = down ? k + 1 : k - 1;
    const beforeX = at(before);
    const beforeY = beforeX - before;
    while (x > beforeX && y > beforeY) {
      said.push('=');
      x -= 1;
      y -= 1;
    }
    if (down) {
      said.push('+');
      y -= 1;
    } else {
      said.push('-');
      x -= 1;
    }
  }
  while (x > 0 && y > 0) {
    said.push('=');
    x -= 1;
    y -= 1;
  }
  return said.reverse();
}

/** A text's lines, each with its line's end where it has one, so that they make the text again. */
export function linesOf(text: string): string[] {
  return text.match(/[^\n]*\n|[^\n]+$/g) ?? [];
}

const segmenter = typeof Intl !== 'undefined' && 'Segmenter' in Intl ? new Intl.Segmenter(undefined, { granularity: 'word' }) : null;

/** A text's words, the room between them and each mark apart, as a reader tells them in any script; together, the text again. */
export function wordsOf(text: string): string[] {
  if (segmenter) return Array.from(segmenter.segment(text), (one) => one.segment);
  return text.match(/\s+|[\p{L}\p{N}\p{M}_]+|[^\s\p{L}\p{N}\p{M}_]/gu) ?? [];
}

/** Pieces of the same kind side by side, as one. */
function joined(pieces: Piece[]): Piece[] {
  const out: Piece[] = [];
  for (const piece of pieces) {
    if (!piece.text) continue;
    const last = out.at(-1);
    if (last && last.kind === piece.kind) last.text += piece.text;
    else out.push({ ...piece });
  }
  return out;
}

/** Two runs of elements as pieces, by their steps; or taken out whole and put in whole where they are too far apart. */
function piecesOf(older: readonly string[], newer: readonly string[], most: number): Piece[] {
  const said = steps(older, newer, most);
  if (!said) return [
    { kind: 'removed', text: older.join('') },
    { kind: 'added', text: newer.join('') },
  ];
  const out: Piece[] = [];
  let x = 0;
  let y = 0;
  for (const step of said) {
    if (step === '=') {
      out.push({ kind: 'same', text: older[x]! });
      x += 1;
      y += 1;
    } else if (step === '-') out.push({ kind: 'removed', text: older[x++]! });
    else out.push({ kind: 'added', text: newer[y++]! });
  }
  return out;
}

/**
 * What changed from `older` to `newer`, as the newer is read with what was
 * taken out where it stood: lines matched first, then the words of each run
 * of lines taken out and put in at the same place. Where a run changed in
 * every word, it is said taken out whole, then put in whole.
 */
export function diff(older: string, newer: string): Piece[] {
  if (older === newer) return older ? [{ kind: 'same', text: older }] : [];
  const lines = piecesOf(linesOf(older), linesOf(newer), MOST_LINES);
  const out: Piece[] = [];
  let removed = '';
  let added = '';
  const flush = () => {
    if (removed && added) {
      const words = piecesOf(wordsOf(removed), wordsOf(added), MOST_WORDS);
      // What is in common that is only the room between words and marks says nothing: the run changed whole.
      const kept = words.some((piece) => piece.kind === 'same' && /[\p{L}\p{N}]/u.test(piece.text));
      out.push(...(kept ? words : [{ kind: 'removed' as const, text: removed }, { kind: 'added' as const, text: added }]));
    } else if (removed) out.push({ kind: 'removed', text: removed });
    else if (added) out.push({ kind: 'added', text: added });
    removed = '';
    added = '';
  };
  for (const piece of lines) {
    if (piece.kind === 'same') {
      flush();
      out.push(piece);
    } else if (piece.kind === 'removed') removed += piece.text;
    else added += piece.text;
  }
  flush();
  return joined(out);
}

/** The text a difference was made from (`older`), or the one it was made to (`newer`): what is the same, and what only that one has. */
export function sideOf(pieces: readonly Piece[], side: 'older' | 'newer'): string {
  const left = side === 'older' ? 'removed' : 'added';
  return pieces
    .filter((piece) => piece.kind === 'same' || piece.kind === left)
    .map((piece) => piece.text)
    .join('');
}

/** How many words were put in and taken out, as a line of the history says it beside what changed. */
export function counted(pieces: readonly Piece[]): { added: number; removed: number } {
  const words = (text: string) => wordsOf(text).filter((one) => one.trim() !== '').length;
  let added = 0;
  let removed = 0;
  for (const piece of pieces) {
    if (piece.kind === 'added') added += words(piece.text);
    if (piece.kind === 'removed') removed += words(piece.text);
  }
  return { added, removed };
}

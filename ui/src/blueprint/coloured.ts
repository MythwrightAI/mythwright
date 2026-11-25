// A blueprint's text in pieces, each with what it is, for showing one that
// is only read (an example under a word of the Reference, or on the card
// about a word pointed at): coloured as the editor colours it, by the
// same reading (`scan.ts`) and the same classes (`bp-<what it is>` in
// the style sheet), with no editor.

import type { BlueprintLanguage } from '../api.ts';
import { scanLine, start } from './scan.ts';
import type { TokenType } from './scan.ts';

export interface Piece {
  text: string;
  /** What it is, where it is anything: a piece between two is the line's own space. */
  type?: TokenType;
  /** Inside a `${ }`. */
  held?: true;
}

/** Each line of a text as its pieces, which together are the line as it is written. */
export function coloured(text: string, language: BlueprintLanguage | null): Piece[][] {
  let state = start(language);
  return text.replace(/\n$/, '').split('\n').map((line) => {
    const read = scanLine(line, state, language);
    state = read.state;
    const pieces: Piece[] = [];
    let at = 0;
    for (const token of read.tokens) {
      if (token.from > at) pieces.push({ text: line.slice(at, token.from) });
      pieces.push({ text: line.slice(token.from, token.to), type: token.type, ...(token.held ? { held: true as const } : {}) });
      at = token.to;
    }
    if (at < line.length) pieces.push({ text: line.slice(at) });
    return pieces;
  });
}

/** The classes a piece is drawn with: the editor's own. */
export function classOf(piece: Piece): string {
  return [piece.type && `bp-${piece.type}`, piece.held && 'bp-held'].filter(Boolean).join(' ');
}

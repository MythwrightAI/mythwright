// The web addresses in words somebody wrote, as links: a function of
// plain text with nothing of the page in it, so that its tests can read
// it.
//
// Only an address on the web is a link (`http://`, `https://`), and only
// one `supportLink` in `help.ts` would draw too, so that nothing the
// client offers to open goes by two rules: never a script, never data,
// never a file of the device. Where it ends is where words would say it
// ends: a full stop, a comma or a bracket after it is the sentence's.

import { supportLink } from './help.ts';

/** A piece of the words: what it says, and where it leads where it is an address. */
export interface Piece {
  text: string;
  href: string | null;
}

// Where an address may begin: `http://` or `https://`, not inside a word (`xhttp://`).
const BEGINS = /(?<![\p{L}\p{N}_])https?:\/\//giu;
// What ends an address: a space, and what no address holds written plain.
const ENDS = /[\s"'<>`]/u;
// What a sentence puts after an address and is not part of it.
const AFTER = /[.,;:!?…]$/u;
const CLOSES: Record<string, string> = { ')': '(', ']': '[', '}': '{' };

/** How many of `char` there are in `text`. */
function times(text: string, char: string): number {
  return text.split(char).length - 1;
}

/** An address as it is written in words, with what the sentence put after it taken off. */
function trimmed(found: string): string {
  let address = found;
  for (;;) {
    const last = address.at(-1) ?? '';
    if (AFTER.test(last)) address = address.slice(0, -1);
    // A closing bracket is the address's only where it closes one the address opened: `…/Foo_(bar)`.
    else if (CLOSES[last] && times(address, last) > times(address, CLOSES[last])) address = address.slice(0, -1);
    else return address;
  }
}

/** Whether an address leads somewhere: it says a place after its `//`, and is one `supportLink` would open. */
function leads(address: string): boolean {
  // A place is written straight after the `//`: `https:///x` is read by a browser as a place called `x`, which nobody wrote.
  if (!supportLink(address) || !/^https?:\/\/[^/\\?#]/i.test(address)) return false;
  try {
    const url = new URL(address);
    return (url.protocol === 'http:' || url.protocol === 'https:') && url.hostname !== '';
  } catch {
    return false;
  }
}

/**
 * The words in pieces, in order: each web address a piece that leads
 * there, and the words between pieces that lead nowhere. Put together
 * the pieces are the words, letter for letter. Words with no address are
 * one piece, and no words are none.
 */
export function piecesOf(words: string): Piece[] {
  const pieces: Piece[] = [];
  let from = 0;
  for (const match of words.matchAll(BEGINS)) {
    const at = match.index ?? 0;
    // One found inside an address already taken is part of it.
    if (at < from) continue;
    const rest = words.slice(at);
    const end = rest.search(ENDS);
    const address = trimmed(end === -1 ? rest : rest.slice(0, end));
    if (!leads(address)) continue;
    if (at > from) pieces.push({ text: words.slice(from, at), href: null });
    pieces.push({ text: address, href: address });
    from = at + address.length;
  }
  if (from < words.length) pieces.push({ text: words.slice(from), href: null });
  return pieces;
}

/** The addresses in the words, each once, in the order they are first written. */
export function addressesIn(words: string): string[] {
  return [...new Set(piecesOf(words).flatMap((piece) => (piece.href ? [piece.href] : [])))];
}

/** What a link to an address is called where it stands by itself: where it leads, with no `http://` before it and no `/` after a place alone. */
export function shortOf(address: string): string {
  return address.replace(/^https?:\/\//i, '').replace(/^([^/?#]*)\/$/, '$1');
}

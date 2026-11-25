// What is written in a channel, read as what it is meant to look like.
//
// A message is text, and stays text: an agent reads it as it was written,
// and so can a person. A few marks in it are drawn as what they mean, the
// ones the composer's tools write and the ones agents write unasked:
//
//   **bold**  _italic_  *italic*  ++underlined++  ~~struck~~  `code`
//   [words](https://somewhere)  and an address written out
//   # a heading (to ###)    > a quotation
//   - an item    1. a numbered item    (further in when written further in)
//   ``` a block of code, between two of these ```
//       or every line of it further in, after an empty line
//   ![a caption](attachment:<id> "side")   a picture (`pictures.ts`)
//   [words](topic:<id>)    a link to another page (`pagelinks.ts`)
//   words.[^1]   and, on a line of its own,   [^1]: where it comes from   (`cited.ts`)
//   | a | b |   then   |---|--:|   then a line a row: a table (`tables.ts`),
//       whose cells hold what a line holds
//
// Deliberately not Markdown: no HTML, and nothing but a table that needs
// more than the line it is on to be understood (a table is its lines, and
// ends at the first that is not one of its rows). What is not one of these
// is shown as it was written.
//
// Each kind of block is read in one place and drawn in one place: a line
// that is a block of its own is read by one of `STANDING` here, and every
// kind is drawn by its entry in `DRAWN` (`ui.tsx`), which the types hold
// to every kind there is. A mark inside a line is one more of `MARKS`.

import { inBy, partsOf, wordsOfItemAt } from './codeblocks.ts';
import { figureOf, PICTURE, pictureOf } from './pictures.ts';
import type { Picture } from './pictures.ts';
import { pageLinkOf } from './pagelinks.ts';
import { tableAt } from './tables.ts';
import type { Align } from './tables.ts';

export type Span =
  | { is: 'text'; text: string }
  | { is: 'code'; text: string }
  | { is: 'bold' | 'italic' | 'underline' | 'strike'; inside: Span[] }
  | { is: 'link'; to: string; inside: Span[] }
  // A link to a page of the topics, by its id, or to one of its replies.
  | { is: 'page'; topic: string; reply: number | null; inside: Span[] }
  // A footnote's marker, by its label.
  | { is: 'cite'; label: string }
  | ({ is: 'image' } & Picture);

/** One thing in a list: how far in it is, what marks it (a dot, or the number it was given), and what it says. */
export interface Item {
  depth: number;
  mark: string;
  spans: Span[];
}

export type Block =
  | { is: 'text'; spans: Span[] }
  | { is: 'heading'; level: 1 | 2 | 3; spans: Span[] }
  | { is: 'quote'; spans: Span[] }
  | { is: 'list'; items: Item[] }
  | { is: 'code'; text: string; lang?: string }
  | ({ is: 'figure' } & Picture)
  // A table: where each column's words stand, its headings, and its rows, each cell read as a line is.
  | { is: 'table'; align: Align[]; head: Span[][]; rows: Span[][][] }
  // A footnote's source, as it was written after its label and its words read.
  | { is: 'note'; label: string; text: string; spans: Span[] };

const NOTE = /^\[\^([^\]\s]+)\]:[ \t]*(.*)$/;

/**
 * The lines that are a block of their own, each read by one function: the
 * first that reads a line as a block has it. A line that is nothing but a
 * picture is a figure.
 */
const STANDING: ((line: string) => Block | null)[] = [
  (line) => {
    const picture = figureOf(line);
    return picture ? { is: 'figure', ...picture } : null;
  },
  (line) => {
    const note = NOTE.exec(line);
    return note ? { is: 'note', label: note[1]!, text: note[2]!.trim(), spans: uncited(spans(note[2]!.trim())) } : null;
  },
];

// What may be inside a pair of marks: something that neither starts nor
// ends with a space or with the mark, on one line. `2 * 3 * 4` and `c++`
// are then not italic or underlined.
const within = (mark: string) => `([^\\s${mark}]|[^\\s${mark}][^\\n]*?[^\\s${mark}])`;

// The marks, in the order they are looked for. A `*` or `_` for italics
// must stand at the edge of a word, or `snake_case_names` would lean; what
// stands before it is caught with it, and put back as it was. An address
// written out ends before the full stop or bracket that follows it.
const MARKS = new RegExp(
  [
    '`([^`\\n]+)`', // 1
    '\\[([^\\]\\n]+)\\]\\(((?:https?://|mailto:|topic:)[^)\\s]+)\\)', // 2, 3
    `\\*\\*${within('*')}\\*\\*`, // 4
    `~~${within('~')}~~`, // 5
    `\\+\\+${within('+')}\\+\\+`, // 6
    `(^|[^\\w*])\\*${within('*')}\\*(?![\\w*])`, // 7, 8
    `(^|\\W)_${within('_')}_(?!\\w)`, // 9, 10
    '(^|[^\\w/.@-])(https?://[^\\s<>]*[^\\s<>.,;:!?)\\]\'"])', // 11, 12
    PICTURE, // 13, 14, 15
    '\\[\\^([^\\]\\s]+)\\]', // 16
  ].join('|'),
  'g',
);

/** Spans with each footnote's marker in them read as the words it was: a source cites nothing, or it would be drawn inside itself. */
function uncited(all: Span[]): Span[] {
  return all.map((span): Span => (span.is === 'cite' ? { is: 'text', text: `[^${span.label}]` } : 'inside' in span ? { ...span, inside: uncited(span.inside) } : span));
}

/** The marks in a run of text, each with what is inside it read the same way. */
export function spans(text: string): Span[] {
  const found: Span[] = [];
  const plain = (words: string) => {
    if (words === '') return;
    const last = found.at(-1);
    if (last?.is === 'text') last.text += words;
    else found.push({ is: 'text', text: words });
  };
  let at = 0;
  const marks = new RegExp(MARKS.source, 'g');
  for (let match = marks.exec(text); match; match = marks.exec(text)) {
    plain(text.slice(at, match.index));
    at = match.index + match[0].length;
    if (match[1] !== undefined) found.push({ is: 'code', text: match[1] });
    else if (match[2] !== undefined) {
      const page = match[3]!.startsWith('topic:') ? pageLinkOf(match[3]!) : undefined;
      // An address that begins as a page's and names none is left as it was written.
      if (page === null) plain(match[0]);
      else if (page) found.push({ is: 'page', ...page, inside: spans(match[2]) });
      else found.push({ is: 'link', to: match[3]!, inside: spans(match[2]) });
    } else if (match[16] !== undefined) found.push({ is: 'cite', label: match[16] });
    else if (match[4] !== undefined) found.push({ is: 'bold', inside: spans(match[4]) });
    else if (match[5] !== undefined) found.push({ is: 'strike', inside: spans(match[5]) });
    else if (match[6] !== undefined) found.push({ is: 'underline', inside: spans(match[6]) });
    else if (match[8] !== undefined) {
      plain(match[7]!);
      found.push({ is: 'italic', inside: spans(match[8]) });
    } else if (match[10] !== undefined) {
      plain(match[9]!);
      found.push({ is: 'italic', inside: spans(match[10]) });
    } else if (match[14] !== undefined) {
      found.push({ is: 'image', ...pictureOf(match[13]!, match[14], match[15]) });
    } else {
      plain(match[11]!);
      found.push({ is: 'link', to: match[12]!, inside: [{ is: 'text', text: match[12]! }] });
    }
  }
  plain(text.slice(at));
  return found;
}

const HEADING = /^(#{1,3}) +(\S.*)$/;
const QUOTE = /^> ?(.*)$/;
const ITEM = /^( *)(?:[-*•]|(\d{1,9})[.)]) +(\S.*)$/;

/** Whether a line is empty: nothing on it but spaces. */
const empty = (line: string) => line.trim() === '';

/** The lines outside any block of code: headings, quotations, lists, and the rest as it was written. */
function lines(part: string): Block[] {
  const blocks: Block[] = [];
  let words: string[] = [];
  let quoted: string[] = [];
  // Each item as it was written, with where its words begin: a line that far in, or as far in as any item's words, goes on with the last.
  let items: { depth: number; mark: string; lines: string[]; at: number }[] = [];
  const close = () => {
    // Empty lines at either end only kept this apart from what is next to it.
    const text = words.join('\n').replace(/^\n+|\s+$/g, '');
    if (text !== '') blocks.push({ is: 'text', spans: spans(text) });
    if (quoted.length > 0) blocks.push({ is: 'quote', spans: spans(quoted.join('\n')) });
    if (items.length > 0) blocks.push({ is: 'list', items: items.map((item) => ({ depth: item.depth, mark: item.mark, spans: spans(item.lines.join('\n').replace(/\s+$/, '')) })) });
    words = [];
    quoted = [];
    items = [];
  };
  // Whether a line goes on with the list there is: it is no item, and is as far in as some item's words.
  const goesOn = (line: string) => items.length > 0 && !empty(line) && wordsOfItemAt(line) === null && items.some((item) => item.at <= inBy(line));
  const all = part.split('\n');
  // The lines a table took, past which the next line is read.
  let taken = 0;
  all.forEach((line, at) => {
    if (at < taken) return;
    const heading = HEADING.exec(line);
    const quote = QUOTE.exec(line);
    const item = ITEM.exec(line);
    const last = items.at(-1);
    const table = last && goesOn(line) ? null : tableAt(all, at);
    const standing = last && goesOn(line) ? null : STANDING.reduce<Block | null>((found, read) => found ?? read(line), null);
    if (last && goesOn(line)) {
      // A paragraph's lines begin where its words do, however far in each was written.
      last.lines.push(line.trimStart());
    } else if (table) {
      close();
      blocks.push({ is: 'table', align: table.align, head: table.head.map((cell) => spans(cell)), rows: table.rows.map((row) => row.map((cell) => spans(cell))) });
      taken = at + table.lines;
    } else if (last && empty(line) && goesOn(all.slice(at + 1).find((next) => !empty(next)) ?? '')) {
      // An empty line between an item and its next paragraph is the item's.
      last.lines.push('');
    } else if (last && empty(line) && ITEM.test(all.slice(at + 1).find((next) => !empty(next)) ?? '')) {
      // And one between two items keeps them one list.
    } else if (standing) {
      close();
      blocks.push(standing);
    } else if (heading) {
      close();
      blocks.push({ is: 'heading', level: heading[1]!.length as 1 | 2 | 3, spans: spans(heading[2]!) });
    } else if (quote) {
      if (quoted.length === 0) close();
      quoted.push(quote[1]!);
    } else if (item) {
      if (items.length === 0) close();
      items.push({ depth: Math.min(Math.floor(item[1]!.length / 2), 4), mark: item[2] ? `${item[2]}.` : '•', lines: [item[3]!], at: wordsOfItemAt(line) ?? 0 });
    } else {
      if (quoted.length > 0 || items.length > 0) close();
      words.push(line);
    }
  });
  close();
  return blocks;
}

/** A message as the blocks it is drawn as: its blocks of code where `codeblocks.ts` finds them, and the lines between. */
export function read(text: string): Block[] {
  return partsOf(text).flatMap((part): Block[] => (part.is === 'code' ? [{ is: 'code', text: part.text, ...(part.lang ? { lang: part.lang } : {}) }] : lines(part.text)));
}

/**
 * The cards named in a run of text, by the ids they have: `shop-12`, where
 * `shop` is a project. What is not a card's id is left as the words it
 * was, so `utf-8` and a date stay what they are; a project's name inside a
 * longer word or a path is not a card either.
 */
export function cardsIn(text: string, projects: string[]): (string | { card: string; project: string })[] {
  if (projects.length === 0) return [text];
  // The longest first, so that `shop-front-3` is a card of `shop-front` where there is one.
  const named = [...projects].sort((a, b) => b.length - a.length).map((id) => id.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'));
  const ids = new RegExp(`(^|[^\\w/.-])((${named.join('|')})-\\d+)(?![\\w-])`, 'g');
  const parts: (string | { card: string; project: string })[] = [];
  let at = 0;
  for (let match = ids.exec(text); match; match = ids.exec(text)) {
    const from = match.index + match[1]!.length;
    if (from > at) parts.push(text.slice(at, from));
    parts.push({ card: match[2]!, project: match[3]! });
    at = from + match[2]!.length;
  }
  if (at < text.length) parts.push(text.slice(at));
  return parts;
}

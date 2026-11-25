// Where the blocks of code are in what somebody wrote, and what each
// holds: a function of plain text with nothing of the page in it, so
// that its tests can read it.
//
// A block of code is written two ways, as Markdown has it:
//
//   ``` between two of these, with what it is written in after the
//   first on its line if anybody says ``` (kept as its `lang`)
//
//       or every line of it four spaces or a tab further in, begun after
//       an empty line or where the text begins
//
// What a block holds is its text as it was written, and nothing else:
// not the marks round it, not the line break before the closing marks,
// not the four spaces or the tab that put each line of an indented one
// further in; a tab inside it stays a tab, and no line break is added at
// its end. That is what its Copy button puts on the clipboard.
//
// A line further in that is an item of a list (`    - inside`) is that
// item, as it always was, and so is one that goes on from the line above
// it with no empty line between: neither is code.
//
// Under an item of a list a line goes on with that item, as Markdown has
// it: where the item's words begin is how far in its lines are written
// (two for `- one`, three for `1. one`), so a line that far in after an
// empty line is the item's next paragraph and no block of code, and a
// block of code inside the item is four further in than that. The list
// has ended at a line, after an empty line, that is less far in than the
// words of every item it had; four in after that is code again.

/** A part of what was written: a block of code, or the words between. */
export type Part = { is: 'code'; text: string; lang?: string } | { is: 'words'; text: string };

// The marks that open or close a block, with what it is written in after the first, which is kept.
const FENCE = /```(?:([\w+#.-]*)\n)?/;
// An item of a list, however far in, as `prose.ts` reads these: how far in, its mark, and the spaces after it.
const ITEM_AT = /^( *)([-*•]|\d{1,9}[.)])( +)\S/;

/** Whether a line is empty: nothing on it but spaces. */
const empty = (line: string) => line.trim() === '';

/** How far in a line is written, a tab going on to the next four. */
export function inBy(line: string): number {
  let column = 0;
  for (const letter of line) {
    if (letter === ' ') column += 1;
    else if (letter === '\t') column += 4 - (column % 4);
    else break;
  }
  return column;
}

/** Where an item's words begin: past how far in it is, its mark, and the spaces after the mark (one, where there are more than four). */
export function wordsOfItemAt(line: string): number | null {
  const item = ITEM_AT.exec(line);
  if (!item) return null;
  const spaces = item[3]!.length > 4 ? 1 : item[3]!.length;
  return item[1]!.length + item[2]!.length + spaces;
}

/** A line with what puts it so far in taken off its front, a tab going on to the next four. */
function outBy(line: string, far: number): string {
  let column = 0;
  let at = 0;
  while (at < line.length && column < far && (line[at] === ' ' || line[at] === '\t')) {
    column += line[at] === '\t' ? 4 - (column % 4) : 1;
    at += 1;
  }
  return line.slice(at);
}

/** The indented blocks among words with no fenced block in them, and the words between. */
function indented(text: string): Part[] {
  const lines = text.split('\n');
  const parts: Part[] = [];
  let words: string[] = [];
  const said = () => {
    if (words.length > 0) parts.push({ is: 'words', text: words.join('\n') });
    words = [];
  };
  // Where the words of each item of a list that is still open begin, the outermost first.
  let items: number[] = [];
  for (let at = 0; at < lines.length; ) {
    const line = lines[at]!;
    const item = wordsOfItemAt(line);
    const afterEmpty = at === 0 || empty(lines[at - 1]!);
    if (item !== null) {
      // An item is beside those whose words begin no further in than its mark, and under them.
      items = [...items.filter((words) => words <= inBy(line)), item];
      words.push(line);
      at += 1;
      continue;
    }
    // After an empty line, a line goes on with the innermost item it is as far in as; with none, the list has ended.
    if (afterEmpty && !empty(line)) items = items.filter((words) => words <= inBy(line));
    // A block of code is four further in than the words of the item it is in, or than the margin.
    const far = (items.at(-1) ?? 0) + 4;
    const begins = afterEmpty && !empty(line) && inBy(line) >= far;
    if (!begins) {
      words.push(line);
      at += 1;
      continue;
    }
    // It goes on through lines that far in and empty lines, and ends at its last line that far in.
    let end = at;
    for (let next = at; next < lines.length && (empty(lines[next]!) || inBy(lines[next]!) >= far); next += 1) {
      if (!empty(lines[next]!)) end = next;
    }
    said();
    const code = lines.slice(at, end + 1).map((one) => outBy(one, far));
    parts.push({ is: 'code', text: code.join('\n') });
    at = end + 1;
  }
  said();
  return parts;
}

/**
 * What was written, in parts, in order: each block of code with the text
 * it holds, and the words between them as they were written, but for the
 * empty lines that only kept a block apart from them. Words with no block
 * of code in them are one part, letter for letter; no words are none.
 */
export function partsOf(text: string): Part[] {
  const parts: Part[] = [];
  // Split by the marks, each followed by what they said it is written in (`undefined` where they said nothing).
  const split = text.split(FENCE);
  for (let at = 0; at < split.length; at += 2) {
    const part = split[at]!;
    if ((at / 2) % 2 === 0) {
      parts.push(...indented(part));
      continue;
    }
    // A fenced block's text ends before the line break that comes before its closing marks.
    const lang = split[at - 1]?.trim().toLowerCase();
    parts.push({ is: 'code', text: part.replace(/\n$/, ''), ...(lang ? { lang } : {}) });
  }
  // The line breaks beside a block are the block's edges, and not words of their own.
  return parts
    .map((part, at): Part => {
      if (part.is === 'code') return part;
      let words = part.text;
      if (parts[at - 1]?.is === 'code') words = words.replace(/^(?:[ \t]*\n)+/, '');
      if (parts[at + 1]?.is === 'code') words = words.replace(/(?:\n[ \t]*)+$/, '').replace(/^[ \t]*$/, '');
      return { is: 'words', text: words };
    })
    .filter((part) => part.is === 'code' || part.text !== '');
}

/** What a block's Copy button says it has done: nothing yet, copied, or could not. */
export type Copied = 'idle' | 'copied' | 'failed';

/** What the button reads while it says so. */
export function copyWord(copied: Copied): string {
  return copied === 'copied' ? 'Copied' : copied === 'failed' ? 'Failed' : 'Copy';
}

/** What is said, for whoever hears it read out, as it has done it. */
export function copySaid(copied: Copied): string {
  return copied === 'copied' ? 'The code was copied.' : copied === 'failed' ? 'The code could not be copied from here: select it and copy it.' : '';
}

/** The languages a block of code is a diagram in, where another program would draw it. */
const DIAGRAMS = ['mermaid'];

/**
 * What draws a block written in a language, of those that draw some
 * (`RENDERERS` in `views/codeblock.tsx`), by the language's name in small
 * letters: none for a block that says no language, or one nothing draws.
 * Only what was set up there counts, never what every object has (a block
 * written in `constructor` is its code).
 */
export function drawerOf<T>(lang: string | undefined, drawers: Record<string, T>): T | null {
  return lang && Object.hasOwn(drawers, lang) ? drawers[lang]! : null;
}

/** What is said under a block that is a diagram nothing here draws (`drawn` are the languages something does): it stays its code, said to be so. */
export function undrawnSaid(lang: string | undefined, drawn: string[]): string | null {
  return lang && DIAGRAMS.includes(lang) && !drawn.includes(lang) ? 'Diagram drawing is not set up in this version.' : null;
}

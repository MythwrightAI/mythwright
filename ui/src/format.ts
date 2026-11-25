// The composer's tools: bold, a list, a quotation and the rest.
//
// What is written in a channel is text, and a tool changes that text: it
// puts the marks round what is selected that `prose.ts` reads back as what
// they mean, or takes them away when they are there already. So what a
// tool did can be seen, undone and written by hand, and an agent reads
// what a person reads.
//
// Each tool is a function from what is written, and what of it is
// selected, to the same after the tool. Nothing here knows of the page.

/** What is written, and what of it is selected: a caret when `from` is `to`. */
export interface Written {
  text: string;
  from: number;
  to: number;
}

export type Tool = 'bold' | 'italic' | 'underline' | 'strike' | 'code' | 'link' | 'numbers' | 'bullets' | 'quote' | 'block' | 'text' | 'h1' | 'h2' | 'h3';

/** The marks that go either side of what a tool is used on. */
const AROUND: Partial<Record<Tool, string>> = { bold: '**', italic: '_', underline: '++', strike: '~~', code: '`' };

/** What begins a line a tool has been used on, and how such a line is known. */
const BEGINS: Partial<Record<Tool, { put: (nth: number) => string; has: RegExp }>> = {
  bullets: { put: () => '- ', has: /^[-*•] +/ },
  numbers: { put: (nth) => `${nth}. `, has: /^\d+[.)] +/ },
  quote: { put: () => '> ', has: /^> ?/ },
  h1: { put: () => '# ', has: /^# +/ },
  h2: { put: () => '## ', has: /^## +/ },
  h3: { put: () => '### ', has: /^### +/ },
  text: { put: () => '', has: /^(?!)/ },
};
/** Anything a line may begin with that says what kind of line it is. */
const ANY_BEGINNING = /^(?:[-*•] +|\d+[.)] +|> ?|#{1,3} +)/;

const FENCE = '```';

/** Puts `mark` either side of what is selected, line by line, or takes it away where every line has it. */
function around(written: Written, mark: string): Written {
  const { text } = written;
  // Without the spaces at its ends: the marks go against the words.
  let { from, to } = written;
  while (from < to && /\s/.test(text[from]!)) from++;
  while (to > from && /\s/.test(text[to - 1]!)) to--;
  if (from === to) {
    const at = written.from;
    // Between a pair with nothing in it, the pair goes: the tool was pressed, and then pressed again.
    if (text.slice(at - mark.length, at) === mark && text.slice(at, at + mark.length) === mark) {
      return { text: text.slice(0, at - mark.length) + text.slice(at + mark.length), from: at - mark.length, to: at - mark.length };
    }
    return { text: text.slice(0, at) + mark + mark + text.slice(at), from: at + mark.length, to: at + mark.length };
  }

  // Each line of what is selected, as where its words begin and end.
  const parts: { from: number; to: number; left: 'out' | 'in' | null; right: 'out' | 'in' | null }[] = [];
  let begin = from;
  for (const line of text.slice(from, to).split('\n')) {
    let start = begin;
    let end = begin + line.length;
    begin = end + 1;
    while (start < end && /\s/.test(text[start]!)) start++;
    while (end > start && /\s/.test(text[end - 1]!)) end--;
    if (start === end) continue;
    const words = text.slice(start, end);
    // Each of its marks is just outside what is selected, or is the end of it.
    const left = text.slice(start - mark.length, start) === mark ? 'out' : words.startsWith(mark) ? 'in' : null;
    const right = text.slice(end, end + mark.length) === mark ? 'out' : words.endsWith(mark) ? 'in' : null;
    const room = (left === 'in' ? mark.length : 0) + (right === 'in' ? mark.length : 0);
    parts.push(left && right && words.length > room ? { from: start, to: end, left, right } : { from: start, to: end, left: null, right: null });
  }
  const marked = parts.every((part) => part.left);

  // Written again from the start, each line with its marks put or taken.
  let next = '';
  let at = 0;
  let first = -1;
  let last = 0;
  for (const part of parts) {
    let start: number;
    if (marked) {
      next += text.slice(at, part.left === 'out' ? part.from - mark.length : part.from);
      start = next.length;
      next += text.slice(part.left === 'out' ? part.from : part.from + mark.length, part.right === 'out' ? part.to : part.to - mark.length);
      last = next.length;
      at = part.right === 'out' ? part.to + mark.length : part.to;
    } else if (part.left) {
      // It has them, and others do not: it keeps them while they are given theirs.
      next += text.slice(at, part.to);
      start = next.length - (part.to - part.from);
      last = next.length;
      at = part.to;
    } else {
      next += text.slice(at, part.from) + mark;
      start = next.length;
      next += text.slice(part.from, part.to);
      last = next.length;
      next += mark;
      at = part.to;
    }
    if (first < 0) first = start;
  }
  return { text: next + text.slice(at), from: first, to: last };
}

/** The lines that what is selected touches: where the first begins and the last ends. */
function touched(written: Written): { begin: number; end: number } {
  const { text, from } = written;
  // A selection that ends at the start of a line does not touch that line.
  const to = written.to > from && text[written.to - 1] === '\n' ? written.to - 1 : written.to;
  const begin = text.lastIndexOf('\n', from - 1) + 1;
  const after = text.indexOf('\n', to);
  return { begin, end: after === -1 ? text.length : after };
}

/** Begins each line touched as `tool` begins a line, or as plain text where every one of them is begun so already. */
function beginning(written: Written, tool: Tool): Written {
  const { put, has } = BEGINS[tool]!;
  const { text } = written;
  const { begin, end } = touched(written);
  const lines = text.slice(begin, end).split('\n');
  // An empty line among others is left empty; alone, it is where the list is about to be written.
  const counts = (line: string) => lines.length === 1 || line.trim() !== '';
  const body = (line: string) => line.replace(/^ */, '');
  const begun = lines.filter(counts).every((line) => has.test(body(line)));
  let nth = 0;
  let caret = written.from;
  // Where the line being looked at begins: as it was, and as it now is.
  let was = begin;
  let is = begin;
  const changed = lines.map((line) => {
    const rest = counts(line) ? body(line).replace(ANY_BEGINNING, '') : line;
    const now = counts(line) ? line.slice(0, line.length - body(line).length) + (begun ? '' : put(++nth)) + rest : line;
    // A caret keeps its place among the words of its line.
    if (written.from >= was && written.from <= was + line.length) {
      caret = is + (now.length - rest.length) + Math.max(written.from - was - (line.length - rest.length), 0);
    }
    was += line.length + 1;
    is += now.length + 1;
    return now;
  });
  const joined = changed.join('\n');
  const next = text.slice(0, begin) + joined + text.slice(end);
  if (written.from === written.to) return { text: next, from: caret, to: caret };
  return { text: next, from: begin, to: begin + joined.length };
}

/** Makes the lines touched a block of code, or what was in the block when they are one already. */
function block(written: Written): Written {
  const { text } = written;
  const { begin, end } = touched(written);
  const body = text.slice(begin, end);
  const fenced = /^```[^\n]*\n([\s\S]*)\n```$/.exec(body);
  if (fenced) {
    return { text: text.slice(0, begin) + fenced[1]! + text.slice(end), from: begin, to: begin + fenced[1]!.length };
  }
  // With the caret inside a block: the marks either side of it go.
  const before = text.slice(0, begin);
  const after = text.slice(end);
  if (/(^|\n)```[^\n]*\n$/.test(before) && /^\n```(\n|$)/.test(after)) {
    const open = before.lastIndexOf(FENCE);
    return { text: text.slice(0, open) + body + after.slice(FENCE.length + 1), from: open, to: open + body.length };
  }
  const put = `${FENCE}\n${body}\n${FENCE}`;
  const inside = begin + FENCE.length + 1;
  return { text: before + put + after, from: inside, to: inside + body.length };
}

const ADDRESS = /^(?:https?:\/\/|mailto:)\S+$/;

/** Makes what is selected the words of a link, and selects what is still to be written: where it leads, or its words. */
function link(written: Written): Written {
  const { text, from, to } = written;
  const chosen = text.slice(from, to);
  const put = (words: string, where: string, select: 'words' | 'where'): Written => {
    const start = select === 'words' ? from + 1 : from + words.length + 3;
    return {
      text: `${text.slice(0, from)}[${words}](${where})${text.slice(to)}`,
      from: start,
      to: start + (select === 'words' ? words.length : where.length),
    };
  };
  if (ADDRESS.test(chosen)) return put('words', chosen, 'words');
  if (chosen.trim() === '') return put('words', 'https://', 'words');
  return put(chosen, 'https://', 'where');
}

/** What is written after `tool` is used on what is selected. */
export function format(tool: Tool, written: Written): Written {
  const mark = AROUND[tool];
  if (mark) return around(written, mark);
  if (tool === 'link') return link(written);
  if (tool === 'block') return block(written);
  return beginning(written, tool);
}

/**
 * A new line where the caret is, when the line it is on is an item of a
 * list or part of a quotation: the next line begins as that one does, with
 * the next number where they are numbered. On an item with nothing in it
 * the list ends instead. `null` anywhere else: a new line is a new line.
 */
export function newline(written: Written): Written | null {
  const { text, from, to } = written;
  if (from !== to) return null;
  const begin = text.lastIndexOf('\n', from - 1) + 1;
  const line = text.slice(begin, from);
  const begun = /^( *)(?:([-*•])|(\d+)([.)])|(>))( *)(.*)$/.exec(line);
  if (!begun) return null;
  const [, indent, dot, number, stop, quote, , said] = begun;
  if (said === '') {
    // Only where the line ends here: there is nothing after the caret to keep.
    if (text[from] !== undefined && text[from] !== '\n') return null;
    return { text: text.slice(0, begin) + text.slice(from), from: begin, to: begin };
  }
  const next = `\n${indent}${dot ?? quote ?? `${Number(number) + 1}${stop}`} `;
  return { text: text.slice(0, from) + next + text.slice(from), from: from + next.length, to: from + next.length };
}

/** A key pressed, as much of it as says which tool it is for. */
export interface Pressed {
  key: string;
  /** Which key it is on the keyboard, whatever it types: `Digit7`. */
  code?: string;
  ctrlKey: boolean;
  metaKey: boolean;
  shiftKey: boolean;
  altKey: boolean;
}

/** The tool a key asks for, if it asks for one. Ctrl, or ⌘ on a Mac, is held for all of them. */
export function toolFor(pressed: Pressed): Tool | null {
  if (!(pressed.ctrlKey || pressed.metaKey)) return null;
  const key = pressed.key.toLowerCase();
  if (pressed.altKey) return pressed.shiftKey && key === 'c' ? 'block' : null;
  if (!pressed.shiftKey) return key === 'b' ? 'bold' : key === 'i' ? 'italic' : key === 'u' ? 'underline' : null;
  if (key === 'x') return 'strike';
  if (key === 'c') return 'code';
  if (key === 'u') return 'link';
  // With Shift held a number key types something else, and what depends on the keyboard.
  if (pressed.code === 'Digit7') return 'numbers';
  if (pressed.code === 'Digit8') return 'bullets';
  if (pressed.code === 'Digit9') return 'quote';
  return null;
}

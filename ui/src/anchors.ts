// What a comment on a selection of a document's text is about, and where
// that is in the text as it is now.
//
// The document is a Markdown file that other programs write, so nothing
// of a comment is kept in it. What the comment is about is kept with the
// comment (`kernel::anchor`): the words quoted, a few words either side of
// them and the headings over the block they are in. Whoever shows the
// document finds the words again by looking for them, in this order:
//
//   1. the quote with the words either side of it, under the same headings
//      first and then anywhere;
//   2. the quote alone under the same headings;
//   3. the quote anywhere;
//   4. not found: the comment is then one on the whole document that says
//      what it was about, and is never dropped.
//
// Where the quote is found more than once at a stage, the one whose
// neighbours are most like the words kept either side of it is taken, and
// of those the first.
//
// A document is looked at as blocks of plain text (a paragraph, a heading,
// an item of a list, a block of code), each with the headings over it and
// where each of its characters is in whatever it was read from: the block
// editor's positions, or the Markdown file's own offsets, which the file
// is read into here as it would be shown (`blocksOfMarkdown`), with what
// only marks the text up (`**`, a link's address) left out. Within a block,
// space of any kind is one space, so that a paragraph wrapped over lines
// in the file and the same one written in blocks read alike.
//
// Everything here is a function of plain values, with its cases
// (`anchors.test.ts`, over `anchorcases.json`).

/** What a comment on a few words is about, as it is kept with the comment. */
export interface Anchor {
  quote: string;
  before?: string;
  after?: string;
  under?: string[];
}

/** A block of a document as plain text: `at[i]` is where its `i`th character is in what it was read from. */
export interface Block {
  text: string;
  under: string[];
  at: number[];
}

/** Where an anchor's words are now: a block, the characters within it, and at which stage they were found. */
export interface Found {
  block: number;
  from: number;
  to: number;
  how: 'exact' | 'heading' | 'anywhere';
}

/** The most characters quoted, kept either side, of headings and of each heading, as the daemon holds them (`kernel::anchor`). */
export const MOST_QUOTED = 400;
export const BESIDE = 32;
export const MOST_HEADINGS = 6;
export const MOST_HEADING = 200;

/** Words as a block reads them: any run of space one space. */
export function spaced(words: string): string {
  return words.replace(/\s+/g, ' ');
}

/** Characters with where each is, put together into a block: space run together and taken off both ends. */
export class Gathering {
  private text: string[] = [];
  private at: number[] = [];
  push(char: string, at: number): void {
    if (/\s/.test(char)) {
      // One space for a run, and none at the start.
      if (this.text.length === 0 || this.text.at(-1) === ' ') return;
      this.text.push(' ');
    } else this.text.push(char);
    this.at.push(at);
  }
  /** The block, or nothing where it holds no words. */
  done(under: string[]): Block | null {
    while (this.text.at(-1) === ' ') {
      this.text.pop();
      this.at.pop();
    }
    if (this.text.length === 0) return null;
    const block = { text: this.text.join(''), under: [...under], at: this.at };
    this.text = [];
    this.at = [];
    return block;
  }
}

/** What headings stand over a block, kept as they come: a heading of a level ends those of its level and below. */
export class Headings {
  private kept: { level: number; text: string }[] = [];
  now(): string[] {
    return this.kept.map((one) => one.text);
  }
  /** What stands over a heading of `level`: those of the levels above it. */
  over(level: number): string[] {
    return this.kept.filter((one) => one.level < level).map((one) => one.text);
  }
  heading(level: number, text: string): void {
    this.kept = this.kept.filter((one) => one.level < level);
    this.kept.push({ level, text });
  }
}

const same = (one: string[], other: string[]) => one.length === other.length && one.every((heading, at) => heading === other[at]);

/** How alike the neighbours of words at `from`..`to` are to what was kept either side: characters in common, nearest first. */
function likeness(text: string, from: number, to: number, before: string, after: string): number {
  let score = 0;
  for (let back = 1; back <= before.length && from - back >= 0 && text[from - back] === before[before.length - back]; back++) score++;
  for (let on = 0; on < after.length && to + on < text.length && text[to + on] === after[on]; on++) score++;
  return score;
}

/** Every place a quote is in a block. */
function places(text: string, quote: string): number[] {
  const all: number[] = [];
  for (let at = text.indexOf(quote); at >= 0; at = text.indexOf(quote, at + 1)) all.push(at);
  return all;
}

/** Where an anchor's words are in a document's blocks now, or nothing where they are no longer there. */
export function found(anchor: Anchor, blocks: Block[]): Found | null {
  const quote = spaced(anchor.quote).trim();
  if (!quote) return null;
  const before = spaced(anchor.before ?? '');
  const after = spaced(anchor.after ?? '');
  const under = anchor.under ?? [];
  type Place = { block: number; from: number; score: number; under: boolean };
  const all: Place[] = blocks.flatMap((block, index) =>
    places(block.text, quote).map((from) => ({
      block: index,
      from,
      score: likeness(block.text, from, from + quote.length, before, after),
      under: same(block.under, under),
    })),
  );
  const best = (some: Place[]): Place | undefined => some.reduce<Place | undefined>((kept, one) => (kept && kept.score >= one.score ? kept : one), undefined);
  const at = (place: Place | undefined, how: Found['how']): Found | null =>
    place ? { block: place.block, from: place.from, to: place.from + quote.length, how } : null;
  // With the words either side, as they were: under the same headings first.
  const exact = all.filter((one) => {
    const text = blocks[one.block]!.text;
    return text.slice(0, one.from).endsWith(before) && text.slice(one.from + quote.length).startsWith(after);
  });
  if (exact.length) return at(exact.find((one) => one.under) ?? exact[0], 'exact');
  return at(best(all.filter((one) => one.under)), 'heading') ?? at(best(all), 'anywhere');
}

/** The characters of a block that lie between two places of what it was read from. */
export function within(block: Block, from: number, to: number): { from: number; to: number } | null {
  const first = block.at.findIndex((at) => at >= from);
  let last = -1;
  block.at.forEach((at, index) => {
    if (at < to) last = index;
  });
  if (first < 0 || last < first) return null;
  return { from: first, to: last + 1 };
}

/** Where characters of a block are in what it was read from: from the first, to just after the last. */
export function spanOf(block: Block, from: number, to: number): { from: number; to: number } {
  return { from: block.at[from]!, to: block.at[to - 1]! + 1 };
}

/**
 * What a comment on the characters `from`..`to` of a block is about: the
 * words, with space at either end left out, a few words either side and
 * the headings over it, cut to what the daemon keeps. Nothing where the
 * words are nothing, or more than may be quoted.
 */
export function anchorOf(block: Block, from: number, to: number): Anchor | null {
  while (from < to && block.text[from] === ' ') from++;
  while (to > from && block.text[to - 1] === ' ') to--;
  const quote = block.text.slice(from, to);
  if (!quote || quote.length > MOST_QUOTED) return null;
  const anchor: Anchor = { quote };
  const before = block.text.slice(Math.max(0, from - BESIDE), from);
  const after = block.text.slice(to, to + BESIDE);
  if (before) anchor.before = before;
  if (after) anchor.after = after;
  const under = block.under.slice(-MOST_HEADINGS).map((heading) => heading.slice(0, MOST_HEADING));
  if (under.length) anchor.under = under;
  return anchor;
}

/** What a comment on a few words says it was about, in a few words: the quote, cut where it is long. */
export function aboutOf(anchor: Anchor, most = 60): string {
  const quote = spaced(anchor.quote).trim();
  return quote.length > most ? `${quote.slice(0, most - 1).trimEnd()}…` : quote;
}

// ---- a Markdown file, read as it would be shown ------------------------------------------------

const FENCE = /^ {0,3}(`{3,}|~{3,})/;
const ATX = /^ {0,3}(#{1,6})(?:[ \t]+|$)/;
const SETEXT = /^ {0,3}(=+|-+)[ \t]*$/;
const RULE = /^ {0,3}([-*_])(?:[ \t]*\1){2,}[ \t]*$/;
const QUOTED = /^ {0,3}>[ ]?/;
const ITEM = /^ {0,3}(?:[-+*]|\d{1,9}[.)])(?:[ \t]+|$)/;
const PUNCTUATION = /[!-/:-@[-`{-~]/;
const WORDY = /[\p{L}\p{N}]/u;

/** What a piece of a line shows, as it would be shown: what only marks it up left out. `at` is where the piece begins in the file. */
function shows(line: string, at: number, into: Gathering): void {
  let i = 0;
  while (i < line.length) {
    const char = line[i]!;
    const rest = line.slice(i);
    if (char === '\\' && i + 1 < line.length && PUNCTUATION.test(line[i + 1]!)) {
      into.push(line[i + 1]!, at + i + 1);
      i += 2;
      continue;
    }
    if (char === '`') {
      const run = /^`+/.exec(rest)![0];
      const close = line.indexOf(run, i + run.length);
      if (close > 0 && line[close + run.length] !== '`') {
        for (let k = i + run.length; k < close; k++) into.push(line[k]!, at + k);
        i = close + run.length;
        continue;
      }
      for (let k = 0; k < run.length; k++) into.push('`', at + i + k);
      i += run.length;
      continue;
    }
    if (char === '!' && line[i + 1] === '[') {
      const link = /^!\[[^\]]*\]\([^)]*\)/.exec(rest);
      if (link) {
        i += link[0].length;
        continue;
      }
    }
    if (char === '[') {
      const link = /^\[([^\]]*)\](?:\(([^)]*)\)|\[[^\]]*\])/.exec(rest);
      if (link) {
        shows(link[1]!, at + i + 1, into);
        i += link[0].length;
        continue;
      }
    }
    if (char === '<') {
      const auto = /^<((?:https?|mailto):[^>\s]*)>/i.exec(rest);
      if (auto) {
        for (let k = 0; k < auto[1]!.length; k++) into.push(auto[1]![k]!, at + i + 1 + k);
        i += auto[0].length;
        continue;
      }
      const tag = /^<\/?[a-z][^>]*>/i.exec(rest);
      if (tag) {
        i += tag[0].length;
        continue;
      }
    }
    if (char === '*' || (char === '~' && line[i + 1] === '~')) {
      i += /^(\*+|~+)/.exec(rest)![0].length;
      continue;
    }
    if (char === '_') {
      const run = /^_+/.exec(rest)![0];
      const inWord = i > 0 && WORDY.test(line[i - 1]!) && WORDY.test(line[i + run.length] ?? '');
      if (!inWord) {
        i += run.length;
        continue;
      }
    }
    into.push(char, at + i);
    i++;
  }
}

/** A Markdown file as the blocks it shows, each character with its offset in the file. */
export function blocksOfMarkdown(source: string): Block[] {
  const blocks: Block[] = [];
  const headings = new Headings();
  let gathering = new Gathering();
  // The words of the paragraph being read, for a heading underlined in the next line.
  let lines: { text: string; at: number }[] = [];
  let fence: string | null = null;
  const end = () => {
    const block = gathering.done(headings.now());
    if (block) blocks.push(block);
    lines = [];
  };
  const heading = (level: number, said: { text: string; at: number }[]) => {
    const words = new Gathering();
    said.forEach((one) => {
      shows(one.text, one.at, words);
      words.push(' ', one.at + one.text.length);
    });
    const block = words.done(headings.over(level));
    if (block) {
      blocks.push(block);
      headings.heading(level, block.text);
    }
  };
  let offset = 0;
  for (const raw of source.split('\n')) {
    const at = offset;
    offset += raw.length + 1;
    const line = raw.replace(/\r$/, '');
    if (fence !== null) {
      if (line.trim().startsWith(fence) && line.trim().replace(/[`~]/g, '') === '') {
        fence = null;
        end();
      } else {
        for (let k = 0; k < line.length; k++) gathering.push(line[k]!, at + k);
        gathering.push(' ', at + line.length);
      }
      continue;
    }
    const fenced = FENCE.exec(line);
    if (fenced) {
      end();
      fence = fenced[1]!;
      continue;
    }
    if (line.trim() === '') {
      end();
      continue;
    }
    const atx = ATX.exec(line);
    if (atx) {
      end();
      let words = line.slice(atx[0].length);
      words = words.replace(/[ \t]+#+[ \t]*$/, '').replace(/^#+[ \t]*$/, '');
      heading(atx[1]!.length, [{ text: words, at: at + atx[0].length }]);
      continue;
    }
    const underline = SETEXT.exec(line);
    if (underline && lines.length) {
      const said = lines;
      gathering = new Gathering();
      lines = [];
      heading(underline[1]!.startsWith('=') ? 1 : 2, said);
      continue;
    }
    if (RULE.test(line)) {
      end();
      continue;
    }
    // What holds the words (a quote, an item of a list) is no part of them; an item begins a block of its own.
    let rest = line;
    let from = at;
    for (;;) {
      const quoted = QUOTED.exec(rest);
      const item = ITEM.exec(rest);
      const prefix = quoted?.[0] ?? item?.[0];
      if (!prefix) break;
      if (item && !quoted) end();
      rest = rest.slice(prefix.length);
      from += prefix.length;
    }
    if (rest.trim() === '') {
      end();
      continue;
    }
    const indent = /^[ \t]*/.exec(rest)![0].length;
    lines.push({ text: rest.slice(indent), at: from + indent });
    shows(rest.slice(indent), from + indent, gathering);
    gathering.push(' ', from + rest.length);
  }
  end();
  return blocks;
}

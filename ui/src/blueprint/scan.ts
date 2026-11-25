// Reads a blueprint a line at a time and says what each piece of it is.
//
// What the editor colours and what it offers both come from here, so the
// two agree on what a place in the file is. It reads the way the daemon's
// own reader does (`crates/reader/src/syntax.rs`), with one difference:
// it never refuses. Half-written text is the usual case in an editor.
//
// Which words belong to the language is not known here. The daemon says
// (`BlueprintLanguage`), and this walks what it said: from the file, which
// is the first shape, down through each field to what that field holds.

import type { BlueprintLanguage, LanguageShape, ValueKind } from '../api.ts';

export type TokenType =
  | 'comment'
  /** Punctuation: `-`, `:`, `,`, brackets, `|`. */
  | 'mark'
  /** A word of the language: `steps`, `do`, `when`. */
  | 'field'
  /** A name of the writer's choosing that something is known by: a step, a param, a handler. */
  | 'name'
  /** Any other key: an argument, the writer's own data. */
  | 'key'
  | 'action'
  | 'blueprint'
  | 'string'
  | 'number'
  | 'atom'
  | 'text'
  /** The `${` and `}` around an expression. */
  | 'hole'
  /** A name an expression starts with: `params`, `steps`. */
  | 'root'
  | 'function'
  | 'member'
  | 'operator'
  | 'space';

export interface Token {
  from: number;
  to: number;
  type: TokenType;
  /** Inside a `${ }`. */
  held?: true;
}

/** What the entries of a mapping or a list are. */
export type Holds =
  | { is: 'shape' | 'map_of' | 'list_of'; shape: string }
  | { is: 'arguments'; of: string | null }
  | { is: 'list'; of: ValueKind }
  | { is: 'data' };

/** A mapping or list that is open at some line. */
export interface Frame {
  /** The column its entries start at; -1 inside `{ }` or `[ ]`. */
  indent: number;
  holds: Holds;
  /** The key it is the value of. Empty for the file and for an item of a list. */
  key: string;
  /** The line it opened on, counted from 0. */
  line: number;
  flow?: '{' | '[';
  /** In `{ }` and `[ ]`: what has been written so far, as key and value. */
  entries?: [string, string][];
}

export interface State {
  /** The number of the line about to be read, counted from 0. */
  line: number;
  frames: Frame[];
  /** A key or dash with nothing after it, which deeper lines would be the value of. */
  awaiting: { indent: number; kind: ValueKind; key: string; line: number; item: boolean } | null;
  /** Inside a `|` or `>` block: the column its text must be deeper than. */
  block: number | null;
  /** Inside `{ }`: the key whose value comes next. */
  want: { kind: ValueKind; key: string } | null;
}

/** What is being written where the text ends. */
export type Place =
  | { at: 'nowhere' }
  | {
      at: 'key';
      /** The mappings this is inside, the nearest last. */
      frames: Frame[];
      typed: string;
      from: number;
      /** A list is expected here and its dash has not been written. */
      dash: boolean;
      /** What a bare value here would be, where one may stand in for the mapping. */
      orValue: ValueKind | null;
    }
  | {
      at: 'value';
      frames: Frame[];
      key: string;
      kind: ValueKind;
      typed: string;
      from: number;
      /** Right after the colon, with no space yet. */
      tight: boolean;
      quoted: boolean;
    }
  | { at: 'expression'; frames: Frame[]; key: string; typed: string; from: number };

export interface Scanned {
  tokens: Token[];
  state: State;
  place: Place;
}

const NOWHERE: Place = { at: 'nowhere' };
const ANY: ValueKind = { kind: 'any' };
const ATOMS = new Set(['true', 'false', 'null']);
const SYMBOLS = ['&&', '||', '??', '==', '!=', '<=', '>=', '<', '>', '!', '+', '-', '*', '/', '%', '(', ')', '[', ']', ',', '.', '?', ':'];
const MARKS = new Set(['(', ')', '[', ']', ',', '.']);
const DURATION = /^(\d+(\.\d+)?\s*(ms|s|m|h|d))+$/;
const NUMBER = /^-?\d+(\.\d+)?([eE][+-]?\d+)?$/;

const shapes = new WeakMap<BlueprintLanguage, Map<string, LanguageShape>>();

/** A shape by its name. */
export function shapeOf(language: BlueprintLanguage | null, name: string): LanguageShape | undefined {
  if (!language) return undefined;
  let known = shapes.get(language);
  if (!known) {
    known = new Map(language.shapes.map((shape) => [shape.name, shape]));
    shapes.set(language, known);
  }
  return known.get(name);
}

/** Where reading starts: in the file, which is the first shape. */
export function start(language: BlueprintLanguage | null): State {
  const file = language?.shapes[0];
  const holds: Holds = file ? { is: 'shape', shape: file.name } : { is: 'data' };
  return { line: 0, frames: [{ indent: 0, holds, key: '', line: 0 }], awaiting: null, block: null, want: null };
}

/** What a value of this kind holds, when it is a mapping or a list. */
export function holdsOf(kind: ValueKind): Holds {
  switch (kind.kind) {
    case 'shape':
    case 'map_of':
    case 'list_of':
      return { is: kind.kind, shape: kind.of };
    case 'arguments':
      return { is: 'arguments', of: kind.of };
    case 'steps':
    case 'capabilities':
      return { is: 'list', of: kind };
    default:
      return { is: 'data' };
  }
}

/** What the value of `key` is, in a mapping that holds this. */
export function kindUnder(holds: Holds, key: string, language: BlueprintLanguage | null): ValueKind {
  switch (holds.is) {
    case 'shape':
    // One of a shape may stand where a list of them could: `on_fail: { do: ... }`.
    case 'list_of':
      return shapeOf(language, holds.shape)?.fields.find((field) => field.name === key)?.value ?? ANY;
    case 'map_of':
      return { kind: 'shape', of: holds.shape };
    default:
      return ANY;
  }
}

/** What one item of a list that holds this is. */
export function itemKind(holds: Holds): ValueKind {
  if (holds.is === 'list_of') return { kind: 'shape', of: holds.shape };
  if (holds.is === 'list') return holds.of;
  return ANY;
}

/** What a bare value is where this kind is expected: a hook may be just an action's name. */
export function bareKind(kind: ValueKind, language: BlueprintLanguage | null): ValueKind {
  return kind.kind === 'shape' || kind.kind === 'list_of' ? (shapeOf(language, kind.of)?.short ?? ANY) : kind;
}

function keyType(holds: Holds, key: string, language: BlueprintLanguage | null): TokenType {
  switch (holds.is) {
    case 'shape':
    case 'list_of':
      return shapeOf(language, holds.shape)?.fields.some((field) => field.name === key) ? 'field' : 'key';
    case 'map_of':
      return 'name';
    default:
      return 'key';
  }
}

function plainType(text: string, kind: ValueKind): TokenType {
  if (kind.kind === 'action' || kind.kind === 'decision') return 'action';
  if (kind.kind === 'blueprint') return 'blueprint';
  if (ATOMS.has(text) || text === '~') return 'atom';
  if (NUMBER.test(text) || DURATION.test(text)) return 'number';
  return 'text';
}

function leading(text: string): number {
  let at = 0;
  while (text[at] === ' ') at++;
  return at;
}

function skipSpaces(text: string, at: number): number {
  while (text[at] === ' ') at++;
  return at;
}

function trimmed(text: string, from: number, to: number): number {
  while (to > from && text[to - 1] === ' ') to--;
  return to;
}

function isItem(text: string, at: number): boolean {
  return text[at] === '-' && (at + 1 >= text.length || text[at + 1] === ' ');
}

/** Whether a `|` or `>` block starts here, and where its mark ends. */
function blockHeader(text: string, at: number): number {
  const head = /^[|>][+-]?(?=\s*(#.*)?$)/.exec(text.slice(at));
  return head ? at + head[0].length : -1;
}

/** The offset after the quote that closes the one at `at`, or -1. */
function quoteEnd(text: string, at: number): number {
  const quote = text[at];
  for (let i = at + 1; i < text.length; i++) {
    const ch = text[i];
    if (ch === quote) {
      if (quote === "'" && text[i + 1] === "'") i++;
      else return i + 1;
    } else if (ch === '\\' && quote === '"') i++;
  }
  return -1;
}

/** Given where a `${` starts, the offset after its `}`, or -1 when it is never closed. */
export function holeEnd(text: string, dollar: number): number {
  let depth = 0;
  for (let at = dollar + 1; at < text.length; at++) {
    const ch = text[at];
    if (ch === '{') depth++;
    else if (ch === '}') {
      depth--;
      if (depth === 0) return at + 1;
    } else if (ch === '"' || ch === "'") {
      at++;
      while (at < text.length && text[at] !== ch) at += text[at] === '\\' ? 2 : 1;
    }
  }
  return -1;
}

/** Where the unquoted value that starts at `offset` ends. */
function plainEnd(text: string, offset: number, stop: 'line' | 'flow' | 'key'): number {
  let at = offset;
  while (at < text.length) {
    const ch = text[at];
    if (ch === '$' && text[at + 1] === '$') at += 2;
    else if (ch === '$' && text[at + 1] === '{') {
      const end = holeEnd(text, at);
      if (end < 0) return text.length;
      at = end;
    } else if (ch === '#' && at > offset && text[at - 1] === ' ') break;
    else if (stop !== 'line' && (ch === ',' || ch === ']' || ch === '}')) break;
    else if (stop === 'key' && ch === ':') break;
    else at++;
  }
  return at;
}

export interface Key {
  text: string;
  from: number;
  to: number;
  colon: number;
  /** The offset just past the colon. */
  after: number;
}

/** Finds `key:` starting at `offset`, if the line has one there. */
export function keyAt(text: string, offset: number): Key | null {
  const endsKey = (at: number) => text[at] === ':' && (at + 1 >= text.length || text[at + 1] === ' ');
  const first = text[offset];
  if (first === '"' || first === "'") {
    const end = quoteEnd(text, offset);
    if (end < 0) return null;
    const colon = skipSpaces(text, end);
    return endsKey(colon) ? { text: text.slice(offset + 1, end - 1), from: offset, to: end, colon, after: colon + 1 } : null;
  }
  let at = offset;
  while (at < text.length) {
    const ch = text[at];
    if (ch === '$' && text[at + 1] === '$') at += 2;
    else if (ch === '$' && text[at + 1] === '{') {
      const end = holeEnd(text, at);
      if (end < 0) return null;
      at = end;
    } else if (ch === '#' && at > offset && text[at - 1] === ' ') return null;
    else if (endsKey(at)) {
      const to = trimmed(text, offset, at);
      return to === offset ? null : { text: text.slice(offset, to), from: offset, to, colon: at, after: at + 1 };
    } else at++;
  }
  return null;
}

/** How deep a line is indented. */
export function depth(text: string): number {
  return leading(text);
}

/**
 * What is written on a line after a key's colon: the value without its
 * quotes or its comment, or, for `{ }` and `[ ]`, the rest of the line as
 * it stands.
 */
export function writtenAfter(text: string, after: number): string {
  const at = skipSpaces(text, after);
  const first = text[at];
  if (first === '{' || first === '[') return text.slice(at).trimEnd();
  if (first === '"' || first === "'") {
    const end = quoteEnd(text, at);
    return end < 0 ? text.slice(at + 1) : text.slice(at + 1, end - 1);
  }
  return text.slice(at, plainEnd(text, at, 'line')).trimEnd();
}

/** One line being read. */
class Line {
  readonly tokens: Token[] = [];
  place: Place = NOWHERE;
  readonly frames: Frame[];
  awaiting: State['awaiting'];
  block: number | null;
  want: State['want'];
  private readonly text: string;
  private readonly line: number;
  private readonly language: BlueprintLanguage | null;
  private readonly typing: boolean;

  constructor(text: string, was: State, language: BlueprintLanguage | null, typing: boolean) {
    this.text = text;
    this.line = was.line;
    this.language = language;
    this.typing = typing;
    this.frames = was.frames.slice();
    this.awaiting = was.awaiting;
    this.block = was.block;
    this.want = was.want;
  }

  private get top(): Frame {
    return this.frames[this.frames.length - 1]!;
  }

  private add(from: number, to: number, type: TokenType, held = false) {
    if (to > from) this.tokens.push(held ? { from, to, type, held: true } : { from, to, type });
  }

  /** Whether the text ends here, where somebody is writing. */
  private writing(at: number): boolean {
    return this.typing && at >= this.text.length;
  }

  read() {
    const text = this.text;
    const indent = leading(text);
    const blank = indent === text.length;
    if (this.block !== null) {
      if (blank || indent > this.block) {
        // The text of a block; an expression in it is still one.
        this.words(indent, text.length, 'string', '');
        return;
      }
      this.block = null;
    }
    if (this.top.flow) {
      this.flow(indent);
      return;
    }
    // A line with nothing on it changes nothing, unless it is where
    // somebody is about to write.
    if (blank && !this.typing) return;
    if (text[indent] === '#') {
      this.add(indent, text.length, 'comment');
      return;
    }
    this.settle(indent, isItem(text, indent));
    this.entry(indent);
  }

  /** Closes what a line at this depth is outside of, and opens what it begins. */
  private settle(indent: number, item: boolean) {
    const waiting = this.awaiting;
    this.awaiting = null;
    // A list may sit at the same depth as the key that owns it.
    if (waiting && (indent > waiting.indent || (indent === waiting.indent && item && !waiting.item))) {
      this.frames.push({ indent, holds: holdsOf(waiting.kind), key: waiting.key, line: waiting.line });
      return;
    }
    while (this.frames.length > 1) {
      const top = this.top;
      const owner = this.frames[this.frames.length - 2]!;
      // A list at its owner's depth ends at the owner's next key.
      if (top.indent > indent || (top.indent === indent && !item && owner.indent === indent)) this.frames.pop();
      else break;
    }
    // Deeper than anything open: the reader refuses this. Treated as the writer's own.
    if (this.top.indent < indent) this.frames.push({ indent, holds: { is: 'data' }, key: '', line: this.line });
  }

  /** An entry of the nearest mapping or list, starting at this column. */
  private entry(at: number) {
    const text = this.text;
    const top = this.top;
    if (isItem(text, at)) {
      this.item(at);
      return;
    }
    const key = keyAt(text, at);
    if (!key) {
      // Not an entry yet: a key still being written, or text the reader would refuse.
      const stop = plainEnd(text, at, 'line');
      this.add(at, trimmed(text, at, stop), 'text');
      this.add(stop, text.length, 'comment');
      if (this.writing(stop)) {
        const dash = top.holds.is === 'list_of' || top.holds.is === 'list';
        this.place = { at: 'key', frames: this.frames.slice(), typed: text.slice(at), from: at, dash, orValue: null };
      }
      return;
    }
    this.add(key.from, key.to, keyType(top.holds, key.text, this.language));
    this.add(key.colon, key.colon + 1, 'mark');
    const kind = kindUnder(top.holds, key.text, this.language);
    const next = skipSpaces(text, key.after);
    if (next >= text.length || text[next] === '#') {
      this.add(next, text.length, 'comment');
      this.awaiting = { indent: at, kind, key: key.text, line: this.line, item: false };
      if (this.writing(next)) this.offer(key.text, kind, '', text.length, text.length === key.after, false);
      return;
    }
    const header = blockHeader(text, next);
    if (header >= 0) {
      this.add(next, header, 'mark');
      this.add(skipSpaces(text, header), text.length, 'comment');
      this.block = at;
      return;
    }
    this.value(next, kind, key.text);
  }

  /** `- ...`: one item of a list. */
  private item(at: number) {
    const text = this.text;
    const kind = itemKind(this.top.holds);
    this.add(at, at + 1, 'mark');
    const next = skipSpaces(text, at + 1);
    const within = (): Frame => ({ indent: next, holds: kind.kind === 'shape' ? holdsOf(kind) : { is: 'data' }, key: '', line: this.line });
    if (next >= text.length || text[next] === '#') {
      this.add(next, text.length, 'comment');
      this.awaiting = { indent: at, kind, key: '', line: this.line, item: true };
      if (this.writing(next) && next > at + 1) {
        const frames = [...this.frames, within()];
        this.place = { at: 'key', frames, typed: '', from: next, dash: false, orValue: bareKind(kind, this.language) };
      }
      return;
    }
    const header = blockHeader(text, next);
    if (header >= 0) {
      this.add(next, header, 'mark');
      this.block = at;
      return;
    }
    const opens = text[next] === '[' || text[next] === '{';
    if (isItem(text, next) || (!opens && keyAt(text, next))) {
      // The rest of the line is the first line of what the item holds,
      // at the column it really starts at.
      this.frames.push(within());
      this.entry(next);
      return;
    }
    const quote = text[next] === '"' || text[next] === "'";
    const stop = plainEnd(text, next, 'line');
    if (kind.kind === 'shape' && !opens && !quote && this.writing(stop)) {
      // A word with no colon yet: a field of the item, or the bare word
      // that may stand for it.
      this.add(next, text.length, 'text');
      const frames = [...this.frames, within()];
      this.place = { at: 'key', frames, typed: text.slice(next), from: next, dash: false, orValue: bareKind(kind, this.language) };
      return;
    }
    this.value(next, kind, '');
  }

  /** A value that begins on this line at `at`. */
  private value(at: number, kind: ValueKind, key: string) {
    const text = this.text;
    const first = text[at];
    if (first === '[' || first === '{') {
      this.open(at, kind, key);
      this.flow(at + 1);
      return;
    }
    let end: number;
    if (first === '"' || first === "'") end = this.quoted(at, kind, key);
    else {
      end = plainEnd(text, at, 'line');
      this.scalar(at, end, kind, key);
    }
    this.rest(end);
  }

  /** What follows a finished value: a comment, or text the reader would refuse. */
  private rest(from: number) {
    const at = skipSpaces(this.text, from);
    this.add(at, this.text.length, this.text[at] === '#' ? 'comment' : 'text');
  }

  private offer(key: string, kind: ValueKind, typed: string, from: number, tight: boolean, quoted: boolean) {
    this.place = { at: 'value', frames: this.frames.slice(), key, kind: bareKind(kind, this.language), typed, from, tight, quoted };
  }

  /** An unquoted value from `from` to `to`. */
  private scalar(from: number, to: number, kind: ValueKind, key: string) {
    const text = this.text;
    const stop = trimmed(text, from, to);
    const raw = text.slice(from, stop);
    const bare = bareKind(kind, this.language);
    // Where somebody is writing, the spaces at the end are inside whatever is still open.
    if (raw.includes('${')) this.words(from, this.writing(to) ? to : stop, plainType(raw.replace(/\$\{.*$/, ''), bare), key);
    else if (bare.kind === 'expression' && !ATOMS.has(raw)) {
      // A condition may be written without its `${ }`.
      this.expression(from, stop);
      if (this.writing(to)) this.place = { at: 'expression', frames: this.frames.slice(), key, typed: text.slice(from), from };
    } else this.add(from, stop, plainType(raw, bare));
    if (this.writing(to) && this.place.at === 'nowhere') this.offer(key, kind, text.slice(from), from, false, false);
  }

  /** A quoted value starting at `at`; returns where it ends. */
  private quoted(at: number, kind: ValueKind, key: string): number {
    const text = this.text;
    const end = quoteEnd(text, at);
    const stop = end < 0 ? text.length : end;
    this.add(at, at + 1, 'string');
    this.words(at + 1, end < 0 ? stop : stop - 1, 'string', key);
    if (end >= 0) this.add(stop - 1, stop, 'string');
    else if (this.typing && this.place.at === 'nowhere') this.offer(key, kind, text.slice(at + 1), at + 1, false, true);
    return stop;
  }

  /** Text that may have `${ }` in it. */
  private words(from: number, to: number, base: TokenType, key: string) {
    const text = this.text;
    let at = from;
    let plain = from;
    while (at < to) {
      if (text.startsWith('$${', at)) at += 3;
      else if (text.startsWith('${', at)) {
        this.add(plain, at, base);
        const close = holeEnd(text, at);
        const closed = close >= 0 && close <= to;
        this.add(at, at + 2, 'hole', true);
        this.expression(at + 2, closed ? close - 1 : to);
        if (closed) this.add(close - 1, close, 'hole', true);
        else if (this.writing(to)) {
          this.place = { at: 'expression', frames: this.frames.slice(), key, typed: text.slice(at + 2), from: at + 2 };
        }
        at = closed ? close : to;
        plain = at;
      } else at++;
    }
    this.add(plain, to, base);
  }

  /** The inside of a `${ }`. */
  private expression(from: number, to: number) {
    const text = this.text;
    const roots = this.language?.names;
    let at = from;
    let before = '';
    while (at < to) {
      const ch = text[at]!;
      const begin = at;
      if (ch === ' ' || ch === '\t') {
        while (at < to && (text[at] === ' ' || text[at] === '\t')) at++;
        this.add(begin, at, 'space', true);
        continue;
      }
      if (/[A-Za-z_]/.test(ch)) {
        while (at < to && /[A-Za-z0-9_]/.test(text[at]!)) at++;
        const word = text.slice(begin, at);
        const called = text[skipSpaces(text, at)] === '(';
        let type: TokenType = 'text';
        if (before === '.') type = 'member';
        else if (called) type = 'function';
        else if (ATOMS.has(word)) type = 'atom';
        else if (word === 'in') type = 'operator';
        else if (!roots || roots.some((root) => root.name === word)) type = 'root';
        this.add(begin, at, type, true);
        before = word;
        continue;
      }
      if (/[0-9]/.test(ch)) {
        // `a[0].x` is a number and then a member; `1.5` is one number.
        while (at < to && (/[0-9]/.test(text[at]!) || (text[at] === '.' && /[0-9]/.test(text[at + 1] ?? '')))) at++;
        this.add(begin, at, 'number', true);
        before = '0';
        continue;
      }
      if (ch === '"' || ch === "'") {
        at++;
        while (at < to && text[at] !== ch) at += text[at] === '\\' ? 2 : 1;
        at = Math.min(at + 1, to);
        this.add(begin, at, 'string', true);
        before = ch;
        continue;
      }
      const symbol = SYMBOLS.find((one) => text.startsWith(one, at)) ?? ch;
      at = Math.min(at + symbol.length, to);
      this.add(begin, at, MARKS.has(symbol) ? 'mark' : SYMBOLS.includes(symbol) ? 'operator' : 'text', true);
      before = symbol;
    }
  }

  /** Opens the `{` or `[` at `at`. */
  private open(at: number, kind: ValueKind, key: string) {
    const bracket = this.text[at] as '{' | '[';
    let holds: Holds = holdsOf(kind);
    if (bracket === '{') {
      // One of a shape may stand where a list of them could.
      if (holds.is === 'list_of') holds = { is: 'shape', shape: holds.shape };
      else if (holds.is === 'list') holds = { is: 'data' };
    } else if (holds.is !== 'list_of' && holds.is !== 'list') holds = { is: 'list', of: ANY };
    this.add(at, at + 1, 'mark');
    this.frames.push({ indent: -1, holds, key, line: this.line, flow: bracket, entries: [] });
    this.want = null;
  }

  /** Writes down what was said for the entry being read in the nearest `{ }` or `[ ]`. */
  private said(value: string) {
    const top = this.top;
    const entries = (top.entries ?? []).slice();
    if (top.flow === '[') entries.push(['', value]);
    else if (entries.length > 0) entries[entries.length - 1] = [entries[entries.length - 1]![0], value];
    this.frames[this.frames.length - 1] = { ...top, entries };
  }

  /** Reads on inside `{ }` or `[ ]`, which may run across lines. */
  private flow(start: number) {
    const text = this.text;
    let at = start;
    for (;;) {
      at = skipSpaces(text, at);
      const top = this.top;
      if (!top.flow) {
        this.rest(at);
        return;
      }
      const within = top.flow === '{' ? this.want : { kind: itemKind(top.holds), key: top.key };
      if (at >= text.length) {
        // A value that runs to where somebody is writing has said what the place is.
        if (!this.typing || this.place.at !== 'nowhere') return;
        if (within) this.offer(within.key, within.kind, '', at, text[at - 1] === ':', false);
        else this.place = { at: 'key', frames: this.frames.slice(), typed: '', from: at, dash: false, orValue: null };
        return;
      }
      const ch = text[at]!;
      if (ch === '#') {
        this.add(at, text.length, 'comment');
        return;
      }
      if (ch === ',' || ch === '}' || ch === ']') {
        this.add(at, at + 1, 'mark');
        if (ch !== ',') this.frames.pop();
        this.want = null;
        at++;
        continue;
      }
      const quote = ch === '"' || ch === "'";
      if (!within) {
        // A key of this mapping.
        const end = quote ? quoteEnd(text, at) : plainEnd(text, at, 'key');
        if (end < 0) {
          this.add(at, text.length, 'string');
          return;
        }
        const to = trimmed(text, at, end);
        const name = quote ? text.slice(at + 1, end - 1) : text.slice(at, to);
        const colon = skipSpaces(text, end);
        if (text[colon] !== ':' || to === at) {
          // Not a key yet: one being written, or something out of place.
          const step = Math.max(to, at + 1);
          this.add(at, step, 'text');
          if (this.writing(colon)) {
            this.place = { at: 'key', frames: this.frames.slice(), typed: text.slice(at), from: at, dash: false, orValue: null };
          }
          at = step;
          continue;
        }
        this.add(at, to, keyType(top.holds, name, this.language));
        this.add(colon, colon + 1, 'mark');
        this.frames[this.frames.length - 1] = { ...top, entries: [...(top.entries ?? []), [name, '']] };
        this.want = { kind: kindUnder(top.holds, name, this.language), key: name };
        at = colon + 1;
        continue;
      }
      if (ch === '[' || ch === '{') {
        this.open(at, within.kind, within.key);
        at++;
        continue;
      }
      let end: number;
      if (quote) end = this.quoted(at, within.kind, within.key);
      else {
        end = plainEnd(text, at, 'flow');
        this.scalar(at, end, within.kind, within.key);
      }
      this.said(text.slice(at, trimmed(text, at, end)));
      this.want = null;
      at = end;
    }
  }
}

/**
 * Reads one line. `typing` says the text ends where somebody is writing:
 * the place there is worked out, and an empty line is somewhere a key is
 * about to be.
 */
export function scanLine(text: string, state: State, language: BlueprintLanguage | null, typing = false): Scanned {
  const line = new Line(text, state, language, typing);
  line.read();
  return {
    tokens: line.tokens,
    state: { line: state.line + 1, frames: line.frames, awaiting: line.awaiting, block: line.block, want: line.want },
    place: line.place,
  };
}

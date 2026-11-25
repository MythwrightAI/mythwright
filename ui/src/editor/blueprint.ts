// A blueprint in the editor: how it is coloured, what is offered while one
// is written, and what is said about a word that is pointed at.
//
// All three are worked out in `../blueprint/`, which knows nothing about
// the editor. This hands them to it. It is fetched the first time a
// blueprint is opened, like any other language.

import { autocompletion, pickedCompletion, startCompletion } from '@codemirror/autocomplete';
import type { Completion, CompletionContext, CompletionResult } from '@codemirror/autocomplete';
import { HighlightStyle, StreamLanguage, syntaxHighlighting } from '@codemirror/language';
import type { Extension } from '@codemirror/state';
import { hoverTooltip } from '@codemirror/view';
import type { EditorView } from '@codemirror/view';
import { Tag } from '@lezer/highlight';
import type { BlueprintLanguage } from '../api.ts';
import { explain, suggest } from '../blueprint/complete.ts';
import type { Note, Suggestion, World } from '../blueprint/complete.ts';
import { classOf, coloured } from '../blueprint/coloured.ts';
import { scanLine, start } from '../blueprint/scan.ts';
import type { State, Token, TokenType } from '../blueprint/scan.ts';

// One tag for each kind of piece, and one for being inside `${ }`. Each
// becomes a class, so the colours are in the stylesheet with the rest.
const TYPES: TokenType[] = ['comment', 'mark', 'field', 'name', 'key', 'action', 'blueprint', 'string', 'number', 'atom', 'text', 'hole', 'root', 'function', 'member', 'operator', 'space'];
const tags: Record<string, Tag> = Object.fromEntries([...TYPES, 'held'].map((name) => [name, Tag.define()]));
const colours = HighlightStyle.define(Object.entries(tags).map(([name, tag]) => ({ tag, class: `bp-${name}` })));

/** Where reading has got to: the line just read, and which of its pieces comes next. */
interface Reading {
  scan: State;
  tokens: Token[];
  next: number;
}

function grammar(language: BlueprintLanguage | null) {
  return StreamLanguage.define<Reading>({
    name: 'blueprint',
    startState: () => ({ scan: start(language), tokens: [], next: 0 }),
    // What is read is replaced, never changed, so a copy can share it.
    copyState: (state) => ({ ...state }),
    token(stream, state) {
      // A line is read whole when it is begun, and then handed over a piece at a time.
      if (stream.sol()) {
        const read = scanLine(stream.string, state.scan, language);
        state.scan = read.state;
        state.tokens = read.tokens;
        state.next = 0;
      }
      let token = state.tokens[state.next];
      while (token && token.to <= stream.pos) token = state.tokens[++state.next];
      if (!token) {
        stream.skipToEnd();
        return null;
      }
      if (stream.pos < token.from) {
        stream.pos = token.from;
        return null;
      }
      stream.pos = token.to;
      state.next++;
      return token.held ? `${token.type} held` : token.type;
    },
    blankLine(state) {
      state.scan = scanLine('', state.scan, language).state;
    },
    indent(state, _after, context) {
      const { awaiting, frames, block } = state.scan;
      if (block !== null) return null;
      // Under a key with nothing after it, one step in.
      if (awaiting) return awaiting.indent + context.unit;
      // Otherwise where the nearest mapping's entries are, which for an
      // item of a list is past its dash.
      const top = frames[frames.length - 1];
      return top && !top.flow ? top.indent : null;
    },
    languageData: { commentTokens: { line: '#' } },
    tokenTable: tags,
  });
}

/** Text with `code` in backticks, as it should look. */
function prose(text: string, into: HTMLElement) {
  text.split('`').forEach((part, index) => {
    if (part === '') return;
    if (index % 2 === 1) into.append(Object.assign(document.createElement('code'), { textContent: part }));
    else into.append(part);
  });
}

/** A blueprint's text coloured as the editor colours it, read only. */
function example(text: string, language: BlueprintLanguage | null): HTMLElement {
  const pre = Object.assign(document.createElement('pre'), { className: 'bp-note-example' });
  for (const line of coloured(text, language)) {
    for (const piece of line) pre.append(piece.type ? Object.assign(document.createElement('span'), { className: classOf(piece), textContent: piece.text }) : piece.text);
    pre.append('\n');
  }
  return pre;
}

function shown(note: Note, title?: string, more?: () => void, language: BlueprintLanguage | null = null): HTMLElement {
  const box = document.createElement('div');
  box.className = 'bp-note';
  if (title) box.append(Object.assign(document.createElement('div'), { className: 'bp-note-title', textContent: title }));
  const says = document.createElement('p');
  prose(note.says, says);
  box.append(says);
  for (const list of note.lists ?? []) {
    box.append(Object.assign(document.createElement('h4'), { textContent: list.title }));
    const rows = document.createElement('dl');
    for (const row of list.rows) {
      rows.append(Object.assign(document.createElement('dt'), { textContent: row.name }));
      const said = document.createElement('dd');
      if (row.type) said.append(Object.assign(document.createElement('span'), { className: 'bp-note-type', textContent: row.type }));
      prose(row.says, said);
      rows.append(said);
    }
    box.append(rows);
  }
  if (note.where) {
    const where = Object.assign(document.createElement('p'), { className: 'bp-note-where' });
    prose(note.where, where);
    box.append(where);
  }
  if (note.example) {
    box.append(Object.assign(document.createElement('h4'), { textContent: 'Example' }));
    box.append(example(note.example, language));
  }
  // One quiet way on to where everything of it is said, for a word that has an entry there.
  if (more) {
    const link = Object.assign(document.createElement('button'), { className: 'bp-note-ref', type: 'button', textContent: 'in the Reference' });
    link.addEventListener('click', more);
    box.append(link);
  }
  return box;
}

/** Puts a suggestion in, leaves the caret where its value goes, and offers what comes next. */
function take(one: Suggestion) {
  return (view: EditorView, completion: Completion, from: number, to: number) => {
    const insert = one.insert ?? one.label;
    view.dispatch({
      changes: { from, to, insert },
      selection: { anchor: from + (one.caret ?? insert.length) },
      annotations: pickedCompletion.of(completion),
      userEvent: 'input.complete',
    });
    if (one.again) startCompletion(view);
  };
}

function offering(language: BlueprintLanguage | null, world: () => World) {
  return (context: CompletionContext): CompletionResult | null => {
    const found = suggest(context.state.doc.toString(), context.pos, language, world());
    if (!found) return null;
    // An empty line is offered its fields when they are asked for, not
    // each time it is indented.
    if (!context.explicit && !found.ready) return null;
    return {
      from: found.from,
      validFor: found.still,
      options: found.options.map((one, index) => ({
        label: one.label,
        detail: one.detail,
        type: one.kind,
        // In the order they were given: what is likeliest is first.
        boost: Math.max(99 - index, -99),
        info: one.note ? () => shown(one.note!) : undefined,
        apply: take(one),
      })),
    };
  };
}

/** Everything the editor needs to show a blueprint as one. */
export function blueprint(language: BlueprintLanguage | null, world: () => World, reference?: (entry: string) => void): Extension {
  return [
    grammar(language),
    syntaxHighlighting(colours),
    autocompletion({
      override: [offering(language, world)],
      icons: false,
      optionClass: (completion) => `bp-option bp-option-${completion.type ?? 'word'}`,
      maxRenderedOptions: 200,
    }),
    hoverTooltip(
      (view, at) => {
        const found = explain(view.state.doc.toString(), at, language, world());
        if (!found) return null;
        const entry = found.entry;
        const more = entry && reference ? () => reference(entry) : undefined;
        return { pos: found.from, end: found.to, above: true, create: () => ({ dom: shown(found.note, found.title, more, language) }) };
      },
      { hoverTime: 250 },
    ),
  ];
}

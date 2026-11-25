// The text editor. This file and everything it imports is its own chunk,
// fetched the first time a document is opened.

import { defaultKeymap, history, historyKeymap, indentWithTab } from '@codemirror/commands';
import { markdown } from '@codemirror/lang-markdown';
import { yaml } from '@codemirror/lang-yaml';
import { bracketMatching, HighlightStyle, indentOnInput, indentUnit, StreamLanguage, syntaxHighlighting } from '@codemirror/language';
import { lintGutter, setDiagnostics } from '@codemirror/lint';
import type { Diagnostic as Lint } from '@codemirror/lint';
import { Compartment, EditorState, StateEffect, StateField } from '@codemirror/state';
import type { Extension, Text } from '@codemirror/state';
import { Decoration, drawSelection, EditorView, gutters, highlightActiveLine, highlightActiveLineGutter, keymap, lineNumbers } from '@codemirror/view';
import type { DecorationSet } from '@codemirror/view';
import { tags } from '@lezer/highlight';
import { marksOf } from '../anchored.ts';
import type { OnWords } from '../anchored.ts';
import { blocksOfMarkdown, found, spanOf } from '../anchors.ts';
import type { Anchor } from '../anchors.ts';
import type { BlueprintLanguage, Diagnostic } from '../api.ts';
import type { World } from '../blueprint/complete.ts';
import { readAhead } from './ahead.ts';
import { changes } from './changes.ts';
import { finding } from './find.tsx';
import type { Keeping } from './find.tsx';
import type { Asked } from './finding.ts';
import type { Language } from './language.ts';

// Colours come from the page's variables, so the editor follows the theme.
const highlight = HighlightStyle.define([
  { tag: [tags.propertyName, tags.definition(tags.propertyName)], color: 'var(--info)' },
  { tag: [tags.string, tags.special(tags.string)], color: 'var(--ok)' },
  { tag: [tags.number, tags.bool, tags.null, tags.atom], color: 'var(--accent)' },
  { tag: [tags.comment, tags.meta], color: 'var(--faint)', fontStyle: 'italic' },
  { tag: [tags.keyword, tags.operator], color: 'var(--agent)' },
  { tag: tags.heading, color: 'var(--text)', fontWeight: '700' },
  { tag: [tags.emphasis], fontStyle: 'italic' },
  { tag: [tags.strong], fontWeight: '700' },
  { tag: [tags.link, tags.url], color: 'var(--info)' },
  { tag: tags.monospace, color: 'var(--accent)' },
]);

export interface EditorOptions {
  parent: HTMLElement;
  doc: string;
  language: Language;
  readOnly: boolean;
  onChange: (text: string) => void;
  onSave: () => void;
  /** Asked to lay the text out, where there is a way of laying it out. */
  onFormat: () => void;
  /** For a blueprint: the language as the daemon describes it, and what else there is to offer. */
  /** `reference`: opens the Reference at an entry, from the card about a word that has one. */
  blueprint?: { language: () => Promise<BlueprintLanguage | null>; world: () => World; reference?: (entry: string) => void };
  /** Where what is searched for in the text is kept while the editor is away. */
  sought: Keeping;
  /** A mark over words commented on pressed: its comments, what they are about and where it is on the screen. */
  onMark?: (seqs: number[], anchor: Anchor, box: DOMRect) => void;
}

export interface Editor {
  text(): string;
  /** Makes the document this text, e.g. when the file changed on disk or was laid out again. */
  replace(text: string): void;
  setReadOnly(readOnly: boolean): void;
  setProblems(problems: Diagnostic[]): void;
  goToLine(line: number): void;
  /** Marks the words each of these comments is about, where they are found in the text read as a document. */
  mark(comments: OnWords[]): void;
  /** Draws the mark of these comments as the one whose card is open. */
  opened(seqs: number[]): void;
  /** Does what a key pressed outside the text asks of the bar that finds in it. */
  find(asked: Asked): void;
  /** Whether that bar is open. */
  finds(): boolean;
  focus(): void;
  /** Where its text is scrolled to, said as the editor itself puts it back (`scrollTo`). */
  scrolled(): unknown;
  /** Scrolls it back to what `scrolled` said, with the text as it is now. */
  scrollTo(scrolled: unknown): void;
  destroy(): void;
}

/**
 * For reading only: nothing changes the text, and the page does not offer
 * to type in it. It can still be selected and copied.
 */
function reading(only: boolean): Extension {
  return [EditorState.readOnly.of(only), EditorView.editable.of(!only)];
}

/** The comments on a few words of a document, and the marks over those words as the text now has them. */
interface Marked {
  comments: OnWords[];
  /** The comments whose card is open: their mark is drawn as the open one. */
  open: number[];
  set: DecorationSet;
  stretches: { from: number; to: number; seqs: number[]; anchor: Anchor }[];
}

const commentsAre = StateEffect.define<OnWords[]>();
const openAre = StateEffect.define<number[]>();

/** Where each comment's words are in the text, read as the document it is, and the marks over them. */
function marked(doc: Text, comments: OnWords[], open: number[]): Marked {
  if (comments.length === 0) return { comments, open, set: Decoration.none, stretches: [] };
  const blocks = blocksOfMarkdown(doc.toString());
  const placed = comments.flatMap(({ seq, anchor }) => {
    const at = found(anchor, blocks);
    return at ? [{ seq, anchor, ...spanOf(blocks[at.block]!, at.from, at.to) }] : [];
  });
  const stretches = marksOf(placed).map((stretch) => ({ ...stretch, anchor: placed.find((one) => one.seq === stretch.seqs[0])!.anchor }));
  const set = Decoration.set(
    stretches.map((one) =>
      Decoration.mark({
        class: one.seqs.some((seq) => open.includes(seq)) ? 'cm-commented cm-open' : 'cm-commented',
        attributes: { 'data-comments': one.seqs.join(' ') },
      }).range(one.from, one.to),
    ),
    true,
  );
  return { comments, open, set, stretches };
}

const commentsMarked = StateField.define<Marked>({
  create: () => ({ comments: [], open: [], set: Decoration.none, stretches: [] }),
  update(was, tr) {
    const told = tr.effects.find((effect) => effect.is(commentsAre));
    const open = tr.effects.find((effect) => effect.is(openAre));
    if (told || open) return marked(tr.state.doc, (told?.value as OnWords[] | undefined) ?? was.comments, (open?.value as number[] | undefined) ?? was.open);
    return tr.docChanged ? marked(tr.state.doc, was.comments, was.open) : was;
  },
  provide: (field) => EditorView.decorations.from(field, (kept) => kept.set),
});

/** A press on a mark opens its comments; the cursor is put there all the same. */
function pressedMarks(open: EditorOptions['onMark']): Extension {
  return EditorView.domEventHandlers({
    click(event, view) {
      const mark = (event.target as Element | null)?.closest?.('.cm-commented');
      if (!mark || !open) return false;
      const at = view.posAtDOM(mark);
      const here = view.state.field(commentsMarked).stretches.filter((one) => one.from <= at && at <= one.to);
      const one = here.at(-1);
      if (one) open(here.flatMap((each) => each.seqs), one.anchor, mark.getBoundingClientRect());
      return false;
    },
  });
}

export function createEditor(options: EditorOptions): Editor {
  const readOnly = new Compartment();
  // Markdown and YAML are here from the start. Any other language is
  // fetched now, and the text is coloured when it arrives.
  const colouring = new Compartment();
  const language: Extension = options.language === 'markdown' ? markdown() : options.language === 'yaml' ? yaml() : [];
  let quiet = false;
  const find = finding(options.sought);
  const view = new EditorView({
    parent: options.parent,
    state: EditorState.create({
      doc: options.doc,
      extensions: [
        // The numbers scroll with the text and are drawn with it. Left to
        // stay put for a long line scrolled sideways (`position: sticky`),
        // they were a layer of their own as tall as the whole text, which
        // the browser drew after the text when it was scrolled quickly, and
        // the numbers came a moment late. Lines are wrapped here, so nothing
        // scrolls sideways for them to stay put for.
        gutters({ fixed: false }),
        lineNumbers(),
        readAhead,
        highlightActiveLine(),
        highlightActiveLineGutter(),
        drawSelection(),
        history(),
        indentOnInput(),
        bracketMatching(),
        indentUnit.of('  '),
        EditorState.tabSize.of(2),
        syntaxHighlighting(highlight),
        lintGutter(),
        colouring.of(language),
        readOnly.of(reading(options.readOnly)),
        find.extension,
        commentsMarked,
        pressedMarks(options.onMark),
        keymap.of([
          {
            key: 'Mod-s',
            // In the bar that finds as in the text.
            scope: 'editor search-panel',
            preventDefault: true,
            run: () => {
              options.onSave();
              return true;
            },
          },
          {
            key: 'Shift-Alt-f',
            preventDefault: true,
            run: () => {
              options.onFormat();
              return true;
            },
          },
          indentWithTab,
          ...defaultKeymap,
          ...historyKeymap,
        ]),
        EditorView.updateListener.of((update) => {
          if (update.docChanged && !quiet) options.onChange(update.state.doc.toString());
        }),
        EditorView.lineWrapping,
      ],
    }),
  });

  const offset = (line: number, col: number): number => {
    const doc = view.state.doc;
    const at = doc.line(Math.min(Math.max(line, 1), doc.lines));
    return Math.min(at.from + Math.max(col - 1, 0), at.to);
  };

  let gone = false;
  void colours(options.language, options.blueprint).then((fetched) => {
    if (fetched && !gone) view.dispatch({ effects: colouring.reconfigure(fetched) });
  });

  return {
    text: () => view.state.doc.toString(),
    replace(text) {
      const now = view.state.doc.toString();
      if (text === now) return;
      // Only what differs is changed, as one step: a cursor stays where it
      // was in the text, and undoing takes the whole of it back.
      quiet = true;
      view.dispatch({ changes: changes(now, text) });
      quiet = false;
    },
    setReadOnly(value) {
      view.dispatch({ effects: readOnly.reconfigure(reading(value)) });
    },
    setProblems(problems) {
      const marks: Lint[] = problems.map((problem) => {
        const from = offset(problem.span.start.line, problem.span.start.col);
        const to = Math.max(from, offset(problem.span.end.line, problem.span.end.col));
        // A problem with no width still needs something to underline.
        return { from, to: to === from ? Math.min(from + 1, view.state.doc.length) : to, severity: problem.severity, message: problem.message };
      });
      view.dispatch(setDiagnostics(view.state, marks));
    },
    goToLine(line) {
      const at = offset(line, 1);
      view.dispatch({ selection: { anchor: at }, effects: EditorView.scrollIntoView(at, { y: 'center' }) });
      view.focus();
    },
    mark: (comments) => view.dispatch({ effects: commentsAre.of(comments) }),
    opened: (seqs) => view.dispatch({ effects: openAre.of(seqs) }),
    find: (asked) => find.ask(view, asked),
    finds: () => find.shown(view),
    focus: () => view.focus(),
    scrolled: () => view.scrollSnapshot(),
    scrollTo: (scrolled) => view.dispatch({ effects: scrolled as ReturnType<EditorView['scrollSnapshot']> }),
    destroy: () => {
      gone = true;
      view.destroy();
    },
  };
}

/**
 * How a language is coloured, and for a blueprint what is offered while
 * writing one, fetched the first time a file in it is opened. `null` for
 * the ones that are here already, and for plain text.
 */
async function colours(language: Language, about: EditorOptions['blueprint']): Promise<Extension | null> {
  // The older way of describing a language: enough to colour it.
  const old = async <M>(fetching: Promise<M>, pick: (found: M) => Parameters<typeof StreamLanguage.define>[0]) =>
    StreamLanguage.define(pick(await fetching));
  switch (language) {
    case 'blueprint': {
      // The language is the daemon's to describe. Without it a blueprint is
      // still coloured, only with less known about it, and nothing is offered.
      const [{ blueprint }, described] = await Promise.all([import('./blueprint.ts'), about?.language() ?? null]);
      return blueprint(described, about?.world ?? (() => ({ blueprints: [], agents: [], devices: [] })), about?.reference);
    }
    case 'javascript':
    case 'typescript':
    case 'jsx':
    case 'tsx': {
      const { javascript } = await import('@codemirror/lang-javascript');
      return javascript({ typescript: language === 'typescript' || language === 'tsx', jsx: language === 'jsx' || language === 'tsx' });
    }
    case 'json':
      return (await import('@codemirror/lang-json')).json();
    case 'rust':
      return (await import('@codemirror/lang-rust')).rust();
    case 'python':
      return (await import('@codemirror/lang-python')).python();
    case 'html':
      return (await import('@codemirror/lang-html')).html();
    case 'css':
      return (await import('@codemirror/lang-css')).css();
    case 'toml':
      return old(import('@codemirror/legacy-modes/mode/toml'), (found) => found.toml);
    case 'shell':
      return old(import('@codemirror/legacy-modes/mode/shell'), (found) => found.shell);
    case 'go':
      return old(import('@codemirror/legacy-modes/mode/go'), (found) => found.go);
    case 'c':
      return old(import('@codemirror/legacy-modes/mode/clike'), (found) => found.c);
    case 'cpp':
      return old(import('@codemirror/legacy-modes/mode/clike'), (found) => found.cpp);
    case 'java':
      return old(import('@codemirror/legacy-modes/mode/clike'), (found) => found.java);
    case 'ruby':
      return old(import('@codemirror/legacy-modes/mode/ruby'), (found) => found.ruby);
    case 'sql':
      return old(import('@codemirror/legacy-modes/mode/sql'), (found) => found.standardSQL);
    case 'dockerfile':
      return old(import('@codemirror/legacy-modes/mode/dockerfile'), (found) => found.dockerFile);
    default:
      return null;
  }
}

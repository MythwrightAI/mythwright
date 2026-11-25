// The block editor, fetched when a document is first opened for writing.
//
// A document is a Markdown file and stays one. This shows it as blocks:
// what is typed is turned into a heading, a list, a quote as it is typed
// (`# `, `- `, `> `), and `/` at the start of a line offers the blocks by
// name. What comes out when it is saved is Markdown again, written the way
// files here are written, so that an edit is a small change to the file.

import { baseKeymap, chainCommands, exitCode, setBlockType, toggleMark, wrapIn } from 'prosemirror-commands';
import { dropCursor } from 'prosemirror-dropcursor';
import { gapCursor } from 'prosemirror-gapcursor';
import { history, redo, undo } from 'prosemirror-history';
import { InputRule, inputRules, textblockTypeInputRule, wrappingInputRule } from 'prosemirror-inputrules';
import { keymap } from 'prosemirror-keymap';
import type { MarkType, Node as Doc } from 'prosemirror-model';
import { liftListItem, sinkListItem, splitListItem, wrapInList } from 'prosemirror-schema-list';
import { EditorState, Plugin, PluginKey, Selection, TextSelection } from 'prosemirror-state';
import type { Command, Transaction } from 'prosemirror-state';
import { Decoration, DecorationSet, EditorView } from 'prosemirror-view';
import { STARTERS } from '../boxes.ts';
import { isBox, matching, splitHead } from './blocks.ts';
import { pageLinks } from './linked.ts';
import type { Linking } from './linked.ts';
import { parsed, schema, written } from './markdown.ts';
import { footnoteRules, numbered, sourceCards } from './noted.ts';
import type { Block } from './blocks.ts';
import { blockHandle } from './handle.ts';
import type { Handing } from './handle.ts';
import { choosePicture, pictureView, picturesTaken } from './pictured.ts';
import type { Picturing } from './pictured.ts';
import { commented, markComments, showOpen } from './selected.ts';
import type { Commenting } from './selected.ts';
import type { OnWords } from '../anchored.ts';

const { nodes, marks } = schema;

/** Text between two markers becomes marked text: `**this**` becomes bold. */
function markRule(pattern: RegExp, mark: MarkType): InputRule {
  return new InputRule(pattern, (state, match, start, end) => {
    const inner = match[2];
    if (!inner) return null;
    const from = start + (match[1]?.length ?? 0);
    return state.tr.replaceWith(from, end, schema.text(inner, [mark.create()])).removeStoredMark(mark);
  });
}

const typed = inputRules({
  rules: [
    textblockTypeInputRule(/^(#{1,3})\s$/, nodes.heading, (match) => ({ level: match[1]!.length })),
    // A list that is typed is a tight one, its items on consecutive lines,
    // which is how a person writes a list in a file.
    wrappingInputRule(/^\s*[-+*]\s$/, nodes.bullet_list, () => ({ tight: true })),
    wrappingInputRule(/^(\d+)\.\s$/, nodes.ordered_list, (match) => ({ order: Number(match[1]), tight: true })),
    wrappingInputRule(/^\s*>\s$/, nodes.blockquote),
    textblockTypeInputRule(/^```$/, nodes.code_block),
    new InputRule(/^(?:---|___)$/, (state, _match, start, end) => {
      const rule = nodes.horizontal_rule.create();
      return state.tr.replaceWith(start - 1, end, [rule, nodes.paragraph.create()]);
    }),
    markRule(/(^|[^*])\*\*([^*]+)\*\*$/, marks.strong),
    markRule(/(^|[^*_])[*_]([^*_]+)[*_]$/, marks.em),
    markRule(/(^|[^`])`([^`]+)`$/, marks.code),
    ...footnoteRules,
  ],
});

/** What choosing a block from the menu does to the line it was asked on. */
function turnInto(block: Block['id']): Command {
  switch (block) {
    case 'h1':
      return setBlockType(nodes.heading, { level: 1 });
    case 'h2':
      return setBlockType(nodes.heading, { level: 2 });
    case 'h3':
      return setBlockType(nodes.heading, { level: 3 });
    case 'bullets':
      return wrapInList(nodes.bullet_list, { tight: true });
    case 'numbers':
      return wrapInList(nodes.ordered_list, { tight: true });
    case 'quote':
      return wrapIn(nodes.blockquote);
    case 'code':
      return setBlockType(nodes.code_block);
    case 'infobox':
    case 'series':
    case 'note':
      // A block of code in the box's language, with what it is to say begun, the cursor at its end.
      return (state, dispatch) => {
        const { $from } = state.selection;
        const starter = STARTERS[block];
        const tr = state.tr.replaceWith($from.before(), $from.after(), nodes.code_block.create({ params: block }, schema.text(starter)));
        dispatch?.(tr.setSelection(Selection.near(tr.doc.resolve($from.before() + 1 + starter.length))).scrollIntoView());
        return true;
      };
    case 'divider':
      return (state, dispatch) => {
        const { $from } = state.selection;
        const at = $from.before();
        dispatch?.(state.tr.replaceWith(at, $from.after(), [nodes.horizontal_rule.create(), nodes.paragraph.create()]));
        return true;
      };
    default:
      return setBlockType(nodes.paragraph);
  }
}

// ---- the `/` menu ------------------------------------------------------------------

interface Asking {
  /** Where the `/` is. */
  from: number;
  typed: string;
}

/** Whether the cursor is at the end of a line that is only `/` and a few letters. */
function asking(state: EditorState): Asking | null {
  const { $from, empty } = state.selection;
  if (!empty || $from.parent.type !== nodes.paragraph) return null;
  const before = $from.parent.textBetween(0, $from.parentOffset, undefined, '￼');
  const found = /^\/([a-z0-9 ]{0,20})$/i.exec(before);
  return found ? { from: $from.start(), typed: found[1] ?? '' } : null;
}

const menuKey = new PluginKey('blocks');

/** The `/` menu; it offers a picture or a page only where the document takes them (`picturing`), and an article's boxes only where it links to pages (`boxes`). */
function blockMenu(host: HTMLElement, picturing: Picturing | undefined, boxes: boolean): Plugin {
  const box = document.createElement('div');
  box.className = 'block-menu';
  box.hidden = true;
  host.append(box);
  let chosen = 0;
  let shown: Block[] = [];
  let putAway = -1;

  const pick = (view: EditorView, block: Block) => {
    const now = asking(view.state);
    if (!now) return;
    // The `/…` goes, and the line becomes what was asked for.
    view.dispatch(view.state.tr.delete(now.from, view.state.selection.from));
    if (block.id === 'picture') return picturing && choosePicture(view, picturing);
    turnInto(block.id)(view.state, view.dispatch);
    view.focus();
  };

  const draw = (view: EditorView) => {
    const now = asking(view.state);
    shown = now && now.from !== putAway ? matching(now.typed).filter((block) => (picturing || block.id !== 'picture') && (boxes || !isBox(block.id))) : [];
    if (!now || now.from !== putAway) putAway = now ? putAway : -1;
    box.hidden = shown.length === 0;
    if (box.hidden) return;
    chosen = Math.min(chosen, shown.length - 1);
    box.replaceChildren(
      ...shown.map((block, index) => {
        const item = document.createElement('button');
        item.type = 'button';
        item.className = index === chosen ? 'chosen' : '';
        item.innerHTML = '<strong></strong><span></span>';
        item.firstElementChild!.textContent = block.label;
        item.lastElementChild!.textContent = block.hint;
        // Before the editor loses the cursor to the click.
        item.addEventListener('mousedown', (event) => {
          event.preventDefault();
          pick(view, block);
        });
        return item;
      }),
    );
    const at = view.coordsAtPos(now!.from);
    const within = host.getBoundingClientRect();
    box.style.left = `${at.left - within.left}px`;
    box.style.top = `${at.bottom - within.top + host.scrollTop + 6}px`;
  };

  return new Plugin({
    key: menuKey,
    view: (view) => {
      draw(view);
      return { update: draw, destroy: () => box.remove() };
    },
    props: {
      handleKeyDown(view, event) {
        if (shown.length === 0) return false;
        if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
          chosen = (chosen + (event.key === 'ArrowDown' ? 1 : shown.length - 1)) % shown.length;
          draw(view);
          return true;
        }
        if (event.key === 'Enter' || event.key === 'Tab') {
          pick(view, shown[chosen]!);
          return true;
        }
        if (event.key === 'Escape') {
          putAway = asking(view.state)?.from ?? -1;
          draw(view);
          return true;
        }
        return false;
      },
    },
  });
}

/** A hint on the empty line the cursor is on, so that the menu is found. */
const hint = new Plugin({
  props: {
    decorations(state) {
      const { $from, empty } = state.selection;
      const line = $from.parent;
      if (!empty || line.type !== nodes.paragraph || line.content.size > 0 || $from.depth !== 1) return null;
      const words = state.doc.childCount === 1 ? 'Write, or type / for a heading, a list, a quote' : 'Type / for blocks';
      return DecorationSet.create(state.doc, [
        Decoration.node($from.before(), $from.after(), { class: 'empty-line', 'data-hint': words }),
      ]);
    },
  },
});

/** At the very start of a heading, a quote or code, backspace makes it plain text again. */
const backToText: Command = (state, dispatch) => {
  const { $from, empty } = state.selection;
  if (!empty || $from.parentOffset > 0 || $from.parent.type === nodes.paragraph) return false;
  if (!$from.parent.isTextblock) return false;
  return setBlockType(nodes.paragraph)(state, dispatch);
};

export interface Writer {
  destroy(): void;
  focus(): void;
  /** Marks the words each of these comments is about, where they are found. */
  mark(comments: OnWords[]): void;
  /** Draws the mark of these comments as the one whose card is open. */
  opened(seqs: number[]): void;
  /** The words of its headings that a page reads as headings (levels 1 to 3), in the order they stand. */
  headings(): string[];
  /** Puts the cursor at the start of the section under the `nth` of those, with its heading at the top of what shows. */
  toSection(nth: number): void;
  /** Which of those the cursor is under: none (-1) before the first. */
  sectionOf(): number;
}

/** The headings a page reads as headings (`prose.ts` reads `#` to `###`), where each is in the document, in order. */
function headingsIn(doc: Doc): { pos: number; node: Doc }[] {
  const found: { pos: number; node: Doc }[] = [];
  doc.descendants((node, pos) => {
    if (node.type !== nodes.heading) return true;
    if ((node.attrs.level as number) <= 3) found.push({ pos, node });
    return false;
  });
  return found;
}

export function createWriter(options: {
  parent: HTMLElement;
  markdown: string;
  onChange: (markdown: string) => void;
  onSave: () => void;
  /** What the page does for the handle beside each block: its menu, and a comment. */
  handing: Handing;
  /** What the page does with comments on words chosen and on words marked. */
  commenting: Commenting;
  /** Where the document takes pictures: what they are attached to, and how. */
  picturing?: Picturing;
  /** Where the document links to other pages as `[[` is typed (a topic's): the page it is, which is not offered. */
  linking?: Linking;
}): Writer {
  const { head, body } = splitHead(options.markdown);
  const view = new EditorView(options.parent, {
    state: EditorState.create({
      doc: parsed(body),
      plugins: [
        // First, so that Ctrl and / is its own and anything typed puts the handle away.
        blockHandle(options.parent, options.handing),
        blockMenu(options.parent, options.picturing, Boolean(options.linking)),
        ...(options.picturing ? [picturesTaken(options.picturing)] : []),
        ...(options.linking ? [pageLinks(options.parent, options.linking)] : []),
        numbered,
        sourceCards,
        commented(options.parent, options.commenting),
        typed,
        keymap({
          'Mod-z': undo,
          'Shift-Mod-z': redo,
          'Mod-y': redo,
          'Mod-b': toggleMark(marks.strong),
          'Mod-i': toggleMark(marks.em),
          'Mod-e': toggleMark(marks.code),
          'Mod-s': () => {
            options.onSave();
            return true;
          },
          Enter: splitListItem(nodes.list_item),
          Tab: sinkListItem(nodes.list_item),
          'Shift-Tab': liftListItem(nodes.list_item),
          'Shift-Enter': chainCommands(exitCode, (state, dispatch) => {
            dispatch?.(state.tr.replaceSelectionWith(nodes.hard_break.create()).scrollIntoView());
            return true;
          }),
          Backspace: backToText,
        }),
        keymap(baseKeymap),
        history(),
        dropCursor(),
        gapCursor(),
        hint,
      ],
    }),
    // A picture is drawn as the page draws it, and nothing at an address somewhere else is fetched.
    nodeViews: { image: pictureView(options.picturing?.record ?? null) },
    dispatchTransaction(transaction: Transaction) {
      view.updateState(view.state.apply(transaction));
      // Only a change to the document is a change to the file: opening one
      // and moving about in it must not rewrite it.
      if (transaction.docChanged) options.onChange(head + written(view.state.doc));
    },
  });
  // A click in the margin round the document, or below its last line, is a
  // click to carry on writing at the end.
  const toEnd = (event: MouseEvent) => {
    if (event.target !== options.parent) return;
    event.preventDefault();
    view.dispatch(view.state.tr.setSelection(Selection.atEnd(view.state.doc)));
    view.focus();
  };
  options.parent.addEventListener('mousedown', toEnd);
  return {
    destroy: () => {
      options.parent.removeEventListener('mousedown', toEnd);
      view.destroy();
    },
    focus: () => view.focus(),
    mark: (comments) => markComments(view, comments),
    opened: (seqs) => showOpen(view, seqs),
    headings: () => headingsIn(view.state.doc).map(({ node }) => node.textContent.trim()),
    toSection: (nth) => {
      const heading = headingsIn(view.state.doc)[nth];
      if (!heading) return view.focus();
      const { doc } = view.state;
      const after = heading.pos + heading.node.nodeSize;
      // Where the section says something before its next heading, at the start of what it says; else at the end of its heading.
      const next = doc.resolve(after).nodeAfter;
      const inside = next && next.type !== nodes.heading ? Selection.findFrom(doc.resolve(after), 1, true) : null;
      const cursor = inside ?? TextSelection.create(doc, after - 1);
      view.dispatch(view.state.tr.setSelection(cursor));
      view.focus();
      (view.nodeDOM(heading.pos) as HTMLElement | null)?.scrollIntoView({ block: 'start' });
    },
    sectionOf: () => {
      const at = view.state.selection.from;
      return headingsIn(view.state.doc).filter((heading) => heading.pos < at).length - 1;
    },
  };
}

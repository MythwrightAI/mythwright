// Comments on a selection of the document, in the block editor.
//
// Words chosen within one block have a small bar over them with Comment
// on it: pressed, a comment is begun on those words,
// with what is either side of them and the headings over them
// (`anchors.ts`). The bar is drawn only once the pointer has let go, so
// that nothing comes under the pointer as a selection is dragged out, and
// it lies over the page in the editor's own scrolling box like the
// handle: nothing of the text moves as it comes and goes.
//
// The words each comment on a few words is about are marked where they
// are found (a `DecorationSet`, worked out again whenever the document or
// the comments change, by looking for the words: nothing is kept in the
// file). A mark is only coloured: pointed at, it is coloured more and
// nothing moves. Pressed, its comments are opened beside it.

import type { Node } from 'prosemirror-model';
import { Plugin, PluginKey, TextSelection } from 'prosemirror-state';
import { Decoration, DecorationSet } from 'prosemirror-view';
import type { EditorView } from 'prosemirror-view';
import { marksOf } from '../anchored.ts';
import type { OnWords } from '../anchored.ts';
import { anchorOf, found, Gathering, Headings, MOST_QUOTED, spanOf, within } from '../anchors.ts';
import type { Anchor, Block } from '../anchors.ts';

/** Where something is on the screen, as the page round the editor is told it. */
export interface Box {
  top: number;
  bottom: number;
  left: number;
  right: number;
}

/** What the page round the editor does with comments on its words. */
export interface Commenting {
  /** Whether the document takes comments now. */
  takes(): boolean;
  /** A comment is begun on these words, which are where `at` says, asked when it is wanted: the page may first make room. */
  begin(anchor: Anchor, at: () => Box): void;
  /** The comments of a mark are opened: the mark is where `at` says. */
  open(seqs: number[], anchor: Anchor, at: () => Box): void;
}

/** The blocks of the document as plain text, each character with its place in the document. */
export function blocksOfDoc(doc: Node): Block[] {
  const blocks: Block[] = [];
  const headings = new Headings();
  doc.descendants((node, at) => {
    if (!node.isTextblock) return true;
    const gathering = new Gathering();
    node.forEach((child, offset) => {
      const pos = at + 1 + offset;
      if (child.isText) for (let k = 0; k < child.text!.length; k++) gathering.push(child.text![k]!, pos + k);
      else if (child.type.name === 'hard_break') gathering.push(' ', pos);
    });
    const heading = node.type.name === 'heading' ? Number(node.attrs.level) : 0;
    const block = gathering.done(heading ? headings.over(heading) : headings.now());
    if (block) {
      blocks.push(block);
      if (heading) headings.heading(heading, block.text);
    }
    return false;
  });
  return blocks;
}

interface Marked {
  comments: OnWords[];
  /** The comments whose card is open: their mark is drawn as the open one. */
  open: number[];
  set: DecorationSet;
  /** Each stretch that is marked, with the comments it stands for. */
  stretches: { from: number; to: number; seqs: number[]; anchor: Anchor }[];
}

const markedKey = new PluginKey<Marked>('commented');
const openKey = 'commented.open';

/** Where each comment's words are in the document, and the marks over them, the one whose card is open drawn as that. */
function marked(doc: Node, comments: OnWords[], open: number[]): Marked {
  const blocks = blocksOfDoc(doc);
  const placed = comments.flatMap(({ seq, anchor }) => {
    const at = found(anchor, blocks);
    return at ? [{ seq, anchor, ...spanOf(blocks[at.block]!, at.from, at.to) }] : [];
  });
  const stretches = marksOf(placed).map((stretch) => ({ ...stretch, anchor: placed.find((one) => one.seq === stretch.seqs[0])!.anchor }));
  const set = DecorationSet.create(
    doc,
    stretches.map((stretch) =>
      Decoration.inline(stretch.from, stretch.to, {
        class: stretch.seqs.some((seq) => open.includes(seq)) ? 'commented open' : 'commented',
        'data-comments': stretch.seqs.join(' '),
      }),
    ),
  );
  return { comments, open, set, stretches };
}

/** Tells the editor which comments' card is open. */
export function showOpen(view: EditorView, seqs: number[]): void {
  view.dispatch(view.state.tr.setMeta(openKey, seqs));
}

/** Tells the editor which comments are on a few of its words. */
export function markComments(view: EditorView, comments: OnWords[]): void {
  view.dispatch(view.state.tr.setMeta(markedKey, comments));
}

/** The words chosen, where they are within one block and may be commented on. */
function chosen(view: EditorView): { anchor: Anchor; from: number; to: number } | null {
  const { selection } = view.state;
  if (!(selection instanceof TextSelection) || selection.empty) return null;
  const { $from, $to } = selection;
  if ($from.parent !== $to.parent || !$from.parent.isTextblock) return null;
  const blocks = blocksOfDoc(view.state.doc);
  const block = blocks.find((one) => one.at.length && one.at[0]! >= $from.start() && one.at[0]! <= $from.end());
  if (!block) return null;
  const chars = within(block, selection.from, selection.to);
  if (!chars || chars.to - chars.from > MOST_QUOTED) return null;
  const anchor = anchorOf(block, chars.from, chars.to);
  return anchor ? { anchor, from: selection.from, to: selection.to } : null;
}

/** Where the words between two places of the document are on the screen now: from the top of the first line to the bottom of the last. */
function between(view: EditorView, from: number, to: number): Box {
  const start = view.coordsAtPos(from, 1);
  const end = view.coordsAtPos(to, -1);
  return { top: start.top, bottom: end.bottom, left: start.left, right: end.right };
}

/** The bar over words chosen, and the marks over words commented on. */
export function commented(host: HTMLElement, commenting: Commenting): Plugin<Marked> {
  const bar = document.createElement('div');
  bar.className = 'select-bar';
  bar.hidden = true;
  bar.setAttribute('role', 'toolbar');
  bar.setAttribute('aria-label', 'Chosen words');
  const button = document.createElement('button');
  button.type = 'button';
  button.className = 'ghost';
  button.textContent = 'Comment';
  bar.append(button);
  host.append(bar);
  // Whether the pointer is held: nothing is drawn under it while a selection is dragged out.
  let held = false;
  let now: ReturnType<typeof chosen> = null;
  // The words a comment was begun on: the bar is not drawn over them again until something else is chosen.
  let begun: string | null = null;

  const draw = (view: EditorView) => {
    now = !held && commenting.takes() ? chosen(view) : null;
    if (now && `${now.from}:${now.to}` === begun) now = null;
    else if (!now || `${now.from}:${now.to}` !== begun) begun = null;
    bar.hidden = now === null;
    if (!now) return;
    const start = view.coordsAtPos(now.from);
    const within = host.getBoundingClientRect();
    const height = bar.offsetHeight || 32;
    // Over the first line of what is chosen, and under the last where that would be above the page's top.
    let top = start.top - within.top + host.scrollTop - height - 6;
    if (top < host.scrollTop) top = view.coordsAtPos(now.to).bottom - within.top + host.scrollTop + 6;
    const left = Math.max(host.scrollLeft, Math.min(start.left - within.left, host.clientWidth - bar.offsetWidth - 8));
    bar.style.top = `${top}px`;
    bar.style.left = `${left}px`;
  };

  return new Plugin<Marked>({
    key: markedKey,
    state: {
      init: (_config, state) => marked(state.doc, [], []),
      apply(tr, was, _old, state) {
        const comments = tr.getMeta(markedKey) as OnWords[] | undefined;
        const open = tr.getMeta(openKey) as number[] | undefined;
        if (comments || open) return marked(state.doc, comments ?? was.comments, open ?? was.open);
        return tr.docChanged ? marked(state.doc, was.comments, was.open) : was;
      },
    },
    view: (view) => {
      const down = () => {
        held = true;
        draw(view);
      };
      const up = () => {
        if (!held) return;
        held = false;
        // The selection a press makes is the editor's a moment after the pointer lets go.
        requestAnimationFrame(() => draw(view));
      };
      view.dom.addEventListener('mousedown', down);
      window.addEventListener('mouseup', up);
      // Before the editor loses the selection to the press.
      button.addEventListener('mousedown', (event) => event.preventDefault());
      button.addEventListener('click', () => {
        if (!now) return;
        const { from, to } = now;
        commenting.begin(now.anchor, () => between(view, from, to));
        begun = `${from}:${to}`;
        bar.hidden = true;
      });
      draw(view);
      return {
        update: draw,
        destroy: () => {
          view.dom.removeEventListener('mousedown', down);
          window.removeEventListener('mouseup', up);
          bar.remove();
        },
      };
    },
    props: {
      decorations: (state) => markedKey.getState(state)?.set ?? null,
      handleClick(view, pos, event) {
        // A press on a mark, which made no selection, opens its comments; the cursor is put there all the same.
        const target = (event.target as Element | null)?.closest?.('.commented');
        if (!target) return false;
        const stretches = markedKey.getState(view.state)?.stretches ?? [];
        const here = stretches.filter((one) => one.from <= pos && pos <= one.to);
        const one = here.at(-1);
        // Where it is asked of the document and not of the element pressed, which is drawn again as the open one.
        if (one) commenting.open(here.flatMap((each) => each.seqs), one.anchor, () => between(view, one.from, one.to));
        return false;
      },
    },
  });
}

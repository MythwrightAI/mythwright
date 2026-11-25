// Footnotes in the block editor: `[^1]` typed is a marker, drawn as a
// small raised number in the order it is first used, as on the page
// (`cited.ts`); a line begun `[^1]: ` is that marker's source, drawn with
// its number. A marker pointed at shows its source in the card the page
// shows it in, and pressed goes to it in the article's References.

import { h, render } from 'preact';
import { InputRule } from 'prosemirror-inputrules';
import type { Node } from 'prosemirror-model';
import { Plugin } from 'prosemirror-state';
import { Decoration, DecorationSet } from 'prosemirror-view';
import { NOT_WRITTEN } from '../cited.ts';
import { Prose } from '../ui.tsx';
import { SourceCard, toNote } from '../views/cited.tsx';
import { schema, written } from './markdown.ts';

const { footnote_ref: marker, footnote_def: source, paragraph } = schema.nodes;

/** What a marker typed in a line is, and a line begun with a marker and `: `. */
export const footnoteRules: InputRule[] = [
  new InputRule(/\[\^([^\]\s]{1,24})\]$/, (state, match, start, end) => state.tr.replaceWith(start, end, marker.create({ label: match[1] }))),
  new InputRule(/^￼:\s$/, (state, _match, start, end) => {
    const $start = state.doc.resolve(start);
    const first = $start.parent.firstChild;
    if ($start.parent.type !== paragraph || $start.parentOffset !== 0 || first?.type !== marker) return null;
    const tr = state.tr.delete(start, end);
    try {
      return tr.setBlockType(start, start, source, { label: first.attrs.label });
    } catch {
      // A paragraph that must stay one (the first of an item in a list) keeps its marker as it is.
      return null;
    }
  }),
];

/** The number of each label in a document: in the order its markers are first used, then the sources nothing uses. */
export function numbersOf(doc: Node): Map<string, number> {
  const numbers = new Map<string, number>();
  const later: string[] = [];
  doc.descendants((node) => {
    if (node.type === marker && !numbers.has(node.attrs.label)) numbers.set(node.attrs.label, numbers.size + 1);
    if (node.type === source) later.push(node.attrs.label);
    return true;
  });
  for (const label of later) if (!numbers.has(label)) numbers.set(label, numbers.size + 1);
  return numbers;
}

/** Each marker and each source drawn with its number. */
export const numbered = new Plugin({
  props: {
    decorations(state) {
      const numbers = numbersOf(state.doc);
      if (numbers.size === 0) return null;
      const drawn: Decoration[] = [];
      state.doc.descendants((node, at) => {
        if (node.type === marker || node.type === source) drawn.push(Decoration.node(at, at + node.nodeSize, { 'data-n': String(numbers.get(node.attrs.label) ?? '') }));
        return true;
      });
      return DecorationSet.create(state.doc, drawn);
    },
  },
});

/** What the source of `label` says in a document, as it is written; nothing where it is not written. */
function sourceOf(doc: Node, label: string): string | null {
  let found: Node | null = null;
  doc.descendants((node) => {
    if (!found && node.type === source && node.attrs.label === label) found = node;
    return !found;
  });
  const one = found as Node | null;
  return one ? written(schema.node('doc', null, [paragraph.create(null, one.content)])).trim() : null;
}

/** A marker's source in a card while it is pointed at; and, pressed, the source in the article's References. */
export const sourceCards = new Plugin({
  view(view) {
    const holder = document.createElement('div');
    document.body.append(holder);
    let leaving: number | null = null;
    const stay = () => {
      if (leaving !== null) window.clearTimeout(leaving);
      leaving = null;
    };
    const hide = () => {
      stay();
      render(null, holder);
    };
    const hideSoon = () => {
      stay();
      leaving = window.setTimeout(hide, 200);
    };
    const over = (event: MouseEvent) => {
      const mark = (event.target as Element | null)?.closest?.('.writer-cite');
      if (!(mark instanceof HTMLElement) || !view.dom.contains(mark)) return;
      stay();
      const label = mark.getAttribute('data-cite') ?? '';
      const text = sourceOf(view.state.doc, label);
      const n = Number(mark.getAttribute('data-n')) || 0;
      render(h(SourceCard, { anchor: { current: mark }, n, onEnter: stay, onLeave: hideSoon, children: text ? h(Prose, { text }) : h('span', { class: 'faint' }, NOT_WRITTEN) }), holder);
    };
    const out = (event: MouseEvent) => {
      if ((event.target as Element | null)?.closest?.('.writer-cite')) hideSoon();
    };
    const pressed = (event: MouseEvent) => {
      const mark = (event.target as Element | null)?.closest?.('.writer-cite');
      if (mark) toNote(mark, mark.getAttribute('data-cite') ?? '');
    };
    view.dom.addEventListener('mouseover', over);
    view.dom.addEventListener('mouseout', out);
    view.dom.addEventListener('click', pressed);
    return {
      destroy: () => {
        view.dom.removeEventListener('mouseover', over);
        view.dom.removeEventListener('mouseout', out);
        view.dom.removeEventListener('click', pressed);
        hide();
        holder.remove();
      },
    };
  },
});

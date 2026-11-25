// Pictures in the block editor: drawn as the page draws them
// (`views/pictured.tsx`), with a small bar on a picture chosen that sets
// where it stands (Side, Across) and its caption; and a picture, an SVG or
// a page that is pasted, dropped or chosen from the `/` menu, attached to
// what the document is of and put in the text on a line of its own.
//
// A picture stays the Markdown image it is (`![caption](attachment:<id>
// "side")`): the bar changes its words in brackets and in quotes, and
// nothing else, so what is saved is the text `pictures.ts` reads.

import { h, render } from 'preact';
import { schema } from './markdown.ts';
import type { Node } from 'prosemirror-model';
import { NodeSelection, Plugin, TextSelection } from 'prosemirror-state';
import type { EditorView, NodeView } from 'prosemirror-view';
import { ATTACHED, captionOf, CHOSEN, placeOf } from '../pictures.ts';
import type { Place } from '../pictures.ts';
import { toast } from '../store.ts';
import { pasted } from '../views/attaching.tsx';
import { Pictured } from '../views/pictured.tsx';

/** What the page lets the editor do with pictures: the record they are attached to, and how files are attached to it. */
export interface Picturing {
  record: string;
  /** Attaches those of the files that are put in a text, as the record's: what each is. The page says why of any refused. */
  attach: (files: File[]) => Promise<{ id: string; name: string; media: string }[]>;
}

const { image, paragraph } = schema.nodes;

/**
 * A picture in the editor: drawn as on the page, and, while it is chosen,
 * a bar over it that sets where it stands and what its caption says.
 */
class PictureView implements NodeView {
  dom: HTMLElement;
  private drawn: HTMLElement;
  private bar: HTMLElement;
  private caption: HTMLInputElement;
  private places: Record<Place, HTMLButtonElement>;
  private node: Node;
  private view: EditorView;
  private getPos: () => number | undefined;
  private record: string | null;

  constructor(node: Node, view: EditorView, getPos: () => number | undefined, record: string | null) {
    this.node = node;
    this.view = view;
    this.getPos = getPos;
    this.record = record;
    this.dom = document.createElement('span');
    this.dom.className = 'writer-picture';
    this.dom.contentEditable = 'false';
    this.drawn = document.createElement('span');
    this.drawn.className = 'writer-picture-drawn';
    this.bar = document.createElement('span');
    this.bar.className = 'writer-picture-bar';
    this.bar.setAttribute('role', 'toolbar');
    this.bar.setAttribute('aria-label', 'Picture');
    const place = (to: Place, label: string, says: string) => {
      const button = document.createElement('button');
      button.type = 'button';
      button.className = 'ghost';
      button.textContent = label;
      button.title = says;
      button.addEventListener('click', () => this.set('title', to === 'across' ? null : to));
      return button;
    };
    this.places = {
      side: place('side', 'Side', 'At the side, with the words beside it on a wide screen'),
      across: place('across', 'Across', 'Across the column'),
    };
    this.caption = document.createElement('input');
    this.caption.type = 'text';
    this.caption.className = 'writer-picture-caption';
    this.caption.placeholder = 'Add a caption';
    this.caption.setAttribute('aria-label', 'Caption');
    this.caption.addEventListener('input', () => this.set('alt', this.caption.value));
    this.caption.addEventListener('keydown', (event) => {
      // Enter and Escape give the keyboard back to the text, after the picture.
      if (event.key !== 'Enter' && event.key !== 'Escape') return;
      event.preventDefault();
      const at = this.getPos();
      if (at === undefined) return;
      this.view.dispatch(this.view.state.tr.setSelection(TextSelection.near(this.view.state.doc.resolve(at + this.node.nodeSize))));
      this.view.focus();
    });
    this.bar.append(this.places.side, this.places.across, this.caption);
    this.dom.append(this.drawn, this.bar);
    this.draw();
  }

  private draw() {
    const picture = { caption: this.node.attrs['alt'] ?? '', to: this.node.attrs['src'] ?? '', place: placeOf(this.node.attrs['title']) };
    this.dom.dataset['place'] = picture.place ?? 'across';
    render(h(Pictured, { picture, record: this.record, figure: true, still: true }), this.drawn);
    for (const [place, button] of Object.entries(this.places)) button.setAttribute('aria-pressed', String((picture.place ?? 'across') === place));
    if (document.activeElement !== this.caption) this.caption.value = picture.caption;
  }

  /** Changes one of its words, keeping it chosen. */
  private set(attr: 'alt' | 'title', value: string | null) {
    const at = this.getPos();
    if (at === undefined) return;
    const tr = this.view.state.tr.setNodeAttribute(at, attr, value);
    this.view.dispatch(tr.setSelection(NodeSelection.create(tr.doc, at)));
  }

  update(node: Node): boolean {
    if (node.type !== this.node.type) return false;
    this.node = node;
    this.draw();
    return true;
  }

  selectNode() {
    this.dom.classList.add('chosen');
  }

  deselectNode() {
    this.dom.classList.remove('chosen');
  }

  // What is pressed or typed in its bar is the bar's, not the editor's.
  stopEvent(event: Event): boolean {
    return event.target instanceof globalThis.Node && this.bar.contains(event.target);
  }

  ignoreMutation(): boolean {
    return true;
  }

  destroy() {
    render(null, this.drawn);
  }
}

/** How the editor draws each picture, as the record `record`'s where it names an attachment. */
export const pictureView = (record: string | null) => (node: Node, view: EditorView, getPos: () => number | undefined) => new PictureView(node, view, getPos, record);

/**
 * Puts these in the text, each a picture on a line of its own: in place of
 * the empty line the cursor is on, or after the block it is in.
 */
function putIn(view: EditorView, attached: { id: string; name: string; media: string }[]): void {
  if (attached.length === 0) return;
  const lines = attached.map((one) => paragraph.create(null, image.create({ src: `${ATTACHED}${one.id}`, alt: captionOf(one.name, one.media), title: null })));
  const { state } = view;
  const { $from } = state.selection;
  const emptyLine = $from.depth === 1 && $from.parent.type === paragraph && $from.parent.content.size === 0;
  const tr = emptyLine ? state.tr.replaceWith($from.before(1), $from.after(1), lines) : state.tr.insert($from.depth === 0 ? $from.pos : $from.after(1), lines);
  view.dispatch(tr.scrollIntoView());
}

/** Attaches the files that are put in a text, and puts them in. */
async function takeIn(view: EditorView, picturing: Picturing, files: File[]): Promise<void> {
  const taken = await picturing.attach(files);
  if (!view.isDestroyed) putIn(view, taken);
}

/** Asks for pictures or a page from the device, and puts what is chosen in the text where the cursor is. */
export function choosePicture(view: EditorView, picturing: Picturing): void {
  const chooser = document.createElement('input');
  chooser.type = 'file';
  chooser.accept = CHOSEN;
  chooser.multiple = true;
  chooser.className = 'writer-chooser';
  chooser.addEventListener('change', () => {
    void takeIn(view, picturing, [...(chooser.files ?? [])]);
    chooser.remove();
  });
  // In the page, so that a browser that asks only of what is there asks of it.
  view.dom.parentElement?.append(chooser);
  chooser.click();
}

/** What takes a picture pasted or dropped into the editor. */
export function picturesTaken(picturing: Picturing): Plugin {
  return new Plugin({
    props: {
      handlePaste(view, event) {
        pasted(event, (files) => takeIn(view, picturing, files), (line) => line && toast(line, true));
        return event.defaultPrevented;
      },
      handleDrop(view, event, _slice, moved) {
        const files = [...(event.dataTransfer?.files ?? [])];
        if (moved || files.length === 0) return false;
        event.preventDefault();
        const at = view.posAtCoords({ left: event.clientX, top: event.clientY });
        if (at) view.dispatch(view.state.tr.setSelection(TextSelection.near(view.state.doc.resolve(at.pos))));
        void takeIn(view, picturing, files);
        return true;
      },
    },
  });
}

// A document, shown as blocks to write in. The editor is a separate part of
// the client, fetched the first time a document is opened this way.

import type { ComponentChildren } from 'preact';
import { createPortal } from 'preact/compat';
import { useEffect, useRef, useState } from 'preact/hooks';
import { marginOf, roomy } from '../anchored.ts';
import type { OnWords } from '../anchored.ts';
import type { Anchor } from '../anchors.ts';
import { partMissing } from '../store.ts';
import { offer, offeredBy, putAway } from '../views/grouped.tsx';
import type { Offered, Said } from '../views/grouped.tsx';
import type { Line } from './handled.ts';
import type { Handing } from './handle.ts';
import type { Linking } from './linked.ts';
import type { Picturing } from './pictured.ts';
import type { Writer as Handle } from './prosemirror.ts';
import type { Box, Commenting } from './selected.ts';

/** Where a mark's comments open: at the height of its words in the page, and beside the text where there is room for that. */
export interface Placed {
  top: number;
  margin: { left: number; width: number } | null;
  /** Where the words are on the screen, for what opens anywhere else. */
  box: Box;
}

/** Where words at `box` are in the page that scrolls, and whether there is room beside its text for their comments. */
function placedIn(host: HTMLElement, box: Box): Placed {
  const page = host.getBoundingClientRect();
  const text = host.querySelector('.ProseMirror')?.getBoundingClientRect() ?? page;
  const room = host.clientWidth - (parseFloat(getComputedStyle(host).paddingRight) || 0);
  return { top: box.top - page.top + host.scrollTop, margin: marginOf(text.right - page.left + host.scrollLeft, room), box };
}

/** The menu a block's grip opens, as the client's own menu draws it, under the grip and with the keyboard in it. */
function menu(grip: HTMLElement, lines: Line[], chose: (line: Line) => void): void {
  const items: (Offered | Said)[] = lines.map((line) =>
    'said' in line
      ? { said: line.said }
      : 'turn' in line
        ? { label: line.label, chosen: line.chosen, act: () => chose(line) }
        : { label: line.label, apart: line.apart, look: line.act === 'delete' ? 'grave' : undefined, act: () => chose(line) },
  );
  const at = grip.getBoundingClientRect();
  offer({ x: at.left, y: at.bottom + 2 }, items, { from: grip, label: 'Block', fit: true });
}

/**
 * `elsewhere` says the cursor was put somewhere else on the page as the document was opened (where a comment is added), and is left there.
 * `onComment` is there where the document takes comments, and begins one on a block.
 */
export function Writer(props: {
  markdown: string;
  onChange: (markdown: string) => void;
  onSave: () => void;
  elsewhere?: boolean;
  onComment?: (begun: (draft: string) => string) => void;
  /** The comments on a few of its words, which are marked where they are found. */
  marked?: OnWords[];
  /** A comment begun on words chosen, and the comments of a mark pressed. */
  onBegin?: (anchor: Anchor, placed: Placed) => void;
  onOpen?: (seqs: number[], anchor: Anchor, placed: Placed) => void;
  /** What lies beside the text in the page that scrolls: a mark's comments in its margin. */
  beside?: ComponentChildren;
  /** The comments whose card is open, whose mark is drawn as the open one. */
  opened?: number[];
  /** Told whether the page makes room to the right of its column, so that what stands over the text keeps to it. */
  onMargined?: (margined: boolean) => void;
  /** Where the document takes pictures pasted, dropped or chosen (a topic's): what they are attached to, and how. */
  picturing?: Picturing;
  /** Where the document links to other pages as `[[` is typed (a topic's): the page it is. */
  linking?: Linking;
  /** Handed the editor once it is made, a moment after it is drawn: what puts the cursor at a section, and says where it is. */
  onReady?: (editor: Handle) => void;
}) {
  const host = useRef<HTMLDivElement>(null);
  const latest = useRef(props);
  latest.current = props;
  const handle = useRef<Handle | null>(null);
  // Where what lies beside the text is put: in the page that scrolls, apart from the editor's own.
  const [aside, setAside] = useState<HTMLElement | null>(null);
  // Whether the page is wide enough to make room beside its column, and whether a card was asked for on it: once it
  // has made room it keeps it, so that a press in the text that puts a card away moves nothing under the pointer.
  const [wide, setWide] = useState(false);
  const [asked, setAsked] = useState(false);
  const margined = wide && (asked || (props.marked?.length ?? 0) > 0);
  const now = useRef({ wide, margined });
  now.current = { wide, margined };
  useEffect(() => {
    const page = host.current;
    if (!page) return;
    const measure = () => {
      const style = getComputedStyle(page);
      setWide(roomy(page.clientWidth - (parseFloat(style.paddingLeft) || 0) - (parseFloat(style.paddingRight) || 0)));
    };
    const watching = new ResizeObserver(measure);
    watching.observe(page);
    measure();
    return () => watching.disconnect();
  }, []);
  useEffect(() => props.onMargined?.(margined), [margined]);

  useEffect(() => {
    let gone = false;
    let writer: Handle | null = null;
    void import('./prosemirror.ts').then(({ createWriter }) => {
      if (gone || !host.current) return;
      const handing: Handing = {
        menu,
        open: offeredBy,
        close: putAway,
        // Asked each time, so that a document that comes to take comments, or stops, is offered the line as it is then.
        get comment() {
          const commenting = latest.current.onComment;
          return commenting ? (begun: (draft: string) => string) => commenting(begun) : undefined;
        },
      };
      // Room is made first where the page has none yet, and where the words are is asked once it is laid out so.
      const placed = (at: () => Box, then: (placed: Placed) => void) => {
        const room = now.current.wide && !now.current.margined;
        if (room) setAsked(true);
        const place = () => host.current && then(placedIn(host.current, at()));
        if (!room) return place();
        requestAnimationFrame(() => requestAnimationFrame(place));
      };
      const commenting: Commenting = {
        takes: () => latest.current.onBegin !== undefined,
        begin: (anchor, at) => placed(at, (where) => latest.current.onBegin?.(anchor, where)),
        open: (seqs, anchor, at) => placed(at, (where) => latest.current.onOpen?.(seqs, anchor, where)),
      };
      writer = createWriter({
        parent: host.current,
        // What it is given once. After that the editor is the one that knows
        // what the document says, and tells the page of each change.
        markdown: latest.current.markdown,
        onChange: (markdown) => latest.current.onChange(markdown),
        onSave: () => latest.current.onSave(),
        handing,
        commenting,
        picturing: latest.current.picturing,
        linking: latest.current.linking,
      });
      handle.current = writer;
      writer.mark(latest.current.marked ?? []);
      writer.opened(latest.current.opened ?? []);
      const beside = document.createElement('div');
      beside.className = 'writer-beside';
      host.current.append(beside);
      setAside(beside);
      if (!latest.current.elsewhere) writer.focus();
      latest.current.onReady?.(writer);
    }, partMissing);
    return () => {
      gone = true;
      writer?.destroy();
      handle.current = null;
    };
  }, []);
  useEffect(() => handle.current?.mark(props.marked ?? []), [props.marked]);
  useEffect(() => handle.current?.opened(props.opened ?? []), [props.opened]);

  return (
    <div class={margined ? 'writer page margined' : 'writer page'} ref={host}>
      {aside && props.beside ? createPortal(<>{props.beside}</>, aside) : null}
    </div>
  );
}

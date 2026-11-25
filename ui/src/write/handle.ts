// The handle beside a block: pointed at (or with the
// cursor in it, where nothing points), a block of the document has a `+`
// and a grip in the margin to its left. The `+` adds an empty block under
// it with the `/` menu open there; the grip carried moves the block, and
// pressed opens a menu of what can be done to it. Ctrl and / opens the same
// menu for the block the cursor is in.
//
// The handle and the line that says where a carried block lands lie over
// the page in the editor's own scrolling box and are no part of the
// document, so nothing of the text moves as they come and go; they are
// there, and only hidden, before anything is pressed. Whatever is done is
// a change of the document like any typed one, saved as any is. What is
// decided (which block, where it lands, what it becomes, which lines the
// menu has) is `handled.ts`, a function with its cases.

import { schema } from './markdown.ts';
import { Fragment } from 'prosemirror-model';
import type { Node } from 'prosemirror-model';
import { Plugin, PluginKey, Selection, TextSelection } from 'prosemirror-state';
import type { Transaction } from 'prosemirror-state';
import { Decoration, DecorationSet } from 'prosemirror-view';
import type { EditorView } from 'prosemirror-view';
import { blockAt, commentOn, kindOf, landing, lineAt, linesOf, moved, turning } from './handled.ts';
import type { Kind, Line, Span } from './handled.ts';

const { nodes } = schema;

/** What the page round the editor does for the handle: draws its menu, and adds a comment where the document takes them. */
export interface Handing {
  /** Opens the menu under the grip; `chose` is told which line was pressed. */
  menu(grip: HTMLElement, lines: Line[], chose: (line: Line) => void): void;
  /** Whether the menu the grip opened is still open. */
  open(grip: HTMLElement): boolean;
  /** Puts that menu away: the grip pressed again does. */
  close(): void;
  /** Begins a comment on the document, saying which block it is about. Where there is none, the document takes no comments. */
  comment?: (begun: (draft: string) => string) => void;
}

const carriedKey = new PluginKey<number | null>('carried');

/** Where each block of the document begins. */
function starts(doc: Node): number[] {
  const at: number[] = [];
  doc.forEach((_node, offset) => at.push(offset));
  return at;
}

/** The lines a block holds, in order: each paragraph, heading, item and line of code, with what is marked in it. */
function linesIn(node: Node): Fragment[] {
  const code = (one: Node) => one.textContent.split('\n').map((text) => (text ? Fragment.from(schema.text(text)) : Fragment.empty));
  // A table kept as its text is held as lines, as code is.
  if (node.type.spec.code) return code(node);
  if (node.isTextblock) return [node.content];
  const lines: Fragment[] = [];
  node.descendants((child) => {
    if (child.type.spec.code) lines.push(...code(child));
    else if (child.isTextblock) lines.push(child.content);
    else return true;
    return false;
  });
  return lines;
}

/** The blocks that what a block held is turned into. */
function built(to: Kind, lines: Fragment[]): Node[] {
  const plain = (line: Fragment) => line.textBetween(0, line.size, '\n', '\n');
  const para = (line: Fragment) => nodes.paragraph.create(null, line);
  return turning(to, lines.length).map(({ kind, lines: held }) => {
    const mine = held.map((at) => lines[at] ?? Fragment.empty);
    switch (kind) {
      case 'h1':
      case 'h2':
      case 'h3':
        return nodes.heading.create({ level: Number(kind.slice(1)) }, mine[0]);
      case 'bullets':
      case 'numbers':
        return (kind === 'bullets' ? nodes.bullet_list : nodes.ordered_list).create(
          { tight: true },
          mine.map((line) => nodes.list_item.create(null, para(line))),
        );
      case 'quote':
        return nodes.blockquote.create(null, mine.map(para));
      case 'code': {
        const text = mine.map(plain).join('\n');
        return nodes.code_block.create(null, text ? schema.text(text) : null);
      }
      default:
        return para(mine[0] ?? Fragment.empty);
    }
  });
}

/** The cursor at the start of the block that is now at `index`. */
function at(tr: Transaction, index: number): Transaction {
  const begins = starts(tr.doc)[Math.min(index, tr.doc.childCount - 1)] ?? 0;
  return tr.setSelection(Selection.near(tr.doc.resolve(begins + 1)));
}

/** A block taken from where it is and put where it is to be, as one change. */
export function move(view: EditorView, from: number, to: number): void {
  const { doc } = view.state;
  const node = doc.child(from);
  const begins = starts(doc)[from]!;
  const tr = view.state.tr.delete(begins, begins + node.nodeSize);
  tr.insert(starts(tr.doc)[to] ?? tr.doc.content.size, node);
  view.dispatch(at(tr, to).scrollIntoView());
}

function turn(view: EditorView, index: number, to: Kind): void {
  const { doc } = view.state;
  const node = doc.child(index);
  const begins = starts(doc)[index]!;
  const lists = [nodes.bullet_list, nodes.ordered_list];
  // A list turned into the other kind of list keeps its items as they are, nested ones with them.
  const into =
    lists.includes(node.type) && (to === 'bullets' || to === 'numbers')
      ? [(to === 'bullets' ? nodes.bullet_list : nodes.ordered_list).create({ tight: node.attrs.tight ?? true }, node.content)]
      : built(to, linesIn(node));
  view.dispatch(at(view.state.tr.replaceWith(begins, begins + node.nodeSize, into), index));
}

function duplicate(view: EditorView, index: number): void {
  const node = view.state.doc.child(index);
  const ends = starts(view.state.doc)[index]! + node.nodeSize;
  view.dispatch(at(view.state.tr.insert(ends, node), index + 1).scrollIntoView());
}

function remove(view: EditorView, index: number): void {
  const { doc } = view.state;
  const begins = starts(doc)[index]!;
  const ends = begins + doc.child(index).nodeSize;
  // A document is never nothing: the last block goes, and an empty line is left to write in.
  const tr = doc.childCount === 1 ? view.state.tr.replaceWith(begins, ends, nodes.paragraph.create()) : view.state.tr.delete(begins, ends);
  view.dispatch(at(tr, Math.max(0, index - (index >= tr.doc.childCount ? 1 : 0))));
}

/** A new empty line under the block, with the `/` menu open on it. */
function addUnder(view: EditorView, index: number): void {
  const node = view.state.doc.child(index);
  const ends = starts(view.state.doc)[index]! + node.nodeSize;
  const tr = view.state.tr.insert(ends, nodes.paragraph.create(null, schema.text('/')));
  view.dispatch(tr.setSelection(TextSelection.create(tr.doc, ends + 2)).scrollIntoView());
  view.focus();
}

/** Whether the keys pressed are the ones that open a block's menu. */
const opensMenu = (event: KeyboardEvent) => event.key === '/' && (event.ctrlKey || event.metaKey) && !event.shiftKey && !event.altKey;

export function blockHandle(host: HTMLElement, handing: Handing): Plugin {
  const box = document.createElement('div');
  box.className = 'block-handle';
  box.dataset.shown = 'false';
  const add = document.createElement('button');
  add.type = 'button';
  add.className = 'block-add';
  add.setAttribute('aria-label', 'Add a block below');
  add.title = 'Add a block below';
  add.textContent = '+';
  const grip = document.createElement('button');
  grip.type = 'button';
  grip.className = 'block-grip';
  grip.setAttribute('aria-label', 'Drag to move, or open the menu');
  grip.setAttribute('aria-haspopup', 'menu');
  grip.title = 'Drag to move · Click to open the menu';
  grip.innerHTML = '<svg viewBox="0 0 10 16" aria-hidden="true"><circle cx="2.5" cy="3" r="1.3"/><circle cx="7.5" cy="3" r="1.3"/><circle cx="2.5" cy="8" r="1.3"/><circle cx="7.5" cy="8" r="1.3"/><circle cx="2.5" cy="13" r="1.3"/><circle cx="7.5" cy="13" r="1.3"/></svg>';
  box.append(add, grip);
  const drop = document.createElement('div');
  drop.className = 'block-drop';
  drop.hidden = true;
  host.append(box, drop);
  // Where nothing points (a finger), the handle is of the block the cursor is in.
  const touched = window.matchMedia('(hover: none)');

  let view: EditorView | null = null;
  /** The block the handle is beside, or none. */
  let shown: number | null = null;
  let pointerY: number | null = null;
  let frame = 0;
  /** A carry under way: the block, where the pointer began, and where it would land. */
  let carry: { from: number; x: number; y: number; going: boolean; gap: number } | null = null;
  /** The click that ends a carry opens nothing (and the next press of the grip forgets it, should no click have come). */
  let carried = false;

  const spans = (): Span[] => {
    if (!view) return [];
    const doc = view.state.doc;
    return starts(doc).map((begins) => {
      const dom = view!.nodeDOM(begins);
      const rect = dom instanceof HTMLElement ? dom.getBoundingClientRect() : { top: 0, bottom: 0 };
      return { top: rect.top, bottom: rect.bottom };
    });
  };

  /** Where the padding box of the editor's scrolling box begins, which is what what lies in it is placed from. */
  const origin = () => {
    box.style.left = '0px';
    box.style.top = '0px';
    const rect = box.getBoundingClientRect();
    return { left: rect.left, top: rect.top };
  };

  const hide = () => {
    if (shown === null && box.dataset.shown === 'false') return;
    shown = null;
    box.dataset.shown = 'false';
  };

  /** The middle of a block's first line, down the page. */
  const firstLine = (index: number): number => {
    const v = view!;
    const node = v.state.doc.child(index);
    const begins = starts(v.state.doc)[index]!;
    let inside: number | null = node.isTextblock ? begins + 1 : null;
    if (inside === null)
      node.descendants((child, offset) => {
        if (inside !== null) return false;
        if (child.isTextblock) inside = begins + 1 + offset + 1;
        return inside === null;
      });
    if (inside !== null) {
      const line = v.coordsAtPos(inside);
      return (line.top + line.bottom) / 2;
    }
    const rect = (v.nodeDOM(begins) as HTMLElement).getBoundingClientRect();
    return (rect.top + rect.bottom) / 2;
  };

  const show = (index: number | null) => {
    if (!view || index === null || index >= view.state.doc.childCount) return hide();
    shown = index;
    const from = origin();
    const column = view.dom.getBoundingClientRect().left;
    // Both where there is room in the margin for both; the grip alone where there is room for that; else none, since nothing lies over the text.
    box.dataset.room = 'both';
    const gap = 4;
    let wide = box.offsetWidth;
    if (column - from.left < wide + gap) {
      box.dataset.room = 'grip';
      wide = box.offsetWidth;
    }
    if (column - from.left < wide + 1 && !(carry || handing.open(grip))) return hide();
    box.style.left = `${Math.max(0, column - from.left - wide - gap)}px`;
    box.style.top = `${firstLine(index) - from.top - box.offsetHeight / 2}px`;
    box.dataset.shown = 'true';
  };

  const follow = () => {
    frame = 0;
    if (carry || pointerY === null || !view) return;
    show(blockAt(spans(), pointerY));
  };

  const onMove = (event: PointerEvent) => {
    if (event.pointerType === 'touch' || carry) return;
    // Over the handle itself it stays where it is, whatever room between two blocks the pointer crosses.
    if (box.contains(event.target as globalThis.Node)) return;
    pointerY = event.clientY;
    if (!frame) frame = requestAnimationFrame(follow);
  };
  const onLeave = (event: PointerEvent) => {
    // A finger lifted leaves everything it was on: the handle is the cursor's there, and stays.
    if (event.pointerType === 'touch') return;
    pointerY = null;
    if (carry || box.contains(document.activeElement) || handing.open(grip)) return;
    hide();
  };
  host.addEventListener('pointermove', onMove);
  host.addEventListener('pointerleave', onLeave);

  const chose = (index: number) => (line: Line) => {
    const v = view;
    if (!v || index >= v.state.doc.childCount) return;
    if ('turn' in line) turn(v, index, line.turn);
    else if ('act' in line) {
      if (line.act === 'duplicate') duplicate(v, index);
      else if (line.act === 'up') move(v, index, index - 1);
      else if (line.act === 'down') move(v, index, index + 1);
      else if (line.act === 'delete') remove(v, index);
      else if (line.act === 'comment') {
        const words = v.state.doc.child(index).textContent;
        hide();
        handing.comment?.((draft) => commentOn(words, draft));
        return;
      }
    }
    v.focus();
  };

  const openMenu = (index: number) => {
    const v = view;
    if (!v) return;
    show(index);
    const node = v.state.doc.child(index);
    const lines = linesOf({ kind: kindOf({ type: node.type.name, level: node.attrs.level }), at: index, of: v.state.doc.childCount, comments: Boolean(handing.comment) });
    handing.menu(grip, lines, chose(index));
  };

  add.addEventListener('mousedown', (event) => event.preventDefault());
  add.addEventListener('click', () => {
    if (view && shown !== null) addUnder(view, shown);
  });

  const ending = () => {
    const was = carry;
    carry = null;
    drop.hidden = true;
    host.classList.remove('block-carrying');
    if (was?.going && view) view.dispatch(view.state.tr.setMeta(carriedKey, null));
    return was;
  };
  grip.addEventListener('pointerdown', (event) => {
    carried = false;
    if (event.button !== 0 || shown === null) return;
    // No text is chosen by it, and the pointer stays the grip's until it is let go.
    event.preventDefault();
    grip.setPointerCapture(event.pointerId);
    carry = { from: shown, x: event.clientX, y: event.clientY, going: false, gap: shown };
  });
  grip.addEventListener('pointermove', (event) => {
    if (!carry || !view) return;
    if (!carry.going) {
      if (Math.hypot(event.clientX - carry.x, event.clientY - carry.y) < 4) return;
      carry.going = true;
      host.classList.add('block-carrying');
      view.dispatch(view.state.tr.setMeta(carriedKey, carry.from));
    }
    // Near the top or the foot of what scrolls, it scrolls, so that a block can be carried further than the page shows.
    const within = host.getBoundingClientRect();
    if (event.clientY < within.top + 32) host.scrollTop -= 12;
    else if (event.clientY > within.bottom - 32) host.scrollTop += 12;
    const all = spans();
    carry.gap = landing(all, event.clientY);
    const from = origin();
    // Back where the block was, nothing would move, and no line says otherwise.
    drop.hidden = moved(carry.from, carry.gap) === null;
    const column = view.dom.getBoundingClientRect();
    drop.style.left = `${column.left - from.left}px`;
    drop.style.width = `${column.width}px`;
    drop.style.top = `${lineAt(all, carry.gap) - from.top - 1}px`;
    show(carry.from);
  });
  grip.addEventListener('pointerup', () => {
    const was = ending();
    if (!was?.going || !view) return;
    carried = true;
    const to = moved(was.from, was.gap);
    if (to !== null) move(view, was.from, to);
    show(to ?? was.from);
  });
  grip.addEventListener('pointercancel', () => {
    ending();
  });
  grip.addEventListener('click', () => {
    if (carried) {
      carried = false;
      return;
    }
    if (handing.open(grip)) handing.close();
    else if (shown !== null) openMenu(shown);
  });
  grip.addEventListener('keydown', (event) => {
    if (event.key !== 'Escape' || !view) return;
    event.preventDefault();
    view.focus();
  });
  const onEscape = (event: KeyboardEvent) => {
    if (event.key !== 'Escape' || !carry) return;
    // Put back where it was; the press that is let go after this opens nothing either.
    event.stopPropagation();
    carried = carry.going;
    ending();
    show(shown);
  };
  window.addEventListener('keydown', onEscape, true);

  return new Plugin({
    key: carriedKey,
    state: {
      init: (): number | null => null,
      apply: (tr, value: number | null): number | null => {
        const said = tr.getMeta(carriedKey);
        return said === undefined ? value : (said as number | null);
      },
    },
    view: (editor) => {
      view = editor;
      return {
        update(editor, before) {
          view = editor;
          const finger = touched.matches && editor.hasFocus();
          if (finger && (!before.selection.eq(editor.state.selection) || !before.doc.eq(editor.state.doc))) return show(editor.state.selection.$from.index(0));
          if (shown !== null && !before.doc.eq(editor.state.doc)) show(Math.min(shown, editor.state.doc.childCount - 1));
        },
        destroy() {
          cancelAnimationFrame(frame);
          host.removeEventListener('pointermove', onMove);
          host.removeEventListener('pointerleave', onLeave);
          window.removeEventListener('keydown', onEscape, true);
          box.remove();
          drop.remove();
        },
      };
    },
    props: {
      decorations(state) {
        const index = carriedKey.getState(state);
        if (index === null || index === undefined || index >= state.doc.childCount) return null;
        const begins = starts(state.doc)[index]!;
        return DecorationSet.create(state.doc, [Decoration.node(begins, begins + state.doc.child(index).nodeSize, { class: 'block-carried' })]);
      },
      handleKeyDown(editor, event) {
        if (opensMenu(event)) {
          openMenu(editor.state.selection.$from.index(0));
          return true;
        }
        // Writing puts the handle away until the pointer moves again; a finger's handle stays with the cursor.
        if (!touched.matches) hide();
        return false;
      },
    },
  });
}

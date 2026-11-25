// A line of a tree in the side panel carried to another place in it, by
// the pointer and by the keyboard. It knows no outline, project or topic:
// a project's outline hands it what a landing is (`outlinedrag.tsx`), and so may anything else drawn as a tree of lines in a panel (a
// wiki's pages). The rules of where a line lands are plain
// values (`treedrag.ts`: `landing`, `stepped`, `steppedRows`).
//
// As a column's head is carried: held and moved past a few pixels it is
// carried, and a press is a press until then. Nothing appears or moves
// under the pointer as the carry begins: the line carried is drawn fainter
// where it stands, the lines' boxes are measured once as it is taken up
// (and again only when the panel is scrolled under it), and what says
// where it would land lies over the page, out of the flow: a line between
// two lines, or a folder's line outlined, which is landed in at its end and
// is not unfolded under the pointer. Escape puts it back and tells nobody;
// let go where it began, or outside the tree, nobody is told; let go
// elsewhere it is one `land`. A press that carried opens nothing. A finger
// does not carry: it scrolls the panel, and whatever offers Move to… is its
// way.
//
// The keyboard's way is Alt and the arrows on a line: up and down among its
// level, right into the folder above it, left out of its folder, each step
// one `land`, the line keeping the keyboard.
//
// What it reads of the page (the tree's lines are found as they are drawn,
// and nothing else is handed in):
// - the handlers it gives are spread on the element that holds the tree;
// - a line that may be carried is an element that says its key as
//   `data-place`, on its own row and never on a wrapper; its own line is
//   that element where it is a `.nav-line` or `.nav-item`, else the first
//   `.nav-line`, `.nav-row` or `.nav-item` in it, and it is taken hold of
//   only by a `.nav-item` whose nearest `[data-place]` or `.tree` is it (so
//   a tree drawn inside a line, a project's Repo, carries nothing);
// - a line that can hold lines (a folder) also says its key as
//   `data-outline`, on the same element, and the lines in it are drawn
//   inside it; it is unfolded where its `.nav-item` says
//   `aria-expanded="true"`, or its arrow does where the arrow is a button
//   of its own beside a line that opens a page (a wiki's page). How deep a
//   line is, is how many such it is in.

import { useRef } from 'preact/hooks';
import { landing, steppedRows } from '../treedrag.ts';
import type { At, Row, Step } from '../treedrag.ts';

/** What a tree hands so that its lines can be carried. */
export interface Carrying {
  /**
   * Puts the line `node` where it was let go or stepped to (`to`: into a
   * folder, or the top, before a line there or at its end). Asked once for
   * each carry that moves something and each step; never for one put back.
   * `by` says whether the pointer or the keyboard took it there. What it
   * resolves to is whether it came there: where it did and the keyboard
   * took it, the line is given the keyboard again where it is drawn now.
   */
  land(node: string, to: At, by: 'pointer' | 'keys'): Promise<unknown>;
  /**
   * Where Alt and an arrow put `node`; nothing for nowhere. Left out, it is
   * worked out from the lines as they are drawn (`steppedRows`).
   */
  step?(node: string, way: Step): At | null;
  /**
   * Whether `node` may be put at `to`. Where it may not, nothing says it
   * would land there and letting it go there tells nobody. Left out, it
   * may be put anywhere in the tree but in itself.
   */
  allows?(node: string, to: At): boolean;
}

/** The handlers a tree's holder is given; none where nothing may be carried. */
export interface Carried {
  onPointerDown?: (event: PointerEvent) => void;
  onClickCapture?: (event: MouseEvent) => void;
  onKeyDown?: (event: KeyboardEvent) => void;
}

/** How far a held line is moved before it is carried: under that, it is a press. */
const CARRIED_PAST = 6;

const STEPS: Record<string, Step> = { ArrowUp: 'up', ArrowDown: 'down', ArrowRight: 'in', ArrowLeft: 'out' };

/** A placeable line's own line on the page: what the pointer presses and what a landing is said beside. */
function lineOf(row: HTMLElement): HTMLElement {
  if (row.matches('.nav-line, .nav-item')) return row;
  return row.querySelector<HTMLElement>('.nav-line, .nav-row, .nav-item') ?? row;
}

/** The placeable line a press or a key is on, where it is one of `tree`'s own: never a line of a tree drawn inside one. */
function rowAt(tree: HTMLElement, target: EventTarget | null): HTMLElement | null {
  const item = target instanceof Element ? target.closest('.nav-item') : null;
  const row = item?.closest<HTMLElement>('[data-place], .tree');
  return row && row.dataset.place !== undefined && tree.contains(row) ? row : null;
}

/** Every placeable line of `tree` as it stands on the page, in the order drawn. */
export function measured(tree: HTMLElement): Row[] {
  return [...tree.querySelectorAll<HTMLElement>('[data-place]')].map((row) => {
    const line = lineOf(row);
    const box = line.getBoundingClientRect();
    const outer = row.parentElement?.closest<HTMLElement>('[data-outline]');
    const folder = row.dataset.outline !== undefined;
    return {
      node: row.dataset.place ?? '',
      into: outer && tree.contains(outer) ? (outer.dataset.outline ?? null) : null,
      folder,
      open: folder && (line.matches('.nav-item[aria-expanded="true"]') || line.querySelector(':scope > :is(.nav-item, .nav-twist)[aria-expanded="true"]') !== null),
      top: box.top,
      bottom: box.bottom,
      end: Math.max(box.bottom, row.getBoundingClientRect().bottom),
      left: box.left,
    };
  });
}

/**
 * What a tree's holder is given so that its lines can be carried: the
 * press that may begin a carry, the click a carry swallows, and the
 * keyboard's steps. Nothing where `carrying` is nothing (whoever reads the
 * tree may not arrange it).
 */
export function useTreeCarry(carrying: Carrying | null): Carried {
  // A press that carried a line is no press of the line.
  const carriedNow = useRef(false);
  if (!carrying) return {};
  const { land, step: stepOf, allows } = carrying;
  const allowed = (node: string, to: At) => !allows || allows(node, to);

  const hold = (event: PointerEvent) => {
    carriedNow.current = false;
    const tree = event.currentTarget as HTMLElement;
    const row = rowAt(tree, event.target);
    if (!row || event.button !== 0 || event.pointerType === 'touch') return;
    const node = row.dataset.place ?? '';
    const from = { x: event.clientX, y: event.clientY };
    let carried: { rows: Row[]; drop: HTMLElement; x: number; y: number; asked: number | null } | null = null;
    /** Where it lands now, where it may: nowhere where it may not. */
    const landed = () => {
      if (!carried) return null;
      const to = landing(carried.rows, node, carried.x, carried.y);
      return to && !to.stays && !allowed(node, { into: to.into, before: to.before }) ? null : to;
    };
    const draw = () => {
      if (!carried) return;
      carried.asked = null;
      const to = landed();
      const drop = carried.drop;
      const right = tree.getBoundingClientRect().right;
      if (to?.line) {
        drop.className = 'outline-drop';
        Object.assign(drop.style, { display: '', top: `${to.line.top - 1}px`, left: `${to.line.left}px`, width: `${Math.max(24, right - to.line.left)}px`, height: '' });
      } else if (to?.folder) {
        const into = carried.rows.find((one) => one.node === to.folder);
        drop.className = 'outline-drop into';
        if (into) Object.assign(drop.style, { display: '', top: `${into.top}px`, left: `${into.left}px`, width: `${Math.max(24, right - into.left)}px`, height: `${into.bottom - into.top}px` });
      } else {
        drop.style.display = 'none';
      }
    };
    const begin = () => {
      // What says where it would land lies over the page: nothing in the tree moves for it.
      const drop = document.createElement('div');
      drop.className = 'outline-drop';
      drop.style.display = 'none';
      document.body.append(drop);
      row.classList.add('outline-carried');
      document.body.classList.add('outline-carrying');
      return { rows: measured(tree), drop, x: from.x, y: from.y, asked: null };
    };
    const ask = () => {
      if (carried && carried.asked === null) carried.asked = requestAnimationFrame(draw);
    };
    const over = (drop: boolean) => {
      window.removeEventListener('pointermove', move);
      window.removeEventListener('pointerup', up);
      window.removeEventListener('pointercancel', off);
      window.removeEventListener('keydown', key, true);
      window.removeEventListener('scroll', scrolled, true);
      if (!carried) return;
      if (carried.asked !== null) cancelAnimationFrame(carried.asked);
      carried.drop.remove();
      row.classList.remove('outline-carried');
      document.body.classList.remove('outline-carrying');
      const to = drop ? landed() : null;
      carried = null;
      carriedNow.current = true;
      // Nobody is told until it is let go; where it was, or nowhere, nobody is told at all.
      if (to && !to.stays) void land(node, { into: to.into, before: to.before }, 'pointer');
    };
    const move = (moved: PointerEvent) => {
      if (!carried) {
        if (Math.hypot(moved.clientX - from.x, moved.clientY - from.y) < CARRIED_PAST) return;
        carried = begin();
      }
      carried.x = moved.clientX;
      carried.y = moved.clientY;
      ask();
    };
    // The panel scrolled under what is carried: its lines are where they are now.
    const scrolled = () => {
      if (!carried) return;
      carried.rows = measured(tree);
      ask();
    };
    const up = () => over(true);
    const off = () => over(false);
    const key = (pressed: KeyboardEvent) => {
      if (pressed.key !== 'Escape' || !carried) return;
      // Escape puts it back, and is nobody else's while it is carried.
      pressed.stopPropagation();
      pressed.preventDefault();
      over(false);
    };
    window.addEventListener('pointermove', move);
    window.addEventListener('pointerup', up);
    window.addEventListener('pointercancel', off);
    window.addEventListener('keydown', key, true);
    window.addEventListener('scroll', scrolled, true);
  };

  const swallow = (event: MouseEvent) => {
    // A press that carried opens nothing and folds nothing; one from the keyboard never carried anything.
    if (!carriedNow.current || event.detail === 0) return;
    carriedNow.current = false;
    event.stopPropagation();
    event.preventDefault();
  };

  const stepped = (event: KeyboardEvent) => {
    const way = event.altKey && !event.ctrlKey && !event.metaKey && !event.shiftKey ? STEPS[event.key] : undefined;
    const tree = event.currentTarget as HTMLElement;
    const row = way ? rowAt(tree, event.target) : null;
    if (!way || !row) return;
    // Alt and an arrow is the step's alone here, and never the browser's going back.
    event.preventDefault();
    event.stopPropagation();
    const node = row.dataset.place ?? '';
    const to = stepOf ? stepOf(node, way) : steppedRows(measured(tree), node, way);
    if (!to || !allowed(node, to)) return;
    void land(node, to, 'keys').then((came) => {
      if (!came) return;
      // The line keeps the keyboard where it has gone.
      requestAnimationFrame(() => {
        const there = [...tree.querySelectorAll<HTMLElement>('[data-place]')].find((one) => one.dataset.place === node);
        const item = there && lineOf(there);
        (item?.matches('.nav-item') ? item : item?.querySelector<HTMLElement>('.nav-item'))?.focus();
      });
    });
  };

  return { onPointerDown: hold, onClickCapture: swallow, onKeyDown: stepped };
}

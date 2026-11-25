// A card that moves to another column slides there, on the
// board's page and on any table laid out in columns (`ColumnsLayout` in
// `laid.tsx`).
//
// What is drawn says how its cards stand, as one word (`arrangementOf`).
// Where that is another than it was last drawn, the boxes of the cards
// are noted as the page is drawn again, before what is drawn reaches the
// page; once it has, they are read again, and each card that is somewhere
// else (`moved` in `slid.ts`) is moved by `transform` alone from where it
// was, by the style sheet in the drawer's time and easing (`--slide`), and
// is then where it always is with nothing on it. Nothing is laid out again
// for it, and no timer is asked.
//
// A card stays inside its column's list, which scrolls, and that list cuts
// off whatever of it is outside: so one whose way begins outside it (from
// another column) is flown over the page as a still copy of it
// (`still.ts`) from where it was to where it is, the card itself waiting
// out of sight in its place until the copy is there, when the copy goes.
//
// Nothing slides under the pointer, nor while somebody carries a card,
// nor for a change somebody made by hand (a card let go where the pointer
// left it is there already): whoever draws the cards says so (`quiet`).
// Where less movement is asked for nothing slides. Several cards moved at
// once (a fleet taking ten) are one drawing, and slide together.

import { useLayoutEffect, useRef } from 'preact/hooks';
import { leaves, moved } from '../slid.ts';
import type { Box } from '../slid.ts';
import { stillOf } from '../still.ts';

/** What a card that slides is called while it does, which the style sheet moves. */
export const SLIDING = 'card-sliding';
/** What the still copy a card is flown as is called. */
export const FLYING = 'card-flying';
/** What a card waiting in its place for its copy to come is called, which the style sheet keeps out of sight. */
export const LANDING = 'card-landing';

/** Where the pointer last was over the page, or nothing where it has left it. */
let pointer: { x: number; y: number } | null = null;
if (typeof document !== 'undefined') {
  const at = (event: PointerEvent) => (pointer = event.pointerType === 'touch' && event.type === 'pointerup' ? null : { x: event.clientX, y: event.clientY });
  document.addEventListener('pointermove', at, { capture: true, passive: true });
  document.addEventListener('pointerdown', at, { capture: true, passive: true });
  document.addEventListener('pointerup', at, { capture: true, passive: true });
  document.documentElement.addEventListener('pointerleave', () => (pointer = null));
}

const stillAsked = (): boolean => typeof matchMedia === 'function' && matchMedia('(prefers-reduced-motion: reduce)').matches;
const boxOf = (one: Element): Box => {
  const { left, top, width, height } = one.getBoundingClientRect();
  return { left, top, width, height };
};

/** The copies flying now, by card, so that one moved again on its way goes on from where its copy is. */
const flying = new Map<string, HTMLElement>();

/** The cards under `root`, each by what it is: one drawn twice (in two bands) is neither. */
function cardsOf(root: HTMLElement): Map<string, HTMLElement> {
  const cards = new Map<string, HTMLElement>();
  const twice = new Set<string>();
  for (const one of root.querySelectorAll<HTMLElement>('[data-slides]')) {
    const id = one.dataset.slides!;
    if (cards.has(id)) twice.add(id);
    cards.set(id, one);
  }
  for (const id of twice) cards.delete(id);
  return cards;
}

/** Where each card is seen now: a card waiting for its copy is where its copy is. */
function seen(root: HTMLElement): Map<string, Box> {
  return new Map([...cardsOf(root)].map(([id, one]) => [id, boxOf(flying.get(id) ?? one)]));
}

function slide(card: HTMLElement, x: number, y: number): void {
  card.style.setProperty('--slid-x', `${x}px`);
  card.style.setProperty('--slid-y', `${y}px`);
  card.classList.add(SLIDING);
}

function fly(id: string, card: HTMLElement, x: number, y: number): void {
  const at = boxOf(card);
  const copy = stillOf(card, { called: FLYING, of: id, whole: true });
  Object.assign(copy.style, { left: `${at.left}px`, top: `${at.top}px`, width: `${at.width}px`, height: `${at.height}px` });
  copy.style.setProperty('--slid-x', `${x}px`);
  copy.style.setProperty('--slid-y', `${y}px`);
  document.body.append(copy);
  card.classList.add(LANDING);
  flying.set(id, copy);
  const moving = copy.getAnimations();
  const landed = () => {
    copy.remove();
    if (flying.get(id) === copy) {
      flying.delete(id);
      card.classList.remove(LANDING);
    }
  };
  if (moving.length === 0) landed();
  else void Promise.allSettled(moving.map((one) => one.finished)).then(landed);
}

/**
 * Slides the cards under `root` that a drawing moved, where the cards
 * stand as `arrangement` says now; nothing where the drawing is `quiet`.
 * Asked as the cards are drawn, every time.
 */
export function useCardSlide(root: { current: HTMLElement | null }, arrangement: string, quiet: boolean, byHand?: { current: string | null }): void {
  const last = useRef<string | null>(null);
  const before = useRef<Map<string, Box> | null>(null);
  // A card let go by hand is where the pointer left it: the drawing that puts it there does not slide it.
  const left = useRef<string | null>(null);
  // What is on the page now is what was drawn last: its boxes are noted before this drawing reaches it.
  if (last.current !== null && arrangement !== last.current && root.current && !quiet && !stillAsked()) {
    before.current = seen(root.current);
    left.current = byHand?.current ?? null;
    if (byHand) byHand.current = null;
  }
  last.current = arrangement;

  useLayoutEffect(() => {
    const was = before.current;
    before.current = null;
    const box = root.current;
    if (!was || !box) return;
    const cards = cardsOf(box);
    // Where each is now with nothing moving it: what slid or flew before is over.
    for (const [id, one] of cards) {
      one.classList.remove(SLIDING, LANDING);
      flying.get(id)?.remove();
      flying.delete(id);
    }
    const now = new Map([...cards].map(([id, one]) => [id, boxOf(one)]));
    const view = boxOf(box);
    const inSight: Box = { left: Math.max(0, view.left), top: Math.max(0, view.top), width: Math.min(innerWidth, view.left + view.width) - Math.max(0, view.left), height: Math.min(innerHeight, view.top + view.height) - Math.max(0, view.top) };
    for (const { id, x, y } of moved(was, now, inSight, pointer)) {
      if (id === left.current) continue;
      const card = cards.get(id)!;
      const list = card.parentElement;
      const from = was.get(id)!;
      if (list && leaves(from, boxOf(list))) fly(id, card, x, y);
      else slide(card, x, y);
    }
  });

  // A card that has slid is where it always is, with nothing on it.
  useLayoutEffect(() => {
    const box = root.current;
    if (!box) return;
    const over = (event: AnimationEvent) => {
      if (event.animationName === 'card-slid') (event.target as HTMLElement).classList.remove(SLIDING);
    };
    box.addEventListener('animationend', over);
    box.addEventListener('animationcancel', over);
    return () => {
      box.removeEventListener('animationend', over);
      box.removeEventListener('animationcancel', over);
    };
  }, [root.current]);
}

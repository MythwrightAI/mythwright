// What is drawn wider than the column it is in (a table, a diagram)
// scrolls sideways in a box of its own, never the page, and the box says
// where it has more than shows: a shade at that edge (`data-more-left`,
// `data-more-right` on the frame round the box, drawn by `prosetables.css`).
// Measured as it is drawn, as it changes size and as it is scrolled: nothing
// is looked at on a timer.

import type { RefObject } from 'preact';
import { useLayoutEffect } from 'preact/hooks';
import { moreAt } from '../diagrams.ts';

/** Marks on `frame` where `scroller` has more than shows, for as long as they are drawn, again whenever `again` changes. */
export function useMoreShown(frame: RefObject<HTMLElement | null>, scroller: RefObject<HTMLElement | null>, again: unknown[] = []): void {
  useLayoutEffect(() => {
    const [marked, box] = [frame.current, scroller.current];
    if (!marked || !box) return;
    const edges = () => {
      const more = moreAt(box.scrollLeft, box.scrollWidth, box.clientWidth);
      marked.toggleAttribute('data-more-left', more.left);
      marked.toggleAttribute('data-more-right', more.right);
    };
    edges();
    const sized = new ResizeObserver(edges);
    sized.observe(box);
    if (box.firstElementChild) sized.observe(box.firstElementChild);
    box.addEventListener('scroll', edges, { passive: true });
    return () => {
      sized.disconnect();
      box.removeEventListener('scroll', edges);
    };
  }, again);
}

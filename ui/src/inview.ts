// What is out of sight costs nothing to lay out.
//
// (A channel lays out its newest fifty messages always, `NEAR` in
// `views/channel.tsx`: what is in sight stays where it is as what is above
// it comes to be laid out at its own height, and a burst of messages each
// standing by a guess at a channel's foot moved it away from its foot.)
//
// A channel of a few thousand messages, and the Tools panel's list of a
// board's cards beside it, were laid out whole at every width the panel's
// edge was dragged through. Now each message and each line of such a list
// is laid out by the browser only while it is near the screen
// (`content-visibility: auto`, in the style sheet), and stands for itself
// out of sight by a size kept for it (`contain-intrinsic-size`).
//
// The size it stands by is the one it last had when it was laid out,
// kept here by what it is (a message's id), so that a list drawn again (a
// tab shown again, a page that scrolls itself back to where it was) is as
// tall as it was where its things were seen, and a place in it by pixels
// is the same place: only what was never seen stands by a guess. And a
// list kept at its foot (a channel at its newest message) is put back
// there whenever what is in sight comes to be laid out at another height
// than it stood by, which is told by the browser and never looked for.

import { useLayoutEffect } from 'preact/hooks';

/** The height each thing last had when it was laid out, by what it is; a few thousand at most, the oldest let go first. */
const sizes = new Map<string, number>();
const MOST = 5000;

/** How tall a thing stands out of sight: as it last was, else `usual`. */
export function sizeOf(id: string, usual: number): number {
  return sizes.get(id) ?? usual;
}

/** What is put on a thing for it to stand by out of sight (`style`), kept as it last was. */
export function standsBy(id: string, usual: number): string {
  return `contain-intrinsic-size: auto ${Math.round(sizeOf(id, usual))}px`;
}

function keep(id: string, height: number): void {
  if (height <= 0) return;
  sizes.delete(id);
  sizes.set(id, height);
  if (sizes.size > MOST) sizes.delete(sizes.keys().next().value!);
}

/**
 * Keeps the size of each thing of `list` (each by its `data-id`) as it is
 * laid out, and calls `resized` when one in sight is laid out at another
 * height (a list kept at its foot puts itself back there). Only what is in
 * sight is watched: the browser says as each comes to be laid out and
 * stops being so (`contentvisibilityautostatechange`, heard on the list as
 * it goes down to the thing).
 */
export function useKeptSizes(list: { current: HTMLElement | null }, resized?: () => void, again: unknown[] = []): void {
  useLayoutEffect(() => {
    const box = list.current;
    if (!box) return;
    const watched = new ResizeObserver((entries) => {
      let moved = false;
      for (const entry of entries) {
        const id = (entry.target as HTMLElement).dataset.id;
        const height = entry.borderBoxSize[0]?.blockSize ?? entry.contentRect.height;
        if (id && sizes.get(id) !== height) {
          keep(id, height);
          moved = true;
        }
      }
      if (moved) resized?.();
    });
    const told = (event: Event) => {
      const one = event.target as HTMLElement;
      if (!one.dataset?.id || one.parentElement !== box) return;
      if ((event as Event & { skipped: boolean }).skipped) watched.unobserve(one);
      else watched.observe(one);
    };
    box.addEventListener('contentvisibilityautostatechange', told, true);
    return () => {
      box.removeEventListener('contentvisibilityautostatechange', told, true);
      watched.disconnect();
    };
  }, again);
}

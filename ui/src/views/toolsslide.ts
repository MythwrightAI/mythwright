// The Tools panel beside a channel comes in and goes out as the drawer does.
//
// The drawer lies over the workbench; the panel takes room beside the
// channel. So the channel is laid out once for each, never as the panel
// moves: opened, the room is taken at once (the channel is drawn once,
// narrower) and the panel slides into it from the channel's right edge,
// moved by `transform` alone; closed, the room is given back at once
// (the channel drawn once, wider) and a still copy of the panel
// (`still.ts`) slides out over the channel's right edge. While either
// moves, the channel cuts off what is past its edge (`drawer.css`), so
// that nothing scrolls sideways and nothing lies over a pane beside it.
// The duration and the easing are the drawer's, and where less movement
// is asked for nothing moves and no copy is put there.
//
// It comes in only where it comes to be beside a channel that is already
// on the page (the Tools button, a thread opened, a message written at
// length). A channel shown again with its panel, as its tab is shown
// again or the page is read again, has it there as it was left, at rest
// at once, as a drawer given back to its tab is. The panel says when it
// is at rest (`data-rest`, by `drawn` in `drawer.ts`), which is what a
// check waits for.
//
// The panel beside a document (`doctools.tsx`) comes and goes the same
// way, beside the document's text: it says which panel it is (`Slides`),
// and the channel's is what is meant where nothing is said.

import { useRef } from 'preact/hooks';

import { goesOut, stillOf } from '../still.ts';

/** What the copy of a panel that goes out is called, which the style sheet gives its box and its way out. */
export const TOOLS_LEAVING = 'context-leaving';
/** What a panel that comes in is called while it is there, which the style sheet slides in once. */
export const TOOLS_COMING = 'context-coming';

/** Which panel slides: what it is called while it is there and while it comes in, what its copy going out is called, and what it is a picture of. */
export interface Slides {
  panel: string;
  coming: string;
  leaving: string;
  of: string;
}

/** The Tools panel beside a channel. */
const TOOLS: Slides = { panel: 'context', coming: TOOLS_COMING, leaving: TOOLS_LEAVING, of: 'Tools' };

/** Puts a still copy of the panel beside `channel` where it stood and has it go out; nothing where there is none or nothing moves it. */
function leave(channel: HTMLElement | null, kind: Slides): void {
  const live = channel?.querySelector<HTMLElement>(`:scope > .${kind.panel}`);
  if (!channel || !live) return;
  // The copy is a new element, and would slide in again where it still said it comes in.
  live.classList.remove(kind.coming);
  const copy = stillOf(live, { called: kind.leaving, of: kind.of, whole: true });
  copy.style.setProperty('--was', `${live.offsetWidth}px`);
  channel.append(copy);
  goesOut(copy);
}

/**
 * Whether the panel beside a channel comes in now, and the channel's box,
 * for a channel drawn with the panel `beside` it or not. Asked as the
 * channel is drawn: where it was beside the channel and is not now, the
 * panel is still on the page, as it was last drawn, to be copied.
 */
export function useSlid(beside: boolean, kind: Slides = TOOLS): { box: { current: HTMLDivElement | null }; coming: boolean } {
  const box = useRef<HTMLDivElement>(null);
  // Whether it was beside the channel the last time it was drawn; nothing yet the first time.
  const was = useRef<boolean | null>(null);
  const coming = useRef(false);
  if (beside && was.current === false) {
    coming.current = true;
    // One going out as it is opened again is gone at once: it would stand over the channel beside the one that comes.
    for (const copy of box.current?.querySelectorAll(`:scope > .${kind.leaving}`) ?? []) copy.remove();
  }
  if (!beside) {
    if (was.current === true) leave(box.current, kind);
    coming.current = false;
  }
  was.current = beside;
  return { box, coming: coming.current };
}

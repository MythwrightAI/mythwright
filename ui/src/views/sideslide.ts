// The side panel comes in and goes out as the drawer and the Tools panel
// beside a channel do, whatever
// folds it away or brings it back: Ctrl and `\`, the button at its head
// and the one that stands in for it, a place of the rail pressed again,
// the page pressed beside it on a phone, or something in it chosen there.
//
// It takes a column of the page, as the Tools panel takes room beside a
// channel, so the workbench is laid out once for each, never as the panel
// moves: brought back, the column is taken at once (the workbench drawn
// once, narrower) and the panel slides into it from behind the rail,
// moved by `transform` alone, with what is past the rail's edge cut off
// as it goes (`clip-path`, which moves nothing and lays nothing out);
// folded away, the column is given back at once (the workbench drawn
// once, wider) and a still copy of the panel (`still.ts`) slides out
// over the room it gave back, behind the rail the same way. On a phone,
// where the rail and the panel lie over the page from its left and come
// and go together, both come in from the screen's edge and a copy of both
// goes out there, the page beside them dimmed and lit again in the same
// time. The duration and the easing are the drawer's, and where
// less movement is asked for nothing moves and no copy is put there
// (`drawer.css`).
//
// It comes in only where it was folded away and is brought back. Another
// place of the rail chosen while it is out is another panel in its place,
// simply there; and a page opened with it out has it there as it was left.
// It says when it is at rest (`data-rest`, by `drawn` in `drawer.ts`),
// which is what a check waits for.

import { useRef } from 'preact/hooks';

import type { SideView } from '../store.ts';
import { goesOut, stillOf } from '../still.ts';

/** What the copy of a side panel that goes out is called, which the style sheet gives its box and its way out. */
export const SIDE_LEAVING = 'side-leaving';
/** What stands in for the dimming of the page beside the panel on a phone as it goes, which the style sheet fades out. */
export const SCRIM_LEAVING = 'side-under-leaving';
/** What the frame is called while its side panel comes in, which the style sheet slides the panel in by once. */
export const SIDE_COMING = 'side-coming';

/** Puts a still copy of the side panel (and on a phone of the rail with it) where it stood and has it go out; nothing where there is none or nothing moves it. */
function leave(shell: HTMLElement | null, phone: boolean): void {
  const live = shell?.querySelector<HTMLElement>(':scope > .sidebar');
  if (!shell || !live) return;
  const rail = phone ? shell.querySelector<HTMLElement>(':scope > .rail') : null;
  const boxes = [live, ...(rail ? [rail] : [])].map((one) => one.getBoundingClientRect());
  const left = Math.min(...boxes.map((box) => box.left));
  const top = Math.min(...boxes.map((box) => box.top));
  const right = Math.max(...boxes.map((box) => box.right));
  const bottom = Math.max(...boxes.map((box) => box.bottom));
  const copy = stillOf(live, { called: SIDE_LEAVING, of: live.getAttribute('aria-label'), whole: true, more: rail ? [rail] : [], placed: { left, top } });
  Object.assign(copy.style, { left: `${left}px`, top: `${top}px`, width: `${right - left}px`, height: `${bottom - top}px` });
  shell.append(copy);
  goesOut(copy);
  // On a phone the page beside them was dimmed, and is lit again in the same time: what dimmed it is gone at once, as the
  // panel is, and a plain stand-in for it, nothing to press, fades out (`SCRIM_LEAVING`).
  if (shell.querySelector(':scope > .side-under')) {
    const dim = document.createElement('div');
    dim.className = SCRIM_LEAVING;
    dim.setAttribute('aria-hidden', 'true');
    dim.inert = true;
    shell.append(dim);
    goesOut(dim);
  }
}

/**
 * Whether the side panel comes in now, and the frame's box, for a frame
 * drawn with the panel showing `view` (none where it is folded away, and
 * nothing at all where no frame is drawn).
 * Asked as the frame is drawn: where it showed something and shows
 * nothing now, the panel is still on the page, as it was last drawn, to
 * be copied.
 */
export function useSideSlid(view: SideView | null | undefined, phone: boolean): { shell: { current: HTMLDivElement | null }; coming: boolean } {
  const shell = useRef<HTMLDivElement>(null);
  // What it showed the last time the frame was drawn: nothing yet the first time.
  const was = useRef<SideView | null | undefined>(undefined);
  const coming = useRef(false);
  if (view && was.current === null) {
    coming.current = true;
    // One going out as it is brought back is gone at once: it would stand over the page beside the one that comes.
    for (const copy of shell.current?.querySelectorAll(`:scope > .${SIDE_LEAVING}`) ?? []) copy.remove();
  } else if (view && was.current && view !== was.current) {
    // Another panel in its place is there at once, and does not come in.
    coming.current = false;
  }
  // No frame is drawn (nobody is signed in): the one drawn next has its panel there as it was left.
  if (view === undefined) {
    was.current = undefined;
    coming.current = false;
    return { shell, coming: false };
  }
  if (!view) {
    if (was.current) leave(shell.current, phone);
    coming.current = false;
  }
  was.current = view;
  return { shell, coming: coming.current };
}

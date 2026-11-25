// The drawer goes out the way it came.
//
// A drawer that is put away is off the page at once, with whatever it
// held; what glides out is a still copy of it, taken in the moment it is
// put away (`still.ts`, which says why it is a copy and why nothing on
// the page finds anything inside it). The style sheet moves the copy
// (`drawer.css`); where nothing moves it (less movement was asked for)
// it is never put there. It is called `drawer-leaving`, never what the
// drawer is called, with what it was a picture of said by its heading
// (`data-of`).

import { effect } from '@preact/signals';

import { goesOut, stillOf as still } from './still.ts';
import { drawer } from './store.ts';

/** What the copy is called, which the style sheet gives the drawer's box and its way out. */
export const LEAVING = 'drawer-leaving';

/** A still copy of a drawer as it stands: its box, and in a closed tree of its own a picture of what was in it. */
export function stillOf(live: HTMLElement): HTMLElement {
  return still(live, { called: LEAVING, of: live.querySelector('.drawer-head h2')?.textContent });
}

/** Puts a still copy of the drawer where it stands and has it go out; nothing where there is no drawer or nothing moves one. */
export function leave(): void {
  if (typeof document === 'undefined') return;
  const live = document.querySelector<HTMLElement>('.main > .drawer');
  if (!live?.parentElement) return;
  const copy = stillOf(live);
  live.parentElement.append(copy);
  goesOut(copy);
}

let open = false;
// Heard as the drawer is put away, before the page is drawn again without it: the drawer is still there to be copied.
effect(() => {
  const now = drawer.value !== null;
  if (open && !now) leave();
  open = now;
});

// A page laid out within the room left of an open drawer, and not under it.
//
// The drawer lies over the right of the workbench (`.main > .drawer`), and
// a blueprint's page has at its right what is pressed while one is open:
// Testing, and Apply, which somebody presses having opened Preview or what
// it is applied with. So while a drawer is open the page keeps
// as much of its right clear as the drawer lies over (`clearOfDrawer` in
// `drawer.ts`), and its head wraps as it does on a narrow screen. Where the
// drawer is, is read from its box as it is laid out and not as it is drawn
// (`offsetLeft`, which its sliding in does not move), so that the page is
// laid out once as it opens and once as it goes, and never as it moves;
// what is followed is the drawer coming and going (a `MutationObserver` on
// the workbench) and its width and the page's (a `ResizeObserver`), and
// the room is set on the page's own style, so nothing is drawn again for
// it. On a phone, where the drawer has the screen, nothing changes.

import { useLayoutEffect } from 'preact/hooks';
import { clearOfDrawer } from '../drawer.ts';

/** Keeps the page clear of an open drawer for as long as it is on the page. */
export function useBesideDrawer(page: { current: HTMLElement | null }): void {
  useLayoutEffect(() => {
    const shown = page.current;
    const main = shown?.closest<HTMLElement>('.main');
    if (!shown || !main) return;
    let drawer: HTMLElement | null = null;
    const place = () => {
      const now = main.querySelector<HTMLElement>(':scope > .drawer');
      if (now !== drawer) {
        if (drawer) sizes.unobserve(drawer);
        drawer = now;
        if (drawer) sizes.observe(drawer);
      }
      const at = shown.getBoundingClientRect();
      const under = drawer?.offsetParent as HTMLElement | null | undefined;
      const left = drawer && under ? under.getBoundingClientRect().left + under.clientLeft + drawer.offsetLeft : null;
      const clear = clearOfDrawer(at.left, at.right, left);
      const room = clear > 0 ? `${clear}px` : '';
      if (shown.style.paddingRight !== room) shown.style.paddingRight = room;
    };
    const sizes = new ResizeObserver(place);
    sizes.observe(shown);
    const comes = new MutationObserver(place);
    comes.observe(main, { childList: true });
    place();
    return () => {
      sizes.disconnect();
      comes.disconnect();
      shown.style.paddingRight = '';
    };
  }, []);
}

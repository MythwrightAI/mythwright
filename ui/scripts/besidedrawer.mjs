// A blueprint's page beside an open drawer (`src/views/besidedrawer.ts`), for
// the checks that hold it: the drawer lies over the right of the workbench,
// and the page is laid out within the room left of it, so that Testing and
// Apply, at the right of its head, are never under it and are pressed by
// the pointer with it open. Where the drawer has the screen (a
// phone) it lies over the page as it always did, and nothing is held.

/**
 * What of the head of the page that is showing reaches under the open
 * drawer, at rest, or cannot be pressed where it is drawn, each said in a
 * few words; and whether the page scrolls sideways. Nothing wrong where
 * all is well.
 */
export const besideDrawer = (page) =>
  page.evaluate(() => {
    const wrong = [];
    const drawer = document.querySelector('.main > .drawer[data-rest]');
    if (!drawer) return { wrong: ['no drawer open'], pressed: [] };
    const edge = drawer.getBoundingClientRect().left;
    const head = [...document.querySelectorAll('.view .view-head')].find((one) => one.getClientRects().length > 0);
    if (!head) return { wrong: ['no head showing'], pressed: [] };
    const seen = (el) => el.getClientRects().length > 0 && el.getBoundingClientRect().width > 0.5;
    // What is drawn of an element is as far as what it is inside of shows it: the end of a path cut short with an
    // ellipsis is past its box, and is not drawn there.
    const shown = (one) => {
      let right = one.getBoundingClientRect().right;
      for (let up = one.parentElement; up && up !== head.parentElement; up = up.parentElement)
        if (getComputedStyle(up).overflowX !== 'visible') right = Math.min(right, up.getBoundingClientRect().right);
      return right;
    };
    for (const one of head.querySelectorAll('*')) {
      if (!seen(one)) continue;
      const at = { right: shown(one) };
      if (at.right > edge + 0.5) wrong.push(`"${(one.textContent ?? '').trim().slice(0, 30) || one.className}" reaches to ${Math.round(at.right)}, under the drawer at ${Math.round(edge)}`);
    }
    const pressed = [];
    for (const one of [head.querySelector('button.head-testing'), [...head.querySelectorAll('button.primary')].find(seen)]) {
      if (!one || !seen(one)) {
        wrong.push(`${one ? one.textContent.trim() : 'a button'} is not in the head`);
        continue;
      }
      const at = one.getBoundingClientRect();
      const under = document.elementFromPoint((at.left + at.right) / 2, (at.top + at.bottom) / 2);
      if (!one.contains(under)) wrong.push(`"${one.textContent.trim()}" is not to be pressed where it is: ${under?.className ?? 'nothing'} is there`);
      else pressed.push(one.textContent.trim());
    }
    const sideways = document.documentElement.scrollWidth - window.innerWidth;
    if (sideways > 0) wrong.push(`the page scrolls sideways by ${sideways}`);
    return { wrong, pressed };
  });

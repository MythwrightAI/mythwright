// What a check holds of a whole page by the boxes: of everything on it
// that is pressed, typed in or read as a heading or a name, none lies over
// another that is not inside it or round it, none is past the window's
// sides, none has its words drawn past its own box where its box cuts them
// off nowhere, and nothing scrolls the page sideways. It answers what is
// wrong, in a sentence a thing, and nothing where all is well; the check
// says what it was looking at.
//
// It is how a theme's look is held (`themed.mjs`): the largest text and
// things standing closest together are what most moves where everything
// stands, on every page.

/** What is looked at: what is pressed, typed in, chosen, or reads as a heading or a name. */
const LOOKED_AT = 'button, input:not([type="hidden"]), select, textarea, a[href], h1, h2, h3, [role="tab"], [role="radio"], .nav-item, .badge, .tag';

/** What `within` (a selector, the whole page where nothing is said) has wrong, by the boxes. */
export const pageCrowded = (page, within = 'body') =>
  page.evaluate(
    ([within, looked]) => {
      const said = [];
      const root = document.querySelector(within);
      if (!root) return [`there is no ${within}`];
      const seen = (one) => {
        const box = one.getBoundingClientRect();
        if (box.width < 1 || box.height < 1) return false;
        const style = getComputedStyle(one);
        if (style.visibility === 'hidden' || Number(style.opacity) === 0) return false;
        // Only what is in the window: what is scrolled out of sight is looked at where it is scrolled to.
        return box.bottom > 0 && box.top < window.innerHeight && box.right > 0 && box.left < window.innerWidth;
      };
      // What scrolls cuts off what is past its edge, so a thing is only where its scroller shows it.
      const shown = (one) => {
        let box = one.getBoundingClientRect();
        for (let up = one.parentElement; up && box; up = up.parentElement) {
          const style = getComputedStyle(up);
          if (style.overflowX === 'visible' && style.overflowY === 'visible') continue;
          const frame = up.getBoundingClientRect();
          const left = Math.max(box.left, frame.left);
          const top = Math.max(box.top, frame.top);
          const right = Math.min(box.right, frame.right);
          const bottom = Math.min(box.bottom, frame.bottom);
          box = right - left > 0.5 && bottom - top > 0.5 ? new DOMRect(left, top, right - left, bottom - top) : null;
        }
        return box;
      };
      // How far the words of `one` reach past its sides, where nothing between them and it cuts them off.
      const wordsPast = (one, box) => {
        if (['INPUT', 'SELECT', 'TEXTAREA'].includes(one.tagName)) return 0;
        const walk = document.createTreeWalker(one, NodeFilter.SHOW_TEXT);
        let most = 0;
        for (let text = walk.nextNode(); text; text = walk.nextNode()) {
          if (!text.textContent.trim()) continue;
          let cut = false;
          for (let up = text.parentElement; up && up !== one.parentElement; up = up.parentElement) {
            const style = getComputedStyle(up);
            if ((up !== one && ['absolute', 'fixed'].includes(style.position)) || style.overflowX !== 'visible') cut = true;
          }
          if (cut) continue;
          const range = document.createRange();
          range.selectNodeContents(text);
          const words = range.getBoundingClientRect();
          // Words that are drawn nowhere (inside something not shown, as the keys a search answers to are under a
          // finger) have no box, which is at the window's corner and so would read as words far past it.
          if (words.width === 0 && words.height === 0) continue;
          most = Math.max(most, box.left - words.left, words.right - box.right);
        }
        return most;
      };
      const name = (one) => (one.getAttribute('aria-label') ?? one.textContent ?? one.tagName).trim().replace(/\s+/g, ' ').slice(0, 32) || one.tagName.toLowerCase();
      const things = [...root.querySelectorAll(looked)].filter(seen).map((one) => ({ one, box: shown(one) })).filter((it) => it.box);
      // What lies over the page on purpose (a menu, a card, a dialog, the drawer) is over what is under it: only
      // what is on the same layer is held apart.
      const layer = (one) => one.closest('.modal, .row-menu, .who-card, .drawer, .headmenu, .iconpick, [role="dialog"], [role="menu"], [role="tooltip"], .cm-tooltip, .cm-panels') ?? null;
      // What stays where it is while what is beside it scrolls (a dialog's foot, a table's heads) is drawn over
      // what is scrolled under it, on purpose, and that is reached by scrolling.
      const staying = (one) => {
        for (let up = one; up; up = up.parentElement) if (getComputedStyle(up).position === 'sticky') return up;
        return null;
      };
      for (const { one, box } of things) {
        if (box.left < -0.5 || box.right > window.innerWidth + 0.5) said.push(`"${name(one)}" is past the window's side`);
        // Words that run past a box that cuts nothing off are drawn over whatever is beside it. What a part lays
        // over its own corner on purpose (a dot, a count) is laid there, and is no words of its.
        const past = wordsPast(one, one.getBoundingClientRect());
        if (past > 1) said.push(`the words of "${name(one)}" run ${Math.round(past)}px past it`);
      }
      // What a box one types in keeps room for at its right (the keys a panel's search box says) is laid in that
      // room on purpose: wholly inside the box and past where what is typed runs to, it lies over nothing typed.
      const keptRoom = (field, other) => {
        if (!['INPUT', 'TEXTAREA'].includes(field.one.tagName)) return false;
        const to = field.box.right - parseFloat(getComputedStyle(field.one).paddingRight);
        return other.box.left >= to - 0.5 && other.box.right <= field.box.right + 0.5 && other.box.top >= field.box.top - 0.5 && other.box.bottom <= field.box.bottom + 0.5;
      };
      for (let at = 0; at < things.length; at += 1) {
        for (let next = at + 1; next < things.length; next += 1) {
          const [a, b] = [things[at], things[next]];
          if (a.one.contains(b.one) || b.one.contains(a.one) || layer(a.one) !== layer(b.one) || staying(a.one) !== staying(b.one)) continue;
          if (keptRoom(a, b) || keptRoom(b, a)) continue;
          const across = Math.min(a.box.right, b.box.right) - Math.max(a.box.left, b.box.left);
          const down = Math.min(a.box.bottom, b.box.bottom) - Math.max(a.box.top, b.box.top);
          if (across > 0.5 && down > 0.5) said.push(`"${name(a.one)}" lies over "${name(b.one)}"`);
        }
      }
      const wide = document.documentElement.scrollWidth - window.innerWidth;
      if (wide > 0) said.push(`the page is ${wide}px wider than the window`);
      return said;
    },
    [within, LOOKED_AT],
  );

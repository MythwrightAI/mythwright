// What a control says of itself when the pointer rests on it.
//
// A view writes `title` and nothing more. The system's own tooltip for it
// sits under the pointer and, in the desktop app, is cut off where the
// window ends: a button at the right of a bar has half its sentence missing.
// So the client draws the sentence itself, under the control and wholly
// inside the window; or, for a line of a list that says so, beside the
// list (`placeBeside`). While it is doing that the `title` is taken off the
// element, or the system would draw its own as well; it is put back when
// the pointer leaves, so what reads the page aloud finds it as written.
//
// A tip that says only the start of what it is about says, in a last faint
// line, where the rest is (`data-tip-more`): a line of the inbox that a
// blueprint told may be thousands of letters, and said whole its tip was
// taller than half the window.
//
// A control that is to say nothing as it is pointed at says so
// (`data-untipped`): it has no `title`, and nothing around it is said in
// its place. The ⋯ at the end of a row of a side panel is one: its tip lay
// over the panel, and what it is called is for whoever hears it read out.

export interface Box {
  left: number;
  top: number;
  width: number;
  height: number;
}

export interface Size {
  width: number;
  height: number;
}

/** How far the sentence stands from what it is about, and from the window's edges. */
const GAP = 6;
const EDGE = 8;
/** How long the pointer rests before anything is said, as the system's own waits. */
const REST = 400;

/**
 * Where a tip of this size goes for what is at `over`, in a window of this
 * size: under it and centred on it, moved in from an edge it would cross,
 * and above it where there is no room below but there is above.
 */
export function place(over: Box, tip: Size, within: Size): { left: number; top: number } {
  const centred = over.left + over.width / 2 - tip.width / 2;
  const left = Math.max(EDGE, Math.min(centred, within.width - EDGE - tip.width));
  const below = over.top + over.height + GAP;
  const above = over.top - GAP - tip.height;
  const top = below + tip.height > within.height - EDGE && above >= EDGE ? above : below;
  return { left, top };
}

/**
 * Where a tip of this size goes for `line`, a line of a list that says
 * its tips beside itself (`data-tip-beside`, whose box is `list`): beside
 * the list on its right, level with the line, where the window has room;
 * else on its left; else under or over the line, as any tip. Moved up to
 * be whole in the window. Put under the line, a tip about a line of the
 * side panel was as wide as more than the panel, and lay over the rail at
 * the window's very edge.
 */
export function placeBeside(list: Box, line: Box, tip: Size, within: Size): { left: number; top: number } {
  const top = Math.max(EDGE, Math.min(line.top, within.height - EDGE - tip.height));
  const right = list.left + list.width + GAP;
  if (right + tip.width <= within.width - EDGE) return { left: right, top };
  const left = list.left - GAP - tip.width;
  if (left >= EDGE) return { left, top };
  return place(line, tip, within);
}

/** Says every `title` on the page in the client's own tooltip, from now on. */
export function sayTitles(): void {
  let held: { on: Element; says: string; watch: MutationObserver } | null = null;
  let tip: HTMLElement | null = null;
  let resting: ReturnType<typeof setTimeout> | undefined;

  /** The tip's words, and under them where the rest is, where the control says so. */
  const fill = (on: Element, says: string) => {
    if (!tip) return;
    tip.textContent = says;
    const more = on.getAttribute('data-tip-more');
    if (!more) return;
    const line = document.createElement('span');
    line.className = 'tip-more';
    line.textContent = more;
    tip.append(line);
  };
  const hide = () => {
    clearTimeout(resting);
    tip?.remove();
    tip = null;
  };
  const show = () => {
    if (!held?.on.isConnected) return;
    tip = document.createElement('div');
    tip.className = 'tip-said';
    tip.setAttribute('role', 'tooltip');
    fill(held.on, held.says);
    document.body.append(tip);
    const within = { width: document.documentElement.clientWidth, height: document.documentElement.clientHeight };
    const beside = held.on.closest('[data-tip-beside]');
    const [over, size] = [held.on.getBoundingClientRect(), tip.getBoundingClientRect()];
    const at = beside ? placeBeside(beside.getBoundingClientRect(), over, size, within) : place(over, size, within);
    tip.style.left = `${at.left}px`;
    tip.style.top = `${at.top}px`;
  };
  const letGo = () => {
    hide();
    if (!held) return;
    held.watch.disconnect();
    if (!held.on.hasAttribute('title')) held.on.setAttribute('title', held.says);
    held = null;
  };
  const take = (on: Element) => {
    const says = on.getAttribute('title') ?? '';
    on.removeAttribute('title');
    // The view may say something else while the pointer is still here (a
    // button that changes what it does when pressed): that is taken too.
    const watch = new MutationObserver(() => {
      const now = on.getAttribute('title');
      if (now === null || !held) return;
      on.removeAttribute('title');
      held.says = now;
      fill(on, now);
    });
    watch.observe(on, { attributes: true, attributeFilter: ['title'] });
    held = { on, says, watch };
  };

  document.addEventListener('pointerover', (event) => {
    // A finger has nothing to rest: what it touches, it presses.
    if (event.pointerType !== 'mouse' || !(event.target instanceof Element)) return;
    if (held?.on.contains(event.target)) return;
    letGo();
    // The nearest that says something, unless something nearer says it is to say nothing.
    const on = event.target.closest('[title], [data-untipped]');
    if (!on || on.hasAttribute('data-untipped') || !on.getAttribute('title')) return;
    take(on);
    resting = setTimeout(show, REST);
  });
  // Pressed, typed at or scrolled away, it has been read or is not wanted.
  // The title stays held until the pointer leaves, so that the system does
  // not start saying it instead.
  document.addEventListener('pointerdown', hide, true);
  document.addEventListener('keydown', hide, true);
  document.addEventListener('scroll', hide, true);
  document.documentElement.addEventListener('pointerleave', letGo);
  window.addEventListener('blur', letGo);
}

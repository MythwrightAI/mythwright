// What floats beside something being written (who can be named, what the
// factory's bot takes, the composer's menus, who can be named on a record),
// drawn on the page's body and laid over everything there, where
// `offeredAt` in `mentions.ts` says. Drawn inside what it is offered for it
// was cut off by whatever that was in: a thread in the tools panel beside a
// channel hid the top of its list under the panel's tabs, and
// its menus the same way. It is placed before it is drawn where anybody
// sees it, and again on every drawing, as the window changes and as
// anything scrolls; it moves nothing, being on no part of the page. The
// keys are those of what it is offered for: nothing here takes the
// keyboard.

import type { ComponentChildren, RefObject } from 'preact';
import { createPortal } from 'preact/compat';
import { useLayoutEffect, useRef } from 'preact/hooks';
import { offeredAt } from '../mentions.ts';
import { classes } from '../util.ts';

/** How wide a list may be beside what it is offered for: at least this, at most that, and that one's width between. */
const NARROWEST = 260;
const WIDEST = 420;
/** Between what floats and what it stands over or under. */
const GAP = 4;
/** Where a message is written at length the list is in from the box's left. */
const IN = 12;

/**
 * Where it is put beside: how far left it begins, how far down its foot may come over what it is offered for and how
 * far up its head may come under it, which of the two it tries first, and, for a list as wide as what it is offered
 * for, how wide that is (a menu says nothing, and is as wide as its style says).
 */
export interface Beside {
  left: number;
  above: number;
  below: number;
  first: 'over' | 'under';
  across?: number;
}

/** Where a box of something being written can be put beside, from its box on the screen. */
export const besideBox = (box: DOMRect, first: 'over' | 'under'): Beside => ({ left: box.left, above: box.top - GAP, below: box.bottom + GAP, first, across: box.width });

/**
 * Where a menu a composer's tool opens is put: over the tools, since they are at the foot of what is written, and
 * under them where a message is written at length and they are at the panel's top. As wide as its style says.
 */
export function besideTool(tool: Element | null): Beside | null {
  if (!tool) return null;
  // Over or under all the tools, which go on to a second line where the box is narrow, and never over one of them.
  const tools = (tool.closest('.composer-foot') ?? tool).getBoundingClientRect();
  return { left: tool.getBoundingClientRect().left, above: tools.top - GAP, below: tools.bottom + GAP, first: tool.closest('.composer.wide') ? 'under' : 'over' };
}

/**
 * The line the caret is on in a box being written in, on the screen: worked out by writing what is before the caret
 * into a copy of the box laid out as it is, which wraps its lines where the box does.
 */
export function caretLine(area: HTMLTextAreaElement): { top: number; bottom: number } {
  const style = getComputedStyle(area);
  const copy = document.createElement('div');
  for (const name of ['boxSizing', 'width', 'paddingTop', 'paddingRight', 'paddingBottom', 'paddingLeft', 'borderTopWidth', 'borderRightWidth', 'borderBottomWidth', 'borderLeftWidth', 'fontFamily', 'fontSize', 'fontWeight', 'fontStyle', 'letterSpacing', 'lineHeight', 'tabSize', 'textIndent', 'wordSpacing'] as const) copy.style[name] = style[name];
  Object.assign(copy.style, { position: 'fixed', left: '0', top: '0', visibility: 'hidden', whiteSpace: 'pre-wrap', overflowWrap: 'break-word', borderStyle: 'solid', overflow: 'hidden', height: 'auto' });
  copy.textContent = area.value.slice(0, area.selectionStart);
  const mark = copy.appendChild(document.createElement('span'));
  mark.textContent = '​';
  document.body.append(copy);
  const top = mark.offsetTop;
  const tall = mark.offsetHeight;
  copy.remove();
  const box = area.getBoundingClientRect();
  return { top: box.top + top - area.scrollTop, bottom: box.top + top - area.scrollTop + tall };
}

interface FloatingProps {
  /** Where it is put beside, asked each time it is placed; nothing where what it is beside is not there. */
  beside: () => Beside | null;
  /** Its own box, for whoever has to tell a press inside it from one outside. */
  own?: RefObject<HTMLDivElement | null>;
  class: string;
  role: 'listbox' | 'menu' | 'tooltip';
  label: string;
  children: ComponentChildren;
}

export function Floating({ beside, own, class: named, role, label, children }: FloatingProps) {
  const mine = useRef<HTMLDivElement>(null);
  const held = own ?? mine;
  useLayoutEffect(() => {
    const place = () => {
      const shown = held.current;
      const at = beside();
      if (!shown || !at) return;
      const window = { width: document.documentElement.clientWidth, height: document.documentElement.clientHeight };
      // What is fixed at the window's top (the desktop window's own bar) is not to be lain under.
      const bar = parseFloat(getComputedStyle(document.documentElement).getPropertyValue('--bar')) || 0;
      // Measured with nothing held, as wide as it may be and where it has the room for that.
      const widths = at.across === undefined ? null : Math.min(WIDEST, Math.max(NARROWEST, at.across), window.width - 16);
      Object.assign(shown.style, { left: '8px', top: '0px', maxHeight: '', ...(widths === null ? {} : { minWidth: `${Math.min(NARROWEST, widths)}px`, maxWidth: `${widths}px` }) });
      const tall = Math.min(shown.scrollHeight + shown.offsetHeight - shown.clientHeight, parseFloat(getComputedStyle(shown).maxHeight) || Infinity);
      const put = offeredAt(at, { width: shown.offsetWidth, height: tall }, { ...window, top: bar });
      Object.assign(shown.style, { left: `${put.left}px`, top: `${put.top}px`, maxHeight: put.most === null ? '' : `${put.most}px`, visibility: '' });
    };
    place();
    window.addEventListener('resize', place);
    window.addEventListener('scroll', place, true);
    return () => {
      window.removeEventListener('resize', place);
      window.removeEventListener('scroll', place, true);
    };
  });
  return createPortal(
    <div ref={held} class={classes('floated', named)} role={role} aria-label={label} style="visibility:hidden">
      {children}
    </div>,
    document.body,
  );
}

interface OfferedProps {
  /** The composer's box, the part over it that takes no room, and the box it is written in. */
  box: RefObject<HTMLElement | null>;
  over: RefObject<HTMLElement | null>;
  area: RefObject<HTMLTextAreaElement | null>;
  /** Written at length in the panel: under the line the caret is on first. */
  wide?: boolean;
  class: string;
  label: string;
  children: ComponentChildren;
}

/** What the composer offers as it is written: over the box and over whatever stands there already; written at length, beside the caret's line. */
export function Offered({ box, over, area, wide, class: named, label, children }: OfferedProps) {
  const beside = (): Beside | null => {
    const around = box.current?.getBoundingClientRect();
    if (!around) return null;
    if (wide && area.current) {
      // The box is the panel's whole height, and what is written in it is under its tools: over none of the line
      // the caret is on, so that what is being typed is seen.
      const line = caretLine(area.current);
      return { left: around.left + IN, above: line.top - GAP, below: line.bottom + GAP, first: 'under', across: around.width - 2 * IN };
    }
    // Over the box it stands over whatever stands there already (what waits to go, the line of who is writing), as it did in among them.
    const standing = over.current?.getBoundingClientRect();
    const above = standing ? standing.top - (standing.height > 0 ? GAP : 0) : around.top - GAP;
    return { left: around.left, above, below: around.bottom + GAP, first: 'over', across: around.width };
  };
  return (
    <Floating beside={beside} class={classes('mentions', 'offered', named)} role="listbox" label={label}>
      {children}
    </Floating>
  );
}

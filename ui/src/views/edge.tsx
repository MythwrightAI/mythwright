// The left edge of a panel at the right of a channel, which is dragged to
// make the panel wider or narrower.
//
// It follows the pointer wherever it goes until it is let go, and what it
// was left at is kept by whoever the panel is. The arrows move it too;
// pressed twice it is as wide as it started. However it is moved, the
// panel is never narrower than it can be read at, and the channel beside
// it, with whatever other panel is open, keeps its share.
//
// While it is held the width goes straight onto the layout, once a frame
// (`held.ts`): whoever keeps the width is told when the edge is let go,
// and the channel beside the panel is not drawn again on the way.

import { useRef } from 'preact/hooks';
import { fitWidth } from '../context.ts';
import { hold, type Holding, holding, keepPlace } from '../held.ts';

/** The property of the channel's grid that says how wide the panel is asked to be. */
export const PANEL_WIDTH = '--context-width';

export function Edge({ what, usual, onWidth }: { what: string; usual: number; onWidth: (width: number, done: boolean) => void }) {
  const held = useRef<{ holding: Holding; from: number } | null>(null);
  // How wide the panel is drawn, which is less than it was asked to be in a window with no room for that.
  const across = (edge: HTMLElement) => Math.round(edge.parentElement?.getBoundingClientRect().width ?? usual);
  // What there is across for this panel and the channel: the whole of it, less any other panel that is open.
  const room = (edge: HTMLElement) => {
    const own = edge.parentElement;
    const all = own?.parentElement;
    if (!own || !all) return window.innerWidth;
    const others = [...all.children].filter((one) => one !== own && one.tagName === 'ASIDE');
    return all.getBoundingClientRect().width - others.reduce((sum, one) => sum + one.getBoundingClientRect().width, 0);
  };
  const done = (edge: HTMLElement) => {
    held.current = null;
    edge.classList.remove('held');
    holding(false);
  };
  return (
    <div
      class="panel-edge"
      role="separator"
      aria-orientation="vertical"
      aria-label={`Make the ${what} wider or narrower`}
      tabIndex={0}
      onPointerDown={(event) => {
        if (event.button !== 0) return;
        event.preventDefault();
        const edge = event.currentTarget;
        const grid = edge.parentElement?.parentElement;
        // What it may be is worked out once, as it is taken hold of: the room does not change while it is held.
        const [from, wide] = [across(edge), room(edge)];
        held.current = {
          from,
          holding: hold({ width: from, x: event.clientX, towards: -1, fit: (asked) => fitWidth(asked, wide), draw: (width) => {
            grid?.style.setProperty(PANEL_WIDTH, `${width}px`);
            keepPlace();
          } }),
        };
        edge.setPointerCapture(event.pointerId);
        edge.classList.add('held');
        holding(true);
      }}
      onPointerMove={(event) => held.current?.holding.move(event.clientX)}
      onPointerUp={(event) => {
        if (!held.current) return;
        held.current.holding.letGo();
        done(event.currentTarget);
        onWidth(across(event.currentTarget), true);
      }}
      onPointerCancel={(event) => {
        if (!held.current) return;
        const was = held.current.holding.callOff();
        done(event.currentTarget);
        onWidth(was, true);
      }}
      onDblClick={(event) => onWidth(fitWidth(usual, room(event.currentTarget)), true)}
      onKeyDown={(event) => {
        const step = event.key === 'ArrowLeft' ? 24 : event.key === 'ArrowRight' ? -24 : 0;
        if (!step) return;
        event.preventDefault();
        onWidth(fitWidth(across(event.currentTarget) + step, room(event.currentTarget)), true);
      }}
    />
  );
}

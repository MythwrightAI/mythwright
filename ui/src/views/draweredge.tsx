// The drawer's left edge, which is dragged to make the drawer wider or
// narrower, whatever is in it. It follows the pointer until it is let go,
// and what it was left at is kept. The arrows move it too; pressed twice
// it is as wide as it started.
//
// While it is held the width goes straight onto the layout, once a frame
// (`held.ts`), and is kept when the edge is let go: what is in the drawer
// is not drawn again on the way, and neither is the page it lies over.

import { useRef } from 'preact/hooks';
import { drawerWidth, fitDrawer, setDrawerWidth, USUAL_DRAWER } from '../drawer.ts';
import { hold, type Holding, holding } from '../held.ts';

/** The property of the workbench that says how wide the drawer is asked to be. */
export const DRAWER_WIDTH = '--drawer-width';

export function DrawerEdge() {
  const held = useRef<Holding | null>(null);
  // The workbench the drawer lies over, which is what it is never wider than.
  const over = (edge: HTMLElement) => edge.parentElement?.parentElement ?? null;
  const room = (edge: HTMLElement) => over(edge)?.getBoundingClientRect().width ?? window.innerWidth;
  // How wide it is drawn, which is less than it was asked to be in a window with no room for that.
  const across = (edge: HTMLElement) => Math.round(edge.parentElement?.getBoundingClientRect().width ?? drawerWidth.value);
  const done = (edge: HTMLElement) => {
    held.current = null;
    edge.classList.remove('held');
    holding(false);
  };
  return (
    <div
      class="drawer-edge"
      role="separator"
      aria-orientation="vertical"
      aria-label="Make the drawer wider or narrower"
      tabIndex={0}
      onPointerDown={(event) => {
        if (event.button !== 0) return;
        event.preventDefault();
        const edge = event.currentTarget;
        const main = over(edge);
        // What it may be is worked out once, as it is taken hold of: the window is as wide as it is while the edge is held.
        const wide = room(edge);
        held.current = hold({ width: across(edge), x: event.clientX, towards: -1, fit: (asked) => fitDrawer(asked, wide), draw: (width) => main?.style.setProperty(DRAWER_WIDTH, `${width}px`) });
        edge.setPointerCapture(event.pointerId);
        edge.classList.add('held');
        holding(true);
      }}
      onPointerMove={(event) => held.current?.move(event.clientX)}
      onPointerUp={(event) => {
        if (!held.current) return;
        const width = held.current.letGo();
        done(event.currentTarget);
        setDrawerWidth(width);
      }}
      onPointerCancel={(event) => {
        if (!held.current) return;
        const was = held.current.callOff();
        done(event.currentTarget);
        setDrawerWidth(was);
      }}
      onDblClick={(event) => setDrawerWidth(fitDrawer(USUAL_DRAWER, room(event.currentTarget)))}
      onKeyDown={(event) => {
        const step = event.key === 'ArrowLeft' ? 24 : event.key === 'ArrowRight' ? -24 : 0;
        if (!step) return;
        event.preventDefault();
        setDrawerWidth(fitDrawer(across(event.currentTarget) + step, room(event.currentTarget)));
      }}
    />
  );
}

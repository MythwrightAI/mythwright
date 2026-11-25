// The side panel's right edge, which is dragged to make the panel wider
// or narrower. It follows the pointer wherever it goes until it is let go,
// and what it was left at is kept. The arrows move it too; pressed twice
// it is as wide as it started.
//
// While it is held the width goes straight onto the layout, once a frame
// (`held.ts`), and is kept when the edge is let go: the page beside the
// panel is not drawn again on the way.

import { useRef } from 'preact/hooks';
import { hold, type Holding, holding, keepPlace } from '../held.ts';
import { fitSide, setSideWidth, sideWidth, USUAL_SIDE } from '../side.ts';

export function SideEdge() {
  const held = useRef<Holding | null>(null);
  const fit = (asked: number) => fitSide(asked, window.innerWidth);
  const done = (edge: HTMLElement) => {
    held.current = null;
    edge.classList.remove('held');
    holding(false);
  };
  return (
    <div
      class="side-edge"
      role="separator"
      aria-orientation="vertical"
      aria-label="Make the side panel wider or narrower"
      tabIndex={0}
      onPointerDown={(event) => {
        if (event.button !== 0) return;
        event.preventDefault();
        const edge = event.currentTarget;
        const shell = edge.parentElement;
        // The window is as wide as it is for as long as the edge is held.
        const wide = window.innerWidth;
        held.current = hold({ width: sideWidth.value, x: event.clientX, towards: 1, fit: (asked) => fitSide(asked, wide), draw: (width) => {
          shell?.style.setProperty('--side-width', `${width}px`);
          keepPlace();
        } });
        edge.setPointerCapture(event.pointerId);
        edge.classList.add('held');
        holding(true);
      }}
      onPointerMove={(event) => held.current?.move(event.clientX)}
      onPointerUp={(event) => {
        if (!held.current) return;
        const width = held.current.letGo();
        done(event.currentTarget);
        setSideWidth(width, true);
      }}
      onPointerCancel={(event) => {
        if (!held.current) return;
        const was = held.current.callOff();
        done(event.currentTarget);
        setSideWidth(was, true);
      }}
      onDblClick={() => setSideWidth(fit(USUAL_SIDE), true)}
      onKeyDown={(event) => {
        const step = event.key === 'ArrowRight' ? 24 : event.key === 'ArrowLeft' ? -24 : 0;
        if (!step) return;
        event.preventDefault();
        setSideWidth(fit(sideWidth.value + step), true);
      }}
    />
  );
}

// Full screen, for what is looked at closely: a view (an attached page) and
// a diagram. A box of the client's own, round what is drawn, is put into
// the browser's full screen (the Fullscreen API), never what is inside it:
// a view's page cannot ask for full screen, and its sandbox is no wider for
// this. The box is the same element in full screen as out of it, so what
// is in it is not made again: a view's page keeps running, as it was.
//
// In full screen a small Exit full screen stands in its top right corner,
// and Escape leaves too (the browser's own way, and ours where a key
// reaches the page first, before a dialog it is in takes Escape to close).
// Where the browser offers no full screen (`document.fullscreenEnabled`),
// the button is not drawn.

import type { ComponentChildren, RefObject } from 'preact';
import { useEffect, useRef, useState } from 'preact/hooks';
import { classes } from '../util.ts';

/** A box that can be put into full screen, whether it is, and the ways in and out. */
export interface FullScreen {
  box: RefObject<HTMLDivElement | null>;
  on: boolean;
  enter: () => void;
  leave: () => void;
}

const leaving = () => {
  if (document.fullscreenElement) document.exitFullscreen().catch(() => {});
};

/** A box that can be put into full screen: hand `box` to a `FullBox`, and `enter` to a `FullButton`. */
export function useFullScreen(): FullScreen {
  const box = useRef<HTMLDivElement>(null);
  const [on, setOn] = useState(false);
  useEffect(() => {
    const told = () => {
      const now = !!box.current && document.fullscreenElement === box.current;
      setOn(now);
      // What has the keyboard is otherwise left behind, outside what shows.
      if (now) box.current?.querySelector<HTMLElement>('.full-exit')?.focus();
    };
    document.addEventListener('fullscreenchange', told);
    return () => document.removeEventListener('fullscreenchange', told);
  }, []);
  useEffect(() => {
    if (!on) return;
    // Caught on its way down, so that a dialog the box is in is not closed by the same Escape.
    const key = (event: KeyboardEvent) => {
      if (event.key !== 'Escape' || document.fullscreenElement !== box.current) return;
      event.stopPropagation();
      event.preventDefault();
      leaving();
    };
    window.addEventListener('keydown', key, true);
    return () => window.removeEventListener('keydown', key, true);
  }, [on]);
  return {
    box,
    on,
    enter: () => {
      box.current?.requestFullscreen().catch(() => {});
    },
    leave: leaving,
  };
}

/** The box that goes into full screen, round what it holds; in full screen, the way out over its corner. */
export function FullBox({ full, class: name, children }: { full: FullScreen; class?: string; children: ComponentChildren }) {
  return (
    <div ref={full.box} class={classes('full-box', name)} data-full={full.on ? '' : undefined}>
      {children}
      {full.on && (
        <button type="button" class="full-exit" onClick={full.leave}>
          Exit full screen
        </button>
      )}
    </div>
  );
}

/** Full screen, where the browser offers it. */
export function FullButton({ full, class: name }: { full: FullScreen; class: string }) {
  if (!document.fullscreenEnabled) return null;
  return (
    <button type="button" class={classes('full-enter', name)} onClick={full.enter}>
      Full screen
    </button>
  );
}

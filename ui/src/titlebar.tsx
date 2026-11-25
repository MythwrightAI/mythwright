// The top of the desktop app's window.
//
// Before anyone is signed in, a slim bar with nothing in it but the three
// things every window has. Signed in, the bar is gone and the three buttons
// sit at the end of the first row of tabs; the empty parts of that row, and
// the head of the side panel, move the window instead (`grab`).
//
// Either way it is see-through: what shows is the page under it, so it is
// the page's colour in light and dark, and dims with the page when a dialog
// is open, while the buttons stay above the dialog and go on working.

import { ask, desktop, framed } from './desktop.ts';
import type { Edge } from './desktop.ts';
import { classes } from './util.ts';

const EDGES: Edge[] = ['n', 's', 'e', 'w', 'ne', 'nw', 'se', 'sw'];

function Icon({ d }: { d: string }) {
  return (
    <svg width="12" height="12" viewBox="0 0 12 12" aria-hidden="true">
      <path d={d} fill="none" stroke="currentColor" stroke-width="1" />
    </svg>
  );
}

/**
 * Whether the window's buttons share the first row of tabs once someone is
 * signed in, instead of having a bar to themselves. Only where the client
 * draws the buttons: where the system draws its own, top left, the bar stays.
 */
export const cornered = framed && desktop?.frame === 'page';
/**
 * The room kept clear at the end of that row, in pixels: the three buttons,
 * and beside them a little that is always empty, so that there is somewhere
 * to take hold of the window however many tabs are open.
 */
export const CORNER = 138 + 40;

/**
 * A press on an empty part of the top of the window moves it, and the second
 * press of a double click maximises it, as on any title bar. Whatever is
 * drawn along the top takes this for its `onMouseDown`.
 */
export function grab(event: MouseEvent): void {
  if (!framed || event.button !== 0 || (event.target as Element).closest('button, a, input, .tab')) return;
  ask(event.detail === 2 ? 'maximize' : 'drag');
}

export function TitleBar({ corner = false }: { corner?: boolean }) {
  if (!framed || !desktop) return null;
  // In the corner there is no bar to press on, only the buttons.
  const inCorner = corner && cornered;
  return (
    <>
      <header class={classes('titlebar', inCorner && 'corner')} data-frame={desktop.frame} onMouseDown={inCorner ? undefined : grab}>
        {desktop.frame === 'page' && (
          <div class="window-controls">
            <button aria-label="Minimise" title="Minimise" onClick={() => ask('minimize')}>
              <Icon d="M2 6.5h8" />
            </button>
            <button aria-label="Maximise" title="Maximise" onClick={() => ask('maximize')}>
              <Icon d="M2.5 2.5h7v7h-7z" />
            </button>
            <button class="close" aria-label="Close" title="Close" onClick={() => ask('close')}>
              <Icon d="M2.5 2.5l7 7M9.5 2.5l-7 7" />
            </button>
          </div>
        )}
      </header>
      {/* With no frame of the system's, the edges are the client's to offer too. */}
      {desktop.frame === 'page' &&
        EDGES.map((edge) => (
          <div
            key={edge}
            class={`window-edge ${edge}`}
            onMouseDown={(event) => event.button === 0 && ask({ resize: edge })}
          />
        ))}
    </>
  );
}

// The desktop app's window, when the client is running in one.
//
// In a browser none of this exists and nothing here does anything. In the
// desktop app the window has no title bar of the system's: the client draws
// its own (see `titlebar.tsx`), and asks the window to do what a title bar
// does (move, resize, minimise, maximise, close) through `window.ipc`.

/** How the window is framed, as the desktop app says when it loads the page. */
export interface Desktop {
  /**
   * `page`: the client draws the bar and its buttons. `overlay`: the system
   * draws its own buttons over the page's top-left corner (macOS) and the
   * client leaves them room. `system`: the system draws a whole title bar
   * and the client draws none.
   */
  frame: 'page' | 'overlay' | 'system';
}

export type Edge = 'n' | 's' | 'e' | 'w' | 'ne' | 'nw' | 'se' | 'sw';
export type Ask = 'drag' | 'minimize' | 'maximize' | 'close' | { resize: Edge };

export const desktop: Desktop | null =
  typeof window !== 'undefined' && window.ipc && window.desktop ? window.desktop : null;

/** Whether the client draws the top of the window. */
export const framed = desktop !== null && desktop.frame !== 'system';

export function ask(what: Ask): void {
  window.ipc?.postMessage(JSON.stringify({ window: what }));
}
